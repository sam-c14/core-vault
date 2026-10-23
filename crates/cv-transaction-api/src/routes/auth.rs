use crate::state::AppState;
use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use cv_auth::{hash_password, verify_client_secret, verify_password};
use cv_types::TenantStatus;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
    if dto.email.is_empty() || dto.password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "status": "ERROR", "message": "Email and password are required" })),
        ));
    }

    let sample_hash = hash_password("AdminSecurePass#2026").map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "status": "ERROR", "message": e.to_string() })),
        )
    })?;

    if verify_password(&dto.password, &sample_hash).is_err() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "status": "UNAUTHORIZED", "message": "Invalid email or password" })),
        ));
    }

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::parse_str("a0000000-0000-0000-0000-000000000001").unwrap();
    let role = "OPERATIONS_OFFICER";

    let token = state
        .jwt_service
        .generate_token(user_id, tenant_id, role)
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

async fn get_current_user(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(
                    json!({ "status": "UNAUTHORIZED", "message": "Missing Authorization header" }),
                ),
            )
        })?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .or_else(|| auth_header.strip_prefix("bearer "))
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "status": "UNAUTHORIZED", "message": "Invalid token scheme, expected Bearer" })),
            )
        })?;

    let claims = state.jwt_service.verify_token(token).map_err(|e| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "status": "UNAUTHORIZED", "message": e.to_string() })),
        )
    })?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "SUCCESS",
            "claims": {
                "user_id": claims.sub,
                "tenant_id": claims.tenant_id,
                "role": claims.role,
                "scopes": claims.scopes,
                "exp": claims.exp
            }
        })),
    ))
}
