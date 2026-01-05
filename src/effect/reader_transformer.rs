//! `ReaderT` - Reader Monad Transformer.
//!
//! `ReaderT` adds environment reading capability to any monad.
//! It transforms a monad M into a monad that has access to an environment R.
//!
//! # Overview
//!
//! `ReaderT<R, M>` encapsulates a function `R -> M` where `R` is the environment
//! type and `M` is the inner monad. This allows composing computations that
//! need to read from an environment while also using the capabilities of the
//! inner monad (e.g., Option for failure, Result for errors, IO for side effects).
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
//! use lambars::effect::ReaderT;
//!
//! let reader: ReaderT<i32, Option<i32>> = ReaderT::new(|environment| Some(environment * 2));
//! assert_eq!(reader.run(21), Some(42));
//! ```
//!
//! With Result:
//!
//! ```rust
//! use lambars::effect::ReaderT;
//!
//! let reader: ReaderT<i32, Result<i32, String>> = ReaderT::new(|environment| Ok(environment * 2));
//! assert_eq!(reader.run(21), Ok(42));
//! ```
//!
//! With IO:
//!
//! ```rust
//! use lambars::effect::{ReaderT, IO};
//!
//! let reader: ReaderT<i32, IO<i32>> = ReaderT::new(|environment| IO::pure(environment * 2));
//! let io = reader.run(21);
//! assert_eq!(io.run_unsafe(), 42);
//! ```

#![forbid(unsafe_code)]

use std::rc::Rc;

use super::IO;

/// A monad transformer that adds environment reading capability.
///
/// `ReaderT<R, M>` represents a computation that, given an environment of type `R`,
/// produces a value wrapped in monad `M`.
///
/// # Type Parameters
///
/// - `R`: The environment type (read-only context)
/// - `M`: The inner monad type (e.g., `Option<A>`, `Result<A, E>`, `IO<A>`)
///
/// # Examples
///
/// ```rust
/// use lambars::effect::ReaderT;
///
/// #[derive(Clone)]
/// struct Config { port: u16 }
///
/// fn get_port() -> ReaderT<Config, Option<u16>> {
///     ReaderT::new(|config: Config| Some(config.port))
/// }
///
/// let config = Config { port: 8080 };
/// assert_eq!(get_port().run(config), Some(8080));
/// ```
pub struct ReaderT<R, M>
where
    R: 'static,
{
    /// The wrapped function from environment to inner monad.
    /// Uses Rc to allow cloning of the `ReaderT` for `flat_map`.
    run_function: Rc<dyn Fn(R) -> M>,
}

impl<R, M> ReaderT<R, M>
where
    R: 'static,
    M: 'static,
{
    /// Creates a new `ReaderT` from a function.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes an environment and produces a wrapped result
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ReaderT;
    ///
    /// let reader: ReaderT<i32, Option<i32>> = ReaderT::new(|environment| Some(environment * 2));
    /// assert_eq!(reader.run(21), Some(42));
    /// ```
    pub fn new<F>(function: F) -> Self
    where
        F: Fn(R) -> M + 'static,
    {
        Self {
            run_function: Rc::new(function),
        }
    }

    /// Runs the `ReaderT` computation with the given environment.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment to run the computation with
    ///
    /// # Returns
    ///
    /// The result wrapped in the inner monad.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ReaderT;
    ///
    /// let reader: ReaderT<i32, Option<i32>> = ReaderT::new(|environment| Some(environment + 1));
    /// assert_eq!(reader.run(41), Some(42));
    /// // ReaderT can be run multiple times
    /// assert_eq!(reader.run(0), Some(1));
    /// ```
    pub fn run(&self, environment: R) -> M {
        (self.run_function)(environment)
    }
}

// =============================================================================
// Clone Implementation
// =============================================================================

impl<R, M> Clone for ReaderT<R, M>
where
    R: 'static,
{
    fn clone(&self) -> Self {
        Self {
            run_function: self.run_function.clone(),
        }
    }
}

// =============================================================================
// Option-specific Methods
// =============================================================================

