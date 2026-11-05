use crate::middleware::AuthClaims;
use crate::state::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use cv_auth::{verify_client_secret, verify_password};
use cv_types::TenantStatus;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginDto {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClientTokenRequestDto {
    pub client_id: String,
    pub client_secret: String,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/token", post(m2m_token))
        .route("/api/v1/auth/me", get(get_current_user))
        .with_state(state)
}

async fn login(
    State(state): State<AppState>,
    Json(dto): Json<LoginDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    if dto.email.trim().is_empty() || dto.password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "status": "ERROR", "message": "Email and password are required" })),
        ));
    }

    if let Some(pool) = &state.pool {
        let user_row = sqlx::query(
            "SELECT id, tenant_id, email, password_hash, first_name, last_name, role, is_active FROM users WHERE email = $1"
        )
        .bind(dto.email.trim())
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Database error: {e}") })),
            )
        })?;

        let user = match user_row {
            Some(u) => u,
            None => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(
                        json!({ "status": "UNAUTHORIZED", "message": "Invalid email or password" }),
                    ),
                ));
            }
        };

        let is_active: bool = user.get("is_active");
        if !is_active {
            return Err((
                StatusCode::FORBIDDEN,
                Json(json!({ "status": "FORBIDDEN", "message": "User account is inactive" })),
            ));
        }

        let stored_hash: String = user.get("password_hash");
        if verify_password(&dto.password, &stored_hash).is_err() {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({ "status": "UNAUTHORIZED", "message": "Invalid email or password" })),
            ));
        }

        let user_id: Uuid = user.get("id");
        let tenant_id: Uuid = user.get("tenant_id");
        let role: String = user.get("role");
        let email: String = user.get("email");

        let token = state
            .jwt_service
            .generate_token(user_id, tenant_id, &role)
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "status": "ERROR", "message": e.to_string() })),
                )
            })?;

        return Ok((
            StatusCode::OK,
            Json(json!({
                "status": "SUCCESS",
                "token": token,
                "token_type": "Bearer",
                "expires_in_seconds": 900,
                "user": {
                    "id": user_id,
                    "email": email,
                    "role": role,
                    "tenant_id": tenant_id
                }
            })),
        ));
    }

    // Fallback for standalone in-memory test harnesses (when DATABASE_URL is unavailable)
    if dto.email.trim() == "admin@corevault.bank" {
        let admin_hash = "$argon2id$v=19$m=19456,t=2,p=1$2EKBk2ShZ0l4lU6lRN++jA$1Uew0+X9Vma66STHRqQ98bv5+z2/4WpHMXf4o+ClKBY";
        if verify_password(&dto.password, admin_hash).is_err() {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({ "status": "UNAUTHORIZED", "message": "Invalid email or password" })),
            ));
        }

        let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let tenant_id = Uuid::parse_str("a0000000-0000-0000-0000-000000000001").unwrap();
        let role = "SUPER_ADMIN";

        let token = state
            .jwt_service
            .generate_token(user_id, tenant_id, role)
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "status": "ERROR", "message": e.to_string() })),
                )
            })?;

        return Ok((
            StatusCode::OK,
            Json(json!({
                "status": "SUCCESS",
                "token": token,
                "token_type": "Bearer",
                "expires_in_seconds": 900,
                "user": {
                    "id": user_id,
                    "email": dto.email,
                    "role": role,
                    "tenant_id": tenant_id
                }
            })),
        ));
    }

    Err((
        StatusCode::UNAUTHORIZED,
        Json(json!({ "status": "UNAUTHORIZED", "message": "Invalid email or password" })),
    ))
}

