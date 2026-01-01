//! Do-notation macro for `AsyncIO` monad.
//!
//! This module provides the `eff_async!` macro, which allows for Haskell-style
//! do-notation when working with `AsyncIO` values.
//!
//! # Syntax
//!
//! The `eff_async!` macro uses `<=` as the bind operator (since `<-` cannot be
//! matched in Rust macros).
//!
//! ```text
//! eff_async! {
//!     pattern <= async_io_expression;  // bind: pattern receives the value from AsyncIO
//!     let pattern = expression;         // pure let: regular let binding
//!     ...
//!     async_io_expression              // final expression: must return AsyncIO
//! }
//! ```
//!
//! # Examples
//!
//! Basic usage:
//!
//! ```rust,ignore
//! use lambars::effect::AsyncIO;
//! use lambars::eff_async;
//!
//! #[tokio::main]
//! async fn main() {
//!     let result = eff_async! {
//!         x <= AsyncIO::pure(5);
//!         y <= AsyncIO::pure(10);
//!         let z = x + y;
//!         AsyncIO::pure(z * 2)
//!     };
//!     assert_eq!(result.run_async().await, 30);
//! }
//! ```
//!
//! With async operations:
//!
//! ```rust,ignore
//! use lambars::effect::AsyncIO;
//! use lambars::eff_async;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     let result = eff_async! {
//!         _ <= AsyncIO::delay_async(Duration::from_millis(10));
//!         data <= fetch_data_async();
//!         let processed = process(data);
//!         validated <= validate_async(processed);
//!         AsyncIO::pure(validated)
//!     };
//! }
//! ```

/// Do-notation macro for `AsyncIO` monad.
///
/// This macro provides a convenient syntax for chaining `AsyncIO` operations,
/// similar to Haskell's do-notation.
///
/// # Syntax
///
/// - `pattern <= async_io_expr;` - Bind: executes the `AsyncIO` and binds the result
/// - `let pattern = expr;` - Pure let: regular Rust let binding
/// - `async_io_expr` - Final expression: must return an `AsyncIO`
///
/// # Note
///
/// The `<=` operator is used instead of `<-` because Rust macros cannot match
/// the `<-` token sequence.
///
/// # Examples
///
/// ```rust,ignore
/// use lambars::effect::AsyncIO;
/// use lambars::eff_async;
///
/// #[tokio::main]
/// async fn main() {
///     let result = eff_async! {
///         x <= AsyncIO::pure(1);
///         y <= AsyncIO::pure(2);
///         AsyncIO::pure(x + y)
///     };
///     assert_eq!(result.run_async().await, 3);
/// }
/// ```
#[macro_export]
macro_rules! eff_async {
    // Terminal case: single expression (must be an AsyncIO)
    ($result:expr) => {
        $result
    };

    // Bind with identifier pattern: `identifier <= async_io; rest`
    ($pattern:ident <= $monad:expr ; $($rest:tt)+) => {
        $monad.flat_map(move |$pattern| {
            $crate::eff_async!($($rest)+)
        })
    };

    // Bind with tuple pattern: `(pattern1, pattern2) <= async_io; rest`
    (($($pattern:tt)*) <= $monad:expr ; $($rest:tt)+) => {
        $monad.flat_map(move |($($pattern)*)| {
            $crate::eff_async!($($rest)+)
        })
    };

    // Bind with wildcard pattern: `_ <= async_io; rest`
    (_ <= $monad:expr ; $($rest:tt)+) => {
        $monad.flat_map(move |_| {
            $crate::eff_async!($($rest)+)
        })
    };

    // Pure let binding with identifier: `let identifier = expr; rest`
    (let $pattern:ident = $expr:expr ; $($rest:tt)+) => {
        {
            let $pattern = $expr;
            $crate::eff_async!($($rest)+)
        }
    };

    // Pure let binding with tuple pattern: `let (a, b) = expr; rest`
    (let ($($pattern:tt)*) = $expr:expr ; $($rest:tt)+) => {
        {
            let ($($pattern)*) = $expr;
            $crate::eff_async!($($rest)+)
        }
    };

    // Pure let binding with type annotation: `let identifier: Type = expr; rest`
    (let $pattern:ident : $ty:ty = $expr:expr ; $($rest:tt)+) => {
        {
            let $pattern: $ty = $expr;
            $crate::eff_async!($($rest)+)
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::effect::AsyncIO;

    #[tokio::test]
    async fn test_eff_async_single_bind() {
        let result = eff_async! {
            x <= AsyncIO::pure(5);
            AsyncIO::pure(x * 2)
        };
        assert_eq!(result.run_async().await, 10);
    }

    #[tokio::test]
    async fn test_eff_async_multiple_binds() {
        let result = eff_async! {
            x <= AsyncIO::pure(5);
            y <= AsyncIO::pure(10);
            AsyncIO::pure(x + y)
        };
        assert_eq!(result.run_async().await, 15);
    }

    #[tokio::test]
    async fn test_eff_async_with_let() {
        let result = eff_async! {
            x <= AsyncIO::pure(5);
            let doubled = x * 2;
            y <= AsyncIO::pure(10);
            AsyncIO::pure(doubled + y)
        };
        assert_eq!(result.run_async().await, 20);
    }

    #[tokio::test]
    async fn test_eff_async_wildcard_pattern() {
        let result = eff_async! {
            _ <= AsyncIO::pure("ignored");
            AsyncIO::pure(42)
        };
        assert_eq!(result.run_async().await, 42);
    }

    #[tokio::test]
    async fn test_eff_async_tuple_pattern() {
        let result = eff_async! {
            (x, y) <= AsyncIO::pure((10, 20));
            AsyncIO::pure(x + y)
        };
        assert_eq!(result.run_async().await, 30);
    }
}