impl<R, A> ReaderT<R, Option<A>>
where
    R: 'static,
    A: 'static,
{
    /// Creates a `ReaderT` that returns a constant value wrapped in Some.
    ///
    /// This is the pure/return operation for `ReaderT` with Option.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to wrap
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ReaderT;
    ///
    /// let reader: ReaderT<i32, Option<i32>> = ReaderT::pure_option(42);
    /// assert_eq!(reader.run(999), Some(42)); // environment is ignored
    /// ```
    pub fn pure_option(value: A) -> Self
    where
        A: Clone,
    {
        Self::new(move |_| Some(value.clone()))
    }

    /// Lifts an Option into `ReaderT`.
    ///
    /// The resulting `ReaderT` ignores the environment and returns the inner Option.
    ///
    /// # Arguments
    ///
    /// * `inner` - The Option to lift
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ReaderT;
    ///
    /// let inner: Option<i32> = Some(42);
    /// let reader: ReaderT<String, Option<i32>> = ReaderT::lift_option(inner);
    /// assert_eq!(reader.run("ignored".to_string()), Some(42));
    /// ```
    pub fn lift_option(inner: Option<A>) -> Self
    where
        A: Clone,
    {
        Self::new(move |_| inner.clone())
    }

    /// Maps a function over the value inside the Option.
    ///
    /// # Arguments
    ///
    /// * `function` - The function to apply to the value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ReaderT;
    ///
    /// let reader: ReaderT<i32, Option<i32>> = ReaderT::new(|environment| Some(environment));
    /// let mapped = reader.fmap_option(|value| value * 2);
    /// assert_eq!(mapped.run(21), Some(42));
    /// ```
    pub fn fmap_option<B, F>(self, function: F) -> ReaderT<R, Option<B>>
    where
        F: Fn(A) -> B + 'static,
        B: 'static,
    {
        let original = self.run_function;
        ReaderT::new(move |environment| (original)(environment).map(&function))
    }

    /// Chains `ReaderT` computations with Option.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes the value and returns a new `ReaderT`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ReaderT;
    ///
    /// let reader: ReaderT<i32, Option<i32>> = ReaderT::new(|environment| Some(environment));
    /// let chained = reader.flat_map_option(|value| {
    ///     ReaderT::new(move |environment| Some(value + environment))
    /// });
    /// assert_eq!(chained.run(10), Some(20)); // 10 + 10
    /// ```
    pub fn flat_map_option<B, F>(self, function: F) -> ReaderT<R, Option<B>>
    where
        F: Fn(A) -> ReaderT<R, Option<B>> + 'static,
        B: 'static,
        R: Clone,
    {
        let original = self.run_function;
        ReaderT::new(move |environment: R| {
            (original)(environment.clone()).and_then(|value| {
                let next = function(value);
                next.run(environment)
            })
        })
    }

    /// Returns the environment wrapped in Some.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ReaderT;
    ///
    /// let reader: ReaderT<i32, Option<i32>> = ReaderT::ask_option();
    /// assert_eq!(reader.run(42), Some(42));
    /// ```
    #[must_use]
    pub fn ask_option() -> Self
    where
        R: Clone,
        A: From<R>,
    {
        Self::new(|environment: R| Some(A::from(environment)))
    }

    /// Runs a computation with a modified environment.
    ///
    /// # Arguments
    ///
    /// * `modifier` - A function that transforms the environment
    /// * `computation` - The computation to run with the modified environment
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ReaderT;
    ///
    /// let reader: ReaderT<i32, Option<i32>> = ReaderT::new(|environment| Some(environment * 2));
    /// let modified = ReaderT::local_option(|environment| environment + 10, reader);
    /// assert_eq!(modified.run(5), Some(30)); // (5 + 10) * 2
    /// ```
    pub fn local_option<F>(modifier: F, computation: Self) -> Self
    where
        F: Fn(R) -> R + 'static,
    {
        let computation_function = computation.run_function;
        Self::new(move |environment| {
            let modified_environment = modifier(environment);
            (computation_function)(modified_environment)
        })
    }
}

