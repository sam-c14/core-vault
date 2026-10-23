pub mod middleware;
pub mod routes;
pub mod state;

pub use state::AppState;

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub fn create_router() -> Router {
    let state = AppState::new();
    create_router_with_state(state)
}

pub fn create_router_with_state(state: AppState) -> Router {
    Router::new()
        .merge(routes::health::router())
        .merge(routes::accounts::router(state.clone()))
        .merge(routes::transfers::router(state.clone()))
        .merge(routes::auth::router(state.clone()))
        .merge(routes::admin_tenants::router(state.clone()))
        .merge(routes::tenant_rules::router(state.clone()))
        .merge(routes::account_controls::router(state.clone()))
        .merge(routes::config::router())
        .merge(routes::audit::router(state.audit_sink.clone()))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
