//! MonadError type class - error handling capability.
//!
//! This module provides the `MonadError` trait which abstracts
//! the ability to throw and catch errors within a monadic context.
//!
//! # Laws
//!
//! All `MonadError` implementations must satisfy these laws:
//!
//! ## Throw Catch Law
//!
//! Catching a thrown error should apply the handler:
//!
//! ```text
//! catch_error(throw_error(e), handler) == handler(e)
//! ```
//!
//! ## Catch Pure Law
//!
//! Catching when there's no error should return the original:
//!
//! ```text
//! catch_error(pure(a), handler) == pure(a)
//! ```
//!
//! ## Throw Short-Circuit Law
//!
//! Throwing an error should short-circuit subsequent computations:
//!
//! ```text
//! throw_error(e).flat_map(f) == throw_error(e)
//! ```
//!
//! # Examples
//!
//! ```rust
//! use lambars::effect::MonadError;
//! use lambars::typeclass::Monad;
//!
//! // Result implements MonadError
//! let result: Result<i32, String> = <Result<i32, String>>::throw_error("error".to_string());
//! assert_eq!(result, Err("error".to_string()));
//!
//! let recovered = <Result<i32, String>>::catch_error(result, |e| Ok(e.len() as i32));
//! assert_eq!(recovered, Ok(5));
//! ```

use crate::typeclass::Monad;

/// A type class for monads that can throw and catch errors.
///
/// `MonadError<E>` extends `Monad` with the ability to handle errors
/// of type `E`. This is the core abstraction for error handling in
/// a functional style.
///
/// # Type Parameters
///
/// - `E`: The error type
///
/// # Laws
///
/// ## Throw Catch Law
///
/// Catching a thrown error should apply the handler:
///
/// ```text
/// catch_error(throw_error(e), handler) == handler(e)
/// ```
///
/// ## Catch Pure Law
///
/// Catching when there's no error should return the original:
///
/// ```text
/// catch_error(pure(a), handler) == pure(a)
/// ```
///
/// ## Throw Short-Circuit Law
///
/// Throwing an error should short-circuit subsequent computations:
///
/// ```text
/// throw_error(e).flat_map(f) == throw_error(e)
/// ```
///
/// # Examples
///
/// ```rust
/// use lambars::effect::MonadError;
/// use lambars::typeclass::Monad;
///
/// fn safe_divide<M: MonadError<String>>(a: i32, b: i32) -> M::WithType<i32>
/// where
///     M::WithType<i32>: From<Result<i32, String>>,
/// {
///     if b == 0 {
///         M::throw_error("division by zero".to_string())
///     } else {
///         M::from_result(Ok(a / b))
///     }
/// }
/// ```
pub trait MonadError<E>: Monad {
    /// Throws an error, short-circuiting the computation.
    ///
    /// This creates a computation that represents a failure with
    /// the given error value. Any subsequent `flat_map` operations
    /// will be skipped.
    ///
    /// # Arguments
    ///
    /// * `error` - The error value to throw
    ///
    /// # Returns
    ///
    /// A computation representing the error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::MonadError;
    ///
    /// let error: Result<i32, String> = <Result<i32, String>>::throw_error("oops".to_string());
    /// assert_eq!(error, Err("oops".to_string()));
    /// ```
    fn throw_error<A>(error: E) -> Self::WithType<A>
    where
        A: 'static;

    /// Catches an error and applies a handler to recover.
    ///
    /// If the computation fails with an error, the handler is applied
    /// to the error to produce a recovery computation. If the computation
    /// succeeds, the handler is not called.
    ///
    /// # Arguments
    ///
    /// * `computation` - The computation that might fail
    /// * `handler` - A function that handles the error
    ///
    /// # Returns
    ///
    /// The original computation if successful, or the result of
    /// applying the handler if it failed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::MonadError;
    ///
    /// let failing: Result<i32, String> = Err("error".to_string());
    /// let recovered = <Result<i32, String>>::catch_error(failing, |e| Ok(e.len() as i32));
    /// assert_eq!(recovered, Ok(5));
    /// ```
    fn catch_error<A, F>(computation: Self::WithType<A>, handler: F) -> Self::WithType<A>
    where
        F: FnOnce(E) -> Self::WithType<A> + 'static,
        A: 'static;

    /// Converts a `Result` into this error-handling monad.
    ///
    /// This is a convenience method for lifting a `Result` into
    /// the monad. `Ok` values become successful computations,
    /// and `Err` values become thrown errors.
    ///
    /// # Arguments
    ///
    /// * `result` - The Result to convert
    ///
    /// # Returns
    ///
    /// A computation representing the Result.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::MonadError;
    ///
    /// let ok: Result<i32, String> = <Result<i32, String>>::from_result(Ok(42));
    /// assert_eq!(ok, Ok(42));
    ///
    /// let err: Result<i32, String> = <Result<i32, String>>::from_result(Err("fail".to_string()));
    /// assert_eq!(err, Err("fail".to_string()));
    /// ```
    fn from_result<A>(result: Result<A, E>) -> Self::WithType<A>
    where
        A: 'static,
        E: 'static;

