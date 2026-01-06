//! `AsyncIO` Monad - Deferred asynchronous side effect handling.
//!
//! The `AsyncIO` type represents an asynchronous computation that may perform
//! side effects. Side effects are not executed until `run_async` is called,
//! maintaining referential transparency in pure code.
//!
//! # Design Philosophy
//!
//! `AsyncIO` "describes" async side effects but doesn't "execute" them. Execution
//! happens only via `run_async().await`, which should be called at the program's
//! "edge" (e.g., in async handlers or the main function).
//!
//! # Examples
//!
//! ```rust,ignore
//! use lambars::effect::AsyncIO;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create a pure AsyncIO action
//!     let async_io = AsyncIO::pure(42);
//!     assert_eq!(async_io.run_async().await, 42);
//!
//!     // Chain AsyncIO actions
//!     let async_io = AsyncIO::pure(10)
//!         .fmap(|x| x * 2)
//!         .flat_map(|x| AsyncIO::pure(x + 1));
//!     assert_eq!(async_io.run_async().await, 21);
//! }
//! ```
//!
//! # Side Effect Deferral
//!
//! ```rust,ignore
//! use lambars::effect::AsyncIO;
//! use std::sync::atomic::{AtomicBool, Ordering};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() {
//!     let executed = Arc::new(AtomicBool::new(false));
//!     let executed_clone = executed.clone();
//!
//!     let async_io = AsyncIO::new(move || {
//!         let flag = executed_clone.clone();
//!         async move {
//!             flag.store(true, Ordering::SeqCst);
//!             42
//!         }
//!     });
//!
//!     // Not executed yet
//!     assert!(!executed.load(Ordering::SeqCst));
//!
//!     // Execute the AsyncIO action
//!     let result = async_io.run_async().await;
//!     assert!(executed.load(Ordering::SeqCst));
//!     assert_eq!(result, 42);
//! }
//! ```

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use crate::control::Either;

/// A monad representing deferred asynchronous side effects.
///
/// `AsyncIO<A>` wraps an asynchronous computation that produces a value of type `A`
/// and may perform side effects. The computation is not executed until `run_async`
/// is called.
///
/// # Type Parameters
///
/// - `A`: The type of the value produced by the async IO action.
///
/// # Monad Laws
///
/// `AsyncIO` satisfies the monad laws:
///
/// 1. **Left Identity**: `AsyncIO::pure(a).flat_map(f) == f(a)`
/// 2. **Right Identity**: `m.flat_map(AsyncIO::pure) == m`
/// 3. **Associativity**: `m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))`
///
/// # Examples
///
/// ```rust,ignore
/// use lambars::effect::AsyncIO;
///
/// #[tokio::main]
/// async fn main() {
///     let async_io = AsyncIO::pure(42);
///     let result = async_io.run_async().await;
///     assert_eq!(result, 42);
/// }
/// ```
pub struct AsyncIO<A> {
    /// The wrapped async computation that produces a value of type `A`.
    run_async_io: Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = A> + Send>> + Send>,
}

// =============================================================================
// Constructors
// =============================================================================

impl<A: 'static> AsyncIO<A> {
    /// Creates a new `AsyncIO` action from an async closure.
    ///
    /// The closure will not be executed until `run_async` is called.
    ///
    /// # Arguments
    ///
    /// * `action` - A closure that returns a Future producing a value of type `A`.
    ///
    /// # Type Parameters
    ///
    /// * `F` - The type of the closure.
    /// * `Fut` - The type of the Future returned by the closure.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    ///
    /// let async_io = AsyncIO::new(|| async {
    ///     tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    ///     42
    /// });
    /// ```
    pub fn new<F, Fut>(action: F) -> Self
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = A> + Send + 'static,
    {
        Self {
            run_async_io: Box::new(move || Box::pin(action())),
        }
    }

    /// Creates an `AsyncIO` from an existing Future.
    ///
    /// The Future should not have been polled yet.
    ///
    /// # Arguments
    ///
    /// * `future` - A Future producing a value of type `A`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    ///
    /// let future = async { 42 };
    /// let async_io = AsyncIO::from_future(future);
    /// ```
    pub fn from_future<Fut>(future: Fut) -> Self
    where
        Fut: Future<Output = A> + Send + 'static,
    {
        Self {
            run_async_io: Box::new(move || Box::pin(future)),
        }
    }
}

impl<A: Send + 'static> AsyncIO<A> {
    /// Wraps a pure value in an `AsyncIO` action.
    ///
    /// This creates an `AsyncIO` action that returns the given value without
    /// performing any side effects.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to wrap.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    ///
    /// let async_io = AsyncIO::pure(42);
    /// // run_async().await will immediately return 42
    /// ```
    pub fn pure(value: A) -> Self {
        Self {
            run_async_io: Box::new(move || Box::pin(async move { value })),
        }
    }
}

// =============================================================================
// Execution Methods
// =============================================================================

impl<A: 'static> AsyncIO<A> {
    /// Executes the `AsyncIO` action and returns the result.
    ///
    /// This is the only way to extract a value from an `AsyncIO` action.
    /// It should be called at the program's "edge" (e.g., in async handlers
    /// or the main function).
    ///
    /// # Safety Note
    ///
    /// This method executes side effects. While it's memory-safe, calling it
    /// breaks referential transparency.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let async_io = AsyncIO::pure(42);
    ///     let result = async_io.run_async().await;
    ///     assert_eq!(result, 42);
    /// }
    /// ```
    pub async fn run_async(self) -> A {
        (self.run_async_io)().await
    }

    /// Converts the `AsyncIO` into a Future.
    ///
    /// This is useful when you need to pass the computation to functions
    /// that expect a Future, such as `tokio::spawn`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    ///
    /// let async_io = AsyncIO::pure(42);
    /// let future = async_io.into_future();
    /// tokio::spawn(future);
    /// ```
    pub async fn into_future(self) -> A
    where
        A: Send,
    {
        self.run_async().await
    }
}

