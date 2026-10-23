use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use cv_audit::MemoryAuditSink;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Clone)]
pub struct AuditState {
    pub sink: Arc<MemoryAuditSink>,
}

pub fn router(sink: Arc<MemoryAuditSink>) -> Router {
    let state = AuditState { sink };
    Router::new()
        .route("/api/v1/audit/logs", get(get_audit_logs))
        .with_state(state)
}

async fn get_audit_logs(State(state): State<AuditState>) -> (StatusCode, Json<Value>) {
    let events = state.sink.get_events().await;
    (
        StatusCode::OK,
        Json(json!({
            "status": "SUCCESS",
            "total_count": events.len(),
            "audit_logs": events
        })),
    )
}
