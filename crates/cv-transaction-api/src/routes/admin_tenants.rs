use crate::middleware::AuthClaims;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use cv_auth::generate_client_credentials;
use cv_types::{Currency, TenantRecord, TenantStatus};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/v1/admin/tenants",
            post(onboard_tenant).get(list_tenants),
        )
        .route("/api/v1/admin/tenants/:id", get(get_tenant))
        .route(
            "/api/v1/admin/tenants/:id/whitelist",
            post(whitelist_tenant),
        )
        .route(
            "/api/v1/admin/tenants/:id/blacklist",
            post(blacklist_tenant),
        )
        .route(
            "/api/v1/admin/tenants/:id/simulate-violation",
            post(simulate_violation),
        )
        .with_state(state)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OnboardTenantDto {
    pub tenant_code: String,
    pub tenant_name: String,
    pub base_currency: Option<Currency>,
    pub allowed_ips: Option<Vec<String>>,
    pub scopes: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BlacklistTenantDto {
    pub reason: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SimulateViolationDto {
    pub violation_type: String,
    pub details: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SafeTenantView {
    pub id: Uuid,
    pub tenant_code: String,
    pub tenant_name: String,
    pub base_currency: Currency,
    pub client_id: Option<String>,
    pub status: TenantStatus,
    pub allowed_ips: Vec<String>,
    pub scopes: Vec<String>,
    pub blacklist_reason: Option<String>,
    pub violation_count: u32,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

impl From<&TenantRecord> for SafeTenantView {
    fn from(t: &TenantRecord) -> Self {
        Self {
            id: t.id,
            tenant_code: t.tenant_code.clone(),
            tenant_name: t.tenant_name.clone(),
            base_currency: t.base_currency,
            client_id: t.client_id.clone(),
            status: t.status,
            allowed_ips: t.allowed_ips.clone(),
            scopes: t.scopes.clone(),
            blacklist_reason: t.blacklist_reason.clone(),
            violation_count: t.violation_count,
            created_at: t.created_at,
            updated_at: t.updated_at,
        }
    }
}

fn parse_tenant_status(s: &str) -> TenantStatus {
    match s {
        "ACTIVE" => TenantStatus::Active,
        "BLACKLISTED" => TenantStatus::Blacklisted,
        "SUSPENDED" => TenantStatus::Suspended,
        _ => TenantStatus::Pending,
    }
}

async fn onboard_tenant(
    auth: AuthClaims,
    State(state): State<AppState>,
    Json(dto): Json<OnboardTenantDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_admin()?;

    if dto.tenant_code.trim().is_empty() || dto.tenant_name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "ERROR",
                "message": "tenant_code and tenant_name are required"
            })),
        ));
    }

    let creds = generate_client_credentials(&dto.tenant_code).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "status": "ERROR", "message": e.to_string() })),
        )
    })?;

    let tenant_id = Uuid::new_v4();
    let now = Utc::now();
    let base_currency = dto.base_currency.unwrap_or(Currency::NGN);
    let allowed_ips = dto
        .allowed_ips
        .unwrap_or_else(|| vec!["127.0.0.1".to_string()]);
    let scopes = dto.scopes.unwrap_or_else(|| {
        vec![
            "transfers:write".to_string(),
            "transfers:read".to_string(),
            "accounts:read".to_string(),
            "accounts:write".to_string(),
        ]
    });

    if let Some(pool) = &state.pool {
        sqlx::query(
            "INSERT INTO tenants (
                id, code, name, country_code, base_currency, is_active,
                client_id, client_secret_hash, status, allowed_ips, scopes,
                blacklist_reason, violation_count, created_at
            ) VALUES (
                $1, $2, $3, 'NG', $4, true,
                $5, $6, 'PENDING', $7, $8,
                NULL, 0, NOW()
            )",
        )
        .bind(tenant_id)
        .bind(dto.tenant_code.to_uppercase())
        .bind(&dto.tenant_name)
        .bind(base_currency.code())
        .bind(&creds.client_id)
        .bind(&creds.client_secret_hash)
        .bind(&allowed_ips)
        .bind(&scopes)
        .execute(pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("DB error: {e}") })),
            )
        })?;

        let audit_payload = json!({
            "tenant_code": dto.tenant_code.to_uppercase(),
            "tenant_name": dto.tenant_name,
            "client_id": creds.client_id
        });

        let _ = sqlx::query(
            "INSERT INTO audit_logs (id, tenant_id, actor_id, actor_type, action, entity_name, entity_id, payload, created_at)
             VALUES ($1, $2, $3, 'USER', 'TENANT_ONBOARDED', 'TENANT', $4, $5, NOW())"
        )
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(auth.user_id())
        .bind(tenant_id)
        .bind(audit_payload)
        .execute(pool)
        .await;
    }

    let tenant = TenantRecord {
        id: tenant_id,
        tenant_code: dto.tenant_code.to_uppercase(),
        tenant_name: dto.tenant_name,
        base_currency,
        client_id: Some(creds.client_id.clone()),
        client_secret_hash: Some(creds.client_secret_hash),
        status: TenantStatus::Pending,
        allowed_ips,
        scopes: scopes.clone(),
        blacklist_reason: None,
        violation_count: 0,
        created_at: now,
        updated_at: now,
    };

    {
        let mut tenants = state.tenants.write().unwrap();
        let mut by_client_id = state.tenants_by_client_id.write().unwrap();
        tenants.insert(tenant_id, tenant.clone());
        by_client_id.insert(creds.client_id.clone(), tenant_id);
    }

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "status": "SUCCESS",
            "message": "Tenant onboarded with PENDING status. Admin whitelisting required before API activation.",
            "tenant": {
                "id": tenant.id,
                "tenant_code": tenant.tenant_code,
                "tenant_name": tenant.tenant_name,
                "base_currency": tenant.base_currency,
                "status": "PENDING",
                "client_id": creds.client_id,
                "client_secret": creds.raw_client_secret, // Exposed only once upon creation
                "scopes": tenant.scopes
            }
        })),
    ))
}

