use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use cv_auth::JwtService;
use cv_transaction_api::create_router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

fn get_test_admin_token() -> String {
    let secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "super_secret_jwt_key_change_in_production_123456".to_string());
    let jwt_service = JwtService::new(&secret);
    let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let tenant_id = Uuid::parse_str("a0000000-0000-0000-0000-000000000001").unwrap();
    jwt_service
        .generate_token(user_id, tenant_id, "SUPER_ADMIN")
        .unwrap()
}

#[tokio::test]
async fn test_seeded_wallet_app_m2m_token_generation() {
    let app = create_router();

    // 1. Valid credentials for seeded native wallet app
    let payload = json!({
        "client_id": "client_wallet_app_seed001",
        "client_secret": "sec_live_wallet_app_seed_key_2026"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/token")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "SUCCESS");
    assert_eq!(json["token_type"], "Bearer");
    assert!(json["access_token"].is_string());
    assert_eq!(json["tenant_id"], "a0000000-0000-0000-0000-000000000001");

    // 2. Invalid client secret rejected with 401
    let invalid_payload = json!({
        "client_id": "client_wallet_app_seed001",
        "client_secret": "wrong_secret_123"
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/token")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(invalid_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_tenant_onboarding_whitelist_and_m2m_lifecycle() {
    let app = create_router();
    let admin_token = get_test_admin_token();
    let random_suffix = &Uuid::new_v4().simple().to_string()[..6];
    let tenant_code = format!("FN_{}", random_suffix);

    // 1. Onboard new client/tenant (requires SUPER_ADMIN token)
    let onboard_payload = json!({
        "tenant_code": tenant_code,
        "tenant_name": "Fintech Neo Global",
        "base_currency": "NGN",
        "scopes": ["transfers:write", "accounts:read"]
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/tenants")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(onboard_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    if status != StatusCode::CREATED {
        eprintln!(
            "ONBOARD FAILED WITH {}: {}",
            status,
            String::from_utf8_lossy(&body)
        );
    }
    assert_eq!(status, StatusCode::CREATED);
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "SUCCESS");
    assert_eq!(json["tenant"]["status"], "PENDING");

    let tenant_id = json["tenant"]["id"].as_str().unwrap().to_string();
    let client_id = json["tenant"]["client_id"].as_str().unwrap().to_string();
    let client_secret = json["tenant"]["client_secret"]
        .as_str()
        .unwrap()
        .to_string();

    // 2. Pending tenant attempts M2M token -> Rejected with 403 Forbidden
    let token_payload = json!({
        "client_id": client_id,
        "client_secret": client_secret
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/token")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(token_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["error"]
        .as_str()
        .unwrap()
        .contains("pending admin approval"));

    // 3. Admin whitelists the tenant
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/admin/tenants/{tenant_id}/whitelist"))
                .method(Method::POST)
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["tenant"]["status"], "ACTIVE");

    // 4. M2M token request succeeds once whitelisted
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/token")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(token_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["access_token"].is_string());

    // 5. Admin blacklists the tenant
    let blacklist_payload = json!({
        "reason": "Suspicious transaction volume / AML evasion pattern"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/admin/tenants/{tenant_id}/blacklist"))
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(blacklist_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // 6. Blacklisted tenant is rejected with 403 Forbidden
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/token")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(token_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["error"].as_str().unwrap().contains("blacklisted"));

    // 7. Re-whitelisting restores access
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/admin/tenants/{tenant_id}/whitelist"))
                .method(Method::POST)
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/token")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(token_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_platform_automated_violation_blacklisting() {
    let app = create_router();
    let admin_token = get_test_admin_token();
    let random_suffix = &Uuid::new_v4().simple().to_string()[..6];
    let tenant_code = format!("VIOL_{}", random_suffix);

    // 1. Onboard dedicated tenant for violation test
    let onboard_payload = json!({
        "tenant_code": tenant_code,
        "tenant_name": "Sanctions Test Tenant",
        "base_currency": "NGN",
        "scopes": ["transfers:write"]
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/tenants")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(onboard_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let tenant_id = json["tenant"]["id"].as_str().unwrap().to_string();
    let client_id = json["tenant"]["client_id"].as_str().unwrap().to_string();
    let client_secret = json["tenant"]["client_secret"]
        .as_str()
        .unwrap()
        .to_string();

    // 2. Whitelist it
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/admin/tenants/{tenant_id}/whitelist"))
                .method(Method::POST)
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 3. Simulate automated platform rule violation
    let violation_payload = json!({
        "violation_type": "SANCTIONS_REGISTRY_MATCH",
        "details": "Client identity matched international OFAC/UN sanctions watch list"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/admin/tenants/{tenant_id}/simulate-violation"
                ))
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(violation_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "BLACKLISTED");
    assert_eq!(json["tenant"]["status"], "BLACKLISTED");
    assert_eq!(json["tenant"]["violation_count"], 1);

    // 4. M2M token request rejected with 403 Forbidden
    let token_payload = json!({
        "client_id": client_id,
        "client_secret": client_secret
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/token")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(token_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_tenant_custom_pnd_rules_and_system_automated_pnd_trigger() {
    let app = create_router();
    let admin_token = get_test_admin_token();
    let random_suffix = &Uuid::new_v4().simple().to_string()[..6];
    let tenant_code = format!("PND_{}", random_suffix);

    // 0. Onboard dedicated tenant
    let onboard_payload = json!({
        "tenant_code": tenant_code,
        "tenant_name": "PND Custom Rules Tenant",
        "base_currency": "NGN",
        "scopes": ["transfers:write", "accounts:write", "accounts:read"]
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/tenants")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(onboard_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let tenant_id = json["tenant"]["id"].as_str().unwrap().to_string();

    // Whitelist the tenant
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/admin/tenants/{tenant_id}/whitelist"))
                .method(Method::POST)
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Create source account
    let s1 = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        % 400000
        + 100000) as u32;
    let acc1_res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/accounts")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "tenant_id": tenant_id,
                        "customer_id": "c0000000-0000-0000-0000-000000000001",
                        "account_name": "PND Test Source Account",
                        "gl_code": "2110001",
                        "account_type": "SAVINGS",
                        "currency": "NGN",
                        "bank_code": "058",
                        "serial_number": s1,
                        "overdraft_limit": "500000.00"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(acc1_res.status(), StatusCode::CREATED);
    let body = acc1_res.into_body().collect().await.unwrap().to_bytes();
    let acc1_json: Value = serde_json::from_slice(&body).unwrap();
    let source_account_id = acc1_json["account"]["id"].as_str().unwrap().to_string();
    let source_nuban = acc1_json["account"]["account_number"]
        .as_str()
        .unwrap()
        .to_string();

    // Create destination account
    let s2 = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        % 400000
        + 500000) as u32;
    let acc2_res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/accounts")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "tenant_id": tenant_id,
                        "customer_id": "c0000000-0000-0000-0000-000000000001",
                        "account_name": "PND Test Destination Account",
                        "gl_code": "2110002",
                        "account_type": "SAVINGS",
                        "currency": "NGN",
                        "bank_code": "058",
                        "serial_number": s2,
                        "overdraft_limit": "0.00"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(acc2_res.status(), StatusCode::CREATED);
    let body = acc2_res.into_body().collect().await.unwrap().to_bytes();
    let acc2_json: Value = serde_json::from_slice(&body).unwrap();
    let destination_account_id = acc2_json["account"]["id"].as_str().unwrap().to_string();

    // 1. Tenant configures custom PND compliance rule
    let rule_payload = json!({
        "tenant_id": tenant_id,
        "rule_name": "KYC_TIER_1_SINGLE_TRANSFER_CAP",
        "rule_type": "KYC_TIER_LIMIT",
        "max_single_amount": "50000.00",
        "is_enabled": true
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/tenants/pnd-rules")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(rule_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    // 2. Fetch rules for tenant
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/tenants/{tenant_id}/pnd-rules"))
                .method(Method::GET)
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(!json["rules"].as_array().unwrap().is_empty());

    // 3. Compliant transfer (25,000 <= 50,000) succeeds
    let compliant_payload = json!({
        "source_account_id": source_account_id,
        "destination_account_id": destination_account_id,
        "amount": "25000.00",
        "currency": "NGN",
        "narration": "Tier 1 allowable payout"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/transfers/intra")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(compliant_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    // 4. Non-compliant transfer (75,000 > 50,000) breaches rule!
    let breach_payload = json!({
        "source_account_id": source_account_id,
        "destination_account_id": destination_account_id,
        "amount": "75000.00",
        "currency": "NGN",
        "narration": "Tier 1 limit breach attempt"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/transfers/intra")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(breach_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Fails with 409 CONFLICT and SYSTEM_AUTOMATED_PND
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "PND_ENFORCED");
    assert_eq!(json["pnd_source"], "SYSTEM_AUTOMATED_PND");
    assert_eq!(json["triggered_rule"], "KYC_TIER_1_SINGLE_TRANSFER_CAP");

    // 5. Subsequent transfer is rejected because account is under PND
    let small_payload = json!({
        "source_account_id": source_account_id,
        "destination_account_id": destination_account_id,
        "amount": "1000.00",
        "currency": "NGN",
        "narration": "Small transfer after PND placed"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/transfers/intra")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(small_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // 6. Tenant lifts the automated PND
    let lift_payload = json!({
        "reason": "Customer provided Tier 2 KYC proof of address and identity"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/accounts/{source_nuban}/pnd/lift"))
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(lift_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["current_status"], "ACTIVE");
    assert!(json["pnd_source"].is_null());

    // 7. Transfer succeeds now that PND is lifted
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/transfers/intra")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(small_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_manual_client_pnd_and_lifting() {
    let app = create_router();
    let admin_token = get_test_admin_token();

    // Ensure account is ACTIVE before test
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/accounts/0581111111/activate")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({"reason": "Reset before test"}).to_string(),
                ))
                .unwrap(),
        )
        .await;

    // 1. Client manually places PND
    let pnd_payload = json!({
        "reason": "Customer called fraud desk: phone stolen"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/accounts/0581111111/pnd")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(pnd_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["current_status"], "POST_NO_DEBIT");
    assert_eq!(json["pnd_source"], "MANUAL_CLIENT_PND");

    // 2. Transfer is blocked
    let transfer_payload = json!({
        "source_account_id": "00000000-0000-0000-0000-000000000001",
        "destination_account_id": "00000000-0000-0000-0000-000000000002",
        "amount": "5000.00",
        "currency": "NGN",
        "narration": "Blocked by manual PND"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/transfers/intra")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(transfer_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // 3. Client lifts manual PND
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/accounts/0581111111/pnd/lift")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["current_status"], "ACTIVE");
    assert!(json["pnd_source"].is_null());

    // 4. Transfer now succeeds
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/transfers/intra")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(transfer_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    if status != StatusCode::CREATED {
        eprintln!(
            "TRANSFER FAILED WITH {}: {}",
            status,
            String::from_utf8_lossy(&body)
        );
    }
    assert_eq!(status, StatusCode::CREATED);

    if let Ok(db_url) = std::env::var("DATABASE_URL") {
        if let Ok(pool) = sqlx::PgPool::connect(&db_url).await {
            let _ = sqlx::query("DELETE FROM tenant_pnd_rules WHERE tenant_id = 'a0000000-0000-0000-0000-000000000001'").execute(&pool).await;
        }
    }
}
