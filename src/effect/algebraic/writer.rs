//! Writer effect for accumulating output.
//!
//! This module provides the `WriterEffect<W>` type that represents computations
//! that can produce output (logs) of type `W` where `W` is a `Monoid`.
//!
//! # Operations
//!
//! - [`WriterEffect::tell`]: Appends output to the log
//! - [`WriterEffect::listen`]: Captures the log produced by a computation
//!
//! # Examples
//!
//! ```rust
//! use lambars::effect::algebraic::{WriterEffect, WriterHandler, Handler, Eff};
//!
//! // Logging example with String
//! let computation = WriterEffect::tell("Hello".to_string())
//!     .then(WriterEffect::tell(" World".to_string()))
//!     .then(Eff::pure(42));
//!
//! let (result, log) = WriterHandler::new().run(computation);
//! assert_eq!(result, 42);
//! assert_eq!(log, "Hello World");
//! ```

use super::eff::{Eff, EffInner, OperationTag};
use super::effect::Effect;
use super::handler::Handler;
use crate::typeclass::Monoid;
use std::cell::RefCell;
use std::marker::PhantomData;

mod writer_operations {
    use super::OperationTag;
    pub const TELL: OperationTag = OperationTag::new(20);
}

/// Writer effect: provides the capability to produce output.
///
/// `WriterEffect<W>` represents the capability to append output of type `W`
/// to a log. The output type must be a `Monoid` so that outputs can be
/// combined and an empty log can be created.
///
/// # Type Parameters
///
/// - `W`: The type of the output (must implement `Monoid`)
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{WriterEffect, WriterHandler, Handler, Eff};
///
/// let computation = WriterEffect::tell(vec!["step1".to_string()])
///     .then(WriterEffect::tell(vec!["step2".to_string()]))
///     .then(Eff::pure("done"));
///
/// let (result, log) = WriterHandler::new().run(computation);
/// assert_eq!(result, "done");
/// assert_eq!(log, vec!["step1".to_string(), "step2".to_string()]);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct WriterEffect<W>(PhantomData<W>);

impl<W: Monoid + 'static> Effect for WriterEffect<W> {
    const NAME: &'static str = "Writer";
}

impl<W: Monoid + Clone + Send + Sync + 'static> WriterEffect<W> {
    /// Appends output to the log.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::algebraic::{WriterEffect, WriterHandler, Handler};
    ///
    /// let computation = WriterEffect::tell("message".to_string());
    /// let ((), log) = WriterHandler::new().run(computation);
    /// assert_eq!(log, "message");
    /// ```
    pub fn tell(output: W) -> Eff<Self, ()> {
        Eff::<Self, ()>::perform_raw::<()>(writer_operations::TELL, output)
    }
}

/// Handler for the Writer effect.
///
/// `WriterHandler<W>` accumulates output as computations execute.
/// The handler returns a tuple `(A, W)` containing the computation result
/// and the accumulated log.
///
/// # Type Parameters
///
/// - `W`: The type of the output (must implement `Monoid`)
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{WriterEffect, WriterHandler, Handler, Eff};
///
/// let handler = WriterHandler::<String>::new();
/// let computation = WriterEffect::tell("a".to_string())
///     .then(WriterEffect::tell("b".to_string()))
///     .then(WriterEffect::tell("c".to_string()))
///     .then(Eff::pure(42));
///
/// let (result, log) = handler.run(computation);
/// assert_eq!(result, 42);
/// assert_eq!(log, "abc");
/// ```
#[derive(Debug, Clone, Default)]
pub struct WriterHandler<W>(PhantomData<W>);

impl<W: Monoid + Clone + 'static> WriterHandler<W> {
    /// Creates a new `WriterHandler`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::algebraic::WriterHandler;
    ///
    /// let handler = WriterHandler::<String>::new();
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }

    /// Runs the computation with a mutable buffer.
    ///
    /// Uses `Vec<W>` to accumulate outputs, achieving O(n) time complexity
    /// instead of O(n^2) with the naive approach.
    fn run_with_buffer<A: 'static>(
        computation: Eff<WriterEffect<W>, A>,
        buffer: &RefCell<Vec<W>>,
    ) -> A {
        let mut current_computation = computation;

        loop {
            let normalized = current_computation.normalize();

            match normalized.inner {
                EffInner::Pure(value) => return value,
                EffInner::Impure(operation) => match operation.operation_tag {
                    writer_operations::TELL => {
                        let output = *operation
                            .arguments
                            .downcast::<W>()
                            .expect("Type mismatch in Writer::tell");
                        buffer.borrow_mut().push(output);
                        current_computation = (operation.continuation)(Box::new(()));
                    }
                    _ => panic!("Unknown Writer operation: {:?}", operation.operation_tag),
                },
                EffInner::FlatMap(_) => {
                    unreachable!("FlatMap should be normalized by normalize()")
                }
            }
        }
    }
}

