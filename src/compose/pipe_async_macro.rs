//! The `pipe_async!` macro for AsyncIO-specific left-to-right function application.
//!
//! This module provides the [`pipe_async!`] macro which applies functions
//! from left to right specifically for [`AsyncIO`](crate::effect::AsyncIO) values.
//!
//! # Background
//!
//! Due to Rust's type system limitations, `AsyncIO` cannot implement the standard
//! `Functor` and `Monad` traits. The issue is that `AsyncIO` requires `Send` bounds
//! on closures and values, but the trait definitions do not include these bounds.
//! As a result, `AsyncIO` cannot be used with the regular [`pipe!`](crate::pipe) macro.
//!
//! `pipe_async!` solves this by directly calling `AsyncIO`'s inherent methods
//! (`fmap` and `flat_map`) instead of trait methods.
//!
//! # Operators
//!
//! - **Lift operator** (`=>`): Apply a pure function using `fmap`
//! - **Bind operator** (`=>>`): Apply a monadic function using `flat_map`
//!
//! # Examples
//!
//! ## Basic usage
//!
//! ```rust,ignore
//! use lambars::effect::AsyncIO;
//! use lambars::pipe_async;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Single lift operation
//!     let result = pipe_async!(AsyncIO::pure(5), => |x| x * 2);
//!     assert_eq!(result.run_async().await, 10);
//!
//!     // Single bind operation
//!     let result = pipe_async!(
//!         AsyncIO::pure(5),
//!         =>> |x| AsyncIO::pure(x * 2)
//!     );
//!     assert_eq!(result.run_async().await, 10);
//! }
//! ```
//!
//! ## Chaining operations
//!
//! ```rust,ignore
//! use lambars::effect::AsyncIO;
//! use lambars::pipe_async;
//!
//! #[tokio::main]
//! async fn main() {
//!     let result = pipe_async!(
//!         AsyncIO::pure(10),
//!         => |x| x / 2,                     // fmap: AsyncIO(5)
//!         =>> |x| AsyncIO::pure(x + 10),    // flat_map: AsyncIO(15)
//!         => |x| x * 2                      // fmap: AsyncIO(30)
//!     );
//!     assert_eq!(result.run_async().await, 30);
//! }
//! ```
//!
//! ## Deferred execution
//!
//! `pipe_async!` preserves the lazy execution semantics of `AsyncIO`.
//! Side effects are not executed until `run_async().await` is called.
//!
//! ```rust,ignore
//! use lambars::effect::AsyncIO;
//! use lambars::pipe_async;
//! use std::sync::Arc;
//! use std::sync::atomic::{AtomicBool, Ordering};
//!
//! #[tokio::main]
//! async fn main() {
//!     let executed = Arc::new(AtomicBool::new(false));
//!     let executed_clone = executed.clone();
//!
//!     let workflow = pipe_async!(
//!         AsyncIO::new(move || async move {
//!             executed_clone.store(true, Ordering::SeqCst);
//!             5
//!         }),
//!         => |x| x * 2
//!     );
//!
//!     // Not executed yet
//!     assert!(!executed.load(Ordering::SeqCst));
//!
//!     // Execute the workflow
//!     let result = workflow.run_async().await;
//!     assert!(executed.load(Ordering::SeqCst));
//!     assert_eq!(result, 10);
//! }
//! ```

