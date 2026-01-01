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
    { self.run_async().await }
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
