//! Stack-safe recursion via trampolining.
//!
//! This module provides the `Trampoline<A>` type for expressing recursive
//! computations in a stack-safe manner. Instead of using the call stack,
//! recursive steps are represented as data that can be interpreted in a loop.
//!
//! # Motivation
//!
//! Rust does not guarantee tail call optimization (TCO). This means that
//! deeply recursive functions can overflow the stack. Trampolining converts
//! recursion into iteration, making it safe for arbitrary depths.
//!
//! # Examples
//!
//! ## Factorial
//!
//! ```rust
//! use lambars::control::Trampoline;
//!
//! fn factorial(n: u64) -> Trampoline<u64> {
//!     factorial_helper(n, 1)
//! }
//!
//! fn factorial_helper(n: u64, accumulator: u64) -> Trampoline<u64> {
//!     if n <= 1 {
//!         Trampoline::done(accumulator)
//!     } else {
//!         Trampoline::suspend(move || factorial_helper(n - 1, n * accumulator))
//!     }
//! }
//!
//! let result = factorial(20).run();
//! assert_eq!(result, 2432902008176640000);
//! ```
//!
//! ## Mutual Recursion
//!
//! ```rust
//! use lambars::control::Trampoline;
//!
//! fn is_even(n: u64) -> Trampoline<bool> {
//!     if n == 0 {
//!         Trampoline::done(true)
//!     } else {
//!         Trampoline::suspend(move || is_odd(n - 1))
//!     }
//! }
//!
//! fn is_odd(n: u64) -> Trampoline<bool> {
//!     if n == 0 {
//!         Trampoline::done(false)
//!     } else {
//!         Trampoline::suspend(move || is_even(n - 1))
//!     }
//! }
//!
//! assert!(is_even(1000).run());
//! assert!(!is_odd(1000).run());
//! ```

use super::either::Either;

/// Internal trait for type erasure in `FlatMap` continuations.
///
/// This trait allows us to store continuations with different intermediate
/// types in the same `Trampoline` enum variant, enabling proper monadic
/// composition without knowing all types at compile time.
trait TrampolineContinuation<A> {
    /// Execute one step of the continuation, returning the next trampoline state.
    fn step(self: Box<Self>) -> Trampoline<A>;
}

/// A wrapper type to hide the internal trait from the public API.
///
/// This is used to avoid exposing the `TrampolineContinuation` trait
/// in the public enum variant.
#[doc(hidden)]
pub struct ContinuationBox<A>(Box<dyn TrampolineContinuation<A>>);

impl<A> ContinuationBox<A> {
    fn new<T: TrampolineContinuation<A> + 'static>(continuation: T) -> Self {
        Self(Box::new(continuation))
    }

    #[inline]
    fn step(self) -> Trampoline<A> {
        self.0.step()
    }
}

/// A data structure for stack-safe recursion.
///
/// `Trampoline<A>` represents a potentially recursive computation that
/// produces a value of type `A`. Instead of using the call stack, recursive
/// steps are encoded as data and interpreted in a loop.
///
/// # Type Parameters
///
/// * `A` - The type of the final result. Must be `'static` due to the
///   internal use of boxed closures.
///
/// # Design
///
/// The trampoline has three states:
///
/// 1. `Done(A)` - The computation has finished with value `A`
/// 2. `Suspend(...)` - The computation needs to continue with another step
/// 3. `FlatMapInternal(...)` - A composition step (internal, for `flat_map`)
///
/// # Laws
///
/// `Trampoline` forms a monad and satisfies:
///
/// - **Left Identity**: `Trampoline::done(a).flat_map(f).run() == f(a).run()`
/// - **Right Identity**: `m.flat_map(Trampoline::done).run() == m.run()`
/// - **Associativity**: `m.flat_map(f).flat_map(g).run() == m.flat_map(|x| f(x).flat_map(g)).run()`
///
/// # Note
///
/// This type does NOT implement `TypeConstructor` because:
/// - `Trampoline<A>` requires `A: 'static` for the boxed closures
/// - This constraint would propagate to `WithType<B>` and limit usability
/// - Instead, we provide standalone `map`, `flat_map`, and `pure` methods
///
/// # Examples
///
/// ```rust
/// use lambars::control::Trampoline;
///
/// // Simple computation
/// let result = Trampoline::done(42).run();
/// assert_eq!(result, 42);
///
/// // Suspended computation
/// let result = Trampoline::suspend(|| Trampoline::done(42)).run();
/// assert_eq!(result, 42);
/// ```
pub enum Trampoline<A> {
    /// The computation has completed with value `A`.
    Done(A),
    /// The computation is suspended and needs another step.
    ///
    /// The boxed function returns the next state of the trampoline.
    Suspend(Box<dyn FnOnce() -> Self + 'static>),
    /// Internal state for `flat_map` composition.
    ///
    /// Uses type erasure via `TrampolineContinuation` to handle
    /// continuations with different intermediate types.
    #[doc(hidden)]
    FlatMapInternal(ContinuationBox<A>),
}

impl<A> Trampoline<A> {
    /// Creates a completed trampoline with the given value.
    ///
    /// This is the return/pure operation for the trampoline monad.
    ///
    /// # Arguments
    ///
    /// * `value` - The final result of the computation
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Trampoline;
    ///
    /// let trampoline = Trampoline::done(42);
    /// assert_eq!(trampoline.run(), 42);
    /// ```
    #[inline]
    pub const fn done(value: A) -> Self {
        Self::Done(value)
    }

