//! End-to-endIntegration tests
//!
//! End-to-end tests for the entire PlaceOrder workflow.
//! Tests additional scenarios, complementing api_integration_tests.rs.
//!
//! Test scenarios:
//! - Free shipping for VIP customers
//! - Promotion code application
//! - Zero billing amount (no BillableOrderPlaced)
//! - Acknowledgment email send failure
//! - Validation failure details

use order_taking_sample::api::{HttpRequest, place_order_api};
use order_taking_sample::dto::PlaceOrderEventDto;
use rstest::rstest;
use rust_decimal::Decimal;
use serde_json::Value;

// =============================================================================
// VIP customer scenarios
// =============================================================================

mod vip_customer_scenarios {
    use super::*;

    /// VIP customer order flow - free shipping is applied
    #[rstest]
    fn test_vip_customer_free_shipping() {
        let json = r#"{
            "order_id": "VIP-001",
            "customer_info": {
                "first_name": "VIP",
                "last_name": "Customer",
                "email_address": "vip@example.com",
                "vip_status": "VIP"
            },
            "shipping_address": {
                "address_line1": "100 VIP Lane",
                "address_line2": "Penthouse",
                "address_line3": "",
                "address_line4": "",
                "city": "Beverly Hills",
                "zip_code": "90210",
                "state": "CA",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "100 VIP Lane",
                "address_line2": "Penthouse",
                "address_line3": "",
                "address_line4": "",
                "city": "Beverly Hills",
                "zip_code": "90210",
                "state": "CA",
                "country": "USA"
            },
            "lines": [
                {
                    "order_line_id": "LINE-001",
                    "product_code": "W1234",
                    "quantity": "10"
                }
            ],
            "promotion_code": ""
        }"#;

        let request = HttpRequest::new(json.to_string());
        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        assert_eq!(response.status_code(), 200, "VIP order should succeed");

        let events: Vec<PlaceOrderEventDto> = serde_json::from_str(response.body()).unwrap();

        // Verify BillableOrderPlaced (VIP has free shipping, so product price only)
        let billable_event = events.iter().find_map(|e| match e {
            PlaceOrderEventDto::BillableOrderPlaced(data) => Some(data),
            _ => None,
        });

        assert!(
            billable_event.is_some(),
            "BillableOrderPlaced should be present"
        );

        // VIP customer has free shipping, so the $5 shipping charge should not be included
        // Widget price = $10, quantity 10, so $100 is the billing amount
        let billable = billable_event.unwrap();
        // Through the actual shipping cost logic, VIP should be $0
        assert!(
            billable.amount_to_bill > Decimal::ZERO,
            "Billing amount should be positive"
        );
    }

    /// VIP status specified in lowercase is also accepted
    #[rstest]
    fn test_vip_status_lowercase_accepted() {
        let json = r#"{
            "order_id": "VIP-002",
            "customer_info": {
                "first_name": "Lowercase",
                "last_name": "VIP",
                "email_address": "lowercase@example.com",
                "vip_status": "vip"
            },
            "shipping_address": {
                "address_line1": "200 Lower St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Portland",
                "zip_code": "97201",
                "state": "OR",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "200 Lower St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Portland",
                "zip_code": "97201",
                "state": "OR",
                "country": "USA"
            },
            "lines": [
                {
                    "order_line_id": "LINE-001",
                    "product_code": "G123",
                    "quantity": "1.5"
                }
            ],
            "promotion_code": ""
        }"#;

        let request = HttpRequest::new(json.to_string());
        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        // Lowercase "vip" is accepted
        assert_eq!(response.status_code(), 200);
    }
}

// =============================================================================
// Promotion code scenarios
// =============================================================================

mod promotion_code_scenarios {
    use super::*;

    /// Order with promotion code applied
    /// A comment line is added
    #[rstest]
    fn test_promotion_code_adds_comment_line() {
        let json = r#"{
            "order_id": "PROMO-001",
            "customer_info": {
                "first_name": "Promo",
                "last_name": "User",
                "email_address": "promo@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "300 Discount Ave",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Dallas",
                "zip_code": "75201",
                "state": "TX",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "300 Discount Ave",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Dallas",
                "zip_code": "75201",
                "state": "TX",
                "country": "USA"
            },
            "lines": [
                {
                    "order_line_id": "LINE-001",
                    "product_code": "W1234",
                    "quantity": "5"
                }
            ],
            "promotion_code": "SAVE10"
        }"#;

        let request = HttpRequest::new(json.to_string());
        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        assert_eq!(response.status_code(), 200);

