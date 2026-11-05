use crate::middleware::{AuthClaims, TenantHeader};
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use cv_account_manager::{AccountManager, CreateAccountDto};
use cv_types::{AccountStatus, AccountType};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/accounts", post(create_account).get(list_accounts))
        .route("/api/v1/accounts/:nuban", get(get_account))
        .with_state(state)
}

async fn list_accounts(
    auth: AuthClaims,
    tenant_hdr: TenantHeader,
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_scope_or_role(
        "accounts:read",
        &[
            "SUPER_ADMIN",
            "OPERATIONS_OFFICER",
            "COMPLIANCE_OFFICER",
            "BRANCH_MANAGER",
            "AUDITOR",
        ],
    )?;

    let target_tenant = if auth.role() == "SUPER_ADMIN" {
        tenant_hdr.id()
    } else {
        Some(tenant_hdr.resolve_for(&auth)?)
    };

    if let Some(pool) = &state.pool {
        let rows = if let Some(tid) = target_tenant {
            sqlx::query(
                "SELECT id, tenant_id, customer_id, account_number, account_name, gl_code, account_type,
                        currency_code, ledger_balance, available_balance, lien_amount, overdraft_limit,
                        status, pnd_source, pnd_reason, version, created_at, updated_at
                 FROM accounts
                 WHERE tenant_id = $1
                 ORDER BY created_at DESC"
            )
            .bind(tid)
            .fetch_all(pool)
            .await
        } else {
            sqlx::query(
                "SELECT id, tenant_id, customer_id, account_number, account_name, gl_code, account_type,
                        currency_code, ledger_balance, available_balance, lien_amount, overdraft_limit,
                        status, pnd_source, pnd_reason, version, created_at, updated_at
                 FROM accounts
                 ORDER BY created_at DESC"
            )
            .fetch_all(pool)
            .await
        }
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Database error: {e}") })),
            )
        })?;

        let list: Vec<Value> = rows
            .iter()
            .map(|r| {
                json!({
                    "id": r.get::<Uuid, _>("id"),
                    "tenant_id": r.get::<Uuid, _>("tenant_id"),
                    "customer_id": r.get::<Option<Uuid>, _>("customer_id"),
                    "account_number": r.get::<String, _>("account_number"),
                    "account_name": r.get::<String, _>("account_name"),
                    "gl_code": r.get::<String, _>("gl_code"),
                    "account_type": r.get::<String, _>("account_type"),
                    "currency": r.get::<String, _>("currency_code").trim(),
                    "ledger_balance": r.get::<Decimal, _>("ledger_balance"),
                    "available_balance": r.get::<Decimal, _>("available_balance"),
                    "lien_amount": r.get::<Decimal, _>("lien_amount"),
                    "overdraft_limit": r.get::<Decimal, _>("overdraft_limit"),
                    "status": r.get::<String, _>("status"),
                    "pnd_source": r.get::<Option<String>, _>("pnd_source"),
                    "pnd_reason": r.get::<Option<String>, _>("pnd_reason"),
                    "version": r.get::<i64, _>("version")
                })
            })
            .collect();

        return Ok((
            StatusCode::OK,
            Json(json!({
                "status": "SUCCESS",
                "accounts": list
            })),
        ));
    }

    let accounts = state.accounts.read().unwrap();
    let list: Vec<_> = if let Some(tid) = target_tenant {
        accounts
            .values()
            .filter(|a| a.tenant_id == tid)
            .cloned()
            .collect()
    } else {
        accounts.values().cloned().collect()
    };
    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "SUCCESS",
            "accounts": list
        })),
    ))
}

