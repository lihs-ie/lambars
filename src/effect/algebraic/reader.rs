//! Reader effect for accessing an environment.
//!
//! This module provides the `ReaderEffect<R>` type that represents computations
//! that can read from a shared environment of type `R`.
//!
//! # Operations
//!
//! - [`ReaderEffect::ask`]: Retrieves the entire environment
//! - [`ReaderEffect::asks`]: Retrieves a projected value from the environment
//! - [`ReaderEffect::local`]: Runs a computation with a modified environment
//!
//! # Examples
//!
//! ```rust
//! use lambars::effect::algebraic::{ReaderEffect, ReaderHandler, Handler};
//!
//! // Define a configuration type
//! #[derive(Clone)]
//! struct Config {
//!     debug_mode: bool,
//!     max_retries: u32,
//! }
//!
//! // Create a computation that reads from the environment
//! let computation = ReaderEffect::<Config>::asks(|config| config.max_retries);
//!
//! // Run with a specific configuration
//! let config = Config { debug_mode: true, max_retries: 3 };
//! let result = ReaderHandler::new(config).run(computation);
//! assert_eq!(result, 3);
//! ```

use super::eff::{Eff, EffInner, OperationTag};
use super::effect::Effect;
use super::handler::Handler;
use std::marker::PhantomData;

mod reader_operations {
    use super::OperationTag;
    pub const ASK: OperationTag = OperationTag::new(1);
}

/// Reader effect: provides access to a shared environment.
///
/// `ReaderEffect<R>` represents the capability to read from an environment
/// of type `R`. The environment is immutable during computation execution.
///
/// # Type Parameters
///
/// - `R`: The type of the environment
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{ReaderEffect, ReaderHandler, Handler};
///
/// let computation = ReaderEffect::<i32>::ask()
///     .fmap(|x| x * 2);
///
/// let result = ReaderHandler::new(21).run(computation);
/// assert_eq!(result, 42);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ReaderEffect<R>(PhantomData<R>);

impl<R: 'static> Effect for ReaderEffect<R> {
    const NAME: &'static str = "Reader";
}

impl<R: Clone + 'static> ReaderEffect<R> {
    /// Retrieves the entire environment.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::algebraic::{ReaderEffect, ReaderHandler, Handler};
    ///
    /// let computation = ReaderEffect::<String>::ask();
    /// let result = ReaderHandler::new("hello".to_string()).run(computation);
    /// assert_eq!(result, "hello");
    /// ```
    #[must_use]
    pub fn ask() -> Eff<Self, R> {
        Eff::<Self, R>::perform_raw::<R>(reader_operations::ASK, ())
    }

    /// Retrieves a projected value from the environment.
    ///
    /// This is a convenience method equivalent to `ask().fmap(projection)`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::algebraic::{ReaderEffect, ReaderHandler, Handler};
    ///
    /// let computation = ReaderEffect::<String>::asks(|s| s.len());
    /// let result = ReaderHandler::new("hello".to_string()).run(computation);
    /// assert_eq!(result, 5);
    /// ```
    pub fn asks<A: 'static, F>(projection: F) -> Eff<Self, A>
    where
        F: FnOnce(R) -> A + 'static,
    {
        Self::ask().fmap(projection)
    }
}

/// Handler for the Reader effect.
///
/// `ReaderHandler<R>` holds the environment value and interprets
/// Reader operations by providing access to it.
///
/// # Type Parameters
///
/// - `R`: The type of the environment
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{ReaderEffect, ReaderHandler, Handler};
///
/// let handler = ReaderHandler::new(42);
/// let computation = ReaderEffect::<i32>::ask();
/// let result = handler.run(computation);
/// assert_eq!(result, 42);
/// ```
#[derive(Debug, Clone)]
pub struct ReaderHandler<R> {
    environment: R,
}

