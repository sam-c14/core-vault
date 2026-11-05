use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use cv_auth::JwtService;
use cv_payment_gateway::{PaymentGateway, PaymentRailProvider};
use cv_transaction_api::create_router;
use cv_types::traits::{AccountEnquiry, OutwardTransfer, PaymentRailAdapter, TransferReceipt};
use http_body_util::BodyExt;
use rust_decimal_macros::dec;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

fn generate_test_admin_token() -> String {
    let jwt_service = JwtService::new("super_secret_jwt_key_change_in_production_123456");
    jwt_service
        .generate_token(
            Uuid::new_v4(),
            Uuid::parse_str("a0000000-0000-0000-0000-000000000001").unwrap(),
            "SUPER_ADMIN",
        )
        .expect("Failed to generate test admin token")
}

#[tokio::test]
async fn test_payment_rails_enquiry_and_outward_dispatch() {
    let app = create_router();
    let token = generate_test_admin_token();

    // 1. Inspect Payment Rails via HTTP
    let rails_req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/payment-rails")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let rails_res = app.clone().oneshot(rails_req).await.unwrap();
    assert_eq!(rails_res.status(), StatusCode::OK);
    let body_bytes = rails_res.into_body().collect().await.unwrap().to_bytes();
    let rails_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(rails_json["status"], "SUCCESS");
    assert_eq!(rails_json["active_provider"], "Anchor");

    // 2. Perform Name Enquiry via default Anchor rail
    let enquiry_payload = json!({
        "bank_code": "044",
        "account_number": "0440001237"
    });

    let enquiry_req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/transfers/enquiry")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&enquiry_payload).unwrap()))
        .unwrap();

    let enquiry_res = app.clone().oneshot(enquiry_req).await.unwrap();
    assert_eq!(enquiry_res.status(), StatusCode::OK);
    let body_bytes = enquiry_res.into_body().collect().await.unwrap().to_bytes();
    let enquiry_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(enquiry_json["status"], "SUCCESS");
    assert_eq!(enquiry_json["provider"], "Anchor");
    assert_eq!(
        enquiry_json["data"]["account_name"],
        "COREVAULT RETAIL CUSTOMER"
    );

    // 3. Dispatch Outward Transfer via default Anchor rail
    let outward_payload = json!({
        "source_account_id": "00000000-0000-0000-0000-000000000001",
        "destination_account": "0440001230",
        "destination_bank_code": "044",
        "amount": "15000.00",
        "currency": "NGN",
        "narration": "Logistics outward payment via Anchor"
    });

    let outward_req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/transfers/outward")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&outward_payload).unwrap()))
        .unwrap();

    let outward_res = app.clone().oneshot(outward_req).await.unwrap();
    assert_eq!(outward_res.status(), StatusCode::CREATED);
    let body_bytes = outward_res.into_body().collect().await.unwrap().to_bytes();
    let outward_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(outward_json["status"], "SUCCESS");
    assert_eq!(outward_json["provider"], "Anchor");
    let session_id = outward_json["data"]["session_id"].as_str().unwrap();
    assert!(session_id.starts_with("tra_")); // Anchor format
    assert_eq!(outward_json["data"]["response_code"], "00");

    // 4. Attempt to switch to direct NIBSS without enterprise crate -> returns 400
    let switch_payload = json!({ "provider": "NibssDirect" });
    let switch_req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/admin/payment-rails/active")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&switch_payload).unwrap()))
        .unwrap();

    let switch_res = app.clone().oneshot(switch_req).await.unwrap();
    assert_eq!(switch_res.status(), StatusCode::BAD_REQUEST);
    let body_bytes = switch_res.into_body().collect().await.unwrap().to_bytes();
    let switch_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(switch_json["status"], "SWITCH_FAILED");
    let err_msg = switch_json["error"].as_str().unwrap();
    assert!(err_msg.contains("requires cba-enterprise cv-rail-nip"));
}

#[tokio::test]
async fn test_direct_payment_gateway_unit_methods() {
    let gateway = PaymentGateway::anchor_default();
    assert_eq!(gateway.active_provider(), PaymentRailProvider::Anchor);

    // Name enquiry directly on gateway
    let enquiry: AccountEnquiry = gateway.name_enquiry("044", "0440001237").await.unwrap();
    assert_eq!(enquiry.account_name, "COREVAULT RETAIL CUSTOMER");

    // Outward transfer directly on gateway
    let transfer = OutwardTransfer {
        transaction_ref: "REF-UNIT-001".to_string(),
        source_account: "0440001237".to_string(),
        destination_account: "0440001230".to_string(),
        destination_bank_code: "044".to_string(),
        amount: dec!(25000.00),
        narration: "Direct unit test".to_string(),
    };

    let receipt: TransferReceipt = gateway.dispatch_outward(&transfer).await.unwrap();
    assert_eq!(receipt.response_code, "00");
    assert!(receipt.session_id.starts_with("tra_"));
}
