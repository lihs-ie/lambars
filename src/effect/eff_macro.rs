//! eff! macro for do-notation style syntax.
//!
//! This module provides the `eff!` macro, which allows chaining monadic
//! operations in a more readable, imperative-looking style similar to
//! Haskell's do-notation or Scala's for-comprehension.
//!
//! # Syntax
//!
//! The macro supports the following constructs:
//!
//! - `pattern <= expression;` - Bind: extracts the value from a monad
//! - `let pattern = expression;` - Pure let binding
//! - `yield expression` - Final expression (wrapped using the monad's pure)
//! - `expression` - Final expression (already a monad)
//!
//! # Operator Choice: `<=`
//!
//! We use `<=` as the bind operator because:
//! - `<-` is not valid in Rust's macro patterns
//! - `<=` is visually similar to `<-` and suggests "bind from"
//! - It's a valid token in Rust macros
//!
//! # Examples
//!
//! ## Option
//!
//! ```rust
//! use functional_rusty::eff;
//! use functional_rusty::typeclass::Monad;
//!
//! let result = eff! {
//!     x <= Some(5);
//!     y <= Some(10);
//!     let z = x + y;
//!     Some(z * 2)
//! };
//! assert_eq!(result, Some(30));
//! ```
//!
//! ## State
//!
//! ```rust
//! use functional_rusty::eff;
//! use functional_rusty::effect::State;
//!
//! let computation: State<i32, i32> = eff! {
//!     current <= State::get();
//!     _ <= State::put(current + 1);
//!     new_state <= State::get();
//!     State::pure(new_state)
//! };
//!
//! let (result, final_state) = computation.run(0);
//! assert_eq!(result, 1);
//! assert_eq!(final_state, 1);
//! ```
//!
//! ## Reader
//!
//! ```rust
//! use functional_rusty::eff;
//! use functional_rusty::effect::Reader;
//!
//! let computation: Reader<i32, i32> = eff! {
//!     environment <= Reader::ask();
//!     let doubled = environment * 2;
//!     Reader::pure(doubled + 1)
//! };
//!
//! assert_eq!(computation.run(10), 21);
//! ```
//!
//! # Implementation Notes
//!
//! The macro expands `pattern <= expression; rest` into:
//! ```rust,ignore
//! expression.flat_map(|pattern| { /* rest */ })
//! ```
//!
//! For Option and Result, this uses the `Monad` trait's `flat_map` method.
//! For State, Reader, and Writer, this uses their inherent `flat_map` methods.

#![forbid(unsafe_code)]

/// A macro for monadic do-notation style syntax.
///
/// This macro allows you to write monadic computations in a more
/// imperative-looking style, similar to Haskell's do-notation.
///
/// # Syntax
///
/// ```text
/// eff! {
///     pattern <= monad_expression;    // Bind operation (flat_map)
///     let pattern = expression;        // Pure let binding
///     monad_expression                 // Final expression (must be a monad)
/// }
/// ```
///
/// # Examples
///
/// ```rust
/// use functional_rusty::eff;
/// use functional_rusty::typeclass::Monad;
///
/// // Option example
/// let result = eff! {
///     x <= Some(5);
///     y <= Some(10);
///     Some(x + y)
/// };
/// assert_eq!(result, Some(15));
///
/// // Short-circuit on None
/// let result: Option<i32> = eff! {
///     x <= Some(5);
///     y <= None::<i32>;
///     Some(x + y)
/// };
/// assert_eq!(result, None);
/// ```
#[macro_export]
macro_rules! eff {
    // ==========================================================================
    // Terminal cases
    // ==========================================================================

    // Case 1: Single expression (terminal) - return as-is
    ($result:expr) => {
        $result
    };

    // ==========================================================================
    // Bind operation: pattern <= monad; rest
    // ==========================================================================

    // Case 2: Bind with identifier pattern
    ($pattern:ident <= $monad:expr ; $($rest:tt)+) => {
        $monad.flat_map(move |$pattern| {
            $crate::eff!($($rest)+)
        })
    };

    // Case 3: Bind with tuple pattern
    (($($pattern:tt)*) <= $monad:expr ; $($rest:tt)+) => {
        $monad.flat_map(move |($($pattern)*)| {
            $crate::eff!($($rest)+)
        })
    };

    // Case 4: Bind with wildcard pattern
    (_ <= $monad:expr ; $($rest:tt)+) => {
        $monad.flat_map(move |_| {
            $crate::eff!($($rest)+)
        })
    };

    // ==========================================================================
    // Let binding: let pattern = expression; rest
    // ==========================================================================

    // Case 5: Pure let binding with identifier
    (let $pattern:ident = $expr:expr ; $($rest:tt)+) => {
        {
            let $pattern = $expr;
            $crate::eff!($($rest)+)
        }
    };

    // Case 6: Pure let binding with tuple pattern
    (let ($($pattern:tt)*) = $expr:expr ; $($rest:tt)+) => {
        {
            let ($($pattern)*) = $expr;
            $crate::eff!($($rest)+)
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::typeclass::Monad;

    #[test]
    fn basic_option_bind() {
        let result = eff! {
            x <= Some(5);
            y <= Some(10);
            Some(x + y)
        };
        assert_eq!(result, Some(15));
    }

    #[test]
    fn option_with_let() {
        let result = eff! {
            x <= Some(5);
            let doubled = x * 2;
            Some(doubled)
        };
        assert_eq!(result, Some(10));
    }

    #[test]
    fn option_short_circuit() {
        let result: Option<i32> = eff! {
            x <= Some(5);
            y <= None::<i32>;
            Some(x + y)
        };
        assert_eq!(result, None);
    }

    #[test]
    fn result_bind() {
        let result: Result<i32, &str> = eff! {
            x <= Ok(5);
            y <= Ok(10);
            Ok(x + y)
        };
        assert_eq!(result, Ok(15));
    }

    #[test]
    fn single_expression() {
        let result = eff! {
            Some(42)
        };
        assert_eq!(result, Some(42));
    }

    #[test]
    fn wildcard_pattern() {
        let result = eff! {
            _ <= Some(5);
            Some(42)
        };
        assert_eq!(result, Some(42));
    }

    #[test]
    fn tuple_pattern() {
        let result = eff! {
            (a, b) <= Some((1, 2));
            Some(a + b)
        };
        assert_eq!(result, Some(3));
    }
}
