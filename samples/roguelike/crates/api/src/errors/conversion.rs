//! Error conversion implementations.
//!
//! This module provides `From` implementations for converting domain
//! and workflow errors to API errors.

use roguelike_domain::common::DomainError;
use roguelike_workflow::errors::WorkflowError;

use super::api_error::ApiError;

// =============================================================================
// From<WorkflowError> for ApiError
// =============================================================================

impl From<WorkflowError> for ApiError {
    fn from(error: WorkflowError) -> Self {
        match error {
            WorkflowError::NotFound {
                entity_type,
                identifier,
            } => ApiError::NotFound {
                entity_type,
                identifier,
            },
            WorkflowError::Domain(domain_error) => domain_error.into(),
            WorkflowError::Conflict { reason } => ApiError::Conflict { reason },
            WorkflowError::Repository { operation, message } => ApiError::InternalError {
                message: format!("Repository {} failed: {}", operation, message),
            },
            WorkflowError::Cache { operation, message } => {
                // Cache errors are typically non-critical, log and continue
                tracing::warn!("Cache {} failed: {}", operation, message);
                ApiError::InternalError {
                    message: format!("Cache {} failed: {}", operation, message),
                }
            }
            WorkflowError::EventStore { operation, message } => ApiError::InternalError {
                message: format!("EventStore {} failed: {}", operation, message),
            },
        }
    }
}

// =============================================================================
// From<DomainError> for ApiError
// =============================================================================

impl From<DomainError> for ApiError {
    fn from(error: DomainError) -> Self {
        match error {
            DomainError::Validation(validation_error) => ApiError::ValidationError {
                message: validation_error.message(),
                field: Some(validation_error.field().to_string()),
            },
            DomainError::GameSession(game_session_error) => {
                // Map specific game session errors
                let message = game_session_error.to_string();
                if message.contains("already completed") || message.contains("already ended") {
                    ApiError::Conflict {
                        reason: "Game session has already ended".to_string(),
                    }
                } else {
                    ApiError::InvalidCommand { reason: message }
                }
            }
            DomainError::Enemy(enemy_error) => ApiError::InvalidCommand {
                reason: enemy_error.to_string(),
            },
            DomainError::Floor(floor_error) => {
                let message = floor_error.to_string();
                if message.contains("not walkable") || message.contains("blocked") {
                    ApiError::InvalidCommand {
                        reason: format!("Invalid movement: {}", message),
                    }
                } else {
                    ApiError::InvalidCommand { reason: message }
                }
            }
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use roguelike_domain::common::ValidationError;
    use rstest::rstest;

    mod from_workflow_error {
        use super::*;

        #[rstest]
        fn not_found_converts_to_not_found() {
            let workflow_error = WorkflowError::not_found("GameSession", "abc-123");
            let api_error: ApiError = workflow_error.into();

            match api_error {
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
        fn conflict_converts_to_conflict() {
            let workflow_error = WorkflowError::conflict("Resource already exists");
            let api_error: ApiError = workflow_error.into();

            match api_error {
                ApiError::Conflict { reason } => {
                    assert_eq!(reason, "Resource already exists");
                }
                _ => panic!("Expected Conflict variant"),
            }
        }

        #[rstest]
        fn repository_converts_to_internal() {
            let workflow_error = WorkflowError::repository("save", "connection timeout");
            let api_error: ApiError = workflow_error.into();

            match api_error {
                ApiError::InternalError { message } => {
                    assert!(message.contains("Repository"));
                    assert!(message.contains("save"));
                    assert!(message.contains("connection timeout"));
                }
                _ => panic!("Expected InternalError variant"),
            }
        }

        #[rstest]
        fn cache_converts_to_internal() {
            let workflow_error = WorkflowError::cache("get", "cache miss");
            let api_error: ApiError = workflow_error.into();

            match api_error {
                ApiError::InternalError { message } => {
                    assert!(message.contains("Cache"));
                }
                _ => panic!("Expected InternalError variant"),
            }
        }

        #[rstest]
        fn event_store_converts_to_internal() {
            let workflow_error = WorkflowError::event_store("append", "conflict");
            let api_error: ApiError = workflow_error.into();

            match api_error {
                ApiError::InternalError { message } => {
                    assert!(message.contains("EventStore"));
                }
                _ => panic!("Expected InternalError variant"),
            }
        }

        #[rstest]
        fn domain_validation_converts_to_validation_error() {
            let validation_error = ValidationError::empty_value("player_name");
            let domain_error = DomainError::Validation(validation_error);
            let workflow_error = WorkflowError::Domain(domain_error);
            let api_error: ApiError = workflow_error.into();

            match api_error {
                ApiError::ValidationError { message, field } => {
                    assert!(message.contains("player_name"));
                    assert_eq!(field, Some("player_name".to_string()));
                }
                _ => panic!("Expected ValidationError variant"),
            }
        }
    }

    mod from_domain_error {
        use super::*;

        #[rstest]
        fn validation_error_converts_correctly() {
            let validation_error = ValidationError::out_of_range("level", 1, 99, 100);
            let domain_error = DomainError::Validation(validation_error);
            let api_error: ApiError = domain_error.into();

            match api_error {
                ApiError::ValidationError { message, field } => {
                    assert!(message.contains("level"));
                    assert!(message.contains("1"));
                    assert!(message.contains("99"));
                    assert_eq!(field, Some("level".to_string()));
                }
                _ => panic!("Expected ValidationError variant"),
            }
        }

        #[rstest]
        fn validation_error_has_400_status() {
            let validation_error = ValidationError::empty_value("name");
            let domain_error = DomainError::Validation(validation_error);
            let api_error: ApiError = domain_error.into();

            assert_eq!(api_error.status_code(), StatusCode::BAD_REQUEST);
        }
    }

    mod status_codes {
        use super::*;

        #[rstest]
        fn workflow_not_found_maps_to_404() {
            let workflow_error = WorkflowError::not_found("GameSession", "abc");
            let api_error: ApiError = workflow_error.into();
            assert_eq!(api_error.status_code(), StatusCode::NOT_FOUND);
        }

        #[rstest]
        fn workflow_conflict_maps_to_409() {
            let workflow_error = WorkflowError::conflict("duplicate");
            let api_error: ApiError = workflow_error.into();
            assert_eq!(api_error.status_code(), StatusCode::CONFLICT);
        }

        #[rstest]
        fn workflow_repository_maps_to_500() {
            let workflow_error = WorkflowError::repository("save", "error");
            let api_error: ApiError = workflow_error.into();
            assert_eq!(api_error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        }

        #[rstest]
        fn workflow_event_store_maps_to_500() {
            let workflow_error = WorkflowError::event_store("append", "error");
            let api_error: ApiError = workflow_error.into();
            assert_eq!(api_error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        }

        #[rstest]
        fn domain_validation_maps_to_400() {
            let validation_error = ValidationError::empty_value("name");
            let domain_error = DomainError::Validation(validation_error);
            let api_error: ApiError = domain_error.into();
            assert_eq!(api_error.status_code(), StatusCode::BAD_REQUEST);
        }
    }
}
