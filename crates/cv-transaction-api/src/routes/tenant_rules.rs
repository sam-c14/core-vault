use crate::middleware::AuthClaims;
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
use sqlx::Row;
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

fn pnd_rule_type_to_str(rt: &PndRuleType) -> &'static str {
    match rt {
        PndRuleType::KycTierLimit => "KYC_TIER_LIMIT",
        PndRuleType::VelocityBurst => "VELOCITY_BURST",
        PndRuleType::DailyDebitCap => "DAILY_DEBIT_CAP",
        PndRuleType::SuspiciousTransfer => "SUSPICIOUS_TRANSFER",
    }
}

fn str_to_pnd_rule_type(s: &str) -> PndRuleType {
    match s {
        "KYC_TIER_LIMIT" => PndRuleType::KycTierLimit,
        "VELOCITY_BURST" => PndRuleType::VelocityBurst,
        "DAILY_DEBIT_CAP" => PndRuleType::DailyDebitCap,
        _ => PndRuleType::SuspiciousTransfer,
    }
}

async fn create_pnd_rule(
    auth: AuthClaims,
    State(state): State<AppState>,
    Json(dto): Json<CreatePndRuleDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_scope_or_role(
        "accounts:write",
        &["SUPER_ADMIN", "OPERATIONS_OFFICER", "COMPLIANCE_OFFICER"],
    )?;

    if auth.role() != "SUPER_ADMIN" && auth.tenant_id() != dto.tenant_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(
                json!({ "status": "FORBIDDEN", "message": "Cannot create rules for another tenant" }),
            ),
        ));
    }

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

    let rule_id = Uuid::new_v4();
    let is_enabled = dto.is_enabled.unwrap_or(true);
    let rule_type_str = pnd_rule_type_to_str(&dto.rule_type);

    if let Some(pool) = &state.pool {
        sqlx::query(
            "INSERT INTO tenant_pnd_rules (
                id, tenant_id, rule_name, rule_type, max_single_amount,
                max_daily_cumulative, action, is_enabled, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, 'SYSTEM_AUTOMATED_PND', $7, NOW())",
        )
        .bind(rule_id)
        .bind(dto.tenant_id)
        .bind(&dto.rule_name)
        .bind(rule_type_str)
        .bind(dto.max_single_amount)
        .bind(dto.max_daily_cumulative)
        .bind(is_enabled)
        .execute(pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Database error: {e}") })),
            )
        })?;

        let audit_payload = json!({
            "rule_name": dto.rule_name,
            "rule_type": rule_type_str,
            "max_single_amount": dto.max_single_amount
        });

        let _ = sqlx::query(
            "INSERT INTO audit_logs (id, tenant_id, actor_id, actor_type, action, entity_name, entity_id, payload, created_at)
             VALUES ($1, $2, $3, 'USER', 'CREATE_PND_RULE', 'TENANT_RULE', $4, $5, NOW())"
        )
        .bind(Uuid::new_v4())
        .bind(dto.tenant_id)
        .bind(auth.user_id())
        .bind(rule_id)
        .bind(audit_payload)
        .execute(pool)
        .await;
    }

    let rule = TenantPndRule {
        id: rule_id,
        tenant_id: dto.tenant_id,
        rule_name: dto.rule_name,
        rule_type: dto.rule_type,
        max_single_amount: dto.max_single_amount,
        max_daily_cumulative: dto.max_daily_cumulative,
        is_enabled,
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
    auth: AuthClaims,
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    auth.require_scope_or_role(
        "accounts:read",
        &[
            "SUPER_ADMIN",
            "OPERATIONS_OFFICER",
            "COMPLIANCE_OFFICER",
            "AUDITOR",
        ],
    )?;

    if auth.role() != "SUPER_ADMIN" && auth.tenant_id() != tenant_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(
                json!({ "status": "FORBIDDEN", "message": "Cannot list rules for another tenant" }),
            ),
        ));
    }

    if let Some(pool) = &state.pool {
        let rows = sqlx::query(
            "SELECT id, tenant_id, rule_name, rule_type, max_single_amount, max_daily_cumulative, is_enabled
             FROM tenant_pnd_rules
             WHERE tenant_id = $1
             ORDER BY created_at DESC"
        )
        .bind(tenant_id)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "ERROR", "message": format!("Database error: {e}") })),
            )
        })?;

        let tenant_rules: Vec<TenantPndRule> = rows
            .iter()
            .map(|r| {
                let rt_str: String = r.get("rule_type");
                TenantPndRule {
                    id: r.get("id"),
                    tenant_id: r.get("tenant_id"),
                    rule_name: r.get("rule_name"),
                    rule_type: str_to_pnd_rule_type(&rt_str),
                    max_single_amount: r.get("max_single_amount"),
                    max_daily_cumulative: r.get("max_daily_cumulative"),
                    is_enabled: r.get("is_enabled"),
                }
            })
            .collect();

        return Ok((
            StatusCode::OK,
            Json(json!({
                "status": "SUCCESS",
                "tenant_id": tenant_id,
                "rules": tenant_rules
            })),
        ));
    }

    let rules = state.pnd_rules.read().unwrap();
    let tenant_rules: Vec<TenantPndRule> = rules
        .iter()
        .filter(|r| r.tenant_id == tenant_id)
        .cloned()
        .collect();

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "SUCCESS",
            "tenant_id": tenant_id,
            "rules": tenant_rules
        })),
    ))
}
