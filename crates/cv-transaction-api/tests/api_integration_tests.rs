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
async fn test_health_live_endpoint() {
    let app = create_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/live")
                .method(Method::GET)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "UP");
    assert_eq!(json["service"], "corevault-monolith");
}

#[tokio::test]
async fn test_health_ready_endpoint() {
    let app = create_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/ready")
                .method(Method::GET)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "READY");
    assert_eq!(json["checks"]["database"], "UP");
    assert_eq!(json["checks"]["valkey"], "UP");
}

#[tokio::test]
async fn test_unauthenticated_requests_rejected() {
    let app = create_router();

    // 1. Unauthenticated transfer rejected with 401
    let transfer_payload = json!({
        "source_account_id": "00000000-0000-0000-0000-000000000001",
        "destination_account_id": "00000000-0000-0000-0000-000000000002",
        "amount": "150000.00",
        "currency": "NGN",
        "narration": "Unauthenticated transfer attempt"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/transfers/intra")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(transfer_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 2. Unauthenticated account creation rejected with 401
    let acc_payload = json!({
        "tenant_id": "a0000000-0000-0000-0000-000000000001",
        "account_name": "Unauth Test Account",
        "gl_code": "2110001",
        "account_type": "SAVINGS",
        "currency": "NGN",
        "bank_code": "058",
        "serial_number": 123456
    });

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/accounts")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(acc_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_account_endpoint() {
    let app = create_router();
    let token = get_test_admin_token();

    let serial = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        % 899999
        + 100000) as u32;

    let payload = json!({
        "tenant_id": "a0000000-0000-0000-0000-000000000001",
        "customer_id": "c0000000-0000-0000-0000-000000000001",
        "account_name": "Amina Bello Current Account",
        "gl_code": "2110002",
        "account_type": "CURRENT",
        "currency": "NGN",
        "bank_code": "058",
        "serial_number": serial,
        "overdraft_limit": "50000.00"
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/accounts")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    if status != StatusCode::CREATED {
        eprintln!(
            "CREATE ACCOUNT FAILED WITH {}: {}",
            status,
            String::from_utf8_lossy(&body)
        );
    }
    assert_eq!(status, StatusCode::CREATED);

    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "SUCCESS");

    let account_num = json["account"]["account_number"].as_str().unwrap();
    assert_eq!(account_num.len(), 10);
    assert_eq!(&account_num[0..3], "058");
    assert_eq!(json["account"]["status"], "ACTIVE");
}

#[tokio::test]
async fn test_intra_transfer_endpoint_balanced() {
    let app = create_router();
    let token = get_test_admin_token();

    let payload = json!({
        "source_account_id": "00000000-0000-0000-0000-000000000001",
        "destination_account_id": "00000000-0000-0000-0000-000000000002",
        "amount": "150000.00",
        "currency": "NGN",
        "narration": "Wholesale goods payment"
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/transfers/intra")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    if status != StatusCode::CREATED {
        eprintln!(
            "CREATE ACCOUNT FAILED WITH {}: {}",
            status,
            String::from_utf8_lossy(&body)
        );
    }
    assert_eq!(status, StatusCode::CREATED);

    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "COMMITTED");
    assert_eq!(json["amount"], "150000.00");
    assert_eq!(json["currency"], "NGN");
    assert!(json["transaction_id"].is_string());
}

#[tokio::test]
async fn test_intra_transfer_endpoint_negative_amount_rejected() {
    let app = create_router();
    let token = get_test_admin_token();

    let payload = json!({
        "source_account_id": "00000000-0000-0000-0000-000000000001",
        "destination_account_id": "00000000-0000-0000-0000-000000000002",
        "amount": "-1500.00",
        "currency": "NGN",
        "narration": "Illegal negative test"
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/transfers/intra")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "VALIDATION_FAILED");
}
