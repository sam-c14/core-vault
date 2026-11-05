use crate::middleware::AuthClaims;
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
use sqlx::Row;
use uuid::Uuid;

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
    auth: AuthClaims,
    State(state): State<AppState>,
    Path(nuban): Path<String>,
    Json(dto): Json<ReasonDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_scope_or_role(
        "accounts:write",
        &["SUPER_ADMIN", "OPERATIONS_OFFICER", "COMPLIANCE_OFFICER"],
    )?;

    if let Some(pool) = &state.pool {
        let mut tx = pool.begin().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB error: {e}") })),
            )
        })?;

        let row_opt = sqlx::query(
            "SELECT id, tenant_id, status FROM accounts WHERE account_number = $1 FOR UPDATE",
        )
        .bind(&nuban)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB error: {e}") })),
            )
        })?;

        let row = match row_opt {
            Some(r) => r,
            None => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(
                        json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                    ),
                ));
            }
        };

        let prev_status: String = row.get("status");
        let account_id: Uuid = row.get("id");
        let tenant_id: Uuid = row.get("tenant_id");

        if auth.role() != "SUPER_ADMIN" && tenant_id != auth.tenant_id() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(
                    json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                ),
            ));
        }

        if auth.role() != "SUPER_ADMIN" && tenant_id != auth.tenant_id() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(
                    json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                ),
            ));
        }

        sqlx::query(
            "UPDATE accounts SET status = 'FROZEN', updated_at = NOW() WHERE account_number = $1",
        )
        .bind(&nuban)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB error: {e}") })),
            )
        })?;

        let audit_payload = json!({
            "account_number": nuban,
            "previous_status": prev_status,
            "current_status": "FROZEN",
            "reason": dto.reason
        });

        let _ = sqlx::query(
            "INSERT INTO audit_logs (id, tenant_id, actor_id, actor_type, action, entity_name, entity_id, payload, created_at)
             VALUES ($1, $2, $3, 'USER', 'FREEZE_ACCOUNT', 'ACCOUNT', $4, $5, NOW())"
        )
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(auth.user_id())
        .bind(account_id)
        .bind(audit_payload)
        .execute(&mut *tx)
        .await;

        tx.commit().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB commit error: {e}") })),
            )
        })?;

        // Update in-memory state
        {
            let mut accounts = state.accounts.write().unwrap();
            let mut by_id = state.accounts_by_id.write().unwrap();
            if let Some(acc) = accounts.get_mut(&nuban) {
                acc.status = AccountStatus::Frozen;
            }
            if let Some(by_id_acc) = by_id.get_mut(&account_id) {
                by_id_acc.status = AccountStatus::Frozen;
            }
        }

        return Ok((
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
        ));
    }

    let mut accounts = state.accounts.write().unwrap();
    let mut accounts_by_id = state.accounts_by_id.write().unwrap();

    let prev_status = if let Some(acc) = accounts.get_mut(&nuban) {
        if auth.role() != "SUPER_ADMIN" && acc.tenant_id != auth.tenant_id() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(
                    json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                ),
            ));
        }
        let prev = acc.status;
        acc.status = AccountStatus::Frozen;
        if let Some(by_id) = accounts_by_id.get_mut(&acc.id.0) {
            by_id.status = AccountStatus::Frozen;
        }
        format!("{prev:?}")
    } else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") })),
        ));
    };

    Ok((
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
    ))
}

