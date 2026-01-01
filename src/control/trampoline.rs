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
///         internal use of boxed closures.
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
    Suspend(Box<dyn FnOnce() -> Trampoline<A> + 'static>),
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
    pub fn done(value: A) -> Self {
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
        F: FnOnce() -> Trampoline<A> + 'static,
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
    pub fn pure(value: A) -> Self {
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
    pub fn run(self) -> A {
        let mut current = self;

        loop {
            match current {
                Self::Done(value) => return value,
                Self::Suspend(thunk) => {
                    current = thunk();
                }
                Self::FlatMapInternal(continuation) => {
                    current = continuation.step();
                }
            }
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
    pub fn resume(self) -> Either<Box<dyn FnOnce() -> Trampoline<A> + 'static>, A> {
        let mut current = self;

        loop {
            match current {
                Self::Done(value) => return Either::Right(value),
                Self::Suspend(thunk) => return Either::Left(thunk),
                Self::FlatMapInternal(continuation) => {
                    // Unwrap FlatMapInternal and continue the loop
                    current = continuation.step();
                }
            }
        }
    }

    /// Applies a function to the result of the trampoline.
    ///
    /// This is the functor `map` operation.
    ///
    /// # Arguments
    ///
    /// * `function` - A function to apply to the final value
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
    pub fn map<B, F>(self, function: F) -> Trampoline<B>
    where
        F: FnOnce(A) -> B + 'static,
        B: 'static,
    {
        self.flat_map(move |a| Trampoline::done(function(a)))
    }

    /// Applies a function that returns a trampoline to the result.
    ///
    /// This is the monadic `bind` (>>=) operation.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes the result and returns a new trampoline
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
    pub fn flat_map<B, F>(self, function: F) -> Trampoline<B>
    where
        F: FnOnce(A) -> Trampoline<B> + 'static,
        B: 'static,
    {
        Trampoline::FlatMapInternal(ContinuationBox::new(FlatMapContinuation {
            trampoline: self,
            function,
        }))
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

/// Internal structure for flat_map continuation.
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

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
}