// =============================================================================
// Functor Operations
// =============================================================================

impl<A: 'static> AsyncIO<A> {
    /// Transforms the result of an `AsyncIO` action using a function.
    ///
    /// This is the `fmap` operation from Functor.
    ///
    /// # Arguments
    ///
    /// * `function` - A function to apply to the result.
    ///
    /// # Type Parameters
    ///
    /// * `B` - The return type of the function.
    /// * `F` - The type of the function.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    ///
    /// let async_io = AsyncIO::pure(21).fmap(|x| x * 2);
    /// assert_eq!(async_io.run_async().await, 42);
    /// ```
    pub fn fmap<B, F>(self, function: F) -> AsyncIO<B>
    where
        F: FnOnce(A) -> B + Send + 'static,
        B: 'static,
    {
        AsyncIO::new(move || async move {
            let value = self.run_async().await;
            function(value)
        })
    }
}

// =============================================================================
// Applicative Operations
// =============================================================================

impl<A: 'static> AsyncIO<A> {
    /// Applies an AsyncIO-wrapped function to this `AsyncIO` value.
    ///
    /// # Arguments
    ///
    /// * `function_async_io` - An `AsyncIO` containing a function.
    ///
    /// # Type Parameters
    ///
    /// * `B` - The return type of the function.
    /// * `F` - The type of the function.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    ///
    /// let function_io = AsyncIO::pure(|x: i32| x * 2);
    /// let value_io = AsyncIO::pure(21);
    /// let result = value_io.apply(function_io).run_async().await;
    /// assert_eq!(result, 42);
    /// ```
    #[must_use]
    pub fn apply<B, F>(self, function_async_io: AsyncIO<F>) -> AsyncIO<B>
    where
        F: FnOnce(A) -> B + Send + 'static,
        B: 'static,
    {
        AsyncIO::new(move || async move {
            let function = function_async_io.run_async().await;
            let value = self.run_async().await;
            function(value)
        })
    }

    /// Combines two `AsyncIO` actions using a function.
    ///
    /// Both computations are executed sequentially, and their results
    /// are combined using the provided function.
    ///
    /// # Arguments
    ///
    /// * `other` - The second `AsyncIO` action.
    /// * `function` - A function to combine the results.
    ///
    /// # Type Parameters
    ///
    /// * `B` - The type of the second `AsyncIO`'s value.
    /// * `C` - The return type of the combining function.
    /// * `F` - The type of the combining function.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    ///
    /// let io1 = AsyncIO::pure(10);
    /// let io2 = AsyncIO::pure(20);
    /// let combined = io1.map2(io2, |a, b| a + b);
    /// assert_eq!(combined.run_async().await, 30);
    /// ```
    pub fn map2<B, C, F>(self, other: AsyncIO<B>, function: F) -> AsyncIO<C>
    where
        A: Send,
        F: FnOnce(A, B) -> C + Send + 'static,
        B: Send + 'static,
        C: 'static,
    {
        AsyncIO::new(move || async move {
            let value_a = self.run_async().await;
            let value_b = other.run_async().await;
            function(value_a, value_b)
        })
    }
}

impl<A: Send + 'static> AsyncIO<A> {
    /// Combines two `AsyncIO` actions into a tuple.
    ///
    /// # Arguments
    ///
    /// * `other` - The second `AsyncIO` action.
    ///
    /// # Type Parameters
    ///
    /// * `B` - The type of the second `AsyncIO`'s value.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    ///
    /// let io1 = AsyncIO::pure(10);
    /// let io2 = AsyncIO::pure(20);
    /// let result = io1.product(io2).run_async().await;
    /// assert_eq!(result, (10, 20));
    /// ```
    #[must_use]
    pub fn product<B>(self, other: AsyncIO<B>) -> AsyncIO<(A, B)>
    where
        B: Send + 'static,
    {
        self.map2(other, |a, b| (a, b))
    }
}

// =============================================================================
// Monad Operations
// =============================================================================

impl<A: 'static> AsyncIO<A> {
    /// Chains `AsyncIO` actions, passing the result of the first to a function
    /// that produces the second.
    ///
    /// This is the `bind` operation from Monad.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes the result and returns a new `AsyncIO` action.
    ///
    /// # Type Parameters
    ///
    /// * `B` - The type of the second `AsyncIO`'s value.
    /// * `F` - The type of the function.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    ///
    /// let async_io = AsyncIO::pure(10).flat_map(|x| AsyncIO::pure(x * 2));
    /// assert_eq!(async_io.run_async().await, 20);
    /// ```
    pub fn flat_map<B, F>(self, function: F) -> AsyncIO<B>
    where
        F: FnOnce(A) -> AsyncIO<B> + Send + 'static,
        B: 'static,
    {
        AsyncIO::new(move || async move {
            let value_a = self.run_async().await;
            let async_io_b = function(value_a);
            async_io_b.run_async().await
        })
    }

    /// Alias for `flat_map`.
    ///
    /// This is the conventional Rust name for monadic bind.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    ///
    /// let async_io = AsyncIO::pure(10).and_then(|x| AsyncIO::pure(x + 5));
    /// assert_eq!(async_io.run_async().await, 15);
    /// ```
    pub fn and_then<B, F>(self, function: F) -> AsyncIO<B>
    where
        F: FnOnce(A) -> AsyncIO<B> + Send + 'static,
        B: 'static,
    {
        self.flat_map(function)
    }

    /// Sequences two `AsyncIO` actions, discarding the result of the first.
    ///
    /// The first action is still executed for its side effects.
    ///
    /// # Arguments
    ///
    /// * `next` - The `AsyncIO` action to execute after this one.
    ///
    /// # Type Parameters
    ///
    /// * `B` - The type of the second `AsyncIO`'s value.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    ///
    /// let async_io = AsyncIO::pure(10).then(AsyncIO::pure(20));
    /// assert_eq!(async_io.run_async().await, 20);
    /// ```
    #[must_use]
    pub fn then<B>(self, next: AsyncIO<B>) -> AsyncIO<B>
    where
        B: 'static,
    {
        self.flat_map(move |_| next)
    }
}

