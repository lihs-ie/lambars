//! `MonadError` type class - error handling capability.
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

// =============================================================================
// MonadErrorExt Trait - Extension for error type transformation
// =============================================================================

/// Extension trait for error type transformation.
///
/// This trait provides `map_error` which transforms the error type of a
/// monadic computation. It is provided as a separate trait because the
/// return type changes (the error type is different).
///
/// # Laws
///
/// ## Identity Law
///
/// Mapping with the identity function returns the original computation:
///
/// ```text
/// computation.map_error(|e| e) == computation
/// ```
///
/// ## Composition Law
///
/// Two consecutive `map_error` calls are equivalent to composing the functions:
///
/// ```text
/// computation.map_error(f).map_error(g) == computation.map_error(|e| g(f(e)))
/// ```
///
/// ## Success Preservation Law
///
/// Success values are not affected by `map_error`:
///
/// ```text
/// pure(a).map_error(f) == pure(a)
/// ```
///
/// # Examples
///
/// ```rust
/// use lambars::effect::MonadErrorExt;
///
/// let computation: Result<i32, i32> = Err(404);
/// let mapped: Result<i32, String> = computation.map_error(|code| {
///     format!("HTTP Error: {}", code)
/// });
/// assert_eq!(mapped, Err("HTTP Error: 404".to_string()));
/// ```
pub trait MonadErrorExt<E> {
    /// The success value type.
    type Value;

    /// Transforms the error type using the provided function.
    ///
    /// This operation transforms errors of type `E` into errors of type `E2`.
    /// Success values are not affected.
    ///
    /// # Arguments
    ///
    /// * `transform` - A function to transform the error. Must be a pure function
    ///   (no side effects, same output for same input).
    ///
    /// # Returns
    ///
    /// A computation with the transformed error type.
    ///
    /// # Errors
    ///
    /// Returns `Err(E2)` if the original computation was an error, with the
    /// error transformed by the provided function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::MonadErrorExt;
    ///
    /// // Transform error type from i32 to String
    /// let computation: Result<i32, i32> = Err(404);
    /// let mapped: Result<i32, String> = computation.map_error(|code| {
    ///     format!("HTTP Error: {}", code)
    /// });
    /// assert_eq!(mapped, Err("HTTP Error: 404".to_string()));
    ///
    /// // Success values are preserved
    /// let success: Result<i32, i32> = Ok(42);
    /// let mapped: Result<i32, String> = success.map_error(|code| {
    ///     format!("HTTP Error: {}", code)
    /// });
    /// assert_eq!(mapped, Ok(42));
    /// ```
    fn map_error<E2, F>(self, transform: F) -> Result<Self::Value, E2>
    where
        F: FnOnce(E) -> E2;
}

impl<A, E> MonadErrorExt<E> for Result<A, E> {
    type Value = A;

