//! `ExceptT` - Except Monad Transformer.
//!
//! `ExceptT` adds error handling capability to any monad.
//! It transforms a monad M into a monad that can fail with error type E.
//!
//! # Overview
//!
//! `ExceptT<E, M>` encapsulates `M<Result<A, E>>` where `E` is the error type
//! and `M` is the inner monad. This allows composing computations that may fail
//! while also using the capabilities of the inner monad.
//!
//! # Design Note
//!
//! Due to Rust's lack of Higher-Kinded Types (HKT), we cannot write a single
//! generic implementation that works for all monads. Instead, we provide
//! specific methods for common monads (Option, Result, IO) using the naming
//! convention `method_option`, `method_result`, `method_io`.
//!
//! # Examples
//!
//! With Option:
//!
//! ```rust
//! use lambars::effect::ExceptT;
//!
//! let except: ExceptT<String, Option<Result<i32, String>>> =
//!     ExceptT::new(Some(Ok(42)));
//! assert_eq!(except.run(), Some(Ok(42)));
//! ```

#![forbid(unsafe_code)]

use super::IO;

/// A monad transformer that adds error handling capability.
///
/// `ExceptT<E, M>` represents a computation that may fail with error type `E`,
/// wrapped in monad `M`.
///
/// # Type Parameters
///
/// - `E`: The error type
/// - `M`: The inner monad type (e.g., `Option<Result<A, E>>`, `Result<Result<A, E>, E2>`, `IO<Result<A, E>>`)
///
/// # Examples
///
/// ```rust
/// use lambars::effect::ExceptT;
///
/// fn validate_positive(value: i32) -> ExceptT<String, Option<Result<i32, String>>> {
///     if value > 0 {
///         ExceptT::pure_option(value)
///     } else {
///         ExceptT::<String, Option<Result<i32, String>>>::throw_option(
///             "Value must be positive".to_string()
///         )
///     }
/// }
///
/// assert_eq!(validate_positive(5).run(), Some(Ok(5)));
/// assert_eq!(
///     validate_positive(-1).run(),
///     Some(Err("Value must be positive".to_string()))
/// );
/// ```
pub struct ExceptT<E, M>
where
    E: 'static,
{
    /// The wrapped monad containing Result<A, E>.
    inner: M,
    /// Phantom data to hold the error type.
    _marker: std::marker::PhantomData<E>,
}

impl<E, M> ExceptT<E, M>
where
    E: 'static,
{
    /// Creates a new `ExceptT` from an inner monad.
    ///
    /// # Arguments
    ///
    /// * `inner` - The inner monad containing Result<A, E>
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ExceptT;
    ///
    /// let except: ExceptT<String, Option<Result<i32, String>>> =
    ///     ExceptT::new(Some(Ok(42)));
    /// assert_eq!(except.run(), Some(Ok(42)));
    /// ```
    pub const fn new(inner: M) -> Self {
        Self {
            inner,
            _marker: std::marker::PhantomData,
        }
    }

    /// Runs the `ExceptT` computation, returning the inner monad.
    ///
    /// # Returns
    ///
    /// The inner monad containing Result<A, E>.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ExceptT;
    ///
    /// let except: ExceptT<String, Option<Result<i32, String>>> =
    ///     ExceptT::new(Some(Ok(42)));
    /// assert_eq!(except.run(), Some(Ok(42)));
    /// ```
    pub fn run(self) -> M {
        self.inner
    }
}

// =============================================================================
// Clone Implementation
// =============================================================================

impl<E, M> Clone for ExceptT<E, M>
where
    E: 'static,
    M: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}

// =============================================================================
// Option-specific Methods
// =============================================================================