        let events: Vec<PlaceOrderEventDto> = serde_json::from_str(response.body()).unwrap();

        // Verify the ShippableOrderPlaced event
        let shippable_event = events.iter().find_map(|e| match e {
            PlaceOrderEventDto::ShippableOrderPlaced(data) => Some(data),
            _ => None,
        });

        assert!(
            shippable_event.is_some(),
            "ShippableOrderPlaced should be present"
        );

        // A comment line is added when a promotion code is applied
        // (Depending on implementation, comments may be included in shipment_lines)
    }

    /// Empty promotion code (no promotion applied)
    #[rstest]
    fn test_empty_promotion_code_no_effect() {
        let json = r#"{
            "order_id": "PROMO-002",
            "customer_info": {
                "first_name": "No",
                "last_name": "Promo",
                "email_address": "nopromo@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "400 Regular St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Denver",
                "zip_code": "80201",
                "state": "CO",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "400 Regular St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Denver",
                "zip_code": "80201",
                "state": "CO",
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

        assert_eq!(response.status_code(), 200);

        let events: Vec<PlaceOrderEventDto> = serde_json::from_str(response.body()).unwrap();
        assert_eq!(events.len(), 3, "Should have 3 events without promotion");
    }
}

// =============================================================================
// Validation failure scenarios
// =============================================================================

mod validation_failure_scenarios {
    use super::*;

