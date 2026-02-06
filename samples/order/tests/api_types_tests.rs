//! Tests for API types
//!
//! Tests for HttpRequest and HttpResponse

use order_taking_sample::api::{HttpRequest, HttpResponse};
use rstest::rstest;

// =============================================================================
// Tests for HttpRequest
// =============================================================================

mod http_request_tests {
    use super::*;

    #[rstest]
    fn test_new() {
        let body = r#"{"order_id": "order-001"}"#.to_string();
        let request = HttpRequest::new(body.clone());

        assert_eq!(request.body(), &body);
    }

    #[rstest]
    fn test_new_empty_body() {
        let request = HttpRequest::new(String::new());

        assert_eq!(request.body(), "");
    }

    #[rstest]
    fn test_body() {
        let json = r#"{"key": "value"}"#.to_string();
        let request = HttpRequest::new(json.clone());

        assert_eq!(request.body(), &json);
    }

    #[rstest]
    fn test_clone() {
        let request1 = HttpRequest::new("test body".to_string());
        let request2 = request1.clone();

        assert_eq!(request1, request2);
    }
}

// =============================================================================
// Tests for HttpResponse
// =============================================================================

mod http_response_tests {
    use super::*;

    #[rstest]
    fn test_new() {
        let body = r#"{"success": true}"#.to_string();
        let response = HttpResponse::new(200, body.clone());

        assert_eq!(response.status_code(), 200);
        assert_eq!(response.body(), &body);
    }

    #[rstest]
    fn test_new_error_response() {
        let body = r#"{"error": "Not found"}"#.to_string();
        let response = HttpResponse::new(404, body.clone());

        assert_eq!(response.status_code(), 404);
        assert_eq!(response.body(), &body);
    }

    #[rstest]
    fn test_ok() {
        let body = r#"{"result": "success"}"#.to_string();
        let response = HttpResponse::ok(body.clone());

        assert_eq!(response.status_code(), 200);
        assert_eq!(response.body(), &body);
    }

    #[rstest]
    fn test_bad_request() {
        let body = r#"{"error": "Invalid input"}"#.to_string();
        let response = HttpResponse::bad_request(body.clone());

        assert_eq!(response.status_code(), 400);
        assert_eq!(response.body(), &body);
    }

    #[rstest]
    fn test_internal_server_error() {
        let body = r#"{"error": "Something went wrong"}"#.to_string();
        let response = HttpResponse::internal_server_error(body.clone());

        assert_eq!(response.status_code(), 500);
        assert_eq!(response.body(), &body);
    }

    #[rstest]
    fn test_status_code() {
        let response = HttpResponse::new(201, "Created".to_string());

        assert_eq!(response.status_code(), 201);
    }

    #[rstest]
    fn test_body() {
        let body = "Response body".to_string();
        let response = HttpResponse::new(200, body.clone());

        assert_eq!(response.body(), &body);
    }

    #[rstest]
    fn test_is_success_200() {
        let response = HttpResponse::ok("success".to_string());

        assert!(response.is_success());
    }

    #[rstest]
    fn test_is_success_201() {
        let response = HttpResponse::new(201, "created".to_string());

        assert!(response.is_success());
    }

    #[rstest]
    fn test_is_success_299() {
        let response = HttpResponse::new(299, "still success".to_string());

        assert!(response.is_success());
    }

    #[rstest]
    fn test_is_success_false_for_400() {
        let response = HttpResponse::bad_request("error".to_string());

        assert!(!response.is_success());
    }

    #[rstest]
    fn test_is_success_false_for_500() {
        let response = HttpResponse::internal_server_error("error".to_string());

        assert!(!response.is_success());
    }

    #[rstest]
    fn test_clone() {
        let response1 = HttpResponse::ok("body".to_string());
        let response2 = response1.clone();

        assert_eq!(response1, response2);
    }
}
