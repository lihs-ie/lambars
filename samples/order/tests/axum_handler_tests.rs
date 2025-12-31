//! axum ハンドラ統合テスト
//!
//! axum ハンドラの動作を検証する。

use axum::http::StatusCode;
use axum::response::IntoResponse;
use order_taking_sample::api::axum_handler::place_order_handler;
use rstest::rstest;

/// 有効な注文 JSON を生成するヘルパー関数
fn create_valid_order_json() -> String {
    r#"{
        "order_id": "order-001",
        "customer_info": {
            "first_name": "John",
            "last_name": "Doe",
            "email_address": "john@example.com",
            "vip_status": "Normal"
        },
        "shipping_address": {
            "address_line1": "123 Main St",
            "address_line2": "",
            "address_line3": "",
            "address_line4": "",
            "city": "New York",
            "zip_code": "10001",
            "state": "NY",
            "country": "US"
        },
        "billing_address": {
            "address_line1": "123 Main St",
            "address_line2": "",
            "address_line3": "",
            "address_line4": "",
            "city": "New York",
            "zip_code": "10001",
            "state": "NY",
            "country": "US"
        },
        "lines": [
            {
                "order_line_id": "line-001",
                "product_code": "W1234",
                "quantity": "10"
            }
        ],
        "promotion_code": ""
    }"#
    .to_string()
}

// =============================================================================
// 成功ケース
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_place_order_handler_success_returns_200() {
    let json = create_valid_order_json();
    let response = place_order_handler(json).await;
    let response = response.into_response();
    assert_eq!(response.status(), StatusCode::OK);
}

#[rstest]
#[tokio::test]
async fn test_place_order_handler_success_with_gizmo_product() {
    // Gizmo 製品は G + 3桁の数字、数量は小数（キログラム）
    let json = r#"{
        "order_id": "order-002",
        "customer_info": {
            "first_name": "Jane",
            "last_name": "Smith",
            "email_address": "jane@example.com",
            "vip_status": "VIP"
        },
        "shipping_address": {
            "address_line1": "456 Oak Ave",
            "address_line2": "",
            "address_line3": "",
            "address_line4": "",
            "city": "Los Angeles",
            "zip_code": "90001",
            "state": "CA",
            "country": "USA"
        },
        "billing_address": {
            "address_line1": "456 Oak Ave",
            "address_line2": "",
            "address_line3": "",
            "address_line4": "",
            "city": "Los Angeles",
            "zip_code": "90001",
            "state": "CA",
            "country": "USA"
        },
        "lines": [
            {
                "order_line_id": "line-001",
                "product_code": "G123",
                "quantity": "3.5"
            }
        ],
        "promotion_code": ""
    }"#;

    let response = place_order_handler(json.to_string()).await;
    let response = response.into_response();
    assert_eq!(response.status(), StatusCode::OK);
}

// =============================================================================
// JSON パースエラー
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_place_order_handler_invalid_json_returns_400() {
    let invalid_json = "{ invalid json }";
    let response = place_order_handler(invalid_json.to_string()).await;
    let response = response.into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[rstest]
#[tokio::test]
async fn test_place_order_handler_empty_body_returns_400() {
    let response = place_order_handler(String::new()).await;
    let response = response.into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// =============================================================================
// バリデーションエラー
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_place_order_handler_empty_order_id_returns_400() {
    let json = r#"{
        "order_id": "",
        "customer_info": {
            "first_name": "John",
            "last_name": "Doe",
            "email_address": "john@example.com",
            "vip_status": "Normal"
        },
        "shipping_address": {
            "address_line1": "123 Main St",
            "address_line2": "",
            "address_line3": "",
            "address_line4": "",
            "city": "New York",
            "zip_code": "10001",
            "state": "NY",
            "country": "US"
        },
        "billing_address": {
            "address_line1": "123 Main St",
            "address_line2": "",
            "address_line3": "",
            "address_line4": "",
            "city": "New York",
            "zip_code": "10001",
            "state": "NY",
            "country": "US"
        },
        "lines": [
            {
                "order_line_id": "line-001",
                "product_code": "W1234",
                "quantity": "10"
            }
        ],
        "promotion_code": ""
    }"#;

    let response = place_order_handler(json.to_string()).await;
    let response = response.into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[rstest]
#[tokio::test]
async fn test_place_order_handler_invalid_product_code_returns_400() {
    let json = r#"{
        "order_id": "order-001",
        "customer_info": {
            "first_name": "John",
            "last_name": "Doe",
            "email_address": "john@example.com",
            "vip_status": "Normal"
        },
        "shipping_address": {
            "address_line1": "123 Main St",
            "address_line2": "",
            "address_line3": "",
            "address_line4": "",
            "city": "New York",
            "zip_code": "10001",
            "state": "NY",
            "country": "US"
        },
        "billing_address": {
            "address_line1": "123 Main St",
            "address_line2": "",
            "address_line3": "",
            "address_line4": "",
            "city": "New York",
            "zip_code": "10001",
            "state": "NY",
            "country": "US"
        },
        "lines": [
            {
                "order_line_id": "line-001",
                "product_code": "INVALID",
                "quantity": "10"
            }
        ],
        "promotion_code": ""
    }"#;

    let response = place_order_handler(json.to_string()).await;
    let response = response.into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[rstest]
#[tokio::test]
async fn test_place_order_handler_invalid_email_returns_400() {
    let json = r#"{
        "order_id": "order-001",
        "customer_info": {
            "first_name": "John",
            "last_name": "Doe",
            "email_address": "invalid-email",
            "vip_status": "Normal"
        },
        "shipping_address": {
            "address_line1": "123 Main St",
            "address_line2": "",
            "address_line3": "",
            "address_line4": "",
            "city": "New York",
            "zip_code": "10001",
            "state": "NY",
            "country": "US"
        },
        "billing_address": {
            "address_line1": "123 Main St",
            "address_line2": "",
            "address_line3": "",
            "address_line4": "",
            "city": "New York",
            "zip_code": "10001",
            "state": "NY",
            "country": "US"
        },
        "lines": [
            {
                "order_line_id": "line-001",
                "product_code": "W1234",
                "quantity": "10"
            }
        ],
        "promotion_code": ""
    }"#;

    let response = place_order_handler(json.to_string()).await;
    let response = response.into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
