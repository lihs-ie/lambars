use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

// =============================================================================
// EnemyError
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnemyError {
    EnemyNotFound { enemy_identifier: String },

    EnemyAlreadyDead { enemy_identifier: String },

    InvalidBehaviorPattern,
}

impl EnemyError {
    #[must_use]
    pub fn enemy_not_found(enemy_identifier: impl Into<String>) -> Self {
        Self::EnemyNotFound {
            enemy_identifier: enemy_identifier.into(),
        }
    }

    #[must_use]
    pub fn enemy_already_dead(enemy_identifier: impl Into<String>) -> Self {
        Self::EnemyAlreadyDead {
            enemy_identifier: enemy_identifier.into(),
        }
    }

    #[must_use]
    pub const fn invalid_behavior_pattern() -> Self {
        Self::InvalidBehaviorPattern
    }

    #[must_use]
    pub const fn is_not_found(&self) -> bool {
        matches!(self, Self::EnemyNotFound { .. })
    }

    #[must_use]
    pub const fn is_already_dead(&self) -> bool {
        matches!(self, Self::EnemyAlreadyDead { .. })
    }

    #[must_use]
    pub const fn is_invalid_behavior(&self) -> bool {
        matches!(self, Self::InvalidBehaviorPattern)
    }

    #[must_use]
    pub fn enemy_identifier(&self) -> Option<&str> {
        match self {
            Self::EnemyNotFound { enemy_identifier }
            | Self::EnemyAlreadyDead { enemy_identifier } => Some(enemy_identifier),
            Self::InvalidBehaviorPattern => None,
        }
    }

    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::EnemyNotFound { enemy_identifier } => {
                format!("Enemy not found: {}", enemy_identifier)
            }
            Self::EnemyAlreadyDead { enemy_identifier } => {
                format!("Enemy is already dead: {}", enemy_identifier)
            }
            Self::InvalidBehaviorPattern => "Invalid behavior pattern".to_string(),
        }
    }

    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(self, Self::InvalidBehaviorPattern)
    }
}

impl fmt::Display for EnemyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message())
    }
}

