//! Continuation monad for continuation-passing style (CPS).
//!
//! This module provides the `Continuation<R, A>` type, which abstracts over
//! continuation-passing style programming. Continuations represent "the rest
//! of the computation" and can be manipulated as first-class values.
//!
//! # Motivation
//!
//! Continuation-passing style is a powerful programming technique that can
//! express complex control flow patterns such as:
//!
//! - Early return / exit
//! - Exception handling
//! - Backtracking
//! - Coroutines
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```rust
//! use lambars::control::Continuation;
//!
//! let cont: Continuation<i32, i32> = Continuation::pure(42);
//! let result = cont.run(|x| x * 2);
//! assert_eq!(result, 84);
//! ```
//!
//! ## Early Return with `call_with_current_continuation_once`
//!
//! ```rust
//! use lambars::control::Continuation;
//!
//! let cont = Continuation::call_with_current_continuation_once(|exit| {
//!     Continuation::pure(1)
//!         .flat_map(|x| {
//!             if x > 10 {
//!                 exit(x * 100) // Early return
//!             } else {
//!                 Continuation::pure(x + 5)
//!             }
//!         })
//! });
//!
//! let result = cont.run(|x| x);
//! assert_eq!(result, 6); // x = 1, not > 10, so returns 1 + 5 = 6
//! ```

use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

/// A boxed continuation function that takes a value and produces a result.
type ContinuationFunction<A, R> = Box<dyn FnOnce(A) -> R>;

/// A boxed CPS function that takes a continuation and produces a result.
type CpsFunction<A, R> = Box<dyn FnOnce(ContinuationFunction<A, R>) -> R>;

/// A shared, mutable holder for a continuation function.
type ContinuationHolder<A, R> = Rc<RefCell<Option<ContinuationFunction<A, R>>>>;

/// A continuation monad representing computations in CPS.
///
/// `Continuation<R, A>` encapsulates a computation that:
/// - Produces a value of type `A`
/// - When given a continuation `(A -> R)`, produces a final result of type `R`
///
/// The internal representation is essentially `(A -> R) -> R`.
///
/// # Type Parameters
///
/// * `R` - The type of the final result (the return type of the whole computation)
/// * `A` - The type of the intermediate value this continuation produces
///
/// # Laws
///
/// `Continuation` forms a monad:
///
/// - **Left Identity**: `Continuation::pure(a).flat_map(f).run(k) == f(a).run(k)`
/// - **Right Identity**: `m.flat_map(Continuation::pure).run(k) == m.run(k)`
/// - **Associativity**: `m.flat_map(f).flat_map(g).run(k) == m.flat_map(|x| f(x).flat_map(g)).run(k)`
///
/// # Examples
///
/// ```rust
/// use lambars::control::Continuation;
///
/// // Create a continuation that doubles its input
/// let double: Continuation<i32, i32> = Continuation::new(|k| k(21) * 2);
///
/// // Run it - the continuation receives 21, returns 21, then we multiply by 2
/// // But actually, k(21) returns what *we* pass as the final continuation
/// // So: k(21) where k = |x| x gives us 21, then * 2 = 42
/// let result = double.run(|x| x);
/// assert_eq!(result, 42);
/// ```
pub struct Continuation<R, A> {
    /// The continuation function: given a continuation `(A -> R)`, produces `R`.
    run_continuation: CpsFunction<A, R>,
    /// Phantom data for the type parameters.
    _marker: PhantomData<(R, A)>,
}

impl<R: 'static, A: 'static> Continuation<R, A> {
    /// Creates a new continuation from a function.
    ///
    /// The function takes a continuation `(A -> R)` and produces the final result `R`.
    ///
    /// # Arguments
    ///
    /// * `run` - A function `(A -> R) -> R`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Continuation;
    ///
    /// // A continuation that passes 42 to its continuation
    /// let cont: Continuation<String, i32> = Continuation::new(|k| k(42));
    /// let result = cont.run(|x| x.to_string());
    /// assert_eq!(result, "42");
    /// ```
    pub fn new<F>(run: F) -> Self
    where
        F: FnOnce(Box<dyn FnOnce(A) -> R>) -> R + 'static,
    {
        Self {
            run_continuation: Box::new(run),
            _marker: PhantomData,
        }
    }

    /// Lifts a pure value into the continuation monad.
    ///
    /// Creates a continuation that simply passes the value to its continuation.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to lift
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Continuation;
    ///
    /// let cont: Continuation<i32, i32> = Continuation::pure(42);
    /// let result = cont.run(|x| x);
    /// assert_eq!(result, 42);
    /// ```
    pub fn pure(value: A) -> Self {
        Self::new(move |continuation| continuation(value))
    }

