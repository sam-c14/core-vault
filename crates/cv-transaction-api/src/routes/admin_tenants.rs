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

#[derive(Debug, Serialize)]
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

async fn onboard_tenant(
    State(state): State<AppState>,
    Json(dto): Json<OnboardTenantDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
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
    let tenant = TenantRecord {
        id: tenant_id,
        tenant_code: dto.tenant_code.to_uppercase(),
        tenant_name: dto.tenant_name,
        base_currency: dto.base_currency.unwrap_or(Currency::NGN),
        client_id: Some(creds.client_id.clone()),
        client_secret_hash: Some(creds.client_secret_hash),
        status: TenantStatus::Pending,
        allowed_ips: dto
            .allowed_ips
            .unwrap_or_else(|| vec!["127.0.0.1".to_string()]),
        scopes: dto.scopes.unwrap_or_else(|| {
            vec![
                "transfers:write".to_string(),
                "accounts:read".to_string(),
                "accounts:write".to_string(),
            ]
        }),
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

async fn list_tenants(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let tenants = state.tenants.read().unwrap();
    let list: Vec<SafeTenantView> = tenants.values().map(SafeTenantView::from).collect();
    (
        StatusCode::OK,
        Json(json!({ "status": "SUCCESS", "tenants": list })),
    )
}

async fn get_tenant(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
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
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
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
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<BlacklistTenantDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
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
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<SimulateViolationDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let mut tenants = state.tenants.write().unwrap();
    if let Some(tenant) = tenants.get_mut(&id) {
        tenant.record_violation(&format!("{}: {}", dto.violation_type, dto.details));
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