async fn list_tenants(
    auth: AuthClaims,
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_admin()?;

    if let Some(pool) = &state.pool {
        let rows = sqlx::query(
            "SELECT id, code, name, base_currency, client_id, status, allowed_ips, scopes,
                    blacklist_reason, violation_count, created_at
             FROM tenants
             ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Database error: {e}") })),
            )
        })?;

        let list: Vec<SafeTenantView> = rows
            .iter()
            .map(|r| {
                let status_str: String = r.get("status");
                let cur_str: String = r.get("base_currency");
                let cur = match cur_str.trim() {
                    "USD" => Currency::USD,
                    "GBP" => Currency::GBP,
                    "EUR" => Currency::EUR,
                    _ => Currency::NGN,
                };
                let created: chrono::DateTime<Utc> = r.get("created_at");
                SafeTenantView {
                    id: r.get("id"),
                    tenant_code: r.get("code"),
                    tenant_name: r.get("name"),
                    base_currency: cur,
                    client_id: r.get("client_id"),
                    status: parse_tenant_status(&status_str),
                    allowed_ips: r
                        .get::<Option<Vec<String>>, _>("allowed_ips")
                        .unwrap_or_default(),
                    scopes: r.get("scopes"),
                    blacklist_reason: r.get("blacklist_reason"),
                    violation_count: r.get::<i32, _>("violation_count") as u32,
                    created_at: created,
                    updated_at: created,
                }
            })
            .collect();

        return Ok((
            StatusCode::OK,
            Json(json!({ "status": "SUCCESS", "tenants": list })),
        ));
    }

    let tenants = state.tenants.read().unwrap();
    let list: Vec<SafeTenantView> = tenants.values().map(SafeTenantView::from).collect();
    Ok((
        StatusCode::OK,
        Json(json!({ "status": "SUCCESS", "tenants": list })),
    ))
}

async fn get_tenant(
    auth: AuthClaims,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_admin()?;

    if let Some(pool) = &state.pool {
        let row_opt = sqlx::query(
            "SELECT id, code, name, base_currency, client_id, status, allowed_ips, scopes,
                    blacklist_reason, violation_count, created_at
             FROM tenants WHERE id = $1",
        )
        .bind(id)
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
                    Json(json!({ "status": "NOT_FOUND", "message": "Tenant not found" })),
                ));
            }
        };

        let status_str: String = r.get("status");
        let cur_str: String = r.get("base_currency");
        let cur = match cur_str.trim() {
            "USD" => Currency::USD,
            "GBP" => Currency::GBP,
            "EUR" => Currency::EUR,
            _ => Currency::NGN,
        };
        let created: chrono::DateTime<Utc> = r.get("created_at");
        let view = SafeTenantView {
            id: r.get("id"),
            tenant_code: r.get("code"),
            tenant_name: r.get("name"),
            base_currency: cur,
            client_id: r.get("client_id"),
            status: parse_tenant_status(&status_str),
            allowed_ips: r
                .get::<Option<Vec<String>>, _>("allowed_ips")
                .unwrap_or_default(),
            scopes: r.get("scopes"),
            blacklist_reason: r.get("blacklist_reason"),
            violation_count: r.get::<i32, _>("violation_count") as u32,
            created_at: created,
            updated_at: created,
        };

        return Ok((
            StatusCode::OK,
            Json(json!({ "status": "SUCCESS", "tenant": view })),
        ));
    }

    let tenants = state.tenants.read().unwrap();
    if let Some(tenant) = tenants.get(&id) {
        Ok((
            StatusCode::OK,
            Json(json!({
                "status": "SUCCESS",
                "tenant": SafeTenantView::from(tenant)
            })),
        ))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "status": "NOT_FOUND", "message": "Tenant not found" })),
        ))
    }
}

