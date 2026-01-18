//! API error handling.
//!
//! This module provides error types and response formatting for the API.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::infrastructure::RepositoryError;

// =============================================================================
// API Error
// =============================================================================

/// API error structure for JSON responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    /// Error code for programmatic handling.
    pub code: String,
    /// Human-readable error message.
    pub message: String,
    /// Optional field-level errors for validation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<FieldError>>,
}

impl ApiError {
    /// Creates a new API error.
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Creates a validation error with field-level details.
    #[must_use]
    pub fn validation(message: impl Into<String>, details: Vec<FieldError>) -> Self {
        Self {
            code: "VALIDATION_ERROR".to_string(),
            message: message.into(),
            details: Some(details),
        }
    }
}

/// Field-level error for validation failures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldError {
    /// Name of the field that failed validation.
    pub field: String,
    /// Error message for this field.
    pub message: String,
}

impl FieldError {
    /// Creates a new field error.
    #[must_use]
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

// =============================================================================
// API Error Response
// =============================================================================

/// API error response containing status code and error details.
#[derive(Debug, Clone)]
pub struct ApiErrorResponse {
    /// HTTP status code.
    pub status: StatusCode,
    /// Error details.
    pub error: ApiError,
}

impl ApiErrorResponse {
    /// Creates a new API error response.
    #[must_use]
    pub const fn new(status: StatusCode, error: ApiError) -> Self {
        Self { status, error }
    }

    /// Creates a 400 Bad Request response.
    #[must_use]
    pub fn bad_request(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, ApiError::new(code, message))
    }

    /// Creates a 400 Bad Request response for validation errors.
    #[must_use]
    pub fn validation_error(message: impl Into<String>, details: Vec<FieldError>) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            ApiError::validation(message, details),
        )
    }

    /// Creates a 404 Not Found response.
    #[must_use]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, ApiError::new("NOT_FOUND", message))
    }

    /// Creates a 409 Conflict response for version conflicts.
    #[must_use]
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::CONFLICT,
            ApiError::new("VERSION_CONFLICT", message),
        )
    }

    /// Creates a 500 Internal Server Error response.
    #[must_use]
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::new("INTERNAL_ERROR", message),
        )
    }
}

impl IntoResponse for ApiErrorResponse {
    fn into_response(self) -> Response {
        (self.status, Json(self.error)).into_response()
    }
}

impl From<RepositoryError> for ApiErrorResponse {
    fn from(error: RepositoryError) -> Self {
        match error {
            RepositoryError::NotFound(msg) => Self::not_found(msg),
            RepositoryError::VersionConflict { expected, found } => {
                Self::conflict(format!("Expected version {expected}, found {found}"))
            }
            // Internal errors should not expose details to clients.
            // In production, log the error details: tracing::error!(%error, "Internal error");
            RepositoryError::DatabaseError(_)
            | RepositoryError::SerializationError(_)
            | RepositoryError::CacheError(_) => Self::internal_error("An internal error occurred"),
        }
    }
}

// =============================================================================
// Validation Error
// =============================================================================

/// Validation error type for domain validation.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Field-level errors.
    pub errors: Vec<FieldError>,
}

impl ValidationError {
    /// Creates a new validation error.
    #[must_use]
    pub const fn new(errors: Vec<FieldError>) -> Self {
        Self { errors }
    }

    /// Creates a validation error with a single field error.
    #[must_use]
    pub fn single(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(vec![FieldError::new(field, message)])
    }

    /// Returns true if there are no validation errors.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }
}

impl From<ValidationError> for ApiErrorResponse {
    fn from(error: ValidationError) -> Self {
        Self::validation_error("Validation failed", error.errors)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn test_api_error_new() {
        let error = ApiError::new("TEST_ERROR", "Test message");
        assert_eq!(error.code, "TEST_ERROR");
        assert_eq!(error.message, "Test message");
        assert!(error.details.is_none());
    }

    #[rstest]
    fn test_api_error_validation() {
        let details = vec![FieldError::new("title", "Title is required")];
        let error = ApiError::validation("Validation failed", details);
        assert_eq!(error.code, "VALIDATION_ERROR");
        assert!(error.details.is_some());
        assert_eq!(error.details.unwrap().len(), 1);
    }

    #[rstest]
    fn test_field_error_new() {
        let error = FieldError::new("title", "Title is required");
        assert_eq!(error.field, "title");
        assert_eq!(error.message, "Title is required");
    }

    #[rstest]
    fn test_api_error_response_bad_request() {
        let response = ApiErrorResponse::bad_request("BAD_INPUT", "Invalid input");
        assert_eq!(response.status, StatusCode::BAD_REQUEST);
        assert_eq!(response.error.code, "BAD_INPUT");
    }

    #[rstest]
    fn test_api_error_response_not_found() {
        let response = ApiErrorResponse::not_found("Task not found");
        assert_eq!(response.status, StatusCode::NOT_FOUND);
        assert_eq!(response.error.code, "NOT_FOUND");
    }

    #[rstest]
    fn test_api_error_response_conflict() {
        let response = ApiErrorResponse::conflict("Version mismatch");
        assert_eq!(response.status, StatusCode::CONFLICT);
        assert_eq!(response.error.code, "VERSION_CONFLICT");
    }

    #[rstest]
    fn test_api_error_response_internal_error() {
        let response = ApiErrorResponse::internal_error("Database error");
        assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response.error.code, "INTERNAL_ERROR");
    }

    #[rstest]
    fn test_repository_error_to_api_error_response() {
        let error = RepositoryError::NotFound("task-123".to_string());
        let response: ApiErrorResponse = error.into();
        assert_eq!(response.status, StatusCode::NOT_FOUND);

        let error = RepositoryError::VersionConflict {
            expected: 1,
            found: 2,
        };
        let response: ApiErrorResponse = error.into();
        assert_eq!(response.status, StatusCode::CONFLICT);

        let error = RepositoryError::DatabaseError("connection failed".to_string());
        let response: ApiErrorResponse = error.into();
        assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[rstest]
    fn test_validation_error_to_api_error_response() {
        let error = ValidationError::single("title", "Title is required");
        let response: ApiErrorResponse = error.into();
        assert_eq!(response.status, StatusCode::BAD_REQUEST);
        assert_eq!(response.error.code, "VALIDATION_ERROR");
    }
}