/// Pipes a value through a series of transformations within the `AsyncIO` context.
///
/// This macro is specifically designed for `AsyncIO` values because `AsyncIO`
/// cannot implement the `Functor` and `Monad` traits due to Rust's type system
/// limitations regarding `Send` bounds.
///
/// # Syntax
///
/// - `pipe_async!(value)` - Converts value to `AsyncIO` using `IntoPipeAsync`
/// - `pipe_async!(value, f)` - Applies `f` using `fmap` (comma syntax)
/// - `pipe_async!(value, => f)` - Applies `f` using `fmap` (explicit lift operator)
/// - `pipe_async!(value, =>> f)` - Applies `f` using `flat_map` (bind operator)
/// - `pipe_async!(value, f, => g, =>> h, ...)` - Chain multiple operations
///
/// # Initial Value
///
/// The initial value is converted to `AsyncIO` using the `IntoPipeAsync` trait:
/// - `AsyncIO<A>` is returned unchanged
/// - Primitive types (`i32`, `String`, `bool`, etc.) are wrapped with `AsyncIO::pure`
/// - User-defined types can be wrapped with `Pure<A>` to enable conversion
///
/// # Operators
///
/// ## Comma Syntax (Implicit fmap)
///
/// A comma-separated function is applied as `fmap`:
///
/// ```rust,ignore
/// pipe_async!(x, f) // expands to: x.into_pipe_async().fmap(f)
/// ```
///
/// ## Lift Operator (`=>`)
///
/// The lift operator applies a pure function `A -> B` within the `AsyncIO` context.
/// It is equivalent to the comma syntax.
///
/// ```rust,ignore
/// pipe_async!(m, => f) // expands to: m.into_pipe_async().fmap(f)
/// ```
///
/// ## Bind Operator (`=>>`)
///
/// The bind operator applies a monadic function `A -> AsyncIO<B>`.
/// It expands to a call to `AsyncIO::flat_map`.
///
/// ```rust,ignore
/// pipe_async!(m, =>> f) // expands to: m.into_pipe_async().flat_map(f)
/// ```
///
/// # Type Constraints
///
/// - The initial value must implement `IntoPipeAsync`
/// - For `=>` and comma: The function must be `FnOnce(A) -> B + Send + 'static`, and `B: 'static`
/// - For `=>>`: The function must be `FnOnce(A) -> AsyncIO<B> + Send + 'static`, and `B: 'static`
///
/// # Examples
///
/// ## Value only (primitive type)
///
/// ```rust,ignore
/// use lambars::pipe_async;
///
/// let result = pipe_async!(42);
/// assert_eq!(result.run_async().await, 42);
/// ```
///
/// ## Comma syntax (implicit fmap)
///
/// ```rust,ignore
/// use lambars::pipe_async;
///
/// let result = pipe_async!(5, |x| x * 2);
/// assert_eq!(result.run_async().await, 10);
/// ```
///
/// ## Multiple fmaps with comma syntax
///
/// ```rust,ignore
/// use lambars::pipe_async;
///
/// let result = pipe_async!(
///     5,
///     |x| x + 1,
///     |x| x * 2
/// );
/// assert_eq!(result.run_async().await, 12); // (5 + 1) * 2
/// ```
///
/// ## Mixed operators
///
/// ```rust,ignore
/// use lambars::effect::AsyncIO;
/// use lambars::pipe_async;
///
/// let result = pipe_async!(
///     5,
///     |x| x + 1,              // comma (fmap): 6
///     => |x| x * 2,           // explicit fmap: 12
///     =>> |x| AsyncIO::pure(x + 3)  // flat_map: 15
/// );
/// assert_eq!(result.run_async().await, 15);
/// ```
///
/// ## User-defined types with Pure wrapper
///
/// ```rust,ignore
/// use lambars::effect::Pure;
/// use lambars::pipe_async;
///
/// struct MyData { value: i32 }
///
/// let result = pipe_async!(Pure(MyData { value: 42 }), |d| d.value * 2);
/// assert_eq!(result.run_async().await, 84);
/// ```
#[macro_export]
macro_rules! pipe_async {
    // Base case: value only - convert to AsyncIO
    ($value:expr) => {{
        $crate::effect::IntoPipeAsync::into_pipe_async($value)
    }};

    // Bind operator with optional trailing comma (terminal case) - highest priority
    ($value:expr, =>> $function:expr $(,)?) => {{
        $crate::effect::IntoPipeAsync::into_pipe_async($value).flat_map($function)
    }};

    // Bind operator with continuation - second priority
    ($value:expr, =>> $function:expr, $($rest:tt)+) => {{
        let __pipe_async_intermediate = $crate::effect::IntoPipeAsync::into_pipe_async($value).flat_map($function);
        $crate::pipe_async!(__pipe_async_intermediate, $($rest)+)
    }};

    // Lift operator with optional trailing comma (terminal case)
    ($value:expr, => $function:expr $(,)?) => {{
        $crate::effect::IntoPipeAsync::into_pipe_async($value).fmap($function)
    }};

    // Lift operator with continuation
    ($value:expr, => $function:expr, $($rest:tt)+) => {{
        let __pipe_async_intermediate = $crate::effect::IntoPipeAsync::into_pipe_async($value).fmap($function);
        $crate::pipe_async!(__pipe_async_intermediate, $($rest)+)
    }};

    // Comma syntax (implicit fmap) with optional trailing comma (terminal case)
    ($value:expr, $function:expr $(,)?) => {{
        $crate::effect::IntoPipeAsync::into_pipe_async($value).fmap($function)
    }};

    // Comma syntax (implicit fmap) with continuation
    ($value:expr, $function:expr, $($rest:tt)+) => {{
        let __pipe_async_intermediate = $crate::effect::IntoPipeAsync::into_pipe_async($value).fmap($function);
        $crate::pipe_async!(__pipe_async_intermediate, $($rest)+)
    }};
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use crate::effect::AsyncIO;
    use rstest::rstest;

    #[rstest]
    #[tokio::test]
    async fn test_pipe_async_value_only() {
        let async_io = AsyncIO::pure(42);
        let result = pipe_async!(async_io);
        assert_eq!(result.run_async().await, 42);
    }

    #[rstest]
    #[tokio::test]
    async fn test_pipe_async_single_lift() {
        let result = pipe_async!(AsyncIO::pure(5), => |x| x * 2);
        assert_eq!(result.run_async().await, 10);
    }

    #[rstest]
    #[tokio::test]
    async fn test_pipe_async_multiple_lifts() {
        let result = pipe_async!(
            AsyncIO::pure(5),
            => |x| x + 1,
            => |x| x * 2,
            => |x| x + 3
        );
        assert_eq!(result.run_async().await, 15); // ((5 + 1) * 2) + 3
    }

    #[rstest]
    #[tokio::test]
    async fn test_pipe_async_type_conversion() {
        let result = pipe_async!(
            AsyncIO::pure(42),
            => |x: i32| x.to_string(),
            => |s: String| s.len()
        );
        assert_eq!(result.run_async().await, 2);
    }

    #[rstest]
    #[tokio::test]
    async fn test_pipe_async_single_bind() {
        let result = pipe_async!(
            AsyncIO::pure(5),
            =>> |x| AsyncIO::pure(x * 2)
        );
        assert_eq!(result.run_async().await, 10);
    }

    #[rstest]
    #[tokio::test]
    async fn test_pipe_async_multiple_binds() {
        let result = pipe_async!(
            AsyncIO::pure(5),
            =>> |x| AsyncIO::pure(x + 1),
            =>> |x| AsyncIO::pure(x * 2),
            =>> |x| AsyncIO::pure(x + 3)
        );
        assert_eq!(result.run_async().await, 15);
    }

    #[rstest]
    #[tokio::test]
    async fn test_pipe_async_mixed_operators() {
        let result = pipe_async!(
            AsyncIO::pure(10),
            => |x| x / 2,
            =>> |x| AsyncIO::pure(x + 10),
            => |x| x * 2
        );
        assert_eq!(result.run_async().await, 30);
    }

    #[rstest]
    #[tokio::test]
    async fn test_pipe_async_alternating_operators() {
        let result = pipe_async!(
            AsyncIO::pure(1),
            => |x| x + 1,
            =>> |x| AsyncIO::pure(x * 2),
            => |x| x + 1,
            =>> |x| AsyncIO::pure(x * 2)
        );
        assert_eq!(result.run_async().await, 10);
    }

    #[rstest]
    #[tokio::test]
    async fn test_pipe_async_with_named_functions() {
        fn double(x: i32) -> i32 {
            x * 2
        }

        fn add_async(x: i32) -> AsyncIO<i32> {
            AsyncIO::pure(x + 10)
        }

        let result = pipe_async!(
            AsyncIO::pure(5),
            => double,
            =>> add_async
        );
        assert_eq!(result.run_async().await, 20);
    }

    #[rstest]
    #[tokio::test]
    async fn test_pipe_async_trailing_comma() {
        let result = pipe_async!(
            AsyncIO::pure(5),
            => |x| x * 2,
        );
        assert_eq!(result.run_async().await, 10);
    }

    #[rstest]
    #[tokio::test]
    async fn test_pipe_async_trailing_comma_bind() {
        let result = pipe_async!(
            AsyncIO::pure(5),
            =>> |x| AsyncIO::pure(x * 2),
        );
        assert_eq!(result.run_async().await, 10);
    }

    #[rstest]
    #[tokio::test]
    async fn test_pipe_async_preserves_lazy_execution() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let async_io = pipe_async!(
            AsyncIO::new(move || async move {
                executed_clone.store(true, Ordering::SeqCst);
                5
            }),
            => |x| x * 2
        );

        assert!(!executed.load(Ordering::SeqCst));

        let result = async_io.run_async().await;
        assert!(executed.load(Ordering::SeqCst));
        assert_eq!(result, 10);
    }

    #[rstest]
    #[tokio::test]
    async fn test_pipe_async_lazy_with_flat_map() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let counter = Arc::new(AtomicUsize::new(0));
        let counter1 = counter.clone();
        let counter2 = counter.clone();

        let async_io = pipe_async!(
            AsyncIO::new(move || async move {
                counter1.fetch_add(1, Ordering::SeqCst);
                5
            }),
            =>> move |x| {
                let c = counter2;
                AsyncIO::new(move || async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    x * 2
                })
            }
        );

        assert_eq!(counter.load(Ordering::SeqCst), 0);

        let result = async_io.run_async().await;
        assert_eq!(counter.load(Ordering::SeqCst), 2);
        assert_eq!(result, 10);
    }

    #[rstest]
    #[tokio::test]
    async fn test_pipe_async_complex_pipeline() {
        let result = pipe_async!(
            AsyncIO::pure(100_i32),
            => |x| x / 10,
            => |x| x.to_string(),
            =>> |s: String| AsyncIO::pure(s.len()),
            => |len| len * 5,
            =>> |x| AsyncIO::pure(x + 1)
        );
        assert_eq!(result.run_async().await, 11);
    }

    #[rstest]
    #[tokio::test]
    async fn test_pipe_async_with_struct_transformation() {
        #[derive(Debug, PartialEq)]
        struct User {
            id: i32,
            name: String,
        }

        #[derive(Debug, PartialEq)]
        struct Profile {
            user_id: i32,
            bio: String,
        }

        let result = pipe_async!(
            AsyncIO::pure(42),
            => |id| User { id, name: "Alice".to_string() },
            =>> |user: User| AsyncIO::pure(Profile {
                user_id: user.id,
                bio: format!("User {}", user.name),
            })
        );

        let profile = result.run_async().await;
        assert_eq!(profile.user_id, 42);
        assert_eq!(profile.bio, "User Alice");
    }

    #[rstest]
    #[tokio::test]
    async fn test_pipe_async_many_operations() {
        let result = pipe_async!(
            AsyncIO::pure(0),
            => |x| x + 1,
            => |x| x + 1,
            => |x| x + 1,
            => |x| x + 1,
            => |x| x + 1,
            => |x| x + 1,
            => |x| x + 1,
            => |x| x + 1,
            => |x| x + 1,
            => |x| x + 1
        );
        assert_eq!(result.run_async().await, 10);
    }

    #[rstest]
    #[tokio::test]
    async fn test_pipe_async_with_unit_type() {
        let result = pipe_async!(
            AsyncIO::pure(()),
            => |()| 42
        );
        assert_eq!(result.run_async().await, 42);
    }

    #[rstest]
    #[tokio::test]
    async fn test_pipe_async_returning_unit() {
        let result = pipe_async!(
            AsyncIO::pure(42),
            => |_| ()
        );
        assert_eq!(result.run_async().await, ());
    }

    // =========================================================================
    // pipe_async! Extension Tests (comma syntax, IntoPipeAsync, Pure)
    // =========================================================================

    mod extension_tests {
        use crate::effect::{AsyncIO, Pure};
        use rstest::rstest;

        // =====================================================================
        // Comma Equals Lift Law
        // =====================================================================

        #[rstest]
        #[case(1)]
        #[case(42)]
        #[case(-100)]
        #[tokio::test]
        async fn comma_equals_lift(#[case] value: i32) {
            let add_one = |x: i32| x + 1;
            let result_comma = pipe_async!(value, add_one);
            let result_lift = pipe_async!(value, => add_one);
            assert_eq!(
                result_comma.run_async().await,
                result_lift.run_async().await
            );
        }

        // =====================================================================
        // Functor Composition Law
        // =====================================================================

        #[rstest]
        #[case(1)]
        #[case(5)]
        #[case(10)]
        #[tokio::test]
        async fn functor_composition(#[case] value: i32) {
            let add_one = |x: i32| x + 1;
            let double = |x: i32| x * 2;
            let result_chain = pipe_async!(value, add_one, double);
            let result_composed = pipe_async!(value, move |v| double(add_one(v)));
            assert_eq!(
                result_chain.run_async().await,
                result_composed.run_async().await
            );
        }

        // =====================================================================
        // Backward Compatibility Tests
        // =====================================================================

        #[rstest]
        #[tokio::test]
        async fn backward_compatibility_lift() {
            let result = pipe_async!(AsyncIO::pure(5), => |x| x * 2);
            assert_eq!(result.run_async().await, 10);
        }

        #[rstest]
        #[tokio::test]
        async fn backward_compatibility_bind() {
            let result = pipe_async!(AsyncIO::pure(5), =>> |x| AsyncIO::pure(x * 2));
            assert_eq!(result.run_async().await, 10);
        }

        #[rstest]
        #[tokio::test]
        async fn backward_compatibility_mixed() {
            let result = pipe_async!(
                AsyncIO::pure(5),
                => |x| x * 2,
                =>> |x| AsyncIO::pure(x + 1)
            );
            assert_eq!(result.run_async().await, 11);
        }

        // =====================================================================
        // Pure Wrapper Tests
        // =====================================================================

        #[rstest]
        #[tokio::test]
        async fn test_pipe_async_with_pure_wrapper() {
            #[derive(Debug)]
            struct MyData {
                value: i32,
            }

            let wrapped = Pure(MyData { value: 42 });
            let result = pipe_async!(wrapped, |d| d.value * 2);
            assert_eq!(result.run_async().await, 84);
        }

        #[rstest]
        #[tokio::test]
        async fn test_pipe_async_with_pure_wrapper_chained() {
            #[derive(Debug)]
            struct MyData {
                value: i32,
            }

            let result = pipe_async!(Pure(MyData { value: 10 }), |d| d.value, |v| v + 5, |v| v
                * 2);
            assert_eq!(result.run_async().await, 30);
        }

        // =====================================================================
        // Unit Type Tests
        // =====================================================================

        #[rstest]
        #[tokio::test]
        async fn test_pipe_async_unit_type() {
            let result = pipe_async!((), |()| 42);
            assert_eq!(result.run_async().await, 42);
        }

        #[rstest]
        #[tokio::test]
        async fn test_pipe_async_unit_type_to_unit() {
            let result = pipe_async!(42, |_| ());
            assert_eq!(result.run_async().await, ());
        }

        // =====================================================================
        // Primitive Value Initial Tests
        // =====================================================================

        #[rstest]
        #[tokio::test]
        async fn test_pipe_async_primitive_i32() {
            let result = pipe_async!(42);
            assert_eq!(result.run_async().await, 42);
        }

        #[rstest]
        #[tokio::test]
        async fn test_pipe_async_primitive_string() {
            let result = pipe_async!(String::from("hello"), |s: String| s.len());
            assert_eq!(result.run_async().await, 5);
        }

        #[rstest]
        #[tokio::test]
        async fn test_pipe_async_primitive_bool() {
            let result = pipe_async!(true, |b: bool| i32::from(b));
            assert_eq!(result.run_async().await, 1);
        }

        #[rstest]
        #[tokio::test]
        async fn test_pipe_async_static_str() {
            let result = pipe_async!("hello", |s: &str| s.len());
            assert_eq!(result.run_async().await, 5);
        }

        // =====================================================================
        // Mixed Operator Tests with Comma
        // =====================================================================

        #[rstest]
        #[tokio::test]
        async fn test_pipe_async_comma_then_bind() {
            let result = pipe_async!(
                5,
                |x| x + 1,
                =>> |x| AsyncIO::pure(x * 2)
            );
            assert_eq!(result.run_async().await, 12);
        }

        #[rstest]
        #[tokio::test]
        async fn test_pipe_async_bind_then_comma() {
            let result = pipe_async!(
                AsyncIO::pure(5),
                =>> |x| AsyncIO::pure(x + 1),
                |x| x * 2
            );
            assert_eq!(result.run_async().await, 12);
        }

        #[rstest]
        #[tokio::test]
        async fn test_pipe_async_all_operators_mixed() {
            let result = pipe_async!(
                5,
                |x| x + 1,                      // comma (implicit fmap): 6
                => |x: i32| x * 2,              // explicit fmap: 12
                =>> |x| AsyncIO::pure(x + 3),   // flat_map: 15
                |x: i32| x.to_string()          // comma (implicit fmap): "15"
            );
            assert_eq!(result.run_async().await, "15");
        }

        // =====================================================================
        // Laziness Tests with New Syntax
        // =====================================================================

        /// Tests that fmap on Pure values applies the function immediately (eager evaluation).
        ///
        /// This is an intentional optimization: since fmap expects pure functions,
        /// the evaluation timing is not observable in terms of the result value
        /// (referential transparency). For Pure values, we apply the function
        /// immediately to avoid Box allocation.
        ///
        /// If lazy evaluation is needed (e.g., for side effects), use `flat_map` instead.
        #[rstest]
        #[tokio::test]
        async fn test_pipe_async_comma_pure_value_eager_evaluation() {
            use std::sync::Arc;
            use std::sync::atomic::{AtomicBool, Ordering};

            let executed = Arc::new(AtomicBool::new(false));
            let executed_clone = executed.clone();

            let async_io = pipe_async!(42, move |x| {
                executed_clone.store(true, Ordering::SeqCst);
                x * 2
            });

            assert!(executed.load(Ordering::SeqCst));

            let result = async_io.run_async().await;
            assert_eq!(result, 84);
        }

        /// Tests that fmap on Deferred values maintains lazy evaluation.
        ///
        /// When starting from a Deferred value (created by `AsyncIO::new`),
        /// the transformation function is not executed until `run_async()` is called.
        #[rstest]
        #[tokio::test]
        async fn test_pipe_async_deferred_preserves_lazy_execution() {
            use std::sync::Arc;
            use std::sync::atomic::{AtomicBool, Ordering};

            let thunk_executed = Arc::new(AtomicBool::new(false));
            let function_executed = Arc::new(AtomicBool::new(false));
            let thunk_executed_clone = thunk_executed.clone();
            let function_executed_clone = function_executed.clone();

            let async_io = AsyncIO::new(move || {
                let flag = thunk_executed_clone;
                async move {
                    flag.store(true, Ordering::SeqCst);
                    42
                }
            })
            .fmap(move |x| {
                function_executed_clone.store(true, Ordering::SeqCst);
                x * 2
            });

            assert!(!thunk_executed.load(Ordering::SeqCst));
            assert!(!function_executed.load(Ordering::SeqCst));

            let result = async_io.run_async().await;
            assert!(thunk_executed.load(Ordering::SeqCst));
            assert!(function_executed.load(Ordering::SeqCst));
            assert_eq!(result, 84);
        }
    }
}