impl Error for EnemyError {}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // Construction Tests
    // =========================================================================

    mod construction {
        use super::*;

        #[rstest]
        fn enemy_not_found_with_string() {
            let error = EnemyError::enemy_not_found("abc-123");
            assert!(
                matches!(error, EnemyError::EnemyNotFound { enemy_identifier } if enemy_identifier == "abc-123")
            );
        }

        #[rstest]
        fn enemy_not_found_with_string_ref() {
            let identifier = String::from("abc-123");
            let error = EnemyError::enemy_not_found(&identifier);
            assert!(
                matches!(error, EnemyError::EnemyNotFound { enemy_identifier } if enemy_identifier == "abc-123")
            );
        }

        #[rstest]
        fn enemy_already_dead_with_string() {
            let error = EnemyError::enemy_already_dead("xyz-789");
            assert!(
                matches!(error, EnemyError::EnemyAlreadyDead { enemy_identifier } if enemy_identifier == "xyz-789")
            );
        }

        #[rstest]
        fn invalid_behavior_pattern() {
            let error = EnemyError::invalid_behavior_pattern();
            assert!(matches!(error, EnemyError::InvalidBehaviorPattern));
        }
    }

    // =========================================================================
    // Predicate Tests
    // =========================================================================

    mod predicates {
        use super::*;

        #[rstest]
        fn is_not_found_true() {
            let error = EnemyError::enemy_not_found("abc-123");
            assert!(error.is_not_found());
        }

        #[rstest]
        fn is_not_found_false_for_already_dead() {
            let error = EnemyError::enemy_already_dead("abc-123");
            assert!(!error.is_not_found());
        }

        #[rstest]
        fn is_not_found_false_for_invalid_behavior() {
            let error = EnemyError::invalid_behavior_pattern();
            assert!(!error.is_not_found());
        }

        #[rstest]
        fn is_already_dead_true() {
            let error = EnemyError::enemy_already_dead("abc-123");
            assert!(error.is_already_dead());
        }

        #[rstest]
        fn is_already_dead_false_for_not_found() {
            let error = EnemyError::enemy_not_found("abc-123");
            assert!(!error.is_already_dead());
        }

        #[rstest]
        fn is_already_dead_false_for_invalid_behavior() {
            let error = EnemyError::invalid_behavior_pattern();
            assert!(!error.is_already_dead());
        }

        #[rstest]
        fn is_invalid_behavior_true() {
            let error = EnemyError::invalid_behavior_pattern();
            assert!(error.is_invalid_behavior());
        }

        #[rstest]
        fn is_invalid_behavior_false_for_not_found() {
            let error = EnemyError::enemy_not_found("abc-123");
            assert!(!error.is_invalid_behavior());
        }

        #[rstest]
        fn is_invalid_behavior_false_for_already_dead() {
            let error = EnemyError::enemy_already_dead("abc-123");
            assert!(!error.is_invalid_behavior());
        }
    }

    // =========================================================================
    // Enemy Identifier Tests
    // =========================================================================

    mod enemy_identifier {
        use super::*;

        #[rstest]
        fn enemy_identifier_for_not_found() {
            let error = EnemyError::enemy_not_found("abc-123");
            assert_eq!(error.enemy_identifier(), Some("abc-123"));
        }

        #[rstest]
        fn enemy_identifier_for_already_dead() {
            let error = EnemyError::enemy_already_dead("xyz-789");
            assert_eq!(error.enemy_identifier(), Some("xyz-789"));
        }

        #[rstest]
        fn enemy_identifier_for_invalid_behavior() {
            let error = EnemyError::invalid_behavior_pattern();
            assert_eq!(error.enemy_identifier(), None);
        }
    }

    // =========================================================================
    // Message Tests
    // =========================================================================

    mod message {
        use super::*;

        #[rstest]
        fn message_for_not_found() {
            let error = EnemyError::enemy_not_found("abc-123");
            let message = error.message();
            assert!(message.contains("not found"));
            assert!(message.contains("abc-123"));
        }

        #[rstest]
        fn message_for_already_dead() {
            let error = EnemyError::enemy_already_dead("xyz-789");
            let message = error.message();
            assert!(message.contains("already dead"));
            assert!(message.contains("xyz-789"));
        }

        #[rstest]
        fn message_for_invalid_behavior() {
            let error = EnemyError::invalid_behavior_pattern();
            let message = error.message();
            assert!(message.contains("Invalid behavior pattern"));
        }
    }

    // =========================================================================
    // Recoverable Tests
    // =========================================================================

    mod recoverable {
        use super::*;

        #[rstest]
        fn not_found_is_not_recoverable() {
            let error = EnemyError::enemy_not_found("abc-123");
            assert!(!error.is_recoverable());
        }

        #[rstest]
        fn already_dead_is_not_recoverable() {
            let error = EnemyError::enemy_already_dead("abc-123");
            assert!(!error.is_recoverable());
        }

        #[rstest]
        fn invalid_behavior_is_recoverable() {
            let error = EnemyError::invalid_behavior_pattern();
            assert!(error.is_recoverable());
        }
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    mod display {
        use super::*;

        #[rstest]
        fn display_not_found() {
            let error = EnemyError::enemy_not_found("abc-123");
            let display = format!("{}", error);
            assert!(display.contains("abc-123"));
            assert!(display.contains("not found"));
        }

        #[rstest]
        fn display_already_dead() {
            let error = EnemyError::enemy_already_dead("xyz-789");
            let display = format!("{}", error);
            assert!(display.contains("xyz-789"));
            assert!(display.contains("already dead"));
        }

        #[rstest]
        fn display_invalid_behavior() {
            let error = EnemyError::invalid_behavior_pattern();
            let display = format!("{}", error);
            assert!(display.contains("Invalid behavior pattern"));
        }
    }

    // =========================================================================
    // Error Trait Tests
    // =========================================================================

    mod error_trait {
        use super::*;

        #[rstest]
        fn implements_error_trait() {
            fn assert_error<E: Error>(_: E) {}

            assert_error(EnemyError::enemy_not_found("abc-123"));
            assert_error(EnemyError::enemy_already_dead("abc-123"));
            assert_error(EnemyError::invalid_behavior_pattern());
        }
    }

    // =========================================================================
    // Clone and Equality Tests
    // =========================================================================

    mod clone_and_equality {
        use super::*;

        #[rstest]
        fn clone_preserves_value() {
            let error = EnemyError::enemy_not_found("abc-123");
            let cloned = error.clone();
            assert_eq!(error, cloned);
        }

        #[rstest]
        fn equality_same_variant_same_id() {
            let error1 = EnemyError::enemy_not_found("abc-123");
            let error2 = EnemyError::enemy_not_found("abc-123");
            assert_eq!(error1, error2);
        }

        #[rstest]
        fn equality_same_variant_different_id() {
            let error1 = EnemyError::enemy_not_found("abc-123");
            let error2 = EnemyError::enemy_not_found("xyz-789");
            assert_ne!(error1, error2);
        }

        #[rstest]
        fn equality_different_variants() {
            let error1 = EnemyError::enemy_not_found("abc-123");
            let error2 = EnemyError::enemy_already_dead("abc-123");
            assert_ne!(error1, error2);
        }
    }
}
