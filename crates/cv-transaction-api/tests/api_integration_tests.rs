use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use cv_transaction_api::create_router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

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
async fn test_create_account_endpoint() {
    let app = create_router();

    let payload = json!({
        "tenant_id": "a0000000-0000-0000-0000-000000000001",
        "customer_id": "c0000000-0000-0000-0000-000000000001",
        "account_name": "Amina Bello Current Account",
        "gl_code": "2110002",
        "account_type": "CURRENT",
        "currency": "NGN",
        "bank_code": "058",
        "serial_number": 987654,
        "overdraft_limit": "50000.00"
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/accounts")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response.into_body().collect().await.unwrap().to_bytes();
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
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "COMMITTED");
    assert_eq!(json["amount"], "150000.00");
    assert_eq!(json["currency"], "NGN");
    assert!(json["transaction_id"].is_string());
}

#[tokio::test]
async fn test_intra_transfer_endpoint_negative_amount_rejected() {
    let app = create_router();

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