// =============================================================================
// Utility Methods
// =============================================================================

impl AsyncIO<()> {
    /// Creates an `AsyncIO` action that waits for a specified duration.
    ///
    /// The delay does not occur until `run_async` is called.
    ///
    /// # Arguments
    ///
    /// * `duration` - How long to wait.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    /// use std::time::Duration;
    ///
    /// let async_io = AsyncIO::delay_async(Duration::from_millis(100));
    /// async_io.run_async().await; // Waits for 100ms
    /// ```
    #[must_use]
    pub fn delay_async(duration: Duration) -> Self {
        Self::new(move || async move {
            tokio::time::sleep(duration).await;
        })
    }
}

impl<A: 'static> AsyncIO<A> {
    /// Returns the result if completed within the timeout, otherwise None.
    ///
    /// # Arguments
    ///
    /// * `duration` - The maximum time to wait.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    /// use std::time::Duration;
    ///
    /// let async_io = AsyncIO::pure(42).timeout(Duration::from_millis(100));
    /// assert_eq!(async_io.run_async().await, Some(42));
    ///
    /// let slow = AsyncIO::delay_async(Duration::from_secs(10))
    ///     .timeout(Duration::from_millis(100));
    /// assert_eq!(slow.run_async().await, None);
    /// ```
    #[must_use]
    pub fn timeout(self, duration: Duration) -> AsyncIO<Option<A>>
    where
        A: Send,
    {
        AsyncIO::new(move || async move {
            (tokio::time::timeout(duration, self.run_async()).await).ok()
        })
    }
}

// =============================================================================
// Timeout Error Type
// =============================================================================

/// Error type representing a timeout.
///
/// Contains information about the timeout duration that was exceeded.
///
/// # Examples
///
/// ```rust,ignore
/// use lambars::effect::TimeoutError;
/// use std::time::Duration;
///
/// let error = TimeoutError {
///     duration: Duration::from_secs(5),
/// };
/// println!("Timeout: {}", error);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeoutError {
    /// The timeout duration that was exceeded.
    pub duration: Duration,
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "operation timed out after {:?}", self.duration)
    }
}

impl std::error::Error for TimeoutError {}

// =============================================================================
// Timeout Result Extension
// =============================================================================

impl<A: 'static> AsyncIO<A> {
    /// Returns a `Result` with the value if completed within the timeout,
    /// otherwise returns a `TimeoutError`.
    ///
    /// Unlike `timeout` which returns `Option<A>`, this method provides
    /// more detailed error information.
    ///
    /// # Arguments
    ///
    /// * `duration` - The maximum time to wait.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{AsyncIO, TimeoutError};
    /// use std::time::Duration;
    ///
    /// let async_io = AsyncIO::pure(42).timeout_result(Duration::from_millis(100));
    /// assert_eq!(async_io.run_async().await, Ok(42));
    ///
    /// let slow = AsyncIO::delay_async(Duration::from_secs(10))
    ///     .timeout_result(Duration::from_millis(100));
    /// match slow.run_async().await {
    ///     Err(e) => assert_eq!(e.duration, Duration::from_millis(100)),
    ///     Ok(_) => panic!("should have timed out"),
    /// }
    /// ```
    #[must_use]
    pub fn timeout_result(self, duration: Duration) -> AsyncIO<Result<A, TimeoutError>>
    where
        A: Send,
    {
        AsyncIO::new(move || async move {
            tokio::time::timeout(duration, self.run_async())
                .await
                .map_err(|_| TimeoutError { duration })
        })
    }
}

// =============================================================================
// Retry Operations
// =============================================================================

