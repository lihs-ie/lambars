//! Tests for error DTOs
//!
//! Tests for PlaceOrderErrorDto

use order_taking_sample::dto::PlaceOrderErrorDto;
use order_taking_sample::simple_types::ValidationError;
use order_taking_sample::workflow::{
    PlaceOrderError, PricingError, RemoteServiceError, ServiceInfo,
};
use rstest::rstest;

// =============================================================================
// Tests for PlaceOrderErrorDto
// =============================================================================

mod place_order_error_dto_tests {
    use super::*;

    #[rstest]
    fn test_from_domain_validation_error() {
        let validation_error = ValidationError::new("OrderId", "cannot be empty");
        let error = PlaceOrderError::Validation(validation_error);
        let dto = PlaceOrderErrorDto::from_domain(&error);

        match dto {
            PlaceOrderErrorDto::Validation {
                field_name,
                message,
            } => {
                assert_eq!(field_name, "OrderId");
                assert_eq!(message, "cannot be empty");
            }
            _ => panic!("Expected Validation error"),
        }
    }

    #[rstest]
    fn test_from_domain_pricing_error() {
        let pricing_error = PricingError::new("Product not found: W9999");
        let error = PlaceOrderError::Pricing(pricing_error);
        let dto = PlaceOrderErrorDto::from_domain(&error);

        match dto {
            PlaceOrderErrorDto::Pricing { message } => {
                assert_eq!(message, "Product not found: W9999");
            }
            _ => panic!("Expected Pricing error"),
        }
    }

    #[rstest]
    fn test_from_domain_remote_service_error() {
        let service = ServiceInfo::new(
            "AddressValidation".to_string(),
            "https://api.example.com/validate".to_string(),
        );
        let remote_error = RemoteServiceError::new(service, "Connection timeout".to_string());
        let error = PlaceOrderError::RemoteService(remote_error);
        let dto = PlaceOrderErrorDto::from_domain(&error);

        match dto {
            PlaceOrderErrorDto::RemoteService {
                service_name,
                service_endpoint,
                message,
            } => {
                assert_eq!(service_name, "AddressValidation");
                assert_eq!(service_endpoint, "https://api.example.com/validate");
                assert_eq!(message, "Connection timeout");
            }
            _ => panic!("Expected RemoteService error"),
        }
    }

    #[rstest]
    fn test_serialize_validation_error() {
        let validation_error = ValidationError::new("EmailAddress", "invalid email format");
        let error = PlaceOrderError::Validation(validation_error);
        let dto = PlaceOrderErrorDto::from_domain(&error);

        let json = serde_json::to_string(&dto).unwrap();

        assert!(json.contains("\"type\":\"Validation\""));
        assert!(json.contains("\"field_name\":\"EmailAddress\""));
        assert!(json.contains("\"message\":\"invalid email format\""));
    }

    #[rstest]
    fn test_serialize_pricing_error() {
        let pricing_error = PricingError::new("Invalid promotion code");
        let error = PlaceOrderError::Pricing(pricing_error);
        let dto = PlaceOrderErrorDto::from_domain(&error);

        let json = serde_json::to_string(&dto).unwrap();

        assert!(json.contains("\"type\":\"Pricing\""));
        assert!(json.contains("\"message\":\"Invalid promotion code\""));
    }

    #[rstest]
    fn test_serialize_remote_service_error() {
        let service = ServiceInfo::new(
            "PricingService".to_string(),
            "https://pricing.example.com".to_string(),
        );
        let remote_error =
            RemoteServiceError::new(service, "500 Internal Server Error".to_string());
        let error = PlaceOrderError::RemoteService(remote_error);
        let dto = PlaceOrderErrorDto::from_domain(&error);

        let json = serde_json::to_string(&dto).unwrap();

        assert!(json.contains("\"type\":\"RemoteService\""));
        assert!(json.contains("\"service_name\":\"PricingService\""));
        assert!(json.contains("\"service_endpoint\":\"https://pricing.example.com\""));
        assert!(json.contains("\"message\":\"500 Internal Server Error\""));
    }

    #[rstest]
    fn test_deserialize_validation_error() {
        let json = r#"{
            "type": "Validation",
            "field_name": "ZipCode",
            "message": "must be 5 digits"
        }"#;

        let dto: PlaceOrderErrorDto = serde_json::from_str(json).unwrap();

        match dto {
            PlaceOrderErrorDto::Validation {
                field_name,
                message,
            } => {
                assert_eq!(field_name, "ZipCode");
                assert_eq!(message, "must be 5 digits");
            }
            _ => panic!("Expected Validation error"),
        }
    }

    #[rstest]
    fn test_deserialize_pricing_error() {
        let json = r#"{
            "type": "Pricing",
            "message": "Price out of range"
        }"#;

        let dto: PlaceOrderErrorDto = serde_json::from_str(json).unwrap();

        match dto {
            PlaceOrderErrorDto::Pricing { message } => {
                assert_eq!(message, "Price out of range");
            }
            _ => panic!("Expected Pricing error"),
        }
    }

    #[rstest]
    fn test_deserialize_remote_service_error() {
        let json = r#"{
            "type": "RemoteService",
            "service_name": "AuthService",
            "service_endpoint": "https://auth.example.com",
            "message": "Unauthorized"
        }"#;

        let dto: PlaceOrderErrorDto = serde_json::from_str(json).unwrap();

        match dto {
            PlaceOrderErrorDto::RemoteService {
                service_name,
                service_endpoint,
                message,
            } => {
                assert_eq!(service_name, "AuthService");
                assert_eq!(service_endpoint, "https://auth.example.com");
                assert_eq!(message, "Unauthorized");
            }
            _ => panic!("Expected RemoteService error"),
        }
    }

    #[rstest]
    fn test_clone_validation() {
        let validation_error = ValidationError::new("Field", "error");
        let error = PlaceOrderError::Validation(validation_error);
        let dto1 = PlaceOrderErrorDto::from_domain(&error);
        let dto2 = dto1.clone();

        assert_eq!(dto1, dto2);
    }

    #[rstest]
    fn test_clone_pricing() {
        let pricing_error = PricingError::new("error");
        let error = PlaceOrderError::Pricing(pricing_error);
        let dto1 = PlaceOrderErrorDto::from_domain(&error);
        let dto2 = dto1.clone();

        assert_eq!(dto1, dto2);
    }

    #[rstest]
    fn test_clone_remote_service() {
        let service = ServiceInfo::new("Service".to_string(), "https://example.com".to_string());
        let remote_error = RemoteServiceError::new(service, "error".to_string());
        let error = PlaceOrderError::RemoteService(remote_error);
        let dto1 = PlaceOrderErrorDto::from_domain(&error);
        let dto2 = dto1.clone();

        assert_eq!(dto1, dto2);
    }
}
