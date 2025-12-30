//! Helper functions (combinators) for function composition.
//!
//! This module provides fundamental combinators that are commonly used
//! in functional programming:
//!
//! - [`identity`]: The identity function (I combinator)
//! - [`constant`]: Creates a function that always returns the same value (K combinator)
//! - [`flip`]: Swaps the arguments of a binary function (C combinator)
//!
//! These functions serve as building blocks for more complex function compositions.

/// Returns the value unchanged.
///
/// The identity function is the unit element of function composition:
/// - `compose!(identity, f)` is equivalent to `f`
/// - `compose!(f, identity)` is equivalent to `f`
///
/// In combinatory logic, this is known as the I combinator.
///
/// # Type Parameters
///
/// * `T` - The type of the value to return
///
/// # Examples
///
/// ```
/// use functional_rusty::compose::identity;
///
/// assert_eq!(identity(42), 42);
/// assert_eq!(identity("hello"), "hello");
/// assert_eq!(identity(vec![1, 2, 3]), vec![1, 2, 3]);
/// ```
///
/// # Use with function composition
///
/// ```
/// use functional_rusty::compose::identity;
/// use functional_rusty::compose;
///
/// fn double(x: i32) -> i32 { x * 2 }
///
/// let composed = compose!(identity, double);
/// assert_eq!(composed(5), double(5));
/// ```
#[inline]
pub fn identity<T>(value: T) -> T {
    value
}

/// Creates a function that always returns the given value, ignoring its input.
///
/// Also known as the K combinator in combinatory logic.
/// Useful when you need a function that always produces the same result
/// regardless of its input.
///
/// # Type Parameters
///
/// * `T` - The type of the constant value (must implement [`Clone`])
/// * `U` - The input type of the returned function (ignored)
///
/// # Arguments
///
/// * `value` - The value that the returned function will always return
///
/// # Returns
///
/// A function that takes any input and returns the constant value.
///
/// # Examples
///
/// ```
/// use functional_rusty::compose::constant;
///
/// // Create a function that always returns 5 for i32 input
/// let always_five_from_int = constant::<_, i32>(5);
/// assert_eq!(always_five_from_int(100), 5);
///
/// // Create a function that always returns 5 for &str input
/// let always_five_from_str = constant::<_, &str>(5);
/// assert_eq!(always_five_from_str("ignored"), 5);
///
/// // Create a function that always returns 5 for () input
/// let always_five_from_unit = constant::<_, ()>(5);
/// assert_eq!(always_five_from_unit(()), 5);
/// ```
///
/// # Use with iterators
///
/// ```
/// use functional_rusty::compose::constant;
///
/// // Replace all elements with zeros
/// let values: Vec<i32> = vec![1, 2, 3].into_iter().map(constant(0)).collect();
/// assert_eq!(values, vec![0, 0, 0]);
/// ```
#[inline]
pub fn constant<T: Clone, U>(value: T) -> impl Fn(U) -> T {
    move |_| value.clone()
}

/// Swaps the arguments of a binary function.
///
/// Given a function `f(a, b)`, returns a new function `g(b, a)` such that
/// `g(b, a) = f(a, b)`.
///
/// Also known as the C combinator (flip) in combinatory logic.
/// Useful for partial application when you want to fix the second argument
/// instead of the first.
///
/// # Laws
///
/// - **Double flip identity**: `flip(flip(f)) == f`
/// - **Flip definition**: `flip(f)(a, b) == f(b, a)`
///
/// # Type Parameters
///
/// * `A` - The type of the first argument of the original function
/// * `B` - The type of the second argument of the original function
/// * `C` - The return type of the function
/// * `F` - The function type (must implement [`Fn`])
///
/// # Arguments
///
/// * `function` - The binary function whose arguments should be swapped
///
/// # Returns
///
/// A new function with swapped argument order.
///
/// # Examples
///
/// ```
/// use functional_rusty::compose::flip;
///
/// fn divide(numerator: f64, denominator: f64) -> f64 {
///     numerator / denominator
/// }
///
/// let flipped_divide = flip(divide);
///
/// // divide(10.0, 2.0) = 5.0
/// assert_eq!(divide(10.0, 2.0), 5.0);
///
/// // flipped_divide(10.0, 2.0) = divide(2.0, 10.0) = 0.2
/// assert!((flipped_divide(10.0, 2.0) - 0.2).abs() < f64::EPSILON);
/// ```
///
/// # Double flip is identity
///
/// ```
/// use functional_rusty::compose::flip;
///
/// fn subtract(minuend: i32, subtrahend: i32) -> i32 {
///     minuend - subtrahend
/// }
///
/// let flipped_once = flip(subtract);
/// let flipped_twice = flip(flipped_once);
///
/// assert_eq!(subtract(10, 3), flipped_twice(10, 3));
/// ```
#[inline]
pub fn flip<A, B, C, F>(function: F) -> impl Fn(B, A) -> C
where
    F: Fn(A, B) -> C,
{
    move |second_argument, first_argument| function(first_argument, second_argument)
}

/// Placeholder marker type for partial application.
///
/// This type is used internally by the [`partial!`](crate::partial) macro.
/// Users should use `__` (double underscore) directly in the macro invocation
/// as a literal token, without importing it.
///
/// # Examples
///
/// ```
/// use functional_rusty::partial;
///
/// fn add(first: i32, second: i32) -> i32 { first + second }
///
/// // Use __ directly as a placeholder - do NOT import it
/// let add_five = partial!(add, 5, __);
/// assert_eq!(add_five(3), 8);
///
/// // Fix the second argument, leave the first as a parameter
/// let add_to_ten = partial!(add, __, 10);
/// assert_eq!(add_to_ten(3), 13);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placeholder;

/// The placeholder constant for partial application.
///
/// **Important**: Do NOT import this constant when using [`partial!`](crate::partial).
/// The macro matches `__` as a literal identifier token. Importing this constant
/// would cause the macro pattern matching to fail.
///
/// This constant exists for potential programmatic use cases, but for the
/// `partial!` macro, simply write `__` directly without importing.
///
/// Note: This is named `__` (double underscore) because Rust's `macro_rules!`
/// cannot match a single underscore `_` as a literal token.
///
/// # Examples
///
/// ```
/// use functional_rusty::partial;
///
/// fn divide(numerator: f64, denominator: f64) -> f64 {
///     numerator / denominator
/// }
///
/// // Use __ directly - do NOT import compose::__
/// let half = partial!(divide, __, 2.0);
/// assert_eq!(half(10.0), 5.0);
/// ```
#[allow(non_upper_case_globals)]
pub const __: Placeholder = Placeholder;

// Curry functions are implemented directly in the macros using closures
// to avoid complex type constraints with impl Trait in return positions.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_with_unit() {
        assert_eq!(identity(()), ());
    }

    #[test]
    fn test_constant_with_reference() {
        let always_hello = constant("hello");
        assert_eq!(always_hello(42), "hello");
    }

    #[test]
    fn test_flip_with_asymmetric_function() {
        fn power(base: i32, exponent: u32) -> i32 {
            base.pow(exponent)
        }

        let flipped_power = flip(power);
        // power(2, 3) = 8
        assert_eq!(power(2, 3), 8);
        // flipped_power(3, 2) = power(2, 3) = 8
        assert_eq!(flipped_power(3, 2), 8);
    }
}
