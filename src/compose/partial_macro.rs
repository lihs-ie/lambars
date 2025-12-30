//! The `partial!` macro for partial function application.
//!
//! This module provides the [`partial!`] macro which allows fixing some
//! arguments of a function while leaving others as parameters.

/// Partially applies arguments to a function.
///
/// Use `__` (double underscore) as a placeholder for arguments that should
/// remain as parameters in the resulting function.
///
/// **Important**: Do NOT import `functional_rusty::compose::__`. The `__` is
/// matched as a literal token by the macro.
///
/// # Syntax
///
/// For a 2-argument function `f(a, b)`:
/// - `partial!(f, value, __)` creates `|b| f(value, b)`
/// - `partial!(f, __, value)` creates `|a| f(a, value)`
/// - `partial!(f, v1, v2)` creates `|| f(v1, v2)` (thunk)
/// - `partial!(f, __, __)` creates `|a, b| f(a, b)` (identity)
///
/// Similar patterns apply for 3-6 argument functions.
///
/// # Type Requirements
///
/// - Fixed values must implement [`Clone`] (since the partial function may be called multiple times)
/// - The original function must implement [`Fn`]
///
/// # Supported Argument Counts
///
/// This macro supports functions with 2 to 6 arguments.
///
/// # Examples
///
/// ## Basic partial application
///
/// ```
/// use functional_rusty::partial;
///
/// fn add(first: i32, second: i32) -> i32 { first + second }
///
/// let add_five = partial!(add, 5, __);
/// assert_eq!(add_five(3), 8);
/// assert_eq!(add_five(10), 15);
/// ```
///
/// ## Fixing the second argument
///
/// ```
/// use functional_rusty::partial;
///
/// fn divide(numerator: f64, denominator: f64) -> f64 {
///     numerator / denominator
/// }
///
/// let half = partial!(divide, __, 2.0);
/// assert_eq!(half(10.0), 5.0);
/// ```
///
/// ## Three-argument function
///
/// ```
/// use functional_rusty::partial;
///
/// fn format_greeting(greeting: &str, name: &str, punctuation: &str) -> String {
///     format!("{}, {}{}", greeting, name, punctuation)
/// }
///
/// let hello_with_exclamation = partial!(format_greeting, "Hello", __, "!");
/// assert_eq!(hello_with_exclamation("Alice"), "Hello, Alice!");
/// ```
///
/// ## Creating a thunk (all arguments fixed)
///
/// ```
/// use functional_rusty::partial;
///
/// fn add(first: i32, second: i32) -> i32 { first + second }
///
/// let thunk = partial!(add, 3, 5);
/// assert_eq!(thunk(), 8);
/// ```
///
/// ## With compose!
///
/// ```
/// use functional_rusty::{compose, partial};
///
/// fn multiply(first: i32, second: i32) -> i32 { first * second }
/// fn add(first: i32, second: i32) -> i32 { first + second }
///
/// let double = partial!(multiply, 2, __);
/// let add_ten = partial!(add, 10, __);
///
/// let double_then_add_ten = compose!(add_ten, double);
/// assert_eq!(double_then_add_ten(5), 20);
/// ```
#[macro_export]
macro_rules! partial {
    // =========================================================================
    // 6-argument functions (most specific patterns first)
    // =========================================================================

    // (f, __, __, __, __, __, __) -> |a, b, c, d, e, f_arg| f(a, b, c, d, e, f_arg)
    ($function:expr, __, __, __, __, __, __ $(,)?) => {{
        let function = $function;
        move |arg1, arg2, arg3, arg4, arg5, arg6| function(arg1, arg2, arg3, arg4, arg5, arg6)
    }};

    // (f, v1, __, __, __, __, __) -> |b, c, d, e, f_arg| f(v1, b, c, d, e, f_arg)
    ($function:expr, $arg1:expr, __, __, __, __, __ $(,)?) => {{
        let function = $function;
        let arg1 = $arg1;
        move |arg2, arg3, arg4, arg5, arg6| function(arg1.clone(), arg2, arg3, arg4, arg5, arg6)
    }};

    // (f, __, __, __, __, __, v6) -> |a, b, c, d, e| f(a, b, c, d, e, v6)
    ($function:expr, __, __, __, __, __, $arg6:expr $(,)?) => {{
        let function = $function;
        let arg6 = $arg6;
        move |arg1, arg2, arg3, arg4, arg5| function(arg1, arg2, arg3, arg4, arg5, arg6.clone())
    }};

    // (f, v1, v2, v3, v4, v5, __) -> |f_arg| f(v1, v2, v3, v4, v5, f_arg)
    ($function:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr, __ $(,)?) => {{
        let function = $function;
        let arg1 = $arg1;
        let arg2 = $arg2;
        let arg3 = $arg3;
        let arg4 = $arg4;
        let arg5 = $arg5;
        move |arg6| {
            function(
                arg1.clone(),
                arg2.clone(),
                arg3.clone(),
                arg4.clone(),
                arg5.clone(),
                arg6,
            )
        }
    }};

    // (f, __, v2, v3, v4, v5, v6) -> |a| f(a, v2, v3, v4, v5, v6)
    ($function:expr, __, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr, $arg6:expr $(,)?) => {{
        let function = $function;
        let arg2 = $arg2;
        let arg3 = $arg3;
        let arg4 = $arg4;
        let arg5 = $arg5;
        let arg6 = $arg6;
        move |arg1| {
            function(
                arg1,
                arg2.clone(),
                arg3.clone(),
                arg4.clone(),
                arg5.clone(),
                arg6.clone(),
            )
        }
    }};

    // (f, v1, v2, v3, v4, v5, v6) -> || f(v1, v2, v3, v4, v5, v6) (thunk - 6 args)
    ($function:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr, $arg6:expr $(,)?) => {{
        let function = $function;
        let arg1 = $arg1;
        let arg2 = $arg2;
        let arg3 = $arg3;
        let arg4 = $arg4;
        let arg5 = $arg5;
        let arg6 = $arg6;
        move || {
            function(
                arg1.clone(),
                arg2.clone(),
                arg3.clone(),
                arg4.clone(),
                arg5.clone(),
                arg6.clone(),
            )
        }
    }};

    // =========================================================================
    // 5-argument functions
    // =========================================================================

    // (f, __, __, __, __, __) -> |a, b, c, d, e| f(a, b, c, d, e)
    ($function:expr, __, __, __, __, __ $(,)?) => {{
        let function = $function;
        move |arg1, arg2, arg3, arg4, arg5| function(arg1, arg2, arg3, arg4, arg5)
    }};

    // (f, v1, __, __, __, __) -> |b, c, d, e| f(v1, b, c, d, e)
    ($function:expr, $arg1:expr, __, __, __, __ $(,)?) => {{
        let function = $function;
        let arg1 = $arg1;
        move |arg2, arg3, arg4, arg5| function(arg1.clone(), arg2, arg3, arg4, arg5)
    }};

    // (f, __, v2, __, __, __) -> |a, c, d, e| f(a, v2, c, d, e)
    ($function:expr, __, $arg2:expr, __, __, __ $(,)?) => {{
        let function = $function;
        let arg2 = $arg2;
        move |arg1, arg3, arg4, arg5| function(arg1, arg2.clone(), arg3, arg4, arg5)
    }};

    // (f, __, __, v3, __, __) -> |a, b, d, e| f(a, b, v3, d, e)
    ($function:expr, __, __, $arg3:expr, __, __ $(,)?) => {{
        let function = $function;
        let arg3 = $arg3;
        move |arg1, arg2, arg4, arg5| function(arg1, arg2, arg3.clone(), arg4, arg5)
    }};

    // (f, __, __, __, v4, __) -> |a, b, c, e| f(a, b, c, v4, e)
    ($function:expr, __, __, __, $arg4:expr, __ $(,)?) => {{
        let function = $function;
        let arg4 = $arg4;
        move |arg1, arg2, arg3, arg5| function(arg1, arg2, arg3, arg4.clone(), arg5)
    }};

    // (f, __, __, __, __, v5) -> |a, b, c, d| f(a, b, c, d, v5)
    ($function:expr, __, __, __, __, $arg5:expr $(,)?) => {{
        let function = $function;
        let arg5 = $arg5;
        move |arg1, arg2, arg3, arg4| function(arg1, arg2, arg3, arg4, arg5.clone())
    }};

    // (f, v1, v2, v3, v4, __) -> |e| f(v1, v2, v3, v4, e)
    ($function:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, __ $(,)?) => {{
        let function = $function;
        let arg1 = $arg1;
        let arg2 = $arg2;
        let arg3 = $arg3;
        let arg4 = $arg4;
        move |arg5| function(arg1.clone(), arg2.clone(), arg3.clone(), arg4.clone(), arg5)
    }};

    // (f, __, v2, v3, v4, v5) -> |a| f(a, v2, v3, v4, v5)
    ($function:expr, __, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr $(,)?) => {{
        let function = $function;
        let arg2 = $arg2;
        let arg3 = $arg3;
        let arg4 = $arg4;
        let arg5 = $arg5;
        move |arg1| function(arg1, arg2.clone(), arg3.clone(), arg4.clone(), arg5.clone())
    }};

    // (f, v1, v2, v3, v4, v5) -> || f(v1, v2, v3, v4, v5) (thunk - 5 args)
    ($function:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr $(,)?) => {{
        let function = $function;
        let arg1 = $arg1;
        let arg2 = $arg2;
        let arg3 = $arg3;
        let arg4 = $arg4;
        let arg5 = $arg5;
        move || {
            function(
                arg1.clone(),
                arg2.clone(),
                arg3.clone(),
                arg4.clone(),
                arg5.clone(),
            )
        }
    }};

    // =========================================================================
    // 4-argument functions
    // =========================================================================

    // (f, __, __, __, __) -> |a, b, c, d| f(a, b, c, d)
    ($function:expr, __, __, __, __ $(,)?) => {{
        let function = $function;
        move |arg1, arg2, arg3, arg4| function(arg1, arg2, arg3, arg4)
    }};

    // (f, v1, __, __, __) -> |b, c, d| f(v1, b, c, d)
    ($function:expr, $arg1:expr, __, __, __ $(,)?) => {{
        let function = $function;
        let arg1 = $arg1;
        move |arg2, arg3, arg4| function(arg1.clone(), arg2, arg3, arg4)
    }};

    // (f, __, v2, __, __) -> |a, c, d| f(a, v2, c, d)
    ($function:expr, __, $arg2:expr, __, __ $(,)?) => {{
        let function = $function;
        let arg2 = $arg2;
        move |arg1, arg3, arg4| function(arg1, arg2.clone(), arg3, arg4)
    }};

    // (f, __, __, v3, __) -> |a, b, d| f(a, b, v3, d)
    ($function:expr, __, __, $arg3:expr, __ $(,)?) => {{
        let function = $function;
        let arg3 = $arg3;
        move |arg1, arg2, arg4| function(arg1, arg2, arg3.clone(), arg4)
    }};

    // (f, __, __, __, v4) -> |a, b, c| f(a, b, c, v4)
    ($function:expr, __, __, __, $arg4:expr $(,)?) => {{
        let function = $function;
        let arg4 = $arg4;
        move |arg1, arg2, arg3| function(arg1, arg2, arg3, arg4.clone())
    }};

    // (f, v1, v2, __, __) -> |c, d| f(v1, v2, c, d)
    ($function:expr, $arg1:expr, $arg2:expr, __, __ $(,)?) => {{
        let function = $function;
        let arg1 = $arg1;
        let arg2 = $arg2;
        move |arg3, arg4| function(arg1.clone(), arg2.clone(), arg3, arg4)
    }};

    // (f, v1, __, v3, __) -> |b, d| f(v1, b, v3, d)
    ($function:expr, $arg1:expr, __, $arg3:expr, __ $(,)?) => {{
        let function = $function;
        let arg1 = $arg1;
        let arg3 = $arg3;
        move |arg2, arg4| function(arg1.clone(), arg2, arg3.clone(), arg4)
    }};

    // (f, v1, __, __, v4) -> |b, c| f(v1, b, c, v4)
    ($function:expr, $arg1:expr, __, __, $arg4:expr $(,)?) => {{
        let function = $function;
        let arg1 = $arg1;
        let arg4 = $arg4;
        move |arg2, arg3| function(arg1.clone(), arg2, arg3, arg4.clone())
    }};

    // (f, __, v2, v3, __) -> |a, d| f(a, v2, v3, d)
    ($function:expr, __, $arg2:expr, $arg3:expr, __ $(,)?) => {{
        let function = $function;
        let arg2 = $arg2;
        let arg3 = $arg3;
        move |arg1, arg4| function(arg1, arg2.clone(), arg3.clone(), arg4)
    }};

    // (f, __, v2, __, v4) -> |a, c| f(a, v2, c, v4)
    ($function:expr, __, $arg2:expr, __, $arg4:expr $(,)?) => {{
        let function = $function;
        let arg2 = $arg2;
        let arg4 = $arg4;
        move |arg1, arg3| function(arg1, arg2.clone(), arg3, arg4.clone())
    }};

    // (f, __, __, v3, v4) -> |a, b| f(a, b, v3, v4)
    ($function:expr, __, __, $arg3:expr, $arg4:expr $(,)?) => {{
        let function = $function;
        let arg3 = $arg3;
        let arg4 = $arg4;
        move |arg1, arg2| function(arg1, arg2, arg3.clone(), arg4.clone())
    }};

    // (f, v1, v2, v3, __) -> |d| f(v1, v2, v3, d)
    ($function:expr, $arg1:expr, $arg2:expr, $arg3:expr, __ $(,)?) => {{
        let function = $function;
        let arg1 = $arg1;
        let arg2 = $arg2;
        let arg3 = $arg3;
        move |arg4| function(arg1.clone(), arg2.clone(), arg3.clone(), arg4)
    }};

    // (f, v1, v2, __, v4) -> |c| f(v1, v2, c, v4)
    ($function:expr, $arg1:expr, $arg2:expr, __, $arg4:expr $(,)?) => {{
        let function = $function;
        let arg1 = $arg1;
        let arg2 = $arg2;
        let arg4 = $arg4;
        move |arg3| function(arg1.clone(), arg2.clone(), arg3, arg4.clone())
    }};

    // (f, v1, __, v3, v4) -> |b| f(v1, b, v3, v4)
    ($function:expr, $arg1:expr, __, $arg3:expr, $arg4:expr $(,)?) => {{
        let function = $function;
        let arg1 = $arg1;
        let arg3 = $arg3;
        let arg4 = $arg4;
        move |arg2| function(arg1.clone(), arg2, arg3.clone(), arg4.clone())
    }};

    // (f, __, v2, v3, v4) -> |a| f(a, v2, v3, v4)
    ($function:expr, __, $arg2:expr, $arg3:expr, $arg4:expr $(,)?) => {{
        let function = $function;
        let arg2 = $arg2;
        let arg3 = $arg3;
        let arg4 = $arg4;
        move |arg1| function(arg1, arg2.clone(), arg3.clone(), arg4.clone())
    }};

    // (f, v1, v2, v3, v4) -> || f(v1, v2, v3, v4) (thunk - 4 args)
    ($function:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr $(,)?) => {{
        let function = $function;
        let arg1 = $arg1;
        let arg2 = $arg2;
        let arg3 = $arg3;
        let arg4 = $arg4;
        move || function(arg1.clone(), arg2.clone(), arg3.clone(), arg4.clone())
    }};

    // =========================================================================
    // 3-argument functions
    // =========================================================================

    // (f, __, __, __) -> |a, b, c| f(a, b, c)
    ($function:expr, __, __, __ $(,)?) => {{
        let function = $function;
        move |arg1, arg2, arg3| function(arg1, arg2, arg3)
    }};

    // (f, v1, __, __) -> |b, c| f(v1, b, c)
    ($function:expr, $arg1:expr, __, __ $(,)?) => {{
        let function = $function;
        let arg1 = $arg1;
        move |arg2, arg3| function(arg1.clone(), arg2, arg3)
    }};

    // (f, __, v2, __) -> |a, c| f(a, v2, c)
    ($function:expr, __, $arg2:expr, __ $(,)?) => {{
        let function = $function;
        let arg2 = $arg2;
        move |arg1, arg3| function(arg1, arg2.clone(), arg3)
    }};

    // (f, __, __, v3) -> |a, b| f(a, b, v3)
    ($function:expr, __, __, $arg3:expr $(,)?) => {{
        let function = $function;
        let arg3 = $arg3;
        move |arg1, arg2| function(arg1, arg2, arg3.clone())
    }};

    // (f, v1, v2, __) -> |c| f(v1, v2, c)
    ($function:expr, $arg1:expr, $arg2:expr, __ $(,)?) => {{
        let function = $function;
        let arg1 = $arg1;
        let arg2 = $arg2;
        move |arg3| function(arg1.clone(), arg2.clone(), arg3)
    }};

    // (f, v1, __, v3) -> |b| f(v1, b, v3)
    ($function:expr, $arg1:expr, __, $arg3:expr $(,)?) => {{
        let function = $function;
        let arg1 = $arg1;
        let arg3 = $arg3;
        move |arg2| function(arg1.clone(), arg2, arg3.clone())
    }};

    // (f, __, v2, v3) -> |a| f(a, v2, v3)
    ($function:expr, __, $arg2:expr, $arg3:expr $(,)?) => {{
        let function = $function;
        let arg2 = $arg2;
        let arg3 = $arg3;
        move |arg1| function(arg1, arg2.clone(), arg3.clone())
    }};

    // (f, v1, v2, v3) -> || f(v1, v2, v3) (thunk - 3 args)
    ($function:expr, $arg1:expr, $arg2:expr, $arg3:expr $(,)?) => {{
        let function = $function;
        let arg1 = $arg1;
        let arg2 = $arg2;
        let arg3 = $arg3;
        move || function(arg1.clone(), arg2.clone(), arg3.clone())
    }};

    // =========================================================================
    // 2-argument functions (must be last due to pattern matching order)
    // =========================================================================

    // (f, __, __) -> |a, b| f(a, b)
    ($function:expr, __, __ $(,)?) => {{
        let function = $function;
        move |arg1, arg2| function(arg1, arg2)
    }};

    // (f, value, __) -> |b| f(value, b)
    ($function:expr, $arg1:expr, __ $(,)?) => {{
        let function = $function;
        let arg1 = $arg1;
        move |arg2| function(arg1.clone(), arg2)
    }};

    // (f, __, value) -> |a| f(a, value)
    ($function:expr, __, $arg2:expr $(,)?) => {{
        let function = $function;
        let arg2 = $arg2;
        move |arg1| function(arg1, arg2.clone())
    }};

    // (f, v1, v2) -> || f(v1, v2) (thunk - 2 args, must be last)
    ($function:expr, $arg1:expr, $arg2:expr $(,)?) => {{
        let function = $function;
        let arg1 = $arg1;
        let arg2 = $arg2;
        move || function(arg1.clone(), arg2.clone())
    }};
}

#[cfg(test)]
mod tests {
    fn add(first: i32, second: i32) -> i32 {
        first + second
    }

    #[test]
    fn test_partial_2_args_first_fixed() {
        let add_five = partial!(add, 5, __);
        assert_eq!(add_five(3), 8);
    }

    #[test]
    fn test_partial_2_args_second_fixed() {
        let add_ten = partial!(add, __, 10);
        assert_eq!(add_ten(5), 15);
    }

    #[test]
    fn test_partial_2_args_both_fixed() {
        let thunk = partial!(add, 3, 5);
        assert_eq!(thunk(), 8);
    }

    #[test]
    fn test_partial_2_args_none_fixed() {
        let same = partial!(add, __, __);
        assert_eq!(same(3, 5), 8);
    }
}
