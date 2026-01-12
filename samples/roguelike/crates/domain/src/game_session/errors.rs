use std::error::Error;
use std::fmt;

use super::GameIdentifier;

// =============================================================================
// GameSessionError
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameSessionError {
    SessionNotFound { session_identifier: GameIdentifier },
    SessionAlreadyExists { session_identifier: GameIdentifier },
    SessionAlreadyCompleted,
    InvalidSeed,
    EventSequenceOutOfOrder { expected: u64, actual: u64 },
}

impl GameSessionError {
    #[must_use]
    pub const fn session_not_found(session_identifier: GameIdentifier) -> Self {
        Self::SessionNotFound { session_identifier }
    }

    #[must_use]
    pub const fn session_already_exists(session_identifier: GameIdentifier) -> Self {
        Self::SessionAlreadyExists { session_identifier }
    }

    #[must_use]
    pub const fn session_already_completed() -> Self {
        Self::SessionAlreadyCompleted
    }

    #[must_use]
    pub const fn invalid_seed() -> Self {
        Self::InvalidSeed
    }

    #[must_use]
    pub const fn event_sequence_out_of_order(expected: u64, actual: u64) -> Self {
        Self::EventSequenceOutOfOrder { expected, actual }
    }

    #[must_use]
    pub const fn is_not_found(&self) -> bool {
        matches!(self, Self::SessionNotFound { .. })
    }

    #[must_use]
    pub const fn is_already_exists(&self) -> bool {
        matches!(self, Self::SessionAlreadyExists { .. })
    }

    #[must_use]
    pub const fn is_already_completed(&self) -> bool {
        matches!(self, Self::SessionAlreadyCompleted)
    }

    #[must_use]
    pub const fn is_invalid_seed(&self) -> bool {
        matches!(self, Self::InvalidSeed)
    }

    #[must_use]
    pub const fn is_event_sequence_out_of_order(&self) -> bool {
        matches!(self, Self::EventSequenceOutOfOrder { .. })
    }

    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        match self {
            Self::SessionNotFound { .. } => true,
            Self::SessionAlreadyExists { .. } => true,
            Self::SessionAlreadyCompleted => false,
            Self::InvalidSeed => true,
            Self::EventSequenceOutOfOrder { .. } => false,
        }
    }

    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::SessionNotFound { session_identifier } => {
                format!("Game session not found: {}", session_identifier)
            }
            Self::SessionAlreadyExists { session_identifier } => {
                format!("Game session already exists: {}", session_identifier)
            }
            Self::SessionAlreadyCompleted => {
                "Game session has already been completed and cannot be modified".to_string()
            }
            Self::InvalidSeed => "Invalid random seed provided".to_string(),
            Self::EventSequenceOutOfOrder { expected, actual } => {
                format!(
                    "Event sequence out of order: expected {}, got {}",
                    expected, actual
                )
            }
        }
    }
}

impl fmt::Display for GameSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message())
    }
}