impl<R: Clone + 'static> ReaderHandler<R> {
    /// Creates a new `ReaderHandler` with the given environment.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::algebraic::ReaderHandler;
    ///
    /// let handler = ReaderHandler::new("config".to_string());
    /// ```
    #[must_use]
    pub const fn new(environment: R) -> Self {
        Self { environment }
    }

    /// Returns a reference to the environment.
    #[must_use]
    pub const fn environment(&self) -> &R {
        &self.environment
    }

    /// Runs a computation with a temporarily modified environment.
    ///
    /// The modifier function transforms the environment for the duration
    /// of the inner computation. After the inner computation completes,
    /// the original environment is restored.
    ///
    /// # Arguments
    ///
    /// * `modifier` - A function that transforms the environment
    /// * `computation` - The computation to run with the modified environment
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::algebraic::{ReaderEffect, ReaderHandler, Handler};
    ///
    /// // Modify environment: multiply by 2
    /// let handler = ReaderHandler::new(21);
    /// let computation = ReaderEffect::<i32>::ask();
    /// let result = handler.run_with_local(|x| x * 2, computation);
    /// assert_eq!(result, 42);
    /// ```
    pub fn run_with_local<A: 'static, F>(
        &self,
        modifier: F,
        computation: Eff<ReaderEffect<R>, A>,
    ) -> A
    where
        F: FnOnce(R) -> R,
    {
        let modified_environment = modifier(self.environment.clone());
        Self::run_with_environment(computation, modified_environment)
    }

    /// Runs the computation with a specific environment (internal).
    ///
    /// Uses an iterative approach for stack safety.
    #[inline]
    fn run_with_environment<A: 'static>(computation: Eff<ReaderEffect<R>, A>, environment: R) -> A {
        let mut current_computation = computation;

        loop {
            let normalized = current_computation.normalize();

            match normalized.inner {
                EffInner::Pure(value) => return value,
                EffInner::Impure(operation) => match operation.operation_tag {
                    reader_operations::ASK => {
                        let continuation = operation.continuation;
                        current_computation = continuation(Box::new(environment.clone()));
                    }
                    _ => panic!("Unknown Reader operation: {:?}", operation.operation_tag),
                },
                EffInner::FlatMap(_) => {
                    unreachable!("FlatMap should be normalized by normalize()")
                }
            }
        }
    }
}

impl<R: Clone + 'static> Handler<ReaderEffect<R>> for ReaderHandler<R> {
    type Output<A> = A;

    fn run<A: 'static>(self, computation: Eff<ReaderEffect<R>, A>) -> A {
        Self::run_with_environment(computation, self.environment)
    }
}

/// Runs a computation with a locally modified environment.
///
/// This function provides the `local` operation for Reader effect.
/// It runs the inner computation with a modified environment, then
/// continues with the original environment.
///
/// # Type Parameters
///
/// - `R`: The environment type
/// - `A`: The result type of the inner computation
/// - `B`: The result type of the continuation
/// - `F`: The modifier function type
///
/// # Arguments
///
/// * `modifier` - A function that transforms the environment
/// * `inner` - The computation to run with the modified environment
/// * `continuation` - The computation to run after, receiving the inner result
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{ReaderEffect, ReaderHandler, Handler, Eff, run_local};
///
/// let handler = ReaderHandler::new(10);
///
/// // Run inner computation with doubled environment, then access original
/// let computation = run_local(
///     |x: i32| x * 2,
///     ReaderEffect::<i32>::ask(),
///     |inner_result| ReaderEffect::ask().fmap(move |outer| (inner_result, outer))
/// );
///
/// let result = handler.run(computation);
/// assert_eq!(result, (20, 10)); // inner is 20, outer is 10
/// ```
pub fn run_local<R, A, B, F, G>(
    modifier: F,
    inner: Eff<ReaderEffect<R>, A>,
    continuation: G,
) -> Eff<ReaderEffect<R>, B>
where
    R: Clone + 'static,
    A: 'static,
    B: 'static,
    F: FnOnce(R) -> R + 'static,
    G: FnOnce(A) -> Eff<ReaderEffect<R>, B> + 'static,
{
    ReaderEffect::ask().flat_map(move |environment| {
        let modified_environment = modifier(environment);
        let inner_result = ReaderHandler::run_with_environment(inner, modified_environment);
        continuation(inner_result)
    })
}

