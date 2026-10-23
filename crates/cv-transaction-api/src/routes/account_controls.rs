use axum::{
    extract::Path,
    http::StatusCode,
    routing::post,
    Json, Router,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub fn router() -> Router {
    Router::new()
        .route("/api/v1/accounts/:nuban/freeze", post(freeze_account))
        .route("/api/v1/accounts/:nuban/pnd", post(place_pnd))
        .route("/api/v1/accounts/:nuban/activate", post(activate_account))
        .route("/api/v1/accounts/:nuban/liens", post(place_lien))
        .route("/api/v1/accounts/:nuban/liens/release", post(release_lien))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReasonDto {
    pub reason: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LienDto {
    pub amount: Decimal,
    pub reason: String,
}

async fn freeze_account(
    Path(nuban): Path<String>,
    Json(dto): Json<ReasonDto>,
) -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "SUCCESS",
            "account_number": nuban,
            "previous_status": "ACTIVE",
            "current_status": "FROZEN",
            "reason": dto.reason,
            "can_debit": false,
            "can_credit": false
        })),
    )
}

async fn place_pnd(
    Path(nuban): Path<String>,
    Json(dto): Json<ReasonDto>,
) -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "SUCCESS",
            "account_number": nuban,
            "previous_status": "ACTIVE",
            "current_status": "POST_NO_DEBIT",
            "reason": dto.reason,
            "can_debit": false,
            "can_credit": true
        })),
    )
}

async fn activate_account(
    Path(nuban): Path<String>,
    Json(dto): Json<ReasonDto>,
) -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "SUCCESS",
            "account_number": nuban,
            "previous_status": "POST_NO_DEBIT",
            "current_status": "ACTIVE",
            "reason": dto.reason,
            "can_debit": true,
            "can_credit": true
        })),
    )
}

async fn place_lien(
    Path(nuban): Path<String>,
    Json(dto): Json<LienDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    if dto.amount <= Decimal::ZERO {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "status": "ERROR", "message": "Lien amount must be strictly positive" })),
        ));
    }

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "SUCCESS",
            "account_number": nuban,
            "placed_lien_amount": dto.amount,
            "reason": dto.reason,
            "action": "LIEN_PLACED"
        })),
    ))
}

async fn release_lien(
    Path(nuban): Path<String>,
    Json(dto): Json<LienDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    if dto.amount <= Decimal::ZERO {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "status": "ERROR", "message": "Release amount must be strictly positive" })),
        ));
    }

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "SUCCESS",
            "account_number": nuban,
            "released_lien_amount": dto.amount,
            "reason": dto.reason,
            "action": "LIEN_RELEASED"
        })),
    ))
}
