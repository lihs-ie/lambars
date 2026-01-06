//! Effect handlers for interpreting effectful computations.
//!
//! This module provides the `Handler` trait and basic handler implementations.
//! Handlers interpret effect operations and produce results.
//!
//! # Examples
//!
//! ```rust
//! use lambars::effect::algebraic::{Eff, NoEffect, PureHandler, Handler};
//!
//! let computation = Eff::<NoEffect, i32>::pure(42);
//! let result = PureHandler.run(computation);
//! assert_eq!(result, 42);
//! ```

use super::eff::Eff;
use super::eff::EffInner;
use super::effect::{Effect, NoEffect};

/// A handler that interprets effect operations.
///
/// Handlers provide the actual implementation for effect operations.
/// Each handler is associated with a specific effect type and determines
/// how operations of that effect are executed.
///
/// # Type Parameters
///
/// - `E`: The effect type this handler can interpret
///
/// # Laws
///
/// ## Handler Identity Law
///
/// Handling a pure computation returns the value appropriately wrapped:
///
/// ```text
/// handler.run(Eff::pure(a)) == wrap(a)
/// ```
///
/// Where `wrap` lifts the value into the handler's output type.
/// For `PureHandler`, `wrap(a) = a`.
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{Eff, NoEffect, Handler, PureHandler};
///
/// let computation = Eff::<NoEffect, i32>::pure(42);
/// let result = PureHandler.run(computation);
/// assert_eq!(result, 42);
/// ```
pub trait Handler<E: Effect>: Sized {
    /// The output type produced by this handler.
    ///
    /// The type parameter `A` is the result type of the computation.
    /// The output type may wrap or transform this result.
    ///
    /// Examples:
    /// - `PureHandler` for `NoEffect`: `Output<A> = A`
    /// - `StateHandler<S>`: `Output<A> = (A, S)`
    /// - `ErrorHandler<Err>`: `Output<A> = Result<A, Err>`
    type Output<A>;

    /// Runs the handler on a computation.
    ///
    /// This method interprets all effect operations in the computation
    /// and produces the final result.
    ///
    /// # Arguments
    ///
    /// * `computation` - The effectful computation to run
    ///
    /// # Returns
    ///
    /// The result of running the computation with this handler
    fn run<A: 'static>(self, computation: Eff<E, A>) -> Self::Output<A>;
}

/// A handler for pure computations (no effects).
///
/// `PureHandler` can only handle `Eff<NoEffect, A>` computations,
/// which are guaranteed to have no effect operations.
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{Eff, NoEffect, PureHandler, Handler};
///
/// let computation = Eff::<NoEffect, i32>::pure(42)
///     .fmap(|x| x * 2);
///
/// let result = PureHandler.run(computation);
/// assert_eq!(result, 84);
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct PureHandler;

impl PureHandler {
    /// Creates a new `PureHandler`.
    #[must_use]
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}

impl Handler<NoEffect> for PureHandler {
    type Output<A> = A;

    fn run<A: 'static>(self, computation: Eff<NoEffect, A>) -> A {
        // Normalize to handle FlatMap chains
        let normalized = computation.normalize();

        match normalized.inner {
            EffInner::Pure(value) => value,
            EffInner::Impure(_) => {
                panic!("NoEffect computation should not have Impure operations")
            }
            EffInner::FlatMap(_) => {
                unreachable!("FlatMap should be normalized by normalize()")
            }
        }
    }
}

