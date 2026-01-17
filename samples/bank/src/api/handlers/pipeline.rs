//! Pipeline utilities for handler composition.
//!
//! This module provides utilities for composing handler operations using
//! lambars pipe! and compose! macros. It enables a more declarative,
//! functional style of handler implementation.
//!
//! # Design Principles
//!
//! - **Pure Functions**: All pipeline steps are pure functions
//! - **Composability**: Steps can be combined using pipe!/compose!
//! - **Type Safety**: Error handling is explicit through Result types
//!
//! # Examples
//!
//! ```rust,ignore
//! use lambars::pipe;
//! use bank::api::handlers::pipeline::*;
//!
//! let result = pipe!(
//!     "550e8400-e29b-41d4-a716-446655440000",
//!     parse_account_id,
//!     => |id| id.to_string()
//! );
//! ```

use axum::http::StatusCode;
use lambars::control::Either;

use crate::api::dto::requests::MoneyDto;
use crate::api::dto::transformers::{TransformationError, dto_to_money};
use crate::api::middleware::error_handler::{
    ApiError, ApiErrorResponse, account_id_error_to_api_error, transformation_error_to_api_error,
};
use crate::domain::value_objects::{AccountId, AccountIdValidationError, Money};

// =============================================================================
// Either to Result Conversion
// =============================================================================

/// Converts an `Either<L, R>` to `Result<R, L>`.
///
/// This is a pure function that transforms the Either type to Rust's Result type,
/// enabling compatibility with pipe! macro's monadic operators.
///
/// # Type Parameters
///
/// * `L` - The left (error) type
/// * `R` - The right (success) type
///
/// # Errors
///
/// Returns `Err(L)` if the Either is `Left(L)`.
///
/// # Examples
///
/// ```rust,ignore
/// use lambars::control::Either;
///
/// let either: Either<&str, i32> = Either::Right(42);
/// let result: Result<i32, &str> = either_to_result(either);
/// assert_eq!(result, Ok(42));
/// ```
#[inline]
pub fn either_to_result<L, R>(either: Either<L, R>) -> Result<R, L> {
    match either {
        Either::Right(value) => Ok(value),
        Either::Left(error) => Err(error),
    }
}

/// Converts a `Result<R, L>` to `Either<L, R>`.
///
/// This is the inverse of `either_to_result`.
///
/// # Examples
///
/// ```rust,ignore
/// let result: Result<i32, &str> = Ok(42);
/// let either: Either<&str, i32> = result_to_either(result);
/// assert!(either.is_right());
/// ```
#[inline]
pub fn result_to_either<L, R>(result: Result<R, L>) -> Either<L, R> {
    match result {
        Ok(value) => Either::Right(value),
        Err(error) => Either::Left(error),
    }
}

// =============================================================================
// Account ID Parsing Pipeline Steps
// =============================================================================

/// Parses an account ID string into an `AccountId`.
///
/// Returns `Result<AccountId, AccountIdValidationError>` for use with pipe! macro.
///
/// # Errors
///
/// Returns `AccountIdValidationError::InvalidUuidFormat` if the string is not a valid UUID.
///
/// # Examples
///
/// ```rust,ignore
/// let result = parse_account_id("550e8400-e29b-41d4-a716-446655440000");
/// assert!(result.is_ok());
/// ```
#[inline]
pub fn parse_account_id(id_string: &str) -> Result<AccountId, AccountIdValidationError> {
    either_to_result(AccountId::create(id_string))
}

/// Converts an account ID validation error to an `ApiErrorResponse`.
///
/// This is a pure function for error transformation in pipelines.
#[inline]
pub fn account_id_error_to_response(error: AccountIdValidationError) -> ApiErrorResponse {
    let (status, api_error) = account_id_error_to_api_error(error);
    ApiErrorResponse::new(status, api_error)
}

/// Parses an account ID string and returns an API-compatible Result.
///
/// This combines parsing and error transformation into a single pipeline step.
///
/// # Errors
///
/// Returns `ApiErrorResponse` with status 400 if the string is not a valid UUID.
///
/// # Examples
///
/// ```rust,ignore
/// use lambars::pipe;
///
/// let result = parse_account_id_for_api("invalid-uuid");
/// assert!(result.is_err());
/// ```
#[inline]
pub fn parse_account_id_for_api(id_string: &str) -> Result<AccountId, ApiErrorResponse> {
    parse_account_id(id_string).map_err(account_id_error_to_response)
}

// =============================================================================
// Money Parsing Pipeline Steps
// =============================================================================