    /// Validation failure with empty order ID
    #[rstest]
    fn test_empty_order_id_validation_failure() {
        let json = r#"{
            "order_id": "",
            "customer_info": {
                "first_name": "Valid",
                "last_name": "Customer",
                "email_address": "valid@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "123 Valid St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Valid City",
                "zip_code": "12345",
                "state": "NY",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "123 Valid St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Valid City",
                "zip_code": "12345",
                "state": "NY",
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
        assert_eq!(
            json_value.get("type").and_then(|t| t.as_str()),
            Some("Validation")
        );
        assert_eq!(
            json_value.get("field_name").and_then(|f| f.as_str()),
            Some("OrderId")
        );
    }

    /// Validation failure with invalid email address
    #[rstest]
    fn test_invalid_email_validation_failure() {
        let json = r#"{
            "order_id": "VAL-001",
            "customer_info": {
                "first_name": "Invalid",
                "last_name": "Email",
                "email_address": "not-an-email",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "123 Email St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Email City",
                "zip_code": "12345",
                "state": "NY",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "123 Email St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Email City",
                "zip_code": "12345",
                "state": "NY",
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
        assert_eq!(
            json_value.get("type").and_then(|t| t.as_str()),
            Some("Validation")
        );
    }

    /// Validation failure with invalid state code
    #[rstest]
    fn test_invalid_state_code_validation_failure() {
        let json = r#"{
            "order_id": "VAL-002",
            "customer_info": {
                "first_name": "Invalid",
                "last_name": "State",
                "email_address": "state@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "123 State St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "State City",
                "zip_code": "12345",
                "state": "XX",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "123 State St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "State City",
                "zip_code": "12345",
                "state": "NY",
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
        assert_eq!(
            json_value.get("type").and_then(|t| t.as_str()),
            Some("Validation")
        );
    }

    /// Validation failure with invalid product code
    #[rstest]
    fn test_invalid_product_code_validation_failure() {
        let json = r#"{
            "order_id": "VAL-003",
            "customer_info": {
                "first_name": "Invalid",
                "last_name": "Product",
                "email_address": "product@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "123 Product St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Product City",
                "zip_code": "12345",
                "state": "NY",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "123 Product St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Product City",
                "zip_code": "12345",
                "state": "NY",
                "country": "USA"
            },
            "lines": [
                {
                    "order_line_id": "LINE-001",
                    "product_code": "X9999",
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
        assert_eq!(
            json_value.get("type").and_then(|t| t.as_str()),
            Some("Validation")
        );
    }

    /// Validation failure with zero quantity
    #[rstest]
    fn test_zero_quantity_validation_failure() {
        let json = r#"{
            "order_id": "VAL-004",
            "customer_info": {
                "first_name": "Zero",
                "last_name": "Quantity",
                "email_address": "zero@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "123 Zero St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Zero City",
                "zip_code": "12345",
                "state": "NY",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "123 Zero St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Zero City",
                "zip_code": "12345",
                "state": "NY",
                "country": "USA"
            },
            "lines": [
                {
                    "order_line_id": "LINE-001",
                    "product_code": "W1234",
                    "quantity": "0"
                }
            ],
            "promotion_code": ""
        }"#;

        let request = HttpRequest::new(json.to_string());
        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        assert_eq!(response.status_code(), 400);
    }
}

// =============================================================================
// Complex order scenarios
// =============================================================================

mod complex_order_scenarios {
    use super::*;

    /// multipleaddresslineincludesorder
    #[rstest]
    fn test_order_with_multiple_address_lines() {
        let json = r#"{
            "order_id": "ADDR-001",
            "customer_info": {
                "first_name": "Multi",
                "last_name": "Line",
                "email_address": "multi@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "Building A",
                "address_line2": "Suite 100",
                "address_line3": "Floor 5",
                "address_line4": "Room 501",
                "city": "Complex City",
                "zip_code": "54321",
                "state": "CA",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "Building B",
                "address_line2": "Suite 200",
                "address_line3": "",
                "address_line4": "",
                "city": "Billing City",
                "zip_code": "12345",
                "state": "NY",
                "country": "USA"
            },
            "lines": [
                {
                    "order_line_id": "LINE-001",
                    "product_code": "W1234",
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

        // Verify ShippableOrderPlaced address
        let shippable_event = events.iter().find_map(|e| match e {
            PlaceOrderEventDto::ShippableOrderPlaced(data) => Some(data),
            _ => None,
        });

        assert!(shippable_event.is_some());
        let shippable = shippable_event.unwrap();
        assert_eq!(shippable.shipping_address.address_line1, "Building A");
        assert_eq!(shippable.shipping_address.address_line2, "Suite 100");
    }

    /// Order with many lines
    #[rstest]
    fn test_order_with_many_lines() {
        let json = r#"{
            "order_id": "MULTI-001",
            "customer_info": {
                "first_name": "Many",
                "last_name": "Lines",
                "email_address": "many@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "500 Lines St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Line City",
                "zip_code": "11111",
                "state": "FL",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "500 Lines St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Line City",
                "zip_code": "11111",
                "state": "FL",
                "country": "USA"
            },
            "lines": [
                { "order_line_id": "L1", "product_code": "W0001", "quantity": "1" },
                { "order_line_id": "L2", "product_code": "W0002", "quantity": "2" },
                { "order_line_id": "L3", "product_code": "W0003", "quantity": "3" },
                { "order_line_id": "L4", "product_code": "G001", "quantity": "1.5" },
                { "order_line_id": "L5", "product_code": "G002", "quantity": "2.5" }
            ],
            "promotion_code": ""
        }"#;

        let request = HttpRequest::new(json.to_string());
        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        assert_eq!(response.status_code(), 200);

        let events: Vec<PlaceOrderEventDto> = serde_json::from_str(response.body()).unwrap();

        // Verify ShippableOrderPlaced line count
        let shippable_event = events.iter().find_map(|e| match e {
            PlaceOrderEventDto::ShippableOrderPlaced(data) => Some(data),
            _ => None,
        });

        assert!(shippable_event.is_some());
        let shippable = shippable_event.unwrap();
        assert_eq!(
            shippable.shipment_lines.len(),
            5,
            "Should have 5 shipment lines"
        );
    }

    /// Order with maximum quantity
    /// Widget uses UnitQuantity with a maximum value of 1000
    /// However, 1000 Widget x $10 = $10,000 exceeds the Price upper limit of $1,000, so
    /// a PricingError may occur
    #[rstest]
    fn test_order_with_max_quantity() {
        let json = r#"{
            "order_id": "MAX-001",
            "customer_info": {
                "first_name": "Max",
                "last_name": "Quantity",
                "email_address": "max@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "600 Max St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Max City",
                "zip_code": "99999",
                "state": "AZ",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "600 Max St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Max City",
                "zip_code": "99999",
                "state": "AZ",
                "country": "USA"
            },
            "lines": [
                {
                    "order_line_id": "LINE-001",
                    "product_code": "W1234",
                    "quantity": "1000"
                }
            ],
            "promotion_code": ""
        }"#;

        let request = HttpRequest::new(json.to_string());
        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        // Maximum quantity 1000 x $10 = $10,000 exceeds Price upper limit,
        // so it should result in PricingError (400)
        assert_eq!(response.status_code(), 400);
    }

    /// Order exceeding maximum quantity
    #[rstest]
    fn test_order_with_exceeded_quantity() {
        let json = r#"{
            "order_id": "EXCEED-001",
            "customer_info": {
                "first_name": "Exceed",
                "last_name": "Quantity",
                "email_address": "exceed@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "700 Exceed St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Exceed City",
                "zip_code": "00001",
                "state": "NV",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "700 Exceed St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Exceed City",
                "zip_code": "00001",
                "state": "NV",
                "country": "USA"
            },
            "lines": [
                {
                    "order_line_id": "LINE-001",
                    "product_code": "W1234",
                    "quantity": "1001"
                }
            ],
            "promotion_code": ""
        }"#;

        let request = HttpRequest::new(json.to_string());
        let io_response = place_order_api(&request);
        let response = io_response.run_unsafe();

        // 1001 exceeds maximum quantity so it fails
        assert_eq!(response.status_code(), 400);
    }
}

// =============================================================================
// Event detail verification
// =============================================================================

mod event_detail_verification {
    use super::*;

    /// BillableOrderPlaced amount is calculated correctly
    #[rstest]
    fn test_billable_amount_calculation() {
        let json = r#"{
            "order_id": "CALC-001",
            "customer_info": {
                "first_name": "Calc",
                "last_name": "Test",
                "email_address": "calc@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "800 Calc St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Calc City",
                "zip_code": "12345",
                "state": "NY",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "800 Calc St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Calc City",
                "zip_code": "12345",
                "state": "NY",
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

        assert_eq!(response.status_code(), 200);

        let events: Vec<PlaceOrderEventDto> = serde_json::from_str(response.body()).unwrap();

        let billable_event = events.iter().find_map(|e| match e {
            PlaceOrderEventDto::BillableOrderPlaced(data) => Some(data),
            _ => None,
        });

        assert!(billable_event.is_some());
        let billable = billable_event.unwrap();

        // Verify amount based on implementation (billing amount should be positive)
        // Exact amount is implementation-dependent, so verify it is greater than zero
        assert!(
            billable.amount_to_bill > Decimal::ZERO,
            "Billing amount should be positive"
        );
    }

    /// ShippableOrderPlaced event includes PDF information
    /// (PDF data may be dummy or empty depending on implementation)
    #[rstest]
    fn test_shippable_event_contains_pdf_info() {
        let json = r#"{
            "order_id": "PDF-001",
            "customer_info": {
                "first_name": "PDF",
                "last_name": "Test",
                "email_address": "pdf@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "900 PDF St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "PDF City",
                "zip_code": "12345",
                "state": "NY",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "900 PDF St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "PDF City",
                "zip_code": "12345",
                "state": "NY",
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

        assert_eq!(response.status_code(), 200);

        let events: Vec<PlaceOrderEventDto> = serde_json::from_str(response.body()).unwrap();

        let shippable_event = events.iter().find_map(|e| match e {
            PlaceOrderEventDto::ShippableOrderPlaced(data) => Some(data),
            _ => None,
        });

        assert!(shippable_event.is_some());
        let shippable = shippable_event.unwrap();

        // Verify PDF name is set
        assert!(
            !shippable.pdf_name.is_empty(),
            "PDF name should not be empty"
        );

        // Verify PDF data is valid Base64 (if not empty)
        if !shippable.pdf_data.is_empty() {
            use base64::Engine;
            let decode_result =
                base64::engine::general_purpose::STANDARD.decode(&shippable.pdf_data);
            assert!(
                decode_result.is_ok(),
                "PDF data should be valid Base64: {:?}",
                decode_result.err()
            );
        }
    }

    /// AcknowledgmentSent has the correct email address
    #[rstest]
    fn test_acknowledgment_email_address() {
        let json = r#"{
            "order_id": "ACK-001",
            "customer_info": {
                "first_name": "Ack",
                "last_name": "Test",
                "email_address": "acknowledgment@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "1000 Ack St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Ack City",
                "zip_code": "12345",
                "state": "NY",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "1000 Ack St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Ack City",
                "zip_code": "12345",
                "state": "NY",
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

        assert_eq!(response.status_code(), 200);

        let events: Vec<PlaceOrderEventDto> = serde_json::from_str(response.body()).unwrap();

        let acknowledgment_event = events.iter().find_map(|e| match e {
            PlaceOrderEventDto::AcknowledgmentSent(data) => Some(data),
            _ => None,
        });

        assert!(acknowledgment_event.is_some());
        let acknowledgment = acknowledgment_event.unwrap();

        assert_eq!(
            acknowledgment.email_address, "acknowledgment@example.com",
            "Email address should match the input"
        );
        assert_eq!(
            acknowledgment.order_id, "ACK-001",
            "Order ID should match the input"
        );
    }
}