async fn whitelist_tenant(
    auth: AuthClaims,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_admin()?;

    if let Some(pool) = &state.pool {
        let row_opt = sqlx::query(
            "UPDATE tenants
             SET status = 'ACTIVE',
                 blacklist_reason = NULL,
                 violation_count = 0
             WHERE id = $1
             RETURNING id, code, name, base_currency, client_id, status, allowed_ips, scopes, blacklist_reason, violation_count, created_at"
        )
        .bind(id)
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
                    Json(json!({ "status": "NOT_FOUND", "message": "Tenant not found" })),
                ));
            }
        };

        let _ = sqlx::query(
            "INSERT INTO audit_logs (id, tenant_id, actor_id, actor_type, action, entity_name, entity_id, payload, created_at)
             VALUES ($1, $2, $3, 'USER', 'TENANT_WHITELISTED', 'TENANT', $4, '{}'::jsonb, NOW())"
        )
        .bind(Uuid::new_v4())
        .bind(id)
        .bind(auth.user_id())
        .bind(id)
        .execute(pool)
        .await;

        let cur_str: String = r.get("base_currency");
        let cur = match cur_str.trim() {
            "USD" => Currency::USD,
            "GBP" => Currency::GBP,
            "EUR" => Currency::EUR,
            _ => Currency::NGN,
        };
        let created: chrono::DateTime<Utc> = r.get("created_at");

        let view = SafeTenantView {
            id: r.get("id"),
            tenant_code: r.get("code"),
            tenant_name: r.get("name"),
            base_currency: cur,
            client_id: r.get("client_id"),
            status: TenantStatus::Active,
            allowed_ips: r
                .get::<Option<Vec<String>>, _>("allowed_ips")
                .unwrap_or_default(),
            scopes: r.get("scopes"),
            blacklist_reason: None,
            violation_count: 0,
            created_at: created,
            updated_at: created,
        };

        // Update in-memory state
        {
            let mut tenants = state.tenants.write().unwrap();
            if let Some(tenant) = tenants.get_mut(&id) {
                tenant.whitelist();
            }
        }

        return Ok((
            StatusCode::OK,
            Json(json!({
                "status": "SUCCESS",
                "message": "Tenant successfully whitelisted and activated",
                "tenant": view
            })),
        ));
    }

    let mut tenants = state.tenants.write().unwrap();
    if let Some(tenant) = tenants.get_mut(&id) {
        tenant.whitelist();
        Ok((
            StatusCode::OK,
            Json(json!({
                "status": "SUCCESS",
                "message": "Tenant successfully whitelisted and activated",
                "tenant": SafeTenantView::from(&*tenant)
            })),
        ))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "status": "NOT_FOUND", "message": "Tenant not found" })),
        ))
    }
}

async fn blacklist_tenant(
    auth: AuthClaims,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<BlacklistTenantDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_admin()?;

    if let Some(pool) = &state.pool {
        let row_opt = sqlx::query(
            "UPDATE tenants
             SET status = 'BLACKLISTED',
                 blacklist_reason = $1
             WHERE id = $2
             RETURNING id, code, name, base_currency, client_id, status, allowed_ips, scopes, blacklist_reason, violation_count, created_at"
        )
        .bind(&dto.reason)
        .bind(id)
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
                    Json(json!({ "status": "NOT_FOUND", "message": "Tenant not found" })),
                ));
            }
        };

        let audit_payload = json!({ "reason": dto.reason });
        let _ = sqlx::query(
            "INSERT INTO audit_logs (id, tenant_id, actor_id, actor_type, action, entity_name, entity_id, payload, created_at)
             VALUES ($1, $2, $3, 'USER', 'TENANT_BLACKLISTED', 'TENANT', $4, $5, NOW())"
        )
        .bind(Uuid::new_v4())
        .bind(id)
        .bind(auth.user_id())
        .bind(id)
        .bind(audit_payload)
        .execute(pool)
        .await;

        let cur_str: String = r.get("base_currency");
        let cur = match cur_str.trim() {
            "USD" => Currency::USD,
            "GBP" => Currency::GBP,
            "EUR" => Currency::EUR,
            _ => Currency::NGN,
        };
        let created: chrono::DateTime<Utc> = r.get("created_at");

        let view = SafeTenantView {
            id: r.get("id"),
            tenant_code: r.get("code"),
            tenant_name: r.get("name"),
            base_currency: cur,
            client_id: r.get("client_id"),
            status: TenantStatus::Blacklisted,
            allowed_ips: r
                .get::<Option<Vec<String>>, _>("allowed_ips")
                .unwrap_or_default(),
            scopes: r.get("scopes"),
            blacklist_reason: Some(dto.reason),
            violation_count: r.get::<i32, _>("violation_count") as u32,
            created_at: created,
            updated_at: created,
        };

        // Update in-memory state
        {
            let mut tenants = state.tenants.write().unwrap();
            if let Some(tenant) = tenants.get_mut(&id) {
                tenant.blacklist(&view.blacklist_reason.clone().unwrap_or_default());
            }
        }

        return Ok((
            StatusCode::OK,
            Json(json!({
                "status": "SUCCESS",
                "message": "Tenant blacklisted",
                "tenant": view
            })),
        ));
    }

    let mut tenants = state.tenants.write().unwrap();
    if let Some(tenant) = tenants.get_mut(&id) {
        tenant.blacklist(&dto.reason);
        Ok((
            StatusCode::OK,
            Json(json!({
                "status": "SUCCESS",
                "message": "Tenant blacklisted",
                "tenant": SafeTenantView::from(&*tenant)
            })),
        ))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "status": "NOT_FOUND", "message": "Tenant not found" })),
        ))
    }
}

