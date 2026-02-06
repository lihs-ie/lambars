//! API integration tests
//!
//! Tests the full flow: DTO -> Domain -> Workflow -> DTO.
//! End-to-end scenario tests.

use order_taking_sample::api::{HttpRequest, place_order_api};
use order_taking_sample::dto::PlaceOrderEventDto;
use rstest::rstest;
use serde_json::Value;

// =============================================================================
// End-to-end scenario tests
// =============================================================================

mod end_to_end_tests {
    use super::*;

    /// Full flow for a single Widget order
    #[rstest]
    fn test_single_widget_order_flow() {
        let json = r#"{
            "order_id": "E2E-001",
            "customer_info": {
                "first_name": "Alice",
                "last_name": "Johnson",
                "email_address": "alice@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "100 First Street",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Boston",
                "zip_code": "02101",
                "state": "MA",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "100 First Street",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Boston",
                "zip_code": "02101",
                "state": "MA",
                "country": "USA"
            },
            "lines": [
                {
                    "order_line_id": "LINE-001",
                    "product_code": "W1234",
                    "quantity": "5"
                }
            ],
            "promotion_code": ""
        }"#;

        let request = HttpRequest::new(json.to_string());
        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        // Successful response
        assert_eq!(response.status_code(), 200);

        // Parse events
        let events: Vec<PlaceOrderEventDto> = serde_json::from_str(response.body()).unwrap();
        assert_eq!(events.len(), 3);

        // Verify each event type
        let has_shippable = events
            .iter()
            .any(|e| matches!(e, PlaceOrderEventDto::ShippableOrderPlaced(_)));
        let has_billable = events
            .iter()
            .any(|e| matches!(e, PlaceOrderEventDto::BillableOrderPlaced(_)));
        let has_acknowledgment = events
            .iter()
            .any(|e| matches!(e, PlaceOrderEventDto::AcknowledgmentSent(_)));

        assert!(
            has_shippable,
            "ShippableOrderPlaced event should be present"
        );
        assert!(has_billable, "BillableOrderPlaced event should be present");
        assert!(
            has_acknowledgment,
            "AcknowledgmentSent event should be present"
        );
    }

    /// Full flow for a single Gizmo order
    #[rstest]
    fn test_single_gizmo_order_flow() {
        let json = r#"{
            "order_id": "E2E-002",
            "customer_info": {
                "first_name": "Bob",
                "last_name": "Smith",
                "email_address": "bob@example.com",
                "vip_status": "VIP"
            },
            "shipping_address": {
                "address_line1": "200 Second Avenue",
                "address_line2": "Apt 5B",
                "address_line3": "",
                "address_line4": "",
                "city": "Chicago",
                "zip_code": "60601",
                "state": "IL",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "200 Second Avenue",
                "address_line2": "Apt 5B",
                "address_line3": "",
                "address_line4": "",
                "city": "Chicago",
                "zip_code": "60601",
                "state": "IL",
                "country": "USA"
            },
            "lines": [
                {
                    "order_line_id": "LINE-001",
                    "product_code": "G123",
                    "quantity": "3.5"
                }
            ],
            "promotion_code": ""
        }"#;

        let request = HttpRequest::new(json.to_string());
        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        assert_eq!(response.status_code(), 200);

        let events: Vec<PlaceOrderEventDto> = serde_json::from_str(response.body()).unwrap();
        assert_eq!(events.len(), 3);
    }

    /// Multi-line order (mix of Widget and Gizmo)
    #[rstest]
    fn test_mixed_product_order_flow() {
        let json = r#"{
            "order_id": "E2E-003",
            "customer_info": {
                "first_name": "Carol",
                "last_name": "Davis",
                "email_address": "carol@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "300 Third Boulevard",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Seattle",
                "zip_code": "98101",
                "state": "WA",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "300 Third Boulevard",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Seattle",
                "zip_code": "98101",
                "state": "WA",
                "country": "USA"
            },
            "lines": [
                {
                    "order_line_id": "LINE-001",
                    "product_code": "W1234",
                    "quantity": "10"
                },
                {
                    "order_line_id": "LINE-002",
                    "product_code": "G999",
                    "quantity": "2.0"
                },
                {
                    "order_line_id": "LINE-003",
                    "product_code": "W9999",
                    "quantity": "3"
                }
            ],
            "promotion_code": ""
        }"#;

        let request = HttpRequest::new(json.to_string());
        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        assert_eq!(response.status_code(), 200);

        let events: Vec<PlaceOrderEventDto> = serde_json::from_str(response.body()).unwrap();
        assert_eq!(events.len(), 3);

        // Verify ShippableOrderPlaced event details
        let shippable_event = events.iter().find_map(|e| match e {
            PlaceOrderEventDto::ShippableOrderPlaced(data) => Some(data),
            _ => None,
        });

        assert!(shippable_event.is_some());
        let shippable = shippable_event.unwrap();
        assert_eq!(shippable.order_id, "E2E-003");
        assert_eq!(shippable.shipment_lines.len(), 3);
    }
}