impl<E, A> ExceptT<E, Option<Result<A, E>>>
where
    E: Clone + 'static,
    A: 'static,
{
    /// Creates an `ExceptT` that returns a constant value.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to return
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ExceptT;
    ///
    /// let except: ExceptT<String, Option<Result<i32, String>>> =
    ///     ExceptT::pure_option(42);
    /// assert_eq!(except.run(), Some(Ok(42)));
    /// ```
    pub const fn pure_option(value: A) -> Self {
        Self::new(Some(Ok(value)))
    }

    /// Creates an `ExceptT` that throws an error.
    ///
    /// # Arguments
    ///
    /// * `error` - The error to throw
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ExceptT;
    ///
    /// let except: ExceptT<String, Option<Result<i32, String>>> =
    ///     ExceptT::<String, Option<Result<i32, String>>>::throw_option("error".to_string());
    /// assert_eq!(except.run(), Some(Err("error".to_string())));
    /// ```
    pub const fn throw_option(error: E) -> Self {
        Self::new(Some(Err(error)))
    }

    /// Lifts an Option into `ExceptT`.
    ///
    /// # Arguments
    ///
    /// * `inner` - The Option to lift
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ExceptT;
    ///
    /// let inner: Option<i32> = Some(42);
    /// let except: ExceptT<String, Option<Result<i32, String>>> =
    ///     ExceptT::lift_option(inner);
    /// assert_eq!(except.run(), Some(Ok(42)));
    /// ```
    pub fn lift_option(inner: Option<A>) -> Self {
        Self::new(inner.map(Ok))
    }

    /// Maps a function over the value inside the `ExceptT`.
    ///
    /// # Arguments
    ///
    /// * `function` - The function to apply to the value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ExceptT;
    ///
    /// let except: ExceptT<String, Option<Result<i32, String>>> =
    ///     ExceptT::new(Some(Ok(21)));
    /// let mapped = except.fmap_option(|v| v * 2);
    /// assert_eq!(mapped.run(), Some(Ok(42)));
    /// ```
    pub fn fmap_option<B, F>(self, function: F) -> ExceptT<E, Option<Result<B, E>>>
    where
        F: FnOnce(A) -> B,
        B: 'static,
    {
        ExceptT::new(self.inner.map(|result| result.map(function)))
    }

    /// Chains `ExceptT` computations with Option.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes the value and returns a new `ExceptT`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ExceptT;
    ///
    /// let except: ExceptT<String, Option<Result<i32, String>>> =
    ///     ExceptT::new(Some(Ok(10)));
    /// let chained = except.flat_map_option(|v| {
    ///     ExceptT::new(Some(Ok(v * 2)))
    /// });
    /// assert_eq!(chained.run(), Some(Ok(20)));
    /// ```
    pub fn flat_map_option<B, F>(self, function: F) -> ExceptT<E, Option<Result<B, E>>>
    where
        F: FnOnce(A) -> ExceptT<E, Option<Result<B, E>>>,
        B: 'static,
    {
        match self.inner {
            Some(Ok(value)) => function(value),
            Some(Err(error)) => ExceptT::new(Some(Err(error))),
            None => ExceptT::new(None),
        }
    }

    /// Catches an error and potentially recovers.
    ///
    /// # Arguments
    ///
    /// * `computation` - The computation that may fail
    /// * `handler` - A function that handles the error and returns a new `ExceptT`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ExceptT;
    ///
    /// let failing: ExceptT<String, Option<Result<i32, String>>> =
    ///     ExceptT::new(Some(Err("error".to_string())));
    /// let recovered = ExceptT::catch_option(failing, |e| {
    ///     ExceptT::new(Some(Ok(e.len() as i32)))
    /// });
    /// assert_eq!(recovered.run(), Some(Ok(5)));
    /// ```
    pub fn catch_option<F>(computation: Self, handler: F) -> Self
    where
        F: FnOnce(E) -> Self,
    {
        match computation.inner {
            Some(Ok(value)) => Self::new(Some(Ok(value))),
            Some(Err(error)) => handler(error),
            None => Self::new(None),
        }
    }
}

// =============================================================================
// Result-specific Methods
// =============================================================================