impl<A: Send + 'static> AsyncIO<A> {
    /// Creates a retryable `AsyncIO` action using a factory function.
    ///
    /// Since `AsyncIO` is consumed on execution, we need a factory that can
    /// create new instances for each retry attempt.
    ///
    /// # Type Parameters
    ///
    /// * `E` - The error type
    /// * `F` - A factory function that creates `AsyncIO<Result<A, E>>`
    ///
    /// # Arguments
    ///
    /// * `factory` - A function that creates a new `AsyncIO` for each attempt
    /// * `max_attempts` - Maximum number of retry attempts
    ///
    /// # Returns
    ///
    /// An `AsyncIO<Result<A, E>>` that will retry on failure.
    ///
    /// # Behavior
    ///
    /// - If `Result` is `Ok`, returns immediately without retry
    /// - If `Result` is `Err`, retries up to `max_attempts` times
    /// - If all attempts fail, returns the last error
    /// - If `max_attempts` is 0, executes only once (no retry)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    /// use std::sync::atomic::{AtomicUsize, Ordering};
    /// use std::sync::Arc;
    ///
    /// let counter = Arc::new(AtomicUsize::new(0));
    /// let counter_clone = counter.clone();
    ///
    /// let result = AsyncIO::retry_with_factory(
    ///     move || {
    ///         let c = counter_clone.clone();
    ///         AsyncIO::new(move || {
    ///             let c = c.clone();
    ///             async move {
    ///                 if c.fetch_add(1, Ordering::SeqCst) < 2 {
    ///                     Err("fail")
    ///                 } else {
    ///                     Ok(42)
    ///                 }
    ///             }
    ///         })
    ///     },
    ///     5,
    /// );
    /// ```
    #[allow(clippy::missing_panics_doc)]
    pub fn retry_with_factory<E, F>(factory: F, max_attempts: usize) -> AsyncIO<Result<A, E>>
    where
        F: Fn() -> AsyncIO<Result<A, E>> + Send + 'static,
        E: Send + 'static,
    {
        let effective_attempts = max_attempts.max(1);

        AsyncIO::new(move || async move {
            let mut last_error: Option<E> = None;

            for _ in 0..effective_attempts {
                let action = factory();
                match action.run_async().await {
                    Ok(value) => return Ok(value),
                    Err(error) => {
                        last_error = Some(error);
                    }
                }
            }

            // All attempts failed, return the last error
            Err(last_error.expect("At least one attempt should have been made"))
        })
    }

    /// Retries with exponential backoff using a factory function.
    ///
    /// Before each retry (i.e., before attempts `2..=max_attempts`), the delay is
    /// `initial_delay * 2^(attempt - 1)`, where `attempt` is the 1-based attempt number.
    ///
    /// # Type Parameters
    ///
    /// * `E` - The error type
    /// * `F` - A factory function that creates `AsyncIO<Result<A, E>>`
    ///
    /// # Arguments
    ///
    /// * `factory` - A function that creates a new `AsyncIO` for each attempt
    /// * `max_attempts` - Maximum number of retry attempts
    /// * `initial_delay` - Initial delay before the first retry
    ///
    /// # Behavior
    ///
    /// - First attempt: no delay
    /// - Second attempt: `initial_delay`
    /// - Third attempt: `initial_delay * 2`
    /// - Fourth attempt: `initial_delay * 4`
    /// - And so on...
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    /// use std::time::Duration;
    ///
    /// let result = AsyncIO::retry_with_backoff_factory(
    ///     || AsyncIO::pure(Err::<i32, _>("error")),
    ///     3,
    ///     Duration::from_millis(100),
    /// );
    /// // Delays: 100ms before 2nd attempt, 200ms before 3rd attempt
    /// ```
    #[allow(clippy::missing_panics_doc)]
    pub fn retry_with_backoff_factory<E, F>(
        factory: F,
        max_attempts: usize,
        initial_delay: Duration,
    ) -> AsyncIO<Result<A, E>>
    where
        F: Fn() -> AsyncIO<Result<A, E>> + Send + 'static,
        E: Send + 'static,
    {
        let effective_attempts = max_attempts.max(1);

        AsyncIO::new(move || async move {
            let mut last_error: Option<E> = None;

            for attempt in 0..effective_attempts {
                // Apply backoff delay before retry (not on first attempt)
                if attempt > 0 {
                    let exponent = u32::try_from(attempt.saturating_sub(1)).unwrap_or(u32::MAX);
                    let delay_multiplier = 2u32.saturating_pow(exponent);
                    let delay = initial_delay.saturating_mul(delay_multiplier);
                    tokio::time::sleep(delay).await;
                }

                let action = factory();
                match action.run_async().await {
                    Ok(value) => return Ok(value),
                    Err(error) => {
                        last_error = Some(error);
                    }
                }
            }

            Err(last_error.expect("At least one attempt should have been made"))
        })
    }
}

// =============================================================================
// Parallel Execution
// =============================================================================

impl<A: Send + 'static> AsyncIO<A> {
    /// Executes two `AsyncIO` actions in parallel and returns both results as a tuple.
    ///
    /// This uses `tokio::join!` to run both futures concurrently.
    ///
    /// # Arguments
    ///
    /// * `other` - The second `AsyncIO` action to run in parallel.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    ///
    /// let a = AsyncIO::pure(1);
    /// let b = AsyncIO::pure(2);
    /// let (x, y) = a.par(b).run_async().await;
    /// assert_eq!((x, y), (1, 2));
    /// ```
    #[must_use]
    pub fn par<B>(self, other: AsyncIO<B>) -> AsyncIO<(A, B)>
    where
        B: Send + 'static,
    {
        AsyncIO::new(move || async move { tokio::join!(self.run_async(), other.run_async()) })
    }

    /// Executes three `AsyncIO` actions in parallel and returns all results as a tuple.
    ///
    /// # Arguments
    ///
    /// * `second` - The second `AsyncIO` action.
    /// * `third` - The third `AsyncIO` action.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    ///
    /// let a = AsyncIO::pure(1);
    /// let b = AsyncIO::pure(2);
    /// let c = AsyncIO::pure(3);
    /// let (x, y, z) = a.par3(b, c).run_async().await;
    /// assert_eq!((x, y, z), (1, 2, 3));
    /// ```
    #[must_use]
    pub fn par3<B, C>(self, second: AsyncIO<B>, third: AsyncIO<C>) -> AsyncIO<(A, B, C)>
    where
        B: Send + 'static,
        C: Send + 'static,
    {
        AsyncIO::new(move || async move {
            tokio::join!(self.run_async(), second.run_async(), third.run_async())
        })
    }

    /// Races two `AsyncIO` actions of the same type, returning whichever completes first.
    ///
    /// The slower computation is cancelled when the faster one completes.
    ///
    /// # Arguments
    ///
    /// * `other` - The second `AsyncIO` action to race against.
    ///
    /// # Note
    ///
    /// Unlike `race` which returns `Either<A, B>`, this method requires both
    /// `AsyncIO` actions to have the same result type and returns the result directly.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    /// use std::time::Duration;
    ///
    /// let slow = AsyncIO::delay_async(Duration::from_millis(100)).fmap(|_| 1);
    /// let fast = AsyncIO::pure(2);
    ///
    /// let result = slow.race_result(fast).run_async().await;
    /// assert_eq!(result, 2); // fast wins
    /// ```
    #[must_use]
    pub fn race_result(self, other: Self) -> Self {
        Self::new(move || async move {
            tokio::select! {
                result = self.run_async() => result,
                result = other.run_async() => result,
            }
        })
    }
}

// =============================================================================
// Resource Management
// =============================================================================

