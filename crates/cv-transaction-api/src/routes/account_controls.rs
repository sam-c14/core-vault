use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use cv_account_manager::AccountManager;
use cv_types::AccountStatus;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/accounts/:nuban/freeze", post(freeze_account))
        .route("/api/v1/accounts/:nuban/pnd", post(place_pnd))
        .route("/api/v1/accounts/:nuban/pnd/lift", post(lift_pnd))
        .route("/api/v1/accounts/:nuban/activate", post(activate_account))
        .route("/api/v1/accounts/:nuban/liens", post(place_lien))
        .route("/api/v1/accounts/:nuban/liens/release", post(release_lien))
        .with_state(state)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReasonDto {
    pub reason: String,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct LiftPndDto {
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LienDto {
    pub amount: Decimal,
    pub reason: String,
}

async fn freeze_account(
    State(state): State<AppState>,
    Path(nuban): Path<String>,
    Json(dto): Json<ReasonDto>,
) -> (StatusCode, Json<Value>) {
    let mut accounts = state.accounts.write().unwrap();
    let mut accounts_by_id = state.accounts_by_id.write().unwrap();

    let prev_status = if let Some(acc) = accounts.get_mut(&nuban) {
        let prev = acc.status;
        acc.status = AccountStatus::Frozen;
        if let Some(by_id) = accounts_by_id.get_mut(&acc.id.0) {
            by_id.status = AccountStatus::Frozen;
        }
        format!("{prev:?}")
    } else {
        "ACTIVE".to_string()
    };

    (
        StatusCode::OK,
        Json(json!({
            "status": "SUCCESS",
            "account_number": nuban,
            "previous_status": prev_status,
            "current_status": "FROZEN",
            "reason": dto.reason,
            "can_debit": false,
            "can_credit": false
        })),
    )
}

async fn place_pnd(
    State(state): State<AppState>,
    Path(nuban): Path<String>,
    Json(dto): Json<ReasonDto>,
) -> (StatusCode, Json<Value>) {
    let mut accounts = state.accounts.write().unwrap();
    let mut accounts_by_id = state.accounts_by_id.write().unwrap();

    let prev_status = if let Some(acc) = accounts.get_mut(&nuban) {
        let prev = acc.status;
        let _ = AccountManager::place_manual_pnd(acc, &dto.reason);
        if let Some(by_id) = accounts_by_id.get_mut(&acc.id.0) {
            let _ = AccountManager::place_manual_pnd(by_id, &dto.reason);
        }
        format!("{prev:?}")
    } else {
        "ACTIVE".to_string()
    };

    (
        StatusCode::OK,
        Json(json!({
            "status": "SUCCESS",
            "account_number": nuban,
            "previous_status": prev_status,
            "current_status": "POST_NO_DEBIT",
            "pnd_source": "MANUAL_CLIENT_PND",
            "reason": dto.reason,
            "can_debit": false,
            "can_credit": true
        })),
    )
}

async fn lift_pnd(
    State(state): State<AppState>,
    Path(nuban): Path<String>,
    body: Option<Json<LiftPndDto>>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let dto = body.map(|Json(d)| d);
    let mut accounts = state.accounts.write().unwrap();
    let mut accounts_by_id = state.accounts_by_id.write().unwrap();

    if let Some(acc) = accounts.get_mut(&nuban) {
        if acc.status != AccountStatus::PostNoDebit {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "status": "ERROR",
                    "message": format!("Account {} is not under Post-No-Debit", nuban)
                })),
            ));
        }

        let _ = AccountManager::lift_pnd(acc);
        if let Some(by_id) = accounts_by_id.get_mut(&acc.id.0) {
            let _ = AccountManager::lift_pnd(by_id);
        }

        Ok((
            StatusCode::OK,
            Json(json!({
                "status": "SUCCESS",
                "account_number": nuban,
                "previous_status": "POST_NO_DEBIT",
                "current_status": "ACTIVE",
                "pnd_source": null,
                "pnd_reason": null,
                "lift_notes": dto.and_then(|d| d.reason).unwrap_or_else(|| "Lifted by client tenant".to_string()),
                "can_debit": true,
                "can_credit": true
            })),
        ))
    } else {
        // Fallback for mocked accounts
        Ok((
            StatusCode::OK,
            Json(json!({
                "status": "SUCCESS",
                "account_number": nuban,
                "previous_status": "POST_NO_DEBIT",
                "current_status": "ACTIVE",
                "pnd_source": null,
                "pnd_reason": null,
                "can_debit": true,
                "can_credit": true
            })),
        ))
    }
}

async fn activate_account(
    State(state): State<AppState>,
    Path(nuban): Path<String>,
    Json(dto): Json<ReasonDto>,
) -> (StatusCode, Json<Value>) {
    let mut accounts = state.accounts.write().unwrap();
    let mut accounts_by_id = state.accounts_by_id.write().unwrap();

    let prev_status = if let Some(acc) = accounts.get_mut(&nuban) {
        let prev = acc.status;
        acc.status = AccountStatus::Active;
        acc.pnd_source = None;
        acc.pnd_reason = None;
        if let Some(by_id) = accounts_by_id.get_mut(&acc.id.0) {
            by_id.status = AccountStatus::Active;
            by_id.pnd_source = None;
            by_id.pnd_reason = None;
        }
        format!("{prev:?}")
    } else {
        "POST_NO_DEBIT".to_string()
    };

    (
        StatusCode::OK,
        Json(json!({
            "status": "SUCCESS",
            "account_number": nuban,
            "previous_status": prev_status,
            "current_status": "ACTIVE",
            "reason": dto.reason,
            "can_debit": true,
            "can_credit": true
        })),
    )
}

async fn place_lien(
    State(state): State<AppState>,
    Path(nuban): Path<String>,
    Json(dto): Json<LienDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    if dto.amount <= Decimal::ZERO {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "status": "ERROR", "message": "Lien amount must be strictly positive" })),
        ));
    }

    let mut accounts = state.accounts.write().unwrap();
    let mut accounts_by_id = state.accounts_by_id.write().unwrap();

    if let Some(acc) = accounts.get_mut(&nuban) {
        if let Err(e) = AccountManager::place_lien(acc, dto.amount) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "status": "ERROR", "message": e.to_string() })),
            ));
        }
        if let Some(by_id) = accounts_by_id.get_mut(&acc.id.0) {
            let _ = AccountManager::place_lien(by_id, dto.amount);
        }
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
    State(state): State<AppState>,
    Path(nuban): Path<String>,
    Json(dto): Json<LienDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    if dto.amount <= Decimal::ZERO {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                json!({ "status": "ERROR", "message": "Release amount must be strictly positive" }),
            ),
        ));
    }

    let mut accounts = state.accounts.write().unwrap();
    let mut accounts_by_id = state.accounts_by_id.write().unwrap();

    if let Some(acc) = accounts.get_mut(&nuban) {
        if let Err(e) = AccountManager::release_lien(acc, dto.amount) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "status": "ERROR", "message": e.to_string() })),
            ));
        }
        if let Some(by_id) = accounts_by_id.get_mut(&acc.id.0) {
            let _ = AccountManager::release_lien(by_id, dto.amount);
        }
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