    /// Runs the continuation with the given final continuation.
    ///
    /// # Arguments
    ///
    /// * `continuation` - The final continuation to apply
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Continuation;
    ///
    /// let cont: Continuation<String, i32> = Continuation::pure(42);
    /// let result = cont.run(|x| format!("The answer is {}", x));
    /// assert_eq!(result, "The answer is 42");
    /// ```
    pub fn run<K>(self, continuation: K) -> R
    where
        K: FnOnce(A) -> R + 'static,
    {
        (self.run_continuation)(Box::new(continuation))
    }

    /// Applies a function to the result of this continuation.
    ///
    /// This is the functor `map` operation.
    ///
    /// # Arguments
    ///
    /// * `function` - A function to apply to the intermediate value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Continuation;
    ///
    /// let cont: Continuation<i32, i32> = Continuation::pure(21);
    /// let doubled = cont.map(|x| x * 2);
    /// let result = doubled.run(|x| x);
    /// assert_eq!(result, 42);
    /// ```
    pub fn map<B: 'static, F>(self, function: F) -> Continuation<R, B>
    where
        F: FnOnce(A) -> B + 'static,
    {
        Continuation::new(move |continuation| self.run(move |a| continuation(function(a))))
    }

    /// Applies a function that returns a continuation to the result.
    ///
    /// This is the monadic `bind` (>>=) operation.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes the intermediate value and returns a new continuation
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Continuation;
    ///
    /// let cont: Continuation<i32, i32> = Continuation::pure(21);
    /// let result = cont.flat_map(|x| Continuation::pure(x * 2));
    /// assert_eq!(result.run(|x| x), 42);
    /// ```
    pub fn flat_map<B: 'static, F>(self, function: F) -> Continuation<R, B>
    where
        F: FnOnce(A) -> Continuation<R, B> + 'static,
    {
        Continuation::new(move |continuation| self.run(move |a| function(a).run(continuation)))
    }

    /// Alias for `flat_map`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Continuation;
    ///
    /// let cont: Continuation<i32, i32> = Continuation::pure(21);
    /// let result = cont.and_then(|x| Continuation::pure(x * 2));
    /// assert_eq!(result.run(|x| x), 42);
    /// ```
    #[inline]
    pub fn and_then<B: 'static, F>(self, function: F) -> Continuation<R, B>
    where
        F: FnOnce(A) -> Continuation<R, B> + 'static,
    {
        self.flat_map(function)
    }

    /// Sequences two continuations, discarding the result of the first.
    ///
    /// # Arguments
    ///
    /// * `next` - The continuation to run after this one
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::control::Continuation;
    ///
    /// let first: Continuation<i32, &str> = Continuation::pure("ignored");
    /// let second: Continuation<i32, i32> = Continuation::pure(42);
    /// let result = first.then(second);
    /// assert_eq!(result.run(|x| x), 42);
    /// ```
    #[inline]
    #[must_use]
    pub fn then<B: 'static>(self, next: Continuation<R, B>) -> Continuation<R, B> {
        self.flat_map(move |_| next)
    }

    /// Captures the current continuation (call/cc, one-shot version).
    ///
    /// This function gives you access to "the rest of the computation" as a
    /// first-class value. The captured continuation can be called to "jump back"
    /// to that point in the computation.
    ///
    /// # Important
    ///
    /// Due to Rust's ownership system, this is a **one-shot** version of call/cc.
    /// The captured continuation can only be called once.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that receives the current continuation as `exit`
    ///
    /// # How It Works
    ///
    /// When `exit(value)` is called:
    /// 1. The entire computation inside `function` is abandoned
    /// 2. `value` is returned as if it were the result of the whole `call_with_current_continuation_once`
    ///
    /// If `exit` is never called, the computation proceeds normally.
    ///
    /// # Examples
    ///
    /// ## Early Return
    ///
    /// ```rust
    /// use lambars::control::Continuation;
    ///
    /// // Early return when a condition is met
    /// let cont = Continuation::call_with_current_continuation_once(|exit| {
    ///     Continuation::pure(20)
    ///         .flat_map(move |x| {
    ///             if x > 10 {
    ///                 exit(x * 100) // Early return with 2000
    ///             } else {
    ///                 Continuation::pure(x + 5)
    ///             }
    ///         })
    /// });
    ///
    /// let result = cont.run(|x| x);
    /// assert_eq!(result, 2000); // 20 > 10, so exit(20 * 100) = 2000
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the captured continuation is called more than once, or if
    /// the continuation is consumed by exit and normal execution path is reached.
    pub fn call_with_current_continuation_once<F>(function: F) -> Self
    where
        F: FnOnce(Box<dyn FnOnce(A) -> Self>) -> Self + 'static,
    {
        Self::new(move |outer_continuation: Box<dyn FnOnce(A) -> R>| {
            // We need to share the outer continuation between the exit path and normal path
            // Since FnOnce can only be called once, we wrap it in Rc<RefCell<Option<...>>>
            let continuation_holder: ContinuationHolder<A, R> =
                Rc::new(RefCell::new(Some(outer_continuation)));

            // Create a clone for the exit function
            let holder_for_exit = continuation_holder.clone();

            // The exit function: when called, it uses the outer continuation directly
            let exit: Box<dyn FnOnce(A) -> Self> = Box::new(move |a: A| {
                Self::new(move |_unused: Box<dyn FnOnce(A) -> R>| {
                    // Take the continuation from the holder
                    let continuation = holder_for_exit
                        .borrow_mut()
                        .take()
                        .expect("continuation already consumed");
                    continuation(a)
                })
            });

            // Run the user's function with the exit capability
            let inner = function(exit);

            // Run the inner continuation
            inner.run(move |a| {
                continuation_holder.borrow_mut().take().map_or_else(
                    // Exit was called, this path shouldn't be reached normally
                    // But if we get here, we need to return something of type R
                    // This is a design limitation of the one-shot call/cc
                    || panic!("continuation was consumed by exit"),
                    |continuation| continuation(a),
                )
            })
        })
    }
}

