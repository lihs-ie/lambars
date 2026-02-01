//! Scala-style for-comprehension macro for iterators with async support.
//!
//! This module provides the [`for_async!`] macro, which allows writing nested
//! iterations with async operations in a flat, declarative style.
//! The result is wrapped in `AsyncIO<Vec<T>>`, ensuring deferred execution
//! and referential transparency.
//!
//! # Overview
//!
//! The `for_async!` macro is the asynchronous version of [`for_!`](crate::for_!).
//! It transforms nested iterations into `AsyncIO<Vec<T>>`, allowing async
//! operations to be performed on each element.
//!
//! # Syntax
//!
//! ```text
//! for_async! {
//!     pattern <= collection;           // Bind: iterate over collection
//!     pattern <~ async_io_expression;  // Async Bind: await AsyncIO result
//!     if let pattern = expression;     // Pattern Guard: match pattern (skip if no match)
//!     if condition;                    // Guard: filter by condition
//!     let pattern = expression;        // Pure let binding
//!     yield expression                 // Final expression (collected in Vec)
//! }
//! ```
//!
//! # Operators
//!
//! - `<=`: Collection bind - iterates over an `IntoIterator`
//! - `<~`: `AsyncIO` bind - awaits an `AsyncIO` and binds the result
//!
//! # Examples
//!
//! ## Basic Iteration
//!
//! ```rust,ignore
//! use lambars::for_async;
//! use lambars::effect::AsyncIO;
//!
//! #[tokio::main]
//! async fn main() {
//!     let result = for_async! {
//!         x <= vec![1, 2, 3];
//!         yield x * 2
//!     };
//!     assert_eq!(result.await, vec![2, 4, 6]);
//! }
//! ```
//!
//! ## With Async Operations
//!
//! ```rust,ignore
//! use lambars::for_async;
//! use lambars::effect::AsyncIO;
//!
//! #[tokio::main]
//! async fn main() {
//!     let result = for_async! {
//!         x <= vec![1, 2, 3];
//!         data <~ AsyncIO::pure(x * 10);  // AsyncIO bind
//!         let processed = data + 1;
//!         yield processed
//!     };
//!     assert_eq!(result.await, vec![11, 21, 31]);
//! }
//! ```
//!
//! ## Nested Iteration
//!
//! ```rust,ignore
//! use lambars::for_async;
//! use lambars::effect::AsyncIO;
//!
//! #[tokio::main]
//! async fn main() {
//!     let result = for_async! {
//!         x <= vec![1, 2];
//!         y <= vec![10, 20];
//!         yield x + y
//!     };
//!     assert_eq!(result.await, vec![11, 21, 12, 22]);
//! }
//! ```
//!
//! # Deferred Execution
//!
//! The returned `AsyncIO<Vec<T>>` is lazily evaluated. No computation
//! occurs until `.await` is called.
//!
//! # Differences from `for_!`
//!
//! | Feature | `for_!` | `for_async!` |
//! |---------|---------|--------------|
//! | Result type | `Vec<T>` | `AsyncIO<Vec<T>>` |
//! | Execution | Immediate | Deferred |
//! | Async support | No | Yes (`<~` operator) |
//! | Feature flag | Always available | Requires `async` feature |
//!
//! # Note on Clone
//!
//! When using outer variables in inner iterations, explicit `.clone()` is required,
//! consistent with the synchronous `for_!` macro.
//!
//! # Performance Tips
//!
//! ## Prefer `let` over `AsyncIO::pure()` for pure computations
//!
//! Pure computations should use `let` bindings instead of `AsyncIO::pure()` for
//! optimal performance:
//!
//! ```rust,ignore
//! // Recommended: Use let for pure computations
//! for_async! {
//!     x <= items;
//!     let doubled = x * 2;  // No overhead - direct value binding
//!     yield doubled
//! }
//!
//! // Not recommended: AsyncIO::pure() adds state machine overhead
//! for_async! {
//!     x <= items;
//!     doubled <~ AsyncIO::pure(x * 2);  // Unnecessary poll overhead
//!     yield doubled
//! }
//! ```
//!
//! ## When to use `<~` (async bind)
//!
//! Use `<~` only for actual async operations:
//!
//! ```rust,ignore
//! for_async! {
//!     x <= items;
//!     result <~ fetch_async(x);   // Actual async operation - use <~
//!     let processed = result * 2; // Pure computation - use let
//!     yield processed
//! }
//! ```
//!
//! ## Performance Comparison
//!
//! | Pattern | Overhead | Use Case |
//! |---------|----------|----------|
//! | `let x = expr;` | None | Pure computations |
//! | `x <~ AsyncIO::pure(expr);` | Minimal (poll overhead) | Avoid - use let instead |
//! | `x <~ async_operation();` | Inherent | Actual async operations |
//!
//! # Implementation Details
//!
//! ## Code Generation
//!
//! The `for_async!` macro generates a static async block wrapped in `AsyncIO::new()`.
//! The control flow (loops, guards, let bindings) is compiled into the async state
//! machine statically.
//!
//! ## Allocation Characteristics
//!
//! - **Outer wrapper**: One `Box<dyn Future>` allocation for the `AsyncIO::new()` call
//! - **Collection binds (`<=`)**: Standard `for` loops within the async block, no
//!   per-iteration allocation
//! - **Let bindings (`let`)**: Zero overhead, pure local variables
//!
//! ## Async Bind Behavior (`<~`)
//!
//! The `<~` operator calls `.await` on the `AsyncIO` expression.
//!
//! **Performance**: For `AsyncIO::pure(value)`, the direct await has zero
//! heap allocation overhead. For deferred `AsyncIO` operations, the allocation
//! is determined by the internal `AsyncIO` state.
//!
//! ## Performance Recommendations
//!
//! - Use `let` bindings for pure computations (zero overhead)
//! - Reserve `<~` for actual async operations that require deferred execution
//! - Avoid `x <~ AsyncIO::pure(expr)`; use `let x = expr;` instead for optimal performance