impl<W: Monoid + Clone + 'static> Handler<WriterEffect<W>> for WriterHandler<W> {
    type Output<A> = (A, W);

    fn run<A: 'static>(self, computation: Eff<WriterEffect<W>, A>) -> (A, W) {
        let buffer = RefCell::new(Vec::new());
        let result = Self::run_with_buffer(computation, &buffer);
        (result, W::combine_all(buffer.into_inner()))
    }
}

/// Runs a computation and captures its log separately.
///
/// This function provides the `listen` operation for Writer effect.
/// It runs the inner computation, captures its log, and returns both
/// the result and the captured log. The log is also accumulated in
/// the outer context.
///
/// # Type Parameters
///
/// - `W`: The log type (must implement `Monoid`)
/// - `A`: The result type of the computation
///
/// # Arguments
///
/// * `computation` - The computation whose log should be captured
///
/// # Returns
///
/// A computation that returns both the result and the captured log.
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{WriterEffect, WriterHandler, Handler, Eff, listen};
///
/// let handler = WriterHandler::<String>::new();
///
/// let computation = listen(
///     WriterEffect::tell("captured".to_string())
///         .then(Eff::pure(42))
/// );
///
/// let ((result, captured_log), total_log) = handler.run(computation);
/// assert_eq!(result, 42);
/// assert_eq!(captured_log, "captured");
/// assert_eq!(total_log, "captured"); // Captured log is also in total
/// ```
pub fn listen<W, A>(computation: Eff<WriterEffect<W>, A>) -> Eff<WriterEffect<W>, (A, W)>
where
    W: Monoid + Clone + Send + Sync + 'static,
    A: 'static,
{
    // First capture the current state, run the computation, capture the delta
    // This is done by running in an isolated handler and telling the result
    // We use a trick: run the inner computation with a fresh handler,
    // then tell the captured log and return both result and log

    // Create a fresh handler to run the inner computation
    let inner_handler = WriterHandler::<W>::new();
    let (inner_result, inner_log) = inner_handler.run(computation);

    // Tell the inner log to the outer context and return the captured data
    WriterEffect::tell(inner_log.clone()).fmap(move |()| (inner_result, inner_log))
}