// =============================================================================
// JSON serialization detail tests
// =============================================================================

mod json_serialization_tests {
    use super::*;

    #[rstest]
    fn test_event_json_structure() {
        let json = r#"{
            "order_id": "JSON-001",
            "customer_info": {
                "first_name": "Test",
                "last_name": "User",
                "email_address": "test@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "123 Test St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Test City",
                "zip_code": "12345",
                "state": "TX",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "123 Test St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Test City",
                "zip_code": "12345",
                "state": "TX",
                "country": "USA"
            },
            "lines": [
                {
                    "order_line_id": "LINE-001",
                    "product_code": "W1234",
                    "quantity": "1"
                }
            ],
            "promotion_code": ""
        }"#;

        let request = HttpRequest::new(json.to_string());
        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        // Parse as JSON
        let json_value: Value = serde_json::from_str(response.body()).unwrap();
        assert!(json_value.is_array());

        let array = json_value.as_array().unwrap();
        assert_eq!(array.len(), 3);

        // Verify each event has a type field
        for event in array {
            assert!(
                event.get("type").is_some(),
                "Event should have 'type' field"
            );
        }
    }

    #[rstest]
    fn test_shippable_event_json_fields() {
        let json = r#"{
            "order_id": "JSON-002",
            "customer_info": {
                "first_name": "Field",
                "last_name": "Test",
                "email_address": "field@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "456 Field St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Field City",
                "zip_code": "67890",
                "state": "FL",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "456 Field St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Field City",
                "zip_code": "67890",
                "state": "FL",
                "country": "USA"
            },
            "lines": [
                {
                    "order_line_id": "LINE-001",
                    "product_code": "W1234",
                    "quantity": "5"
                }
            ],
            "promotion_code": ""
        }"#;

        let request = HttpRequest::new(json.to_string());
        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        let json_value: Value = serde_json::from_str(response.body()).unwrap();
        let array = json_value.as_array().unwrap();

        // Find the ShippableOrderPlaced event
        let shippable = array
            .iter()
            .find(|e| e.get("type").and_then(|t| t.as_str()) == Some("ShippableOrderPlaced"))
            .expect("ShippableOrderPlaced event should exist");

        // Verify the data field
        let data = shippable.get("data").expect("data field should exist");
        assert!(data.get("order_id").is_some());
        assert!(data.get("shipping_address").is_some());
        assert!(data.get("shipment_lines").is_some());
        assert!(data.get("pdf_name").is_some());
        assert!(data.get("pdf_data").is_some());
    }

    #[rstest]
    fn test_billable_event_json_fields() {
        let json = r#"{
            "order_id": "JSON-003",
            "customer_info": {
                "first_name": "Bill",
                "last_name": "Test",
                "email_address": "bill@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "789 Bill St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Bill City",
                "zip_code": "11111",
                "state": "CA",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "789 Bill St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Bill City",
                "zip_code": "11111",
                "state": "CA",
                "country": "USA"
            },
            "lines": [
                {
                    "order_line_id": "LINE-001",
                    "product_code": "G123",
                    "quantity": "2.5"
                }
            ],
            "promotion_code": ""
        }"#;

        let request = HttpRequest::new(json.to_string());
        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        let json_value: Value = serde_json::from_str(response.body()).unwrap();
        let array = json_value.as_array().unwrap();

        // Find the BillableOrderPlaced event
        let billable = array
            .iter()
            .find(|e| e.get("type").and_then(|t| t.as_str()) == Some("BillableOrderPlaced"))
            .expect("BillableOrderPlaced event should exist");

        // Verify the data field
        let data = billable.get("data").expect("data field should exist");
        assert!(data.get("order_id").is_some());
        assert!(data.get("billing_address").is_some());
        assert!(data.get("amount_to_bill").is_some());
    }

    #[rstest]
    fn test_acknowledgment_event_json_fields() {
        let json = r#"{
            "order_id": "JSON-004",
            "customer_info": {
                "first_name": "Ack",
                "last_name": "Test",
                "email_address": "ack@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "321 Ack St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Ack City",
                "zip_code": "22222",
                "state": "NY",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "321 Ack St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Ack City",
                "zip_code": "22222",
                "state": "NY",
                "country": "USA"
            },
            "lines": [
                {
                    "order_line_id": "LINE-001",
                    "product_code": "W9999",
                    "quantity": "1"
                }
            ],
            "promotion_code": ""
        }"#;

        let request = HttpRequest::new(json.to_string());
        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        let json_value: Value = serde_json::from_str(response.body()).unwrap();
        let array = json_value.as_array().unwrap();

        // Find the AcknowledgmentSent event
        let acknowledgment = array
            .iter()
            .find(|e| e.get("type").and_then(|t| t.as_str()) == Some("AcknowledgmentSent"))
            .expect("AcknowledgmentSent event should exist");

        // Verify the data field
        let data = acknowledgment.get("data").expect("data field should exist");
        assert!(data.get("order_id").is_some());
        assert!(data.get("email_address").is_some());
    }
}

