//! The curry macro family for converting multi-argument functions to curried form.
//!
//! This module provides macros for currying functions with 2 to 6 arguments.
//! Currying transforms a function that takes multiple arguments into a sequence
//! of functions, each taking a single argument.
//!
//! # Design Decisions
//!
//! The curry macros use `std::rc::Rc` internally to share the function and arguments
//! across multiple closure invocations. This allows:
//!
//! - The curried function to be called multiple times
//! - Partial applications to be reused
//! - Arguments that don't implement `Copy` to work correctly
//!
//! Note: The returned closures implement `Fn`, so they can be used with
//! `compose!`, `pipe!`, and other combinators.

/// Converts a 2-argument function into a curried form.
///
/// Given a function `f(a, b) -> c`, returns a closure that takes `a` and returns
/// another closure that takes `b` and returns `c`.
///
/// # Type Requirements
///
/// - The function must implement [`Fn`]
/// - Argument types must implement [`Clone`] (for reusability of partial applications)
///
/// # Examples
///
/// ## Basic currying
///
/// ```
/// use lambars::curry2;
///
/// fn add(first: i32, second: i32) -> i32 { first + second }
///
/// let curried_add = curry2!(add);
/// assert_eq!(curried_add(5)(3), 8);
/// ```
///
/// ## Partial application
///
/// ```
/// use lambars::curry2;
///
/// fn multiply(first: i32, second: i32) -> i32 { first * second }
///
/// let curried = curry2!(multiply);
/// let double = curried(2);
/// let triple = curried(3);
///
/// assert_eq!(double(5), 10);
/// assert_eq!(triple(5), 15);
/// ```
///
/// ## With closures
///
/// ```
/// use lambars::curry2;
///
/// let add_closure = |first: i32, second: i32| first + second;
/// let curried = curry2!(add_closure);
///
/// assert_eq!(curried(10)(20), 30);
/// ```
#[macro_export]
macro_rules! curry2 {
    ($function:expr $(,)?) => {{
        let function = ::std::rc::Rc::new($function);
        move |arg1| {
            let function = ::std::rc::Rc::clone(&function);
            let arg1 = ::std::rc::Rc::new(arg1);
            move |arg2| {
                function(
                    ::std::rc::Rc::unwrap_or_clone(::std::rc::Rc::clone(&arg1)),
                    arg2,
                )
            }
        }
    }};
}

/// Converts a 3-argument function into a curried form.
///
/// Given a function `f(a, b, c) -> d`, returns nested closures that take one
/// argument at a time.
///
/// # Type Requirements
///
/// - The function must implement [`Fn`]
/// - Argument types (except the last) must implement [`Clone`]
///
/// # Examples
///
/// ## Basic currying
///
/// ```
/// use lambars::curry3;
///
/// fn add_three(first: i32, second: i32, third: i32) -> i32 {
///     first + second + third
/// }
///
/// let curried = curry3!(add_three);
/// assert_eq!(curried(1)(2)(3), 6);
/// ```
///
/// ## Step-by-step application
///
/// ```
/// use lambars::curry3;
///
/// fn volume(width: f64, height: f64, depth: f64) -> f64 {
///     width * height * depth
/// }
///
/// let curried_volume = curry3!(volume);
/// let with_width = curried_volume(2.0);
/// let with_width_height = with_width(3.0);
/// let result = with_width_height(4.0);
///
/// assert!((result - 24.0).abs() < f64::EPSILON);
/// ```
#[macro_export]
macro_rules! curry3 {
    ($function:expr $(,)?) => {{
        let function = ::std::rc::Rc::new($function);
        move |arg1| {
            let function = ::std::rc::Rc::clone(&function);
            let arg1 = ::std::rc::Rc::new(arg1);
            move |arg2| {
                let function = ::std::rc::Rc::clone(&function);
                let arg1 = ::std::rc::Rc::clone(&arg1);
                let arg2 = ::std::rc::Rc::new(arg2);
                move |arg3| {
                    function(
                        ::std::rc::Rc::unwrap_or_clone(::std::rc::Rc::clone(&arg1)),
                        ::std::rc::Rc::unwrap_or_clone(::std::rc::Rc::clone(&arg2)),
                        arg3,
                    )
                }
            }
        }
    }};
}

