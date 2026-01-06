//! Effect trait for algebraic effect systems.
//!
//! This module provides the fundamental trait for defining effects.
//! An effect represents a capability that a computation may require,
//! and handlers provide the actual implementation.
//!
//! # Examples
//!
//! ```rust
//! use lambars::effect::algebraic::Effect;
//!
//! struct LogEffect;
//!
//! impl Effect for LogEffect {
//!     const NAME: &'static str = "Log";
//! }
//! ```

use std::any::TypeId;

/// A trait for defining effects.
///
/// Effect is a marker trait that declares what capabilities a computation
/// requires. The actual implementation is provided by handlers.
///
/// Each effect type must have a unique name for debugging and error messages.
///
/// # Type Parameters
///
/// Implementing types must be `'static` to allow runtime type identification.
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::Effect;
///
/// struct CounterEffect;
///
/// impl Effect for CounterEffect {
///     const NAME: &'static str = "Counter";
/// }
///
/// assert_eq!(CounterEffect::NAME, "Counter");
/// ```
pub trait Effect: 'static {
    /// The name of this effect (for debugging and error messages).
    const NAME: &'static str;

    /// Returns the `TypeId` of this effect.
    ///
    /// This can be used to distinguish different effect types at runtime.
    #[must_use]
    #[inline]
    fn type_id() -> TypeId {
        TypeId::of::<Self>()
    }
}

/// A marker type representing computations with no effects.
///
/// `NoEffect` is used for pure computations that do not require any
/// capabilities from the environment.
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::NoEffect;
/// use lambars::effect::algebraic::Effect;
///
/// assert_eq!(NoEffect::NAME, "NoEffect");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoEffect;

impl Effect for NoEffect {
    const NAME: &'static str = "NoEffect";
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    struct TestEffect;

    impl Effect for TestEffect {
        const NAME: &'static str = "TestEffect";
    }

    struct AnotherEffect;

    impl Effect for AnotherEffect {
        const NAME: &'static str = "AnotherEffect";
    }

    #[rstest]
    fn effect_name_is_accessible() {
        assert_eq!(TestEffect::NAME, "TestEffect");
    }

    #[rstest]
    fn effect_type_id_is_unique() {
        let type_id_test = TestEffect::type_id();
        let type_id_another = AnotherEffect::type_id();
        assert_ne!(type_id_test, type_id_another);
    }

    #[rstest]
    fn effect_type_id_is_consistent() {
        let type_id_first = TestEffect::type_id();
        let type_id_second = TestEffect::type_id();
        assert_eq!(type_id_first, type_id_second);
    }

    #[rstest]
    fn no_effect_name() {
        assert_eq!(NoEffect::NAME, "NoEffect");
    }

    #[rstest]
    fn no_effect_type_id_differs_from_other_effects() {
        let type_id_no_effect = NoEffect::type_id();
        let type_id_test = TestEffect::type_id();
        assert_ne!(type_id_no_effect, type_id_test);
    }

    #[rstest]
    fn no_effect_is_debug() {
        let no_effect = NoEffect;
        let debug_string = format!("{no_effect:?}");
        assert_eq!(debug_string, "NoEffect");
    }

    #[rstest]
    fn no_effect_is_clone() {
        let no_effect = NoEffect;
        let cloned = no_effect;
        assert_eq!(no_effect, cloned);
    }

    #[rstest]
    fn no_effect_is_copy() {
        let no_effect = NoEffect;
        let copied = no_effect;
        assert_eq!(no_effect, copied);
    }

    #[rstest]
    fn no_effect_is_eq() {
        let first = NoEffect;
        let second = NoEffect;
        assert_eq!(first, second);
    }

    #[rstest]
    fn effect_with_generic_parameter() {
        struct GenericEffect<T>(std::marker::PhantomData<T>);

        impl<T: 'static> Effect for GenericEffect<T> {
            const NAME: &'static str = "GenericEffect";
        }

        let type_id_i32 = GenericEffect::<i32>::type_id();
        let type_id_string = GenericEffect::<String>::type_id();
        assert_ne!(type_id_i32, type_id_string);
    }
}