    fn map_error<E2, F>(self, transform: F) -> Result<A, E2>
    where
        F: FnOnce(E) -> E2,
    {
        self.map_err(transform)
    }
}

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

    /// Transforms an error within the same error type.
    ///
    /// This is similar to `map_error` but keeps the error type the same.
    /// Useful for adding context information to errors.
    ///
    /// # Arguments
    ///
    /// * `computation` - The computation that might fail
    /// * `transform` - A function to transform the error. Must be a pure function
    ///   (no side effects, same output for same input).
    ///
    /// # Returns
    ///
    /// The computation with the transformed error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::MonadError;
    ///
    /// fn read_config() -> Result<String, String> {
    ///     Err("file not found".to_string())
    /// }
    ///
    /// let result = <Result<String, String>>::adapt_error(
    ///     read_config(),
    ///     |error| format!("failed to read config: {}", error)
    /// );
    /// assert_eq!(result, Err("failed to read config: file not found".to_string()));
    /// ```
    fn adapt_error<A, F>(computation: Self::WithType<A>, transform: F) -> Self::WithType<A>
    where
        F: FnOnce(E) -> E,
        A: 'static;

    /// Converts an error to a success value.
    ///
    /// If the computation fails, the handler is applied to the error
    /// to produce a success value. This is different from `catch_error`
    /// in that the handler returns a plain value, not a computation.
    ///
    /// # Arguments
    ///
    /// * `computation` - The computation that might fail
    /// * `handler` - A function that converts the error to a success value.
    ///   Must be a pure function (no side effects, same output for same input).
    ///
    /// # Returns
    ///
    /// The original value if successful, or the handler's result if it failed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::MonadError;
    ///
    /// let failing: Result<i32, String> = Err("error".to_string());
    /// let handled = <Result<i32, String>>::handle_error(failing, |_| 0);
    /// assert_eq!(handled, Ok(0));
    /// ```
    fn handle_error<A, F>(computation: Self::WithType<A>, handler: F) -> Self::WithType<A>
    where
        F: FnOnce(E) -> A,
        A: 'static;

    /// Recovers from specific errors using a partial function.
    ///
    /// The partial handler receives a reference to the error and returns
    /// `Some(value)` if it can handle the error, or `None` if it cannot.
    /// If `None` is returned, the original error is preserved.
    ///
    /// # Arguments
    ///
    /// * `computation` - The computation that might fail
    /// * `partial_handler` - A function that may handle the error.
    ///   Must be a pure function (no side effects, same output for same input).
    ///
    /// # Returns
    ///
    /// The original value if successful, the handler's result if it matched,
    /// or the original error if the handler returned `None`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::MonadError;
    ///
    /// #[derive(Debug, Clone, PartialEq)]
    /// enum AppError {
    ///     NotFound,
    ///     Unauthorized,
    /// }
    ///
    /// let not_found: Result<i32, AppError> = Err(AppError::NotFound);
    /// let recovered = <Result<i32, AppError>>::recover(not_found, |e| {
    ///     match e {
    ///         AppError::NotFound => Some(0),
    ///         _ => None,
    ///     }
    /// });
    /// assert_eq!(recovered, Ok(0));
    /// ```
    fn recover<A, F>(computation: Self::WithType<A>, partial_handler: F) -> Self::WithType<A>
    where
        F: FnOnce(&E) -> Option<A>,
        A: 'static;

    /// Recovers from specific errors using a partial function that returns a computation.
    ///
    /// Similar to `recover`, but the handler returns a computation instead of a plain value.
    ///
    /// # Arguments
    ///
    /// * `computation` - The computation that might fail
    /// * `partial_handler` - A function that may handle the error.
    ///   Must be a pure function (no side effects, same output for same input).
    ///
    /// # Returns
    ///
    /// The original value if successful, the handler's computation if it matched,
    /// or the original error if the handler returned `None`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::MonadError;
    ///
    /// fn fetch_from_database(key: &str) -> Result<String, String> {
    ///     Ok("data from db".to_string())
    /// }
    ///
    /// let cache_result: Result<String, String> = Err("cache miss".to_string());
    /// let recovered = <Result<String, String>>::recover_with_partial(
    ///     cache_result,
    ///     |error| {
    ///         if error.contains("cache miss") {
    ///             Some(fetch_from_database("user:1"))
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// );
    /// assert_eq!(recovered, Ok("data from db".to_string()));
    /// ```
    fn recover_with_partial<A, F>(
        computation: Self::WithType<A>,
        partial_handler: F,
    ) -> Self::WithType<A>
    where
        F: FnOnce(&E) -> Option<Self::WithType<A>>,
        A: 'static;

    /// Ensures a condition holds for the success value.
    ///
    /// If the computation succeeds and the predicate returns `true`,
    /// the value is returned. If the predicate returns `false`,
    /// an error is generated using the error function.
    ///
    /// # Arguments
    ///
    /// * `computation` - The computation to validate
    /// * `error` - A function to generate the error (lazily evaluated).
    ///   Must be a pure function (no side effects, same output for same input).
    /// * `predicate` - A function to test the value.
    ///   Must be a pure function (no side effects, same output for same input).
    ///
    /// # Returns
    ///
    /// The original value if successful and predicate holds,
    /// or an error if the predicate fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::MonadError;
    ///
    /// fn validate_age(age: i32) -> Result<i32, String> {
    ///     <Result<i32, String>>::ensure(
    ///         Ok(age),
    ///         || "Age must be between 0 and 150".to_string(),
    ///         |&a| a >= 0 && a <= 150
    ///     )
    /// }
    ///
    /// assert_eq!(validate_age(25), Ok(25));
    /// assert_eq!(validate_age(-5), Err("Age must be between 0 and 150".to_string()));
    /// ```
    fn ensure<A, F, P>(computation: Self::WithType<A>, error: F, predicate: P) -> Self::WithType<A>
    where
        F: FnOnce() -> E,
        P: FnOnce(&A) -> bool,
        A: 'static;

    /// Ensures a condition holds for the success value, with value-dependent error.
    ///
    /// Similar to `ensure`, but the error function receives a reference to the value,
    /// allowing the error message to include information about the failing value.
    ///
    /// # Arguments
    ///
    /// * `computation` - The computation to validate
    /// * `error_fn` - A function to generate the error from the value reference.
    ///   Must be a pure function (no side effects, same output for same input).
    /// * `predicate` - A function to test the value.
    ///   Must be a pure function (no side effects, same output for same input).
    ///
    /// # Returns
    ///
    /// The original value if successful and predicate holds,
    /// or an error if the predicate fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::MonadError;
    ///
    /// fn validate_positive(n: i32) -> Result<i32, String> {
    ///     <Result<i32, String>>::ensure_or(
    ///         Ok(n),
    ///         |v| format!("{} is not a positive number", v),
    ///         |&v| v > 0
    ///     )
    /// }
    ///
    /// assert_eq!(validate_positive(42), Ok(42));
    /// assert_eq!(validate_positive(-5), Err("-5 is not a positive number".to_string()));
    /// ```
    fn ensure_or<A, F, P>(
        computation: Self::WithType<A>,
        error_fn: F,
        predicate: P,
    ) -> Self::WithType<A>
    where
        F: FnOnce(&A) -> E,
        P: FnOnce(&A) -> bool,
        A: 'static;

    /// Transforms both success and error values to the same type.
    ///
    /// This is useful when you want to convert a computation to a unified
    /// result type, regardless of whether it succeeded or failed.
    ///
    /// # Arguments
    ///
    /// * `computation` - The computation to transform
    /// * `recover` - A function to transform errors.
    ///   Must be a pure function (no side effects, same output for same input).
    /// * `transform` - A function to transform success values.
    ///   Must be a pure function (no side effects, same output for same input).
    ///
    /// # Returns
    ///
    /// A successful computation containing the transformed value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::MonadError;
    ///
    /// fn to_status_message(result: Result<i32, String>) -> Result<String, String> {
    ///     <Result<i32, String>>::redeem(
    ///         result,
    ///         |e| format!("Error: {}", e),
    ///         |v| format!("Success: {}", v)
    ///     )
    /// }
    ///
    /// assert_eq!(to_status_message(Ok(42)), Ok("Success: 42".to_string()));
    /// assert_eq!(
    ///     to_status_message(Err("not found".to_string())),
    ///     Ok("Error: not found".to_string())
    /// );
    /// ```
    fn redeem<A, B, Recover, Transform>(
        computation: Self::WithType<A>,
        recover: Recover,
        transform: Transform,
    ) -> Self::WithType<B>
    where
        Recover: FnOnce(E) -> B,
        Transform: FnOnce(A) -> B,
        A: 'static,
        B: 'static;

    /// Transforms both success and error values to computations.
    ///
    /// Similar to `redeem`, but the transformation functions return
    /// computations instead of plain values.
    ///
    /// # Arguments
    ///
    /// * `computation` - The computation to transform
    /// * `recover` - A function to transform errors into computations.
    ///   Must be a pure function (no side effects, same output for same input).
    /// * `bind` - A function to transform success values into computations.
    ///   Must be a pure function (no side effects, same output for same input).
    ///
    /// # Returns
    ///
    /// The result of the appropriate transformation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::MonadError;
    ///
    /// fn process_result(result: Result<i32, String>) -> Result<String, String> {
    ///     <Result<i32, String>>::redeem_with(
    ///         result,
    ///         |e| Ok(format!("Handled error: {}", e)),
    ///         |v| {
    ///             if v > 100 {
    ///                 Err("Value too large".to_string())
    ///             } else {
    ///                 Ok(format!("Processed: {}", v))
    ///             }
    ///         }
    ///     )
    /// }
    ///
    /// assert_eq!(process_result(Ok(42)), Ok("Processed: 42".to_string()));
    /// assert_eq!(process_result(Ok(200)), Err("Value too large".to_string()));
    /// assert_eq!(
    ///     process_result(Err("not found".to_string())),
    ///     Ok("Handled error: not found".to_string())
    /// );
    /// ```
    fn redeem_with<A, B, Recover, Bind>(
        computation: Self::WithType<A>,
        recover: Recover,
        bind: Bind,
    ) -> Self::WithType<B>
    where
        Recover: FnOnce(E) -> Self::WithType<B>,
        Bind: FnOnce(A) -> Self::WithType<B>,
        A: 'static,
        B: 'static;
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
        computation.or(default)
    }

    fn adapt_error<A, F>(computation: Result<A, E>, transform: F) -> Result<A, E>
    where
        F: FnOnce(E) -> E,
        A: 'static,
    {
        computation.map_err(transform)
    }

    fn handle_error<A, F>(computation: Result<A, E>, handler: F) -> Result<A, E>
    where
        F: FnOnce(E) -> A,
        A: 'static,
    {
        match computation {
            Ok(value) => Ok(value),
            Err(error) => Ok(handler(error)),
        }
    }

    fn recover<A, F>(computation: Result<A, E>, partial_handler: F) -> Result<A, E>
    where
        F: FnOnce(&E) -> Option<A>,
        A: 'static,
    {
        match computation {
            Ok(value) => Ok(value),
            Err(error) => partial_handler(&error).map_or_else(|| Err(error), Ok),
        }
    }

    fn recover_with_partial<A, F>(computation: Result<A, E>, partial_handler: F) -> Result<A, E>
    where
        F: FnOnce(&E) -> Option<Result<A, E>>,
        A: 'static,
    {
        match computation {
            Ok(value) => Ok(value),
            Err(error) => partial_handler(&error).unwrap_or_else(|| Err(error)),
        }
    }

    fn ensure<A, F, P>(computation: Result<A, E>, error: F, predicate: P) -> Result<A, E>
    where
        F: FnOnce() -> E,
        P: FnOnce(&A) -> bool,
        A: 'static,
    {
        computation.and_then(|value| {
            if predicate(&value) {
                Ok(value)
            } else {
                Err(error())
            }
        })
    }

    fn ensure_or<A, F, P>(computation: Result<A, E>, error_fn: F, predicate: P) -> Result<A, E>
    where
        F: FnOnce(&A) -> E,
        P: FnOnce(&A) -> bool,
        A: 'static,
    {
        computation.and_then(|value| {
            if predicate(&value) {
                Ok(value)
            } else {
                Err(error_fn(&value))
            }
        })
    }

    fn redeem<A, B, Recover, Transform>(
        computation: Result<A, E>,
        recover: Recover,
        transform: Transform,
    ) -> Result<B, E>
    where
        Recover: FnOnce(E) -> B,
        Transform: FnOnce(A) -> B,
        A: 'static,
        B: 'static,
    {
        match computation {
            Ok(value) => Ok(transform(value)),
            Err(error) => Ok(recover(error)),
        }
    }

    fn redeem_with<A, B, Recover, Bind>(
        computation: Result<A, E>,
        recover: Recover,
        bind: Bind,
    ) -> Result<B, E>
    where
        Recover: FnOnce(E) -> Result<B, E>,
        Bind: FnOnce(A) -> Result<B, E>,
        A: 'static,
        B: 'static,
    {
        match computation {
            Ok(value) => bind(value),
            Err(error) => recover(error),
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
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
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
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
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
        let left: Result<String, String> = thrown.flat_map(|n| Ok(format!("got: {n}")));
        let right: Result<String, String> = <Result<String, String>>::throw_error(error);

        assert_eq!(left, right);
    }

    // =========================================================================
    // MonadErrorExt Tests
    // =========================================================================

    #[rstest]
    fn monad_error_ext_trait_is_defined() {
        fn assert_trait_exists<E, T: MonadErrorExt<E>>() {}
        let _ = assert_trait_exists::<String, Result<i32, String>>;
    }

    #[rstest]
    fn result_map_error_transforms_err() {
        let computation: Result<i32, i32> = Err(404);
        let mapped: Result<i32, String> =
            computation.map_error(|code| format!("HTTP Error: {code}"));
        assert_eq!(mapped, Err("HTTP Error: 404".to_string()));
    }

    #[rstest]
    fn result_map_error_preserves_ok() {
        let computation: Result<i32, i32> = Ok(42);
        let mapped: Result<i32, String> =
            computation.map_error(|code| format!("HTTP Error: {code}"));
        assert_eq!(mapped, Ok(42));
    }

    // =========================================================================
    // adapt_error Tests
    // =========================================================================

    #[rstest]
    fn result_adapt_error_transforms_err() {
        let computation: Result<i32, String> = Err("file not found".to_string());
        let adapted = <Result<i32, String>>::adapt_error(computation, |e| {
            format!("failed to read config: {e}")
        });
        assert_eq!(
            adapted,
            Err("failed to read config: file not found".to_string())
        );
    }

    #[rstest]
    fn result_adapt_error_preserves_ok() {
        let computation: Result<i32, String> = Ok(42);
        let adapted = <Result<i32, String>>::adapt_error(computation, |e| {
            format!("failed to read config: {e}")
        });
        assert_eq!(adapted, Ok(42));
    }

    // =========================================================================
    // handle_error Tests
    // =========================================================================

    #[rstest]
    fn result_handle_error_converts_err_to_ok() {
        let failing: Result<i32, String> = Err("error".to_string());
        let handled = <Result<i32, String>>::handle_error(failing, |_| 0);
        assert_eq!(handled, Ok(0));
    }

    #[rstest]
    fn result_handle_error_preserves_ok() {
        let success: Result<i32, String> = Ok(42);
        let handled = <Result<i32, String>>::handle_error(success, |_| 0);
        assert_eq!(handled, Ok(42));
    }

    #[rstest]
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    fn result_handle_error_uses_error_value() {
        let failing: Result<i32, String> = Err("hello".to_string());
        let handled = <Result<i32, String>>::handle_error(failing, |e| e.len() as i32);
        assert_eq!(handled, Ok(5));
    }

    // =========================================================================
    // recover Tests
    // =========================================================================

    #[derive(Debug, Clone, PartialEq)]
    #[allow(dead_code)]
    enum TestError {
        NotFound,
        Unauthorized,
        Internal(String),
    }

    #[rstest]
    fn result_recover_matches_and_recovers() {
        let not_found: Result<i32, TestError> = Err(TestError::NotFound);
        let recovered = <Result<i32, TestError>>::recover(not_found, |e| match e {
            TestError::NotFound => Some(0),
            _ => None,
        });
        assert_eq!(recovered, Ok(0));
    }

    #[rstest]
    fn result_recover_does_not_match() {
        let unauthorized: Result<i32, TestError> = Err(TestError::Unauthorized);
        let not_recovered = <Result<i32, TestError>>::recover(unauthorized, |e| match e {
            TestError::NotFound => Some(0),
            _ => None,
        });
        assert_eq!(not_recovered, Err(TestError::Unauthorized));
    }

    #[rstest]
    fn result_recover_preserves_ok() {
        let success: Result<i32, TestError> = Ok(42);
        let recovered = <Result<i32, TestError>>::recover(success, |_| Some(0));
        assert_eq!(recovered, Ok(42));
    }

    // =========================================================================
    // recover_with_partial Tests
    // =========================================================================

    #[rstest]
    fn result_recover_with_partial_matches_and_recovers() {
        let cache_miss: Result<String, String> = Err("cache miss".to_string());
        let recovered = <Result<String, String>>::recover_with_partial(cache_miss, |error| {
            if error.contains("cache miss") {
                Some(Ok("data from db".to_string()))
            } else {
                None
            }
        });
        assert_eq!(recovered, Ok("data from db".to_string()));
    }

    #[rstest]
    fn result_recover_with_partial_does_not_match() {
        let other_error: Result<String, String> = Err("network error".to_string());
        let not_recovered = <Result<String, String>>::recover_with_partial(other_error, |error| {
            if error.contains("cache miss") {
                Some(Ok("data from db".to_string()))
            } else {
                None
            }
        });
        assert_eq!(not_recovered, Err("network error".to_string()));
    }

    #[rstest]
    fn result_recover_with_partial_preserves_ok() {
        let success: Result<String, String> = Ok("cached data".to_string());
        let recovered = <Result<String, String>>::recover_with_partial(success, |_| {
            Some(Ok("data from db".to_string()))
        });
        assert_eq!(recovered, Ok("cached data".to_string()));
    }

    #[rstest]
    fn result_recover_with_partial_can_return_err() {
        let cache_miss: Result<String, String> = Err("cache miss".to_string());
        let recovered = <Result<String, String>>::recover_with_partial(cache_miss, |_| {
            Some(Err("database error".to_string()))
        });
        assert_eq!(recovered, Err("database error".to_string()));
    }

    // =========================================================================
    // ensure Tests
    // =========================================================================

    #[rstest]
    fn result_ensure_passes_when_predicate_true() {
        let computation: Result<i32, String> = Ok(25);
        let ensured = <Result<i32, String>>::ensure(
            computation,
            || "Age must be between 0 and 150".to_string(),
            |&a| (0..=150).contains(&a),
        );
        assert_eq!(ensured, Ok(25));
    }

    #[rstest]
    fn result_ensure_fails_when_predicate_false() {
        let computation: Result<i32, String> = Ok(-5);
        let ensured = <Result<i32, String>>::ensure(
            computation,
            || "Age must be between 0 and 150".to_string(),
            |&a| (0..=150).contains(&a),
        );
        assert_eq!(ensured, Err("Age must be between 0 and 150".to_string()));
    }

    #[rstest]
    fn result_ensure_propagates_err() {
        let computation: Result<i32, String> = Err("initial error".to_string());
        let ensured = <Result<i32, String>>::ensure(
            computation,
            || "Age must be between 0 and 150".to_string(),
            |&a| (0..=150).contains(&a),
        );
        assert_eq!(ensured, Err("initial error".to_string()));
    }

    // =========================================================================
    // ensure_or Tests
    // =========================================================================

    #[rstest]
    fn result_ensure_or_passes_when_predicate_true() {
        let computation: Result<i32, String> = Ok(42);
        let ensured = <Result<i32, String>>::ensure_or(
            computation,
            |v| format!("{v} is not a positive number"),
            |&v| v > 0,
        );
        assert_eq!(ensured, Ok(42));
    }

    #[rstest]
    fn result_ensure_or_fails_with_value_in_error() {
        let computation: Result<i32, String> = Ok(-5);
        let ensured = <Result<i32, String>>::ensure_or(
            computation,
            |v| format!("{v} is not a positive number"),
            |&v| v > 0,
        );
        assert_eq!(ensured, Err("-5 is not a positive number".to_string()));
    }

    #[rstest]
    fn result_ensure_or_propagates_err() {
        let computation: Result<i32, String> = Err("initial error".to_string());
        let ensured = <Result<i32, String>>::ensure_or(
            computation,
            |v| format!("{v} is not a positive number"),
            |&v| v > 0,
        );
        assert_eq!(ensured, Err("initial error".to_string()));
    }

    // =========================================================================
    // redeem Tests
    // =========================================================================

    #[rstest]
    fn result_redeem_transforms_ok() {
        let success: Result<i32, String> = Ok(42);
        let redeemed = <Result<i32, String>>::redeem(
            success,
            |e| format!("Error: {e}"),
            |v| format!("Success: {v}"),
        );
        assert_eq!(redeemed, Ok("Success: 42".to_string()));
    }

    #[rstest]
    fn result_redeem_transforms_err() {
        let failing: Result<i32, String> = Err("not found".to_string());
        let redeemed = <Result<i32, String>>::redeem(
            failing,
            |e| format!("Error: {e}"),
            |v| format!("Success: {v}"),
        );
        assert_eq!(redeemed, Ok("Error: not found".to_string()));
    }

    // =========================================================================
    // redeem_with Tests
    // =========================================================================

    #[rstest]
    fn result_redeem_with_transforms_ok() {
        let success: Result<i32, String> = Ok(42);
        let redeemed = <Result<i32, String>>::redeem_with(
            success,
            |e| Ok(format!("Handled error: {e}")),
            |v| Ok(format!("Processed: {v}")),
        );
        assert_eq!(redeemed, Ok("Processed: 42".to_string()));
    }

    #[rstest]
    fn result_redeem_with_transforms_err() {
        let failing: Result<i32, String> = Err("not found".to_string());
        let redeemed = <Result<i32, String>>::redeem_with(
            failing,
            |e| Ok(format!("Handled error: {e}")),
            |v| Ok(format!("Processed: {v}")),
        );
        assert_eq!(redeemed, Ok("Handled error: not found".to_string()));
    }

    #[rstest]
    fn result_redeem_with_bind_can_fail() {
        let success: Result<i32, String> = Ok(200);
        let redeemed = <Result<i32, String>>::redeem_with(
            success,
            |e| Ok(format!("Handled error: {e}")),
            |v| {
                if v > 100 {
                    Err("Value too large".to_string())
                } else {
                    Ok(format!("Processed: {v}"))
                }
            },
        );
        assert_eq!(redeemed, Err("Value too large".to_string()));
    }

    // =========================================================================
    // Law Tests for New Methods
    // =========================================================================

    // map_error Identity Law
    #[rstest]
    fn result_map_error_identity_law() {
        let computation: Result<i32, String> = Err("error".to_string());
        let mapped: Result<i32, String> = computation.clone().map_error(|e| e);
        assert_eq!(mapped, computation);
    }

    // map_error Composition Law
    #[rstest]
    fn result_map_error_composition_law() {
        let computation: Result<i32, String> = Err("error".to_string());
        let function1 = |e: String| format!("f1: {e}");
        let function2 = |e: String| format!("f2: {e}");

        let left: Result<i32, String> = computation
            .clone()
            .map_error(function1)
            .map_error(function2);
        let right: Result<i32, String> = computation.map_error(|e| function2(function1(e)));
        assert_eq!(left, right);
    }

    // map_error Success Preservation Law
    #[rstest]
    fn result_map_error_success_preservation_law() {
        let success: Result<i32, String> = Ok(42);
        let mapped: Result<i32, String> = success.clone().map_error(|e| format!("wrapped: {e}"));
        assert_eq!(mapped, success);
    }

    // adapt_error Identity Law
    #[rstest]
    fn result_adapt_error_identity_law() {
        let computation: Result<i32, String> = Err("error".to_string());
        let adapted = <Result<i32, String>>::adapt_error(computation.clone(), |e| e);
        assert_eq!(adapted, computation);
    }

    // adapt_error Success Preservation Law
    #[rstest]
    fn result_adapt_error_success_preservation_law() {
        let success: Result<i32, String> = Ok(42);
        let adapted =
            <Result<i32, String>>::adapt_error(success.clone(), |e| format!("context: {e}"));
        assert_eq!(adapted, success);
    }

    // handle_error Handle Throw Law
    #[rstest]
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    fn result_handle_error_handle_throw_law() {
        let error = "test".to_string();
        let handler = |e: String| e.len() as i32;

        let left = <Result<i32, String>>::handle_error(
            <Result<i32, String>>::throw_error(error.clone()),
            handler,
        );
        let right: Result<i32, String> = <Result<(), String>>::pure(handler(error));
        assert_eq!(left, right);
    }

    // handle_error Success Preservation Law
    #[rstest]
    fn result_handle_error_success_preservation_law() {
        let success: Result<i32, String> = Ok(42);
        let handled = <Result<i32, String>>::handle_error(success.clone(), |_| 0);
        assert_eq!(handled, success);
    }

    // recover Matching Law
    #[rstest]
    fn result_recover_matching_law() {
        let error = "error".to_string();
        let value = 42;

        let left =
            <Result<i32, String>>::recover(<Result<i32, String>>::throw_error(error), |_| {
                Some(value)
            });
        let right: Result<i32, String> = <Result<(), String>>::pure(value);
        assert_eq!(left, right);
    }

    // recover Non-Matching Law
    #[rstest]
    fn result_recover_non_matching_law() {
        let error = "error".to_string();

        let left: Result<i32, String> = <Result<i32, String>>::recover(
            <Result<i32, String>>::throw_error(error.clone()),
            |_| None,
        );
        let right: Result<i32, String> = <Result<i32, String>>::throw_error(error);
        assert_eq!(left, right);
    }

    // recover Success Preservation Law
    #[rstest]
    fn result_recover_success_preservation_law() {
        let success: Result<i32, String> = Ok(42);
        let recovered = <Result<i32, String>>::recover(success.clone(), |_| Some(0));
        assert_eq!(recovered, success);
    }

    // recover_with_partial Matching Law
    #[rstest]
    fn result_recover_with_partial_matching_law() {
        let error = "error".to_string();
        let recovery: Result<i32, String> = Ok(42);

        let left = <Result<i32, String>>::recover_with_partial(
            <Result<i32, String>>::throw_error(error),
            |_| Some(recovery.clone()),
        );
        assert_eq!(left, recovery);
    }

    // recover_with_partial Non-Matching Law
    #[rstest]
    fn result_recover_with_partial_non_matching_law() {
        let error = "error".to_string();

        let left: Result<i32, String> = <Result<i32, String>>::recover_with_partial(
            <Result<i32, String>>::throw_error(error.clone()),
            |_| None,
        );
        let right: Result<i32, String> = <Result<i32, String>>::throw_error(error);
        assert_eq!(left, right);
    }

    // ensure True Law
    #[rstest]
    fn result_ensure_true_law() {
        let value = 42;
        let success: Result<i32, String> = Ok(value);

        let ensured =
            <Result<i32, String>>::ensure(success.clone(), || "error".to_string(), |_| true);
        assert_eq!(ensured, success);
    }

    // ensure False Law
    #[rstest]
    fn result_ensure_false_law() {
        let value = 42;
        let error = "error".to_string();
        let success: Result<i32, String> = Ok(value);

        let ensured = <Result<i32, String>>::ensure(success, || error.clone(), |_| false);
        let thrown: Result<i32, String> = <Result<i32, String>>::throw_error(error);
        assert_eq!(ensured, thrown);
    }

    // ensure Error Passthrough Law
    #[rstest]
    fn result_ensure_error_passthrough_law() {
        let error1 = "error1".to_string();
        let error2 = "error2".to_string();
        let thrown: Result<i32, String> = <Result<i32, String>>::throw_error(error1.clone());

        let ensured = <Result<i32, String>>::ensure(thrown, || error2, |_| false);
        let original: Result<i32, String> = <Result<i32, String>>::throw_error(error1);
        assert_eq!(ensured, original);
    }

    // ensure_or True Law
    #[rstest]
    fn result_ensure_or_true_law() {
        let value = 42;
        let success: Result<i32, String> = Ok(value);

        let ensured =
            <Result<i32, String>>::ensure_or(success.clone(), |_| "error".to_string(), |_| true);
        assert_eq!(ensured, success);
    }

    // ensure_or False Law
    #[rstest]
    fn result_ensure_or_false_law() {
        let value = 42;
        let success: Result<i32, String> = Ok(value);

        let ensured =
            <Result<i32, String>>::ensure_or(success, |v| format!("Invalid value: {v}"), |_| false);
        let thrown: Result<i32, String> =
            <Result<i32, String>>::throw_error(format!("Invalid value: {value}"));
        assert_eq!(ensured, thrown);
    }

    // redeem Success Law
    #[rstest]
    fn result_redeem_success_law() {
        let value = 42;
        let success: Result<i32, String> = Ok(value);
        let transform = |v: i32| format!("success: {v}");

        let redeemed =
            <Result<i32, String>>::redeem(success, |_| "recovered".to_string(), transform);
        let expected: Result<String, String> = <Result<(), String>>::pure(transform(value));
        assert_eq!(redeemed, expected);
    }

    // redeem Error Law
    #[rstest]
    fn result_redeem_error_law() {
        let error = "error".to_string();
        let failed: Result<i32, String> = <Result<i32, String>>::throw_error(error.clone());
        let recover = |e: String| format!("error: {e}");

        let redeemed = <Result<i32, String>>::redeem(failed, recover, |v| format!("success: {v}"));
        let expected: Result<String, String> = <Result<(), String>>::pure(recover(error));
        assert_eq!(redeemed, expected);
    }

    // redeem_with Success Law
    #[rstest]
    fn result_redeem_with_success_law() {
        let value = 42;
        let success: Result<i32, String> = Ok(value);
        let bind = |v: i32| -> Result<String, String> { Ok(format!("success: {v}")) };

        let redeemed =
            <Result<i32, String>>::redeem_with(success, |_| Ok("recovered".to_string()), bind);
        let expected = bind(value);
        assert_eq!(redeemed, expected);
    }

    // redeem_with Error Law
    #[rstest]
    fn result_redeem_with_error_law() {
        let error = "error".to_string();
        let failed: Result<i32, String> = <Result<i32, String>>::throw_error(error.clone());
        let recover = |e: String| -> Result<String, String> { Ok(format!("error: {e}")) };

        let redeemed =
            <Result<i32, String>>::redeem_with(failed, recover, |v| Ok(format!("success: {v}")));
        let expected = recover(error);
        assert_eq!(redeemed, expected);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use crate::typeclass::Applicative;
    use proptest::prelude::*;

    proptest! {
        // =====================================================================
        // map_error Property Tests
        // =====================================================================

        #[test]
        fn prop_map_error_identity(
            computation in prop::result::maybe_ok(any::<i32>(), any::<String>())
        ) {
            let mapped: Result<i32, String> = computation.clone().map_error(|e| e);
            prop_assert_eq!(mapped, computation);
        }

        #[test]
        fn prop_map_error_composition(
            computation in prop::result::maybe_ok(any::<i32>(), any::<String>())
        ) {
            let function1 = |e: String| format!("f1: {e}");
            let function2 = |e: String| format!("f2: {e}");

            let left: Result<i32, String> = computation.clone().map_error(function1).map_error(function2);
            let right: Result<i32, String> = computation.map_error(|e| function2(function1(e)));
            prop_assert_eq!(left, right);
        }

        #[test]
        fn prop_map_error_success_preservation(value in any::<i32>()) {
            let success: Result<i32, String> = Ok(value);
            let mapped: Result<i32, String> = success.clone().map_error(|e| format!("wrapped: {e}"));
            prop_assert_eq!(mapped, success);
        }

        // =====================================================================
        // adapt_error Property Tests
        // =====================================================================

        #[test]
        fn prop_adapt_error_identity(
            computation in prop::result::maybe_ok(any::<i32>(), any::<String>())
        ) {
            let adapted = <Result<i32, String>>::adapt_error(computation.clone(), |e| e);
            prop_assert_eq!(adapted, computation);
        }

        #[test]
        fn prop_adapt_error_success_preservation(value in any::<i32>()) {
            let success: Result<i32, String> = Ok(value);
            let adapted = <Result<i32, String>>::adapt_error(success.clone(), |e| format!("context: {e}"));
            prop_assert_eq!(adapted, success);
        }

        // =====================================================================
        // handle_error Property Tests
        // =====================================================================

        #[test]
        #[allow(clippy::large_stack_arrays)]
        fn prop_handle_error_handle_throw(error in any::<String>()) {
            let handler = |e: String| e.len();

            let left = <Result<usize, String>>::handle_error(
                <Result<usize, String>>::throw_error(error.clone()),
                handler,
            );
            let right: Result<usize, String> = <Result<(), String>>::pure(handler(error));
            prop_assert_eq!(left, right);
        }

        #[test]
        fn prop_handle_error_success_preservation(value in any::<i32>()) {
            let success: Result<i32, String> = Ok(value);
            let handled = <Result<i32, String>>::handle_error(success.clone(), |_| 0);
            prop_assert_eq!(handled, success);
        }

        // =====================================================================
        // recover Property Tests
        // =====================================================================

        #[test]
        #[allow(clippy::large_stack_arrays)]
        fn prop_recover_matching(error in any::<String>(), value in any::<i32>()) {
            let left = <Result<i32, String>>::recover(
                <Result<i32, String>>::throw_error(error),
                |_| Some(value),
            );
            let right: Result<i32, String> = <Result<(), String>>::pure(value);
            prop_assert_eq!(left, right);
        }

        #[test]
        #[allow(clippy::large_stack_arrays)]
        fn prop_recover_non_matching(error in any::<String>()) {
            let left: Result<i32, String> = <Result<i32, String>>::recover(
                <Result<i32, String>>::throw_error(error.clone()),
                |_| None,
            );
            let right: Result<i32, String> = <Result<i32, String>>::throw_error(error);
            prop_assert_eq!(left, right);
        }

        #[test]
        fn prop_recover_success_preservation(value in any::<i32>()) {
            let success: Result<i32, String> = Ok(value);
            let recovered = <Result<i32, String>>::recover(success.clone(), |_| Some(0));
            prop_assert_eq!(recovered, success);
        }

        // =====================================================================
        // recover_with_partial Property Tests
        // =====================================================================

        #[test]
        #[allow(clippy::large_stack_arrays)]
        fn prop_recover_with_partial_matching(error in any::<String>(), recovery_value in any::<i32>()) {
            let recovery: Result<i32, String> = Ok(recovery_value);
            let left = <Result<i32, String>>::recover_with_partial(
                <Result<i32, String>>::throw_error(error),
                |_| Some(recovery.clone()),
            );
            prop_assert_eq!(left, recovery);
        }

        #[test]
        #[allow(clippy::large_stack_arrays)]
        fn prop_recover_with_partial_non_matching(error in any::<String>()) {
            let left: Result<i32, String> = <Result<i32, String>>::recover_with_partial(
                <Result<i32, String>>::throw_error(error.clone()),
                |_| None,
            );
            let right: Result<i32, String> = <Result<i32, String>>::throw_error(error);
            prop_assert_eq!(left, right);
        }

        // =====================================================================
        // ensure Property Tests
        // =====================================================================

        #[test]
        fn prop_ensure_true(value in any::<i32>()) {
            let success: Result<i32, String> = Ok(value);
            let ensured = <Result<i32, String>>::ensure(success.clone(), || "error".to_string(), |_| true);
            prop_assert_eq!(ensured, success);
        }

        #[test]
        #[allow(clippy::large_stack_arrays)]
        fn prop_ensure_false(value in any::<i32>(), error in any::<String>()) {
            let success: Result<i32, String> = Ok(value);
            let ensured = <Result<i32, String>>::ensure(success, || error.clone(), |_| false);
            let thrown: Result<i32, String> = <Result<i32, String>>::throw_error(error);
            prop_assert_eq!(ensured, thrown);
        }

        #[test]
        #[allow(clippy::large_stack_arrays)]
        fn prop_ensure_error_passthrough(error1 in any::<String>(), error2 in any::<String>()) {
            let thrown: Result<i32, String> = <Result<i32, String>>::throw_error(error1.clone());
            let ensured = <Result<i32, String>>::ensure(thrown, || error2, |_| false);
            let original: Result<i32, String> = <Result<i32, String>>::throw_error(error1);
            prop_assert_eq!(ensured, original);
        }

        // =====================================================================
        // ensure_or Property Tests
        // =====================================================================

        #[test]
        fn prop_ensure_or_true(value in any::<i32>()) {
            let success: Result<i32, String> = Ok(value);
            let ensured = <Result<i32, String>>::ensure_or(success.clone(), |_| "error".to_string(), |_| true);
            prop_assert_eq!(ensured, success);
        }

        #[test]
        fn prop_ensure_or_false(value in any::<i32>()) {
            let success: Result<i32, String> = Ok(value);
            let ensured = <Result<i32, String>>::ensure_or(
                success,
                |v| format!("Invalid value: {v}"),
                |_| false,
            );
            let thrown: Result<i32, String> = <Result<i32, String>>::throw_error(format!("Invalid value: {value}"));
            prop_assert_eq!(ensured, thrown);
        }

        // =====================================================================
        // redeem Property Tests
        // =====================================================================

        #[test]
        fn prop_redeem_success(value in any::<i32>()) {
            let success: Result<i32, String> = Ok(value);
            let transform = |v: i32| format!("success: {v}");

            let redeemed = <Result<i32, String>>::redeem(success, |_| "recovered".to_string(), transform);
            let expected: Result<String, String> = <Result<(), String>>::pure(transform(value));
            prop_assert_eq!(redeemed, expected);
        }

        #[test]
        #[allow(clippy::large_stack_arrays)]
        fn prop_redeem_error(error in any::<String>()) {
            let failed: Result<i32, String> = <Result<i32, String>>::throw_error(error.clone());
            let recover = |e: String| format!("error: {e}");

            let redeemed = <Result<i32, String>>::redeem(failed, recover, |v| format!("success: {v}"));
            let expected: Result<String, String> = <Result<(), String>>::pure(recover(error));
            prop_assert_eq!(redeemed, expected);
        }

        // =====================================================================
        // redeem_with Property Tests
        // =====================================================================

        #[test]
        fn prop_redeem_with_success(value in any::<i32>()) {
            let success: Result<i32, String> = Ok(value);
            let bind = |v: i32| -> Result<String, String> { Ok(format!("success: {v}")) };

            let redeemed = <Result<i32, String>>::redeem_with(
                success,
                |_| Ok("recovered".to_string()),
                bind,
            );
            let expected = bind(value);
            prop_assert_eq!(redeemed, expected);
        }

        #[test]
        #[allow(clippy::large_stack_arrays)]
        fn prop_redeem_with_error(error in any::<String>()) {
            let failed: Result<i32, String> = <Result<i32, String>>::throw_error(error.clone());
            let recover = |e: String| -> Result<String, String> { Ok(format!("error: {e}")) };

            let redeemed = <Result<i32, String>>::redeem_with(
                failed,
                recover,
                |v| Ok(format!("success: {v}")),
            );
            let expected = recover(error);
            prop_assert_eq!(redeemed, expected);
        }
    }
}
