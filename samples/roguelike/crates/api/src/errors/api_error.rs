//! API error type with HTTP response conversion.
//!
//! This module provides the main error type for API handlers with
//! automatic conversion to HTTP responses via Axum's `IntoResponse`.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

use crate::dto::response::ErrorResponse;

// =============================================================================
// ApiError
// =============================================================================

/// API error type for HTTP handlers.
///
/// This enum represents all possible errors that can occur in API handlers.
/// Each variant maps to a specific HTTP status code and error response format.
///
/// # Examples
///
/// ```ignore
/// use roguelike_api::errors::ApiError;
///
/// fn get_game(id: &str) -> Result<Game, ApiError> {
///     Err(ApiError::not_found("GameSession", id))
/// }
/// ```
#[derive(Debug, Error)]
pub enum ApiError {
    /// The requested resource was not found.
    #[error("{entity_type} with identifier '{identifier}' not found")]
    NotFound {
        /// The type of entity that was not found.
        entity_type: String,
        /// The identifier used to look up the entity.
        identifier: String,
    },

    /// Validation error for request data.
    #[error("Validation error: {message}")]
    ValidationError {
        /// The validation error message.
        message: String,
        /// The field that failed validation, if applicable.
        field: Option<String>,
    },

    /// The command or action is invalid.
    #[error("Invalid command: {reason}")]
    InvalidCommand {
        /// The reason the command is invalid.
        reason: String,
    },

    /// A conflict occurred (e.g., resource already exists, invalid state).
    #[error("Conflict: {reason}")]
    Conflict {
        /// The reason for the conflict.
        reason: String,
    },

    /// An internal server error occurred.
    #[error("Internal server error: {message}")]
    InternalError {
        /// The error message.
        message: String,
    },
}

// =============================================================================
// Factory Methods
// =============================================================================

impl ApiError {
    /// Creates a new `NotFound` error.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let error = ApiError::not_found("GameSession", "abc-123");
    /// ```
    #[must_use]
    pub fn not_found(entity_type: impl Into<String>, identifier: impl Into<String>) -> Self {
        Self::NotFound {
            entity_type: entity_type.into(),
            identifier: identifier.into(),
        }
    }

    /// Creates a new `ValidationError`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let error = ApiError::validation("Player name must not be empty");
    /// ```
    #[must_use]
    pub fn validation(message: impl Into<String>) -> Self {
        Self::ValidationError {
            message: message.into(),
            field: None,
        }
    }

    /// Creates a new `ValidationError` with a field name.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let error = ApiError::validation_field("player_name", "must not be empty");
    /// ```
    #[must_use]
    pub fn validation_field(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ValidationError {
            message: message.into(),
            field: Some(field.into()),
        }
    }

    /// Creates a new `InvalidCommand` error.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let error = ApiError::invalid_command("Cannot move through walls");
    /// ```
    #[must_use]
    pub fn invalid_command(reason: impl Into<String>) -> Self {
        Self::InvalidCommand {
            reason: reason.into(),
        }
    }

    /// Creates a new `Conflict` error.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let error = ApiError::conflict("Game session has already ended");
    /// ```
    #[must_use]
    pub fn conflict(reason: impl Into<String>) -> Self {
        Self::Conflict {
            reason: reason.into(),
        }
    }

    /// Creates a new `InternalError`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let error = ApiError::internal("Database connection failed");
    /// ```
    #[must_use]
    pub fn internal(message: impl Into<String>) -> Self {
        Self::InternalError {
            message: message.into(),
        }
    }
}

// =============================================================================
// Query Methods
// =============================================================================

impl ApiError {
    /// Returns the HTTP status code for this error.
    #[must_use]
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::ValidationError { .. } => StatusCode::BAD_REQUEST,
            Self::InvalidCommand { .. } => StatusCode::BAD_REQUEST,
            Self::Conflict { .. } => StatusCode::CONFLICT,
            Self::InternalError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Returns the error code string for this error.
    #[must_use]
    pub fn error_code(&self) -> String {
        match self {
            Self::NotFound { entity_type, .. } => {
                format!("{}_NOT_FOUND", entity_type.to_uppercase().replace(' ', "_"))
            }
            Self::ValidationError { .. } => "VALIDATION_ERROR".to_string(),
            Self::InvalidCommand { .. } => "INVALID_COMMAND".to_string(),
            Self::Conflict { .. } => "CONFLICT".to_string(),
            Self::InternalError { .. } => "INTERNAL_ERROR".to_string(),
        }
    }

    /// Returns true if this error is a client error (4xx).
    #[must_use]
    pub fn is_client_error(&self) -> bool {
        self.status_code().is_client_error()
    }

    /// Returns true if this error is a server error (5xx).
    #[must_use]
    pub fn is_server_error(&self) -> bool {
        self.status_code().is_server_error()
    }
}

