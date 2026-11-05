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

/// Axum extractor that reads the optional `x-tenant-id` header from HTTP request parts.
#[derive(Clone, Copy, Debug, Default)]
pub struct TenantHeader(pub Option<Uuid>);

#[async_trait]
impl<S> FromRequestParts<S> for TenantHeader
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(header_val) = parts.headers.get("x-tenant-id") {
            let str_val = header_val.to_str().map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "status": "ERROR",
                        "message": "Invalid x-tenant-id header: not valid ASCII/UTF-8"
                    })),
                )
            })?;
            let id = Uuid::parse_str(str_val.trim()).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "status": "ERROR",
                        "message": "Invalid x-tenant-id header: expected valid UUID"
                    })),
                )
            })?;
            Ok(TenantHeader(Some(id)))
        } else {
            Ok(TenantHeader(None))
        }
    }
}

impl TenantHeader {
    pub fn id(&self) -> Option<Uuid> {
        self.0
    }

    /// Resolves the effective tenant ID given the caller's AuthClaims.
    ///
    /// - If the caller is `SUPER_ADMIN` and `x-tenant-id` header is present, returns the header's tenant ID.
    /// - If the caller is `SUPER_ADMIN` and `x-tenant-id` header is absent, returns `auth.tenant_id()`.
    /// - If the caller is NOT `SUPER_ADMIN`:
    ///   - If `x-tenant-id` is provided and differs from `auth.tenant_id()`, returns a 403 FORBIDDEN error.
    ///   - Otherwise, returns `auth.tenant_id()`.
    pub fn resolve_for(
        &self,
        auth: &AuthClaims,
    ) -> Result<Uuid, (StatusCode, Json<serde_json::Value>)> {
        match self.0 {
            Some(header_tenant) => {
                if auth.role() != "SUPER_ADMIN" && auth.tenant_id() != header_tenant {
                    return Err((
                        StatusCode::FORBIDDEN,
                        Json(json!({
                            "status": "FORBIDDEN",
                            "message": "x-tenant-id header does not match authenticated tenant"
                        })),
                    ));
                }
                Ok(header_tenant)
            }
            None => Ok(auth.tenant_id()),
        }
    }

    /// Requires the `x-tenant-id` header to be explicitly present and valid.
    pub fn require_explicit(
        &self,
        auth: &AuthClaims,
    ) -> Result<Uuid, (StatusCode, Json<serde_json::Value>)> {
        let header_tenant = self.0.ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "status": "ERROR",
                    "message": "Missing required header: x-tenant-id"
                })),
            )
        })?;

        if auth.role() != "SUPER_ADMIN" && auth.tenant_id() != header_tenant {
            return Err((
                StatusCode::FORBIDDEN,
                Json(json!({
                    "status": "FORBIDDEN",
                    "message": "x-tenant-id header does not match authenticated tenant"
                })),
            ));
        }

        Ok(header_tenant)
    }
}
