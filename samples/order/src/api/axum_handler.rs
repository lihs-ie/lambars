//! axum ハンドラ
//!
//! axum フレームワーク用のハンドラ関数を提供する。
//! IO モナドの `run_unsafe()` を「世界の端」で呼び出す。

use axum::{http::StatusCode, response::IntoResponse};

use crate::api::{HttpRequest, place_order_api};

/// POST /place-order ハンドラ
///
/// JSON リクエストを受け取り、`place_order_api` を呼び出し、
/// IO モナドを実行してレスポンスを返す。
///
/// # Arguments
///
/// * `body` - リクエストボディ（JSON 文字列）
///
/// # Returns
///
/// axum のレスポンス型
///
/// # Examples
///
/// ```ignore
/// use axum::{routing::post, Router};
/// use order_taking_sample::api::axum_handler::place_order_handler;
///
/// let app = Router::new().route("/place-order", post(place_order_handler));
/// ```
#[allow(clippy::unused_async)] // axum ハンドラは async が必須
pub async fn place_order_handler(body: String) -> impl IntoResponse {
    // HttpRequest を構築
    let request = HttpRequest::new(body);

    // IO モナドを取得
    let io_response = place_order_api(&request);

    // IO を実行（世界の端）
    let response = io_response.run_unsafe();

    // axum レスポンスに変換
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
        // 無効な order_id（空文字列）
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