/// Parses a `MoneyDto` into a `Money` domain object.
///
/// Returns `Result<Money, TransformationError>` for use with pipe! macro.
///
/// # Errors
///
/// Returns `TransformationError` if the amount or currency is invalid.
#[inline]
pub fn parse_money(dto: &MoneyDto) -> Result<Money, TransformationError> {
    either_to_result(dto_to_money(dto))
}

/// Converts a transformation error to an `ApiErrorResponse`.
///
/// This is a pure function for error transformation in pipelines.
#[inline]
pub fn transformation_error_to_response(error: TransformationError) -> ApiErrorResponse {
    let (status, api_error) = transformation_error_to_api_error(error);
    ApiErrorResponse::new(status, api_error)
}

/// Parses a `MoneyDto` and returns an API-compatible Result.
///
/// This combines parsing and error transformation into a single pipeline step.
///
/// # Errors
///
/// Returns `ApiErrorResponse` with status 400 if the amount or currency is invalid.
#[inline]
pub fn parse_money_for_api(dto: &MoneyDto) -> Result<Money, ApiErrorResponse> {
    parse_money(dto).map_err(transformation_error_to_response)
}

// =============================================================================
// Common API Error Responses
// =============================================================================

/// Creates an account not found error response.
#[inline]
pub fn account_not_found_response(account_id: &str) -> ApiErrorResponse {
    ApiErrorResponse::new(
        StatusCode::NOT_FOUND,
        ApiError::with_details(
            "ACCOUNT_NOT_FOUND",
            "The specified account was not found",
            serde_json::json!({ "account_id": account_id }),
        ),
    )
}

