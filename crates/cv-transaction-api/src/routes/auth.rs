use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use cv_auth::{hash_password, verify_password, JwtService};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct AuthState {
    pub jwt_service: Arc<JwtService>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginDto {
    pub email: String,
    pub password: String,
}

pub fn router(jwt_service: Arc<JwtService>) -> Router {
    let state = AuthState { jwt_service };
    Router::new()
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/me", get(get_current_user))
        .with_state(state)
}

async fn login(
    State(state): State<AuthState>,
    Json(dto): Json<LoginDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    // Demonstration/MVP: Verify standard credentials or seed admin
    if dto.email.is_empty() || dto.password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "status": "ERROR", "message": "Email and password are required" })),
        ));
    }

    // Demo password verification using Argon2id
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

async fn get_current_user(
    State(state): State<AuthState>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "status": "UNAUTHORIZED", "message": "Missing Authorization header" })),
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
                "exp": claims.exp
            }
        })),
    ))
}
