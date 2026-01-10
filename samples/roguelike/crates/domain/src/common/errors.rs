//! Error types for the common domain module.
//!
//! This module provides validation and domain error types used across
//! all subdomains in the roguelike game.

use std::error::Error;
use std::fmt;

// =============================================================================
// ValidationError
// =============================================================================

/// Validation error variants for domain value objects.
///
/// This enum represents common validation failures that can occur when
/// constructing value objects with constraints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// A required field was empty.
    EmptyValue {
        /// The name of the field that was empty.
        field: String,
    },
    /// A value was outside the valid range.
    OutOfRange {
        /// The name of the field that was out of range.
        field: String,
        /// The minimum allowed value (as string for flexibility).
        min: String,
        /// The maximum allowed value (as string for flexibility).
        max: String,
        /// The actual value that was provided (as string).
        actual: String,
    },
    /// A value had an invalid format.
    InvalidFormat {
        /// The name of the field with invalid format.
        field: String,
        /// Description of the expected format.
        expected: String,
    },
    /// A constraint was violated.
    ConstraintViolation {
        /// The name of the field that violated the constraint.
        field: String,
        /// Description of the constraint that was violated.
        constraint: String,
    },
}

impl ValidationError {
    /// Returns the name of the field that caused the error.
    pub fn field(&self) -> &str {
        match self {
            Self::EmptyValue { field }
            | Self::OutOfRange { field, .. }
            | Self::InvalidFormat { field, .. }
            | Self::ConstraintViolation { field, .. } => field,
        }
    }

    /// Returns a human-readable error message.
    pub fn message(&self) -> String {
        match self {
            Self::EmptyValue { field } => {
                format!("'{}' must not be empty", field)
            }
            Self::OutOfRange {
                field,
                min,
                max,
                actual,
            } => {
                format!(
                    "'{}' must be between {} and {}, but was {}",
                    field, min, max, actual
                )
            }
            Self::InvalidFormat { field, expected } => {
                format!("'{}' has invalid format: expected {}", field, expected)
            }
            Self::ConstraintViolation { field, constraint } => {
                format!("'{}' violates constraint: {}", field, constraint)
            }
        }
    }

    /// Creates an empty value error.
    pub fn empty_value(field: impl Into<String>) -> Self {
        Self::EmptyValue {
            field: field.into(),
        }
    }

    /// Creates an out of range error.
    pub fn out_of_range(
        field: impl Into<String>,
        min: impl ToString,
        max: impl ToString,
        actual: impl ToString,
    ) -> Self {
        Self::OutOfRange {
            field: field.into(),
            min: min.to_string(),
            max: max.to_string(),
            actual: actual.to_string(),
        }
    }

    /// Creates an invalid format error.
    pub fn invalid_format(field: impl Into<String>, expected: impl Into<String>) -> Self {
        Self::InvalidFormat {
            field: field.into(),
            expected: expected.into(),
        }
    }

    /// Creates a constraint violation error.
    pub fn constraint_violation(field: impl Into<String>, constraint: impl Into<String>) -> Self {
        Self::ConstraintViolation {
            field: field.into(),
            constraint: constraint.into(),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message())
    }
}

impl Error for ValidationError {}

// =============================================================================
// DomainError
// =============================================================================

/// Domain-level error types.
///
/// This enum wraps all subdomain errors and provides a unified error type
/// for the domain layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    /// A validation error occurred.
    Validation(ValidationError),
    /// A game session error occurred.
    GameSession(crate::game_session::GameSessionError),
    /// An enemy error occurred.
    Enemy(crate::enemy::EnemyError),
    /// A floor error occurred.
    Floor(crate::floor::FloorError),
    // Future subdomain errors will be added here:
    // Player(PlayerError),
    // Combat(CombatError),
    // Item(ItemError),
    // Command(CommandError),
}

impl DomainError {
    /// Returns true if this is a validation error.
    pub fn is_validation_error(&self) -> bool {
        matches!(self, Self::Validation(_))
    }

    /// Returns true if this is a game session error.
    pub fn is_game_session_error(&self) -> bool {
        matches!(self, Self::GameSession(_))
    }

    /// Returns true if this is an enemy error.
    pub fn is_enemy_error(&self) -> bool {
        matches!(self, Self::Enemy(_))
    }

