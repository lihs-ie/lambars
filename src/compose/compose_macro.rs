//! The `compose!` macro for function composition.
//!
//! This module provides the [`compose!`] macro which composes functions
//! from right to left, following the mathematical notation for function composition.

/// Composes functions from right to left.
///
/// `compose!(f, g, h)(x)` is equivalent to `f(g(h(x)))`.
///
/// This follows the mathematical convention where function composition reads
/// right-to-left: the rightmost function is applied first.
///
/// # Laws
///
/// The composition operation satisfies the following laws:
///
/// - **Associativity**: `compose!(f, compose!(g, h)) == compose!(compose!(f, g), h)`
/// - **Left Identity**: `compose!(identity, f) == f`
/// - **Right Identity**: `compose!(f, identity) == f`
///
/// # Syntax
///
/// - `compose!(f)` - Returns `f` unchanged (identity composition)
/// - `compose!(f, g)` - Returns `|x| f(g(x))`
/// - `compose!(f, g, h)` - Returns `|x| f(g(h(x)))`
/// - `compose!(f, g, h, ...)` - Composes any number of functions
///
/// # Type Requirements
///
/// All functions must implement the [`Fn`] trait. The output type of each
/// function must match the input type of the next function in the chain
/// (reading right to left).
///
/// # Examples
///
/// ## Basic composition
///
/// ```
/// use lambars::compose;
///
/// fn add_one(x: i32) -> i32 { x + 1 }
/// fn double(x: i32) -> i32 { x * 2 }
///
/// // compose!(f, g)(x) = f(g(x)) = add_one(double(5)) = add_one(10) = 11
/// let composed = compose!(add_one, double);
/// assert_eq!(composed(5), 11);
/// ```
///
/// ## Three-function composition
///
/// ```
/// use lambars::compose;
///
/// fn add_one(x: i32) -> i32 { x + 1 }
/// fn double(x: i32) -> i32 { x * 2 }
/// fn square(x: i32) -> i32 { x * x }
///
/// // compose!(f, g, h)(x) = f(g(h(x)))
/// // = add_one(double(square(3))) = add_one(double(9)) = add_one(18) = 19
/// let composed = compose!(add_one, double, square);
/// assert_eq!(composed(3), 19);
/// ```
///
/// ## Immediate application
///
/// ```
/// use lambars::compose;
///
/// fn add_one(x: i32) -> i32 { x + 1 }
/// fn double(x: i32) -> i32 { x * 2 }
///
/// // Can apply immediately without storing in a variable
/// let result = compose!(add_one, double)(5);
/// assert_eq!(result, 11);
/// ```
///
/// ## Type conversion
///
/// ```
/// use lambars::compose;
///
/// fn to_string(x: i32) -> String { x.to_string() }
/// fn get_length(s: String) -> usize { s.len() }
///
/// // Types flow through the composition
/// let composed = compose!(get_length, to_string);
/// assert_eq!(composed(12345), 5);
/// ```
///
/// ## With closures capturing environment
///
/// ```
/// use lambars::compose;
///
/// let multiplier = 3;
/// let multiply = |x: i32| x * multiplier;
/// let add_ten = |x: i32| x + 10;
///
/// let composed = compose!(add_ten, multiply);
/// assert_eq!(composed(5), 25); // add_ten(multiply(5)) = add_ten(15) = 25
/// ```
///
/// ## Verifying associativity
///
/// ```
/// use lambars::compose;
///
/// fn f(x: i32) -> i32 { x + 1 }
/// fn g(x: i32) -> i32 { x * 2 }
/// fn h(x: i32) -> i32 { x - 3 }
///
/// // These are equivalent due to associativity
/// let left = compose!(f, compose!(g, h));
/// let right = compose!(compose!(f, g), h);
///
/// assert_eq!(left(10), right(10));
/// ```
#[macro_export]
macro_rules! compose {
    // Single function: identity composition
    // Just returns the function as-is
    ($function:expr) => {
        $function
    };

    // Two functions: basic composition
    // compose!(f, g)(x) = f(g(x))
    ($outer_function:expr, $inner_function:expr $(,)?) => {{
        let outer = $outer_function;
        let inner = $inner_function;
        move |input| outer(inner(input))
    }};

    // Three or more functions: recursive composition
    // compose!(f, g, h, ...) = compose!(f, compose!(g, h, ...))
    ($outer_function:expr, $($remaining_functions:expr),+ $(,)?) => {{
        let outer = $outer_function;
        let inner_composed = $crate::compose!($($remaining_functions),+);
        move |input| outer(inner_composed(input))
    }};
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_compose_single() {
        let double = |x: i32| x * 2;
        let composed = compose!(double);
        assert_eq!(composed(5), 10);
    }

    #[test]
    fn test_compose_two() {
        let add_one = |x: i32| x + 1;
        let double = |x: i32| x * 2;
        let composed = compose!(add_one, double);
        assert_eq!(composed(5), 11);
    }

    #[test]
    fn test_compose_three() {
        let add_one = |x: i32| x + 1;
        let double = |x: i32| x * 2;
        let square = |x: i32| x * x;
        let composed = compose!(add_one, double, square);
        assert_eq!(composed(3), 19);
    }
}
