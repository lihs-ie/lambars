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

/// Pipes an `AsyncIO` value through a series of transformations.
///
/// This macro is specifically designed for `AsyncIO` values because `AsyncIO`
/// cannot implement the `Functor` and `Monad` traits due to Rust's type system
/// limitations regarding `Send` bounds.
///
/// # Syntax
///
/// - `pipe_async!(async_io)` - Returns the `AsyncIO` unchanged
/// - `pipe_async!(async_io, => f)` - Applies `f` using `fmap` (lift operator)
/// - `pipe_async!(async_io, =>> f)` - Applies `f` using `flat_map` (bind operator)
/// - `pipe_async!(async_io, => f, =>> g, => h, ...)` - Chain multiple operations
///
/// # Operators
///
/// ## Lift Operator (`=>`)
///
/// The lift operator applies a pure function `A -> B` within the `AsyncIO` context.
/// It expands to a call to `AsyncIO::fmap`.
///
/// ```rust,ignore
/// pipe_async!(m, => f) // expands to: m.fmap(f)
/// ```
///
/// ## Bind Operator (`=>>`)
///
/// The bind operator applies a monadic function `A -> AsyncIO<B>`.
/// It expands to a call to `AsyncIO::flat_map`.
///
/// ```rust,ignore
/// pipe_async!(m, =>> f) // expands to: m.flat_map(f)
/// ```
///
/// # Type Constraints
///
/// - The initial value must be of type `AsyncIO<A>`
/// - For `=>`: The function must be `FnOnce(A) -> B + Send + 'static`, and `B: 'static`
/// - For `=>>`: The function must be `FnOnce(A) -> AsyncIO<B> + Send + 'static`, and `B: 'static`
///
/// # Examples
///
/// ## Value only
///
/// ```rust,ignore
/// use lambars::effect::AsyncIO;
/// use lambars::pipe_async;
///
/// let async_io = AsyncIO::pure(42);
/// let result = pipe_async!(async_io);
/// // result is the same as async_io
/// ```
///
/// ## Single lift
///
/// ```rust,ignore
/// use lambars::effect::AsyncIO;
/// use lambars::pipe_async;
///
/// let result = pipe_async!(AsyncIO::pure(5), => |x| x * 2);
/// assert_eq!(result.run_async().await, 10);
/// ```
///
/// ## Multiple lifts
///
/// ```rust,ignore
/// use lambars::effect::AsyncIO;
/// use lambars::pipe_async;
///
/// let result = pipe_async!(
///     AsyncIO::pure(5),
///     => |x| x + 1,
///     => |x| x * 2
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
/// fn double(x: i32) -> i32 { x * 2 }
/// fn add_async(x: i32) -> AsyncIO<i32> { AsyncIO::pure(x + 10) }
///
/// let result = pipe_async!(
///     AsyncIO::pure(5),
///     => double,      // fmap: AsyncIO(10)
///     =>> add_async   // flat_map: AsyncIO(20)
/// );
/// assert_eq!(result.run_async().await, 20);
/// ```
#[macro_export]
macro_rules! pipe_async {
    // Base case: value only
    ($value:expr) => {
        $value
    };

    // Lift operator with optional trailing comma (terminal case)
    ($value:expr, => $function:expr $(,)?) => {{
        $value.fmap($function)
    }};

    // Lift operator with continuation
    ($value:expr, => $function:expr, $($rest:tt)+) => {{
        let __pipe_async_intermediate = $value.fmap($function);
        $crate::pipe_async!(__pipe_async_intermediate, $($rest)+)
    }};

    // Bind operator with optional trailing comma (terminal case)
    ($value:expr, =>> $function:expr $(,)?) => {{
        $value.flat_map($function)
    }};

    // Bind operator with continuation
    ($value:expr, =>> $function:expr, $($rest:tt)+) => {{
        let __pipe_async_intermediate = $value.flat_map($function);
        $crate::pipe_async!(__pipe_async_intermediate, $($rest)+)
    }};
}

#[cfg(test)]
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
}
