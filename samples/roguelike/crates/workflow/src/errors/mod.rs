//! Error types for the workflow layer.
//!
//! This module provides error types specific to the application workflow layer,
//! including domain error wrapping and infrastructure-related errors.
//!
//! # Error Categories
//!
//! - [`WorkflowError::Domain`]: Wraps domain layer errors
//! - [`WorkflowError::NotFound`]: Entity not found in storage
//! - [`WorkflowError::Conflict`]: State conflicts (e.g., concurrent updates)
//! - [`WorkflowError::Repository`]: Repository operation failures
//! - [`WorkflowError::Cache`]: Cache operation failures
//! - [`WorkflowError::EventStore`]: Event store operation failures
//!
//! # Examples
//!
//! ```
//! use roguelike_workflow::errors::WorkflowError;
//!
//! // Create a not found error
//! let error = WorkflowError::not_found("GameSession", "abc-123");
//! assert!(error.is_not_found());
//!
//! // Create a repository error
//! let error = WorkflowError::repository("save", "connection timeout");
//! assert!(error.is_repository());
//! ```

use roguelike_domain::common::DomainError;
use std::error::Error;
use std::fmt;

// =============================================================================
// WorkflowError
// =============================================================================

/// Error types for workflow operations.
///
/// This enum represents all possible errors that can occur during workflow
/// execution, including domain errors and infrastructure-related errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowError {
    /// A domain error occurred during workflow execution.
    Domain(DomainError),

    /// The requested entity was not found.
    NotFound {
        /// The type of entity that was not found (e.g., "GameSession", "Player").
        entity_type: String,
        /// The identifier used to look up the entity.
        identifier: String,
    },

    /// A conflict occurred (e.g., concurrent modification, duplicate entry).
    Conflict {
        /// The reason for the conflict.
        reason: String,
    },

    /// A repository operation failed.
    Repository {
        /// The operation that failed (e.g., "save", "find_by_id", "delete").
        operation: String,
        /// Detailed error message.
        message: String,
    },

    /// A cache operation failed.
    Cache {
        /// The operation that failed (e.g., "get", "set", "invalidate").
        operation: String,
        /// Detailed error message.
        message: String,
    },

    /// An event store operation failed.
    EventStore {
        /// The operation that failed (e.g., "append", "load_events").
        operation: String,
        /// Detailed error message.
        message: String,
    },
}

// =============================================================================
// Factory Methods
// =============================================================================

impl WorkflowError {
    /// Creates a new `NotFound` error.
    ///
    /// # Arguments
    ///
    /// * `entity_type` - The type of entity that was not found.
    /// * `identifier` - The identifier used to look up the entity.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::errors::WorkflowError;
    ///
    /// let error = WorkflowError::not_found("GameSession", "abc-123");
    /// assert!(error.is_not_found());
    /// ```
    #[must_use]
    pub fn not_found(entity_type: impl Into<String>, identifier: impl Into<String>) -> Self {
        Self::NotFound {
            entity_type: entity_type.into(),
            identifier: identifier.into(),
        }
    }

    /// Creates a new `Conflict` error.
    ///
    /// # Arguments
    ///
    /// * `reason` - The reason for the conflict.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::errors::WorkflowError;
    ///
    /// let error = WorkflowError::conflict("Game session already exists");
    /// assert!(error.is_conflict());
    /// ```
    #[must_use]
    pub fn conflict(reason: impl Into<String>) -> Self {
        Self::Conflict {
            reason: reason.into(),
        }
    }

    /// Creates a new `Repository` error.
    ///
    /// # Arguments
    ///
    /// * `operation` - The operation that failed.
    /// * `message` - Detailed error message.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::errors::WorkflowError;
    ///
    /// let error = WorkflowError::repository("save", "connection timeout");
    /// assert!(error.is_repository());
    /// ```
    #[must_use]
    pub fn repository(operation: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Repository {
            operation: operation.into(),
            message: message.into(),
        }
    }

    /// Creates a new `Cache` error.
    ///
    /// # Arguments
    ///
    /// * `operation` - The operation that failed.
    /// * `message` - Detailed error message.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::errors::WorkflowError;
    ///
    /// let error = WorkflowError::cache("get", "cache miss");
    /// assert!(error.is_cache());
    /// ```
    #[must_use]
    pub fn cache(operation: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Cache {
            operation: operation.into(),
            message: message.into(),
        }
    }

