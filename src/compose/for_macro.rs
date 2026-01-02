//! Scala-style for-comprehension macro for iterators.
//!
//! This module provides the [`for_!`] macro, which allows writing nested
//! iterations in a flat, declarative style similar to Scala's for-comprehension.
//!
//! # Overview
//!
//! The `for_!` macro transforms nested `flat_map` operations into a more
//! readable, imperative-looking syntax. Unlike the [`eff!`](crate::eff!) macro
//! which works with monads (Option, Result, IO, etc.), `for_!` is specifically
//! designed for iterator-based operations and collects results into a `Vec`.
//!
//! # Syntax
//!
//! ```text
//! for_! {
//!     pattern <= collection;    // Bind: iterate over collection
//!     let pattern = expression; // Pure let binding
//!     yield expression          // Final expression (wrapped in Vec)
//! }
//! ```
//!
//! # Supported Patterns
//!
//! - **Identifier pattern**: `x <= collection;`
//! - **Tuple pattern**: `(a, b) <= collection;`
//! - **Wildcard pattern**: `_ <= collection;`
//! - **Let binding (identifier)**: `let x = expression;`
//! - **Let binding (tuple)**: `let (a, b) = expression;`
//!
//! # Operator Choice: `<=`
//!
//! We use `<=` as the bind operator because:
//! - `<-` is not valid in Rust's macro patterns
//! - `<=` is visually similar to `<-` and suggests "bind from"
//! - It maintains consistency with the [`eff!`](crate::eff!) macro
//!
//! # Examples
//!
//! ## Basic Iteration
//!
//! ```rust
//! use lambars::for_;
//!
//! let result = for_! {
//!     x <= vec![1, 2, 3];
//!     yield x * 2
//! };
//! assert_eq!(result, vec![2, 4, 6]);
//! ```
//!
//! ## Nested Iteration
//!
//! ```rust
//! use lambars::for_;
//!
//! let result = for_! {
//!     x <= vec![1, 2];
//!     y <= vec![10, 20];
//!     yield x + y
//! };
//! assert_eq!(result, vec![11, 21, 12, 22]);
//! ```
//!
//! ## With Let Bindings
//!
//! ```rust
//! use lambars::for_;
//!
//! let result = for_! {
//!     x <= vec![1, 2, 3];
//!     let doubled = x * 2;
//!     yield doubled + 1
//! };
//! assert_eq!(result, vec![3, 5, 7]);
//! ```
//!
//! ## Tuple Pattern
//!
//! ```rust
//! use lambars::for_;
//!
//! let pairs = vec![(1, "a"), (2, "b")];
//! let result = for_! {
//!     (num, letter) <= pairs;
//!     yield format!("{}{}", num, letter)
//! };
//! assert_eq!(result, vec!["1a", "2b"]);
//! ```
//!
//! ## Scala-style Recommendation Feed Example
//!
//! ```rust
//! use lambars::for_;
//!
//! #[derive(Clone)]
//! struct Book {
//!     title: String,
//!     authors: Vec<String>,
//! }
//!
//! #[derive(Clone)]
//! struct Movie {
//!     title: String,
//! }
//!
//! fn book_adaptations(author: &str) -> Vec<Movie> {
//!     match author {
//!         "Tolkien" => vec![Movie { title: "LOTR".to_string() }],
//!         _ => vec![],
//!     }
//! }
//!
//! let books = vec![
//!     Book {
//!         title: "The Hobbit".to_string(),
//!         authors: vec!["Tolkien".to_string()],
//!     },
//! ];
//!
//! let result = for_! {
//!     book <= books.clone();
//!     author <= book.authors.clone();
//!     movie <= book_adaptations(&author);
//!     yield format!(
//!         "You may like {}, because you liked {}'s {}",
//!         movie.title, author, book.title
//!     )
//! };
//!
//! assert_eq!(result, vec!["You may like LOTR, because you liked Tolkien's The Hobbit"]);
//! ```
//!
//! # Implementation Details
//!
//! The macro expands `pattern <= collection; rest` into:
//!
//! ```rust,ignore
//! collection.into_iter().flat_map(|pattern| {
//!     /* expanded rest */
//! }).collect::<Vec<_>>()
//! ```
//!
//! The terminal `yield expression` expands to `vec![expression]`.
//!
//! # Differences from eff! macro
//!
//! | Feature | `for_!` | `eff!` |
//! |---------|---------|--------|
//! | Target | Iterators | Monads |
//! | Result | `Vec<T>` | Monad type |
//! | Terminal | `yield expression` | `expression` |
//! | Method | `into_iter().flat_map()` | `flat_map()` |
//!
//! # Important Notes on Clone
//!
//! When using outer variables inside inner iterations, you must explicitly
//! clone them:
//!
//! ```rust
//! use lambars::for_;
//!
//! let xs = vec![1, 2];
//! let ys = vec![10, 20];
//!
//! // ys must be cloned because it's used in the inner loop
//! let result = for_! {
//!     x <= xs;
//!     y <= ys.clone();  // Explicit clone required
//!     yield x + y
//! };
//! ```
//!
//! This is intentional to:
//! - Maintain Rust's explicit ownership semantics
//! - Avoid hidden performance costs
//! - Make the code predictable and debuggable

#![forbid(unsafe_code)]

