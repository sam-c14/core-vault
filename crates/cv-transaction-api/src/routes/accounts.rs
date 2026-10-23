use axum::{
    extract::Path,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use cv_account_manager::{AccountManager, CreateAccountDto};
use serde_json::{json, Value};

pub fn router() -> Router {
    Router::new()
        .route("/api/v1/accounts", post(create_account))
        .route("/api/v1/accounts/:nuban", get(get_account))
}

async fn create_account(Json(dto): Json<CreateAccountDto>) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    match AccountManager::build_new_account(dto) {
        Ok(account) => Ok((StatusCode::CREATED, Json(json!({
            "status": "SUCCESS",
            "account": account
        })))),
        Err(err) => Err((StatusCode::BAD_REQUEST, Json(json!({
            "status": "ERROR",
            "message": err.to_string()
        })))),
    }
}

async fn get_account(Path(nuban): Path<String>) -> (StatusCode, Json<Value>) {
    // Scaffold implementation
    (StatusCode::OK, Json(json!({
        "account_number": nuban,
        "status": "ACTIVE",
        "ledger_balance": "0.0000",
        "available_balance": "0.0000"
    })))
}
