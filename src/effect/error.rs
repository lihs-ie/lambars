//! Error types for the effect system.
//!
//! This module provides error types that can occur when working with
//! effect transformers, particularly when `IO` or `AsyncIO` computations
//! are consumed more than once.

/// Represents the type of effect that was consumed.
///
/// This enum is used to identify which type of effect (`IO` or `AsyncIO`)
/// was already consumed when an `AlreadyConsumedError` occurs.
///
/// # Examples
///
/// ```rust
/// use lambars::effect::EffectType;
///
/// let effect_type = EffectType::IO;
/// assert_eq!(format!("{}", effect_type), "IO");
///
/// let async_effect_type = EffectType::AsyncIO;
/// assert_eq!(format!("{}", async_effect_type), "AsyncIO");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectType {
    /// Synchronous IO effect.
    IO,
    /// Asynchronous IO effect.
    AsyncIO,
}

impl std::fmt::Display for EffectType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IO => write!(formatter, "IO"),
            Self::AsyncIO => write!(formatter, "AsyncIO"),
        }
    }
}

/// Represents an error when an `IO` or `AsyncIO` has already been consumed.
///
/// This error occurs when a lifted `IO`/`AsyncIO` is executed more than once.
/// `IO` and `AsyncIO` are designed to be consumed exactly once, and attempting
/// to execute them multiple times results in this error.
///
/// # Examples
///
/// ```rust
/// use lambars::effect::{AlreadyConsumedError, EffectType};
///
/// let error = AlreadyConsumedError {
///     transformer_name: "ReaderT",
///     method_name: "try_lift_io",
///     effect_type: EffectType::IO,
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
    /// The type of effect that was consumed.
    pub effect_type: EffectType,
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
/// use lambars::effect::{EffectError, AlreadyConsumedError, EffectType};
///
/// let error = EffectError::AlreadyConsumed(AlreadyConsumedError {
///     transformer_name: "ReaderT",
///     method_name: "try_lift_io",
///     effect_type: EffectType::IO,
/// });
/// println!("{}", error);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectError {
    /// An `IO` or `AsyncIO` has already been consumed.
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
    use rstest::rstest;
    use std::error::Error;

    #[rstest]
    #[case(EffectType::IO, "IO")]
    #[case(EffectType::AsyncIO, "AsyncIO")]
    fn effect_type_display(#[case] effect_type: EffectType, #[case] expected: &str) {
        assert_eq!(format!("{effect_type}"), expected);
    }

    #[rstest]
    #[case(EffectType::IO, EffectType::IO, true)]
    #[case(EffectType::AsyncIO, EffectType::AsyncIO, true)]
    #[case(EffectType::IO, EffectType::AsyncIO, false)]
    fn effect_type_equality(
        #[case] effect_type1: EffectType,
        #[case] effect_type2: EffectType,
        #[case] expected: bool,
    ) {
        assert_eq!(effect_type1 == effect_type2, expected);
    }

    #[rstest]
    #[case(EffectType::IO)]
    #[case(EffectType::AsyncIO)]
    fn effect_type_clone(#[case] effect_type: EffectType) {
        let cloned = effect_type;
        assert_eq!(effect_type, cloned);
    }

    #[rstest]
    #[case(EffectType::IO)]
    #[case(EffectType::AsyncIO)]
    fn effect_type_debug(#[case] effect_type: EffectType) {
        let debug_string = format!("{effect_type:?}");
        assert!(!debug_string.is_empty());
    }

    #[rstest]
    #[case(
        "ReaderT",
        "try_lift_io",
        EffectType::IO,
        "ReaderT::try_lift_io: IO already consumed. Use the transformer only once."
    )]
    #[case(
        "StateT",
        "try_lift_async_io",
        EffectType::AsyncIO,
        "StateT::try_lift_async_io: AsyncIO already consumed. Use the transformer only once."
    )]
    #[case(
        "ReaderT",
        "try_lift_async_io",
        EffectType::AsyncIO,
        "ReaderT::try_lift_async_io: AsyncIO already consumed. Use the transformer only once."
    )]
    #[case(
        "StateT",
        "try_lift_io",
        EffectType::IO,
        "StateT::try_lift_io: IO already consumed. Use the transformer only once."
    )]
    fn already_consumed_error_display(
        #[case] transformer_name: &'static str,
        #[case] method_name: &'static str,
        #[case] effect_type: EffectType,
        #[case] expected: &str,
    ) {
        let error = AlreadyConsumedError {
            transformer_name,
            method_name,
            effect_type,
        };
        assert_eq!(format!("{error}"), expected);
    }

    #[rstest]
    #[case(
        "ReaderT",
        "try_lift_io",
        EffectType::IO,
        "ReaderT",
        "try_lift_io",
        EffectType::IO,
        true
    )]
    #[case(
        "ReaderT",
        "try_lift_io",
        EffectType::IO,
        "StateT",
        "try_lift_io",
        EffectType::IO,
        false
    )]
    #[case(
        "ReaderT",
        "try_lift_io",
        EffectType::IO,
        "ReaderT",
        "try_lift_async_io",
        EffectType::IO,
        false
    )]
    #[case(
        "ReaderT",
        "try_lift_io",
        EffectType::IO,
        "ReaderT",
        "try_lift_io",
        EffectType::AsyncIO,
        false
    )]
    #[allow(clippy::too_many_arguments)]
    fn already_consumed_error_equality(
        #[case] transformer_name1: &'static str,
        #[case] method_name1: &'static str,
        #[case] effect_type1: EffectType,
        #[case] transformer_name2: &'static str,
        #[case] method_name2: &'static str,
        #[case] effect_type2: EffectType,
        #[case] expected: bool,
    ) {
        let error1 = AlreadyConsumedError {
            transformer_name: transformer_name1,
            method_name: method_name1,
            effect_type: effect_type1,
        };
        let error2 = AlreadyConsumedError {
            transformer_name: transformer_name2,
            method_name: method_name2,
            effect_type: effect_type2,
        };
        assert_eq!(error1 == error2, expected);
    }

    #[rstest]
    #[case("ReaderT", "try_lift_io", EffectType::IO)]
    #[case("StateT", "try_lift_async_io", EffectType::AsyncIO)]
    fn already_consumed_error_clone(
        #[case] transformer_name: &'static str,
        #[case] method_name: &'static str,
        #[case] effect_type: EffectType,
    ) {
        let error = AlreadyConsumedError {
            transformer_name,
            method_name,
            effect_type,
        };
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    #[rstest]
    #[case("ReaderT", "try_lift_io", EffectType::IO)]
    #[case("StateT", "try_lift_async_io", EffectType::AsyncIO)]
    fn already_consumed_error_debug(
        #[case] transformer_name: &'static str,
        #[case] method_name: &'static str,
        #[case] effect_type: EffectType,
    ) {
        let error = AlreadyConsumedError {
            transformer_name,
            method_name,
            effect_type,
        };
        let debug_string = format!("{error:?}");
        assert!(debug_string.contains("AlreadyConsumedError"));
        assert!(debug_string.contains(transformer_name));
        assert!(debug_string.contains(method_name));
    }

    #[rstest]
    #[case("ReaderT", "try_lift_io", EffectType::IO)]
    #[case("StateT", "try_lift_async_io", EffectType::AsyncIO)]
    fn already_consumed_error_implements_error_trait(
        #[case] transformer_name: &'static str,
        #[case] method_name: &'static str,
        #[case] effect_type: EffectType,
    ) {
        let error = AlreadyConsumedError {
            transformer_name,
            method_name,
            effect_type,
        };
        let _: &dyn Error = &error;
    }

    #[rstest]
    #[case("ReaderT", "try_lift_io", EffectType::IO)]
    #[case("StateT", "try_lift_async_io", EffectType::AsyncIO)]
    fn already_consumed_error_source_is_none(
        #[case] transformer_name: &'static str,
        #[case] method_name: &'static str,
        #[case] effect_type: EffectType,
    ) {
        let error = AlreadyConsumedError {
            transformer_name,
            method_name,
            effect_type,
        };
        assert!(error.source().is_none());
    }

    #[rstest]
    #[case(
        "ReaderT",
        "try_lift_io",
        EffectType::IO,
        "ReaderT::try_lift_io: IO already consumed. Use the transformer only once."
    )]
    #[case(
        "StateT",
        "try_lift_async_io",
        EffectType::AsyncIO,
        "StateT::try_lift_async_io: AsyncIO already consumed. Use the transformer only once."
    )]
    fn effect_error_display(
        #[case] transformer_name: &'static str,
        #[case] method_name: &'static str,
        #[case] effect_type: EffectType,
        #[case] expected: &str,
    ) {
        let error = EffectError::AlreadyConsumed(AlreadyConsumedError {
            transformer_name,
            method_name,
            effect_type,
        });
        assert_eq!(format!("{error}"), expected);
    }

    #[rstest]
    #[case("ReaderT", "try_lift_io", EffectType::IO)]
    #[case("StateT", "try_lift_async_io", EffectType::AsyncIO)]
    fn effect_error_clone(
        #[case] transformer_name: &'static str,
        #[case] method_name: &'static str,
        #[case] effect_type: EffectType,
    ) {
        let error = EffectError::AlreadyConsumed(AlreadyConsumedError {
            transformer_name,
            method_name,
            effect_type,
        });
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    #[rstest]
    #[case("ReaderT", "try_lift_io", EffectType::IO)]
    #[case("StateT", "try_lift_async_io", EffectType::AsyncIO)]
    fn effect_error_debug(
        #[case] transformer_name: &'static str,
        #[case] method_name: &'static str,
        #[case] effect_type: EffectType,
    ) {
        let error = EffectError::AlreadyConsumed(AlreadyConsumedError {
            transformer_name,
            method_name,
            effect_type,
        });
        let debug_string = format!("{error:?}");
        assert!(debug_string.contains("AlreadyConsumed"));
    }

    #[rstest]
    #[case("ReaderT", "try_lift_io", EffectType::IO)]
    #[case("StateT", "try_lift_async_io", EffectType::AsyncIO)]
    fn effect_error_source_is_none(
        #[case] transformer_name: &'static str,
        #[case] method_name: &'static str,
        #[case] effect_type: EffectType,
    ) {
        let error = EffectError::AlreadyConsumed(AlreadyConsumedError {
            transformer_name,
            method_name,
            effect_type,
        });
        assert!(error.source().is_none());
    }
}
