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
//!     assert_eq!(result.run_async().await, vec![2, 4, 6]);
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
//!     assert_eq!(result.run_async().await, vec![11, 21, 31]);
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
//!     assert_eq!(result.run_async().await, vec![11, 21, 12, 22]);
//! }
//! ```
//!
//! # Deferred Execution
//!
//! The returned `AsyncIO<Vec<T>>` is lazily evaluated. No computation
//! occurs until `run_async().await` is called.
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
///     let pattern = expression;        // Pure let binding
///     yield expression                 // Final expression (collected in Vec)
/// }
/// ```
///
/// # Operators
///
/// - `<=`: Collection bind - iterates over an `IntoIterator`
/// - `<~`: `AsyncIO` bind - awaits an `AsyncIO` and binds the result
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
///     assert_eq!(result.run_async().await, vec![2, 4, 6]);
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
///     assert_eq!(result.run_async().await, vec![11, 21, 31]);
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
///     assert_eq!(result.run_async().await, vec![11, 21, 12, 22]);
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
///     assert_eq!(result.run_async().await, vec!["1a", "2b", "3c"]);
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
///     assert_eq!(result.run_async().await, vec!["x", "x", "x"]);
/// }
/// ```
///
/// # Deferred Execution
///
/// The returned `AsyncIO<Vec<T>>` is lazily evaluated. No computation
/// occurs until `run_async().await` is called.
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

    // AsyncIO bind with identifier pattern
    (@inner $results:ident; $pattern:ident <~ $async_io:expr ; $($rest:tt)+) => {{
        let $pattern = $async_io.run_async().await;
        $crate::for_async!(@inner $results; $($rest)+);
    }};

    // AsyncIO bind with tuple pattern
    (@inner $results:ident; ($($pattern:tt)*) <~ $async_io:expr ; $($rest:tt)+) => {{
        let ($($pattern)*) = $async_io.run_async().await;
        $crate::for_async!(@inner $results; $($rest)+);
    }};

    // AsyncIO bind with wildcard pattern
    (@inner $results:ident; _ <~ $async_io:expr ; $($rest:tt)+) => {{
        let _ = $async_io.run_async().await;
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
mod tests {
    #[tokio::test]
    async fn test_inline_single_iteration() {
        let result = for_async! {
            x <= vec![1, 2, 3];
            yield x * 2
        };
        assert_eq!(result.run_async().await, vec![2, 4, 6]);
    }
}