#![forbid(unsafe_code)]

/// A macro for Scala-style for-comprehension over iterators with async support.
///
/// This macro allows you to write nested iterations with async operations
/// in a flat, declarative style. The result is wrapped in `AsyncIO<Vec<T>>`,
/// ensuring deferred execution and referential transparency.
///
/// # Syntax
///
/// ```text
/// for_async! {
///     pattern <= collection;           // Bind: iterate over collection
///     pattern <~ async_io_expression;  // Async Bind: await AsyncIO result
///     if condition;                    // Guard: filter by condition
///     let pattern = expression;        // Pure let binding
///     yield expression                 // Final expression (collected in Vec)
/// }
/// ```
///
/// # Operators
///
/// - `<=`: Collection bind - iterates over an `IntoIterator`
/// - `<~`: `AsyncIO` bind - awaits an `AsyncIO` and binds the result
/// - `if condition;`: Guard expression - skips iteration if condition is false
///
/// # Examples
///
/// ## Basic iteration
///
/// ```rust,ignore
/// use lambars::for_async;
/// use lambars::effect::AsyncIO;
///
/// #[tokio::main]
/// async fn main() {
///     let result = for_async! {
///         x <= vec![1, 2, 3];
///         yield x * 2
///     };
///     assert_eq!(result.await, vec![2, 4, 6]);
/// }
/// ```
///
/// ## With async operations
///
/// ```rust,ignore
/// use lambars::for_async;
/// use lambars::effect::AsyncIO;
///
/// #[tokio::main]
/// async fn main() {
///     let result = for_async! {
///         x <= vec![1, 2, 3];
///         data <~ AsyncIO::pure(x * 10);
///         let processed = data + 1;
///         yield processed
///     };
///     assert_eq!(result.await, vec![11, 21, 31]);
/// }
/// ```
///
/// ## Nested iteration
///
/// ```rust,ignore
/// use lambars::for_async;
/// use lambars::effect::AsyncIO;
///
/// #[tokio::main]
/// async fn main() {
///     let result = for_async! {
///         x <= vec![1, 2];
///         y <= vec![10, 20];
///         yield x + y
///     };
///     assert_eq!(result.await, vec![11, 21, 12, 22]);
/// }
/// ```
///
/// ## Tuple pattern
///
/// ```rust,ignore
/// use lambars::for_async;
/// use lambars::effect::AsyncIO;
///
/// #[tokio::main]
/// async fn main() {
///     let pairs = vec![(1, "a"), (2, "b"), (3, "c")];
///     let result = for_async! {
///         (num, letter) <= pairs;
///         yield format!("{}{}", num, letter)
///     };
///     assert_eq!(result.await, vec!["1a", "2b", "3c"]);
/// }
/// ```
///
/// ## Wildcard pattern
///
/// ```rust,ignore
/// use lambars::for_async;
/// use lambars::effect::AsyncIO;
///
/// #[tokio::main]
/// async fn main() {
///     let result = for_async! {
///         _ <= vec![1, 2, 3];
///         yield "x"
///     };
///     assert_eq!(result.await, vec!["x", "x", "x"]);
/// }
/// ```
///
/// # Deferred Execution
///
/// The returned `AsyncIO<Vec<T>>` is lazily evaluated. No computation
/// occurs until `.await` is called.
///
/// # Note on Clone
///
/// When using outer variables in inner iterations, explicit `.clone()` is required,
/// consistent with the synchronous `for_!` macro.
#[cfg(feature = "async")]
#[macro_export]
macro_rules! for_async {
    // ==========================================================================
    // Entry point: wrap everything in AsyncIO::new
    // ==========================================================================

    // Entry with collection bind (identifier pattern)
    ($pattern:ident <= $collection:expr ; $($rest:tt)+) => {{
        $crate::effect::AsyncIO::new(move || async move {
            let mut __results = Vec::new();
            for $pattern in $collection.into_iter() {
                $crate::for_async!(@inner __results; $($rest)+);
            }
            __results
        })
    }};

    // Entry with collection bind (tuple pattern)
    (($($pattern:tt)*) <= $collection:expr ; $($rest:tt)+) => {{
        $crate::effect::AsyncIO::new(move || async move {
            let mut __results = Vec::new();
            for ($($pattern)*) in $collection.into_iter() {
                $crate::for_async!(@inner __results; $($rest)+);
            }
            __results
        })
    }};

    // Entry with collection bind (wildcard pattern)
    (_ <= $collection:expr ; $($rest:tt)+) => {{
        $crate::effect::AsyncIO::new(move || async move {
            let mut __results = Vec::new();
            for _ in $collection.into_iter() {
                $crate::for_async!(@inner __results; $($rest)+);
            }
            __results
        })
    }};

    // ==========================================================================
    // Internal rules (@inner): process the rest of the comprehension
    // ==========================================================================

    // Terminal case: yield expression
    (@inner $results:ident; yield $result:expr) => {{
        $results.push($result);
    }};

    // =========================================================================
    // @inner pattern guard rules (if let pattern = expression;)
    // Must be placed BEFORE regular guard rules for correct matching
    // =========================================================================

    // @inner pattern guard (with following statements)
    // Uses $pattern:pat to match any pattern (Rust 2021+)
    (@inner $results:ident; if let $pattern:pat = $expr:expr ; $($rest:tt)+) => {{
        if let $pattern = $expr {
            $crate::for_async!(@inner $results; $($rest)+);
        }
    }};

    // =========================================================================
    // @inner guard expression rules
    // =========================================================================

    // @inner guard expression (with following statements)
    // If condition is true, continue with rest; otherwise skip (no push)
    (@inner $results:ident; if $condition:expr ; $($rest:tt)+) => {{
        if $condition {
            $crate::for_async!(@inner $results; $($rest)+);
        }
    }};

    // AsyncIO bind with identifier pattern
    (@inner $results:ident; $pattern:ident <~ $async_io:expr ; $($rest:tt)+) => {{
        let $pattern = $async_io.await;
        $crate::for_async!(@inner $results; $($rest)+);
    }};

    // AsyncIO bind with tuple pattern
    (@inner $results:ident; ($($pattern:tt)*) <~ $async_io:expr ; $($rest:tt)+) => {{
        let ($($pattern)*) = $async_io.await;
        $crate::for_async!(@inner $results; $($rest)+);
    }};

    // AsyncIO bind with wildcard pattern
    (@inner $results:ident; _ <~ $async_io:expr ; $($rest:tt)+) => {{
        let _ = $async_io.await;
        $crate::for_async!(@inner $results; $($rest)+);
    }};

    // Collection bind with identifier pattern
    (@inner $results:ident; $pattern:ident <= $collection:expr ; $($rest:tt)+) => {{
        for $pattern in $collection.into_iter() {
            $crate::for_async!(@inner $results; $($rest)+);
        }
    }};

    // Collection bind with tuple pattern
    (@inner $results:ident; ($($pattern:tt)*) <= $collection:expr ; $($rest:tt)+) => {{
        for ($($pattern)*) in $collection.into_iter() {
            $crate::for_async!(@inner $results; $($rest)+);
        }
    }};

    // Collection bind with wildcard pattern
    (@inner $results:ident; _ <= $collection:expr ; $($rest:tt)+) => {{
        for _ in $collection.into_iter() {
            $crate::for_async!(@inner $results; $($rest)+);
        }
    }};

    // Pure let binding with identifier
    (@inner $results:ident; let $pattern:ident = $expr:expr ; $($rest:tt)+) => {{
        let $pattern = $expr;
        $crate::for_async!(@inner $results; $($rest)+);
    }};

    // Pure let binding with tuple pattern
    (@inner $results:ident; let ($($pattern:tt)*) = $expr:expr ; $($rest:tt)+) => {{
        let ($($pattern)*) = $expr;
        $crate::for_async!(@inner $results; $($rest)+);
    }};
}