async fn simulate_violation(
    auth: AuthClaims,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<SimulateViolationDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_admin()?;

    let violation_text = format!("{}: {}", dto.violation_type, dto.details);

    if let Some(pool) = &state.pool {
        let row_opt = sqlx::query(
            "UPDATE tenants
             SET status = 'BLACKLISTED',
                 blacklist_reason = $1,
                 violation_count = violation_count + 1
             WHERE id = $2
             RETURNING id, code, name, base_currency, client_id, status, allowed_ips, scopes, blacklist_reason, violation_count, created_at"
        )
        .bind(&violation_text)
        .bind(id)
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
                    Json(json!({ "status": "NOT_FOUND", "message": "Tenant not found" })),
                ));
            }
        };

        let audit_payload = json!({
            "violation_type": dto.violation_type,
            "details": dto.details
        });

        let _ = sqlx::query(
            "INSERT INTO audit_logs (id, tenant_id, actor_id, actor_type, action, entity_name, entity_id, payload, created_at)
             VALUES ($1, $2, $3, 'SYSTEM', 'TENANT_VIOLATION_TRIGGERED', 'TENANT', $4, $5, NOW())"
        )
        .bind(Uuid::new_v4())
        .bind(id)
        .bind(auth.user_id())
        .bind(id)
        .bind(audit_payload)
        .execute(pool)
        .await;

        let cur_str: String = r.get("base_currency");
        let cur = match cur_str.trim() {
            "USD" => Currency::USD,
            "GBP" => Currency::GBP,
            "EUR" => Currency::EUR,
            _ => Currency::NGN,
        };
        let created: chrono::DateTime<Utc> = r.get("created_at");

        let view = SafeTenantView {
            id: r.get("id"),
            tenant_code: r.get("code"),
            tenant_name: r.get("name"),
            base_currency: cur,
            client_id: r.get("client_id"),
            status: TenantStatus::Blacklisted,
            allowed_ips: r
                .get::<Option<Vec<String>>, _>("allowed_ips")
                .unwrap_or_default(),
            scopes: r.get("scopes"),
            blacklist_reason: Some(violation_text.clone()),
            violation_count: r.get::<i32, _>("violation_count") as u32,
            created_at: created,
            updated_at: created,
        };

        // Update in-memory state
        {
            let mut tenants = state.tenants.write().unwrap();
            if let Some(tenant) = tenants.get_mut(&id) {
                tenant.record_violation(&violation_text);
            }
        }

        return Ok((
            StatusCode::OK,
            Json(json!({
                "status": "BLACKLISTED",
                "message": "Platform automated blacklist triggered due to platform rule violation. Client credentials revoked.",
                "tenant": view
            })),
        ));
    }

    let mut tenants = state.tenants.write().unwrap();
    if let Some(tenant) = tenants.get_mut(&id) {
        tenant.record_violation(&violation_text);
        Ok((
            StatusCode::OK,
            Json(json!({
                "status": "BLACKLISTED",
                "message": "Platform automated blacklist triggered due to platform rule violation. Client credentials revoked.",
                "tenant": SafeTenantView::from(&*tenant)
            })),
        ))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "status": "NOT_FOUND", "message": "Tenant not found" })),
        ))
    }
}
