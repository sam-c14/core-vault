pub mod middleware;
pub mod routes;

use axum::Router;
use cv_audit::MemoryAuditSink;
use cv_auth::JwtService;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub fn create_router() -> Router {
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "super_secret_jwt_key_change_in_production_123456".to_string());
    let jwt_service = Arc::new(JwtService::new(&jwt_secret));
    let audit_sink = Arc::new(MemoryAuditSink::new());

    Router::new()
        .merge(routes::health::router())
        .merge(routes::accounts::router())
        .merge(routes::transfers::router())
        .merge(routes::auth::router(jwt_service))
        .merge(routes::account_controls::router())
        .merge(routes::config::router())
        .merge(routes::audit::router(audit_sink))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
