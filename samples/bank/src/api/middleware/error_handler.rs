//! Error handling middleware for the API layer.
//!
//! This module provides API error types and conversion functions for
//! transforming domain errors into HTTP responses.
//!
//! # Bifunctor-like Error Transformation
//!
//! Error transformation follows a bifunctor-like pattern where errors
//! are mapped from domain types to API types while preserving the
//! structure of the error handling.
//!
//! # Examples
//!
//! ```rust,ignore
//! use bank::api::middleware::error_handler::{ApiError, domain_error_to_api_error};
//! use bank::domain::account::errors::DomainError;
//!
//! let domain_error = DomainError::AccountNotFound(account_id);
//! let (status, api_error) = domain_error_to_api_error(domain_error);
//! // status == StatusCode::NOT_FOUND
//! ```

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::api::dto::transformers::TransformationError;
use crate::domain::account::errors::DomainError;
use crate::domain::value_objects::AccountIdValidationError;

/// API error response.
///
/// This structure is serialized to JSON for error responses.
///
/// # Example JSON
///
/// ```json
/// {
///     "code": "ACCOUNT_NOT_FOUND",
///     "message": "The specified account was not found",
///     "details": {
///         "account_id": "..."
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApiError {
    /// A machine-readable error code.
    pub code: String,
    /// A human-readable error message.
    pub message: String,
    /// Optional additional error details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    /// Creates a new `ApiError` without details.
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Creates a new `ApiError` with details.
    #[must_use]
    pub fn with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: Some(details),
        }
    }

    /// Creates a bad request error.
    #[must_use]
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new("BAD_REQUEST", message)
    }

    /// Creates a not found error.
    #[must_use]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new("NOT_FOUND", message)
    }

    /// Creates a conflict error.
    #[must_use]
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new("CONFLICT", message)
    }

    /// Creates an internal server error.
    #[must_use]
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new("INTERNAL_ERROR", message)
    }

    /// Creates an unprocessable entity error.
    #[must_use]
    pub fn unprocessable_entity(message: impl Into<String>) -> Self {
        Self::new("UNPROCESSABLE_ENTITY", message)
    }
}

/// Response wrapper that includes HTTP status code with `ApiError`.
#[derive(Debug, Clone)]
pub struct ApiErrorResponse {
    /// The HTTP status code.
    pub status: StatusCode,
    /// The error body.
    pub error: ApiError,
}

impl ApiErrorResponse {
    /// Creates a new `ApiErrorResponse`.
    #[must_use]
    pub const fn new(status: StatusCode, error: ApiError) -> Self {
        Self { status, error }
    }
}

impl IntoResponse for ApiErrorResponse {
    fn into_response(self) -> Response {
        (self.status, Json(self.error)).into_response()
    }
}

// =============================================================================
// Error Conversion Functions (Pure Functions)
// =============================================================================

/// Converts a domain error to an API error response.
///
/// This is a pure function that maps domain errors to appropriate
/// HTTP status codes and API error structures.
///
/// # Bifunctor Pattern
///
/// This function acts like the left-side transformation of a bifunctor,
/// mapping the error channel while preserving the overall structure.
///
/// # Arguments
///
/// * `error` - The domain error to convert
///
/// # Returns
///
/// A tuple of (`StatusCode`, `ApiError`)
///
/// # Error Mapping
///
/// | Domain Error | HTTP Status | Error Code |
/// |-------------|-------------|------------|
/// | AccountNotFound | 404 | ACCOUNT_NOT_FOUND |
/// | InsufficientBalance | 400 | INSUFFICIENT_BALANCE |
/// | AccountClosed | 409 | ACCOUNT_CLOSED |
/// | AccountFrozen | 409 | ACCOUNT_FROZEN |
/// | InvalidAmount | 400 | INVALID_AMOUNT |
/// | ConcurrencyConflict | 409 | CONCURRENCY_CONFLICT |
#[must_use]
pub fn domain_error_to_api_error(error: DomainError) -> (StatusCode, ApiError) {
    match error {
        DomainError::AccountNotFound(account_id) => (
            StatusCode::NOT_FOUND,
            ApiError::with_details(
                "ACCOUNT_NOT_FOUND",
                "The specified account was not found",
                serde_json::json!({
                    "account_id": account_id.to_string()
                }),
            ),
        ),
        DomainError::InsufficientBalance {
            required,
            available,
        } => (
            StatusCode::BAD_REQUEST,
            ApiError::with_details(
                "INSUFFICIENT_BALANCE",
                "Insufficient balance for the requested operation",
                serde_json::json!({
                    "required": required.amount().to_string(),
                    "available": available.amount().to_string(),
                    "currency": required.currency().to_string()
                }),
            ),
        ),
        DomainError::AccountClosed(account_id) => (
            StatusCode::CONFLICT,
            ApiError::with_details(
                "ACCOUNT_CLOSED",
                "The account is closed and cannot accept operations",
                serde_json::json!({
                    "account_id": account_id.to_string()
                }),
            ),
        ),
        DomainError::AccountFrozen(account_id) => (
            StatusCode::CONFLICT,
            ApiError::with_details(
                "ACCOUNT_FROZEN",
                "The account is frozen and operations are temporarily suspended",
                serde_json::json!({
                    "account_id": account_id.to_string()
                }),
            ),
        ),
        DomainError::InvalidAmount(reason) => (
            StatusCode::BAD_REQUEST,
            ApiError::with_details(
                "INVALID_AMOUNT",
                "The provided amount is invalid",
                serde_json::json!({
                    "reason": reason
                }),
            ),
        ),
        DomainError::ConcurrencyConflict { expected, actual } => (
            StatusCode::CONFLICT,
            ApiError::with_details(
                "CONCURRENCY_CONFLICT",
                "The resource was modified by another request",
                serde_json::json!({
                    "expected_version": expected,
                    "actual_version": actual
                }),
            ),
        ),
    }
}

