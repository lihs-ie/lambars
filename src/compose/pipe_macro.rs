//! The `pipe!` macro for left-to-right function application.
//!
//! This module provides the [`pipe!`] macro which applies functions
//! from left to right, following the data flow style of programming.

/// Pipes a value through a series of functions from left to right.
///
/// `pipe!(x, f, g, h)` is equivalent to `h(g(f(x)))`.
///
/// This is the "data flow" style of function application, where the value
/// flows through transformations in the order they are written. This often
/// matches the mental model of processing data through a pipeline.
///
/// # Relationship with compose!
///
/// `pipe!(x, f, g, h)` is equivalent to `compose!(h, g, f)(x)`.
///
/// While [`compose!`](crate::compose!) creates a new function, `pipe!` immediately
/// applies the transformations to a value.
///
/// # Syntax
///
/// - `pipe!(x)` - Returns `x` unchanged
/// - `pipe!(x, f)` - Returns `f(x)`
/// - `pipe!(x, f, g)` - Returns `g(f(x))`
/// - `pipe!(x, f, g, h, ...)` - Returns `...h(g(f(x)))`
///
/// # Type Requirements
///
/// Each function only needs to implement [`FnOnce`], since each function
/// is called exactly once. This allows using functions that consume their
/// captured environment.
///
/// # Examples
///
/// ## Basic pipeline
///
/// ```
/// use functional_rusty::pipe;
///
/// fn add_one(x: i32) -> i32 { x + 1 }
/// fn double(x: i32) -> i32 { x * 2 }
///
/// // pipe!(x, f, g) = g(f(x)) = add_one(double(5)) = add_one(10) = 11
/// let result = pipe!(5, double, add_one);
/// assert_eq!(result, 11);
/// ```
///
/// ## Multi-step pipeline
///
/// ```
/// use functional_rusty::pipe;
///
/// fn square(x: i32) -> i32 { x * x }
/// fn double(x: i32) -> i32 { x * 2 }
/// fn add_one(x: i32) -> i32 { x + 1 }
///
/// // 3 -> square(3)=9 -> double(9)=18 -> add_one(18)=19
/// let result = pipe!(3, square, double, add_one);
/// assert_eq!(result, 19);
/// ```
///
/// ## Type conversion through pipeline
///
/// ```
/// use functional_rusty::pipe;
///
/// fn to_string(x: i32) -> String { x.to_string() }
/// fn get_length(s: String) -> usize { s.len() }
///
/// let result = pipe!(12345, to_string, get_length);
/// assert_eq!(result, 5);
/// ```
///
/// ## With consuming closures
///
/// ```
/// use functional_rusty::pipe;
///
/// fn consume_and_double(v: Vec<i32>) -> Vec<i32> {
///     v.into_iter().map(|x| x * 2).collect()
/// }
///
/// fn consume_and_filter(v: Vec<i32>) -> Vec<i32> {
///     v.into_iter().filter(|x| *x > 5).collect()
/// }
///
/// let result = pipe!(
///     vec![1, 2, 3, 4, 5],
///     consume_and_double,
///     consume_and_filter
/// );
/// assert_eq!(result, vec![6, 8, 10]);
/// ```
///
/// ## String processing pipeline
///
/// ```
/// use functional_rusty::pipe;
///
/// fn to_uppercase(s: &str) -> String { s.to_uppercase() }
/// fn add_exclamation(s: String) -> String { format!("{}!", s) }
///
/// let result = pipe!("hello", to_uppercase, add_exclamation);
/// assert_eq!(result, "HELLO!");
/// ```
///
/// ## Equivalence with compose
///
/// ```
/// use functional_rusty::{compose, pipe};
///
/// fn f(x: i32) -> i32 { x + 1 }
/// fn g(x: i32) -> i32 { x * 2 }
/// fn h(x: i32) -> i32 { x - 3 }
///
/// // These are equivalent
/// let pipe_result = pipe!(10, f, g, h);
/// let compose_result = compose!(h, g, f)(10);
///
/// assert_eq!(pipe_result, compose_result);
/// ```
#[macro_export]
macro_rules! pipe {
    // Value only: return as is
    ($value:expr) => {
        $value
    };

    // Single function: apply it
    ($value:expr, $function:expr $(,)?) => {
        $function($value)
    };

    // Multiple functions: apply left to right recursively
    ($value:expr, $function:expr, $($remaining_functions:expr),+ $(,)?) => {
        $crate::pipe!($function($value), $($remaining_functions),+)
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_pipe_value_only() {
        let result = pipe!(42);
        assert_eq!(result, 42);
    }

    #[test]
    fn test_pipe_single() {
        let double = |x: i32| x * 2;
        let result = pipe!(5, double);
        assert_eq!(result, 10);
    }

    #[test]
    fn test_pipe_two() {
        let add_one = |x: i32| x + 1;
        let double = |x: i32| x * 2;
        // double(5) = 10, add_one(10) = 11
        let result = pipe!(5, double, add_one);
        assert_eq!(result, 11);
    }

    #[test]
    fn test_pipe_three() {
        let square = |x: i32| x * x;
        let double = |x: i32| x * 2;
        let add_one = |x: i32| x + 1;
        // square(3) = 9, double(9) = 18, add_one(18) = 19
        let result = pipe!(3, square, double, add_one);
        assert_eq!(result, 19);
    }
}
