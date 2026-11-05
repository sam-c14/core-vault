use crate::middleware::AuthClaims;
use crate::state::AppState;
use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/audit/logs", get(get_audit_logs))
        .with_state(state)
}

async fn get_audit_logs(
    auth: AuthClaims,
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_scope_or_role(
        "admin:read",
        &["SUPER_ADMIN", "AUDITOR", "COMPLIANCE_OFFICER"],
    )?;

    if let Some(pool) = &state.pool {
        let is_super_admin = auth.role() == "SUPER_ADMIN";
        let rows = if is_super_admin {
            sqlx::query(
                "SELECT id, tenant_id, actor_id, actor_type, action, entity_name, entity_id,
                        before_state, after_state, ip_address, trace_id, payload, created_at
                 FROM audit_logs
                 ORDER BY created_at DESC
                 LIMIT 100",
            )
            .fetch_all(pool)
            .await
        } else {
            sqlx::query(
                "SELECT id, tenant_id, actor_id, actor_type, action, entity_name, entity_id,
                        before_state, after_state, ip_address, trace_id, payload, created_at
                 FROM audit_logs
                 WHERE tenant_id = $1
                 ORDER BY created_at DESC
                 LIMIT 100",
            )
            .bind(auth.tenant_id())
            .fetch_all(pool)
            .await
        }
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Database error: {e}") })),
            )
        })?;

        let logs: Vec<Value> = rows
            .iter()
            .map(|r| {
                let created: chrono::DateTime<Utc> = r.get("created_at");
                json!({
                    "id": r.get::<Uuid, _>("id"),
                    "tenant_id": r.get::<Uuid, _>("tenant_id"),
                    "actor_id": r.get::<Uuid, _>("actor_id"),
                    "actor_type": r.get::<String, _>("actor_type"),
                    "action": r.get::<String, _>("action"),
                    "entity_name": r.get::<String, _>("entity_name"),
                    "entity_id": r.get::<Uuid, _>("entity_id"),
                    "before_state": r.get::<Option<Value>, _>("before_state"),
                    "after_state": r.get::<Option<Value>, _>("after_state"),
                    "payload": r.get::<Option<Value>, _>("payload"),
                    "created_at": created
                })
            })
            .collect();

        let total = logs.len();
        return Ok((
            StatusCode::OK,
            Json(json!({
                "status": "SUCCESS",
                "total_count": total,
                "audit_logs": logs
            })),
        ));
    }

    let events = state.audit_sink.get_events().await;
    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "SUCCESS",
            "total_count": events.len(),
            "audit_logs": events
        })),
    ))
}