async fn place_pnd(
    auth: AuthClaims,
    State(state): State<AppState>,
    Path(nuban): Path<String>,
    Json(dto): Json<ReasonDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_scope_or_role(
        "accounts:write",
        &["SUPER_ADMIN", "OPERATIONS_OFFICER", "COMPLIANCE_OFFICER"],
    )?;

    if let Some(pool) = &state.pool {
        let mut tx = pool.begin().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB error: {e}") })),
            )
        })?;

        let row_opt = sqlx::query(
            "SELECT id, tenant_id, status FROM accounts WHERE account_number = $1 FOR UPDATE",
        )
        .bind(&nuban)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB error: {e}") })),
            )
        })?;

        let row = match row_opt {
            Some(r) => r,
            None => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(
                        json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                    ),
                ));
            }
        };

        let prev_status: String = row.get("status");
        let account_id: Uuid = row.get("id");
        let tenant_id: Uuid = row.get("tenant_id");

        if auth.role() != "SUPER_ADMIN" && tenant_id != auth.tenant_id() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(
                    json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                ),
            ));
        }

        if auth.role() != "SUPER_ADMIN" && tenant_id != auth.tenant_id() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(
                    json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                ),
            ));
        }

        sqlx::query(
            "UPDATE accounts
             SET status = 'POST_NO_DEBIT',
                 pnd_source = 'MANUAL_CLIENT_PND',
                 pnd_reason = $1,
                 pnd_placed_at = NOW(),
                 updated_at = NOW()
             WHERE account_number = $2",
        )
        .bind(&dto.reason)
        .bind(&nuban)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB error: {e}") })),
            )
        })?;

        let audit_payload = json!({
            "account_number": nuban,
            "previous_status": prev_status,
            "current_status": "POST_NO_DEBIT",
            "pnd_source": "MANUAL_CLIENT_PND",
            "reason": dto.reason
        });

        let _ = sqlx::query(
            "INSERT INTO audit_logs (id, tenant_id, actor_id, actor_type, action, entity_name, entity_id, payload, created_at)
             VALUES ($1, $2, $3, 'USER', 'PLACE_PND', 'ACCOUNT', $4, $5, NOW())"
        )
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(auth.user_id())
        .bind(account_id)
        .bind(audit_payload)
        .execute(&mut *tx)
        .await;

        tx.commit().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB commit error: {e}") })),
            )
        })?;

        // Update in-memory state
        {
            let mut accounts = state.accounts.write().unwrap();
            let mut by_id = state.accounts_by_id.write().unwrap();
            if let Some(acc) = accounts.get_mut(&nuban) {
                let _ = AccountManager::place_manual_pnd(acc, &dto.reason);
            }
            if let Some(by_id_acc) = by_id.get_mut(&account_id) {
                let _ = AccountManager::place_manual_pnd(by_id_acc, &dto.reason);
            }
        }

        return Ok((
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
        ));
    }

    let mut accounts = state.accounts.write().unwrap();
    let mut accounts_by_id = state.accounts_by_id.write().unwrap();

    let prev_status = if let Some(acc) = accounts.get_mut(&nuban) {
        if auth.role() != "SUPER_ADMIN" && acc.tenant_id != auth.tenant_id() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(
                    json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                ),
            ));
        }
        let prev = acc.status;
        let _ = AccountManager::place_manual_pnd(acc, &dto.reason);
        if let Some(by_id) = accounts_by_id.get_mut(&acc.id.0) {
            let _ = AccountManager::place_manual_pnd(by_id, &dto.reason);
        }
        format!("{prev:?}")
    } else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") })),
        ));
    };

    Ok((
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
    ))
}

