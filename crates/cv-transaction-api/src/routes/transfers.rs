use crate::middleware::AuthClaims;
use crate::state::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use cv_account_manager::{AccountManager, AccountRecord};
use cv_audit::{AuditAction, AuditEvent, AuditSink};
use cv_double_entry_engine::{validate_journal_batch, PostingLeg};
use cv_payment_gateway::PaymentRailProvider;
use cv_types::traits::{OutwardTransfer, PaymentRailAdapter};
use cv_types::{
    AccountId, AccountStatus, AccountType, Currency, EntryType, PndRuleType, PndSource,
    TenantPndRule,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/transfers/intra", post(post_intra_transfer))
        .route("/api/v1/transfers/enquiry", post(name_enquiry))
        .route("/api/v1/transfers/outward", post(post_outward_transfer))
        .route("/api/v1/payment-rails", get(get_payment_rails))
        .route("/api/v1/admin/payment-rails/active", post(set_active_rail))
        .with_state(state)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NameEnquiryDto {
    pub bank_code: String,
    pub account_number: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OutwardTransferDto {
    pub source_account_id: Uuid,
    pub destination_account: String,
    pub destination_bank_code: String,
    pub amount: Decimal,
    pub currency: Currency,
    pub narration: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SwitchRailDto {
    pub provider: String,
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
    auth: AuthClaims,
    State(state): State<AppState>,
    Json(dto): Json<IntraTransferDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    // 0. Permission check
    auth.require_scope_or_role("transfers:write", &["SUPER_ADMIN", "OPERATIONS_OFFICER"])?;

    // 1. Basic validation
    if dto.amount <= Decimal::ZERO {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "VALIDATION_FAILED",
                "error": "Transfer amount must be strictly positive"
            })),
        ));
    }

    if dto.source_account_id == dto.destination_account_id {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "VALIDATION_FAILED",
                "error": "Source and destination accounts cannot be identical"
            })),
        ));
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

    // 3. Database ACID Transaction Execution
    if let Some(pool) = &state.pool {
        let mut tx = pool.begin().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Failed to begin DB transaction: {e}") })),
            )
        })?;

        // Deterministic lock order to prevent deadlock
        let (first_id, second_id) = if dto.source_account_id < dto.destination_account_id {
            (dto.source_account_id, dto.destination_account_id)
        } else {
            (dto.destination_account_id, dto.source_account_id)
        };

        let row1 = sqlx::query(
            "SELECT id, tenant_id, account_number, account_name, gl_code, account_type, currency_code,
                    ledger_balance, available_balance, lien_amount, overdraft_limit, status,
                    pnd_source, pnd_reason, version
             FROM accounts WHERE id = $1 FOR UPDATE"
        )
        .bind(first_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Database lock error: {e}") })),
            )
        })?;

        let row2 = sqlx::query(
            "SELECT id, tenant_id, account_number, account_name, gl_code, account_type, currency_code,
                    ledger_balance, available_balance, lien_amount, overdraft_limit, status,
                    pnd_source, pnd_reason, version
             FROM accounts WHERE id = $1 FOR UPDATE"
        )
        .bind(second_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Database lock error: {e}") })),
            )
        })?;

        let (source_row, dest_row) = match (row1, row2) {
            (Some(r1), Some(r2)) => {
                let id1: Uuid = r1.get("id");
                if id1 == dto.source_account_id {
                    (r1, r2)
                } else {
                    (r2, r1)
                }
            }
            (None, _) | (_, None) => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(
                        json!({ "status": "NOT_FOUND", "message": "Source or destination account not found" }),
                    ),
                ));
            }
        };

        // Source account restrictions
        let source_status_str: String = source_row.get("status");
        let source_pnd_source: Option<String> = source_row.get("pnd_source");
        let source_pnd_reason: Option<String> = source_row.get("pnd_reason");
        let source_tenant_id: Uuid = source_row.get("tenant_id");
        let source_nuban: String = source_row.get("account_number");

        if auth.role() != "SUPER_ADMIN" && source_tenant_id != auth.tenant_id() {
            return Err((
                StatusCode::FORBIDDEN,
                Json(json!({
                    "status": "FORBIDDEN",
                    "message": "Source account does not belong to authenticated tenant"
                })),
            ));
        }

        let source_status = match source_status_str.as_str() {
            "ACTIVE" => AccountStatus::Active,
            "POST_NO_DEBIT" => AccountStatus::PostNoDebit,
            "FROZEN" => AccountStatus::Frozen,
            "DORMANT" => AccountStatus::Dormant,
            "CLOSED" => AccountStatus::Closed,
            _ => AccountStatus::PendingOpening,
        };

        if !source_status.can_debit() {
            return Err((
                StatusCode::FORBIDDEN,
                Json(json!({
                    "status": "ACCOUNT_RESTRICTED",
                    "error": format!("Account is under restriction ({source_status_str}) and cannot be debited"),
                    "pnd_source": source_pnd_source,
                    "pnd_reason": source_pnd_reason
                })),
            ));
        }

        // Destination account restrictions
        let dest_status_str: String = dest_row.get("status");
        if dest_status_str == "FROZEN" || dest_status_str == "CLOSED" {
            return Err((
                StatusCode::FORBIDDEN,
                Json(json!({
                    "status": "ACCOUNT_RESTRICTED",
                    "error": format!("Destination account is restricted ({dest_status_str}) and cannot receive credits")
                })),
            ));
        }

        // Check custom tenant PND compliance rules
        let pnd_rules_rows = sqlx::query(
            "SELECT id, tenant_id, rule_name, rule_type, max_single_amount, max_daily_cumulative, is_enabled
             FROM tenant_pnd_rules WHERE tenant_id = $1 AND is_enabled = true"
        )
        .bind(source_tenant_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Database rule lookup error: {e}") })),
            )
        })?;

        let mut tenant_rules = Vec::new();
        for r in pnd_rules_rows {
            let rule_type_str: String = r.get("rule_type");
            let rule_type = match rule_type_str.as_str() {
                "KYC_TIER_LIMIT" => PndRuleType::KycTierLimit,
                "VELOCITY_BURST" => PndRuleType::VelocityBurst,
                "DAILY_DEBIT_CAP" => PndRuleType::DailyDebitCap,
                _ => PndRuleType::SuspiciousTransfer,
            };
            tenant_rules.push(TenantPndRule {
                id: r.get("id"),
                tenant_id: r.get("tenant_id"),
                rule_name: r.get("rule_name"),
                rule_type,
                max_single_amount: r.get("max_single_amount"),
                max_daily_cumulative: r.get("max_daily_cumulative"),
                is_enabled: r.get("is_enabled"),
            });
        }

        let pnd_source_enum = match source_pnd_source.as_deref() {
            Some("MANUAL_CLIENT_PND") => Some(PndSource::ManualClientPnd),
            Some("SYSTEM_AUTOMATED_PND") => Some(PndSource::SystemAutomatedPnd),
            _ => None,
        };

        let source_record = AccountRecord {
            id: AccountId(dto.source_account_id),
            tenant_id: source_tenant_id,
            customer_id: None,
            account_number: source_nuban.clone(),
            account_name: source_row.get("account_name"),
            gl_code: source_row.get("gl_code"),
            account_type: AccountType::Savings,
            currency: dto.currency,
            ledger_balance: source_row.get("ledger_balance"),
            available_balance: source_row.get("available_balance"),
            lien_amount: source_row.get("lien_amount"),
            overdraft_limit: source_row.get("overdraft_limit"),
            status: source_status,
            pnd_source: pnd_source_enum,
            pnd_reason: source_pnd_reason,
            version: source_row.get("version"),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        if let Some(triggered_rule) = AccountManager::evaluate_transaction_pnd_rules(
            &source_record,
            dto.amount,
            &tenant_rules,
        ) {
            let rule_name = triggered_rule.rule_name.clone();
            let pnd_reason = format!(
                "Transfer amount {} {} breached tenant rule limit",
                dto.amount, dto.currency
            );

            // Update account to POST_NO_DEBIT in database
            sqlx::query(
                "UPDATE accounts
                 SET status = 'POST_NO_DEBIT',
                     pnd_source = 'SYSTEM_AUTOMATED_PND',
                     pnd_reason = $1,
                     pnd_placed_at = NOW(),
                     updated_at = NOW()
                 WHERE id = $2"
            )
            .bind(&pnd_reason)
            .bind(dto.source_account_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "status": "ERROR", "message": format!("Failed to apply PND in DB: {e}") })),
                )
            })?;

            // Record audit log for automated PND
            let audit_payload = json!({
                "rule_name": rule_name,
                "amount": dto.amount,
                "currency": dto.currency,
                "source_account_id": dto.source_account_id
            });

            let _ = sqlx::query(
                "INSERT INTO audit_logs (id, tenant_id, actor_id, actor_type, action, entity_name, entity_id, payload, created_at)
                 VALUES ($1, $2, $3, 'SYSTEM', 'SYSTEM_AUTOMATED_PND_APPLIED', 'ACCOUNT', $4, $5, NOW())"
            )
            .bind(Uuid::new_v4())
            .bind(source_tenant_id)
            .bind(auth.user_id())
            .bind(dto.source_account_id)
            .bind(audit_payload)
            .execute(&mut *tx)
            .await;

            // Commit the PND update
            let _ = tx.commit().await;

            return Err((
                StatusCode::CONFLICT,
                Json(json!({
                    "status": "PND_ENFORCED",
                    "error": format!(
                        "Transaction breached tenant compliance rule '{rule_name}'. Account has been automatically placed under SYSTEM_AUTOMATED_PND"
                    ),
                    "triggered_rule": rule_name,
                    "pnd_source": "SYSTEM_AUTOMATED_PND"
                })),
            ));
        }

        // Available balance check
        let source_avail: Decimal = source_row.get("available_balance");
        let source_overdraft: Decimal = source_row.get("overdraft_limit");
        let source_lien: Decimal = source_row.get("lien_amount");
        let effective_available = source_avail + source_overdraft - source_lien;

        if dto.amount > effective_available {
            return Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({
                    "status": "INSUFFICIENT_FUNDS",
                    "error": "Source account available balance insufficient for debit"
                })),
            ));
        }

        // Execute Balance Updates
        let source_ledger: Decimal = source_row.get("ledger_balance");
        let dest_ledger: Decimal = dest_row.get("ledger_balance");
        let dest_id: Uuid = dest_row.get("id");
        let dest_gl: String = dest_row.get("gl_code");
        let source_gl: String = source_row.get("gl_code");

        let new_source_ledger = source_ledger - dto.amount;
        let new_dest_ledger = dest_ledger + dto.amount;

        sqlx::query(
            "UPDATE accounts
             SET ledger_balance = ledger_balance - $1,
                 available_balance = available_balance - $1,
                 version = version + 1,
                 updated_at = NOW()
             WHERE id = $2"
        )
        .bind(dto.amount)
        .bind(dto.source_account_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Failed to update source balance: {e}") })),
            )
        })?;

        sqlx::query(
            "UPDATE accounts
             SET ledger_balance = ledger_balance + $1,
                 available_balance = available_balance + $1,
                 version = version + 1,
                 updated_at = NOW()
             WHERE id = $2"
        )
        .bind(dto.amount)
        .bind(dest_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Failed to update destination balance: {e}") })),
            )
        })?;

        // Insert Transaction Header
        let txn_id = Uuid::new_v4();
        let txn_ref = format!("TXN_{}", Uuid::new_v4().simple());
        let idempotency_key = format!("IDEMP_{}", Uuid::new_v4().simple());
        let currency_code = dto.currency.code();

        sqlx::query(
            "INSERT INTO transactions (
                id, tenant_id, transaction_ref, idempotency_key, channel, status,
                total_amount, currency_code, narration, metadata, created_at
            ) VALUES ($1, $2, $3, $4, 'API', 'COMMITTED', $5, $6, $7, '{}'::jsonb, NOW())"
        )
        .bind(txn_id)
        .bind(source_tenant_id)
        .bind(&txn_ref)
        .bind(&idempotency_key)
        .bind(dto.amount)
        .bind(currency_code)
        .bind(&dto.narration)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Failed to insert transaction record: {e}") })),
            )
        })?;

        // Insert Dual-Leg Immutable Journal Entries
        // DR Leg
        sqlx::query(
            "INSERT INTO journal_entries (
                id, tenant_id, transaction_id, account_id, gl_code, entry_type,
                amount, currency_code, running_balance, created_at
            ) VALUES ($1, $2, $3, $4, $5, 'DR', $6, $7, $8, NOW())"
        )
        .bind(Uuid::new_v4())
        .bind(source_tenant_id)
        .bind(txn_id)
        .bind(dto.source_account_id)
        .bind(&source_gl)
        .bind(dto.amount)
        .bind(currency_code)
        .bind(new_source_ledger)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Failed to insert DR journal entry: {e}") })),
            )
        })?;

        // CR Leg
        sqlx::query(
            "INSERT INTO journal_entries (
                id, tenant_id, transaction_id, account_id, gl_code, entry_type,
                amount, currency_code, running_balance, created_at
            ) VALUES ($1, $2, $3, $4, $5, 'CR', $6, $7, $8, NOW())"
        )
        .bind(Uuid::new_v4())
        .bind(source_tenant_id)
        .bind(txn_id)
        .bind(dest_id)
        .bind(&dest_gl)
        .bind(dto.amount)
        .bind(currency_code)
        .bind(new_dest_ledger)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Failed to insert CR journal entry: {e}") })),
            )
        })?;

        // Insert Audit Log Record
        let audit_payload = json!({
            "source_account_id": dto.source_account_id,
            "destination_account_id": dto.destination_account_id,
            "amount": dto.amount,
            "currency": dto.currency,
            "narration": dto.narration,
            "transaction_ref": txn_ref
        });

        let _ = sqlx::query(
            "INSERT INTO audit_logs (id, tenant_id, actor_id, actor_type, action, entity_name, entity_id, payload, created_at)
             VALUES ($1, $2, $3, 'USER', 'INTRA_TRANSFER_COMMITTED', 'TRANSACTION', $4, $5, NOW())"
        )
        .bind(Uuid::new_v4())
        .bind(source_tenant_id)
        .bind(auth.user_id())
        .bind(txn_id)
        .bind(audit_payload)
        .execute(&mut *tx)
        .await;

        // Commit ACID Transaction
        tx.commit().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Transaction commit failed: {e}") })),
            )
        })?;

        return Ok((
            StatusCode::CREATED,
            Json(json!({
                "status": "COMMITTED",
                "transaction_id": txn_id,
                "amount": dto.amount,
                "currency": dto.currency,
                "narration": dto.narration
            })),
        ));
    }

    // In-memory fallback if database pool unavailable
    {
        let accounts_by_id = state.accounts_by_id.read().unwrap();
        if let Some(source_account) = accounts_by_id.get(&dto.source_account_id) {
            if auth.role() != "SUPER_ADMIN" && source_account.tenant_id != auth.tenant_id() {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "status": "FORBIDDEN",
                        "message": "Source account does not belong to authenticated tenant"
                    })),
                ));
            }

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

            let rules = state.pnd_rules.read().unwrap();
            if let Some(triggered_rule) =
                AccountManager::evaluate_transaction_pnd_rules(source_account, dto.amount, &rules)
            {
                let rule_name = triggered_rule.rule_name.clone();
                drop(rules);
                drop(accounts_by_id);

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

    let txn_id = Uuid::new_v4();
    let audit_event = AuditEvent {
        id: Uuid::new_v4(),
        tenant_id: auth.tenant_id(),
        actor_id: auth.user_id(),
        action: AuditAction::TransferPosted,
        entity_name: "TRANSACTION".to_string(),
        entity_id: txn_id,
        payload: json!({
            "source_account_id": dto.source_account_id,
            "destination_account_id": dto.destination_account_id,
            "amount": dto.amount,
            "currency": dto.currency.code(),
        }),
        timestamp: chrono::Utc::now(),
    };
    let _ = state.audit_sink.record(audit_event).await;

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

async fn name_enquiry(
    auth: AuthClaims,
    State(state): State<AppState>,
    Json(dto): Json<NameEnquiryDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_scope_or_role("transfers:read", &["SUPER_ADMIN", "OPERATIONS_OFFICER"])?;

    if dto.account_number.trim().is_empty() || dto.bank_code.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "VALIDATION_FAILED",
                "error": "bank_code and account_number are required"
            })),
        ));
    }

    let enquiry = state
        .payment_gateway
        .name_enquiry(&dto.bank_code, &dto.account_number)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "status": "ENQUIRY_FAILED",
                    "error": e.to_string()
                })),
            )
        })?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "SUCCESS",
            "provider": state.payment_gateway.active_provider().to_string(),
            "data": {
                "account_number": enquiry.account_number,
                "account_name": enquiry.account_name,
                "bank_code": enquiry.bank_code,
                "kyc_tier": enquiry.kyc_tier
            }
        })),
    ))
}