    /// Returns a default computation if the original fails.
    ///
    /// This is a simpler alternative to `catch_error` when you
    /// just want to provide a fallback value without inspecting
    /// the error.
    ///
    /// Note: This method is named `recover_with` to avoid collision with
    /// the standard library's `Result::or_else` method.
    ///
    /// # Arguments
    ///
    /// * `computation` - The computation that might fail
    /// * `default` - The fallback computation to use on error
    ///
    /// # Returns
    ///
    /// The original computation if successful, or the default if it failed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::MonadError;
    ///
    /// let failing: Result<i32, String> = Err("error".to_string());
    /// let with_default = <Result<i32, String>>::recover_with(failing, Ok(0));
    /// assert_eq!(with_default, Ok(0));
    /// ```
    fn recover_with<A>(
        computation: Self::WithType<A>,
        default: Self::WithType<A>,
    ) -> Self::WithType<A>
    where
        A: 'static;
}

// =============================================================================
// Result<T, E> Implementation
// =============================================================================

impl<T, E: Clone> MonadError<E> for Result<T, E> {
    fn throw_error<A>(error: E) -> Result<A, E>
    where
        A: 'static,
    {
        Err(error)
    }

    fn catch_error<A, F>(computation: Result<A, E>, handler: F) -> Result<A, E>
    where
        F: FnOnce(E) -> Result<A, E> + 'static,
        A: 'static,
    {
        match computation {
            Ok(value) => Ok(value),
            Err(error) => handler(error),
        }
    }

    fn from_result<A>(result: Result<A, E>) -> Result<A, E>
    where
        A: 'static,
        E: 'static,
    {
        result
    }

    fn recover_with<A>(computation: Result<A, E>, default: Result<A, E>) -> Result<A, E>
    where
        A: 'static,
    {
        match computation {
            Ok(value) => Ok(value),
            Err(_) => default,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typeclass::Applicative;
    use rstest::rstest;

    // =========================================================================
    // Trait Definition Tests
    // =========================================================================

    #[rstest]
    fn monad_error_trait_is_defined() {
        // Just verify the trait exists and can be referenced
        fn assert_trait_exists<M: MonadError<String>>() {}
        let _ = assert_trait_exists::<Result<(), String>>;
    }

    #[rstest]
    fn monad_error_requires_monad() {
        // MonadError should require Monad as a supertrait
        fn assert_monad<M: Monad>() {}
        fn assert_monad_error<M: MonadError<String>>() {
            // If M implements MonadError, it must also implement Monad
            assert_monad::<M>();
        }
        let _ = assert_monad_error::<Result<(), String>>;
    }

    // =========================================================================
    // Result Implementation Tests
    // =========================================================================

    #[rstest]
    fn result_throw_error_creates_err() {
        let result: Result<i32, String> =
            <Result<i32, String>>::throw_error("test error".to_string());
        assert_eq!(result, Err("test error".to_string()));
    }

    #[rstest]
    fn result_catch_error_recovers() {
        let failing: Result<i32, String> = Err("error".to_string());
        let recovered = <Result<i32, String>>::catch_error(failing, |e| Ok(e.len() as i32));
        assert_eq!(recovered, Ok(5));
    }

    #[rstest]
    fn result_catch_error_preserves_ok() {
        let success: Result<i32, String> = Ok(42);
        let result = <Result<i32, String>>::catch_error(success, |_| Ok(0));
        assert_eq!(result, Ok(42));
    }

    #[rstest]
    fn result_from_result_ok() {
        let input: Result<i32, String> = Ok(42);
        let result: Result<i32, String> = <Result<i32, String>>::from_result(input);
        assert_eq!(result, Ok(42));
    }

    #[rstest]
    fn result_from_result_err() {
        let input: Result<i32, String> = Err("error".to_string());
        let result: Result<i32, String> = <Result<i32, String>>::from_result(input);
        assert_eq!(result, Err("error".to_string()));
    }

    #[rstest]
    fn result_recover_with_uses_default_on_err() {
        let failing: Result<i32, String> = Err("error".to_string());
        let result = <Result<i32, String>>::recover_with(failing, Ok(0));
        assert_eq!(result, Ok(0));
    }

    #[rstest]
    fn result_recover_with_keeps_ok() {
        let success: Result<i32, String> = Ok(42);
        let result = <Result<i32, String>>::recover_with(success, Ok(0));
        assert_eq!(result, Ok(42));
    }

    // =========================================================================
    // Law Tests
    // =========================================================================

    #[rstest]
    fn result_throw_catch_law() {
        let error = "test".to_string();
        let handler = |e: String| Ok::<i32, String>(e.len() as i32);

        let left: Result<i32, String> = <Result<i32, String>>::catch_error(
            <Result<i32, String>>::throw_error(error.clone()),
            handler,
        );
        let right: Result<i32, String> = handler(error);

        assert_eq!(left, right);
    }

    #[rstest]
    fn result_catch_pure_law() {
        let value = 42;
        let handler = |_: String| Ok::<i32, String>(0);

        let pure_value: Result<i32, String> = <Result<(), String>>::pure(value);
        let left = <Result<i32, String>>::catch_error(pure_value.clone(), handler);

        assert_eq!(left, pure_value);
    }

    #[rstest]
    fn result_throw_short_circuit_law() {
        let error = "error".to_string();

        let thrown: Result<i32, String> = <Result<i32, String>>::throw_error(error.clone());
        let left: Result<String, String> = thrown.flat_map(|n| Ok(format!("got: {}", n)));
        let right: Result<String, String> = <Result<String, String>>::throw_error(error);

        assert_eq!(left, right);
    }
}