// =============================================================================
// IntoResponse Implementation
// =============================================================================

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status_code = self.status_code();
        let error_code = self.error_code();
        let message = self.to_string();

        let error_response = ErrorResponse::new(error_code, message);

        (status_code, Json(error_response)).into_response()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    mod factory_methods {
        use super::*;

        #[rstest]
        fn not_found_creates_error() {
            let error = ApiError::not_found("GameSession", "abc-123");
            match error {
                ApiError::NotFound {
                    entity_type,
                    identifier,
                } => {
                    assert_eq!(entity_type, "GameSession");
                    assert_eq!(identifier, "abc-123");
                }
                _ => panic!("Expected NotFound variant"),
            }
        }

        #[rstest]
        fn validation_creates_error() {
            let error = ApiError::validation("Invalid input");
            match error {
                ApiError::ValidationError { message, field } => {
                    assert_eq!(message, "Invalid input");
                    assert!(field.is_none());
                }
                _ => panic!("Expected ValidationError variant"),
            }
        }

        #[rstest]
        fn validation_field_creates_error_with_field() {
            let error = ApiError::validation_field("player_name", "must not be empty");
            match error {
                ApiError::ValidationError { message, field } => {
                    assert_eq!(message, "must not be empty");
                    assert_eq!(field, Some("player_name".to_string()));
                }
                _ => panic!("Expected ValidationError variant"),
            }
        }

        #[rstest]
        fn invalid_command_creates_error() {
            let error = ApiError::invalid_command("Cannot move through walls");
            match error {
                ApiError::InvalidCommand { reason } => {
                    assert_eq!(reason, "Cannot move through walls");
                }
                _ => panic!("Expected InvalidCommand variant"),
            }
        }

        #[rstest]
        fn conflict_creates_error() {
            let error = ApiError::conflict("Resource already exists");
            match error {
                ApiError::Conflict { reason } => {
                    assert_eq!(reason, "Resource already exists");
                }
                _ => panic!("Expected Conflict variant"),
            }
        }

        #[rstest]
        fn internal_creates_error() {
            let error = ApiError::internal("Database error");
            match error {
                ApiError::InternalError { message } => {
                    assert_eq!(message, "Database error");
                }
                _ => panic!("Expected InternalError variant"),
            }
        }
    }

    mod status_code {
        use super::*;

        #[rstest]
        fn not_found_returns_404() {
            let error = ApiError::not_found("GameSession", "abc");
            assert_eq!(error.status_code(), StatusCode::NOT_FOUND);
        }

        #[rstest]
        fn validation_error_returns_400() {
            let error = ApiError::validation("Invalid");
            assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
        }

        #[rstest]
        fn invalid_command_returns_400() {
            let error = ApiError::invalid_command("Invalid");
            assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
        }

        #[rstest]
        fn conflict_returns_409() {
            let error = ApiError::conflict("Conflict");
            assert_eq!(error.status_code(), StatusCode::CONFLICT);
        }

        #[rstest]
        fn internal_error_returns_500() {
            let error = ApiError::internal("Error");
            assert_eq!(error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    mod error_code {
        use super::*;

        #[rstest]
        fn not_found_error_code() {
            let error = ApiError::not_found("GameSession", "abc");
            assert_eq!(error.error_code(), "GAMESESSION_NOT_FOUND");
        }

        #[rstest]
        fn not_found_error_code_with_space() {
            let error = ApiError::not_found("Game Session", "abc");
            assert_eq!(error.error_code(), "GAME_SESSION_NOT_FOUND");
        }

        #[rstest]
        fn validation_error_code() {
            let error = ApiError::validation("Invalid");
            assert_eq!(error.error_code(), "VALIDATION_ERROR");
        }

        #[rstest]
        fn invalid_command_error_code() {
            let error = ApiError::invalid_command("Invalid");
            assert_eq!(error.error_code(), "INVALID_COMMAND");
        }

        #[rstest]
        fn conflict_error_code() {
            let error = ApiError::conflict("Conflict");
            assert_eq!(error.error_code(), "CONFLICT");
        }

        #[rstest]
        fn internal_error_code() {
            let error = ApiError::internal("Error");
            assert_eq!(error.error_code(), "INTERNAL_ERROR");
        }
    }

    mod query_methods {
        use super::*;

        #[rstest]
        fn is_client_error_for_404() {
            let error = ApiError::not_found("GameSession", "abc");
            assert!(error.is_client_error());
            assert!(!error.is_server_error());
        }

        #[rstest]
        fn is_client_error_for_400() {
            let error = ApiError::validation("Invalid");
            assert!(error.is_client_error());
            assert!(!error.is_server_error());
        }

        #[rstest]
        fn is_client_error_for_409() {
            let error = ApiError::conflict("Conflict");
            assert!(error.is_client_error());
            assert!(!error.is_server_error());
        }

        #[rstest]
        fn is_server_error_for_500() {
            let error = ApiError::internal("Error");
            assert!(!error.is_client_error());
            assert!(error.is_server_error());
        }
    }

    mod display {
        use super::*;

        #[rstest]
        fn not_found_display() {
            let error = ApiError::not_found("GameSession", "abc-123");
            assert_eq!(
                error.to_string(),
                "GameSession with identifier 'abc-123' not found"
            );
        }

        #[rstest]
        fn validation_error_display() {
            let error = ApiError::validation("Invalid input");
            assert_eq!(error.to_string(), "Validation error: Invalid input");
        }

        #[rstest]
        fn invalid_command_display() {
            let error = ApiError::invalid_command("Cannot move");
            assert_eq!(error.to_string(), "Invalid command: Cannot move");
        }

        #[rstest]
        fn conflict_display() {
            let error = ApiError::conflict("Already exists");
            assert_eq!(error.to_string(), "Conflict: Already exists");
        }

        #[rstest]
        fn internal_error_display() {
            let error = ApiError::internal("Database failed");
            assert_eq!(error.to_string(), "Internal server error: Database failed");
        }
    }

    mod debug {
        use super::*;

        #[rstest]
        fn not_found_debug() {
            let error = ApiError::not_found("GameSession", "abc");
            let debug_string = format!("{:?}", error);
            assert!(debug_string.contains("NotFound"));
            assert!(debug_string.contains("GameSession"));
        }

        #[rstest]
        fn validation_error_debug() {
            let error = ApiError::validation("Invalid");
            let debug_string = format!("{:?}", error);
            assert!(debug_string.contains("ValidationError"));
        }
    }
}
