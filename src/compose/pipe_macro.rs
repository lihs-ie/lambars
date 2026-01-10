//! The `pipe!` macro for left-to-right function application.
//!
//! This module provides the [`pipe!`] macro which applies functions
//! from left to right, following the data flow style of programming.
//!
//! The macro supports three types of operators:
//!
//! - **Regular application** (`,`): Direct function application
//! - **Lift operator** (`=>`): Apply a pure function within a monadic context
//! - **Bind operator** (`=>>`): Apply a monadic function (`flat_map`)

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
/// ## Basic Syntax (Pure Functions)
///
/// - `pipe!(x)` - Returns `x` unchanged
/// - `pipe!(x, f)` - Returns `f(x)`
/// - `pipe!(x, f, g)` - Returns `g(f(x))`
/// - `pipe!(x, f, g, h, ...)` - Returns `...h(g(f(x)))`
///
/// ## Monadic Operators
///
/// For working with monadic types (`Option`, `Result`, `Box`, `Identity`, etc.):
///
/// - `pipe!(m, => f)` - Lift operator: applies `f` using `fmap` (equivalent to `m.flat_map(|v| M::pure(f(v)))`)
/// - `pipe!(m, =>> f)` - Bind operator: applies `f` using `flat_map` (equivalent to `m.flat_map(f)`)
///
/// These operators can be mixed freely:
///
/// - `pipe!(m, => f, =>> g, => h)` - Mix lift and bind operators
///
/// # Type Requirements
///
/// Each function only needs to implement [`FnOnce`], since each function
/// is called exactly once. This allows using functions that consume their
/// captured environment.
///
/// For the lift operator (`=>`), the value must implement [`Functor`](crate::typeclass::Functor).
/// For the bind operator (`=>>`), the value must implement [`Monad`](crate::typeclass::Monad).
///
/// # Examples
///
/// ## Basic pipeline
///
/// ```
/// use lambars::pipe;
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
/// use lambars::pipe;
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
/// use lambars::pipe;
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
/// use lambars::pipe;
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
/// use lambars::pipe;
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
/// use lambars::{compose, pipe};
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
///
/// ## Monadic pipeline with lift operator
///
/// The lift operator (`=>`) applies a pure function within a monadic context:
///
/// ```
/// use lambars::pipe;
///
/// // Apply pure functions to Option values
/// let result = pipe!(
///     Some(5),
///     => |x| x + 1,    // Some(6)
///     => |x| x * 2     // Some(12)
/// );
/// assert_eq!(result, Some(12));
///
/// // None propagates through the pipeline
/// let result: Option<i32> = pipe!(
///     None,
///     => |x: i32| x + 1
/// );
/// assert_eq!(result, None);
/// ```
///
/// ## Monadic pipeline with bind operator
///
/// The bind operator (`=>>`) applies a monadic function using `flat_map`:
///
/// ```
/// use lambars::pipe;
///
/// fn safe_divide(x: i32) -> Option<i32> {
///     if x != 0 { Some(100 / x) } else { None }
/// }
///
/// fn safe_negate(x: i32) -> Option<i32> {
///     Some(-x)
/// }
///
/// // Chain monadic functions
/// let result = pipe!(
///     Some(4),
///     =>> safe_divide,   // Some(25)
///     =>> safe_negate    // Some(-25)
/// );
/// assert_eq!(result, Some(-25));
///
/// // Failure propagates
/// let result = pipe!(
///     Some(0),
///     =>> safe_divide,   // None (division by zero)
///     =>> safe_negate    // Not executed
/// );
/// assert_eq!(result, None);
/// ```
///
/// ## Mixed operators
///
/// Lift and bind operators can be freely combined:
///
/// ```
/// use lambars::pipe;
///
/// let result = pipe!(
///     Some(10),
///     => |x| x / 2,                               // lift: Some(5)
///     =>> |x| if x > 0 { Some(x + 10) } else { None }, // bind: Some(15)
///     => |x| x * 2                                // lift: Some(30)
/// );
/// assert_eq!(result, Some(30));
/// ```
#[macro_export]
macro_rules! pipe {
    ($value:expr) => {
        $value
    };

    // Lift operator: uses Functor::fmap for simpler type inference than flat_map + pure
    ($value:expr, => $function:expr $(,)?) => {{
        use $crate::typeclass::Functor;
        $value.fmap($function)
    }};

    ($value:expr, => $function:expr, $($rest:tt)+) => {{
        use $crate::typeclass::Functor;
        let __pipe_intermediate = $value.fmap($function);
        $crate::pipe!(__pipe_intermediate, $($rest)+)
    }};

    // Bind operator: direct flat_map application
    ($value:expr, =>> $function:expr $(,)?) => {{
        use $crate::typeclass::Monad;
        $value.flat_map($function)
    }};

    ($value:expr, =>> $function:expr, $($rest:tt)+) => {{
        use $crate::typeclass::Monad;
        let __pipe_intermediate = $value.flat_map($function);
        $crate::pipe!(__pipe_intermediate, $($rest)+)
    }};

    // Pure function application
    ($value:expr, $function:expr $(,)?) => {
        $function($value)
    };

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

#[cfg(test)]
mod monad_extension_tests {
    use crate::typeclass::Identity;
    use rstest::rstest;

    mod lift_operator_tests {
        use super::*;

        #[rstest]
        fn option_lift_single_pure_function() {
            let result = pipe!(Some(5), => |x| x * 2);
            assert_eq!(result, Some(10));
        }

        #[rstest]
        fn option_lift_with_none() {
            let result: Option<i32> = pipe!(None::<i32>, => |x| x * 2);
            assert_eq!(result, None);
        }

        #[rstest]
        fn option_lift_multiple_pure_functions() {
            let result = pipe!(
                Some(5),
                => |x| x + 1,
                => |x| x * 2
            );
            assert_eq!(result, Some(12)); // (5 + 1) * 2 = 12
        }

        #[rstest]
        fn option_lift_with_named_function() {
            fn double(x: i32) -> i32 {
                x * 2
            }
            let result = pipe!(Some(5), => double);
            assert_eq!(result, Some(10));
        }

        #[rstest]
        fn result_lift_single_pure_function() {
            let result: Result<i32, &str> = pipe!(Ok(5), => |x| x * 2);
            assert_eq!(result, Ok(10));
        }

        #[rstest]
        fn result_lift_with_err() {
            let result: Result<i32, &str> = pipe!(Err("error"), => |x: i32| x * 2);
            assert_eq!(result, Err("error"));
        }

        #[rstest]
        fn result_lift_multiple_pure_functions() {
            let result: Result<i32, &str> = pipe!(
                Ok(5),
                => |x| x + 1,
                => |x| x * 2
            );
            assert_eq!(result, Ok(12));
        }

        #[rstest]
        fn box_lift_single_pure_function() {
            let result = pipe!(Box::new(5), => |x| x * 2);
            assert_eq!(*result, 10);
        }

        #[rstest]
        fn box_lift_multiple_pure_functions() {
            let result = pipe!(
                Box::new(5),
                => |x| x + 1,
                => |x| x * 2
            );
            assert_eq!(*result, 12);
        }

        #[rstest]
        fn identity_lift_single_pure_function() {
            let result = pipe!(Identity::new(5), => |x| x * 2);
            assert_eq!(result, Identity::new(10));
        }

        #[rstest]
        fn identity_lift_multiple_pure_functions() {
            let result = pipe!(
                Identity::new(5),
                => |x| x + 1,
                => |x| x * 2
            );
            assert_eq!(result, Identity::new(12));
        }
    }

    mod bind_operator_tests {
        use super::*;

        #[rstest]
        fn option_bind_single_monadic_function() {
            let result = pipe!(Some(5), =>> |x| if x > 0 { Some(x * 2) } else { None });
            assert_eq!(result, Some(10));
        }

        #[rstest]
        fn option_bind_with_none_result() {
            let result = pipe!(Some(-5), =>> |x| if x > 0 { Some(x * 2) } else { None });
            assert_eq!(result, None);
        }

        #[rstest]
        fn option_bind_with_none_input() {
            let result: Option<i32> =
                pipe!(None::<i32>, =>> |x| if x > 0 { Some(x * 2) } else { None });
            assert_eq!(result, None);
        }

        #[rstest]
        #[allow(clippy::cast_possible_truncation)]
        fn option_bind_multiple_monadic_functions() {
            let safe_divide = |x: i32| if x != 0 { Some(100 / x) } else { None };
            let safe_sqrt = |x: i32| {
                if x >= 0 {
                    Some(f64::from(x).sqrt() as i32)
                } else {
                    None
                }
            };

            let result = pipe!(
                Some(4),
                =>> safe_divide,
                =>> safe_sqrt
            );
            assert_eq!(result, Some(5)); // 100/4 = 25, sqrt(25) = 5
        }

        #[rstest]
        fn option_bind_chain_with_failure_in_middle() {
            let result = pipe!(
                Some(0),
                =>> |x| if x != 0 { Some(100 / x) } else { None },
                =>> |x| Some(x * 2)
            );
            assert_eq!(result, None);
        }

        #[rstest]
        fn result_bind_single_monadic_function() {
            let result: Result<i32, &str> = pipe!(
                Ok(5),
                =>> |x| if x > 0 { Ok(x * 2) } else { Err("negative") }
            );
            assert_eq!(result, Ok(10));
        }

        #[rstest]
        fn result_bind_with_err_result() {
            let result: Result<i32, &str> = pipe!(
                Ok(-5),
                =>> |x| if x > 0 { Ok(x * 2) } else { Err("negative") }
            );
            assert_eq!(result, Err("negative"));
        }

        #[rstest]
        fn result_bind_with_err_input() {
            let result: Result<i32, &str> = pipe!(
                Err("initial error"),
                =>> |x: i32| if x > 0 { Ok(x * 2) } else { Err("negative") }
            );
            assert_eq!(result, Err("initial error"));
        }

        #[rstest]
        fn result_bind_multiple_monadic_functions() {
            let result: Result<i32, &str> = pipe!(
                Ok(10),
                =>> |x| if x > 0 { Ok(x + 5) } else { Err("not positive") },
                =>> |x| if x < 100 { Ok(x * 2) } else { Err("too large") }
            );
            assert_eq!(result, Ok(30));
        }

        #[rstest]
        fn box_bind_single_monadic_function() {
            let result = pipe!(Box::new(5), =>> |x| Box::new(x * 2));
            assert_eq!(*result, 10);
        }

        #[rstest]
        fn box_bind_multiple_monadic_functions() {
            let result = pipe!(
                Box::new(5),
                =>> |x| Box::new(x + 1),
                =>> |x| Box::new(x * 2)
            );
            assert_eq!(*result, 12);
        }

        #[rstest]
        fn identity_bind_single_monadic_function() {
            let result = pipe!(Identity::new(5), =>> |x| Identity::new(x * 2));
            assert_eq!(result, Identity::new(10));
        }

        #[rstest]
        fn identity_bind_multiple_monadic_functions() {
            let result = pipe!(
                Identity::new(5),
                =>> |x| Identity::new(x + 1),
                =>> |x| Identity::new(x * 2)
            );
            assert_eq!(result, Identity::new(12));
        }
    }

    mod mixed_operators_tests {
        use super::*;

        #[rstest]
        fn option_lift_then_bind() {
            let result = pipe!(
                Some(5),
                => |x| x + 1,                                    // lift: Some(6)
                =>> |x| if x > 0 { Some(x * 2) } else { None }   // bind: Some(12)
            );
            assert_eq!(result, Some(12));
        }

        #[rstest]
        fn option_bind_then_lift() {
            let result = pipe!(
                Some(5),
                =>> |x| if x > 0 { Some(x * 2) } else { None },  // bind: Some(10)
                => |x| x + 1                                     // lift: Some(11)
            );
            assert_eq!(result, Some(11));
        }

        #[rstest]
        fn option_complex_mixed_chain() {
            let result = pipe!(
                Some(10),
                => |x| x / 2,                                     // lift: Some(5)
                =>> |x| if x > 0 { Some(x + 10) } else { None },  // bind: Some(15)
                => |x| x * 2,                                     // lift: Some(30)
                =>> |x| Some(x - 5)                               // bind: Some(25)
            );
            assert_eq!(result, Some(25));
        }

        #[rstest]
        fn option_mixed_with_failure() {
            let result = pipe!(
                Some(10),
                => |x| x - 20,                                    // lift: Some(-10)
                =>> |x| if x > 0 { Some(x) } else { None },       // bind: None
                => |x| x * 2                                      // not executed
            );
            assert_eq!(result, None);
        }

        #[rstest]
        fn result_lift_then_bind() {
            let result: Result<i32, &str> = pipe!(
                Ok(5),
                => |x| x + 1,
                =>> |x| if x > 0 { Ok(x * 2) } else { Err("negative") }
            );
            assert_eq!(result, Ok(12));
        }

        #[rstest]
        fn result_bind_then_lift() {
            let result: Result<i32, &str> = pipe!(
                Ok(5),
                =>> |x| if x > 0 { Ok(x * 2) } else { Err("negative") },
                => |x| x + 1
            );
            assert_eq!(result, Ok(11));
        }

        #[rstest]
        fn box_mixed_operators() {
            let result = pipe!(
                Box::new(5),
                => |x| x + 1,           // lift: Box(6)
                =>> |x| Box::new(x * 2) // bind: Box(12)
            );
            assert_eq!(*result, 12);
        }

        #[rstest]
        fn identity_mixed_operators() {
            let result = pipe!(
                Identity::new(5),
                => |x| x + 1,                  // lift: Identity(6)
                =>> |x| Identity::new(x * 2)   // bind: Identity(12)
            );
            assert_eq!(result, Identity::new(12));
        }

        #[rstest]
        fn mixed_with_named_functions() {
            fn add_one(x: i32) -> i32 {
                x + 1
            }
            fn safe_double(x: i32) -> Option<i32> {
                if x > 0 { Some(x * 2) } else { None }
            }

            let result = pipe!(
                Some(5),
                => add_one,
                =>> safe_double
            );
            assert_eq!(result, Some(12));
        }
    }

    mod backward_compatibility_tests {
        use rstest::rstest;

        #[rstest]
        fn original_pipe_value_only_still_works() {
            let result = pipe!(42);
            assert_eq!(result, 42);
        }

        #[rstest]
        fn original_pipe_single_function_still_works() {
            let double = |x: i32| x * 2;
            let result = pipe!(5, double);
            assert_eq!(result, 10);
        }

        #[rstest]
        fn original_pipe_multiple_functions_still_works() {
            let add_one = |x: i32| x + 1;
            let double = |x: i32| x * 2;
            let square = |x: i32| x * x;
            let result = pipe!(3, square, double, add_one);
            assert_eq!(result, 19); // 3^2 = 9, 9*2 = 18, 18+1 = 19
        }

        #[rstest]
        fn original_pipe_with_trailing_comma_still_works() {
            let double = |x: i32| x * 2;
            let result = pipe!(5, double,);
            assert_eq!(result, 10);
        }
    }

    mod edge_cases_tests {
        use super::*;

        #[rstest]
        fn single_lift_operator_only() {
            let result = pipe!(Some(5), => |x| x * 2);
            assert_eq!(result, Some(10));
        }

        #[rstest]
        fn single_bind_operator_only() {
            let result = pipe!(Some(5), =>> |x| Some(x * 2));
            assert_eq!(result, Some(10));
        }

        #[rstest]
        fn many_lift_operators() {
            let result = pipe!(
                Some(1),
                => |x| x + 1,
                => |x| x + 1,
                => |x| x + 1,
                => |x| x + 1,
                => |x| x + 1
            );
            assert_eq!(result, Some(6));
        }

        #[rstest]
        fn many_bind_operators() {
            let result = pipe!(
                Some(1),
                =>> |x| Some(x + 1),
                =>> |x| Some(x + 1),
                =>> |x| Some(x + 1),
                =>> |x| Some(x + 1),
                =>> |x| Some(x + 1)
            );
            assert_eq!(result, Some(6));
        }

        #[rstest]
        fn alternating_lift_and_bind() {
            let result = pipe!(
                Some(1),
                => |x| x + 1,
                =>> |x| Some(x * 2),
                => |x| x + 1,
                =>> |x| Some(x * 2)
            );
            // 1 -> 2 -> 4 -> 5 -> 10
            assert_eq!(result, Some(10));
        }

        #[rstest]
        fn type_changing_through_pipeline() {
            let result = pipe!(
                Some(42),
                => |x: i32| x.to_string(),
                => |s: String| s.len()
            );
            assert_eq!(result, Some(2)); // "42" has length 2
        }

        #[rstest]
        fn type_changing_with_bind() {
            let result = pipe!(
                Some(42),
                =>> |x: i32| Some(x.to_string()),
                =>> |s: String| Some(s.len())
            );
            assert_eq!(result, Some(2));
        }
    }
}

#[cfg(all(test, feature = "effect"))]
mod io_pipe_tests {
    use crate::effect::IO;
    use rstest::rstest;

    #[rstest]
    fn io_lift_operator() {
        let result = pipe!(IO::pure(5), => |x| x * 2).run_unsafe();
        assert_eq!(result, 10);
    }

    #[rstest]
    fn io_bind_operator() {
        let result = pipe!(
            IO::pure(5),
            =>> |x| IO::pure(x * 2)
        )
        .run_unsafe();
        assert_eq!(result, 10);
    }

    #[rstest]
    fn io_mixed_with_pure_functions() {
        let result = pipe!(
            IO::pure(5),
            => |x| x + 1,            // lift: IO(6)
            =>> |x| IO::pure(x * 2), // bind: IO(12)
            => |x| x.to_string()     // lift: IO("12")
        )
        .run_unsafe();
        assert_eq!(result, "12");
    }

    #[rstest]
    fn io_chain_multiple_lifts() {
        let result = pipe!(
            IO::pure(1),
            => |x| x + 1,
            => |x| x * 2,
            => |x| x + 3
        )
        .run_unsafe();
        assert_eq!(result, 7); // ((1 + 1) * 2) + 3 = 7
    }

    #[rstest]
    fn io_chain_multiple_binds() {
        let result = pipe!(
            IO::pure(1),
            =>> |x| IO::pure(x + 1),
            =>> |x| IO::pure(x * 2),
            =>> |x| IO::pure(x + 3)
        )
        .run_unsafe();
        assert_eq!(result, 7);
    }

    #[rstest]
    fn io_preserves_deferred_execution() {
        use std::cell::Cell;
        use std::rc::Rc;

        let counter = Rc::new(Cell::new(0));
        let counter_clone = counter.clone();

        let io = pipe!(
            IO::new(move || {
                counter_clone.set(counter_clone.get() + 1);
                5
            }),
            => |x| x * 2
        );

        // IO が作成されただけでは実行されない
        assert_eq!(counter.get(), 0);

        // run_unsafe() を呼ぶと実行される
        let result = io.run_unsafe();
        assert_eq!(result, 10);
        assert_eq!(counter.get(), 1);
    }
}
