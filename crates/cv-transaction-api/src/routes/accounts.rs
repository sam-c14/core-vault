use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use cv_account_manager::{AccountManager, CreateAccountDto};
use serde_json::{json, Value};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/accounts", post(create_account))
        .route("/api/v1/accounts/:nuban", get(get_account))
        .with_state(state)
}

async fn create_account(
    State(state): State<AppState>,
    Json(dto): Json<CreateAccountDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    match AccountManager::build_new_account(dto) {
        Ok(account) => {
            {
                let mut accounts = state.accounts.write().unwrap();
                let mut by_id = state.accounts_by_id.write().unwrap();
                accounts.insert(account.account_number.clone(), account.clone());
                by_id.insert(account.id.0, account.clone());
            }

            Ok((
                StatusCode::CREATED,
                Json(json!({
                    "status": "SUCCESS",
                    "account": account
                })),
            ))
        }
        Err(err) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "ERROR",
                "message": err.to_string()
            })),
        )),
    }
}

async fn get_account(
    State(state): State<AppState>,
    Path(nuban): Path<String>,
) -> (StatusCode, Json<Value>) {
    let accounts = state.accounts.read().unwrap();
    if let Some(acc) = accounts.get(&nuban) {
        (
            StatusCode::OK,
            Json(json!({
                "account_number": acc.account_number,
                "status": acc.status,
                "pnd_source": acc.pnd_source,
                "pnd_reason": acc.pnd_reason,
                "ledger_balance": acc.ledger_balance,
                "available_balance": acc.available_balance,
                "effective_available_funds": acc.effective_available_funds()
            })),
        )
    } else {
        (
            StatusCode::OK,
            Json(json!({
                "account_number": nuban,
                "status": "ACTIVE",
                "ledger_balance": "0.0000",
                "available_balance": "0.0000"
            })),
        )
    }
}