#[cfg(all(feature = "async", test))]
#[allow(deprecated)]
mod tests {
    #[tokio::test]
    async fn test_inline_single_iteration() {
        let result = for_async! {
            x <= vec![1, 2, 3];
            yield x * 2
        };
        assert_eq!(result.await, vec![2, 4, 6]);
    }

    // =========================================================================
    // Guard expression tests
    // =========================================================================

    #[tokio::test]
    async fn test_async_guard_basic() {
        let result = for_async! {
            x <= vec![1, 2, 3, 4, 5];
            if x % 2 == 0;
            yield x
        };
        assert_eq!(result.await, vec![2, 4]);
    }

    #[tokio::test]
    async fn test_async_guard_multiple() {
        let result = for_async! {
            x <= 1..=20i32;
            if x % 2 == 0;
            if x > 10;
            yield x
        };
        assert_eq!(result.await, vec![12, 14, 16, 18, 20]);
    }

    // =========================================================================
    // Pattern guard tests
    // =========================================================================

    #[tokio::test]
    async fn test_async_pattern_guard_option_some() {
        fn maybe_double(x: i32) -> Option<i32> {
            if x > 0 { Some(x * 2) } else { None }
        }

        let result = for_async! {
            x <= vec![-1, 0, 1, 2, 3];
            if let Some(doubled) = maybe_double(x);
            yield doubled
        };
        assert_eq!(result.await, vec![2, 4, 6]);
    }

    #[tokio::test]
    async fn test_async_pattern_guard_with_regular_guard() {
        let items = vec![Some(1), None, Some(5), Some(10)];
        let result = for_async! {
            item <= items;
            if let Some(value) = item;
            if value > 3;
            yield value
        };
        assert_eq!(result.await, vec![5, 10]);
    }
}