    /// Returns true if this is a floor error.
    pub fn is_floor_error(&self) -> bool {
        matches!(self, Self::Floor(_))
    }

    /// Returns true if this error is recoverable.
    ///
    /// Validation errors are generally recoverable as they indicate
    /// invalid input that can be corrected by the user.
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Validation(_) => true,
            Self::GameSession(error) => error.is_recoverable(),
            Self::Enemy(error) => error.is_recoverable(),
            Self::Floor(error) => error.is_recoverable(),
        }
    }
}

impl fmt::Display for DomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(error) => write!(formatter, "Validation error: {}", error),
            Self::GameSession(error) => write!(formatter, "Game session error: {}", error),
            Self::Enemy(error) => write!(formatter, "Enemy error: {}", error),
            Self::Floor(error) => write!(formatter, "Floor error: {}", error),
        }
    }
}

impl Error for DomainError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Validation(error) => Some(error),
            Self::GameSession(error) => Some(error),
            Self::Enemy(error) => Some(error),
            Self::Floor(error) => Some(error),
        }
    }
}

impl From<ValidationError> for DomainError {
    fn from(error: ValidationError) -> Self {
        Self::Validation(error)
    }
}

impl From<crate::game_session::GameSessionError> for DomainError {
    fn from(error: crate::game_session::GameSessionError) -> Self {
        Self::GameSession(error)
    }
}

impl From<crate::enemy::EnemyError> for DomainError {
    fn from(error: crate::enemy::EnemyError) -> Self {
        Self::Enemy(error)
    }
}