// Special implementation for when R = A (same type for ask)
impl<R> ReaderT<R, Option<R>>
where
    R: Clone + 'static,
{
    /// Returns the environment wrapped in Some.
    ///
    /// This is the ask operation for `MonadReader`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ReaderT;
    ///
    /// let reader: ReaderT<i32, Option<i32>> = ReaderT::ask_option();
    /// assert_eq!(reader.run(42), Some(42));
    /// ```
    #[allow(dead_code)]
    fn ask_option_same_type() -> Self {
        Self::new(|environment: R| Some(environment))
    }
}

// =============================================================================
// Result-specific Methods
// =============================================================================

impl<R, A, E> ReaderT<R, Result<A, E>>
where
    R: 'static,
    A: 'static,
    E: 'static,
{
    /// Creates a `ReaderT` that returns a constant value wrapped in Ok.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to wrap
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ReaderT;
    ///
    /// let reader: ReaderT<i32, Result<i32, String>> = ReaderT::pure_result(42);
    /// assert_eq!(reader.run(999), Ok(42));
    /// ```
    pub fn pure_result(value: A) -> Self
    where
        A: Clone,
    {
        Self::new(move |_| Ok(value.clone()))
    }

    /// Lifts a Result into `ReaderT`.
    ///
    /// # Arguments
    ///
    /// * `inner` - The Result to lift
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ReaderT;
    ///
    /// let inner: Result<i32, String> = Ok(42);
    /// let reader: ReaderT<i32, Result<i32, String>> = ReaderT::lift_result(inner);
    /// assert_eq!(reader.run(999), Ok(42));
    /// ```
    pub fn lift_result(inner: Result<A, E>) -> Self
    where
        A: Clone,
        E: Clone,
    {
        Self::new(move |_| inner.clone())
    }

    /// Maps a function over the value inside the Result.
    ///
    /// # Arguments
    ///
    /// * `function` - The function to apply to the value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ReaderT;
    ///
    /// let reader: ReaderT<i32, Result<i32, String>> = ReaderT::new(|environment| Ok(environment));
    /// let mapped = reader.fmap_result(|value| value * 2);
    /// assert_eq!(mapped.run(21), Ok(42));
    /// ```
    pub fn fmap_result<B, F>(self, function: F) -> ReaderT<R, Result<B, E>>
    where
        F: Fn(A) -> B + 'static,
        B: 'static,
    {
        let original = self.run_function;
        ReaderT::new(move |environment| (original)(environment).map(&function))
    }

    /// Chains `ReaderT` computations with Result.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes the value and returns a new `ReaderT`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ReaderT;
    ///
    /// let reader: ReaderT<i32, Result<i32, String>> = ReaderT::new(|environment| Ok(environment));
    /// let chained = reader.flat_map_result(|value| {
    ///     ReaderT::new(move |environment| Ok(value + environment))
    /// });
    /// assert_eq!(chained.run(10), Ok(20));
    /// ```
    pub fn flat_map_result<B, F>(self, function: F) -> ReaderT<R, Result<B, E>>
    where
        F: Fn(A) -> ReaderT<R, Result<B, E>> + 'static,
        B: 'static,
        R: Clone,
    {
        let original = self.run_function;
        ReaderT::new(
            move |environment: R| match (original)(environment.clone()) {
                Ok(value) => {
                    let next = function(value);
                    next.run(environment)
                }
                Err(error) => Err(error),
            },
        )
    }

    /// Returns the environment wrapped in Ok.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ReaderT;
    ///
    /// let reader: ReaderT<i32, Result<i32, String>> = ReaderT::ask_result();
    /// assert_eq!(reader.run(42), Ok(42));
    /// ```
    #[must_use]
    pub fn ask_result() -> Self
    where
        R: Clone,
        A: From<R>,
    {
        Self::new(|environment: R| Ok(A::from(environment)))
    }

    /// Runs a computation with a modified environment.
    ///
    /// # Arguments
    ///
    /// * `modifier` - A function that transforms the environment
    /// * `computation` - The computation to run with the modified environment
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::ReaderT;
    ///
    /// let reader: ReaderT<i32, Result<i32, String>> = ReaderT::new(|environment| Ok(environment * 2));
    /// let modified = ReaderT::local_result(|environment| environment + 10, reader);
    /// assert_eq!(modified.run(5), Ok(30));
    /// ```
    pub fn local_result<F>(modifier: F, computation: Self) -> Self
    where
        F: Fn(R) -> R + 'static,
    {
        let computation_function = computation.run_function;
        Self::new(move |environment| {
            let modified_environment = modifier(environment);
            (computation_function)(modified_environment)
        })
    }
}