/// A macro for Scala-style for-comprehension over iterators.
///
/// This macro allows you to write nested iterations in a flat,
/// declarative style, similar to Scala's for-comprehension.
///
/// # Syntax
///
/// ```text
/// for_! {
///     pattern <= collection;    // Bind: iterate over collection
///     let pattern = expression; // Pure let binding
///     yield expression          // Final expression (wrapped in Vec)
/// }
/// ```
///
/// # Examples
///
/// ## Basic iteration
///
/// ```rust
/// use lambars::for_;
///
/// let result = for_! {
///     x <= vec![1, 2, 3];
///     yield x * 2
/// };
/// assert_eq!(result, vec![2, 4, 6]);
/// ```
///
/// ## Nested iteration
///
/// ```rust
/// use lambars::for_;
///
/// let result = for_! {
///     x <= vec![1, 2];
///     y <= vec![10, 20];
///     yield x + y
/// };
/// assert_eq!(result, vec![11, 21, 12, 22]);
/// ```
///
/// ## With let bindings
///
/// ```rust
/// use lambars::for_;
///
/// let result = for_! {
///     x <= vec![1, 2, 3];
///     let doubled = x * 2;
///     yield doubled + 1
/// };
/// assert_eq!(result, vec![3, 5, 7]);
/// ```
///
/// ## Tuple pattern
///
/// ```rust
/// use lambars::for_;
///
/// let pairs = vec![(1, "a"), (2, "b")];
/// let result = for_! {
///     (num, letter) <= pairs;
///     yield format!("{}{}", num, letter)
/// };
/// assert_eq!(result, vec!["1a", "2b"]);
/// ```
///
/// ## Wildcard pattern
///
/// ```rust
/// use lambars::for_;
///
/// let result = for_! {
///     _ <= vec![1, 2, 3];
///     yield "x"
/// };
/// assert_eq!(result, vec!["x", "x", "x"]);
/// ```
#[macro_export]
macro_rules! for_ {
    // ==========================================================================
    // Terminal case: yield wraps result in vec![]
    // ==========================================================================

    (yield $result:expr) => {
        vec![$result]
    };

    // ==========================================================================
    // Bind operation: pattern <= collection; rest
    // ==========================================================================

    // Bind with identifier pattern
    // This is the most common case: x <= collection;
    ($pattern:ident <= $collection:expr ; $($rest:tt)+) => {{
        $collection.into_iter().flat_map(|$pattern| {
            $crate::for_!($($rest)+)
        }).collect::<Vec<_>>()
    }};

    // Bind with tuple pattern
    // Handles cases like: (a, b) <= collection;
    (($($pattern:tt)*) <= $collection:expr ; $($rest:tt)+) => {{
        $collection.into_iter().flat_map(|($($pattern)*)| {
            $crate::for_!($($rest)+)
        }).collect::<Vec<_>>()
    }};

    // Bind with wildcard pattern
    // Handles cases like: _ <= collection;
    (_ <= $collection:expr ; $($rest:tt)+) => {{
        $collection.into_iter().flat_map(|_| {
            $crate::for_!($($rest)+)
        }).collect::<Vec<_>>()
    }};

    // ==========================================================================
    // Let binding: let pattern = expression; rest
    // ==========================================================================

    // Pure let binding with identifier
    // Handles cases like: let x = expr;
    (let $pattern:ident = $expr:expr ; $($rest:tt)+) => {{
        let $pattern = $expr;
        $crate::for_!($($rest)+)
    }};

    // Pure let binding with tuple pattern
    // Handles cases like: let (a, b) = expr;
    (let ($($pattern:tt)*) = $expr:expr ; $($rest:tt)+) => {{
        let ($($pattern)*) = $expr;
        $crate::for_!($($rest)+)
    }};
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_yield_only() {
        let result = for_! {
            yield 42
        };
        assert_eq!(result, vec![42]);
    }

    #[test]
    fn test_single_iteration() {
        let result = for_! {
            x <= vec![1, 2, 3];
            yield x * 2
        };
        assert_eq!(result, vec![2, 4, 6]);
    }

    #[test]
    fn test_nested_iteration() {
        let result = for_! {
            x <= vec![1, 2];
            y <= vec![10, 20];
            yield x + y
        };
        assert_eq!(result, vec![11, 21, 12, 22]);
    }

    #[test]
    fn test_tuple_pattern() {
        let pairs = vec![(1, "a"), (2, "b")];
        let result = for_! {
            (num, letter) <= pairs;
            yield format!("{}{}", num, letter)
        };
        assert_eq!(result, vec!["1a", "2b"]);
    }

    #[test]
    fn test_wildcard_pattern() {
        let result = for_! {
            _ <= vec![1, 2, 3];
            yield "x"
        };
        assert_eq!(result, vec!["x", "x", "x"]);
    }

    #[test]
    fn test_let_binding() {
        let result = for_! {
            x <= vec![1, 2, 3];
            let doubled = x * 2;
            yield doubled
        };
        assert_eq!(result, vec![2, 4, 6]);
    }

    #[test]
    fn test_let_tuple_binding() {
        let result = for_! {
            pair <= vec![(1, 2), (3, 4)];
            let (a, b) = pair;
            yield a + b
        };
        assert_eq!(result, vec![3, 7]);
    }

    #[test]
    fn test_empty_collection() {
        let empty: Vec<i32> = vec![];
        let result = for_! {
            x <= empty;
            yield x * 2
        };
        assert_eq!(result, Vec::<i32>::new());
    }
}