impl<A: 'static> AsyncIO<A> {
    /// Safely acquires, uses, and releases a resource.
    ///
    /// This is the bracket pattern from functional programming, ensuring that
    /// the resource is released even if the use function fails or panics.
    ///
    /// # Type Parameters
    ///
    /// * `Resource` - The type of the resource being managed
    /// * `Acquire` - The function type for acquiring the resource
    /// * `Use` - The function type for using the resource
    /// * `Release` - The function type for releasing the resource
    ///
    /// # Arguments
    ///
    /// * `acquire` - A function that creates an `AsyncIO` to acquire the resource
    /// * `use_resource` - A function that uses the resource and returns an `AsyncIO`
    /// * `release` - A function that releases the resource
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    ///
    /// let result = AsyncIO::bracket(
    ///     || AsyncIO::pure(42),           // acquire
    ///     |r| AsyncIO::pure(r * 2),       // use
    ///     |_| AsyncIO::pure(()),          // release
    /// );
    /// assert_eq!(result.run_async().await, 84);
    /// ```
    pub fn bracket<Resource, Acquire, Use, Release>(
        acquire: Acquire,
        use_resource: Use,
        release: Release,
    ) -> Self
    where
        Acquire: FnOnce() -> AsyncIO<Resource> + Send + 'static,
        Use: FnOnce(Resource) -> Self + Send + 'static,
        Release: FnOnce(Resource) -> AsyncIO<()> + Send + 'static,
        Resource: Clone + Send + 'static,
        A: Send,
    {
        Self::new(move || async move {
            use futures::FutureExt;
            use std::panic::AssertUnwindSafe;

            // 1. Acquire the resource
            let resource = acquire().run_async().await;
            let resource_for_release = resource.clone();

            // 2. Use the resource, catching any panics
            let result = AssertUnwindSafe(use_resource(resource).run_async())
                .catch_unwind()
                .await;

            // 3. Release the resource (always executed), also catching panics
            let release_result = AssertUnwindSafe(release(resource_for_release).run_async())
                .catch_unwind()
                .await;

            // 4. Return the result or re-panic, ensuring the original panic is preserved
            match (result, release_result) {
                (Ok(value), Ok(())) => value,
                (Err(original_panic), Ok(())) => std::panic::resume_unwind(original_panic),
                (Ok(_), Err(release_panic)) => std::panic::resume_unwind(release_panic),
                (Err(original_panic), Err(_release_panic)) => {
                    // Suppress release panic in favor of original panic
                    eprintln!(
                        "AsyncIO::bracket: panic in release while unwinding original panic; \
                         suppressing release panic in favor of original panic"
                    );
                    std::panic::resume_unwind(original_panic)
                }
            }
        })
    }
}

impl<A: Send + 'static> AsyncIO<A> {
    /// Ensures a cleanup action is always executed after this `AsyncIO`,
    /// regardless of success or failure.
    ///
    /// Similar to `finally` in Java/JavaScript.
    ///
    /// # Type Parameters
    ///
    /// * `F` - The cleanup function type
    /// * `Cleanup` - The cleanup Future type
    ///
    /// # Arguments
    ///
    /// * `cleanup` - A function that returns a Future for cleanup
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    ///
    /// let operation = AsyncIO::pure(42)
    ///     .finally_async(|| async { println!("cleanup"); });
    /// ```
    #[must_use]
    pub fn finally_async<F, Cleanup>(self, cleanup: F) -> Self
    where
        F: FnOnce() -> Cleanup + Send + 'static,
        Cleanup: std::future::Future<Output = ()> + Send + 'static,
    {
        Self::new(move || async move {
            use futures::FutureExt;
            use std::panic::AssertUnwindSafe;

            // Execute self, catching any panics
            let result = AssertUnwindSafe(self.run_async()).catch_unwind().await;

            // Always run cleanup, but don't let a cleanup panic
            // suppress the original result/panic.
            let cleanup_result = AssertUnwindSafe(cleanup()).catch_unwind().await;

            if cleanup_result.is_err() {
                eprintln!("AsyncIO::finally_async: cleanup panicked");
            }

            // Return the result or re-panic
            match result {
                Ok(value) => value,
                Err(panic_info) => std::panic::resume_unwind(panic_info),
            }
        })
    }
}

impl<A, E> AsyncIO<Result<A, E>>
where
    A: Send + 'static,
    E: Send + 'static,
{
    /// Executes a callback when this `AsyncIO` returns an error.
    ///
    /// The error is still propagated after the callback executes.
    /// Useful for logging or metrics.
    ///
    /// # Type Parameters
    ///
    /// * `F` - The callback function type
    /// * `Callback` - The callback Future type
    ///
    /// # Arguments
    ///
    /// * `callback` - A function that receives the error and returns a Future
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    ///
    /// let action: AsyncIO<Result<i32, String>> = AsyncIO::pure(Err("error".to_string()));
    /// let with_logging = action.on_error(|e| async move {
    ///     eprintln!("Error occurred: {}", e);
    /// });
    /// ```
    #[must_use]
    pub fn on_error<F, Callback>(self, callback: F) -> Self
    where
        F: FnOnce(&E) -> Callback + Send + 'static,
        Callback: std::future::Future<Output = ()> + Send + 'static,
    {
        Self::new(move || async move {
            let result = self.run_async().await;

            if let Err(ref error) = result {
                callback(error).await;
            }

            result
        })
    }
}