    /// Creates a suspended trampoline that will continue with the given thunk.
    ///
    /// The thunk is not evaluated until `run()` is called.
    ///
    /// # Arguments
    ///
    /// * `thunk` - A function that produces the next trampoline state
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Trampoline;
    ///
    /// let trampoline = Trampoline::suspend(|| Trampoline::done(42));
    /// assert_eq!(trampoline.run(), 42);
    /// ```
    #[inline]
    pub fn suspend<F>(thunk: F) -> Self
    where
        F: FnOnce() -> Self + 'static,
    {
        Self::Suspend(Box::new(thunk))
    }

    /// Alias for `done`. Lifts a value into the trampoline context.
    ///
    /// This corresponds to the `pure` operation in Applicative.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Trampoline;
    ///
    /// let trampoline = Trampoline::pure(42);
    /// assert_eq!(trampoline.run(), 42);
    /// ```
    #[inline]
    pub const fn pure(value: A) -> Self {
        Self::done(value)
    }
}

impl<A: 'static> Trampoline<A> {
    /// Runs the trampoline to completion and returns the final value.
    ///
    /// This method iteratively evaluates the trampoline steps until
    /// a `Done` state is reached. The evaluation uses constant stack space.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Trampoline;
    ///
    /// fn count_down(n: u64) -> Trampoline<u64> {
    ///     if n == 0 {
    ///         Trampoline::done(0)
    ///     } else {
    ///         Trampoline::suspend(move || count_down(n - 1))
    ///     }
    /// }
    ///
    /// // This would overflow the stack with regular recursion
    /// let result = count_down(100_000).run();
    /// assert_eq!(result, 0);
    /// ```
    #[inline]
    pub fn run(self) -> A {
        if let Self::Done(value) = self {
            return value;
        }

        let mut current = self;

        loop {
            current = match current {
                Self::Done(value) => return value,
                Self::Suspend(thunk) => thunk(),
                Self::FlatMapInternal(continuation) => {
                    let mut inner = continuation.step();
                    while let Self::FlatMapInternal(next_continuation) = inner {
                        inner = next_continuation.step();
                    }
                    inner
                }
            };
        }
    }

    /// Default batch size for `run_batched()`.
    const DEFAULT_BATCH_SIZE: usize = 16;

    /// Batch size for processing consecutive `FlatMapInternal` chains in `run_optimized()`.
    const FLATMAP_CHAIN_BATCH_SIZE: usize = 16;

