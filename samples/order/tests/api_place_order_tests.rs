//! place_order_api のテスト
//!
//! place_order_api 関数のテスト

use order_taking_sample::api::{HttpRequest, place_order_api};
use order_taking_sample::dto::PlaceOrderEventDto;
use rstest::rstest;

// =============================================================================
// 正常系テスト
// =============================================================================

mod success_tests {
    use super::*;

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
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "123 Main St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "New York",
                "zip_code": "10001",
                "state": "NY",
                "country": "USA"
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

    #[rstest]
    fn test_valid_order_returns_200() {
        let request = HttpRequest::new(create_valid_order_json());

        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        assert_eq!(response.status_code(), 200);
    }

    #[rstest]
    fn test_valid_order_returns_events() {
        let request = HttpRequest::new(create_valid_order_json());

        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        // レスポンスボディをデシリアライズ
        let events: Vec<PlaceOrderEventDto> = serde_json::from_str(response.body()).unwrap();

        // 3つのイベント（Shippable, Billable, Acknowledgment）が返される
        assert_eq!(events.len(), 3);
    }

    #[rstest]
    fn test_valid_order_contains_shippable_event() {
        let request = HttpRequest::new(create_valid_order_json());

        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        let events: Vec<PlaceOrderEventDto> = serde_json::from_str(response.body()).unwrap();

        let has_shippable = events
            .iter()
            .any(|e| matches!(e, PlaceOrderEventDto::ShippableOrderPlaced(_)));

        assert!(has_shippable);
    }

    #[rstest]
    fn test_valid_order_contains_billable_event() {
        let request = HttpRequest::new(create_valid_order_json());

        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        let events: Vec<PlaceOrderEventDto> = serde_json::from_str(response.body()).unwrap();

        let has_billable = events
            .iter()
            .any(|e| matches!(e, PlaceOrderEventDto::BillableOrderPlaced(_)));

        assert!(has_billable);
    }

    #[rstest]
    fn test_valid_order_contains_acknowledgment_event() {
        let request = HttpRequest::new(create_valid_order_json());

        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        let events: Vec<PlaceOrderEventDto> = serde_json::from_str(response.body()).unwrap();

        let has_acknowledgment = events
            .iter()
            .any(|e| matches!(e, PlaceOrderEventDto::AcknowledgmentSent(_)));

        assert!(has_acknowledgment);
    }

    #[rstest]
    fn test_valid_order_with_multiple_lines() {
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
                "address_line2": "Suite 100",
                "address_line3": "",
                "address_line4": "",
                "city": "Los Angeles",
                "zip_code": "90001",
                "state": "CA",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "456 Oak Ave",
                "address_line2": "Suite 100",
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
                    "product_code": "W1234",
                    "quantity": "5"
                },
                {
                    "order_line_id": "line-002",
                    "product_code": "G123",
                    "quantity": "2.5"
                }
            ],
            "promotion_code": ""
        }"#;

        let request = HttpRequest::new(json.to_string());

        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        assert_eq!(response.status_code(), 200);
    }
}

// =============================================================================
// JSON パースエラーテスト
// =============================================================================

mod json_parse_error_tests {
    use super::*;

    #[rstest]
    fn test_invalid_json_returns_400() {
        let request = HttpRequest::new("not valid json".to_string());

        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        assert_eq!(response.status_code(), 400);
    }

    #[rstest]
    fn test_invalid_json_returns_error_message() {
        let request = HttpRequest::new("{invalid json}".to_string());

        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        assert!(response.body().contains("JsonParseError"));
    }

    #[rstest]
    fn test_empty_body_returns_400() {
        let request = HttpRequest::new(String::new());

        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        assert_eq!(response.status_code(), 400);
    }

    #[rstest]
    fn test_missing_required_field_returns_400() {
        let json = r#"{
            "customer_info": {
                "first_name": "John",
                "last_name": "Doe",
                "email_address": "john@example.com",
                "vip_status": "Normal"
            }
        }"#;

        let request = HttpRequest::new(json.to_string());

        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        assert_eq!(response.status_code(), 400);
    }
}

// =============================================================================
// バリデーションエラーテスト
// =============================================================================

mod validation_error_tests {
    use super::*;

    #[rstest]
    fn test_empty_order_id_returns_400() {
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
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "123 Main St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "New York",
                "zip_code": "10001",
                "state": "NY",
                "country": "USA"
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

        let request = HttpRequest::new(json.to_string());

        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        assert_eq!(response.status_code(), 400);
        assert!(response.body().contains("Validation"));
    }

    #[rstest]
    fn test_invalid_email_returns_400() {
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
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "123 Main St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "New York",
                "zip_code": "10001",
                "state": "NY",
                "country": "USA"
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

        let request = HttpRequest::new(json.to_string());

        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        assert_eq!(response.status_code(), 400);
        assert!(response.body().contains("Validation"));
    }

    #[rstest]
    fn test_invalid_product_code_returns_400() {
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
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "123 Main St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "New York",
                "zip_code": "10001",
                "state": "NY",
                "country": "USA"
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

        let request = HttpRequest::new(json.to_string());

        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        assert_eq!(response.status_code(), 400);
        assert!(response.body().contains("Validation"));
    }

    #[rstest]
    fn test_invalid_zip_code_returns_400() {
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
                "zip_code": "1234",
                "state": "NY",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "123 Main St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "New York",
                "zip_code": "10001",
                "state": "NY",
                "country": "USA"
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

        let request = HttpRequest::new(json.to_string());

        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        assert_eq!(response.status_code(), 400);
        assert!(response.body().contains("Validation"));
    }
}

// =============================================================================
// レスポンス形式テスト
// =============================================================================

mod response_format_tests {
    use super::*;

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
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "123 Main St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "New York",
                "zip_code": "10001",
                "state": "NY",
                "country": "USA"
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

    #[rstest]
    fn test_success_response_is_valid_json() {
        let request = HttpRequest::new(create_valid_order_json());

        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        // レスポンスボディが有効な JSON であることを確認
        let parse_result: Result<serde_json::Value, _> = serde_json::from_str(response.body());
        assert!(parse_result.is_ok());
    }

    #[rstest]
    fn test_error_response_is_valid_json() {
        let request = HttpRequest::new("invalid json".to_string());

        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        // エラーレスポンスも有効な JSON であることを確認
        let parse_result: Result<serde_json::Value, _> = serde_json::from_str(response.body());
        assert!(parse_result.is_ok());
    }

    #[rstest]
    fn test_success_response_contains_type_field() {
        let request = HttpRequest::new(create_valid_order_json());

        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        // 各イベントに type フィールドが含まれている
        assert!(response.body().contains("\"type\""));
    }

    #[rstest]
    fn test_validation_error_response_format() {
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
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "123 Main St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "New York",
                "zip_code": "10001",
                "state": "NY",
                "country": "USA"
            },
            "lines": [],
            "promotion_code": ""
        }"#;

        let request = HttpRequest::new(json.to_string());

        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        // Validation エラーの形式を確認
        assert!(response.body().contains("\"type\":\"Validation\""));
        assert!(response.body().contains("\"field_name\""));
        assert!(response.body().contains("\"message\""));
    }
}