impl<E, A, E2> ExceptT<E, Result<Result<A, E>, E2>>
where
    E: Clone + 'static,
    A: 'static,
    E2: 'static,
{
    /// Creates an `ExceptT` that returns a constant value.
    pub const fn pure_result(value: A) -> Self {
        Self::new(Ok(Ok(value)))
    }

    /// Creates an `ExceptT` that throws an error.
    pub const fn throw_result(error: E) -> Self {
        Self::new(Ok(Err(error)))
    }

    /// Lifts a Result into `ExceptT`.
    pub fn lift_result(inner: Result<A, E2>) -> Self {
        Self::new(inner.map(Ok))
    }

    /// Maps a function over the value inside the `ExceptT`.
    pub fn fmap_result<B, F>(self, function: F) -> ExceptT<E, Result<Result<B, E>, E2>>
    where
        F: FnOnce(A) -> B,
        B: 'static,
    {
        ExceptT::new(self.inner.map(|result| result.map(function)))
    }

    /// Chains `ExceptT` computations with Result.
    pub fn flat_map_result<B, F>(self, function: F) -> ExceptT<E, Result<Result<B, E>, E2>>
    where
        F: FnOnce(A) -> ExceptT<E, Result<Result<B, E>, E2>>,
        B: 'static,
    {
        match self.inner {
            Ok(Ok(value)) => function(value),
            Ok(Err(error)) => ExceptT::new(Ok(Err(error))),
            Err(outer_error) => ExceptT::new(Err(outer_error)),
        }
    }

    /// Catches an error and potentially recovers.
    pub fn catch_result<F>(computation: Self, handler: F) -> Self
    where
        F: FnOnce(E) -> Self,
    {
        match computation.inner {
            Ok(Ok(value)) => Self::new(Ok(Ok(value))),
            Ok(Err(error)) => handler(error),
            Err(outer_error) => Self::new(Err(outer_error)),
        }
    }
}

// =============================================================================
// IO-specific Methods
// =============================================================================

impl<E, A> ExceptT<E, IO<Result<A, E>>>
where
    E: Clone + 'static,
    A: 'static,
{
    /// Creates an `ExceptT` that returns a constant value.
    pub fn pure_io(value: A) -> Self {
        Self::new(IO::pure(Ok(value)))
    }

    /// Creates an `ExceptT` that throws an error.
    pub fn throw_io(error: E) -> Self {
        Self::new(IO::pure(Err(error)))
    }

    /// Lifts an IO into `ExceptT`.
    pub fn lift_io(inner: IO<A>) -> Self {
        Self::new(inner.fmap(Ok))
    }

    /// Maps a function over the value inside the `ExceptT`.
    pub fn fmap_io<B, F>(self, function: F) -> ExceptT<E, IO<Result<B, E>>>
    where
        F: FnOnce(A) -> B + 'static,
        B: 'static,
    {
        ExceptT::new(self.inner.fmap(move |result| result.map(function)))
    }

    /// Chains `ExceptT` computations with IO.
    pub fn flat_map_io<B, F>(self, function: F) -> ExceptT<E, IO<Result<B, E>>>
    where
        F: FnOnce(A) -> ExceptT<E, IO<Result<B, E>>> + 'static,
        B: 'static,
    {
        ExceptT::new(self.inner.flat_map(move |result| match result {
            Ok(value) => function(value).inner,
            Err(error) => IO::pure(Err(error)),
        }))
    }

    /// Catches an error and potentially recovers.
    pub fn catch_io<F>(computation: Self, handler: F) -> Self
    where
        F: FnOnce(E) -> Self + 'static,
    {
        Self::new(computation.inner.flat_map(move |result| match result {
            Ok(value) => IO::pure(Ok(value)),
            Err(error) => handler(error).inner,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn except_transformer_new_and_run() {
        let except: ExceptT<String, Option<Result<i32, String>>> = ExceptT::new(Some(Ok(42)));
        assert_eq!(except.run(), Some(Ok(42)));
    }

    #[test]
    fn except_transformer_clone() {
        let except: ExceptT<String, Option<Result<i32, String>>> = ExceptT::new(Some(Ok(42)));
        let cloned = except.clone();
        assert_eq!(except.run(), Some(Ok(42)));
        assert_eq!(cloned.run(), Some(Ok(42)));
    }

    #[test]
    fn except_transformer_pure_option() {
        let except: ExceptT<String, Option<Result<i32, String>>> = ExceptT::pure_option(42);
        assert_eq!(except.run(), Some(Ok(42)));
    }

    #[test]
    fn except_transformer_throw_option() {
        let except: ExceptT<String, Option<Result<i32, String>>> =
            ExceptT::<String, Option<Result<i32, String>>>::throw_option("error".to_string());
        assert_eq!(except.run(), Some(Err("error".to_string())));
    }

    #[test]
    fn except_transformer_flat_map_option() {
        let except: ExceptT<String, Option<Result<i32, String>>> = ExceptT::new(Some(Ok(10)));
        let chained = except.flat_map_option(|v| ExceptT::new(Some(Ok(v * 2))));
        assert_eq!(chained.run(), Some(Ok(20)));
    }
}