async fn post_outward_transfer(
    auth: AuthClaims,
    State(state): State<AppState>,
    Json(dto): Json<OutwardTransferDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_scope_or_role("transfers:write", &["SUPER_ADMIN", "OPERATIONS_OFFICER"])?;

    if dto.amount <= Decimal::ZERO {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "VALIDATION_FAILED",
                "error": "Transfer amount must be strictly positive"
            })),
        ));
    }

    if dto.destination_account.trim().is_empty() || dto.destination_bank_code.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "VALIDATION_FAILED",
                "error": "destination_account and destination_bank_code are required"
            })),
        ));
    }

    let txn_ref = format!("CV-OUT-{}", Uuid::new_v4().simple());

    // 1. Double-entry posting leg validation:
    // Debit Customer Deposit GL (2110001), Credit Outward Settlement Clearing GL (1120001)
    let legs = vec![
        PostingLeg {
            account_id: dto.source_account_id,
            gl_code: "2110001".to_string(),
            currency: dto.currency,
            entry_type: EntryType::Debit,
            amount: dto.amount,
        },
        PostingLeg {
            account_id: Uuid::nil(), // Virtual clearing suspense
            gl_code: "1120001".to_string(),
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

    // 2. Database ACID transaction (if DB connected)
    if let Some(pool) = &state.pool {
        let mut tx = pool.begin().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Failed to begin DB transaction: {e}") })),
            )
        })?;

        let src_row = sqlx::query(
            "SELECT id, tenant_id, account_number, ledger_balance, available_balance, status FROM accounts WHERE id = $1 FOR UPDATE"
        )
        .bind(dto.source_account_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Database read error: {e}") })),
            )
        })?;

        let src = match src_row {
            Some(row) => row,
            None => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(
                        json!({ "status": "ACCOUNT_NOT_FOUND", "error": "Source account not found" }),
                    ),
                ));
            }
        };

        let src_tenant_id: Uuid = src.get("tenant_id");
        if auth.role() != "SUPER_ADMIN" && src_tenant_id != auth.tenant_id() {
            return Err((
                StatusCode::FORBIDDEN,
                Json(json!({
                    "status": "FORBIDDEN",
                    "message": "Source account does not belong to authenticated tenant"
                })),
            ));
        }

        let src_status: String = src.get("status");
        if src_status == "FROZEN" || src_status == "POST_NO_DEBIT" {
            return Err((
                StatusCode::FORBIDDEN,
                Json(json!({
                    "status": "ACCOUNT_RESTRICTED",
                    "error": format!("Source account is under restriction ({src_status}) and cannot be debited")
                })),
            ));
        }

        let src_available: Decimal = src.get("available_balance");
        if src_available < dto.amount {
            return Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({
                    "status": "INSUFFICIENT_FUNDS",
                    "error": format!("Available balance {src_available} is insufficient for amount {}", dto.amount)
                })),
            ));
        }

        // Deduct balance from source account
        sqlx::query(
            "UPDATE accounts SET ledger_balance = ledger_balance - $1, available_balance = available_balance - $1, version = version + 1, updated_at = NOW() WHERE id = $2"
        )
        .bind(dto.amount)
        .bind(dto.source_account_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Failed to update account balance: {e}") })),
            )
        })?;

        // 3. Dispatch outward transfer via the active PaymentRailAdapter (Anchor default)
        let src_nuban: String = src.get("account_number");
        let outward_req = OutwardTransfer {
            transaction_ref: txn_ref.clone(),
            source_account: src_nuban,
            destination_account: dto.destination_account.clone(),
            destination_bank_code: dto.destination_bank_code.clone(),
            amount: dto.amount,
            narration: dto.narration.clone(),
        };

        let receipt = state
            .payment_gateway
            .dispatch_outward(&outward_req)
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({
                        "status": "RAIL_DISPATCH_FAILED",
                        "error": format!("Payment rail dispatch failed: {e}")
                    })),
                )
            })?;

        let txn_id = Uuid::new_v4();
        let idempotency_key = format!("IDEMP_{}", Uuid::new_v4().simple());
        let currency_code = dto.currency.code();

        // Insert transaction record
        sqlx::query(
            "INSERT INTO transactions (
                id, tenant_id, transaction_ref, idempotency_key, channel, status,
                total_amount, currency_code, narration, metadata, created_at
            ) VALUES ($1, $2, $3, $4, 'NIP', 'COMMITTED', $5, $6, $7, '{}'::jsonb, NOW())"
        )
        .bind(txn_id)
        .bind(auth.tenant_id())
        .bind(&txn_ref)
        .bind(&idempotency_key)
        .bind(dto.amount)
        .bind(currency_code)
        .bind(&dto.narration)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Failed to insert transaction: {e}") })),
            )
        })?;

        tx.commit().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Transaction commit failed: {e}") })),
            )
        })?;

        return Ok((
            StatusCode::CREATED,
            Json(json!({
                "status": "SUCCESS",
                "provider": state.payment_gateway.active_provider().to_string(),
                "data": {
                    "transaction_id": txn_id,
                    "reference": txn_ref,
                    "session_id": receipt.session_id,
                    "response_code": receipt.response_code,
                    "message": receipt.message,
                    "amount": dto.amount,
                    "currency": dto.currency,
                    "destination_account": dto.destination_account,
                    "destination_bank_code": dto.destination_bank_code,
                }
            })),
        ));
    }

    // In-memory fallback
    {
        let accounts = state.accounts_by_id.read().unwrap();
        if let Some(src) = accounts.get(&dto.source_account_id) {
            if auth.role() != "SUPER_ADMIN" && src.tenant_id != auth.tenant_id() {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "status": "FORBIDDEN",
                        "message": "Source account does not belong to authenticated tenant"
                    })),
                ));
            }
        }
    }

    let outward_req = OutwardTransfer {
        transaction_ref: txn_ref.clone(),
        source_account: "0581111111".to_string(),
        destination_account: dto.destination_account.clone(),
        destination_bank_code: dto.destination_bank_code.clone(),
        amount: dto.amount,
        narration: dto.narration.clone(),
    };

    let receipt = state
        .payment_gateway
        .dispatch_outward(&outward_req)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({
                    "status": "RAIL_DISPATCH_FAILED",
                    "error": format!("Payment rail dispatch failed: {e}")
                })),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "status": "SUCCESS",
            "provider": state.payment_gateway.active_provider().to_string(),
            "data": {
                "transaction_id": Uuid::new_v4(),
                "reference": txn_ref,
                "session_id": receipt.session_id,
                "response_code": receipt.response_code,
                "message": receipt.message,
                "amount": dto.amount,
                "currency": dto.currency,
                "destination_account": dto.destination_account,
                "destination_bank_code": dto.destination_bank_code,
            }
        })),
    ))
}

async fn get_payment_rails(
    auth: AuthClaims,
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_scope_or_role("transfers:read", &["SUPER_ADMIN", "OPERATIONS_OFFICER"])?;

    let active = state.payment_gateway.active_provider();
    let registered = state.payment_gateway.registered_providers();

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "SUCCESS",
            "active_provider": active.to_string(),
            "registered_providers": registered.iter().map(|p| p.to_string()).collect::<Vec<_>>()
        })),
    ))
}

async fn set_active_rail(
    auth: AuthClaims,
    State(state): State<AppState>,
    Json(dto): Json<SwitchRailDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_admin()?;

    let provider = match dto.provider.trim() {
        "Anchor" => PaymentRailProvider::Anchor,
        "NibssDirect" => PaymentRailProvider::NibssDirect,
        other => PaymentRailProvider::Custom(other.to_string()),
    };

    state
        .payment_gateway
        .set_active_provider(provider.clone())
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "status": "SWITCH_FAILED",
                    "error": e.to_string()
                })),
            )
        })?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "SUCCESS",
            "message": format!("Payment rail successfully switched to {}", provider),
            "active_provider": provider.to_string()
        })),
    ))
}