/// Converts a 4-argument function into a curried form.
///
/// Given a function `f(a, b, c, d) -> e`, returns nested closures that take one
/// argument at a time.
///
/// # Type Requirements
///
/// - The function must implement [`Fn`]
/// - Argument types (except the last) must implement [`Clone`]
///
/// # Examples
///
/// ```
/// use lambars::curry4;
///
/// fn sum_four(a: i32, b: i32, c: i32, d: i32) -> i32 {
///     a + b + c + d
/// }
///
/// let curried = curry4!(sum_four);
/// assert_eq!(curried(1)(2)(3)(4), 10);
/// ```
#[macro_export]
macro_rules! curry4 {
    ($function:expr $(,)?) => {{
        let function = ::std::rc::Rc::new($function);
        move |arg1| {
            let function = ::std::rc::Rc::clone(&function);
            let arg1 = ::std::rc::Rc::new(arg1);
            move |arg2| {
                let function = ::std::rc::Rc::clone(&function);
                let arg1 = ::std::rc::Rc::clone(&arg1);
                let arg2 = ::std::rc::Rc::new(arg2);
                move |arg3| {
                    let function = ::std::rc::Rc::clone(&function);
                    let arg1 = ::std::rc::Rc::clone(&arg1);
                    let arg2 = ::std::rc::Rc::clone(&arg2);
                    let arg3 = ::std::rc::Rc::new(arg3);
                    move |arg4| {
                        function(
                            ::std::rc::Rc::unwrap_or_clone(::std::rc::Rc::clone(&arg1)),
                            ::std::rc::Rc::unwrap_or_clone(::std::rc::Rc::clone(&arg2)),
                            ::std::rc::Rc::unwrap_or_clone(::std::rc::Rc::clone(&arg3)),
                            arg4,
                        )
                    }
                }
            }
        }
    }};
}

/// Converts a 5-argument function into a curried form.
///
/// Given a function `f(a, b, c, d, e) -> r`, returns nested closures that take one
/// argument at a time.
///
/// # Type Requirements
///
/// - The function must implement [`Fn`]
/// - Argument types (except the last) must implement [`Clone`]
///
/// # Examples
///
/// ```
/// use lambars::curry5;
///
/// fn sum_five(a: i32, b: i32, c: i32, d: i32, e: i32) -> i32 {
///     a + b + c + d + e
/// }
///
/// let curried = curry5!(sum_five);
/// assert_eq!(curried(1)(2)(3)(4)(5), 15);
/// ```
#[macro_export]
macro_rules! curry5 {
    ($function:expr $(,)?) => {{
        let function = ::std::rc::Rc::new($function);
        move |arg1| {
            let function = ::std::rc::Rc::clone(&function);
            let arg1 = ::std::rc::Rc::new(arg1);
            move |arg2| {
                let function = ::std::rc::Rc::clone(&function);
                let arg1 = ::std::rc::Rc::clone(&arg1);
                let arg2 = ::std::rc::Rc::new(arg2);
                move |arg3| {
                    let function = ::std::rc::Rc::clone(&function);
                    let arg1 = ::std::rc::Rc::clone(&arg1);
                    let arg2 = ::std::rc::Rc::clone(&arg2);
                    let arg3 = ::std::rc::Rc::new(arg3);
                    move |arg4| {
                        let function = ::std::rc::Rc::clone(&function);
                        let arg1 = ::std::rc::Rc::clone(&arg1);
                        let arg2 = ::std::rc::Rc::clone(&arg2);
                        let arg3 = ::std::rc::Rc::clone(&arg3);
                        let arg4 = ::std::rc::Rc::new(arg4);
                        move |arg5| {
                            function(
                                ::std::rc::Rc::unwrap_or_clone(::std::rc::Rc::clone(&arg1)),
                                ::std::rc::Rc::unwrap_or_clone(::std::rc::Rc::clone(&arg2)),
                                ::std::rc::Rc::unwrap_or_clone(::std::rc::Rc::clone(&arg3)),
                                ::std::rc::Rc::unwrap_or_clone(::std::rc::Rc::clone(&arg4)),
                                arg5,
                            )
                        }
                    }
                }
            }
        }
    }};
}