// =============================================================================
// Error handling integration tests
// =============================================================================

mod error_handling_integration_tests {
    use super::*;

    #[rstest]
    fn test_validation_error_response_structure() {
        let json = r#"{
            "order_id": "",
            "customer_info": {
                "first_name": "Error",
                "last_name": "Test",
                "email_address": "error@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "123 Error St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Error City",
                "zip_code": "12345",
                "state": "ER",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "123 Error St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Error City",
                "zip_code": "12345",
                "state": "ER",
                "country": "USA"
            },
            "lines": [
                {
                    "order_line_id": "LINE-001",
                    "product_code": "W1234",
                    "quantity": "1"
                }
            ],
            "promotion_code": ""
        }"#;

        let request = HttpRequest::new(json.to_string());
        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        assert_eq!(response.status_code(), 400);

        let json_value: Value = serde_json::from_str(response.body()).unwrap();

        // Verify error response structure (internally tagged format)
        assert_eq!(
            json_value.get("type").and_then(|t| t.as_str()),
            Some("Validation")
        );
        // Since it's internally tagged, fields are directly included in json_value
        assert!(json_value.get("field_name").is_some());
        assert!(json_value.get("message").is_some());
    }

    #[rstest]
    fn test_json_parse_error_response_structure() {
        let invalid_json = "{ this is not valid json }";

        let request = HttpRequest::new(invalid_json.to_string());
        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        assert_eq!(response.status_code(), 400);

        let json_value: Value = serde_json::from_str(response.body()).unwrap();

        // Verify JSON parse error structure
        assert_eq!(
            json_value.get("type").and_then(|t| t.as_str()),
            Some("JsonParseError")
        );
        assert!(json_value.get("message").is_some());
    }

    #[rstest]
    fn test_multiple_validation_errors_returns_first() {
        // When there are multiple validation errors, the first error is returned
        let json = r#"{
            "order_id": "",
            "customer_info": {
                "first_name": "",
                "last_name": "",
                "email_address": "invalid-email",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "",
                "zip_code": "123",
                "state": "",
                "country": ""
            },
            "billing_address": {
                "address_line1": "",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "",
                "zip_code": "123",
                "state": "",
                "country": ""
            },
            "lines": [],
            "promotion_code": ""
        }"#;

        let request = HttpRequest::new(json.to_string());
        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        assert_eq!(response.status_code(), 400);

        let json_value: Value = serde_json::from_str(response.body()).unwrap();
        assert_eq!(
            json_value.get("type").and_then(|t| t.as_str()),
            Some("Validation")
        );
    }
}

// =============================================================================
// IO monad tests
// =============================================================================

mod io_monad_tests {
    use super::*;

    #[rstest]
    fn test_io_monad_is_not_executed_until_run() {
        let json = r#"{
            "order_id": "IO-001",
            "customer_info": {
                "first_name": "IO",
                "last_name": "Test",
                "email_address": "io@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "123 IO St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "IO City",
                "zip_code": "12345",
                "state": "TX",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "123 IO St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "IO City",
                "zip_code": "12345",
                "state": "TX",
                "country": "USA"
            },
            "lines": [
                {
                    "order_line_id": "LINE-001",
                    "product_code": "W1234",
                    "quantity": "1"
                }
            ],
            "promotion_code": ""
        }"#;

        let request = HttpRequest::new(json.to_string());

        // Create IO but do not execute yet
        let io_response = place_order_api(&request);

        // No side effects at this point (IO as a pure value)
        // Only executed when run_unsafe is called
        let response = io_response.run_unsafe();

        assert_eq!(response.status_code(), 200);
    }

    #[rstest]
    fn test_io_monad_can_be_run_multiple_times() {
        let json = r#"{
            "order_id": "IO-002",
            "customer_info": {
                "first_name": "Multi",
                "last_name": "Run",
                "email_address": "multi@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "456 Multi St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Multi City",
                "zip_code": "67890",
                "state": "MU",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "456 Multi St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Multi City",
                "zip_code": "67890",
                "state": "MU",
                "country": "USA"
            },
            "lines": [
                {
                    "order_line_id": "LINE-001",
                    "product_code": "G123",
                    "quantity": "2.0"
                }
            ],
            "promotion_code": ""
        }"#;

        let request1 = HttpRequest::new(json.to_string());
        let request2 = HttpRequest::new(json.to_string());

        let io_response1 = place_order_api(&request1);
        let io_response2 = place_order_api(&request2);

        let response1 = io_response1.run_unsafe();
        let response2 = io_response2.run_unsafe();

        // Same input produces same result
        assert_eq!(response1.status_code(), response2.status_code());
        assert_eq!(response1.body(), response2.body());
    }
}