/// A composed handler that applies two handlers in sequence.
///
/// `ComposedHandler` combines two handlers, applying the first handler
/// and then the second. This is useful for handling multiple effects
/// in a single computation.
///
/// # Type Parameters
///
/// - `H1`: The first handler type
/// - `H2`: The second handler type
///
/// # Note
///
/// The composed handler does not implement `Handler` directly because
/// the output types depend on the specific handlers being composed.
/// Instead, use the `run_first` and `run_second` methods to apply
/// handlers in sequence.
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{
///     ComposedHandler, ReaderEffect, ReaderHandler, StateEffect, StateHandler,
/// };
///
/// let composed = ComposedHandler::new(
///     ReaderHandler::new(10),
///     StateHandler::new(0),
/// );
/// ```
#[derive(Debug, Clone)]
pub struct ComposedHandler<H1, H2> {
    first: H1,
    second: H2,
}

impl<H1, H2> ComposedHandler<H1, H2> {
    /// Creates a new composed handler.
    ///
    /// # Arguments
    ///
    /// * `first` - The first handler to apply
    /// * `second` - The second handler to apply
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::algebraic::{
    ///     ComposedHandler, ReaderHandler, StateHandler,
    /// };
    ///
    /// let composed = ComposedHandler::new(
    ///     ReaderHandler::new(42),
    ///     StateHandler::new("initial".to_string()),
    /// );
    /// ```
    #[must_use]
    pub const fn new(first: H1, second: H2) -> Self {
        Self { first, second }
    }

    /// Returns a reference to the first handler.
    #[must_use]
    pub const fn first(&self) -> &H1 {
        &self.first
    }

    /// Returns a reference to the second handler.
    #[must_use]
    pub const fn second(&self) -> &H2 {
        &self.second
    }

    /// Consumes the composed handler and returns both handlers.
    #[must_use]
    pub fn into_parts(self) -> (H1, H2) {
        (self.first, self.second)
    }
}

impl<H1: Default, H2: Default> Default for ComposedHandler<H1, H2> {
    fn default() -> Self {
        Self::new(H1::default(), H2::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn pure_handler_extracts_pure_value() {
        let computation = Eff::<NoEffect, i32>::pure(42);
        let result = PureHandler.run(computation);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn pure_handler_with_string() {
        let computation = Eff::<NoEffect, String>::pure("hello".to_string());
        let result = PureHandler.run(computation);
        assert_eq!(result, "hello");
    }

    #[rstest]
    fn pure_handler_with_complex_type() {
        let computation = Eff::<NoEffect, Vec<i32>>::pure(vec![1, 2, 3]);
        let result = PureHandler.run(computation);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[rstest]
    fn pure_handler_with_fmap() {
        let computation = Eff::<NoEffect, i32>::pure(21).fmap(|x| x * 2);
        let result = PureHandler.run(computation);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn pure_handler_with_multiple_fmap() {
        let computation = Eff::<NoEffect, i32>::pure(10)
            .fmap(|x| x + 5)
            .fmap(|x| x * 2)
            .fmap(|x| x - 10);
        let result = PureHandler.run(computation);
        assert_eq!(result, 20);
    }

    #[rstest]
    fn pure_handler_with_flat_map() {
        let computation = Eff::<NoEffect, i32>::pure(10).flat_map(|x| Eff::pure(x + 5));
        let result = PureHandler.run(computation);
        assert_eq!(result, 15);
    }

    #[rstest]
    fn pure_handler_with_multiple_flat_map() {
        let computation = Eff::<NoEffect, i32>::pure(1)
            .flat_map(|x| Eff::pure(x + 1))
            .flat_map(|x| Eff::pure(x * 2))
            .flat_map(|x| Eff::pure(x + 10));
        let result = PureHandler.run(computation);
        assert_eq!(result, 14);
    }

    #[rstest]
    fn pure_handler_with_and_then() {
        let computation = Eff::<NoEffect, i32>::pure(10).and_then(|x| Eff::pure(x + 5));
        let result = PureHandler.run(computation);
        assert_eq!(result, 15);
    }

    #[rstest]
    fn pure_handler_with_then() {
        let computation = Eff::<NoEffect, i32>::pure(10).then(Eff::pure(42));
        let result = PureHandler.run(computation);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn pure_handler_with_map2() {
        let computation = Eff::<NoEffect, i32>::pure(10)
            .map2(Eff::pure(20), |value_a, value_b| value_a + value_b);
        let result = PureHandler.run(computation);
        assert_eq!(result, 30);
    }

    #[rstest]
    fn pure_handler_with_map2_different_types() {
        let computation = Eff::<NoEffect, i32>::pure(42)
            .map2(Eff::pure("hello"), |number, text| {
                format!("{number}: {text}")
            });
        let result = PureHandler.run(computation);
        assert_eq!(result, "42: hello");
    }

    #[rstest]
    fn pure_handler_with_product() {
        let computation = Eff::<NoEffect, i32>::pure(1).product(Eff::pure(2));
        let result = PureHandler.run(computation);
        assert_eq!(result, (1, 2));
    }

    #[rstest]
    fn pure_handler_with_product_different_types() {
        let computation = Eff::<NoEffect, i32>::pure(42).product(Eff::pure("hello"));
        let result = PureHandler.run(computation);
        assert_eq!(result, (42, "hello"));
    }

    #[rstest]
    fn pure_handler_deep_flat_map_is_stack_safe() {
        let mut computation: Eff<NoEffect, i32> = Eff::pure(0);
        for _ in 0..10000 {
            computation = computation.flat_map(|x| Eff::pure(x + 1));
        }
        let result = PureHandler.run(computation);
        assert_eq!(result, 10000);
    }

    #[rstest]
    fn pure_handler_deep_fmap_is_stack_safe() {
        let mut computation: Eff<NoEffect, i32> = Eff::pure(0);
        for _ in 0..10000 {
            computation = computation.fmap(|x| x + 1);
        }
        let result = PureHandler.run(computation);
        assert_eq!(result, 10000);
    }

    #[rstest]
    fn pure_handler_mixed_operations_is_stack_safe() {
        let mut computation: Eff<NoEffect, i32> = Eff::pure(0);
        for index in 0..5000 {
            if index % 2 == 0 {
                computation = computation.flat_map(|x| Eff::pure(x + 1));
            } else {
                computation = computation.fmap(|x| x + 1);
            }
        }
        let result = PureHandler.run(computation);
        assert_eq!(result, 5000);
    }

    #[rstest]
    fn eff_fmap_method() {
        let eff: Eff<NoEffect, i32> = Eff::pure(10);
        let mapped = eff.fmap(|x| x * 2);
        let result = PureHandler.run(mapped);
        assert_eq!(result, 20);
    }

    #[rstest]
    fn eff_pure_method() {
        let eff: Eff<NoEffect, i32> = Eff::pure(42);
        let result = PureHandler.run(eff);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn eff_map2_method() {
        let eff_a: Eff<NoEffect, i32> = Eff::pure(10);
        let eff_b: Eff<NoEffect, i32> = Eff::pure(20);
        let combined = eff_a.map2(eff_b, |value_a, value_b| value_a + value_b);
        let result = PureHandler.run(combined);
        assert_eq!(result, 30);
    }

    #[rstest]
    fn eff_flat_map_method() {
        let eff: Eff<NoEffect, i32> = Eff::pure(10);
        let chained = eff.flat_map(|x| Eff::pure(x + 5));
        let result = PureHandler.run(chained);
        assert_eq!(result, 15);
    }

    #[rstest]
    fn eff_monad_left_identity() {
        // pure(a).flat_map(f) == f(a)
        let value = 42;
        let function = |x: i32| Eff::<NoEffect, i32>::pure(x * 2);

        let left = Eff::<NoEffect, i32>::pure(value).flat_map(function);
        let right = function(value);

        let left_result = PureHandler.run(left);
        let right_result = PureHandler.run(right);

        assert_eq!(left_result, right_result);
        assert_eq!(left_result, 84);
    }

    #[rstest]
    fn eff_monad_right_identity() {
        // m.flat_map(pure) == m
        let computation = Eff::<NoEffect, i32>::pure(42);
        let result = computation.flat_map(Eff::pure);
        let value = PureHandler.run(result);
        assert_eq!(value, 42);
    }

    #[rstest]
    fn eff_monad_associativity() {
        // m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
        let function1 = |x: i32| Eff::<NoEffect, i32>::pure(x + 1);
        let function2 = |x: i32| Eff::<NoEffect, i32>::pure(x * 2);

        let left = Eff::<NoEffect, i32>::pure(5)
            .flat_map(function1)
            .flat_map(function2);
        let right =
            Eff::<NoEffect, i32>::pure(5).flat_map(move |x| function1(x).flat_map(function2));

        let left_result = PureHandler.run(left);
        let right_result = PureHandler.run(right);

        assert_eq!(left_result, right_result);
        assert_eq!(left_result, 12);
    }

    #[rstest]
    fn pure_handler_is_debug() {
        let handler = PureHandler;
        let debug_string = format!("{handler:?}");
        assert_eq!(debug_string, "PureHandler");
    }

    #[rstest]
    fn pure_handler_is_clone() {
        let handler = PureHandler;
        let cloned = handler;
        let _ = cloned;
    }

    #[rstest]
    fn pure_handler_is_copy() {
        let handler = PureHandler;
        let copied = handler;
        let _ = copied;
    }

    #[rstest]
    #[allow(clippy::default_constructed_unit_structs)]
    fn pure_handler_is_default() {
        let handler = PureHandler::default();
        let computation = Eff::<NoEffect, i32>::pure(42);
        let result = handler.run(computation);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn pure_handler_new_creates_handler() {
        let handler = PureHandler::new();
        let computation = Eff::<NoEffect, i32>::pure(42);
        let result = handler.run(computation);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn composed_handler_new_creates_handler() {
        let composed = ComposedHandler::new(PureHandler, PureHandler);
        assert_eq!(format!("{:?}", composed.first()), "PureHandler");
        assert_eq!(format!("{:?}", composed.second()), "PureHandler");
    }

    #[rstest]
    fn composed_handler_into_parts() {
        let composed = ComposedHandler::new(PureHandler, PureHandler);
        let (first, second) = composed.into_parts();
        let _ = first;
        let _ = second;
    }

    #[rstest]
    fn composed_handler_is_debug() {
        let composed = ComposedHandler::new(PureHandler, PureHandler);
        let debug_string = format!("{composed:?}");
        assert!(debug_string.contains("ComposedHandler"));
    }

    #[rstest]
    fn composed_handler_is_clone() {
        let composed = ComposedHandler::new(PureHandler, PureHandler);
        let cloned = composed;
        assert_eq!(format!("{:?}", cloned.first()), "PureHandler");
    }

    #[rstest]
    #[allow(clippy::default_constructed_unit_structs)]
    fn composed_handler_is_default() {
        let composed: ComposedHandler<PureHandler, PureHandler> = ComposedHandler::default();
        assert_eq!(format!("{:?}", composed.first()), "PureHandler");
    }

    #[rstest]
    fn composed_handler_with_different_handlers() {
        use crate::effect::algebraic::{ReaderHandler, StateHandler};

        let composed = ComposedHandler::new(ReaderHandler::new(42), StateHandler::new(0));

        assert_eq!(*composed.first().environment(), 42);
        assert_eq!(*composed.second().initial_state(), 0);
    }
}