impl From<crate::floor::FloorError> for DomainError {
    fn from(error: crate::floor::FloorError) -> Self {
        Self::Floor(error)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // ValidationError Tests
    // =========================================================================

    mod validation_error {
        use super::*;

        #[rstest]
        fn empty_value_field() {
            let error = ValidationError::empty_value("name");
            assert_eq!(error.field(), "name");
        }

        #[rstest]
        fn empty_value_message() {
            let error = ValidationError::empty_value("description");
            assert_eq!(error.message(), "'description' must not be empty");
        }

        #[rstest]
        fn empty_value_display() {
            let error = ValidationError::empty_value("title");
            assert_eq!(format!("{}", error), "'title' must not be empty");
        }

        #[rstest]
        fn out_of_range_field() {
            let error = ValidationError::out_of_range("level", 1, 99, 100);
            assert_eq!(error.field(), "level");
        }

        #[rstest]
        fn out_of_range_message() {
            let error = ValidationError::out_of_range("health", 0, 9999, 10000);
            assert_eq!(
                error.message(),
                "'health' must be between 0 and 9999, but was 10000"
            );
        }

        #[rstest]
        fn out_of_range_display() {
            let error = ValidationError::out_of_range("stat", 1, 99, 0);
            assert_eq!(
                format!("{}", error),
                "'stat' must be between 1 and 99, but was 0"
            );
        }

        #[rstest]
        fn invalid_format_field() {
            let error = ValidationError::invalid_format("email", "valid email address");
            assert_eq!(error.field(), "email");
        }

        #[rstest]
        fn invalid_format_message() {
            let error = ValidationError::invalid_format("date", "YYYY-MM-DD");
            assert_eq!(
                error.message(),
                "'date' has invalid format: expected YYYY-MM-DD"
            );
        }

        #[rstest]
        fn invalid_format_display() {
            let error = ValidationError::invalid_format("uuid", "UUID v4 format");
            assert_eq!(
                format!("{}", error),
                "'uuid' has invalid format: expected UUID v4 format"
            );
        }

        #[rstest]
        fn constraint_violation_field() {
            let error =
                ValidationError::constraint_violation("health", "must not exceed max_health");
            assert_eq!(error.field(), "health");
        }

        #[rstest]
        fn constraint_violation_message() {
            let error = ValidationError::constraint_violation("mana", "must not exceed max_mana");
            assert_eq!(
                error.message(),
                "'mana' violates constraint: must not exceed max_mana"
            );
        }

        #[rstest]
        fn constraint_violation_display() {
            let error = ValidationError::constraint_violation(
                "password",
                "must contain special characters",
            );
            assert_eq!(
                format!("{}", error),
                "'password' violates constraint: must contain special characters"
            );
        }

        #[rstest]
        fn equality() {
            let error1 = ValidationError::empty_value("field");
            let error2 = ValidationError::empty_value("field");
            let error3 = ValidationError::empty_value("other_field");

            assert_eq!(error1, error2);
            assert_ne!(error1, error3);
        }

        #[rstest]
        fn clone() {
            let error = ValidationError::out_of_range("value", 0, 100, 150);
            let cloned = error.clone();
            assert_eq!(error, cloned);
        }

        #[rstest]
        fn debug_format() {
            let error = ValidationError::empty_value("test");
            let debug_string = format!("{:?}", error);
            assert!(debug_string.contains("EmptyValue"));
            assert!(debug_string.contains("test"));
        }
    }

    // =========================================================================
    // DomainError Tests
    // =========================================================================

    mod domain_error {
        use super::*;
        use crate::enemy::EnemyError;

        #[rstest]
        fn from_validation_error() {
            let validation_error = ValidationError::empty_value("field");
            let domain_error: DomainError = validation_error.clone().into();

            assert!(matches!(domain_error, DomainError::Validation(_)));
        }

        #[rstest]
        fn from_enemy_error() {
            let enemy_error = EnemyError::enemy_not_found("abc-123");
            let domain_error: DomainError = enemy_error.clone().into();

            assert!(matches!(domain_error, DomainError::Enemy(_)));
        }

        #[rstest]
        fn is_validation_error_returns_true_for_validation() {
            let error = DomainError::Validation(ValidationError::empty_value("field"));
            assert!(error.is_validation_error());
        }

        #[rstest]
        fn is_enemy_error_returns_true_for_enemy() {
            let error = DomainError::Enemy(EnemyError::enemy_not_found("abc-123"));
            assert!(error.is_enemy_error());
        }

        #[rstest]
        fn is_enemy_error_returns_false_for_validation() {
            let error = DomainError::Validation(ValidationError::empty_value("field"));
            assert!(!error.is_enemy_error());
        }

        #[rstest]
        fn is_recoverable_returns_true_for_validation() {
            let error = DomainError::Validation(ValidationError::empty_value("field"));
            assert!(error.is_recoverable());
        }

        #[rstest]
        fn is_recoverable_returns_false_for_enemy_not_found() {
            let error = DomainError::Enemy(EnemyError::enemy_not_found("abc-123"));
            assert!(!error.is_recoverable());
        }

        #[rstest]
        fn is_recoverable_returns_true_for_invalid_behavior() {
            let error = DomainError::Enemy(EnemyError::invalid_behavior_pattern());
            assert!(error.is_recoverable());
        }

        #[rstest]
        fn display() {
            let error = DomainError::Validation(ValidationError::empty_value("name"));
            assert_eq!(
                format!("{}", error),
                "Validation error: 'name' must not be empty"
            );
        }

        #[rstest]
        fn display_enemy_error() {
            let error = DomainError::Enemy(EnemyError::enemy_not_found("abc-123"));
            let display = format!("{}", error);
            assert!(display.contains("Enemy error"));
            assert!(display.contains("abc-123"));
        }

        #[rstest]
        fn source() {
            let error = DomainError::Validation(ValidationError::empty_value("field"));
            let source = error.source();
            assert!(source.is_some());
        }

        #[rstest]
        fn source_enemy_error() {
            let error = DomainError::Enemy(EnemyError::enemy_not_found("abc-123"));
            let source = error.source();
            assert!(source.is_some());
        }

        #[rstest]
        fn equality() {
            let error1 = DomainError::Validation(ValidationError::empty_value("field"));
            let error2 = DomainError::Validation(ValidationError::empty_value("field"));
            let error3 = DomainError::Validation(ValidationError::empty_value("other"));

            assert_eq!(error1, error2);
            assert_ne!(error1, error3);
        }

        #[rstest]
        fn clone() {
            let error = DomainError::Validation(ValidationError::empty_value("test"));
            let cloned = error.clone();
            assert_eq!(error, cloned);
        }

        #[rstest]
        fn debug_format() {
            let error = DomainError::Validation(ValidationError::empty_value("field"));
            let debug_string = format!("{:?}", error);
            assert!(debug_string.contains("Validation"));
        }
    }
}
