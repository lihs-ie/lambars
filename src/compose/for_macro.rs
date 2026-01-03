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
//!     if condition;             // Guard: filter by condition
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
//! - **Guard expression**: `if condition;` (skips iteration if condition is false)
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
//! ## Guard Expression (Filtering)
//!
//! ```rust
//! use lambars::for_;
//!
//! // Basic guard - filter even numbers
//! let result = for_! {
//!     x <= vec![1, 2, 3, 4, 5];
//!     if x % 2 == 0;
//!     yield x
//! };
//! assert_eq!(result, vec![2, 4]);
//! ```
//!
//! ## Multiple Guards
//!
//! ```rust
//! use lambars::for_;
//!
//! // Multiple guards act as AND conditions
//! let result = for_! {
//!     x <= 1..=100i32;
//!     if x % 2 == 0;
//!     if x % 3 == 0;
//!     if x < 50;
//!     yield x
//! };
//! assert_eq!(result, vec![6, 12, 18, 24, 30, 36, 42, 48]);
//! ```
//!
//! ## Guard with Let Binding
//!
//! ```rust
//! use lambars::for_;
//!
//! let result = for_! {
//!     x <= vec![1, 2, 3, 4, 5];
//!     let squared = x * x;
//!     if squared > 10;
//!     yield squared
//! };
//! assert_eq!(result, vec![16, 25]);
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
//! The macro uses internal rules for optimized expansion:
//!
//! ## Optimizations
//!
//! 1. **Single iteration uses `map`**: When there's only one iteration followed by `yield`,
//!    the macro uses `map` instead of `flat_map` + `vec![]` for better performance.
//!
//! 2. **Entry points delegate to optimized rules**: Public entry points delegate to
//!    internal `@collect` rules that handle different patterns optimally.
//!
//! ## Expansion Example
//!
//! ```rust,ignore
//! // Single iteration:
//! for_! { x <= xs; yield x * 2 }
//! // Expands to:
//! xs.into_iter().map(|x| x * 2).collect::<Vec<_>>()
//!
//! // Nested iteration:
//! for_! { x <= xs; y <= ys; yield x + y }
//! // Expands to:
//! xs.into_iter().flat_map(|x| {
//!     ys.into_iter().flat_map(|y| {
//!         vec![x + y]
//!     }).collect::<Vec<_>>()
//! }).collect::<Vec<_>>()
//! ```
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
///     if condition;             // Guard: filter by condition
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
///
/// ## Guard expression
///
/// ```rust
/// use lambars::for_;
///
/// let result = for_! {
///     x <= vec![1, 2, 3, 4, 5];
///     if x % 2 == 0;
///     yield x
/// };
/// assert_eq!(result, vec![2, 4]);
/// ```
#[macro_export]
macro_rules! for_ {
    // =========================================================================
    // Internal rules: @collect for optimized expansion
    // =========================================================================

    // @collect single iteration with identifier: use map (optimization)
    // This rule must come BEFORE general rules for proper matching
    (@collect $pattern:ident <= $collection:expr ; yield $result:expr) => {{
        $collection.into_iter().map(|$pattern| $result).collect::<Vec<_>>()
    }};

    // @collect single iteration with tuple pattern: use map
    (@collect ($($pattern:tt)*) <= $collection:expr ; yield $result:expr) => {{
        $collection.into_iter().map(|($($pattern)*)| $result).collect::<Vec<_>>()
    }};

    // @collect single iteration with wildcard: use map
    (@collect _ <= $collection:expr ; yield $result:expr) => {{
        $collection.into_iter().map(|_| $result).collect::<Vec<_>>()
    }};

    // @collect with identifier pattern (general nested case)
    (@collect $pattern:ident <= $collection:expr ; $($rest:tt)+) => {{
        $collection.into_iter().flat_map(|$pattern| {
            $crate::for_!($($rest)+)
        }).collect::<Vec<_>>()
    }};

    // @collect with tuple pattern (general nested case)
    (@collect ($($pattern:tt)*) <= $collection:expr ; $($rest:tt)+) => {{
        $collection.into_iter().flat_map(|($($pattern)*)| {
            $crate::for_!($($rest)+)
        }).collect::<Vec<_>>()
    }};

    // @collect with wildcard pattern (general nested case)
    (@collect _ <= $collection:expr ; $($rest:tt)+) => {{
        $collection.into_iter().flat_map(|_| {
            $crate::for_!($($rest)+)
        }).collect::<Vec<_>>()
    }};

    // =========================================================================
    // @collect guard expression rules
    // =========================================================================

    // @collect guard expression followed by yield (terminal optimization)
    // If condition is true, wrap result in vec; otherwise return empty vec
    (@collect if $condition:expr ; yield $result:expr) => {{
        if $condition {
            vec![$result]
        } else {
            vec![]
        }
    }};

    // @collect guard expression (with following statements)
    // If condition is true, continue with rest; otherwise return empty vec
    (@collect if $condition:expr ; $($rest:tt)+) => {{
        if $condition {
            $crate::for_!(@collect $($rest)+)
        } else {
            vec![]
        }
    }};

    // @collect let binding with identifier
    (@collect let $pattern:ident = $expr:expr ; $($rest:tt)+) => {{
        let $pattern = $expr;
        $crate::for_!(@collect $($rest)+)
    }};

    // @collect let binding with tuple pattern
    (@collect let ($($pattern:tt)*) = $expr:expr ; $($rest:tt)+) => {{
        let ($($pattern)*) = $expr;
        $crate::for_!(@collect $($rest)+)
    }};

    // @collect terminal case: yield wraps result in vec![]
    (@collect yield $result:expr) => {{
        vec![$result]
    }};

    // =========================================================================
    // Public entry points
    // =========================================================================

    // Terminal case: yield wraps result in vec![]
    (yield $result:expr) => {
        vec![$result]
    };

    // Bind with identifier pattern - delegates to @collect
    ($pattern:ident <= $collection:expr ; $($rest:tt)+) => {{
        $crate::for_!(@collect $pattern <= $collection ; $($rest)+)
    }};

    // Bind with tuple pattern - delegates to @collect
    (($($pattern:tt)*) <= $collection:expr ; $($rest:tt)+) => {{
        $crate::for_!(@collect ($($pattern)*) <= $collection ; $($rest)+)
    }};

    // Bind with wildcard pattern - delegates to @collect
    (_ <= $collection:expr ; $($rest:tt)+) => {{
        $crate::for_!(@collect _ <= $collection ; $($rest)+)
    }};

    // Guard expression - delegates to @collect
    (if $condition:expr ; $($rest:tt)+) => {{
        $crate::for_!(@collect if $condition ; $($rest)+)
    }};

    // Pure let binding with identifier
    (let $pattern:ident = $expr:expr ; $($rest:tt)+) => {{
        let $pattern = $expr;
        $crate::for_!($($rest)+)
    }};

    // Pure let binding with tuple pattern
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

    // =========================================================================
    // Tests for @collect optimization
    // =========================================================================

    #[test]
    fn test_collect_single_iteration_uses_map() {
        // This should use map optimization
        let result = for_! {
            x <= vec![1, 2, 3];
            yield x * 2
        };
        assert_eq!(result, vec![2, 4, 6]);
    }

    #[test]
    fn test_collect_with_tuple_pattern() {
        let result = for_!(@collect (a, b) <= vec![(1, 2), (3, 4)]; yield a + b);
        assert_eq!(result, vec![3, 7]);
    }

    #[test]
    fn test_collect_with_wildcard_pattern() {
        let result = for_!(@collect _ <= vec![1, 2, 3]; yield 42);
        assert_eq!(result, vec![42, 42, 42]);
    }

    // =========================================================================
    // Edge case tests from implementation plan
    // =========================================================================

    #[test]
    fn test_empty_collection_edge_case() {
        let result = for_! { x <= Vec::<i32>::new(); yield x };
        assert!(result.is_empty());
    }

    #[test]
    fn test_single_element_collection() {
        let result = for_! { x <= vec![42]; yield x * 2 };
        assert_eq!(result, vec![84]);
    }

    #[test]
    fn test_four_level_nesting() {
        let result = for_! {
            a <= vec![1, 2];
            b <= vec![10, 20];
            c <= vec![100, 200];
            d <= vec![1000, 2000];
            yield a + b + c + d
        };
        assert_eq!(result.len(), 16);
        // Verify first and last elements
        assert_eq!(result[0], 1 + 10 + 100 + 1000); // 1111
        assert_eq!(result[15], 2 + 20 + 200 + 2000); // 2222
    }

    #[test]
    fn test_three_level_nesting() {
        let result = for_! {
            x <= vec![1, 2];
            y <= vec![10, 20];
            z <= vec![100, 200];
            yield x + y + z
        };
        assert_eq!(result, vec![111, 211, 121, 221, 112, 212, 122, 222]);
    }

    #[test]
    fn test_single_iteration_with_let_binding() {
        let result = for_! {
            x <= vec![1, 2, 3];
            let y = x * 2;
            yield y
        };
        assert_eq!(result, vec![2, 4, 6]);
    }

    #[test]
    fn test_nested_with_let_binding_in_middle() {
        let result = for_! {
            x <= vec![1, 2];
            let x_squared = x * x;
            y <= vec![10, 20];
            yield x_squared + y
        };
        assert_eq!(result, vec![11, 21, 14, 24]);
    }

    // =========================================================================
    // Guard expression tests
    // =========================================================================

    #[test]
    fn test_guard_basic_filter() {
        let result = for_! {
            x <= vec![1, 2, 3, 4, 5];
            if x % 2 == 0;
            yield x
        };
        assert_eq!(result, vec![2, 4]);
    }

    #[test]
    fn test_guard_all_pass() {
        let result = for_! {
            x <= vec![2, 4, 6];
            if x % 2 == 0;
            yield x
        };
        assert_eq!(result, vec![2, 4, 6]);
    }

    #[test]
    fn test_guard_all_fail() {
        let result = for_! {
            x <= vec![1, 3, 5];
            if x % 2 == 0;
            yield x
        };
        assert!(result.is_empty());
    }

    #[test]
    fn test_guard_empty_collection() {
        let result = for_! {
            x <= Vec::<i32>::new();
            if x > 0;
            yield x
        };
        assert!(result.is_empty());
    }

    #[test]
    fn test_guard_after_let() {
        let result = for_! {
            x <= vec![1, 2, 3, 4, 5];
            let squared = x * x;
            if squared > 10;
            yield squared
        };
        assert_eq!(result, vec![16, 25]);
    }

    #[test]
    fn test_guard_nested() {
        let result = for_! {
            x <= vec![1, 2];
            y <= vec![10, 20];
            if x + y > 15;
            yield (x, y)
        };
        assert_eq!(result, vec![(1, 20), (2, 20)]);
    }

    #[test]
    fn test_guard_multiple() {
        let result = for_! {
            x <= 1..=20i32;
            if x % 2 == 0;
            if x > 10;
            yield x
        };
        assert_eq!(result, vec![12, 14, 16, 18, 20]);
    }

    #[test]
    fn test_guard_between_binds() {
        let result = for_! {
            x <= vec![1, 2, 3];
            if x % 2 == 1;
            y <= vec![10, 20];
            yield (x, y)
        };
        assert_eq!(result, vec![(1, 10), (1, 20), (3, 10), (3, 20)]);
    }
}