impl Error for GameSessionError {}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // Constructor Tests
    // =========================================================================

    mod constructors {
        use super::*;

        #[rstest]
        fn session_not_found() {
            let identifier = GameIdentifier::new();
            let error = GameSessionError::session_not_found(identifier);
            assert!(matches!(
                error,
                GameSessionError::SessionNotFound { session_identifier } if session_identifier == identifier
            ));
        }

        #[rstest]
        fn session_already_exists() {
            let identifier = GameIdentifier::new();
            let error = GameSessionError::session_already_exists(identifier);
            assert!(matches!(
                error,
                GameSessionError::SessionAlreadyExists { session_identifier } if session_identifier == identifier
            ));
        }

        #[rstest]
        fn session_already_completed() {
            let error = GameSessionError::session_already_completed();
            assert!(matches!(error, GameSessionError::SessionAlreadyCompleted));
        }

        #[rstest]
        fn invalid_seed() {
            let error = GameSessionError::invalid_seed();
            assert!(matches!(error, GameSessionError::InvalidSeed));
        }

        #[rstest]
        fn event_sequence_out_of_order() {
            let error = GameSessionError::event_sequence_out_of_order(5, 3);
            assert!(matches!(
                error,
                GameSessionError::EventSequenceOutOfOrder {
                    expected: 5,
                    actual: 3
                }
            ));
        }
    }

    // =========================================================================
    // Predicate Tests
    // =========================================================================

    mod predicates {
        use super::*;

        #[rstest]
        fn is_not_found_returns_true_for_session_not_found() {
            let error = GameSessionError::session_not_found(GameIdentifier::new());
            assert!(error.is_not_found());
        }

        #[rstest]
        fn is_not_found_returns_false_for_other_errors() {
            assert!(!GameSessionError::session_already_completed().is_not_found());
            assert!(!GameSessionError::invalid_seed().is_not_found());
        }

        #[rstest]
        fn is_already_exists_returns_true_for_session_already_exists() {
            let error = GameSessionError::session_already_exists(GameIdentifier::new());
            assert!(error.is_already_exists());
        }

        #[rstest]
        fn is_already_exists_returns_false_for_other_errors() {
            assert!(!GameSessionError::session_already_completed().is_already_exists());
            assert!(!GameSessionError::invalid_seed().is_already_exists());
        }

        #[rstest]
        fn is_already_completed_returns_true_for_session_already_completed() {
            let error = GameSessionError::session_already_completed();
            assert!(error.is_already_completed());
        }

        #[rstest]
        fn is_already_completed_returns_false_for_other_errors() {
            assert!(
                !GameSessionError::session_not_found(GameIdentifier::new()).is_already_completed()
            );
            assert!(!GameSessionError::invalid_seed().is_already_completed());
        }

        #[rstest]
        fn is_invalid_seed_returns_true_for_invalid_seed() {
            let error = GameSessionError::invalid_seed();
            assert!(error.is_invalid_seed());
        }

        #[rstest]
        fn is_invalid_seed_returns_false_for_other_errors() {
            assert!(!GameSessionError::session_already_completed().is_invalid_seed());
            assert!(!GameSessionError::session_not_found(GameIdentifier::new()).is_invalid_seed());
        }

        #[rstest]
        fn is_event_sequence_out_of_order_returns_true_for_event_sequence_out_of_order() {
            let error = GameSessionError::event_sequence_out_of_order(5, 3);
            assert!(error.is_event_sequence_out_of_order());
        }

        #[rstest]
        fn is_event_sequence_out_of_order_returns_false_for_other_errors() {
            assert!(
                !GameSessionError::session_already_completed().is_event_sequence_out_of_order()
            );
            assert!(!GameSessionError::invalid_seed().is_event_sequence_out_of_order());
        }
    }

    // =========================================================================
    // Recoverable Tests
    // =========================================================================

    mod recoverable {
        use super::*;

        #[rstest]
        fn session_not_found_is_recoverable() {
            let error = GameSessionError::session_not_found(GameIdentifier::new());
            assert!(error.is_recoverable());
        }

        #[rstest]
        fn session_already_exists_is_recoverable() {
            let error = GameSessionError::session_already_exists(GameIdentifier::new());
            assert!(error.is_recoverable());
        }

        #[rstest]
        fn session_already_completed_is_not_recoverable() {
            let error = GameSessionError::session_already_completed();
            assert!(!error.is_recoverable());
        }

        #[rstest]
        fn invalid_seed_is_recoverable() {
            let error = GameSessionError::invalid_seed();
            assert!(error.is_recoverable());
        }

        #[rstest]
        fn event_sequence_out_of_order_is_not_recoverable() {
            let error = GameSessionError::event_sequence_out_of_order(5, 3);
            assert!(!error.is_recoverable());
        }
    }

    // =========================================================================
    // Message Tests
    // =========================================================================

    mod message {
        use super::*;

        #[rstest]
        fn session_not_found_message() {
            let identifier = GameIdentifier::new();
            let error = GameSessionError::session_not_found(identifier);
            let message = error.message();
            assert!(message.contains("not found"));
            assert!(message.contains(&identifier.to_string()));
        }

        #[rstest]
        fn session_already_exists_message() {
            let identifier = GameIdentifier::new();
            let error = GameSessionError::session_already_exists(identifier);
            let message = error.message();
            assert!(message.contains("already exists"));
            assert!(message.contains(&identifier.to_string()));
        }

        #[rstest]
        fn session_already_completed_message() {
            let error = GameSessionError::session_already_completed();
            let message = error.message();
            assert!(message.contains("completed"));
            assert!(message.contains("cannot be modified"));
        }

        #[rstest]
        fn invalid_seed_message() {
            let error = GameSessionError::invalid_seed();
            let message = error.message();
            assert!(message.contains("Invalid"));
            assert!(message.contains("seed"));
        }

        #[rstest]
        fn event_sequence_out_of_order_message() {
            let error = GameSessionError::event_sequence_out_of_order(5, 3);
            let message = error.message();
            assert!(message.contains("sequence"));
            assert!(message.contains("5"));
            assert!(message.contains("3"));
        }
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    mod display {
        use super::*;

        #[rstest]
        fn display_matches_message() {
            let error = GameSessionError::session_already_completed();
            assert_eq!(format!("{}", error), error.message());
        }

        #[rstest]
        fn display_for_all_variants() {
            let variants = vec![
                GameSessionError::session_not_found(GameIdentifier::new()),
                GameSessionError::session_already_exists(GameIdentifier::new()),
                GameSessionError::session_already_completed(),
                GameSessionError::invalid_seed(),
                GameSessionError::event_sequence_out_of_order(5, 3),
            ];

            for error in variants {
                let display = format!("{}", error);
                assert!(!display.is_empty());
            }
        }
    }

    // =========================================================================
    // Trait Implementation Tests
    // =========================================================================

    mod traits {
        use super::*;

        #[rstest]
        fn equality() {
            let identifier = GameIdentifier::new();
            let error1 = GameSessionError::session_not_found(identifier);
            let error2 = GameSessionError::session_not_found(identifier);
            let error3 = GameSessionError::session_not_found(GameIdentifier::new());

            assert_eq!(error1, error2);
            assert_ne!(error1, error3);
        }

        #[rstest]
        fn equality_for_unit_variants() {
            assert_eq!(
                GameSessionError::session_already_completed(),
                GameSessionError::session_already_completed()
            );
            assert_eq!(
                GameSessionError::invalid_seed(),
                GameSessionError::invalid_seed()
            );
        }

        #[rstest]
        fn clone() {
            let error = GameSessionError::session_not_found(GameIdentifier::new());
            let cloned = error.clone();
            assert_eq!(error, cloned);
        }

        #[rstest]
        fn debug_format() {
            let error = GameSessionError::session_already_completed();
            let debug = format!("{:?}", error);
            assert!(debug.contains("SessionAlreadyCompleted"));
        }

        #[rstest]
        fn implements_error_trait() {
            let error: Box<dyn Error> = Box::new(GameSessionError::invalid_seed());
            assert!(error.source().is_none());
        }
    }
}