async fn create_account(
    auth: AuthClaims,
    tenant_hdr: TenantHeader,
    State(state): State<AppState>,
    Json(dto): Json<CreateAccountDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_scope_or_role("accounts:write", &["SUPER_ADMIN", "OPERATIONS_OFFICER"])?;

    let effective_tenant_id = if auth.role() == "SUPER_ADMIN" {
        tenant_hdr
            .id()
            .or(dto.tenant_id)
            .unwrap_or(auth.tenant_id())
    } else {
        tenant_hdr.resolve_for(&auth)?
    };

    let account =
        AccountManager::build_new_account_with_tenant(dto, effective_tenant_id).map_err(|err| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "status": "ERROR",
                    "message": err.to_string()
                })),
            )
        })?;

    if let Some(pool) = &state.pool {
        let mut tx = pool.begin().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Transaction start failed: {e}") })),
            )
        })?;

        let acc_type_str = match account.account_type {
            AccountType::Savings => "SAVINGS",
            AccountType::Current => "CURRENT",
            AccountType::Domiciliary => "DOMICILIARY",
            AccountType::Loan => "LOAN",
            AccountType::InternalGl => "INTERNAL_GL",
            AccountType::Escrow => "ESCROW",
        };

        let status_str = match account.status {
            AccountStatus::Active => "ACTIVE",
            AccountStatus::PostNoDebit => "POST_NO_DEBIT",
            AccountStatus::Frozen => "FROZEN",
            AccountStatus::Dormant => "DORMANT",
            AccountStatus::Closed => "CLOSED",
            _ => "PENDING_OPENING",
        };

        sqlx::query(
            "INSERT INTO accounts (
                id, tenant_id, customer_id, account_number, account_name,
                gl_code, account_type, currency_code, ledger_balance, available_balance,
                lien_amount, overdraft_limit, status, version, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, NOW(), NOW())"
        )
        .bind(account.id.0)
        .bind(account.tenant_id)
        .bind(account.customer_id)
        .bind(&account.account_number)
        .bind(&account.account_name)
        .bind(&account.gl_code)
        .bind(acc_type_str)
        .bind(account.currency.code())
        .bind(account.ledger_balance)
        .bind(account.available_balance)
        .bind(account.lien_amount)
        .bind(account.overdraft_limit)
        .bind(status_str)
        .bind(account.version)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Failed to insert account into DB: {e}") })),
            )
        })?;

        let audit_payload = json!({
            "account_number": account.account_number,
            "account_name": account.account_name,
            "account_type": acc_type_str,
            "currency": account.currency.code(),
            "gl_code": account.gl_code
        });

        let _ = sqlx::query(
            "INSERT INTO audit_logs (id, tenant_id, actor_id, actor_type, action, entity_name, entity_id, payload, created_at)
             VALUES ($1, $2, $3, 'USER', 'CREATE_ACCOUNT', 'ACCOUNT', $4, $5, NOW())"
        )
        .bind(Uuid::new_v4())
        .bind(account.tenant_id)
        .bind(auth.user_id())
        .bind(account.id.0)
        .bind(audit_payload)
        .execute(&mut *tx)
        .await;

        tx.commit().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Transaction commit failed: {e}") })),
            )
        })?;
    }

    // Also update in-memory state for synchronous references
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

async fn get_account(
    auth: AuthClaims,
    State(state): State<AppState>,
    Path(nuban): Path<String>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_scope_or_role(
        "accounts:read",
        &[
            "SUPER_ADMIN",
            "OPERATIONS_OFFICER",
            "COMPLIANCE_OFFICER",
            "BRANCH_MANAGER",
            "AUDITOR",
        ],
    )?;

    if let Some(pool) = &state.pool {
        let row_opt = sqlx::query(
            "SELECT id, tenant_id, customer_id, account_number, account_name, gl_code, account_type,
                    currency_code, ledger_balance, available_balance, lien_amount, overdraft_limit,
                    status, pnd_source, pnd_reason, version, created_at, updated_at
             FROM accounts
             WHERE account_number = $1"
        )
        .bind(&nuban)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Database error: {e}") })),
            )
        })?;

        let r = match row_opt {
            Some(row) => row,
            None => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(
                        json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                    ),
                ));
            }
        };

        let row_tenant_id: Uuid = r.get("tenant_id");
        if auth.role() != "SUPER_ADMIN" && row_tenant_id != auth.tenant_id() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(
                    json!({ "status": "NOT_FOUND", "message": format!("Account {nuban} not found") }),
                ),
            ));
        }

        let ledger: Decimal = r.get("ledger_balance");
        let available: Decimal = r.get("available_balance");
        let overdraft: Decimal = r.get("overdraft_limit");
        let lien: Decimal = r.get("lien_amount");
        let effective = available + overdraft - lien;

        return Ok((
            StatusCode::OK,
            Json(json!({
                "account_number": r.get::<String, _>("account_number"),
                "account_name": r.get::<String, _>("account_name"),
                "status": r.get::<String, _>("status"),
                "pnd_source": r.get::<Option<String>, _>("pnd_source"),
                "pnd_reason": r.get::<Option<String>, _>("pnd_reason"),
                "ledger_balance": ledger,
                "available_balance": available,
                "lien_amount": lien,
                "overdraft_limit": overdraft,
                "effective_available_funds": effective
            })),
        ));
    }

    let accounts = state.accounts.read().unwrap();
    if let Some(acc) = accounts.get(&nuban) {
        if auth.role() != "SUPER_ADMIN" && acc.tenant_id != auth.tenant_id() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(json!({
                    "status": "NOT_FOUND",
                    "message": format!("Account {nuban} not found")
                })),
            ));
        }

        Ok((
            StatusCode::OK,
            Json(json!({
                "account_number": acc.account_number,
                "account_name": acc.account_name,
                "status": acc.status,
                "pnd_source": acc.pnd_source,
                "pnd_reason": acc.pnd_reason,
                "ledger_balance": acc.ledger_balance,
                "available_balance": acc.available_balance,
                "lien_amount": acc.lien_amount,
                "overdraft_limit": acc.overdraft_limit,
                "effective_available_funds": acc.effective_available_funds()
            })),
        ))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "status": "NOT_FOUND",
                "message": format!("Account {nuban} not found")
            })),
        ))
    }
}