async fn lift_pnd(
    auth: AuthClaims,
    State(state): State<AppState>,
    Path(nuban): Path<String>,
    body: Option<Json<LiftPndDto>>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_scope_or_role(
        "accounts:write",
        &["SUPER_ADMIN", "OPERATIONS_OFFICER", "COMPLIANCE_OFFICER"],
    )?;

    let dto = body.map(|Json(d)| d);
    let lift_notes = dto
        .and_then(|d| d.reason)
        .unwrap_or_else(|| "Lifted by client tenant".to_string());

    if let Some(pool) = &state.pool {
        let mut tx = pool.begin().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB error: {e}") })),
            )
        })?;

        let row_opt = sqlx::query(
            "SELECT id, tenant_id, status FROM accounts WHERE account_number = $1 FOR UPDATE",
        )
        .bind(&nuban)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB error: {e}") })),
            )
        })?;

        let row = match row_opt {
            Some(r) => r,
            None => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(
                        json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                    ),
                ));
            }
        };

        let account_id: Uuid = row.get("id");
        let tenant_id: Uuid = row.get("tenant_id");

        if auth.role() != "SUPER_ADMIN" && tenant_id != auth.tenant_id() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(
                    json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                ),
            ));
        }

        let current_status: String = row.get("status");
        if current_status != "POST_NO_DEBIT" {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "status": "ERROR",
                    "message": format!("Account {} is not under Post-No-Debit", nuban)
                })),
            ));
        }

        sqlx::query(
            "UPDATE accounts
             SET status = 'ACTIVE',
                 pnd_source = NULL,
                 pnd_reason = NULL,
                 pnd_placed_at = NULL,
                 updated_at = NOW()
             WHERE account_number = $1",
        )
        .bind(&nuban)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB error: {e}") })),
            )
        })?;

        let audit_payload = json!({
            "account_number": nuban,
            "previous_status": "POST_NO_DEBIT",
            "current_status": "ACTIVE",
            "lift_notes": lift_notes
        });

        let _ = sqlx::query(
            "INSERT INTO audit_logs (id, tenant_id, actor_id, actor_type, action, entity_name, entity_id, payload, created_at)
             VALUES ($1, $2, $3, 'USER', 'LIFT_PND', 'ACCOUNT', $4, $5, NOW())"
        )
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(auth.user_id())
        .bind(account_id)
        .bind(audit_payload)
        .execute(&mut *tx)
        .await;

        tx.commit().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB commit error: {e}") })),
            )
        })?;

        // Update in-memory state
        {
            let mut accounts = state.accounts.write().unwrap();
            let mut by_id = state.accounts_by_id.write().unwrap();
            if let Some(acc) = accounts.get_mut(&nuban) {
                let _ = AccountManager::lift_pnd(acc);
            }
            if let Some(by_id_acc) = by_id.get_mut(&account_id) {
                let _ = AccountManager::lift_pnd(by_id_acc);
            }
        }

        return Ok((
            StatusCode::OK,
            Json(json!({
                "status": "SUCCESS",
                "account_number": nuban,
                "previous_status": "POST_NO_DEBIT",
                "current_status": "ACTIVE",
                "pnd_source": null,
                "pnd_reason": null,
                "lift_notes": lift_notes,
                "can_debit": true,
                "can_credit": true
            })),
        ));
    }

    let mut accounts = state.accounts.write().unwrap();
    let mut accounts_by_id = state.accounts_by_id.write().unwrap();

    if let Some(acc) = accounts.get_mut(&nuban) {
        if auth.role() != "SUPER_ADMIN" && acc.tenant_id != auth.tenant_id() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(
                    json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                ),
            ));
        }

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
                "lift_notes": lift_notes,
                "can_debit": true,
                "can_credit": true
            })),
        ))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") })),
        ))
    }
}

async fn activate_account(
    auth: AuthClaims,
    State(state): State<AppState>,
    Path(nuban): Path<String>,
    Json(dto): Json<ReasonDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_scope_or_role(
        "accounts:write",
        &["SUPER_ADMIN", "OPERATIONS_OFFICER", "COMPLIANCE_OFFICER"],
    )?;

    if let Some(pool) = &state.pool {
        let mut tx = pool.begin().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB error: {e}") })),
            )
        })?;

        let row_opt = sqlx::query(
            "SELECT id, tenant_id, status FROM accounts WHERE account_number = $1 FOR UPDATE",
        )
        .bind(&nuban)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB error: {e}") })),
            )
        })?;

        let row = match row_opt {
            Some(r) => r,
            None => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(
                        json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                    ),
                ));
            }
        };

        let prev_status: String = row.get("status");
        let account_id: Uuid = row.get("id");
        let tenant_id: Uuid = row.get("tenant_id");

        if auth.role() != "SUPER_ADMIN" && tenant_id != auth.tenant_id() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(
                    json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                ),
            ));
        }

        if auth.role() != "SUPER_ADMIN" && tenant_id != auth.tenant_id() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(
                    json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                ),
            ));
        }

        sqlx::query(
            "UPDATE accounts
             SET status = 'ACTIVE',
                 pnd_source = NULL,
                 pnd_reason = NULL,
                 pnd_placed_at = NULL,
                 updated_at = NOW()
             WHERE account_number = $1",
        )
        .bind(&nuban)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB error: {e}") })),
            )
        })?;

        let audit_payload = json!({
            "account_number": nuban,
            "previous_status": prev_status,
            "current_status": "ACTIVE",
            "reason": dto.reason
        });

        let _ = sqlx::query(
            "INSERT INTO audit_logs (id, tenant_id, actor_id, actor_type, action, entity_name, entity_id, payload, created_at)
             VALUES ($1, $2, $3, 'USER', 'ACTIVATE_ACCOUNT', 'ACCOUNT', $4, $5, NOW())"
        )
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(auth.user_id())
        .bind(account_id)
        .bind(audit_payload)
        .execute(&mut *tx)
        .await;

        tx.commit().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB commit error: {e}") })),
            )
        })?;

        // Update in-memory state
        {
            let mut accounts = state.accounts.write().unwrap();
            let mut by_id = state.accounts_by_id.write().unwrap();
            if let Some(acc) = accounts.get_mut(&nuban) {
                acc.status = AccountStatus::Active;
                acc.pnd_source = None;
                acc.pnd_reason = None;
            }
            if let Some(by_id_acc) = by_id.get_mut(&account_id) {
                by_id_acc.status = AccountStatus::Active;
                by_id_acc.pnd_source = None;
                by_id_acc.pnd_reason = None;
            }
        }

        return Ok((
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
        ));
    }

    let mut accounts = state.accounts.write().unwrap();
    let mut accounts_by_id = state.accounts_by_id.write().unwrap();

    let prev_status = if let Some(acc) = accounts.get_mut(&nuban) {
        if auth.role() != "SUPER_ADMIN" && acc.tenant_id != auth.tenant_id() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(
                    json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                ),
            ));
        }
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
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") })),
        ));
    };

    Ok((
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
    ))
}

