use crate::middleware::AuthClaims;
use crate::state::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
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
        .route("/api/v1/auth/client-login", post(client_login))
        .route("/api/v1/auth/token", post(m2m_token))
        .route("/api/v1/auth/me", get(get_current_user))
        .with_state(state)
}

type BasicAuthResult = Result<Option<(String, String)>, (StatusCode, Json<Value>)>;

fn extract_basic_auth(headers: &HeaderMap) -> BasicAuthResult {
    if let Some(auth_val) = headers.get(axum::http::header::AUTHORIZATION) {
        let auth_str = auth_val.to_str().map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "status": "ERROR",
                    "message": "Invalid Authorization header: non-ASCII characters"
                })),
            )
        })?;

        let trimmed = auth_str.trim();
        if trimmed.to_ascii_lowercase().starts_with("basic ") {
            let encoded = trimmed[6..].trim();
            let decoded_bytes = BASE64.decode(encoded).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "status": "ERROR",
                        "message": "Invalid Base64 encoding in Authorization header"
                    })),
                )
            })?;
            let decoded_str = String::from_utf8(decoded_bytes).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "status": "ERROR",
                        "message": "Authorization header is not valid UTF-8"
                    })),
                )
            })?;
            let mut parts = decoded_str.splitn(2, ':');
            let client_id = parts.next().unwrap_or("").trim().to_string();
            let client_secret = parts.next().unwrap_or("").trim().to_string();
            if client_id.is_empty() || client_secret.is_empty() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "status": "ERROR",
                        "message": "Invalid Basic Auth: format must be client_id:client_secret"
                    })),
                ));
            }
            return Ok(Some((client_id, client_secret)));
        }
    }
    Ok(None)
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(dto): Json<LoginDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    if let Some((client_id, client_secret)) = extract_basic_auth(&headers)? {
        return handle_client_login(&state, &client_id, &client_secret, &dto).await;
    }
    handle_staff_login(&state, &dto).await
}

async fn client_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(dto): Json<LoginDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let (client_id, client_secret) = extract_basic_auth(&headers)?.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "status": "UNAUTHORIZED",
                "message": "Missing or invalid Basic Auth Authorization header (expected Basic base64(client_id:client_secret))"
            })),
        )
    })?;

    handle_client_login(&state, &client_id, &client_secret, &dto).await
}

async fn handle_client_login(
    state: &AppState,
    client_id: &str,
    client_secret: &str,
    dto: &LoginDto,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    if dto.email.trim().is_empty() || dto.password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "status": "ERROR", "message": "Email and password are required" })),
        ));
    }

    if let Some(pool) = &state.pool {
        // 1. Verify tenant credentials
        let tenant_row = sqlx::query(
            "SELECT id, status, client_secret_hash, blacklist_reason, violation_count
             FROM tenants WHERE client_id = $1",
        )
        .bind(client_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Database error: {e}") })),
            )
        })?;

        let trow = match tenant_row {
            Some(r) => r,
            None => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "status": "UNAUTHORIZED",
                        "message": "Invalid client credentials in Authorization header"
                    })),
                ));
            }
        };

        let stored_hash: Option<String> = trow.get("client_secret_hash");
        let hash_str = stored_hash.as_deref().unwrap_or("");
        if verify_client_secret(client_secret, hash_str).is_err() {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "status": "UNAUTHORIZED",
                    "message": "Invalid client credentials in Authorization header"
                })),
            ));
        }

        let tenant_status: String = trow.get("status");
        let tenant_id: Uuid = trow.get("id");
        let blacklist_reason: Option<String> = trow.get("blacklist_reason");
        let violation_count: i32 = trow.get("violation_count");

        match tenant_status.as_str() {
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
            "ACTIVE" => {}
            _ => {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "status": "FORBIDDEN",
                        "error": "Tenant is not active"
                    })),
                ));
            }
        }

        // 2. Verify business user scoped strictly to this specific tenant
        let user_row = sqlx::query(
            "SELECT id, tenant_id, email, password_hash, first_name, last_name, role, is_active
             FROM users
             WHERE email = $1 AND tenant_id = $2",
        )
        .bind(dto.email.trim())
        .bind(tenant_id)
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
                    Json(json!({
                        "status": "UNAUTHORIZED",
                        "message": "Invalid email or password for this business tenant"
                    })),
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

        let user_pw_hash: String = user.get("password_hash");
        if verify_password(&dto.password, &user_pw_hash).is_err() {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "status": "UNAUTHORIZED",
                    "message": "Invalid email or password for this business tenant"
                })),
            ));
        }

        let user_id: Uuid = user.get("id");
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
                "tenant_id": tenant_id,
                "user": {
                    "id": user_id,
                    "email": email,
                    "role": role,
                    "tenant_id": tenant_id
                }
            })),
        ));
    }

    // In-memory fallback if no DB connected
    let tenant_id = {
        let by_client = state.tenants_by_client_id.read().unwrap();
        match by_client.get(client_id) {
            Some(id) => *id,
            None => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "status": "UNAUTHORIZED",
                        "message": "Invalid client credentials in Authorization header"
                    })),
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
                    Json(json!({
                        "status": "UNAUTHORIZED",
                        "message": "Invalid client credentials in Authorization header"
                    })),
                ));
            }
        }
    };

    let stored_hash = tenant.client_secret_hash.as_deref().unwrap_or("");
    if verify_client_secret(client_secret, stored_hash).is_err() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "status": "UNAUTHORIZED",
                "message": "Invalid client credentials in Authorization header"
            })),
        ));
    }

    match tenant.status {
        TenantStatus::Blacklisted => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(json!({
                    "status": "FORBIDDEN",
                    "error": format!("Client/Tenant is blacklisted: {}", tenant.blacklist_reason.as_deref().unwrap_or("Platform policy breach.")),
                    "violation_count": tenant.violation_count
                })),
            ));
        }
        TenantStatus::Pending => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(json!({
                    "status": "FORBIDDEN",
                    "error": "Client/Tenant onboarding is pending admin approval and whitelisting"
                })),
            ));
        }
        TenantStatus::Suspended => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(json!({
                    "status": "FORBIDDEN",
                    "error": "Client/Tenant account is suspended"
                })),
            ));
        }
        TenantStatus::Active => {}
    }

    let default_hash = "$argon2id$v=19$m=19456,t=2,p=1$2EKBk2ShZ0l4lU6lRN++jA$1Uew0+X9Vma66STHRqQ98bv5+z2/4WpHMXf4o+ClKBY";
    if verify_password(&dto.password, default_hash).is_err() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "status": "UNAUTHORIZED",
                "message": "Invalid email or password for this business tenant"
            })),
        ));
    }

    let user_id = Uuid::new_v4();
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
            "tenant_id": tenant_id,
            "user": {
                "id": user_id,
                "email": dto.email,
                "role": role,
                "tenant_id": tenant_id
            }
        })),
    ))
}

async fn handle_staff_login(
    state: &AppState,
    dto: &LoginDto,
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