async fn m2m_token(
    State(state): State<AppState>,
    Json(dto): Json<ClientTokenRequestDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    if dto.client_id.trim().is_empty() || dto.client_secret.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                json!({ "status": "ERROR", "message": "client_id and client_secret are required" }),
            ),
        ));
    }

    if let Some(pool) = &state.pool {
        let tenant_row = sqlx::query(
            "SELECT id, code, name, client_id, client_secret_hash, status, allowed_ips, scopes, blacklist_reason, violation_count FROM tenants WHERE client_id = $1"
        )
        .bind(dto.client_id.trim())
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Database error: {e}") })),
            )
        })?;

        let row = match tenant_row {
            Some(r) => r,
            None => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(
                        json!({ "status": "UNAUTHORIZED", "message": "Invalid client_id or client_secret" }),
                    ),
                ));
            }
        };

        let stored_hash: Option<String> = row.get("client_secret_hash");
        let hash_str = stored_hash.as_deref().unwrap_or("");
        if verify_client_secret(&dto.client_secret, hash_str).is_err() {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(
                    json!({ "status": "UNAUTHORIZED", "message": "Invalid client_id or client_secret" }),
                ),
            ));
        }

        let status_str: String = row.get("status");
        let tenant_id: Uuid = row.get("id");
        let scopes: Vec<String> = row.get("scopes");
        let blacklist_reason: Option<String> = row.get("blacklist_reason");
        let violation_count: i32 = row.get("violation_count");

        match status_str.as_str() {
            "BLACKLISTED" => {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "status": "FORBIDDEN",
                        "error": format!("Client/Tenant is blacklisted: {}", blacklist_reason.as_deref().unwrap_or("Platform policy breach. Contact CBA Super Admin to re-whitelist.")),
                        "violation_count": violation_count
                    })),
                ));
            }
            "PENDING" => {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "status": "FORBIDDEN",
                        "error": "Client/Tenant onboarding is pending admin approval and whitelisting"
                    })),
                ));
            }
            "SUSPENDED" => {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "status": "FORBIDDEN",
                        "error": "Client/Tenant account is suspended"
                    })),
                ));
            }
            "ACTIVE" => {
                let token = state
                    .jwt_service
                    .generate_m2m_token(tenant_id, tenant_id, &scopes)
                    .map_err(|e| {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({ "status": "ERROR", "message": e.to_string() })),
                        )
                    })?;

                return Ok((
                    StatusCode::OK,
                    Json(json!({
                        "status": "SUCCESS",
                        "access_token": token,
                        "token_type": "Bearer",
                        "expires_in_seconds": 3600,
                        "tenant_id": tenant_id,
                        "scopes": scopes
                    })),
                ));
            }
            _ => {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(json!({ "status": "FORBIDDEN", "error": "Unknown tenant status" })),
                ));
            }
        }
    }

    // In-memory fallback if no DB connected
    let tenant_id = {
        let by_client = state.tenants_by_client_id.read().unwrap();
        match by_client.get(&dto.client_id) {
            Some(id) => *id,
            None => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(
                        json!({ "status": "UNAUTHORIZED", "message": "Invalid client_id or client_secret" }),
                    ),
                ));
            }
        }
    };

    let tenant = {
        let tenants = state.tenants.read().unwrap();
        match tenants.get(&tenant_id) {
            Some(t) => t.clone(),
            None => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(
                        json!({ "status": "UNAUTHORIZED", "message": "Invalid client_id or client_secret" }),
                    ),
                ));
            }
        }
    };

    let stored_hash = tenant.client_secret_hash.as_deref().unwrap_or("");
    if verify_client_secret(&dto.client_secret, stored_hash).is_err() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(
                json!({ "status": "UNAUTHORIZED", "message": "Invalid client_id or client_secret" }),
            ),
        ));
    }

    match tenant.status {
        TenantStatus::Blacklisted => Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "FORBIDDEN",
                "error": format!("Client/Tenant is blacklisted: {}", tenant.blacklist_reason.as_deref().unwrap_or("Platform policy breach. Contact CBA Super Admin to re-whitelist.")),
                "violation_count": tenant.violation_count
            })),
        )),
        TenantStatus::Pending => Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "FORBIDDEN",
                "error": "Client/Tenant onboarding is pending admin approval and whitelisting"
            })),
        )),
        TenantStatus::Suspended => Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "FORBIDDEN",
                "error": "Client/Tenant account is suspended"
            })),
        )),
        TenantStatus::Active => {
            let token = state
                .jwt_service
                .generate_m2m_token(tenant.id, tenant.id, &tenant.scopes)
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "status": "ERROR", "message": e.to_string() })),
                    )
                })?;

            Ok((
                StatusCode::OK,
                Json(json!({
                    "status": "SUCCESS",
                    "access_token": token,
                    "token_type": "Bearer",
                    "expires_in_seconds": 3600,
                    "tenant_id": tenant.id,
                    "scopes": tenant.scopes
                })),
            ))
        }
    }
}

async fn get_current_user(claims: AuthClaims) -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "SUCCESS",
            "claims": {
                "user_id": claims.0.sub,
                "tenant_id": claims.0.tenant_id,
                "role": claims.0.role,
                "scopes": claims.0.scopes,
                "exp": claims.0.exp
            }
        })),
    )
}