// =============================================================================
// Debug Implementation
// =============================================================================

impl<R, A> std::fmt::Debug for Continuation<R, A> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Continuation")
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn test_continuation_pure() {
        let cont: Continuation<i32, i32> = Continuation::pure(42);
        let result = cont.run(|x| x);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn test_continuation_map() {
        let cont: Continuation<i32, i32> = Continuation::pure(21);
        let doubled = cont.map(|x| x * 2);
        let result = doubled.run(|x| x);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn test_continuation_flat_map() {
        let cont: Continuation<i32, i32> = Continuation::pure(21);
        let result = cont.flat_map(|x| Continuation::pure(x * 2));
        assert_eq!(result.run(|x| x), 42);
    }

    #[rstest]
    fn test_continuation_then() {
        let first: Continuation<i32, &str> = Continuation::pure("ignored");
        let second: Continuation<i32, i32> = Continuation::pure(42);
        let result = first.then(second);
        assert_eq!(result.run(|x| x), 42);
    }

    #[rstest]
    fn test_continuation_call_cc_no_exit() {
        let cont =
            Continuation::call_with_current_continuation_once(|_exit| Continuation::pure(42));
        let result = cont.run(|x| x);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn test_continuation_call_cc_with_exit() {
        let cont: Continuation<i32, i32> =
            Continuation::call_with_current_continuation_once(|exit| exit(42));
        let result = cont.run(|x| x);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn test_continuation_call_cc_early_return() {
        let cont = Continuation::call_with_current_continuation_once(|exit| {
            Continuation::pure(1).flat_map(move |x| {
                if x > 10 {
                    exit(x * 100)
                } else {
                    Continuation::pure(x + 5)
                }
            })
        });
        let result = cont.run(|x| x);
        assert_eq!(result, 6); // 1 + 5 = 6, since 1 is not > 10
    }

    #[rstest]
    fn test_continuation_call_cc_early_return_triggered() {
        let cont = Continuation::call_with_current_continuation_once(|exit| {
            Continuation::pure(20).flat_map(move |x| {
                if x > 10 {
                    exit(x * 100)
                } else {
                    Continuation::pure(x + 5)
                }
            })
        });
        let result = cont.run(|x| x);
        assert_eq!(result, 2000); // 20 > 10, so exit(20 * 100) = 2000
    }

    #[rstest]
    fn test_continuation_complex_composition() {
        let result: i32 = Continuation::pure(10)
            .flat_map(|x| Continuation::pure(x + 5))
            .flat_map(|x| Continuation::pure(x * 2))
            .map(|x| x + 1)
            .run(|x| x);

        // (10 + 5) * 2 + 1 = 31
        assert_eq!(result, 31);
    }
}
