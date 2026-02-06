//! `PlaceOrder` API
//!
//! HTTP API endpoint for the `PlaceOrder` workflow.
//! Implemented in subsequent steps.

use lambars::effect::IO;

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

/// HTTP API endpoint for the `PlaceOrder` workflow
///
/// Receives an HTTP request, executes the `PlaceOrder` workflow,
/// and returns an HTTP response.
///
/// # Processing Flow
///
/// 1. Deserialize the request body into `OrderFormDto`
/// 2. Convert `OrderFormDto` to `UnvalidatedOrder`
/// 3. Execute the `place_order` workflow
/// 4. On success: serialize events to `PlaceOrderEventDto` and return 200
/// 5. On failure: serialize errors to `PlaceOrderErrorDto` and return 400/500
///
/// # Arguments
///
/// * `request` - HTTP request
///
/// # Returns
///
/// `IO<HttpResponse>` - Response containing side effects
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
    // Step 1: Deserialize the request body
    let order_form_dto: OrderFormDto = match serde_json::from_str(request.body()) {
        Ok(dto) => dto,
        Err(error) => {
            return IO::pure(create_json_parse_error_response(&error));
        }
    };

    // Step 2: Convert DTO to domain type
    let unvalidated_order = order_form_dto.to_unvalidated_order();

    // Step 3: Execute the workflow
    let workflow_io = place_order(
        &check_product_exists,
        &check_address_exists,
        &get_pricing_function,
        &calculate_shipping_cost,
        &create_acknowledgment_letter,
        &send_acknowledgment,
        &unvalidated_order,
    );

    // Step 4-5: Convert the result to an HTTP response
    workflow_io.fmap(|result| match result {
        Ok(events) => create_success_response(&events),
        Err(error) => create_error_response(&error),
    })
}

/// Creates a success response
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

/// Creates an error response
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

/// Creates a JSON parse error response
fn create_json_parse_error_response(error: &serde_json::Error) -> HttpResponse {
    let error_message = format!(
        r#"{{"type":"JsonParseError","message":"{}"}}"#,
        escape_json_string(&error.to_string())
    );
    HttpResponse::bad_request(error_message)
}

/// Determines the status code based on the error type
const fn determine_error_status_code(error: &PlaceOrderError) -> u16 {
    match error {
        PlaceOrderError::Validation(_) | PlaceOrderError::Pricing(_) => 400,
        PlaceOrderError::RemoteService(_) => 500,
    }
}

/// Escapes a string for JSON embedding
fn escape_json_string(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