/// Converts a 6-argument function into a curried form.
///
/// Given a function `f(a, b, c, d, e, g) -> r`, returns nested closures that take one
/// argument at a time.
///
/// # Type Requirements
///
/// - The function must implement [`Fn`]
/// - Argument types (except the last) must implement [`Clone`]
///
/// # Examples
///
/// ```
/// use lambars::curry6;
///
/// fn sum_six(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) -> i32 {
///     a + b + c + d + e + f
/// }
///
/// let curried = curry6!(sum_six);
/// assert_eq!(curried(1)(2)(3)(4)(5)(6), 21);
/// ```
#[macro_export]
macro_rules! curry6 {
    ($function:expr $(,)?) => {{
        let function = ::std::rc::Rc::new($function);
        move |arg1| {
            let function = ::std::rc::Rc::clone(&function);
            let arg1 = ::std::rc::Rc::new(arg1);
            move |arg2| {
                let function = ::std::rc::Rc::clone(&function);
                let arg1 = ::std::rc::Rc::clone(&arg1);
                let arg2 = ::std::rc::Rc::new(arg2);
                move |arg3| {
                    let function = ::std::rc::Rc::clone(&function);
                    let arg1 = ::std::rc::Rc::clone(&arg1);
                    let arg2 = ::std::rc::Rc::clone(&arg2);
                    let arg3 = ::std::rc::Rc::new(arg3);
                    move |arg4| {
                        let function = ::std::rc::Rc::clone(&function);
                        let arg1 = ::std::rc::Rc::clone(&arg1);
                        let arg2 = ::std::rc::Rc::clone(&arg2);
                        let arg3 = ::std::rc::Rc::clone(&arg3);
                        let arg4 = ::std::rc::Rc::new(arg4);
                        move |arg5| {
                            let function = ::std::rc::Rc::clone(&function);
                            let arg1 = ::std::rc::Rc::clone(&arg1);
                            let arg2 = ::std::rc::Rc::clone(&arg2);
                            let arg3 = ::std::rc::Rc::clone(&arg3);
                            let arg4 = ::std::rc::Rc::clone(&arg4);
                            let arg5 = ::std::rc::Rc::new(arg5);
                            move |arg6| {
                                function(
                                    ::std::rc::Rc::unwrap_or_clone(::std::rc::Rc::clone(&arg1)),
                                    ::std::rc::Rc::unwrap_or_clone(::std::rc::Rc::clone(&arg2)),
                                    ::std::rc::Rc::unwrap_or_clone(::std::rc::Rc::clone(&arg3)),
                                    ::std::rc::Rc::unwrap_or_clone(::std::rc::Rc::clone(&arg4)),
                                    ::std::rc::Rc::unwrap_or_clone(::std::rc::Rc::clone(&arg5)),
                                    arg6,
                                )
                            }
                        }
                    }
                }
            }
        }
    }};
}

#[cfg(test)]
mod tests {
    fn add(first: i32, second: i32) -> i32 {
        first + second
    }

    fn add_three(first: i32, second: i32, third: i32) -> i32 {
        first + second + third
    }

    #[test]
    fn test_curry2_basic() {
        let curried = curry2!(add);
        assert_eq!(curried(5)(3), 8);
    }

    #[test]
    fn test_curry2_partial() {
        let curried = curry2!(add);
        let add_five = curried(5);
        assert_eq!(add_five(3), 8);
        assert_eq!(add_five(10), 15);
    }

    #[test]
    fn test_curry3_basic() {
        let curried = curry3!(add_three);
        assert_eq!(curried(1)(2)(3), 6);
    }

    #[test]
    fn test_curry3_partial() {
        let curried = curry3!(add_three);
        let with_first = curried(10);
        let with_first_second = with_first(20);
        assert_eq!(with_first_second(30), 60);
    }
}