#[cfg(test)]
#[allow(
    clippy::no_effect_underscore_binding,
    clippy::redundant_clone,
    clippy::items_after_statements,
    clippy::ignored_unit_patterns
)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn writer_effect_name_is_writer() {
        assert_eq!(WriterEffect::<String>::NAME, "Writer");
    }

    #[rstest]
    fn writer_effect_is_debug() {
        let effect: WriterEffect<String> = WriterEffect(PhantomData);
        let debug_string = format!("{effect:?}");
        assert!(debug_string.contains("WriterEffect"));
    }

    #[rstest]
    fn writer_effect_is_clone() {
        let effect: WriterEffect<String> = WriterEffect(PhantomData);
        let _cloned = effect;
    }

    #[rstest]
    fn writer_effect_is_copy() {
        let effect: WriterEffect<String> = WriterEffect(PhantomData);
        let _copied = effect;
    }

    #[rstest]
    fn writer_handler_new_creates_handler() {
        let _handler = WriterHandler::<String>::new();
    }

    #[rstest]
    fn writer_handler_is_debug() {
        let handler = WriterHandler::<String>::new();
        let debug_string = format!("{handler:?}");
        assert!(debug_string.contains("WriterHandler"));
    }

    #[rstest]
    fn writer_handler_is_clone() {
        let handler = WriterHandler::<String>::new();
        let _cloned = handler.clone();
    }

    #[rstest]
    #[allow(clippy::default_constructed_unit_structs)]
    fn writer_handler_is_default() {
        let _handler = WriterHandler::<String>::default();
    }

    // tell Operation Tests

    #[rstest]
    fn writer_tell_appends_to_log() {
        let handler = WriterHandler::<String>::new();
        let computation = WriterEffect::tell("hello".to_string());
        let ((), log) = handler.run(computation);
        assert_eq!(log, "hello");
    }

    #[rstest]
    fn writer_tell_multiple_times() {
        let handler = WriterHandler::<String>::new();
        let computation = WriterEffect::tell("a".to_string())
            .then(WriterEffect::tell("b".to_string()))
            .then(WriterEffect::tell("c".to_string()));
        let ((), log) = handler.run(computation);
        assert_eq!(log, "abc");
    }

    #[rstest]
    fn writer_tell_with_vec() {
        let handler = WriterHandler::<Vec<String>>::new();
        let computation = WriterEffect::tell(vec!["step1".to_string()])
            .then(WriterEffect::tell(vec!["step2".to_string()]))
            .then(WriterEffect::tell(vec!["step3".to_string()]));
        let ((), log) = handler.run(computation);
        assert_eq!(
            log,
            vec![
                "step1".to_string(),
                "step2".to_string(),
                "step3".to_string()
            ]
        );
    }

    #[rstest]
    fn writer_tell_empty() {
        let handler = WriterHandler::<String>::new();
        let computation = WriterEffect::tell(String::new());
        let ((), log) = handler.run(computation);
        assert_eq!(log, "");
    }

    // listen Operation Tests

    #[rstest]
    fn writer_listen_captures_inner_log() {
        let handler = WriterHandler::<String>::new();
        let computation = listen(WriterEffect::tell("inner".to_string()).then(Eff::pure(42)));
        let ((result, inner_log), total_log) = handler.run(computation);
        assert_eq!(result, 42);
        assert_eq!(inner_log, "inner");
        assert_eq!(total_log, "inner");
    }

    #[rstest]
    fn writer_listen_with_multiple_tells() {
        let handler = WriterHandler::<String>::new();
        let computation = listen(
            WriterEffect::tell("a".to_string())
                .then(WriterEffect::tell("b".to_string()))
                .then(Eff::pure(100)),
        );
        let ((result, inner_log), total_log) = handler.run(computation);
        assert_eq!(result, 100);
        assert_eq!(inner_log, "ab");
        assert_eq!(total_log, "ab");
    }

    #[rstest]
    fn writer_listen_pure_has_empty_log() {
        let handler = WriterHandler::<String>::new();
        let computation = listen(Eff::<WriterEffect<String>, i32>::pure(42));
        let ((result, inner_log), total_log) = handler.run(computation);
        assert_eq!(result, 42);
        assert_eq!(inner_log, "");
        assert_eq!(total_log, "");
    }

    #[rstest]
    fn writer_listen_followed_by_tell() {
        let handler = WriterHandler::<String>::new();
        let computation = listen(WriterEffect::tell("inner".to_string()).then(Eff::pure(42)))
            .flat_map(|(result, inner_log)| {
                WriterEffect::tell(" outer".to_string()).fmap(move |_| (result, inner_log))
            });
        let ((result, inner_log), total_log) = handler.run(computation);
        assert_eq!(result, 42);
        assert_eq!(inner_log, "inner");
        assert_eq!(total_log, "inner outer");
    }

    #[rstest]
    fn writer_tell_then_pure() {
        let handler = WriterHandler::<String>::new();
        let computation = WriterEffect::tell("log".to_string()).then(Eff::pure(42));
        let (result, log) = handler.run(computation);
        assert_eq!(result, 42);
        assert_eq!(log, "log");
    }

    #[rstest]
    fn writer_pure_value_has_empty_log() {
        let handler = WriterHandler::<String>::new();
        let computation: Eff<WriterEffect<String>, i32> = Eff::pure(42);
        let (result, log) = handler.run(computation);
        assert_eq!(result, 42);
        assert_eq!(log, "");
    }

    #[rstest]
    fn writer_fmap_does_not_affect_log() {
        let handler = WriterHandler::<String>::new();
        let computation = WriterEffect::tell("log".to_string())
            .then(Eff::pure(21))
            .fmap(|x| x * 2);
        let (result, log) = handler.run(computation);
        assert_eq!(result, 42);
        assert_eq!(log, "log");
    }

    #[rstest]
    fn writer_logging_pattern() {
        let handler = WriterHandler::<Vec<String>>::new();

        fn log_step(step: &str) -> Eff<WriterEffect<Vec<String>>, ()> {
            WriterEffect::tell(vec![step.to_string()])
        }

        let computation = log_step("Starting computation")
            .then(Eff::pure(10))
            .flat_map(|x| log_step("Processing").fmap(move |_| x * 2))
            .flat_map(|x| log_step("Finishing").fmap(move |_| x));

        let (result, log) = handler.run(computation);
        assert_eq!(result, 20);
        assert_eq!(
            log,
            vec![
                "Starting computation".to_string(),
                "Processing".to_string(),
                "Finishing".to_string()
            ]
        );
    }

    #[rstest]
    fn writer_deep_chain_is_stack_safe() {
        let handler = WriterHandler::<Vec<i32>>::new();
        let mut computation: Eff<WriterEffect<Vec<i32>>, ()> = Eff::pure(());
        for index in 0..1000 {
            let index_copy = index;
            computation = computation.then(WriterEffect::tell(vec![index_copy]));
        }
        let ((), log) = handler.run(computation);
        assert_eq!(log.len(), 1000);
        assert_eq!(log[0], 0);
        assert_eq!(log[999], 999);
    }

    #[rstest]
    fn writer_deep_flat_map_is_stack_safe() {
        let handler = WriterHandler::<Vec<i32>>::new();
        let mut computation: Eff<WriterEffect<Vec<i32>>, i32> = Eff::pure(0);
        for _ in 0..1000 {
            computation =
                computation.flat_map(|x| WriterEffect::tell(vec![x]).fmap(move |_| x + 1));
        }
        let (result, log) = handler.run(computation);
        assert_eq!(result, 1000);
        assert_eq!(log.len(), 1000);
    }

    #[rstest]
    fn writer_empty_tell_is_identity() {
        let handler = WriterHandler::<String>::new();
        let computation =
            WriterEffect::tell(String::empty()).then(WriterEffect::tell("test".to_string()));
        let ((), log) = handler.run(computation);
        assert_eq!(log, "test");
    }

    #[rstest]
    fn writer_tell_order_matters() {
        let handler = WriterHandler::<String>::new();
        let computation1 =
            WriterEffect::tell("a".to_string()).then(WriterEffect::tell("b".to_string()));
        let computation2 =
            WriterEffect::tell("b".to_string()).then(WriterEffect::tell("a".to_string()));
        let ((), log1) = handler.clone().run(computation1);
        let ((), log2) = handler.run(computation2);
        assert_eq!(log1, "ab");
        assert_eq!(log2, "ba");
        assert_ne!(log1, log2);
    }

    #[rstest]
    fn writer_tell_1000_times_preserves_order() {
        let handler = WriterHandler::<Vec<i32>>::new();
        let computation = (0..1000).fold(Eff::pure(()), |computation, index| {
            computation.then(WriterEffect::tell(vec![index]))
        });
        let ((), log) = handler.run(computation);

        let expected: Vec<i32> = (0..1000).collect();
        assert_eq!(log, expected);
    }

    #[rstest]
    fn writer_tell_order_preserved_with_string() {
        let handler = WriterHandler::<String>::new();
        let computation = WriterEffect::tell("first".to_string())
            .then(WriterEffect::tell("-".to_string()))
            .then(WriterEffect::tell("second".to_string()))
            .then(WriterEffect::tell("-".to_string()))
            .then(WriterEffect::tell("third".to_string()));
        let ((), log) = handler.run(computation);
        assert_eq!(log, "first-second-third");
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Verifies that tell operations preserve order, which is essential
        /// for Monoid law compliance (associativity).
        #[test]
        fn prop_tell_order_preserved(tells in prop::collection::vec(any::<i32>(), 0..100)) {
            let handler = WriterHandler::<Vec<i32>>::new();
            let computation = tells.iter().fold(
                Eff::pure(()),
                |computation, &value| computation.then(WriterEffect::tell(vec![value]))
            );
            let ((), log) = handler.run(computation);
            prop_assert_eq!(log, tells);
        }

        /// Verifies that Vec buffer + combine_all produces the same result
        /// as sequential combine.
        #[test]
        fn prop_result_equivalence_with_string(
            parts in prop::collection::vec("[a-z]{1,5}", 0..50)
        ) {
            let handler = WriterHandler::<String>::new();
            let computation = parts.iter().fold(
                Eff::pure(()),
                |computation, part| computation.then(WriterEffect::tell(part.clone()))
            );
            let ((), log) = handler.run(computation);

            let expected = String::combine_all(parts);
            prop_assert_eq!(log, expected);
        }
    }
}
