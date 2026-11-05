use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use cv_auth::{hash_password, JwtService};
use cv_transaction_api::create_router;
use http_body_util::BodyExt;
use rust_decimal_macros::dec;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

fn get_admin_token() -> String {
    let secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "super_secret_jwt_key_change_in_production_123456".to_string());
    let jwt_service = JwtService::new(&secret);
    let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let tenant_id = Uuid::parse_str("a0000000-0000-0000-0000-000000000001").unwrap();
    jwt_service
        .generate_token(user_id, tenant_id, "SUPER_ADMIN")
        .unwrap()
}

fn basic_auth_header(client_id: &str, client_secret: &str) -> String {
    let raw = format!("{client_id}:{client_secret}");
    let encoded = BASE64.encode(raw.as_bytes());
    format!("Basic {encoded}")
}

#[tokio::test]
async fn test_client_login_with_basic_auth_and_email_password() {
    let app = create_router();
    let admin_token = get_admin_token();

    // 1. Onboard a new tenant
    let suffix = &Uuid::new_v4().simple().to_string()[..6];
    let onboard_payload = json!({
        "tenant_code": format!("TN_{suffix}"),
        "tenant_name": "Acme Fintech Corp",
        "base_currency": "NGN",
        "scopes": ["accounts:read", "accounts:write", "transfers:write"]
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
    let json_val: Value = serde_json::from_slice(&body).unwrap();
    let tenant_id = json_val["tenant"]["id"].as_str().unwrap();
    let client_id = json_val["tenant"]["client_id"].as_str().unwrap();
    let client_secret = json_val["tenant"]["client_secret"].as_str().unwrap();

    // 2. Whitelist the tenant to make them ACTIVE
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

    // 3. Insert a business user for this tenant in DB (if DB available)
    let user_email = format!("ops_{suffix}@acmefintech.com");
    let user_pw = "BusinessUserSecret#2026";
    let pw_hash = hash_password(user_pw).unwrap();

    if let Ok(db_url) = std::env::var("DATABASE_URL") {
        let pool = sqlx::PgPool::connect(&db_url).await.unwrap();
        let user_id = Uuid::new_v4();
        let tenant_uuid = Uuid::parse_str(tenant_id).unwrap();

        sqlx::query(
            "INSERT INTO users (id, tenant_id, email, password_hash, first_name, last_name, role, is_active)
             VALUES ($1, $2, $3, $4, 'Alice', 'FintechOps', 'OPERATIONS_OFFICER', true)"
        )
        .bind(user_id)
        .bind(tenant_uuid)
        .bind(&user_email)
        .bind(&pw_hash)
        .execute(&pool)
        .await
        .unwrap();
    }

    // 4. Client login via POST /api/v1/auth/client-login with Basic Auth + JSON email/password
    let basic_header = basic_auth_header(client_id, client_secret);
    let login_payload = json!({
        "email": user_email,
        "password": user_pw
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/client-login")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, &basic_header)
                .body(Body::from(login_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let login_res: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(login_res["status"], "SUCCESS");
    assert!(login_res["token"].is_string());
    assert_eq!(login_res["user"]["email"], user_email);
    assert_eq!(login_res["user"]["tenant_id"], tenant_id);

    // 5. Client login also works via POST /api/v1/auth/login with Basic Auth
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/login")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, &basic_header)
                .body(Body::from(login_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    // 6. Invalid client_secret rejected with 401 Unauthorized
    let wrong_client_auth = basic_auth_header(client_id, "wrong_secret");
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/client-login")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, &wrong_client_auth)
                .body(Body::from(login_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 7. Invalid user password rejected with 401 Unauthorized
    let wrong_pw_payload = json!({
        "email": user_email,
        "password": "WrongPassword123!"
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/client-login")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, &basic_header)
                .body(Body::from(wrong_pw_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 8. Cross-tenant user login rejected: another tenant trying to log in with this user's email
    let other_tenant_auth = basic_auth_header(
        "client_wallet_app_seed001",
        "sec_live_wallet_app_seed_key_2026",
    );
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/client-login")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, &other_tenant_auth)
                .body(Body::from(login_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_tenant_header_and_cross_tenant_isolation() {
    let app = create_router();
    let admin_token = get_admin_token();
    let jwt_service = JwtService::new(
        &std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "super_secret_jwt_key_change_in_production_123456".to_string()),
    );

    // 1. Onboard Tenant A
    let suffix_a = &Uuid::new_v4().simple().to_string()[..6];
    let res_a = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/tenants")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "tenant_code": format!("TA_{suffix_a}"),
                        "tenant_name": "Tenant Alpha Bank",
                        "base_currency": "NGN"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = res_a.into_body().collect().await.unwrap().to_bytes();
    let val_a: Value = serde_json::from_slice(&body).unwrap();
    let tenant_a_id = Uuid::parse_str(val_a["tenant"]["id"].as_str().unwrap()).unwrap();

    // Whitelist Tenant A
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/admin/tenants/{tenant_a_id}/whitelist"))
                .method(Method::POST)
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 2. Onboard Tenant B
    let suffix_b = &Uuid::new_v4().simple().to_string()[..6];
    let res_b = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/tenants")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "tenant_code": format!("TB_{suffix_b}"),
                        "tenant_name": "Tenant Beta Fintech",
                        "base_currency": "NGN"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = res_b.into_body().collect().await.unwrap().to_bytes();
    let val_b: Value = serde_json::from_slice(&body).unwrap();
    let tenant_b_id = Uuid::parse_str(val_b["tenant"]["id"].as_str().unwrap()).unwrap();

    // Whitelist Tenant B
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/admin/tenants/{tenant_b_id}/whitelist"))
                .method(Method::POST)
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Generate tokens for Tenant A operator and Tenant B operator
    let user_a_id = Uuid::new_v4();
    let user_b_id = Uuid::new_v4();
    let token_a = jwt_service
        .generate_token(user_a_id, tenant_a_id, "OPERATIONS_OFFICER")
        .unwrap();
    let token_b = jwt_service
        .generate_token(user_b_id, tenant_b_id, "OPERATIONS_OFFICER")
        .unwrap();

    // 3. Tenant A creates an account without `tenant_id` in JSON body, using `x-tenant-id` header
    let serial_a = (Uuid::new_v4().as_fields().0 % 800000) + 100000;
    let create_acc_a_payload = json!({
        "account_name": "Alpha Operations Account",
        "gl_code": "2110001",
        "account_type": "CURRENT",
        "currency": "NGN",
        "bank_code": "058",
        "serial_number": serial_a
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/accounts")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token_a}"))
                .header("x-tenant-id", tenant_a_id.to_string())
                .body(Body::from(create_acc_a_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let acc_a: Value = serde_json::from_slice(&body).unwrap();
    let nuban_a = acc_a["account"]["account_number"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(acc_a["account"]["tenant_id"], tenant_a_id.to_string());

    // 4. Non-super-admin trying to pass a different tenant's `x-tenant-id` is rejected with 403 Forbidden
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/accounts")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token_b}"))
                .header("x-tenant-id", tenant_a_id.to_string()) // Mismatched tenant
                .body(Body::from(create_acc_a_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 5. Tenant B creates their own account
    let serial_b = (Uuid::new_v4().as_fields().0 % 800000) + 100000;
    let create_acc_b_payload = json!({
        "account_name": "Beta Settlement Account",
        "gl_code": "2110001",
        "account_type": "CURRENT",
        "currency": "NGN",
        "bank_code": "058",
        "serial_number": serial_b
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/accounts")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token_b}"))
                .header("x-tenant-id", tenant_b_id.to_string())
                .body(Body::from(create_acc_b_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let acc_b: Value = serde_json::from_slice(&body).unwrap();

    // 6. Cross-Tenant Account Inspection Blocked:
    // Tenant B attempts to call GET /api/v1/accounts/:nuban_a
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/accounts/{nuban_a}"))
                .method(Method::GET)
                .header(header::AUTHORIZATION, format!("Bearer {token_b}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // But Tenant A querying their own account succeeds:
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/accounts/{nuban_a}"))
                .method(Method::GET)
                .header(header::AUTHORIZATION, format!("Bearer {token_a}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // And Super Admin querying Tenant A's account succeeds:
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/accounts/{nuban_a}"))
                .method(Method::GET)
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 7. Cross-Tenant Freeze Blocked:
    // Tenant B attempts to freeze Tenant A's account
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/accounts/{nuban_a}/freeze"))
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token_b}"))
                .body(Body::from(
                    json!({ "reason": "Hostile freeze attempt" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 8. Cross-Tenant PND Placement Blocked:
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/accounts/{nuban_a}/pnd"))
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token_b}"))
                .body(Body::from(
                    json!({ "reason": "Hostile PND attempt" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 9. Cross-Tenant Lien Placement Blocked:
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/accounts/{nuban_a}/liens"))
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token_b}"))
                .body(Body::from(
                    json!({ "amount": dec!(5000.00), "reason": "Hostile Lien" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 10. Tenant PND Rules creation via x-tenant-id header (no tenant_id in body):
    let pnd_rule_payload = json!({
        "rule_name": "Tier 1 Daily Cap",
        "rule_type": "DAILY_DEBIT_CAP",
        "max_single_amount": dec!(50000.00),
        "max_daily_cumulative": dec!(100000.00)
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/tenants/pnd-rules")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token_a}"))
                .header("x-tenant-id", tenant_a_id.to_string())
                .body(Body::from(pnd_rule_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // 11. Tenant A lists rules via GET /api/v1/tenants/pnd-rules (without path param)
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/tenants/pnd-rules")
                .method(Method::GET)
                .header(header::AUTHORIZATION, format!("Bearer {token_a}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let rules_val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(rules_val["rules"].as_array().unwrap().len(), 1);
    assert_eq!(rules_val["rules"][0]["rule_name"], "Tier 1 Daily Cap");

    // 12. Tenant B listing rules returns empty (none created for Tenant B)
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/tenants/pnd-rules")
                .method(Method::GET)
                .header(header::AUTHORIZATION, format!("Bearer {token_b}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let rules_b: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(rules_b["rules"].as_array().unwrap().len(), 0);

    // 13. Cross-Tenant Debit Blocked on Intra-Transfer:
    // Tenant B attempts to debit Tenant A's account
    let acc_a_id = Uuid::parse_str(acc_a["account"]["id"].as_str().unwrap()).unwrap();
    let acc_b_id = Uuid::parse_str(acc_b["account"]["id"].as_str().unwrap()).unwrap();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/transfers/intra")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token_b}"))
                .body(Body::from(
                    json!({
                        "source_account_id": acc_a_id,
                        "destination_account_id": acc_b_id,
                        "amount": dec!(1000.00),
                        "currency": "NGN",
                        "narration": "Illegal cross-tenant theft attempt"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 14. Cross-Tenant Debit Blocked on Outward-Transfer:
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/transfers/outward")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token_b}"))
                .body(Body::from(
                    json!({
                        "source_account_id": acc_a_id,
                        "destination_account": "0123456789",
                        "destination_bank_code": "058",
                        "amount": dec!(1000.00),
                        "currency": "NGN",
                        "narration": "Illegal outward transfer attempt"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}
