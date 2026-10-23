use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use cv_types::{PndRuleType, TenantPndRule};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/tenants/pnd-rules", post(create_pnd_rule))
        .route(
            "/api/v1/tenants/:tenant_id/pnd-rules",
            get(list_tenant_pnd_rules),
        )
        .with_state(state)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreatePndRuleDto {
    pub tenant_id: Uuid,
    pub rule_name: String,
    pub rule_type: PndRuleType,
    pub max_single_amount: Option<Decimal>,
    pub max_daily_cumulative: Option<Decimal>,
    pub is_enabled: Option<bool>,
}

async fn create_pnd_rule(
    State(state): State<AppState>,
    Json(dto): Json<CreatePndRuleDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    if dto.rule_name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "status": "ERROR", "message": "rule_name is required" })),
        ));
    }

    if let Some(single) = dto.max_single_amount {
        if single <= Decimal::ZERO {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(
                    json!({ "status": "ERROR", "message": "max_single_amount must be strictly positive" }),
                ),
            ));
        }
    }

    if let Some(daily) = dto.max_daily_cumulative {
        if daily <= Decimal::ZERO {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(
                    json!({ "status": "ERROR", "message": "max_daily_cumulative must be strictly positive" }),
                ),
            ));
        }
    }

    let rule = TenantPndRule {
        id: Uuid::new_v4(),
        tenant_id: dto.tenant_id,
        rule_name: dto.rule_name,
        rule_type: dto.rule_type,
        max_single_amount: dto.max_single_amount,
        max_daily_cumulative: dto.max_daily_cumulative,
        is_enabled: dto.is_enabled.unwrap_or(true),
    };

    {
        let mut rules = state.pnd_rules.write().unwrap();
        rules.push(rule.clone());
    }

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "status": "SUCCESS",
            "message": "Tenant custom compliance PND rule registered",
            "rule": rule
        })),
    ))
}

async fn list_tenant_pnd_rules(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
) -> (StatusCode, Json<Value>) {
    let rules = state.pnd_rules.read().unwrap();
    let tenant_rules: Vec<TenantPndRule> = rules
        .iter()
        .filter(|r| r.tenant_id == tenant_id)
        .cloned()
        .collect();

    (
        StatusCode::OK,
        Json(json!({
            "status": "SUCCESS",
            "tenant_id": tenant_id,
            "rules": tenant_rules
        })),
    )
}