    /// Runs the trampoline to completion using batch processing.
    ///
    /// This method processes multiple steps in each iteration of the main loop,
    /// reducing loop overhead for computations with many steps.
    ///
    /// # Performance
    ///
    /// Batch processing amortizes the loop overhead across multiple steps.
    /// The default batch size (16) provides a good balance between reduced
    /// overhead and responsiveness.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Trampoline;
    ///
    /// fn count_down(n: u64) -> Trampoline<u64> {
    ///     if n == 0 {
    ///         Trampoline::done(0)
    ///     } else {
    ///         Trampoline::suspend(move || count_down(n - 1))
    ///     }
    /// }
    ///
    /// let result = count_down(10000).run_batched();
    /// assert_eq!(result, 0);
    /// ```
    #[inline]
    pub fn run_batched(self) -> A {
        self.run_with_batch_size(Self::DEFAULT_BATCH_SIZE)
    }

    /// Runs the trampoline to completion using the specified batch size.
    ///
    /// # Arguments
    ///
    /// * `batch_size` - The number of steps to process in each batch.
    ///   Must be greater than 0.
    ///
    /// # Panics
    ///
    /// Panics if `batch_size` is 0.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Trampoline;
    ///
    /// fn count_down(n: u64) -> Trampoline<u64> {
    ///     if n == 0 {
    ///         Trampoline::done(0)
    ///     } else {
    ///         Trampoline::suspend(move || count_down(n - 1))
    ///     }
    /// }
    ///
    /// // Use a larger batch size for deeper recursion
    /// let result = count_down(100000).run_with_batch_size(32);
    /// assert_eq!(result, 0);
    /// ```
    #[inline]
    pub fn run_with_batch_size(self, batch_size: usize) -> A {
        assert!(batch_size > 0, "batch_size must be greater than 0");

        let mut current = self;

        loop {
            for _ in 0..batch_size {
                match current {
                    Self::Done(value) => return value,
                    Self::Suspend(thunk) => current = thunk(),
                    Self::FlatMapInternal(continuation) => current = continuation.step(),
                }
            }
        }
    }

    /// Runs the trampoline to completion using an optimized strategy.
    ///
    /// This method combines early return for `Done` state and batch processing
    /// of `FlatMapInternal` chains for better performance with deep `flat_map` chains.
    ///
    /// # When to Use
    ///
    /// - Use [`run()`](Self::run) for general cases or shallow recursion
    /// - Use `run_optimized()` when you have deep `flat_map` chains (100+ levels)
    /// - Use [`run_batched()`](Self::run_batched) for deep `Suspend` chains
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Trampoline;
    ///
    /// fn count_down(n: u64) -> Trampoline<u64> {
    ///     if n == 0 {
    ///         Trampoline::done(0)
    ///     } else {
    ///         Trampoline::suspend(move || count_down(n - 1))
    ///     }
    /// }
    ///
    /// let result = count_down(100000).run_optimized();
    /// assert_eq!(result, 0);
    /// ```
    #[inline]
    pub fn run_optimized(self) -> A {
        if let Self::Done(value) = self {
            return value;
        }

        let mut current = self;

        loop {
            current = match current {
                Self::Done(value) => return value,
                Self::Suspend(thunk) => thunk(),
                Self::FlatMapInternal(continuation) => {
                    let mut inner = continuation.step();
                    for _ in 0..Self::FLATMAP_CHAIN_BATCH_SIZE {
                        match inner {
                            Self::FlatMapInternal(next_continuation) => {
                                inner = next_continuation.step();
                            }
                            _ => break,
                        }
                    }
                    inner
                }
            };
        }
    }

    /// Takes one step of the trampoline computation.
    ///
    /// Returns either:
    /// - `Right(A)` if the computation is complete
    /// - `Left(thunk)` if there's more work to do
    ///
    /// This method is useful for incremental evaluation or debugging.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::{Trampoline, Either};
    ///
    /// let trampoline = Trampoline::suspend(|| Trampoline::done(42));
    ///
    /// match trampoline.resume() {
    ///     Either::Left(thunk) => {
    ///         // More work to do
    ///         let next = thunk();
    ///         assert!(matches!(next.resume(), Either::Right(42)));
    ///     }
    ///     Either::Right(value) => {
    ///         // Already done
    ///         assert_eq!(value, 42);
    ///     }
    /// }
    /// ```
    pub fn resume(self) -> Either<Box<dyn FnOnce() -> Self + 'static>, A> {
        let mut current = self;

