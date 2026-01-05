//! Error types for the effect system.
//!
//! This module provides error types that can occur when working with
//! effect transformers, particularly when `IO` or `AsyncIO` computations
//! are consumed more than once.

/// Represents an error when an `IO` or `AsyncIO` has already been consumed.
///
/// This error occurs when a lifted `IO`/`AsyncIO` is executed more than once.
/// `IO` and `AsyncIO` are designed to be consumed exactly once, and attempting
/// to execute them multiple times results in this error.
///
/// # Examples
///
/// ```rust
/// use lambars::effect::AlreadyConsumedError;
///
/// let error = AlreadyConsumedError {
///     transformer_name: "ReaderT",
///     method_name: "try_lift_io",
///     effect_type: "IO",
/// };
/// assert_eq!(
///     format!("{}", error),
///     "ReaderT::try_lift_io: IO already consumed. Use the transformer only once."
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlreadyConsumedError {
    /// The name of the transformer where the error occurred.
    pub transformer_name: &'static str,
    /// The name of the method where the error occurred.
    pub method_name: &'static str,
    /// The type of effect that was consumed (`"IO"` or `"AsyncIO"`).
    pub effect_type: &'static str,
}

impl std::fmt::Display for AlreadyConsumedError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}::{}: {} already consumed. Use the transformer only once.",
            self.transformer_name, self.method_name, self.effect_type
        )
    }
}

impl std::error::Error for AlreadyConsumedError {}

/// Represents errors that can occur in the effect system.
///
/// This enum provides a unified error type for all effect-related errors.
/// Currently, it only contains `AlreadyConsumed`, but it is designed to be
/// extensible for future error types.
///
/// # Examples
///
/// ```rust
/// use lambars::effect::{EffectError, AlreadyConsumedError};
///
/// let error = EffectError::AlreadyConsumed(AlreadyConsumedError {
///     transformer_name: "ReaderT",
///     method_name: "try_lift_io",
///     effect_type: "IO",
/// });
/// println!("{}", error);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectError {
    /// The IO/AsyncIO has already been consumed.
    AlreadyConsumed(AlreadyConsumedError),
}

impl std::fmt::Display for EffectError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyConsumed(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for EffectError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_already_consumed_error_display() {
        let error = AlreadyConsumedError {
            transformer_name: "ReaderT",
            method_name: "try_lift_io",
            effect_type: "IO",
        };
        assert_eq!(
            format!("{error}"),
            "ReaderT::try_lift_io: IO already consumed. Use the transformer only once."
        );
    }

    #[test]
    fn test_already_consumed_error_display_async_io() {
        let error = AlreadyConsumedError {
            transformer_name: "StateT",
            method_name: "try_lift_async_io",
            effect_type: "AsyncIO",
        };
        assert_eq!(
            format!("{error}"),
            "StateT::try_lift_async_io: AsyncIO already consumed. Use the transformer only once."
        );
    }

    #[test]
    fn test_effect_error_display() {
        let error = EffectError::AlreadyConsumed(AlreadyConsumedError {
            transformer_name: "StateT",
            method_name: "try_lift_async_io",
            effect_type: "AsyncIO",
        });
        assert_eq!(
            format!("{error}"),
            "StateT::try_lift_async_io: AsyncIO already consumed. Use the transformer only once."
        );
    }

    #[test]
    fn test_already_consumed_error_equality() {
        let error1 = AlreadyConsumedError {
            transformer_name: "ReaderT",
            method_name: "try_lift_io",
            effect_type: "IO",
        };
        let error2 = AlreadyConsumedError {
            transformer_name: "ReaderT",
            method_name: "try_lift_io",
            effect_type: "IO",
        };
        let error3 = AlreadyConsumedError {
            transformer_name: "StateT",
            method_name: "try_lift_io",
            effect_type: "IO",
        };
        assert_eq!(error1, error2);
        assert_ne!(error1, error3);
    }

    #[test]
    fn test_effect_error_equality() {
        let error1 = EffectError::AlreadyConsumed(AlreadyConsumedError {
            transformer_name: "ReaderT",
            method_name: "try_lift_io",
            effect_type: "IO",
        });
        let error2 = EffectError::AlreadyConsumed(AlreadyConsumedError {
            transformer_name: "ReaderT",
            method_name: "try_lift_io",
            effect_type: "IO",
        });
        assert_eq!(error1, error2);
    }

    #[test]
    fn test_already_consumed_error_clone() {
        let error = AlreadyConsumedError {
            transformer_name: "ReaderT",
            method_name: "try_lift_io",
            effect_type: "IO",
        };
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    #[test]
    fn test_effect_error_clone() {
        let error = EffectError::AlreadyConsumed(AlreadyConsumedError {
            transformer_name: "ReaderT",
            method_name: "try_lift_io",
            effect_type: "IO",
        });
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    #[test]
    fn test_already_consumed_error_debug() {
        let error = AlreadyConsumedError {
            transformer_name: "ReaderT",
            method_name: "try_lift_io",
            effect_type: "IO",
        };
        let debug_string = format!("{error:?}");
        assert!(debug_string.contains("AlreadyConsumedError"));
        assert!(debug_string.contains("ReaderT"));
        assert!(debug_string.contains("try_lift_io"));
        assert!(debug_string.contains("IO"));
    }

    #[test]
    fn test_effect_error_debug() {
        let error = EffectError::AlreadyConsumed(AlreadyConsumedError {
            transformer_name: "ReaderT",
            method_name: "try_lift_io",
            effect_type: "IO",
        });
        let debug_string = format!("{error:?}");
        assert!(debug_string.contains("AlreadyConsumed"));
    }

    #[test]
    fn test_effect_error_source() {
        use std::error::Error;

        let error = EffectError::AlreadyConsumed(AlreadyConsumedError {
            transformer_name: "ReaderT",
            method_name: "try_lift_io",
            effect_type: "IO",
        });
        assert!(error.source().is_none());
    }

    #[test]
    fn test_already_consumed_error_is_error() {
        use std::error::Error;

        let error = AlreadyConsumedError {
            transformer_name: "ReaderT",
            method_name: "try_lift_io",
            effect_type: "IO",
        };
        let _: &dyn Error = &error;
    }

    #[test]
    fn test_already_consumed_error_source() {
        use std::error::Error;

        let error = AlreadyConsumedError {
            transformer_name: "ReaderT",
            method_name: "try_lift_io",
            effect_type: "IO",
        };
        assert!(error.source().is_none());
    }
}
