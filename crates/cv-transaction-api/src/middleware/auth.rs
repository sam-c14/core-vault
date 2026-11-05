use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header, request::Parts, StatusCode},
    Json,
};
use cv_auth::Claims;
use serde_json::json;
use uuid::Uuid;

use crate::state::AppState;

/// Axum extractor that parses and validates a Bearer JWT token from the Authorization header.
#[derive(Clone, Debug)]
pub struct AuthClaims(pub Claims);

#[async_trait]
impl FromRequestParts<AppState> for AuthClaims {
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "status": "UNAUTHORIZED",
                        "message": "Missing Authorization header"
                    })),
                )
            })?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .or_else(|| auth_header.strip_prefix("bearer "))
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "status": "UNAUTHORIZED",
                        "message": "Invalid token scheme, expected Bearer <token>"
                    })),
                )
            })?;

        let claims = state.jwt_service.verify_token(token).map_err(|e| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "status": "UNAUTHORIZED",
                    "message": format!("Token verification failed: {e}")
                })),
            )
        })?;

        Ok(AuthClaims(claims))
    }
}

impl AuthClaims {
    pub fn claims(&self) -> &Claims {
        &self.0
    }

    pub fn user_id(&self) -> Uuid {
        self.0.sub
    }

    pub fn tenant_id(&self) -> Uuid {
        self.0.tenant_id
    }

    pub fn role(&self) -> &str {
        &self.0.role
    }

    /// Enforces that the caller has SUPER_ADMIN role.
    pub fn require_admin(&self) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
        if self.0.role == "SUPER_ADMIN" {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                Json(json!({
                    "status": "FORBIDDEN",
                    "message": "SUPER_ADMIN privileges required for this action"
                })),
            ))
        }
    }

    /// Enforces that caller is either SUPER_ADMIN or holds one of the specified staff roles.
    pub fn require_staff_role(
        &self,
        allowed_roles: &[&str],
    ) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
        if self.0.role == "SUPER_ADMIN" || allowed_roles.contains(&self.0.role.as_str()) {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                Json(json!({
                    "status": "FORBIDDEN",
                    "message": "Insufficient staff permissions for this action"
                })),
            ))
        }
    }

    /// Enforces that caller has either the required M2M scope, wildcard scope, or a matching staff role.
    pub fn require_scope_or_role(
        &self,
        required_scope: &str,
        allowed_roles: &[&str],
    ) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
        if self.0.role == "SUPER_ADMIN" || allowed_roles.contains(&self.0.role.as_str()) {
            return Ok(());
        }

        if self
            .0
            .scopes
            .iter()
            .any(|s| s == "*" || s == required_scope)
        {
            return Ok(());
        }

        Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "FORBIDDEN",
                "message": format!("Insufficient permissions: required scope '{required_scope}'")
            })),
        ))
    }
}
