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

fn get_jwt_service() -> JwtService {
    let secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "super_secret_jwt_key_change_in_production_123456".to_string());
    JwtService::new(&secret)
}

fn generate_token_for_role(role: &str, scopes: &[String]) -> String {
    let jwt_service = get_jwt_service();
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::parse_str("a0000000-0000-0000-0000-000000000001").unwrap();
    if role == "API_CLIENT" {
        jwt_service
            .generate_m2m_token(user_id, tenant_id, scopes)
            .unwrap()
    } else {
        jwt_service
            .generate_token(user_id, tenant_id, role)
            .unwrap()
    }
}

#[tokio::test]
async fn test_db_backed_staff_login_verification() {
    let app = create_router();

    // 1. Valid login for seeded Super Admin
    let valid_login = json!({
        "email": "admin@corevault.bank",
        "password": "AdminSecurePass#2026"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/login")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(valid_login.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "SUCCESS");
    assert_eq!(json["token_type"], "Bearer");
    assert_eq!(json["user"]["email"], "admin@corevault.bank");
    assert_eq!(json["user"]["role"], "SUPER_ADMIN");
    let token = json["token"].as_str().unwrap().to_string();

    // 2. Wrong password returns 401
    let wrong_pw = json!({
        "email": "admin@corevault.bank",
        "password": "WrongPassword#2026"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/login")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(wrong_pw.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 3. Non-existent user returns 401
    let nonexistent_user = json!({
        "email": "nonexistent@corevault.bank",
        "password": "AnyPassword#2026"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/login")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(nonexistent_user.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 4. Test /api/v1/auth/me with Bearer token
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/me")
                .method(Method::GET)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "SUCCESS");
    assert_eq!(json["claims"]["role"], "SUPER_ADMIN");
}

#[tokio::test]
async fn test_route_role_and_scope_gating_restrictions() {
    let app = create_router();

    // Token for an unprivileged client with only read scope
    let limited_token = generate_token_for_role("API_CLIENT", &["accounts:read".to_string()]);

    // 1. Attempting admin tenant onboarding with non-admin token -> 403 FORBIDDEN
    let onboard_payload = json!({
        "tenant_code": "UNAUTH_NEO",
        "tenant_name": "Unauthorized Onboard Attempt"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/tenants")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {limited_token}"))
                .body(Body::from(onboard_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 2. Attempting transfer with token lacking 'transfers:write' -> 403 FORBIDDEN
    let transfer_payload = json!({
        "source_account_id": "00000000-0000-0000-0000-000000000001",
        "destination_account_id": "00000000-0000-0000-0000-000000000002",
        "amount": "1000.00",
        "currency": "NGN",
        "narration": "Unauthorized transfer scope check"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/transfers/intra")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {limited_token}"))
                .body(Body::from(transfer_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 3. Attempting account freeze with token lacking 'accounts:write' -> 403 FORBIDDEN
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/accounts/0581111111/freeze")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {limited_token}"))
                .body(Body::from(json!({"reason": "Unauth freeze"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 4. Attempting to fetch audit logs with token lacking 'admin:read' -> 403 FORBIDDEN
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/audit/logs")
                .method(Method::GET)
                .header(header::AUTHORIZATION, format!("Bearer {limited_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_db_transaction_account_controls_and_audit_pipeline() {
    let app = create_router();
    let admin_token = generate_token_for_role("SUPER_ADMIN", &["*".to_string()]);

    // 1. Activate account to start clean
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/accounts/0581111111/activate")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({"reason": "Test setup activate"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 2. Place a Lien of 250,000 NGN in a DB transaction
    let lien_payload = json!({
        "amount": "250000.00",
        "reason": "Court dispute lien placement"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/accounts/0581111111/liens")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(lien_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "SUCCESS");
    assert_eq!(json["action"], "LIEN_PLACED");
    assert_eq!(json["placed_lien_amount"], "250000.00");

    // 3. Query account to verify lien updated in DB
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/accounts/0581111111")
                .method(Method::GET)
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["lien_amount"], "250000.0000");

    // 4. Release the Lien of 250,000 NGN in a DB transaction
    let release_payload = json!({
        "amount": "250000.00",
        "reason": "Court dispute resolved"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/accounts/0581111111/liens/release")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(release_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["action"], "LIEN_RELEASED");

    // 5. Freeze the account in a DB transaction
    let freeze_payload = json!({
        "reason": "AML law enforcement freeze order"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/accounts/0581111111/freeze")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(freeze_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["current_status"], "FROZEN");
    assert_eq!(json["can_debit"], false);

    // 6. Attempt transfer while FROZEN -> 403 ACCOUNT_RESTRICTED
    let transfer_payload = json!({
        "source_account_id": "00000000-0000-0000-0000-000000000001",
        "destination_account_id": "00000000-0000-0000-0000-000000000002",
        "amount": "10000.00",
        "currency": "NGN",
        "narration": "Transfer attempt while account is frozen"
    });

    let res = app
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

    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ACCOUNT_RESTRICTED");

    // 7. Unfreeze / Activate account
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/accounts/0581111111/activate")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({"reason": "Freeze lifted by compliance"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    // 8. Fetch real audit logs from PostgreSQL table
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/audit/logs")
                .method(Method::GET)
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "SUCCESS");
    assert!(json["total_count"].as_u64().unwrap() > 0);
    assert!(!json["audit_logs"].as_array().unwrap().is_empty());
}