    /// Creates a new `EventStore` error.
    ///
    /// # Arguments
    ///
    /// * `operation` - The operation that failed.
    /// * `message` - Detailed error message.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::errors::WorkflowError;
    ///
    /// let error = WorkflowError::event_store("append", "event version conflict");
    /// assert!(error.is_event_store());
    /// ```
    #[must_use]
    pub fn event_store(operation: impl Into<String>, message: impl Into<String>) -> Self {
        Self::EventStore {
            operation: operation.into(),
            message: message.into(),
        }
    }
}

// =============================================================================
// Query Methods
// =============================================================================

impl WorkflowError {
    /// Returns `true` if this is a domain error.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::errors::WorkflowError;
    /// use roguelike_domain::common::{DomainError, ValidationError};
    ///
    /// let error = WorkflowError::Domain(
    ///     DomainError::Validation(ValidationError::empty_value("field"))
    /// );
    /// assert!(error.is_domain());
    /// ```
    #[must_use]
    pub const fn is_domain(&self) -> bool {
        matches!(self, Self::Domain(_))
    }

    /// Returns `true` if this is a not found error.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::errors::WorkflowError;
    ///
    /// let error = WorkflowError::not_found("GameSession", "abc-123");
    /// assert!(error.is_not_found());
    /// ```
    #[must_use]
    pub const fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound { .. })
    }

    /// Returns `true` if this is a conflict error.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::errors::WorkflowError;
    ///
    /// let error = WorkflowError::conflict("duplicate entry");
    /// assert!(error.is_conflict());
    /// ```
    #[must_use]
    pub const fn is_conflict(&self) -> bool {
        matches!(self, Self::Conflict { .. })
    }

    /// Returns `true` if this is a repository error.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::errors::WorkflowError;
    ///
    /// let error = WorkflowError::repository("save", "timeout");
    /// assert!(error.is_repository());
    /// ```
    #[must_use]
    pub const fn is_repository(&self) -> bool {
        matches!(self, Self::Repository { .. })
    }

    /// Returns `true` if this is a cache error.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::errors::WorkflowError;
    ///
    /// let error = WorkflowError::cache("get", "miss");
    /// assert!(error.is_cache());
    /// ```
    #[must_use]
    pub const fn is_cache(&self) -> bool {
        matches!(self, Self::Cache { .. })
    }

    /// Returns `true` if this is an event store error.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::errors::WorkflowError;
    ///
    /// let error = WorkflowError::event_store("append", "conflict");
    /// assert!(error.is_event_store());
    /// ```
    #[must_use]
    pub const fn is_event_store(&self) -> bool {
        matches!(self, Self::EventStore { .. })
    }

    /// Returns `true` if this error is recoverable.
    ///
    /// Recoverable errors are those that might succeed on retry or with
    /// user intervention:
    /// - Domain validation errors (user can fix input)
    /// - Not found errors (resource might appear later)
    /// - Cache errors (non-critical for most operations)
    ///
    /// Non-recoverable errors:
    /// - Repository errors (typically indicate infrastructure issues)
    /// - Event store errors (may indicate data corruption)
    /// - Conflict errors (require explicit resolution)
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::errors::WorkflowError;
    ///
    /// let not_found = WorkflowError::not_found("GameSession", "abc");
    /// assert!(not_found.is_recoverable());
    ///
    /// let conflict = WorkflowError::conflict("duplicate");
    /// assert!(!conflict.is_recoverable());
    /// ```
    #[must_use]
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Domain(domain_error) => domain_error.is_recoverable(),
            Self::NotFound { .. } => true,
            Self::Cache { .. } => true,
            Self::Conflict { .. } => false,
            Self::Repository { .. } => false,
            Self::EventStore { .. } => false,
        }
    }
}

// =============================================================================
// Display and Error Implementations
// =============================================================================

impl fmt::Display for WorkflowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => write!(formatter, "{}", error),
            Self::NotFound {
                entity_type,
                identifier,
            } => {
                write!(
                    formatter,
                    "{} with identifier '{}' not found",
                    entity_type, identifier
                )
            }
            Self::Conflict { reason } => {
                write!(formatter, "Conflict: {}", reason)
            }
            Self::Repository { operation, message } => {
                write!(formatter, "Repository {} failed: {}", operation, message)
            }
            Self::Cache { operation, message } => {
                write!(formatter, "Cache {} failed: {}", operation, message)
            }
            Self::EventStore { operation, message } => {
                write!(formatter, "EventStore {} failed: {}", operation, message)
            }
        }
    }
}

