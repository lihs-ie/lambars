//! Axum handler
//!
//! Provides handler functions for the axum framework.
//! Converts IO monads to `AsyncIO` and calls `run_async()` at the "edge of the world".

use axum::{http::StatusCode, response::IntoResponse};

use crate::api::{HttpRequest, place_order_api};

/// POST /place-order handler
///
/// Receives a JSON request, calls `place_order_api`,
/// converts the IO monad to `AsyncIO` for async execution, and returns the response.
///
/// # Arguments
///
/// * `body` - Request body (JSON string)
///
/// # Returns
///
/// An axum response type
///
/// # Examples
///
/// ```ignore
/// use axum::{routing::post, Router};
/// use order_taking_sample::api::axum_handler::place_order_handler;
///
/// let app = Router::new().route("/place-order", post(place_order_handler));
/// ```
pub async fn place_order_handler(body: String) -> impl IntoResponse {
    // Build HttpRequest
    let request = HttpRequest::new(body);

    // Get IO monad
    let io_response = place_order_api(&request);

    // Convert IO to AsyncIO and execute asynchronously (edge of the world)
    let response = io_response.to_async().run_async().await;

    // Convert to axum response
    (
        StatusCode::from_u16(response.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        response.body().to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_place_order_handler_with_valid_json() {
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
                    "product_code": "W1234",
                    "quantity": "10"
                }
            ],
            "promotion_code": ""
        }"#;

        let response = place_order_handler(json.to_string()).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_place_order_handler_with_invalid_json() {
        let invalid_json = "{ invalid json }";

        let response = place_order_handler(invalid_json.to_string()).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_place_order_handler_with_validation_error() {
        // Invalid order_id (empty string)
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
}
