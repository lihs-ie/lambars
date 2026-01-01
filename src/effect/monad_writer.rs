//! `MonadWriter` type class - log output capability.
//!
//! This module provides the `MonadWriter` trait which abstracts
//! the ability to accumulate output alongside a computation.
//! This is the core abstraction of the Writer monad pattern.
//!
//! # Laws
//!
//! All `MonadWriter` implementations must satisfy these laws:
//!
//! ## Tell Monoid Law
//!
//! Consecutive tells should be equivalent to telling the combined output:
//!
//! ```text
//! tell(w1).then(tell(w2)) == tell(w1.combine(w2))
//! ```
//!
//! ## Listen Tell Law
//!
//! Listening to a tell should return the output along with the result:
//!
//! ```text
//! listen(tell(w)) == tell(w).map(|_| ((), w))
//! ```
//!
//! ## Pass Identity Law
//!
//! Passing an identity function should not change the computation:
//!
//! ```text
//! pass(m.map(|a| (a, |w| w))) == m
//! ```
//!
//! ## Censor Definition
//!
//! Censor is defined in terms of pass:
//!
//! ```text
//! censor(f, m) == pass(m.map(|a| (a, f)))
//! ```
//!
//! # Examples
//!
//! ```rust,ignore
//! use lambars::effect::MonadWriter;
//!
//! // Writer implementation will provide concrete examples
//! ```

use crate::typeclass::{Monad, Monoid};