impl Error for WorkflowError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Domain(error) => Some(error),
            _ => None,
        }
    }
}

// =============================================================================
// From Implementations
// =============================================================================

impl From<DomainError> for WorkflowError {
    fn from(error: DomainError) -> Self {
        Self::Domain(error)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use roguelike_domain::common::ValidationError;
    use rstest::rstest;

    // =========================================================================
    // Factory Method Tests
    // =========================================================================

    mod factory_methods {
        use super::*;

        #[rstest]
        fn not_found_creates_error() {
            let error = WorkflowError::not_found("GameSession", "abc-123");

            match error {
                WorkflowError::NotFound {
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
        fn conflict_creates_error() {
            let error = WorkflowError::conflict("duplicate entry");

            match error {
                WorkflowError::Conflict { reason } => {
                    assert_eq!(reason, "duplicate entry");
                }
                _ => panic!("Expected Conflict variant"),
            }
        }

        #[rstest]
        fn repository_creates_error() {
            let error = WorkflowError::repository("save", "connection timeout");

            match error {
                WorkflowError::Repository { operation, message } => {
                    assert_eq!(operation, "save");
                    assert_eq!(message, "connection timeout");
                }
                _ => panic!("Expected Repository variant"),
            }
        }

        #[rstest]
        fn cache_creates_error() {
            let error = WorkflowError::cache("get", "cache miss");

            match error {
                WorkflowError::Cache { operation, message } => {
                    assert_eq!(operation, "get");
                    assert_eq!(message, "cache miss");
                }
                _ => panic!("Expected Cache variant"),
            }
        }

        #[rstest]
        fn event_store_creates_error() {
            let error = WorkflowError::event_store("append", "version conflict");

            match error {
                WorkflowError::EventStore { operation, message } => {
                    assert_eq!(operation, "append");
                    assert_eq!(message, "version conflict");
                }
                _ => panic!("Expected EventStore variant"),
            }
        }
    }

    // =========================================================================
    // Query Method Tests
    // =========================================================================

    mod query_methods {
        use super::*;

        #[rstest]
        fn is_domain_returns_true_for_domain() {
            let error = WorkflowError::Domain(DomainError::Validation(
                ValidationError::empty_value("field"),
            ));
            assert!(error.is_domain());
        }

        #[rstest]
        fn is_domain_returns_false_for_others() {
            let error = WorkflowError::not_found("GameSession", "abc");
            assert!(!error.is_domain());
        }

        #[rstest]
        fn is_not_found_returns_true_for_not_found() {
            let error = WorkflowError::not_found("GameSession", "abc");
            assert!(error.is_not_found());
        }

        #[rstest]
        fn is_not_found_returns_false_for_others() {
            let error = WorkflowError::conflict("reason");
            assert!(!error.is_not_found());
        }

        #[rstest]
        fn is_conflict_returns_true_for_conflict() {
            let error = WorkflowError::conflict("reason");
            assert!(error.is_conflict());
        }

        #[rstest]
        fn is_conflict_returns_false_for_others() {
            let error = WorkflowError::not_found("GameSession", "abc");
            assert!(!error.is_conflict());
        }

        #[rstest]
        fn is_repository_returns_true_for_repository() {
            let error = WorkflowError::repository("save", "timeout");
            assert!(error.is_repository());
        }

        #[rstest]
        fn is_repository_returns_false_for_others() {
            let error = WorkflowError::not_found("GameSession", "abc");
            assert!(!error.is_repository());
        }

        #[rstest]
        fn is_cache_returns_true_for_cache() {
            let error = WorkflowError::cache("get", "miss");
            assert!(error.is_cache());
        }

        #[rstest]
        fn is_cache_returns_false_for_others() {
            let error = WorkflowError::not_found("GameSession", "abc");
            assert!(!error.is_cache());
        }

        #[rstest]
        fn is_event_store_returns_true_for_event_store() {
            let error = WorkflowError::event_store("append", "conflict");
            assert!(error.is_event_store());
        }

        #[rstest]
        fn is_event_store_returns_false_for_others() {
            let error = WorkflowError::not_found("GameSession", "abc");
            assert!(!error.is_event_store());
        }
    }

    // =========================================================================
    // Recoverability Tests
    // =========================================================================

    mod recoverability {
        use super::*;

        #[rstest]
        fn domain_validation_is_recoverable() {
            let error = WorkflowError::Domain(DomainError::Validation(
                ValidationError::empty_value("field"),
            ));
            assert!(error.is_recoverable());
        }

        #[rstest]
        fn not_found_is_recoverable() {
            let error = WorkflowError::not_found("GameSession", "abc");
            assert!(error.is_recoverable());
        }

        #[rstest]
        fn cache_is_recoverable() {
            let error = WorkflowError::cache("get", "miss");
            assert!(error.is_recoverable());
        }

        #[rstest]
        fn conflict_is_not_recoverable() {
            let error = WorkflowError::conflict("duplicate");
            assert!(!error.is_recoverable());
        }

        #[rstest]
        fn repository_is_not_recoverable() {
            let error = WorkflowError::repository("save", "timeout");
            assert!(!error.is_recoverable());
        }

        #[rstest]
        fn event_store_is_not_recoverable() {
            let error = WorkflowError::event_store("append", "conflict");
            assert!(!error.is_recoverable());
        }
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    mod display {
        use super::*;

        #[rstest]
        fn domain_error_display() {
            let error = WorkflowError::Domain(DomainError::Validation(
                ValidationError::empty_value("name"),
            ));
            let display = format!("{}", error);
            assert!(display.contains("'name' must not be empty"));
        }

        #[rstest]
        fn not_found_display() {
            let error = WorkflowError::not_found("GameSession", "abc-123");
            let display = format!("{}", error);
            assert_eq!(display, "GameSession with identifier 'abc-123' not found");
        }

        #[rstest]
        fn conflict_display() {
            let error = WorkflowError::conflict("duplicate entry");
            let display = format!("{}", error);
            assert_eq!(display, "Conflict: duplicate entry");
        }

        #[rstest]
        fn repository_display() {
            let error = WorkflowError::repository("save", "connection timeout");
            let display = format!("{}", error);
            assert_eq!(display, "Repository save failed: connection timeout");
        }

        #[rstest]
        fn cache_display() {
            let error = WorkflowError::cache("get", "cache miss");
            let display = format!("{}", error);
            assert_eq!(display, "Cache get failed: cache miss");
        }

        #[rstest]
        fn event_store_display() {
            let error = WorkflowError::event_store("append", "version conflict");
            let display = format!("{}", error);
            assert_eq!(display, "EventStore append failed: version conflict");
        }
    }

    // =========================================================================
    // Error Source Tests
    // =========================================================================

    mod error_source {
        use super::*;

        #[rstest]
        fn domain_error_has_source() {
            let error = WorkflowError::Domain(DomainError::Validation(
                ValidationError::empty_value("field"),
            ));
            assert!(error.source().is_some());
        }

        #[rstest]
        fn not_found_has_no_source() {
            let error = WorkflowError::not_found("GameSession", "abc");
            assert!(error.source().is_none());
        }

        #[rstest]
        fn conflict_has_no_source() {
            let error = WorkflowError::conflict("reason");
            assert!(error.source().is_none());
        }

        #[rstest]
        fn repository_has_no_source() {
            let error = WorkflowError::repository("save", "timeout");
            assert!(error.source().is_none());
        }

        #[rstest]
        fn cache_has_no_source() {
            let error = WorkflowError::cache("get", "miss");
            assert!(error.source().is_none());
        }

        #[rstest]
        fn event_store_has_no_source() {
            let error = WorkflowError::event_store("append", "conflict");
            assert!(error.source().is_none());
        }
    }

    // =========================================================================
    // From Trait Tests
    // =========================================================================

    mod from_trait {
        use super::*;

        #[rstest]
        fn from_domain_error() {
            let domain_error = DomainError::Validation(ValidationError::empty_value("field"));
            let workflow_error: WorkflowError = domain_error.clone().into();

            match workflow_error {
                WorkflowError::Domain(error) => {
                    assert_eq!(error, domain_error);
                }
                _ => panic!("Expected Domain variant"),
            }
        }

        #[rstest]
        fn from_validation_error_via_domain() {
            let validation_error = ValidationError::empty_value("field");
            let domain_error: DomainError = validation_error.into();
            let workflow_error: WorkflowError = domain_error.into();

            assert!(workflow_error.is_domain());
        }
    }

    // =========================================================================
    // Equality Tests
    // =========================================================================

    mod equality {
        use super::*;

        #[rstest]
        fn not_found_equality() {
            let error1 = WorkflowError::not_found("GameSession", "abc");
            let error2 = WorkflowError::not_found("GameSession", "abc");
            let error3 = WorkflowError::not_found("GameSession", "xyz");

            assert_eq!(error1, error2);
            assert_ne!(error1, error3);
        }

        #[rstest]
        fn conflict_equality() {
            let error1 = WorkflowError::conflict("reason");
            let error2 = WorkflowError::conflict("reason");
            let error3 = WorkflowError::conflict("other");

            assert_eq!(error1, error2);
            assert_ne!(error1, error3);
        }

        #[rstest]
        fn repository_equality() {
            let error1 = WorkflowError::repository("save", "timeout");
            let error2 = WorkflowError::repository("save", "timeout");
            let error3 = WorkflowError::repository("save", "error");

            assert_eq!(error1, error2);
            assert_ne!(error1, error3);
        }

        #[rstest]
        fn different_variants_not_equal() {
            let error1 = WorkflowError::not_found("GameSession", "abc");
            let error2 = WorkflowError::conflict("reason");

            assert_ne!(error1, error2);
        }
    }

    // =========================================================================
    // Clone Tests
    // =========================================================================

    mod clone_tests {
        use super::*;

        #[rstest]
        fn not_found_clone() {
            let error = WorkflowError::not_found("GameSession", "abc");
            let cloned = error.clone();
            assert_eq!(error, cloned);
        }

        #[rstest]
        fn conflict_clone() {
            let error = WorkflowError::conflict("reason");
            let cloned = error.clone();
            assert_eq!(error, cloned);
        }

        #[rstest]
        fn repository_clone() {
            let error = WorkflowError::repository("save", "timeout");
            let cloned = error.clone();
            assert_eq!(error, cloned);
        }

        #[rstest]
        fn cache_clone() {
            let error = WorkflowError::cache("get", "miss");
            let cloned = error.clone();
            assert_eq!(error, cloned);
        }

        #[rstest]
        fn event_store_clone() {
            let error = WorkflowError::event_store("append", "conflict");
            let cloned = error.clone();
            assert_eq!(error, cloned);
        }

        #[rstest]
        fn domain_clone() {
            let error = WorkflowError::Domain(DomainError::Validation(
                ValidationError::empty_value("field"),
            ));
            let cloned = error.clone();
            assert_eq!(error, cloned);
        }
    }

    // =========================================================================
    // Debug Tests
    // =========================================================================

    mod debug_tests {
        use super::*;

        #[rstest]
        fn not_found_debug() {
            let error = WorkflowError::not_found("GameSession", "abc");
            let debug_string = format!("{:?}", error);
            assert!(debug_string.contains("NotFound"));
            assert!(debug_string.contains("GameSession"));
            assert!(debug_string.contains("abc"));
        }

        #[rstest]
        fn conflict_debug() {
            let error = WorkflowError::conflict("reason");
            let debug_string = format!("{:?}", error);
            assert!(debug_string.contains("Conflict"));
            assert!(debug_string.contains("reason"));
        }

        #[rstest]
        fn repository_debug() {
            let error = WorkflowError::repository("save", "timeout");
            let debug_string = format!("{:?}", error);
            assert!(debug_string.contains("Repository"));
            assert!(debug_string.contains("save"));
            assert!(debug_string.contains("timeout"));
        }

        #[rstest]
        fn cache_debug() {
            let error = WorkflowError::cache("get", "miss");
            let debug_string = format!("{:?}", error);
            assert!(debug_string.contains("Cache"));
            assert!(debug_string.contains("get"));
            assert!(debug_string.contains("miss"));
        }

        #[rstest]
        fn event_store_debug() {
            let error = WorkflowError::event_store("append", "conflict");
            let debug_string = format!("{:?}", error);
            assert!(debug_string.contains("EventStore"));
            assert!(debug_string.contains("append"));
            assert!(debug_string.contains("conflict"));
        }
    }
}
