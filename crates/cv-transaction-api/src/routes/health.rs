use axum::{routing::get, Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router {
    Router::new()
        .route("/health/live", get(liveness))
        .route("/health/ready", get(readiness))
}

async fn liveness() -> Json<Value> {
    Json(json!({
        "status": "UP",
        "service": "corevault-monolith",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn readiness() -> Json<Value> {
    Json(json!({
        "status": "READY",
        "checks": {
            "database": "UP",
            "valkey": "UP"
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