/// A type class for monads that can accumulate output.
///
/// `MonadWriter<W>` extends `Monad` with the ability to produce
/// output of type `W` alongside the main computation. The output
/// type must be a `Monoid` to support combining outputs from
/// sequential computations.
///
/// This is useful for logging, metrics collection, and other
/// patterns where computations need to produce auxiliary output.
///
/// # Type Parameters
///
/// - `W`: The output type (must be a Monoid)
///
/// # Laws
///
/// ## Tell Monoid Law
///
/// Consecutive tells should combine outputs:
///
/// ```text
/// tell(w1).then(tell(w2)) == tell(w1.combine(w2))
/// ```
///
/// ## Listen Tell Law
///
/// Listening captures the output:
///
/// ```text
/// listen(tell(w)) == tell(w).map(|_| ((), w))
/// ```
///
/// ## Pass Identity Law
///
/// Passing identity preserves the computation:
///
/// ```text
/// pass(m.map(|a| (a, |w| w))) == m
/// ```
///
/// ## Censor Definition
///
/// Censor is a specialized pass:
///
/// ```text
/// censor(f, m) == pass(m.map(|a| (a, f)))
/// ```
///
/// # Examples
///
/// ```rust,ignore
/// use lambars::effect::{MonadWriter, Writer};
///
/// let writer = Writer::tell(vec!["step 1".to_string()])
///     .flat_map(|_| Writer::tell(vec!["step 2".to_string()]))
///     .map(|_| 42);
///
/// let (result, output) = writer.run();
/// assert_eq!(result, 42);
/// assert_eq!(output, vec!["step 1", "step 2"]);
/// ```
pub trait MonadWriter<W>: Monad
where
    W: Monoid,
{
    /// Appends output to the accumulated output.
    ///
    /// This is the fundamental write operation. It produces a
    /// computation that adds the given output to the accumulated
    /// output and returns unit.
    ///
    /// # Arguments
    ///
    /// * `output` - The output to append
    ///
    /// # Returns
    ///
    /// A computation that appends the output and produces unit.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{MonadWriter, Writer};
    ///
    /// let writer: Writer<Vec<String>, ()> =
    ///     Writer::tell(vec!["log message".to_string()]);
    /// let (_, output) = writer.run();
    /// assert_eq!(output, vec!["log message"]);
    /// ```
    fn tell(output: W) -> Self::WithType<()>;

    /// Executes a computation and also returns its output.
    ///
    /// This allows inspecting the output that a computation produces.
    /// The output is still accumulated, but also returned as part
    /// of the result.
    ///
    /// # Arguments
    ///
    /// * `computation` - The computation whose output to capture
    ///
    /// # Returns
    ///
    /// A computation that produces a tuple of the original result
    /// and the captured output.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{MonadWriter, Writer};
    ///
    /// let writer = Writer::tell(vec!["log".to_string()]).map(|_| 42);
    /// let listened = Writer::listen(writer);
    /// let ((result, captured_output), total_output) = listened.run();
    /// assert_eq!(result, 42);
    /// assert_eq!(captured_output, vec!["log"]);
    /// assert_eq!(total_output, vec!["log"]);
    /// ```
    fn listen<A>(computation: Self::WithType<A>) -> Self::WithType<(A, W)>
    where
        A: 'static;

    /// Executes a computation with a function that modifies the output.
    ///
    /// The computation produces a tuple of a result and a function.
    /// The function is applied to transform the accumulated output.
    ///
    /// # Arguments
    ///
    /// * `computation` - A computation that produces (result, `output_modifier`)
    ///
    /// # Returns
    ///
    /// A computation that applies the modifier to the output.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{MonadWriter, Writer};
    ///
    /// let computation = Writer::new(
    ///     (42, |output: Vec<String>| output.into_iter().map(|s| s.to_uppercase()).collect()),
    ///     vec!["hello".to_string()]
    /// );
    /// let passed = Writer::pass(computation);
    /// let (result, output) = passed.run();
    /// assert_eq!(result, 42);
    /// assert_eq!(output, vec!["HELLO"]);
    /// ```
    fn pass<A, F>(computation: Self::WithType<(A, F)>) -> Self::WithType<A>
    where
        F: FnOnce(W) -> W + 'static,
        A: 'static;

    /// Modifies the output of a computation.
    ///
    /// This is a convenience function that applies a modifier function
    /// to transform the accumulated output of a computation.
    /// It is equivalent to `pass(computation.map(|a| (a, modifier)))`.
    ///
    /// # Arguments
    ///
    /// * `modifier` - A function that transforms the output
    /// * `computation` - The computation whose output to modify
    ///
    /// # Returns
    ///
    /// A computation with modified output.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{MonadWriter, Writer};
    ///
    /// let writer = Writer::tell(vec!["hello".to_string()]).map(|_| 42);
    /// let censored = Writer::censor(
    ///     |output| output.into_iter().map(|s| s.to_uppercase()).collect(),
    ///     writer
    /// );
    /// let (result, output) = censored.run();
    /// assert_eq!(result, 42);
    /// assert_eq!(output, vec!["HELLO"]);
    /// ```
    fn censor<A, F>(modifier: F, computation: Self::WithType<A>) -> Self::WithType<A>
    where
        F: FnOnce(W) -> W + Clone + 'static,
        A: 'static;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::marker::PhantomData;

    // =========================================================================
    // Trait Definition Tests
    // =========================================================================

    #[test]
    fn monad_writer_trait_is_defined() {
        // Just verify the trait exists and can be referenced
        // The function is not called, but the compiler verifies the trait bounds
        fn assert_trait_exists<M: MonadWriter<Vec<String>>>() {
            let _ = PhantomData::<M>;
        }
        // Verify the function compiles - we don't call it since no types implement MonadWriter yet
        let _ = PhantomData::<fn()>;
        fn _type_check() {
            fn _inner<M: MonadWriter<Vec<String>>>() {
                assert_trait_exists::<M>();
            }
        }
    }

    #[test]
    fn monad_writer_requires_monad() {
        // MonadWriter should require Monad as a supertrait
        // This is verified by the trait definition itself
        fn assert_monad<M: Monad>() {
            let _ = PhantomData::<M>;
        }
        fn assert_monad_writer<M: MonadWriter<Vec<String>>>() {
            // If M implements MonadWriter, it must also implement Monad
            assert_monad::<M>();
        }
        // Verify the function compiles - we don't call it
        let _ = PhantomData::<fn()>;
        fn _type_check() {
            fn _inner<M: MonadWriter<Vec<String>>>() {
                assert_monad_writer::<M>();
            }
        }
    }

    #[test]
    fn monad_writer_requires_monoid_for_output() {
        // The output type W must be a Monoid
        fn assert_monoid<W: Monoid>() {
            let _ = PhantomData::<W>;
        }
        fn assert_monad_writer<W: Monoid, M: MonadWriter<W>>() {
            assert_monoid::<W>();
        }
        // Verify the function compiles - we don't call it
        let _ = PhantomData::<fn()>;
        fn _type_check() {
            fn _inner<W: Monoid, M: MonadWriter<W>>() {
                assert_monad_writer::<W, M>();
            }
        }
    }
}