#[cfg(test)]
#[allow(
    clippy::no_effect_underscore_binding,
    clippy::redundant_clone,
    clippy::redundant_closure
)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn reader_effect_name_is_reader() {
        assert_eq!(ReaderEffect::<i32>::NAME, "Reader");
    }

    #[rstest]
    fn reader_effect_is_debug() {
        let effect: ReaderEffect<i32> = ReaderEffect(PhantomData);
        let debug_string = format!("{effect:?}");
        assert!(debug_string.contains("ReaderEffect"));
    }

    #[rstest]
    fn reader_effect_is_clone() {
        let effect: ReaderEffect<i32> = ReaderEffect(PhantomData);
        let _cloned = effect;
    }

    #[rstest]
    fn reader_effect_is_copy() {
        let effect: ReaderEffect<i32> = ReaderEffect(PhantomData);
        let _copied = effect;
    }

    #[rstest]
    fn reader_handler_new_creates_handler() {
        let handler = ReaderHandler::new(42);
        assert_eq!(*handler.environment(), 42);
    }

    #[rstest]
    fn reader_handler_is_debug() {
        let handler = ReaderHandler::new(42);
        let debug_string = format!("{handler:?}");
        assert!(debug_string.contains("ReaderHandler"));
        assert!(debug_string.contains("42"));
    }

    #[rstest]
    fn reader_handler_is_clone() {
        let handler = ReaderHandler::new(42);
        let cloned = handler.clone();
        assert_eq!(*cloned.environment(), 42);
    }

    // ask Operation Tests

    #[rstest]
    fn reader_ask_returns_environment() {
        let handler = ReaderHandler::new(42);
        let computation = ReaderEffect::<i32>::ask();
        let result = handler.run(computation);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn reader_ask_with_string() {
        let handler = ReaderHandler::new("hello".to_string());
        let computation = ReaderEffect::<String>::ask();
        let result = handler.run(computation);
        assert_eq!(result, "hello");
    }

    #[rstest]
    fn reader_ask_with_complex_type() {
        #[derive(Clone, Debug, PartialEq)]
        struct Config {
            value: i32,
            name: String,
        }

        let config = Config {
            value: 42,
            name: "test".to_string(),
        };
        let handler = ReaderHandler::new(config.clone());
        let computation = ReaderEffect::<Config>::ask();
        let result = handler.run(computation);
        assert_eq!(result, config);
    }

    // asks Operation Tests

    #[rstest]
    fn reader_asks_projects_environment() {
        let handler = ReaderHandler::new("hello".to_string());
        let computation = ReaderEffect::asks(|s: String| s.len());
        let result = handler.run(computation);
        assert_eq!(result, 5);
    }

    #[rstest]
    fn reader_asks_with_struct_field() {
        #[derive(Clone)]
        struct Config {
            max_retries: u32,
        }

        let handler = ReaderHandler::new(Config { max_retries: 3 });
        let computation = ReaderEffect::asks(|config: Config| config.max_retries);
        let result = handler.run(computation);
        assert_eq!(result, 3);
    }

    #[rstest]
    fn reader_asks_with_complex_projection() {
        let handler = ReaderHandler::new(vec![1, 2, 3, 4, 5]);
        let computation = ReaderEffect::asks(|v: Vec<i32>| v.iter().sum::<i32>());
        let result = handler.run(computation);
        assert_eq!(result, 15);
    }

    // run_with_local Operation Tests

    #[rstest]
    fn reader_run_with_local_modifies_environment() {
        let handler = ReaderHandler::new(10);
        let computation = ReaderEffect::<i32>::ask();
        let result = handler.run_with_local(|x| x * 2, computation);
        assert_eq!(result, 20);
    }

    #[rstest]
    fn reader_run_with_local_with_asks() {
        let handler = ReaderHandler::new("hello".to_string());
        let computation = ReaderEffect::asks(|s: String| s.len());
        let result = handler.run_with_local(|s| format!("{s} world"), computation);
        assert_eq!(result, 11); // "hello world".len()
    }

    // run_local Function Tests

    #[rstest]
    fn run_local_modifies_environment_temporarily() {
        let handler = ReaderHandler::new(10);
        let computation = run_local(
            |x: i32| x * 2,
            ReaderEffect::<i32>::ask(),
            |inner_result| ReaderEffect::ask().fmap(move |outer| (inner_result, outer)),
        );
        let result = handler.run(computation);
        assert_eq!(result, (20, 10));
    }

    #[rstest]
    fn run_local_nested() {
        let handler = ReaderHandler::new(5);
        let computation = run_local(
            |x: i32| x * 2, // 5 -> 10
            run_local(
                |x: i32| x + 3, // 5 -> 8 (note: modifier sees original, not modified)
                ReaderEffect::<i32>::ask(),
                |inner| Eff::pure(inner), // inner = 8
            ),
            |outer_inner| Eff::pure(outer_inner),
        );
        // First run_local modifies 5 to 10 for the outer computation
        // The inner run_local runs with 10, modifies to 13
        // Actually no - run_local gets the current env, modifies it, runs inner
        // So outer run_local: gets 5, modifies to 10, runs inner
        // Inner computation is another run_local which gets 10, modifies to 13
        let result = handler.run(computation);
        // When we run the outer run_local:
        // - It asks for env -> gets 5
        // - Modifies 5 to 10
        // - Runs inner computation with env=10
        // The inner computation is run_local(...) which:
        // - Gets env from handler.run_with_environment(inner, 10)
        // - But run_local starts with ask(), which gets 10
        // - Modifies 10 to 13
        // - Runs innermost ask() with env=13
        // - Returns 13
        assert_eq!(result, 13);
    }

    #[rstest]
    fn reader_operations_can_be_chained() {
        let handler = ReaderHandler::new(10);
        let computation =
            ReaderEffect::<i32>::ask().flat_map(|x| ReaderEffect::asks(move |y: i32| x + y));
        let result = handler.run(computation);
        assert_eq!(result, 20); // 10 + 10
    }

    #[rstest]
    fn reader_multiple_asks_in_chain() {
        let handler = ReaderHandler::new(5);
        let computation = ReaderEffect::<i32>::ask()
            .flat_map(|a| ReaderEffect::ask().fmap(move |b| a + b))
            .flat_map(|sum| ReaderEffect::ask().fmap(move |c| sum + c));
        let result = handler.run(computation);
        assert_eq!(result, 15); // 5 + 5 + 5
    }

    #[rstest]
    fn reader_fmap_transforms_result() {
        let handler = ReaderHandler::new(21);
        let computation = ReaderEffect::<i32>::ask().fmap(|x| x * 2);
        let result = handler.run(computation);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn reader_pure_value_ignores_environment() {
        let handler = ReaderHandler::new(100);
        let computation: Eff<ReaderEffect<i32>, &str> = Eff::pure("constant");
        let result = handler.run(computation);
        assert_eq!(result, "constant");
    }

    #[rstest]
    fn reader_then_sequences_computations() {
        let handler = ReaderHandler::new(42);
        let computation = ReaderEffect::<i32>::ask()
            .fmap(|_| "first")
            .then(ReaderEffect::ask().fmap(|_| "second"));
        let result = handler.run(computation);
        assert_eq!(result, "second");
    }

    #[rstest]
    fn reader_deep_chain_is_stack_safe() {
        let handler = ReaderHandler::new(0);
        let mut computation: Eff<ReaderEffect<i32>, i32> = ReaderEffect::ask();
        for _ in 0..1000 {
            computation = computation.flat_map(|x| ReaderEffect::ask().fmap(move |y| x + y));
        }
        let result = handler.run(computation);
        assert_eq!(result, 0); // 0 added 1000 times
    }

    #[rstest]
    fn reader_deep_fmap_is_stack_safe() {
        let handler = ReaderHandler::new(1);
        let mut computation: Eff<ReaderEffect<i32>, i32> = ReaderEffect::ask();
        for _ in 0..1000 {
            computation = computation.fmap(|x| x + 1);
        }
        let result = handler.run(computation);
        assert_eq!(result, 1001); // 1 + 1000
    }

    #[rstest]
    fn reader_config_based_computation() {
        #[derive(Clone)]
        struct AppConfig {
            multiplier: i32,
            offset: i32,
        }

        let handler = ReaderHandler::new(AppConfig {
            multiplier: 3,
            offset: 10,
        });

        let computation =
            ReaderEffect::asks(|config: AppConfig| config.multiplier).flat_map(|mult| {
                ReaderEffect::asks(move |config: AppConfig| config.offset).fmap(move |off| {
                    // value * multiplier + offset
                    5 * mult + off
                })
            });

        let result = handler.run(computation);
        assert_eq!(result, 25); // 5 * 3 + 10
    }
}
