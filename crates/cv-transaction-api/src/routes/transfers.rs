use crate::state::AppState;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use cv_account_manager::AccountManager;
use cv_double_entry_engine::{validate_journal_batch, PostingLeg};
use cv_types::{Currency, EntryType};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/transfers/intra", post(post_intra_transfer))
        .with_state(state)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IntraTransferDto {
    pub source_account_id: Uuid,
    pub destination_account_id: Uuid,
    pub amount: Decimal,
    pub currency: Currency,
    pub narration: String,
}

async fn post_intra_transfer(
    State(state): State<AppState>,
    Json(dto): Json<IntraTransferDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    // 1. Account restriction & automated tenant compliance PND rule check
    {
        let accounts_by_id = state.accounts_by_id.read().unwrap();
        if let Some(source_account) = accounts_by_id.get(&dto.source_account_id) {
            // Check if source account is restricted from debiting
            if !source_account.status.can_debit() {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "status": "ACCOUNT_RESTRICTED",
                        "error": format!("Account is under restriction ({}) and cannot be debited", source_account.status),
                        "pnd_source": source_account.pnd_source,
                        "pnd_reason": source_account.pnd_reason
                    })),
                ));
            }

            // Check if transfer breaches any custom tenant PND rules
            let rules = state.pnd_rules.read().unwrap();
            if let Some(triggered_rule) =
                AccountManager::evaluate_transaction_pnd_rules(source_account, dto.amount, &rules)
            {
                let rule_name = triggered_rule.rule_name.clone();
                drop(rules);
                drop(accounts_by_id);

                // Automatically dish out SYSTEM_AUTOMATED_PND on source account
                let mut accounts_by_id = state.accounts_by_id.write().unwrap();
                let mut accounts = state.accounts.write().unwrap();

                if let Some(acc) = accounts_by_id.get_mut(&dto.source_account_id) {
                    let _ = AccountManager::place_system_automated_pnd(
                        acc,
                        &rule_name,
                        &format!(
                            "Transfer amount {} {} breached tenant rule limit",
                            dto.amount, dto.currency
                        ),
                    );
                    if let Some(by_nuban) = accounts.get_mut(&acc.account_number) {
                        let _ = AccountManager::place_system_automated_pnd(
                            by_nuban,
                            &rule_name,
                            &format!(
                                "Transfer amount {} {} breached tenant rule limit",
                                dto.amount, dto.currency
                            ),
                        );
                    }
                }

                return Err((
                    StatusCode::CONFLICT,
                    Json(json!({
                        "status": "PND_ENFORCED",
                        "error": format!(
                            "Transaction breached tenant compliance rule '{}'. Account has been automatically placed under SYSTEM_AUTOMATED_PND",
                            rule_name
                        ),
                        "triggered_rule": rule_name,
                        "pnd_source": "SYSTEM_AUTOMATED_PND"
                    })),
                ));
            }
        }
    }

    // 2. Double-entry posting batch validation
    let legs = vec![
        PostingLeg {
            account_id: dto.source_account_id,
            gl_code: "2110001".to_string(),
            currency: dto.currency,
            entry_type: EntryType::Debit,
            amount: dto.amount,
        },
        PostingLeg {
            account_id: dto.destination_account_id,
            gl_code: "2110002".to_string(),
            currency: dto.currency,
            entry_type: EntryType::Credit,
            amount: dto.amount,
        },
    ];

    if let Err(err) = validate_journal_batch(&legs) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "VALIDATION_FAILED",
                "error": err.to_string()
            })),
        ));
    }

    let txn_id = Uuid::new_v4();
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "status": "COMMITTED",
            "transaction_id": txn_id,
            "amount": dto.amount,
            "currency": dto.currency,
            "narration": dto.narration
        })),
    ))
}