impl<A: Send + 'static> AsyncIO<A> {
    /// Races two `AsyncIO` actions, returning whichever completes first.
    ///
    /// The result is wrapped in `Either`: `Left` if the first completes first,
    /// `Right` if the second completes first.
    ///
    /// # Arguments
    ///
    /// * `other` - The second `AsyncIO` action to race against.
    ///
    /// # Type Parameters
    ///
    /// * `B` - The type of the second `AsyncIO`'s value.
    ///
    /// # Note
    ///
    /// The slower computation is cancelled when the faster one completes.
    /// This follows standard `tokio::select!` semantics.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    /// use lambars::control::Either;
    /// use std::time::Duration;
    ///
    /// let slow = AsyncIO::delay_async(Duration::from_millis(100)).fmap(|_| "slow");
    /// let fast = AsyncIO::pure("fast");
    ///
    /// let result = slow.race(fast).run_async().await;
    /// assert!(matches!(result, Either::Right("fast")));
    /// ```
    #[must_use]
    pub fn race<B>(self, other: AsyncIO<B>) -> AsyncIO<Either<A, B>>
    where
        B: Send + 'static,
    {
        AsyncIO::new(move || async move {
            tokio::select! {
                value_a = self.run_async() => Either::Left(value_a),
                value_b = other.run_async() => Either::Right(value_b),
            }
        })
    }

    /// Catches panics in an `AsyncIO` action and converts them to a Result.
    ///
    /// If the `AsyncIO` action panics, the handler is called with the panic info
    /// and should return an error value.
    ///
    /// # Arguments
    ///
    /// * `handler` - A function to handle the panic and return an error value.
    ///
    /// # Type Parameters
    ///
    /// * `E` - The error type to return on panic.
    /// * `F` - The type of the handler function.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::AsyncIO;
    ///
    /// let panicking = AsyncIO::new(|| async { panic!("oops") });
    /// let recovered = panicking.catch_async(|_| "recovered".to_string());
    /// assert_eq!(recovered.run_async().await, Err("recovered".to_string()));
    ///
    /// let successful = AsyncIO::pure(42);
    /// let with_catch = successful.catch_async(|_| "error".to_string());
    /// assert_eq!(with_catch.run_async().await, Ok(42));
    /// ```
    pub fn catch_async<E, F>(self, handler: F) -> AsyncIO<Result<A, E>>
    where
        F: FnOnce(Box<dyn std::any::Any + Send>) -> E + Send + 'static,
        E: Send + 'static,
    {
        use futures::FutureExt;
        use std::panic::AssertUnwindSafe;

        AsyncIO::new(move || async move {
            let result = AssertUnwindSafe(self.run_async()).catch_unwind().await;
            match result {
                Ok(value) => Ok(value),
                Err(panic_info) => Err(handler(panic_info)),
            }
        })
    }
}

// =============================================================================
// Conversion to/from IO
// =============================================================================

impl<A: Send + 'static> AsyncIO<A> {
    /// Converts an `AsyncIO` to a synchronous IO.
    ///
    /// This creates a new tokio runtime to execute the async computation
    /// synchronously.
    ///
    /// # Warning
    ///
    /// This method cannot be used within an async context as it creates
    /// a new runtime. Using it inside an async function will cause a
    /// "nested runtime" panic.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{AsyncIO, IO};
    ///
    /// fn main() {
    ///     let async_io = AsyncIO::pure(42);
    ///     let io = async_io.to_sync();
    ///     let result = io.run_unsafe();
    ///     assert_eq!(result, 42);
    /// }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if creating the tokio runtime fails.
    #[must_use]
    pub fn to_sync(self) -> super::IO<A> {
        super::IO::new(move || {
            let runtime =
                tokio::runtime::Runtime::new().expect("Failed to create tokio runtime for to_sync");
            runtime.block_on(self.run_async())
        })
    }
}

// =============================================================================
// Display Implementation
// =============================================================================

impl<A> std::fmt::Display for AsyncIO<A> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "<AsyncIO>")
    }
}