        loop {
            match current {
                Self::Done(value) => return Either::Right(value),
                Self::Suspend(thunk) => return Either::Left(thunk),
                Self::FlatMapInternal(continuation) => current = continuation.step(),
            }
        }
    }

    /// Applies a function to the result of the trampoline.
    ///
    /// This is the functor `map` operation.
    ///
    /// # Note on Evaluation Timing
    ///
    /// When the source `Trampoline` is in the `Done` state, the function is applied
    /// immediately without creating intermediate structures. This optimization assumes
    /// that `function` is a pure function (no side effects). If you pass a function
    /// with side effects, the timing of those effects may differ from other implementations.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Trampoline;
    ///
    /// let trampoline = Trampoline::done(21);
    /// let doubled = trampoline.map(|x| x * 2);
    /// assert_eq!(doubled.run(), 42);
    /// ```
    #[inline]
    pub fn map<B, F>(self, function: F) -> Trampoline<B>
    where
        F: FnOnce(A) -> B + 'static,
        B: 'static,
    {
        match self {
            Self::Done(value) => Trampoline::Done(function(value)),
            _ => self.flat_map(move |a| Trampoline::done(function(a))),
        }
    }

    /// Applies a function that returns a trampoline to the result.
    ///
    /// This is the monadic `bind` (>>=) operation.
    ///
    /// # Note on Evaluation Timing
    ///
    /// When the source `Trampoline` is in the `Done` state, the function is applied
    /// immediately without creating an intermediate `FlatMapInternal` wrapper. This
    /// optimization assumes that `function` is a pure function (no side effects).
    /// If you pass a function with side effects, the timing of those effects may
    /// differ from other implementations.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Trampoline;
    ///
    /// let trampoline = Trampoline::done(21);
    /// let result = trampoline.flat_map(|x| Trampoline::done(x * 2));
    /// assert_eq!(result.run(), 42);
    /// ```
    #[inline]
    pub fn flat_map<B, F>(self, function: F) -> Trampoline<B>
    where
        F: FnOnce(A) -> Trampoline<B> + 'static,
        B: 'static,
    {
        match self {
            Self::Done(value) => function(value),
            _ => Trampoline::FlatMapInternal(ContinuationBox::new(FlatMapContinuation {
                trampoline: self,
                function,
            })),
        }
    }

    /// Alias for `flat_map`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Trampoline;
    ///
    /// let trampoline = Trampoline::done(21);
    /// let result = trampoline.and_then(|x| Trampoline::done(x * 2));
    /// assert_eq!(result.run(), 42);
    /// ```
    #[inline]
    pub fn and_then<B, F>(self, function: F) -> Trampoline<B>
    where
        F: FnOnce(A) -> Trampoline<B> + 'static,
        B: 'static,
    {
        self.flat_map(function)
    }

    /// Sequences two trampolines, discarding the result of the first.
    ///
    /// # Arguments
    ///
    /// * `next` - The trampoline to execute after this one
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Trampoline;
    ///
    /// let first = Trampoline::done("ignored");
    /// let second = Trampoline::done(42);
    /// let result = first.then(second);
    /// assert_eq!(result.run(), 42);
    /// ```
    #[inline]
    pub fn then<B: 'static>(self, next: Trampoline<B>) -> Trampoline<B> {
        self.flat_map(move |_| next)
    }
}

/// Internal structure for `flat_map` continuation.
///
/// This struct captures the current trampoline state and the continuation
/// function to apply when the current state reaches `Done`.
struct FlatMapContinuation<A, B, F>
where
    F: FnOnce(A) -> Trampoline<B>,
{
    trampoline: Trampoline<A>,
    function: F,
}

impl<A: 'static, B: 'static, F> TrampolineContinuation<B> for FlatMapContinuation<A, B, F>
where
    F: FnOnce(A) -> Trampoline<B> + 'static,
{
    fn step(self: Box<Self>) -> Trampoline<B> {
        match self.trampoline {
            Trampoline::Done(a) => (self.function)(a),
            Trampoline::Suspend(thunk) => {
                let function = self.function;
                Trampoline::suspend(move || thunk().flat_map(function))
            }
            Trampoline::FlatMapInternal(inner) => {
                // Use associativity to flatten: (m >>= f) >>= g == m >>= (\x -> f x >>= g)
                let function = self.function;
                inner.0.step().flat_map(function)
            }
        }
    }
}