/// Creates an event store error response.
#[inline]
pub fn event_store_error_response(error: &dyn std::error::Error) -> ApiErrorResponse {
    ApiErrorResponse::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        ApiError::with_details(
            "EVENT_STORE_ERROR",
            "Failed to load account events",
            serde_json::json!({ "error": error.to_string() }),
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::Currency;
    use rstest::rstest;

    // =========================================================================
    // either_to_result Tests
    // =========================================================================

    #[rstest]
    fn either_to_result_right_returns_ok() {
        let either: Either<&str, i32> = Either::Right(42);
        let result = either_to_result(either);
        assert_eq!(result, Ok(42));
    }

    #[rstest]
    fn either_to_result_left_returns_err() {
        let either: Either<&str, i32> = Either::Left("error");
        let result = either_to_result(either);
        assert_eq!(result, Err("error"));
    }

    #[rstest]
    fn either_to_result_is_pure() {
        let either: Either<&str, i32> = Either::Right(42);
        let result1 = either_to_result(either);
        let either: Either<&str, i32> = Either::Right(42);
        let result2 = either_to_result(either);
        assert_eq!(result1, result2);
    }

    // =========================================================================
    // result_to_either Tests
    // =========================================================================

    #[rstest]
    fn result_to_either_ok_returns_right() {
        let result: Result<i32, &str> = Ok(42);
        let either = result_to_either(result);
        assert!(either.is_right());
        assert_eq!(either.unwrap_right(), 42);
    }

    #[rstest]
    fn result_to_either_err_returns_left() {
        let result: Result<i32, &str> = Err("error");
        let either = result_to_either(result);
        assert!(either.is_left());
        assert_eq!(either.unwrap_left(), "error");
    }

    #[rstest]
    fn either_result_roundtrip_preserves_value() {
        let original: Either<&str, i32> = Either::Right(42);
        let converted = result_to_either(either_to_result(original));
        assert!(converted.is_right());
        assert_eq!(converted.unwrap_right(), 42);
    }

    // =========================================================================
    // parse_account_id Tests
    // =========================================================================

    #[rstest]
    fn parse_account_id_valid_returns_ok() {
        let result = parse_account_id("550e8400-e29b-41d4-a716-446655440000");
        assert!(result.is_ok());
    }

    #[rstest]
    fn parse_account_id_invalid_returns_err() {
        let result = parse_account_id("not-a-uuid");
        assert!(result.is_err());
    }

    #[rstest]
    fn parse_account_id_for_api_valid_returns_ok() {
        let result = parse_account_id_for_api("550e8400-e29b-41d4-a716-446655440000");
        assert!(result.is_ok());
    }

    #[rstest]
    fn parse_account_id_for_api_invalid_returns_api_error() {
        let result = parse_account_id_for_api("not-a-uuid");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
    }

    // =========================================================================
    // parse_money Tests
    // =========================================================================

    #[rstest]
    fn parse_money_valid_returns_ok() {
        let dto = MoneyDto {
            amount: "10000".to_string(),
            currency: "JPY".to_string(),
        };
        let result = parse_money(&dto);
        assert!(result.is_ok());
        let money = result.unwrap();
        assert_eq!(money.amount().to_string(), "10000");
        assert_eq!(money.currency(), Currency::JPY);
    }

    #[rstest]
    fn parse_money_invalid_amount_returns_err() {
        let dto = MoneyDto {
            amount: "not-a-number".to_string(),
            currency: "JPY".to_string(),
        };
        let result = parse_money(&dto);
        assert!(result.is_err());
    }

    #[rstest]
    fn parse_money_invalid_currency_returns_err() {
        let dto = MoneyDto {
            amount: "10000".to_string(),
            currency: "INVALID".to_string(),
        };
        let result = parse_money(&dto);
        assert!(result.is_err());
    }

    #[rstest]
    fn parse_money_for_api_valid_returns_ok() {
        let dto = MoneyDto {
            amount: "10000".to_string(),
            currency: "JPY".to_string(),
        };
        let result = parse_money_for_api(&dto);
        assert!(result.is_ok());
    }

    #[rstest]
    fn parse_money_for_api_invalid_returns_api_error() {
        let dto = MoneyDto {
            amount: "not-a-number".to_string(),
            currency: "JPY".to_string(),
        };
        let result = parse_money_for_api(&dto);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
    }

    // =========================================================================
    // Pipe! Integration Tests
    // =========================================================================

    #[rstest]
    fn pipe_with_account_id_parsing() {
        use lambars::pipe;

        // Valid UUID through pipeline - use lift operator on Result
        let result = pipe!(
            parse_account_id_for_api("550e8400-e29b-41d4-a716-446655440000"),
            => |id| id.to_string()
        );
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[rstest]
    fn pipe_with_invalid_account_id() {
        use lambars::pipe;

        // Invalid UUID propagates error through lift operator
        let result: Result<String, ApiErrorResponse> = pipe!(
            parse_account_id_for_api("invalid"),
            => |id| id.to_string()
        );
        assert!(result.is_err());
    }

    #[rstest]
    fn pipe_chain_multiple_validations() {
        use lambars::pipe;

        let account_id_str = "550e8400-e29b-41d4-a716-446655440000";
        let money_dto = MoneyDto {
            amount: "10000".to_string(),
            currency: "JPY".to_string(),
        };

        // Chain validations using flat_map (bind) - use move to capture money_dto
        let result = pipe!(
            parse_account_id_for_api(account_id_str),
            =>> move |id| parse_money_for_api(&money_dto).map(|money| (id, money))
        );

        assert!(result.is_ok());
        let (id, money) = result.unwrap();
        assert_eq!(id.to_string(), account_id_str);
        assert_eq!(money.amount().to_string(), "10000");
    }

    #[rstest]
    fn pipe_chain_fails_on_first_error() {
        use lambars::pipe;

        let invalid_account_id = "invalid";
        let money_dto = MoneyDto {
            amount: "10000".to_string(),
            currency: "JPY".to_string(),
        };

        // First validation fails, second is not executed
        let result = pipe!(
            parse_account_id_for_api(invalid_account_id),
            =>> move |id| parse_money_for_api(&money_dto).map(|money| (id, money))
        );

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
    }

    // =========================================================================
    // Error Response Tests
    // =========================================================================

    #[rstest]
    fn account_not_found_response_has_correct_status() {
        let response = account_not_found_response("test-id");
        assert_eq!(response.status, StatusCode::NOT_FOUND);
    }

    #[rstest]
    fn event_store_error_response_has_correct_status() {
        let error = std::io::Error::other("test error");
        let response = event_store_error_response(&error);
        assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    // =========================================================================
    // Pure Function Tests (Referential Transparency)
    // =========================================================================

    #[rstest]
    fn pipeline_functions_are_referentially_transparent() {
        let id_str = "550e8400-e29b-41d4-a716-446655440000";
        let dto = MoneyDto {
            amount: "10000".to_string(),
            currency: "JPY".to_string(),
        };

        // Same input always produces same output
        assert_eq!(
            parse_account_id(id_str).map(|id| id.to_string()),
            parse_account_id(id_str).map(|id| id.to_string())
        );
        assert_eq!(
            parse_money(&dto).map(|m| m.amount().to_string()),
            parse_money(&dto).map(|m| m.amount().to_string())
        );
    }
}