impl<A: 'static> crate::typeclass::AsyncIOLike for AsyncIO<A> {
    type Value = A;

    fn into_async_io(self) -> Self
    where
        A: Send + 'static,
    {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[test]
    fn test_display_async_io() {
        let async_io = AsyncIO::pure(42);
        assert_eq!(format!("{async_io}"), "<AsyncIO>");
    }

    // =========================================================================
    // Original Tests
    // =========================================================================

    #[tokio::test]
    async fn test_async_io_pure_and_run() {
        let async_io = AsyncIO::pure(42);
        assert_eq!(async_io.run_async().await, 42);
    }

    #[tokio::test]
    async fn test_async_io_new_and_run() {
        let async_io = AsyncIO::new(|| async { 10 + 20 });
        assert_eq!(async_io.run_async().await, 30);
    }

    #[tokio::test]
    async fn test_async_io_fmap() {
        let async_io = AsyncIO::pure(21).fmap(|x| x * 2);
        assert_eq!(async_io.run_async().await, 42);
    }

    #[tokio::test]
    async fn test_async_io_flat_map() {
        let async_io = AsyncIO::pure(10).flat_map(|x| AsyncIO::pure(x * 2));
        assert_eq!(async_io.run_async().await, 20);
    }

    #[tokio::test]
    async fn test_async_io_and_then() {
        let async_io = AsyncIO::pure(10).and_then(|x| AsyncIO::pure(x + 5));
        assert_eq!(async_io.run_async().await, 15);
    }

    #[tokio::test]
    async fn test_async_io_then() {
        let async_io = AsyncIO::pure(10).then(AsyncIO::pure(20));
        assert_eq!(async_io.run_async().await, 20);
    }

    #[tokio::test]
    async fn test_async_io_map2() {
        let async_io = AsyncIO::pure(10).map2(AsyncIO::pure(20), |a, b| a + b);
        assert_eq!(async_io.run_async().await, 30);
    }

    #[tokio::test]
    async fn test_async_io_product() {
        let async_io = AsyncIO::pure(10).product(AsyncIO::pure(20));
        assert_eq!(async_io.run_async().await, (10, 20));
    }

    // =========================================================================
    // TimeoutError Tests
    // =========================================================================

    #[test]
    fn test_timeout_error_display() {
        let error = TimeoutError {
            duration: Duration::from_secs(5),
        };
        assert_eq!(format!("{error}"), "operation timed out after 5s");
    }

    #[test]
    fn test_timeout_error_equality() {
        let error1 = TimeoutError {
            duration: Duration::from_secs(5),
        };
        let error2 = TimeoutError {
            duration: Duration::from_secs(5),
        };
        let error3 = TimeoutError {
            duration: Duration::from_secs(10),
        };

        assert_eq!(error1, error2);
        assert_ne!(error1, error3);
    }

    #[test]
    fn test_timeout_error_clone() {
        let error = TimeoutError {
            duration: Duration::from_millis(100),
        };
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    #[test]
    fn test_timeout_error_debug() {
        let error = TimeoutError {
            duration: Duration::from_millis(100),
        };
        let debug_str = format!("{error:?}");
        assert!(debug_str.contains("TimeoutError"));
        assert!(debug_str.contains("100"));
    }

    // =========================================================================
    // timeout_result Tests
    // =========================================================================

    #[tokio::test]
    async fn test_timeout_result_completes_in_time() {
        let action = AsyncIO::pure(42);
        let result = action
            .timeout_result(Duration::from_secs(1))
            .run_async()
            .await;
        assert_eq!(result, Ok(42));
    }

    #[tokio::test]
    async fn test_timeout_result_times_out() {
        let slow = AsyncIO::delay_async(Duration::from_secs(10)).fmap(|()| 42);
        let result = slow
            .timeout_result(Duration::from_millis(50))
            .run_async()
            .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.duration, Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_timeout_result_is_lazy() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let action = AsyncIO::new(move || async move {
            executed_clone.store(true, Ordering::SeqCst);
            42
        })
        .timeout_result(Duration::from_secs(1));

        // Not executed yet
        assert!(!executed.load(Ordering::SeqCst));

        let _ = action.run_async().await;
        assert!(executed.load(Ordering::SeqCst));
    }

    // =========================================================================
    // Retry Operation Tests
    // =========================================================================

    #[tokio::test]
    async fn test_retry_success_on_first_attempt() {
        let result: Result<i32, &str> = AsyncIO::retry_with_factory(|| AsyncIO::pure(Ok(42)), 3)
            .run_async()
            .await;
        assert_eq!(result, Ok(42));
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = AsyncIO::retry_with_factory(
            move || {
                let counter = counter_clone.clone();
                AsyncIO::new(move || async move {
                    let count = counter.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        Err("temporary error")
                    } else {
                        Ok(42)
                    }
                })
            },
            5,
        );

        assert_eq!(result.run_async().await, Ok(42));
        assert_eq!(counter.load(Ordering::SeqCst), 3); // 3 attempts (2 failures + 1 success)
    }

    #[tokio::test]
    async fn test_retry_all_failures() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = AsyncIO::retry_with_factory(
            move || {
                let counter = counter_clone.clone();
                AsyncIO::new(move || async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, _>("permanent error")
                })
            },
            3,
        );

        assert_eq!(result.run_async().await, Err("permanent error"));
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_zero_attempts() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = AsyncIO::retry_with_factory(
            move || {
                let counter = counter_clone.clone();
                AsyncIO::new(move || async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, _>("error")
                })
            },
            0,
        );

        assert_eq!(result.run_async().await, Err("error"));
        assert_eq!(counter.load(Ordering::SeqCst), 1); // Only 1 attempt even with 0
    }

    #[tokio::test]
    async fn test_retry_is_lazy() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let action = AsyncIO::retry_with_factory(
            move || {
                let executed = executed_clone.clone();
                AsyncIO::new(move || async move {
                    executed.store(true, Ordering::SeqCst);
                    Ok::<i32, &str>(42)
                })
            },
            3,
        );

        // Not executed yet
        assert!(!executed.load(Ordering::SeqCst));

        // Execute
        let _ = action.run_async().await;
        assert!(executed.load(Ordering::SeqCst));
    }

    // =========================================================================
    // Retry with Backoff Tests
    // =========================================================================

    #[tokio::test]
    async fn test_retry_with_backoff_success() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = AsyncIO::retry_with_backoff_factory(
            move || {
                let counter = counter_clone.clone();
                AsyncIO::new(move || async move {
                    if counter.fetch_add(1, Ordering::SeqCst) < 1 {
                        Err("temporary")
                    } else {
                        Ok(42)
                    }
                })
            },
            3,
            Duration::from_millis(10),
        );

        assert_eq!(result.run_async().await, Ok(42));
    }

    #[tokio::test]
    async fn test_retry_with_backoff_timing() {
        use std::time::Instant;

        let start = Instant::now();

        let result = AsyncIO::retry_with_backoff_factory(
            || AsyncIO::pure(Err::<i32, _>("error")),
            3,
            Duration::from_millis(50),
        );

        assert_eq!(result.run_async().await, Err("error"));

        // 50ms + 100ms = 150ms should have elapsed
        assert!(start.elapsed() >= Duration::from_millis(150));
    }

    // =========================================================================
    // Parallel Execution Tests
    // =========================================================================

    #[tokio::test]
    async fn test_par_both_results() {
        let first = AsyncIO::pure(1);
        let second = AsyncIO::pure(2);
        let (first_value, second_value) = first.par(second).run_async().await;
        assert_eq!((first_value, second_value), (1, 2));
    }

    #[tokio::test]
    async fn test_par_is_faster_than_sequential() {
        use std::time::Instant;

        let slow_first = AsyncIO::delay_async(Duration::from_millis(100)).fmap(|()| 1);
        let slow_second = AsyncIO::delay_async(Duration::from_millis(100)).fmap(|()| 2);

        let start = Instant::now();
        let (first_value, second_value) = slow_first.par(slow_second).run_async().await;
        let elapsed = start.elapsed();

        assert_eq!((first_value, second_value), (1, 2));
        // Parallel execution should be less than 200ms (about 100ms + margin)
        assert!(elapsed < Duration::from_millis(150));
    }

    #[tokio::test]
    async fn test_par3_all_results() {
        let first = AsyncIO::pure(1);
        let second = AsyncIO::pure(2);
        let third = AsyncIO::pure(3);
        let (first_value, second_value, third_value) = first.par3(second, third).run_async().await;
        assert_eq!((first_value, second_value, third_value), (1, 2, 3));
    }

    #[tokio::test]
    async fn test_par_is_lazy() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let executed_first = Arc::new(AtomicBool::new(false));
        let executed_second = Arc::new(AtomicBool::new(false));
        let executed_first_clone = executed_first.clone();
        let executed_second_clone = executed_second.clone();

        let first = AsyncIO::new(move || async move {
            executed_first_clone.store(true, Ordering::SeqCst);
            1
        });
        let second = AsyncIO::new(move || async move {
            executed_second_clone.store(true, Ordering::SeqCst);
            2
        });

        let parred = first.par(second);

        // Not executed yet
        assert!(!executed_first.load(Ordering::SeqCst));
        assert!(!executed_second.load(Ordering::SeqCst));

        let _ = parred.run_async().await;

        assert!(executed_first.load(Ordering::SeqCst));
        assert!(executed_second.load(Ordering::SeqCst));
    }

    // =========================================================================
    // race_result Tests
    // =========================================================================

    #[tokio::test]
    async fn test_race_result_fast_wins() {
        let fast = AsyncIO::pure(1);
        let slow = AsyncIO::delay_async(Duration::from_millis(100)).fmap(|()| 2);

        let result = fast.race_result(slow).run_async().await;
        assert_eq!(result, 1);
    }

    #[tokio::test]
    async fn test_race_result_second_fast_wins() {
        let slow = AsyncIO::delay_async(Duration::from_millis(100)).fmap(|()| 1);
        let fast = AsyncIO::pure(2);

        let result = slow.race_result(fast).run_async().await;
        assert_eq!(result, 2);
    }

    #[tokio::test]
    async fn test_race_result_cancels_loser() {
        // Note: This test verifies the loser doesn't complete fully
        // by checking timing, not by directly observing cancellation

        let start = std::time::Instant::now();
        let slow = AsyncIO::delay_async(Duration::from_secs(10)).fmap(|()| 1);
        let fast = AsyncIO::pure(2);

        let result = slow.race_result(fast).run_async().await;
        let elapsed = start.elapsed();

        assert_eq!(result, 2);
        // If the slow one wasn't cancelled, this would take 10 seconds
        assert!(elapsed < Duration::from_millis(100));
    }

    // =========================================================================
    // Bracket Tests
    // =========================================================================

    #[tokio::test]
    async fn test_bracket_normal_flow() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let released = Arc::new(AtomicBool::new(false));
        let released_clone = released.clone();

        let result = AsyncIO::bracket(
            || AsyncIO::pure(42),
            |value| AsyncIO::pure(value * 2),
            move |_| {
                AsyncIO::new(move || async move {
                    released_clone.store(true, Ordering::SeqCst);
                })
            },
        );

        assert_eq!(result.run_async().await, 84);
        assert!(released.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_bracket_releases_on_use_failure() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let released = Arc::new(AtomicBool::new(false));
        let released_clone = released.clone();

        let result: AsyncIO<Result<i32, &str>> = AsyncIO::bracket(
            || AsyncIO::pure(42),
            |_| AsyncIO::pure(Err("error")),
            move |_| {
                AsyncIO::new(move || async move {
                    released_clone.store(true, Ordering::SeqCst);
                })
            },
        );

        assert_eq!(result.run_async().await, Err("error"));
        assert!(released.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_bracket_is_lazy() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let acquired = Arc::new(AtomicBool::new(false));
        let acquired_clone = acquired.clone();

        let action = AsyncIO::bracket(
            move || {
                AsyncIO::new(move || async move {
                    acquired_clone.store(true, Ordering::SeqCst);
                    42
                })
            },
            AsyncIO::pure,
            |_| AsyncIO::pure(()),
        );

        // Not executed yet
        assert!(!acquired.load(Ordering::SeqCst));

        let _ = action.run_async().await;
        assert!(acquired.load(Ordering::SeqCst));
    }

    // =========================================================================
    // finally_async Tests
    // =========================================================================

    #[tokio::test]
    async fn test_finally_async_on_success() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let result = AsyncIO::pure(42).finally_async(move || async move {
            executed_clone.store(true, Ordering::SeqCst);
        });

        assert_eq!(result.run_async().await, 42);
        assert!(executed.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_finally_async_preserves_result() {
        let result: Result<i32, &str> = AsyncIO::pure(Ok(42))
            .finally_async(|| async {})
            .run_async()
            .await;

        assert_eq!(result, Ok(42));
    }

    #[tokio::test]
    async fn test_finally_async_is_lazy() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let main_executed = Arc::new(AtomicBool::new(false));
        let cleanup_executed = Arc::new(AtomicBool::new(false));
        let main_clone = main_executed.clone();
        let cleanup_clone = cleanup_executed.clone();

        let action = AsyncIO::new(move || async move {
            main_clone.store(true, Ordering::SeqCst);
            42
        })
        .finally_async(move || async move {
            cleanup_clone.store(true, Ordering::SeqCst);
        });

        // Not executed yet
        assert!(!main_executed.load(Ordering::SeqCst));
        assert!(!cleanup_executed.load(Ordering::SeqCst));

        let _ = action.run_async().await;

        assert!(main_executed.load(Ordering::SeqCst));
        assert!(cleanup_executed.load(Ordering::SeqCst));
    }

    // =========================================================================
    // on_error Tests
    // =========================================================================

    #[tokio::test]
    async fn test_on_error_executes_callback() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let action: AsyncIO<Result<i32, String>> = AsyncIO::pure(Err("error".to_string()));
        let result = action
            .on_error(move |_| async move {
                called_clone.store(true, Ordering::SeqCst);
            })
            .run_async()
            .await;

        assert_eq!(result, Err("error".to_string()));
        assert!(called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_on_error_not_called_on_success() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let action: AsyncIO<Result<i32, String>> = AsyncIO::pure(Ok(42));
        let result = action
            .on_error(move |_| async move {
                called_clone.store(true, Ordering::SeqCst);
            })
            .run_async()
            .await;

        assert_eq!(result, Ok(42));
        assert!(!called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_on_error_propagates_error() {
        let action: AsyncIO<Result<i32, String>> = AsyncIO::pure(Err("original error".to_string()));
        let result = action.on_error(|_| async {}).run_async().await;

        assert_eq!(result, Err("original error".to_string()));
    }
}