// =============================================================================
// Debug Implementation
// =============================================================================

impl<A: std::fmt::Debug> std::fmt::Debug for Trampoline<A> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Done(value) => formatter.debug_tuple("Done").field(value).finish(),
            Self::Suspend(_) => formatter.debug_tuple("Suspend").field(&"<thunk>").finish(),
            Self::FlatMapInternal(_) => formatter
                .debug_tuple("FlatMapInternal")
                .field(&"<continuation>")
                .finish(),
        }
    }
}

impl<A: std::fmt::Display> std::fmt::Display for Trampoline<A> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Done(value) => write!(formatter, "Done({value})"),
            Self::Suspend(_) => write!(formatter, "<Suspend>"),
            Self::FlatMapInternal(_) => write!(formatter, "<FlatMap>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[rstest]
    fn test_display_done() {
        let trampoline = Trampoline::done(42);
        assert_eq!(format!("{trampoline}"), "Done(42)");
    }

    #[rstest]
    fn test_display_suspend() {
        let trampoline: Trampoline<i32> = Trampoline::suspend(|| Trampoline::done(42));
        assert_eq!(format!("{trampoline}"), "<Suspend>");
    }

    #[rstest]
    fn test_display_flatmap_internal() {
        let trampoline: Trampoline<i32> = Trampoline::suspend(|| Trampoline::done(21))
            .flat_map(|value| Trampoline::done(value * 2));
        assert_eq!(format!("{trampoline}"), "<FlatMap>");
    }

    // =========================================================================
    // Original Tests
    // =========================================================================

    #[rstest]
    fn test_trampoline_done() {
        let trampoline = Trampoline::done(42);
        assert_eq!(trampoline.run(), 42);
    }

    #[rstest]
    fn test_trampoline_suspend() {
        let trampoline = Trampoline::suspend(|| Trampoline::done(42));
        assert_eq!(trampoline.run(), 42);
    }

    #[rstest]
    fn test_trampoline_map() {
        let trampoline = Trampoline::done(21);
        let doubled = trampoline.map(|x| x * 2);
        assert_eq!(doubled.run(), 42);
    }

    #[rstest]
    fn test_trampoline_flat_map() {
        let trampoline = Trampoline::done(21);
        let result = trampoline.flat_map(|x| Trampoline::done(x * 2));
        assert_eq!(result.run(), 42);
    }

    #[rstest]
    fn test_trampoline_factorial() {
        fn factorial(n: u64) -> Trampoline<u64> {
            factorial_helper(n, 1)
        }

        fn factorial_helper(n: u64, accumulator: u64) -> Trampoline<u64> {
            if n <= 1 {
                Trampoline::done(accumulator)
            } else {
                Trampoline::suspend(move || factorial_helper(n - 1, n * accumulator))
            }
        }

        assert_eq!(factorial(0).run(), 1);
        assert_eq!(factorial(1).run(), 1);
        assert_eq!(factorial(5).run(), 120);
        assert_eq!(factorial(10).run(), 3_628_800);
    }

    #[rstest]
    fn test_trampoline_mutual_recursion() {
        fn is_even(n: u64) -> Trampoline<bool> {
            if n == 0 {
                Trampoline::done(true)
            } else {
                Trampoline::suspend(move || is_odd(n - 1))
            }
        }

        fn is_odd(n: u64) -> Trampoline<bool> {
            if n == 0 {
                Trampoline::done(false)
            } else {
                Trampoline::suspend(move || is_even(n - 1))
            }
        }

        assert!(is_even(0).run());
        assert!(!is_odd(0).run());
        assert!(!is_even(1).run());
        assert!(is_odd(1).run());
        assert!(is_even(100).run());
        assert!(!is_odd(100).run());
    }

    // =========================================================================
    // flat_map Done eager evaluation tests
    // =========================================================================

    #[rstest]
    fn test_flat_map_done_eager_evaluation() {
        let trampoline = Trampoline::done(42);
        let result = trampoline.flat_map(|x| Trampoline::done(x * 2));

        assert!(
            matches!(result, Trampoline::Done(84)),
            "Expected Done(84), got {result:?}"
        );
    }

    #[rstest]
    fn test_flat_map_done_to_suspend() {
        let trampoline = Trampoline::done(42);
        let result = trampoline.flat_map(|x| Trampoline::suspend(move || Trampoline::done(x * 2)));

        assert!(
            matches!(result, Trampoline::Suspend(_)),
            "Expected Suspend, got {result:?}"
        );
        assert_eq!(result.run(), 84);
    }

    #[rstest]
    fn test_flat_map_suspend_creates_flatmap_internal() {
        let trampoline: Trampoline<i32> = Trampoline::suspend(|| Trampoline::done(42));
        let result = trampoline.flat_map(|x| Trampoline::done(x * 2));

        assert!(
            matches!(result, Trampoline::FlatMapInternal(_)),
            "Expected FlatMapInternal, got {result:?}"
        );
        assert_eq!(result.run(), 84);
    }

    #[rstest]
    fn test_flat_map_done_chain() {
        let result = Trampoline::done(1)
            .flat_map(|x| Trampoline::done(x + 1))
            .flat_map(|x| Trampoline::done(x * 2))
            .flat_map(|x| Trampoline::done(x + 10));

        assert!(
            matches!(result, Trampoline::Done(14)),
            "Expected Done(14), got {result:?}"
        );
        assert_eq!(result.run(), 14);
    }

    // =========================================================================
    // FlatMap chain optimization tests
    // =========================================================================

    #[rstest]
    fn test_deep_flatmap_chain_from_suspend() {
        let trampoline: Trampoline<i32> = Trampoline::suspend(|| Trampoline::done(0));
        let result = (0..100).fold(trampoline, |accumulator, _| {
            accumulator.flat_map(|x| Trampoline::done(x + 1))
        });

        assert_eq!(result.run(), 100);
    }

    #[rstest]
    fn test_nested_flatmap() {
        let trampoline: Trampoline<i32> = Trampoline::suspend(|| Trampoline::done(1));
        let result = trampoline.flat_map(|x| {
            Trampoline::suspend(move || Trampoline::done(x * 2))
                .flat_map(|y| Trampoline::done(y + 10))
        });

        assert_eq!(result.run(), 12);
    }

    #[rstest]
    fn test_long_flatmap_chain_correctness() {
        let depth = 100;
        let mut trampoline = Trampoline::suspend(|| Trampoline::done(0u64));

        for index in 1..=depth {
            let index_copy = index;
            trampoline = trampoline.flat_map(move |x| Trampoline::done(x + index_copy));
        }

        let expected = (depth * (depth + 1)) / 2;
        assert_eq!(trampoline.run(), expected);
    }

    // =========================================================================
    // run() early return tests
    // =========================================================================

    #[rstest]
    fn test_done_early_return() {
        let trampoline = Trampoline::done(42);
        assert_eq!(trampoline.run(), 42);
    }

    #[rstest]
    fn test_done_early_return_string() {
        let trampoline = Trampoline::done("hello".to_string());
        assert_eq!(trampoline.run(), "hello");
    }

    #[rstest]
    fn test_suspend_works_after_early_check() {
        let trampoline = Trampoline::suspend(|| Trampoline::done(100));
        assert_eq!(trampoline.run(), 100);
    }

    // =========================================================================
    // map Done eager evaluation tests
    // =========================================================================

    #[rstest]
    fn test_map_done_eager_evaluation() {
        let trampoline = Trampoline::done(42);
        let result = trampoline.map(|x| x * 2);

        assert!(
            matches!(result, Trampoline::Done(84)),
            "Expected Done(84), got {result:?}"
        );
    }

    #[rstest]
    fn test_map_suspend_creates_flatmap_internal() {
        let trampoline: Trampoline<i32> = Trampoline::suspend(|| Trampoline::done(42));
        let result = trampoline.map(|x| x * 2);

        assert!(
            matches!(result, Trampoline::FlatMapInternal(_)),
            "Expected FlatMapInternal, got {result:?}"
        );
        assert_eq!(result.run(), 84);
    }

    #[rstest]
    fn test_map_done_chain() {
        let result = Trampoline::done(1)
            .map(|x| x + 1)
            .map(|x| x * 2)
            .map(|x| x + 10);

        assert!(
            matches!(result, Trampoline::Done(14)),
            "Expected Done(14), got {result:?}"
        );
        assert_eq!(result.run(), 14);
    }

    // =========================================================================
    // Batch processing tests
    // =========================================================================

    #[rstest]
    fn test_run_batched_same_as_run() {
        fn count_down(n: u64) -> Trampoline<u64> {
            if n == 0 {
                Trampoline::done(0)
            } else {
                Trampoline::suspend(move || count_down(n - 1))
            }
        }

        let result_run = count_down(100).run();
        let result_batched = count_down(100).run_batched();

        assert_eq!(result_run, result_batched);
    }

    #[rstest]
    #[case(1)]
    #[case(4)]
    #[case(16)]
    #[case(64)]
    fn test_run_with_batch_size(#[case] batch_size: usize) {
        fn sum_to(n: u64) -> Trampoline<u64> {
            sum_to_helper(n, 0)
        }

        fn sum_to_helper(n: u64, accumulator: u64) -> Trampoline<u64> {
            if n == 0 {
                Trampoline::done(accumulator)
            } else {
                Trampoline::suspend(move || sum_to_helper(n - 1, accumulator + n))
            }
        }

        let result = sum_to(100).run_with_batch_size(batch_size);
        let expected = 100 * 101 / 2;
        assert_eq!(result, expected);
    }

    #[rstest]
    #[should_panic(expected = "batch_size must be greater than 0")]
    fn test_batch_size_zero_panics() {
        Trampoline::done(42).run_with_batch_size(0);
    }

    #[rstest]
    fn test_run_batched_done() {
        let trampoline = Trampoline::done(42);
        assert_eq!(trampoline.run_batched(), 42);
    }

    #[rstest]
    fn test_run_batched_flatmap_chain() {
        let trampoline: Trampoline<i32> = Trampoline::suspend(|| Trampoline::done(0));
        let result = (0..50).fold(trampoline, |accumulator, _| {
            accumulator.flat_map(|x| Trampoline::done(x + 1))
        });

        assert_eq!(result.run_batched(), 50);
    }

    // =========================================================================
    // run_optimized tests
    // =========================================================================

    #[rstest]
    fn test_run_optimized_same_as_run() {
        fn count_down(n: u64) -> Trampoline<u64> {
            if n == 0 {
                Trampoline::done(0)
            } else {
                Trampoline::suspend(move || count_down(n - 1))
            }
        }

        let result_run = count_down(100).run();
        let result_optimized = count_down(100).run_optimized();

        assert_eq!(result_run, result_optimized);
    }

    #[rstest]
    fn test_run_optimized_done() {
        let trampoline = Trampoline::done(42);
        assert_eq!(trampoline.run_optimized(), 42);
    }

    #[rstest]
    fn test_run_optimized_flatmap_chain() {
        let trampoline: Trampoline<i32> = Trampoline::suspend(|| Trampoline::done(0));
        let result = (0..100).fold(trampoline, |accumulator, _| {
            accumulator.flat_map(|x| Trampoline::done(x + 1))
        });

        assert_eq!(result.run_optimized(), 100);
    }

    #[rstest]
    fn test_run_optimized_nested() {
        let trampoline: Trampoline<i32> = Trampoline::suspend(|| Trampoline::done(1));
        let result = trampoline.flat_map(|x| {
            Trampoline::suspend(move || Trampoline::done(x * 2))
                .flat_map(|y| Trampoline::done(y + 10))
        });

        assert_eq!(result.run_optimized(), 12);
    }

    #[rstest]
    fn test_run_optimized_sum() {
        fn sum_to(n: u64) -> Trampoline<u64> {
            sum_to_helper(n, 0)
        }

        fn sum_to_helper(n: u64, accumulator: u64) -> Trampoline<u64> {
            if n == 0 {
                Trampoline::done(accumulator)
            } else {
                Trampoline::suspend(move || sum_to_helper(n - 1, accumulator + n))
            }
        }

        let result = sum_to(100).run_optimized();
        let expected = 100 * 101 / 2;
        assert_eq!(result, expected);
    }
}