/// Converts a transformation error to an API error response.
///
/// This is a pure function for DTO validation errors.
#[must_use]
pub fn transformation_error_to_api_error(error: TransformationError) -> (StatusCode, ApiError) {
    match error {
        TransformationError::InvalidAmount(value) => (
            StatusCode::BAD_REQUEST,
            ApiError::with_details(
                "INVALID_AMOUNT",
                "The provided amount is not a valid number",
                serde_json::json!({
                    "value": value
                }),
            ),
        ),
        TransformationError::InvalidCurrency(value) => (
            StatusCode::BAD_REQUEST,
            ApiError::with_details(
                "INVALID_CURRENCY",
                "The provided currency is not supported",
                serde_json::json!({
                    "value": value,
                    "supported": ["JPY", "USD", "EUR"]
                }),
            ),
        ),
    }
}

/// Converts an account ID validation error to an API error response.
///
/// This is a pure function for account ID validation errors.
#[must_use]
pub fn account_id_error_to_api_error(error: AccountIdValidationError) -> (StatusCode, ApiError) {
    match error {
        AccountIdValidationError::InvalidUuidFormat(value) => (
            StatusCode::BAD_REQUEST,
            ApiError::with_details(
                "INVALID_ACCOUNT_ID",
                "The provided account ID is not a valid UUID",
                serde_json::json!({
                    "value": value
                }),
            ),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::{AccountId, Currency, Money};
    use rstest::rstest;

    // =========================================================================
    // ApiError Construction Tests
    // =========================================================================

    #[rstest]
    fn api_error_new_creates_without_details() {
        let error = ApiError::new("TEST_CODE", "Test message");

        assert_eq!(error.code, "TEST_CODE");
        assert_eq!(error.message, "Test message");
        assert!(error.details.is_none());
    }

    #[rstest]
    fn api_error_with_details_creates_with_details() {
        let details = serde_json::json!({"key": "value"});
        let error = ApiError::with_details("TEST_CODE", "Test message", details.clone());

        assert_eq!(error.code, "TEST_CODE");
        assert_eq!(error.message, "Test message");
        assert_eq!(error.details, Some(details));
    }

    #[rstest]
    fn api_error_bad_request_creates_correctly() {
        let error = ApiError::bad_request("Bad request message");

        assert_eq!(error.code, "BAD_REQUEST");
        assert_eq!(error.message, "Bad request message");
    }

    #[rstest]
    fn api_error_not_found_creates_correctly() {
        let error = ApiError::not_found("Not found message");

        assert_eq!(error.code, "NOT_FOUND");
        assert_eq!(error.message, "Not found message");
    }

    #[rstest]
    fn api_error_conflict_creates_correctly() {
        let error = ApiError::conflict("Conflict message");

        assert_eq!(error.code, "CONFLICT");
        assert_eq!(error.message, "Conflict message");
    }

    #[rstest]
    fn api_error_internal_error_creates_correctly() {
        let error = ApiError::internal_error("Internal error message");

        assert_eq!(error.code, "INTERNAL_ERROR");
        assert_eq!(error.message, "Internal error message");
    }

    #[rstest]
    fn api_error_unprocessable_entity_creates_correctly() {
        let error = ApiError::unprocessable_entity("Unprocessable entity message");

        assert_eq!(error.code, "UNPROCESSABLE_ENTITY");
        assert_eq!(error.message, "Unprocessable entity message");
    }

    // =========================================================================
    // ApiError Serialization Tests
    // =========================================================================

    #[rstest]
    fn api_error_serializes_without_details() {
        let error = ApiError::new("TEST_CODE", "Test message");
        let json = serde_json::to_string(&error).unwrap();

        assert!(json.contains("\"code\":\"TEST_CODE\""));
        assert!(json.contains("\"message\":\"Test message\""));
        assert!(!json.contains("\"details\""));
    }

    #[rstest]
    fn api_error_serializes_with_details() {
        let error = ApiError::with_details(
            "TEST_CODE",
            "Test message",
            serde_json::json!({"key": "val"}),
        );
        let json = serde_json::to_string(&error).unwrap();

        assert!(json.contains("\"details\":"));
        assert!(json.contains("\"key\":\"val\""));
    }

    // =========================================================================
    // domain_error_to_api_error Tests
    // =========================================================================

    #[rstest]
    fn domain_error_to_api_error_account_not_found() {
        let account_id = AccountId::generate();
        let error = DomainError::AccountNotFound(account_id);

        let (status, api_error) = domain_error_to_api_error(error);

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(api_error.code, "ACCOUNT_NOT_FOUND");
        assert!(api_error.details.is_some());
    }

    #[rstest]
    fn domain_error_to_api_error_insufficient_balance() {
        let error = DomainError::InsufficientBalance {
            required: Money::new(10000, Currency::JPY),
            available: Money::new(5000, Currency::JPY),
        };

        let (status, api_error) = domain_error_to_api_error(error);

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(api_error.code, "INSUFFICIENT_BALANCE");
        assert!(api_error.details.is_some());
        let details = api_error.details.unwrap();
        assert_eq!(details["required"], "10000");
        assert_eq!(details["available"], "5000");
    }

    #[rstest]
    fn domain_error_to_api_error_account_closed() {
        let account_id = AccountId::generate();
        let error = DomainError::AccountClosed(account_id);

        let (status, api_error) = domain_error_to_api_error(error);

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(api_error.code, "ACCOUNT_CLOSED");
    }

    #[rstest]
    fn domain_error_to_api_error_account_frozen() {
        let account_id = AccountId::generate();
        let error = DomainError::AccountFrozen(account_id);

        let (status, api_error) = domain_error_to_api_error(error);

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(api_error.code, "ACCOUNT_FROZEN");
    }

    #[rstest]
    fn domain_error_to_api_error_invalid_amount() {
        let error = DomainError::InvalidAmount("negative value".to_string());

        let (status, api_error) = domain_error_to_api_error(error);

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(api_error.code, "INVALID_AMOUNT");
    }

    #[rstest]
    fn domain_error_to_api_error_concurrency_conflict() {
        let error = DomainError::ConcurrencyConflict {
            expected: 5,
            actual: 7,
        };

        let (status, api_error) = domain_error_to_api_error(error);

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(api_error.code, "CONCURRENCY_CONFLICT");
        let details = api_error.details.unwrap();
        assert_eq!(details["expected_version"], 5);
        assert_eq!(details["actual_version"], 7);
    }

    #[rstest]
    fn domain_error_to_api_error_is_pure() {
        let error1 = DomainError::InvalidAmount("test".to_string());
        let error2 = DomainError::InvalidAmount("test".to_string());

        let (status1, api_error1) = domain_error_to_api_error(error1);
        let (status2, api_error2) = domain_error_to_api_error(error2);

        assert_eq!(status1, status2);
        assert_eq!(api_error1, api_error2);
    }

    // =========================================================================
    // transformation_error_to_api_error Tests
    // =========================================================================

    #[rstest]
    fn transformation_error_to_api_error_invalid_amount() {
        let error = TransformationError::InvalidAmount("bad".to_string());

        let (status, api_error) = transformation_error_to_api_error(error);

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(api_error.code, "INVALID_AMOUNT");
    }

    #[rstest]
    fn transformation_error_to_api_error_invalid_currency() {
        let error = TransformationError::InvalidCurrency("XYZ".to_string());

        let (status, api_error) = transformation_error_to_api_error(error);

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(api_error.code, "INVALID_CURRENCY");
        let details = api_error.details.unwrap();
        assert!(
            details["supported"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("JPY"))
        );
    }

    // =========================================================================
    // account_id_error_to_api_error Tests
    // =========================================================================

    #[rstest]
    fn account_id_error_to_api_error_invalid_format() {
        let error = AccountIdValidationError::InvalidUuidFormat("bad-uuid".to_string());

        let (status, api_error) = account_id_error_to_api_error(error);

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(api_error.code, "INVALID_ACCOUNT_ID");
        let details = api_error.details.unwrap();
        assert_eq!(details["value"], "bad-uuid");
    }

    // =========================================================================
    // ApiErrorResponse Tests
    // =========================================================================

    #[rstest]
    fn api_error_response_new_creates_correctly() {
        let error = ApiError::not_found("Test");
        let response = ApiErrorResponse::new(StatusCode::NOT_FOUND, error.clone());

        assert_eq!(response.status, StatusCode::NOT_FOUND);
        assert_eq!(response.error, error);
    }
}
