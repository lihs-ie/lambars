//! `PlaceOrder` API
//!
//! `PlaceOrder` ワークフローの HTTP API エンドポイント。
//! 後続のステップで実装する。

use functional_rusty::effect::IO;

use crate::api::HttpRequest;
use crate::api::types::HttpResponse;
use crate::dto::{OrderFormDto, PlaceOrderErrorDto, PlaceOrderEventDto};
use crate::workflow::{PlaceOrderError, place_order};

use super::dependencies::{
    calculate_shipping_cost, check_address_exists, check_product_exists,
    create_acknowledgment_letter, get_pricing_function, send_acknowledgment,
};

// =============================================================================
// place_order_api (REQ-089)
// =============================================================================

/// `PlaceOrder` ワークフローの HTTP API エンドポイント
///
/// HTTP リクエストを受け取り、`PlaceOrder` ワークフローを実行し、
/// HTTP レスポンスを返す。
///
/// # 処理フロー
///
/// 1. リクエストボディを `OrderFormDto` にデシリアライズ
/// 2. `OrderFormDto` を `UnvalidatedOrder` に変換
/// 3. `place_order` ワークフローを実行
/// 4. 成功時: イベントを `PlaceOrderEventDto` にシリアライズして 200 を返す
/// 5. 失敗時: エラーを `PlaceOrderErrorDto` にシリアライズして 400/500 を返す
///
/// # Arguments
///
/// * `request` - HTTP リクエスト
///
/// # Returns
///
/// `IO<HttpResponse>` - 副作用を含むレスポンス
///
/// # Examples
///
/// ```ignore
/// use order_taking_sample::api::{HttpRequest, place_order_api};
///
/// let json = r#"{"order_id": "order-001", ...}"#;
/// let request = HttpRequest::new(json.to_string());
/// let io_response = place_order_api(request);
/// let response = io_response.run_unsafe();
/// ```
#[must_use]
pub fn place_order_api(request: &HttpRequest) -> IO<HttpResponse> {
    // Step 1: リクエストボディをデシリアライズ
    let order_form_dto: OrderFormDto = match serde_json::from_str(request.body()) {
        Ok(dto) => dto,
        Err(error) => {
            return IO::pure(create_json_parse_error_response(&error));
        }
    };

    // Step 2: DTO をドメイン型に変換
    let unvalidated_order = order_form_dto.to_unvalidated_order();

    // Step 3: ワークフローを実行
    let workflow_io = place_order(
        &check_product_exists,
        &check_address_exists,
        &get_pricing_function,
        &calculate_shipping_cost,
        &create_acknowledgment_letter,
        &send_acknowledgment,
        &unvalidated_order,
    );

    // Step 4-5: 結果を HTTP レスポンスに変換
    workflow_io.fmap(|result| match result {
        Ok(events) => create_success_response(&events),
        Err(error) => create_error_response(&error),
    })
}

/// 成功レスポンスを生成する
fn create_success_response(events: &[crate::workflow::PlaceOrderEvent]) -> HttpResponse {
    let event_dtos = PlaceOrderEventDto::from_domain_list(events);
    serde_json::to_string(&event_dtos).map_or_else(
        |_| {
            HttpResponse::internal_server_error(
                r#"{"type":"SerializationError","message":"Failed to serialize response"}"#
                    .to_string(),
            )
        },
        HttpResponse::ok,
    )
}

/// エラーレスポンスを生成する
fn create_error_response(error: &PlaceOrderError) -> HttpResponse {
    let error_dto = PlaceOrderErrorDto::from_domain(error);
    let status_code = determine_error_status_code(error);
    serde_json::to_string(&error_dto).map_or_else(
        |_| {
            HttpResponse::internal_server_error(
                r#"{"type":"SerializationError","message":"Failed to serialize error"}"#
                    .to_string(),
            )
        },
        |json| HttpResponse::new(status_code, json),
    )
}

/// JSON パースエラーレスポンスを生成する
fn create_json_parse_error_response(error: &serde_json::Error) -> HttpResponse {
    let error_message = format!(
        r#"{{"type":"JsonParseError","message":"{}"}}"#,
        escape_json_string(&error.to_string())
    );
    HttpResponse::bad_request(error_message)
}

/// エラーの種類に応じたステータスコードを決定する
const fn determine_error_status_code(error: &PlaceOrderError) -> u16 {
    match error {
        PlaceOrderError::Validation(_) | PlaceOrderError::Pricing(_) => 400,
        PlaceOrderError::RemoteService(_) => 500,
    }
}

/// JSON 文字列用のエスケープ処理
fn escape_json_string(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