async fn place_lien(
    auth: AuthClaims,
    State(state): State<AppState>,
    Path(nuban): Path<String>,
    Json(dto): Json<LienDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_scope_or_role(
        "accounts:write",
        &["SUPER_ADMIN", "OPERATIONS_OFFICER", "COMPLIANCE_OFFICER"],
    )?;

    if dto.amount <= Decimal::ZERO {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "status": "ERROR", "message": "Lien amount must be strictly positive" })),
        ));
    }

    if let Some(pool) = &state.pool {
        let mut tx = pool.begin().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB error: {e}") })),
            )
        })?;

        let row_opt = sqlx::query(
            "SELECT id, tenant_id, available_balance, lien_amount FROM accounts WHERE account_number = $1 FOR UPDATE"
        )
        .bind(&nuban)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB error: {e}") })),
            )
        })?;

        let row = match row_opt {
            Some(r) => r,
            None => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(
                        json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                    ),
                ));
            }
        };

        let account_id: Uuid = row.get("id");
        let tenant_id: Uuid = row.get("tenant_id");

        if auth.role() != "SUPER_ADMIN" && tenant_id != auth.tenant_id() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(
                    json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                ),
            ));
        }

        let available: Decimal = row.get("available_balance");
        if dto.amount > available {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(
                    json!({ "status": "ERROR", "message": "Insufficient available balance to place lien" }),
                ),
            ));
        }

        sqlx::query(
            "UPDATE accounts
             SET lien_amount = lien_amount + $1,
                 available_balance = available_balance - $1,
                 updated_at = NOW()
             WHERE account_number = $2",
        )
        .bind(dto.amount)
        .bind(&nuban)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB error: {e}") })),
            )
        })?;

        let audit_payload = json!({
            "account_number": nuban,
            "lien_amount": dto.amount,
            "reason": dto.reason,
            "action": "LIEN_PLACED"
        });

        let _ = sqlx::query(
            "INSERT INTO audit_logs (id, tenant_id, actor_id, actor_type, action, entity_name, entity_id, payload, created_at)
             VALUES ($1, $2, $3, 'USER', 'PLACE_LIEN', 'ACCOUNT', $4, $5, NOW())"
        )
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(auth.user_id())
        .bind(account_id)
        .bind(audit_payload)
        .execute(&mut *tx)
        .await;

        tx.commit().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB commit error: {e}") })),
            )
        })?;

        // Update in-memory state
        {
            let mut accounts = state.accounts.write().unwrap();
            let mut by_id = state.accounts_by_id.write().unwrap();
            if let Some(acc) = accounts.get_mut(&nuban) {
                let _ = AccountManager::place_lien(acc, dto.amount);
            }
            if let Some(by_id_acc) = by_id.get_mut(&account_id) {
                let _ = AccountManager::place_lien(by_id_acc, dto.amount);
            }
        }

        return Ok((
            StatusCode::OK,
            Json(json!({
                "status": "SUCCESS",
                "account_number": nuban,
                "placed_lien_amount": dto.amount,
                "reason": dto.reason,
                "action": "LIEN_PLACED"
            })),
        ));
    }

    let mut accounts = state.accounts.write().unwrap();
    let mut accounts_by_id = state.accounts_by_id.write().unwrap();

    if let Some(acc) = accounts.get_mut(&nuban) {
        if auth.role() != "SUPER_ADMIN" && acc.tenant_id != auth.tenant_id() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(
                    json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                ),
            ));
        }
        if let Err(e) = AccountManager::place_lien(acc, dto.amount) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "status": "ERROR", "message": e.to_string() })),
            ));
        }
        if let Some(by_id) = accounts_by_id.get_mut(&acc.id.0) {
            let _ = AccountManager::place_lien(by_id, dto.amount);
        }
    } else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") })),
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
    auth: AuthClaims,
    State(state): State<AppState>,
    Path(nuban): Path<String>,
    Json(dto): Json<LienDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_scope_or_role(
        "accounts:write",
        &["SUPER_ADMIN", "OPERATIONS_OFFICER", "COMPLIANCE_OFFICER"],
    )?;

    if dto.amount <= Decimal::ZERO {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                json!({ "status": "ERROR", "message": "Release amount must be strictly positive" }),
            ),
        ));
    }

    if let Some(pool) = &state.pool {
        let mut tx = pool.begin().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB error: {e}") })),
            )
        })?;

        let row_opt = sqlx::query(
            "SELECT id, tenant_id, available_balance, lien_amount FROM accounts WHERE account_number = $1 FOR UPDATE"
        )
        .bind(&nuban)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB error: {e}") })),
            )
        })?;

        let row = match row_opt {
            Some(r) => r,
            None => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(
                        json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                    ),
                ));
            }
        };

        let account_id: Uuid = row.get("id");
        let tenant_id: Uuid = row.get("tenant_id");

        if auth.role() != "SUPER_ADMIN" && tenant_id != auth.tenant_id() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(
                    json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                ),
            ));
        }

        let active_lien: Decimal = row.get("lien_amount");
        if dto.amount > active_lien {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(
                    json!({ "status": "ERROR", "message": "Release amount exceeds active lien balance" }),
                ),
            ));
        }

        sqlx::query(
            "UPDATE accounts
             SET lien_amount = lien_amount - $1,
                 available_balance = available_balance + $1,
                 updated_at = NOW()
             WHERE account_number = $2",
        )
        .bind(dto.amount)
        .bind(&nuban)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB error: {e}") })),
            )
        })?;

        let audit_payload = json!({
            "account_number": nuban,
            "released_lien_amount": dto.amount,
            "reason": dto.reason,
            "action": "LIEN_RELEASED"
        });

        let _ = sqlx::query(
            "INSERT INTO audit_logs (id, tenant_id, actor_id, actor_type, action, entity_name, entity_id, payload, created_at)
             VALUES ($1, $2, $3, 'USER', 'RELEASE_LIEN', 'ACCOUNT', $4, $5, NOW())"
        )
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(auth.user_id())
        .bind(account_id)
        .bind(audit_payload)
        .execute(&mut *tx)
        .await;

        tx.commit().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB commit error: {e}") })),
            )
        })?;

        // Update in-memory state
        {
            let mut accounts = state.accounts.write().unwrap();
            let mut by_id = state.accounts_by_id.write().unwrap();
            if let Some(acc) = accounts.get_mut(&nuban) {
                let _ = AccountManager::release_lien(acc, dto.amount);
            }
            if let Some(by_id_acc) = by_id.get_mut(&account_id) {
                let _ = AccountManager::release_lien(by_id_acc, dto.amount);
            }
        }

        return Ok((
            StatusCode::OK,
            Json(json!({
                "status": "SUCCESS",
                "account_number": nuban,
                "released_lien_amount": dto.amount,
                "reason": dto.reason,
                "action": "LIEN_RELEASED"
            })),
        ));
    }

    let mut accounts = state.accounts.write().unwrap();
    let mut accounts_by_id = state.accounts_by_id.write().unwrap();

    if let Some(acc) = accounts.get_mut(&nuban) {
        if auth.role() != "SUPER_ADMIN" && acc.tenant_id != auth.tenant_id() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(
                    json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                ),
            ));
        }
        if let Err(e) = AccountManager::release_lien(acc, dto.amount) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "status": "ERROR", "message": e.to_string() })),
            ));
        }
        if let Some(by_id) = accounts_by_id.get_mut(&acc.id.0) {
            let _ = AccountManager::release_lien(by_id, dto.amount);
        }
    } else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") })),
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