// =============================================================================
// IO-specific Methods
// =============================================================================

impl<R, A> ReaderT<R, IO<A>>
where
    R: 'static,
    A: 'static,
{
    /// Creates a `ReaderT` that returns a constant value wrapped in `IO::pure`.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to wrap
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::{ReaderT, IO};
    ///
    /// let reader: ReaderT<i32, IO<i32>> = ReaderT::pure_io(42);
    /// let io = reader.run(999);
    /// assert_eq!(io.run_unsafe(), 42);
    /// ```
    pub fn pure_io(value: A) -> Self
    where
        A: Clone,
    {
        Self::new(move |_| IO::pure(value.clone()))
    }

    /// Lifts an IO into `ReaderT`.
    ///
    /// # Arguments
    ///
    /// * `inner` - The IO to lift
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::{ReaderT, IO};
    ///
    /// let inner = IO::pure(42);
    /// let reader: ReaderT<String, IO<i32>> = ReaderT::lift_io(inner);
    /// let io = reader.run("ignored".to_string());
    /// assert_eq!(io.run_unsafe(), 42);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the resulting `ReaderT` is run more than once.
    #[must_use]
    pub fn lift_io(inner: IO<A>) -> Self {
        // IO is not Clone, so we wrap in Rc
        let inner_rc = Rc::new(std::cell::RefCell::new(Some(inner)));
        Self::new(move |_| {
            // Take the IO from the RefCell (can only be called once)
            inner_rc.borrow_mut().take().unwrap_or_else(|| {
                panic!("ReaderT::lift_io: IO already consumed. Use the ReaderT only once.")
            })
        })
    }

    /// Maps a function over the value inside the IO.
    ///
    /// # Arguments
    ///
    /// * `function` - The function to apply to the value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::{ReaderT, IO};
    ///
    /// let reader: ReaderT<i32, IO<i32>> = ReaderT::new(|environment| IO::pure(environment));
    /// let mapped = reader.fmap_io(|value| value * 2);
    /// let io = mapped.run(21);
    /// assert_eq!(io.run_unsafe(), 42);
    /// ```
    pub fn fmap_io<B, F>(self, function: F) -> ReaderT<R, IO<B>>
    where
        F: Fn(A) -> B + 'static,
        B: 'static,
    {
        let original = self.run_function;
        let function_rc = Rc::new(function);
        ReaderT::new(move |environment| {
            let io = (original)(environment);
            let function_clone = function_rc.clone();
            io.fmap(move |value| function_clone(value))
        })
    }

    /// Chains `ReaderT` computations with IO.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes the value and returns a new `ReaderT`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::{ReaderT, IO};
    ///
    /// let reader: ReaderT<i32, IO<i32>> = ReaderT::new(|environment| IO::pure(environment));
    /// let chained = reader.flat_map_io(|value| {
    ///     ReaderT::new(move |environment| IO::pure(value + environment))
    /// });
    /// let io = chained.run(10);
    /// assert_eq!(io.run_unsafe(), 20);
    /// ```
    pub fn flat_map_io<B, F>(self, function: F) -> ReaderT<R, IO<B>>
    where
        F: Fn(A) -> ReaderT<R, IO<B>> + 'static,
        B: 'static,
        R: Clone,
    {
        let original = self.run_function;
        let function_rc = Rc::new(function);
        ReaderT::new(move |environment: R| {
            let environment_clone = environment.clone();
            let io = (original)(environment);
            let function_clone = function_rc.clone();
            io.flat_map(move |value| {
                let next = function_clone(value);
                next.run(environment_clone)
            })
        })
    }

    /// Returns the environment wrapped in `IO::pure`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::{ReaderT, IO};
    ///
    /// let reader: ReaderT<i32, IO<i32>> = ReaderT::ask_io();
    /// let io = reader.run(42);
    /// assert_eq!(io.run_unsafe(), 42);
    /// ```
    #[must_use]
    pub fn ask_io() -> Self
    where
        R: Clone,
        A: From<R>,
    {
        Self::new(|environment: R| IO::pure(A::from(environment)))
    }

    /// Runs a computation with a modified environment.
    ///
    /// # Arguments
    ///
    /// * `modifier` - A function that transforms the environment
    /// * `computation` - The computation to run with the modified environment
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::{ReaderT, IO};
    ///
    /// let reader: ReaderT<i32, IO<i32>> = ReaderT::new(|environment| IO::pure(environment * 2));
    /// let modified = ReaderT::local_io(|environment| environment + 10, reader);
    /// let io = modified.run(5);
    /// assert_eq!(io.run_unsafe(), 30);
    /// ```
    #[allow(dead_code)]
    pub fn local_io<F>(modifier: F, computation: Self) -> Self
    where
        F: Fn(R) -> R + 'static,
    {
        let computation_function = computation.run_function;
        Self::new(move |environment| {
            let modified_environment = modifier(environment);
            (computation_function)(modified_environment)
        })
    }
}

// =============================================================================
// AsyncIO-specific Methods (requires async feature)
// =============================================================================

#[cfg(feature = "async")]
use super::AsyncIO;

#[cfg(feature = "async")]
impl<R, A> ReaderT<R, AsyncIO<A>>
where
    R: 'static,
    A: Send + 'static,
{
    /// Creates a `ReaderT` that returns a constant value wrapped in `AsyncIO::pure`.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to wrap
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{ReaderT, AsyncIO};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let reader: ReaderT<i32, AsyncIO<i32>> = ReaderT::pure_async_io(42);
    ///     let async_io = reader.run(999);
    ///     assert_eq!(async_io.run_async().await, 42);
    /// }
    /// ```
    pub fn pure_async_io(value: A) -> Self
    where
        A: Clone,
    {
        Self::new(move |_| AsyncIO::pure(value.clone()))
    }

    /// Maps a function over the value inside the `AsyncIO`.
    ///
    /// # Arguments
    ///
    /// * `function` - The function to apply to the value
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{ReaderT, AsyncIO};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let reader: ReaderT<i32, AsyncIO<i32>> = ReaderT::new(|environment| AsyncIO::pure(environment));
    ///     let mapped = reader.fmap_async_io(|value| value * 2);
    ///     let async_io = mapped.run(21);
    ///     assert_eq!(async_io.run_async().await, 42);
    /// }
    /// ```
    pub fn fmap_async_io<B, F>(self, function: F) -> ReaderT<R, AsyncIO<B>>
    where
        F: Fn(A) -> B + Send + Sync + 'static,
        B: Send + 'static,
    {
        let original = self.run_function;
        let function_rc = std::sync::Arc::new(function);
        ReaderT::new(move |environment| {
            let async_io = (original)(environment);
            let function_clone = function_rc.clone();
            async_io.fmap(move |value| function_clone(value))
        })
    }

    /// Chains `ReaderT` computations with `AsyncIO`.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes the value and returns a new `ReaderT`
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{ReaderT, AsyncIO};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let reader: ReaderT<i32, AsyncIO<i32>> = ReaderT::new(|environment| AsyncIO::pure(environment));
    ///     let chained = reader.flat_map_async_io(|value| {
    ///         ReaderT::new(move |environment| AsyncIO::pure(value + environment))
    ///     });
    ///     let async_io = chained.run(10);
    ///     assert_eq!(async_io.run_async().await, 20);
    /// }
    /// ```
    pub fn flat_map_async_io<B, F>(self, function: F) -> ReaderT<R, AsyncIO<B>>
    where
        F: Fn(A) -> ReaderT<R, AsyncIO<B>> + Send + Sync + 'static,
        B: Send + 'static,
        R: Clone + Send,
    {
        let original = self.run_function;
        let function_arc = std::sync::Arc::new(function);
        ReaderT::new(move |environment: R| {
            let environment_clone = environment.clone();
            let async_io = (original)(environment);
            let function_clone = function_arc.clone();
            async_io.flat_map(move |value| {
                let next = function_clone(value);
                next.run(environment_clone)
            })
        })
    }

    /// Returns the environment wrapped in `AsyncIO::pure`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{ReaderT, AsyncIO};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let reader: ReaderT<i32, AsyncIO<i32>> = ReaderT::ask_async_io();
    ///     let async_io = reader.run(42);
    ///     assert_eq!(async_io.run_async().await, 42);
    /// }
    /// ```
    #[must_use]
    pub fn ask_async_io() -> Self
    where
        R: Clone + Send,
        A: From<R>,
    {
        Self::new(|environment: R| AsyncIO::pure(A::from(environment)))
    }

    /// Runs a computation with a modified environment.
    ///
    /// # Arguments
    ///
    /// * `modifier` - A function that transforms the environment
    /// * `computation` - The computation to run with the modified environment
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{ReaderT, AsyncIO};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let reader: ReaderT<i32, AsyncIO<i32>> = ReaderT::new(|environment| AsyncIO::pure(environment * 2));
    ///     let modified = ReaderT::local_async_io(|environment| environment + 10, reader);
    ///     let async_io = modified.run(5);
    ///     assert_eq!(async_io.run_async().await, 30);
    /// }
    /// ```
    pub fn local_async_io<F>(modifier: F, computation: Self) -> Self
    where
        F: Fn(R) -> R + 'static,
    {
        let computation_function = computation.run_function;
        Self::new(move |environment| {
            let modified_environment = modifier(environment);
            (computation_function)(modified_environment)
        })
    }

    /// Lifts an `AsyncIO` into `ReaderT`.
    ///
    /// The resulting `ReaderT` ignores the environment and returns the
    /// inner `AsyncIO` directly.
    ///
    /// # Important: Single Use Only
    ///
    /// The resulting `ReaderT` can only be run **once**. Running it multiple
    /// times will cause a panic. This is because `AsyncIO` is not `Clone`,
    /// so we cannot share the inner computation across multiple runs.
    ///
    /// # Arguments
    ///
    /// * `inner` - The `AsyncIO` to lift into `ReaderT`
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{ReaderT, AsyncIO};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let async_io = AsyncIO::pure(42);
    ///     let reader: ReaderT<i32, AsyncIO<i32>> = ReaderT::lift_async_io(async_io);
    ///     let result = reader.run(999).run_async().await;
    ///     assert_eq!(result, 42);
    /// }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the `ReaderT` is run more than once.
    #[must_use]
    pub fn lift_async_io(inner: AsyncIO<A>) -> Self
    where
        A: Clone,
    {
        let inner_arc = std::sync::Arc::new(std::sync::Mutex::new(Some(inner)));
        Self::new(move |_| {
            let mut guard = inner_arc.lock().unwrap();
            guard.take().unwrap_or_else(|| {
                panic!(
                    "ReaderT::lift_async_io: AsyncIO already consumed. Use the ReaderT only once."
                )
            })
        })
    }

    /// Projects a value from the environment and wraps it in `AsyncIO::pure`.
    ///
    /// This is a convenience method that combines `ask_async_io` with a projection
    /// function. It follows the Reader monad law: `asks f == ask.fmap(f)`.
    ///
    /// # Arguments
    ///
    /// * `projection` - A function that extracts or transforms a value from the environment
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{ReaderT, AsyncIO};
    ///
    /// #[derive(Clone)]
    /// struct Config { multiplier: i32 }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let reader: ReaderT<Config, AsyncIO<i32>> =
    ///         ReaderT::asks_async_io(|config: Config| config.multiplier * 2);
    ///     let result = reader.run(Config { multiplier: 21 }).run_async().await;
    ///     assert_eq!(result, 42);
    /// }
    /// ```
    #[must_use]
    pub fn asks_async_io<F>(projection: F) -> Self
    where
        R: Clone + Send,
        F: Fn(R) -> A + Send + Sync + 'static,
    {
        Self::new(move |environment: R| AsyncIO::pure(projection(environment)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reader_transformer_new_and_run() {
        let reader: ReaderT<i32, Option<i32>> = ReaderT::new(|environment| Some(environment * 2));
        assert_eq!(reader.run(21), Some(42));
    }

    #[test]
    fn reader_transformer_clone() {
        let reader: ReaderT<i32, Option<i32>> = ReaderT::new(|environment| Some(environment * 2));
        let cloned = reader.clone();
        assert_eq!(reader.run(21), Some(42));
        assert_eq!(cloned.run(21), Some(42));
    }

    #[test]
    fn reader_transformer_pure_option() {
        let reader: ReaderT<i32, Option<i32>> = ReaderT::pure_option(42);
        assert_eq!(reader.run(999), Some(42));
    }

    #[test]
    fn reader_transformer_lift_option() {
        let reader: ReaderT<i32, Option<i32>> = ReaderT::lift_option(Some(42));
        assert_eq!(reader.run(999), Some(42));
    }

    #[test]
    fn reader_transformer_fmap_option() {
        let reader: ReaderT<i32, Option<i32>> = ReaderT::new(Some);
        let mapped = reader.fmap_option(|value| value * 2);
        assert_eq!(mapped.run(21), Some(42));
    }

    #[test]
    fn reader_transformer_flat_map_option() {
        let reader: ReaderT<i32, Option<i32>> = ReaderT::new(Some);
        let chained = reader
            .flat_map_option(|value| ReaderT::new(move |environment| Some(value + environment)));
        assert_eq!(chained.run(10), Some(20));
    }

    // =========================================================================
    // AsyncIO-specific Tests (requires async feature)
    // =========================================================================

    #[cfg(feature = "async")]
    mod async_io_tests {
        use super::*;

        #[tokio::test]
        async fn reader_lift_async_io_ignores_environment() {
            let async_io = AsyncIO::pure(42);
            let reader: ReaderT<i32, AsyncIO<i32>> = ReaderT::lift_async_io(async_io);
            let result = reader.run(999).run_async().await;
            assert_eq!(result, 42);
        }

        #[tokio::test]
        async fn reader_lift_async_io_preserves_value() {
            let async_io = AsyncIO::new(|| async { "hello".to_string() });
            let reader: ReaderT<(), AsyncIO<String>> = ReaderT::lift_async_io(async_io);
            let result = reader.run(()).run_async().await;
            assert_eq!(result, "hello");
        }

        #[tokio::test]
        async fn reader_lift_pure_law() {
            let value = 42;
            let via_lift: ReaderT<(), AsyncIO<i32>> = ReaderT::lift_async_io(AsyncIO::pure(value));
            let via_pure: ReaderT<(), AsyncIO<i32>> = ReaderT::pure_async_io(value);

            assert_eq!(
                via_lift.run(()).run_async().await,
                via_pure.run(()).run_async().await
            );
        }

        #[tokio::test]
        async fn reader_asks_async_io_projects_value() {
            #[derive(Clone)]
            struct Env {
                value: i32,
            }

            let reader: ReaderT<Env, AsyncIO<i32>> =
                ReaderT::asks_async_io(|environment: Env| environment.value * 2);
            let result = reader.run(Env { value: 21 }).run_async().await;
            assert_eq!(result, 42);
        }

        #[tokio::test]
        async fn reader_ask_asks_law() {
            let projection = |x: i32| x * 3;

            // asks_async_io を使用
            let reader1: ReaderT<i32, AsyncIO<i32>> = ReaderT::asks_async_io(projection);
            let result1 = reader1.run(10).run_async().await;

            // ask_async_io().fmap_async_io() を使用
            let reader2: ReaderT<i32, AsyncIO<i32>> =
                ReaderT::<i32, AsyncIO<i32>>::ask_async_io().fmap_async_io(projection);
            let result2 = reader2.run(10).run_async().await;

            assert_eq!(result1, result2);
        }
    }
}
