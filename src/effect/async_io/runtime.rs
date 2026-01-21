//! Runtime sharing mechanism for `AsyncIO`.
//!
//! This module provides a global tokio runtime and utilities for efficient
//! async execution without creating new runtimes or `EnterGuard`s on each call.
//!
//! # Design Philosophy
//!
//! To minimize overhead from runtime initialization and `EnterGuard` creation,
//! this module provides:
//!
//! 1. **Global Runtime**: A lazily-initialized multi-thread runtime that is
//!    shared across all `AsyncIO` operations. The runtime is created once and
//!    never dropped (static lifetime).
//!
//! 2. **Handle Caching**: Thread-local caching of runtime handles to avoid
//!    repeated lookups. When inside a runtime, the current handle is used
//!    directly. When outside, the global runtime's handle is cached.
//!
//! 3. **Blocking Execution**: A `run_blocking` function that executes futures
//!    efficiently by using `block_in_place` when already inside a multi-thread
//!    runtime, avoiding nested runtime panics.
//!
//! # Performance Characteristics
//!
//! - `global()`: O(1) after first initialization (static `LazyLock`)
//! - `handle()`: O(1) with thread-local caching
//! - `run_blocking()`: No additional Enter/Drop overhead when inside runtime
//!
//! # Runtime Flavor Considerations
//!
//! This module handles different runtime flavors appropriately:
//!
//! - **Multi-thread runtime**: Uses `block_in_place` for efficient blocking
//!   execution without nested runtime panics.
//! - **Current-thread runtime**: Returns an error via `BlockingError::CurrentThreadRuntime`
//!   because `block_in_place` is not supported in current-thread runtimes.
//!
//! When inside a runtime, the current runtime's handle is preferred over the
//! global runtime to preserve tracing context and metrics settings.
//!
//! # Examples
//!
//! ```rust,ignore
//! use lambars::effect::async_io::runtime::{global, handle, run_blocking};
//!
//! // Get the global runtime
//! let runtime = global();
//!
//! // Get a cached handle
//! let obtained_handle = handle();
//!
//! // Execute a future blocking (returns Result to handle current_thread runtime)
//! let result = run_blocking(async { 42 });
//! ```

use std::cell::RefCell;
use std::error::Error;
use std::fmt;
use std::future::Future;
use std::sync::LazyLock;

use tokio::runtime::{Builder, Handle, Runtime, RuntimeFlavor};

// =============================================================================
// Global Runtime
// =============================================================================

/// Global tokio runtime initialized lazily on first access.
///
/// This runtime is configured with:
/// - Multi-thread scheduler
/// - Worker threads equal to the number of CPU cores
/// - All features enabled (io, time, etc.)
///
/// The runtime has static lifetime and is never dropped.
static GLOBAL_RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    Builder::new_multi_thread()
        .worker_threads(num_cpus::get())
        .enable_all()
        .build()
        .expect("Failed to create global tokio runtime")
});

/// Returns a reference to the global runtime.
///
/// The runtime is lazily initialized on first call and shared across
/// all subsequent calls. The same instance is returned from any thread.
///
/// # Returns
///
/// A static reference to the global `Runtime`.
///
/// # Examples
///
/// ```rust,ignore
/// use lambars::effect::async_io::runtime::global;
///
/// let runtime = global();
/// runtime.block_on(async {
///     // async work here
/// });
/// ```
#[inline]
#[must_use]
pub fn global() -> &'static Runtime {
    &GLOBAL_RUNTIME
}

// =============================================================================
// Handle Caching
// =============================================================================

thread_local! {
    /// Thread-local cached handle to the global runtime.
    ///
    /// This avoids repeated calls to `global().handle()` by caching the
    /// handle per-thread. The handle is cloned on first access.
    static CACHED_HANDLE: RefCell<Option<Handle>> = const { RefCell::new(None) };
}

/// Returns a handle to the current or global runtime.
///
/// This function first attempts to get the current runtime's handle
/// (if running inside a tokio runtime). If not inside a runtime,
/// it returns a cached handle to the global runtime.
///
/// # Handle Priority
///
/// 1. If inside a tokio runtime: returns `Handle::current()`
/// 2. Otherwise: returns cached `global().handle()` (initializing if needed)
///
/// # Returns
///
/// A cloned `Handle` to the runtime.
///
/// # Examples
///
/// ```rust,ignore
/// use lambars::effect::async_io::runtime::handle;
///
/// // From outside any runtime
/// let obtained_handle = handle();
/// obtained_handle.spawn(async { /* work */ });
///
/// // From inside a runtime (e.g., in a #[tokio::test])
/// #[tokio::test]
/// async fn test() {
///     let obtained_handle = handle(); // Returns current runtime's handle
/// }
/// ```
///
/// # Note
///
/// This function never panics. The internal `unwrap()` is safe because
/// the cached value is always set before being accessed.
#[inline]
#[must_use]
#[allow(clippy::missing_panics_doc)] // unwrap is safe: we just set the value
pub fn handle() -> Handle {
    // First, try to get the current runtime's handle
    if let Ok(current_handle) = Handle::try_current() {
        return current_handle;
    }

    // Not inside a runtime, use cached global handle
    CACHED_HANDLE.with(|cached| {
        let mut cached = cached.borrow_mut();
        if cached.is_none() {
            *cached = Some(global().handle().clone());
        }
        cached.as_ref().unwrap().clone()
    })
}

// =============================================================================
// Blocking Error
// =============================================================================

/// Error type for blocking execution failures.
///
/// This error is returned when `try_run_blocking` cannot execute a future
/// due to runtime constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockingError {
    /// Cannot use `block_in_place` in a current-thread runtime.
    ///
    /// The `block_in_place` function is only supported in multi-thread
    /// runtimes. When called from within a current-thread runtime,
    /// this error is returned instead of panicking.
    CurrentThreadRuntime,

    /// The runtime flavor is not supported for blocking execution.
    ///
    /// This error is returned when `try_run_blocking` is called from within
    /// a runtime with an unknown or unsupported flavor (e.g., a new flavor
    /// added in a future version of tokio).
    ///
    /// This variant exists for forward compatibility with future tokio versions
    /// that may introduce new runtime flavors.
    UnsupportedRuntimeFlavor,
}

impl fmt::Display for BlockingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CurrentThreadRuntime => {
                write!(
                    formatter,
                    "cannot execute blocking operation in current-thread runtime: \
                     block_in_place is only supported in multi-thread runtimes"
                )
            }
            Self::UnsupportedRuntimeFlavor => {
                write!(
                    formatter,
                    "cannot execute blocking operation: \
                     the runtime flavor is not supported for blocking execution"
                )
            }
        }
    }
}

impl Error for BlockingError {}

// =============================================================================
// Blocking Execution
// =============================================================================

/// Attempts to execute a future synchronously, blocking the current thread.
///
/// This function provides an efficient way to run async code from synchronous
/// contexts. It handles the complexity of being inside or outside a tokio
/// runtime automatically:
///
/// - **Inside a multi-thread runtime**: Uses `block_in_place` with the current
///   runtime's handle to avoid nested runtime panics while preserving the
///   caller's runtime context (tracing, metrics, etc.).
/// - **Inside a current-thread runtime**: Returns `Err(BlockingError::CurrentThreadRuntime)`
///   because `block_in_place` is not supported in current-thread runtimes.
/// - **Outside a runtime**: Uses the global runtime's `block_on`.
///
/// # Runtime Context Preservation
///
/// When called from within a runtime, this function uses `Handle::current()`
/// to preserve the caller's runtime context. This ensures that tracing spans,
/// metrics, and other runtime-specific settings are properly inherited.
///
/// # Arguments
///
/// * `future` - The future to execute.
///
/// # Returns
///
/// `Ok(T)` with the future's output on success, or `Err(BlockingError)` if
/// execution is not possible in the current context.
///
/// # Errors
///
/// Returns `Err(BlockingError::CurrentThreadRuntime)` when called from within
/// a current-thread tokio runtime, as `block_in_place` is not supported in
/// that context.
///
/// # Examples
///
/// ```rust,ignore
/// use lambars::effect::async_io::runtime::try_run_blocking;
///
/// // From synchronous code (outside any runtime)
/// let result = try_run_blocking(async {
///     tokio::time::sleep(std::time::Duration::from_millis(10)).await;
///     42
/// });
/// assert_eq!(result, Ok(42));
/// ```
///
/// ```rust,ignore
/// use lambars::effect::async_io::runtime::{try_run_blocking, BlockingError};
///
/// // From inside a current-thread runtime
/// #[tokio::test(flavor = "current_thread")]
/// async fn test() {
///     let result = tokio::task::spawn_blocking(|| {
///         try_run_blocking(async { 42 })
///     }).await.unwrap();
///     // Returns error because current_thread runtime doesn't support block_in_place
///     assert_eq!(result, Err(BlockingError::CurrentThreadRuntime));
/// }
/// ```
///
/// ```rust,ignore
/// use lambars::effect::async_io::runtime::try_run_blocking;
///
/// // From inside a multi-thread runtime's spawn_blocking
/// #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
/// async fn test() {
///     let result = tokio::task::spawn_blocking(|| {
///         try_run_blocking(async { 42 })
///     }).await.unwrap();
///     assert_eq!(result, Ok(42));
/// }
/// ```
#[inline]
pub fn try_run_blocking<F, T>(future: F) -> Result<T, BlockingError>
where
    F: Future<Output = T>,
{
    // Check if we're inside a tokio runtime
    if let Ok(current_handle) = Handle::try_current() {
        // Inside a runtime: check runtime flavor
        match current_handle.runtime_flavor() {
            RuntimeFlavor::MultiThread => {
                // Multi-thread runtime: use block_in_place with current handle
                // to preserve the caller's runtime context (tracing, metrics, etc.)
                Ok(tokio::task::block_in_place(|| {
                    current_handle.block_on(future)
                }))
            }
            RuntimeFlavor::CurrentThread => {
                // Current-thread runtime: block_in_place is not supported
                Err(BlockingError::CurrentThreadRuntime)
            }
            // Handle any future runtime flavors conservatively
            _ => {
                // Unknown runtime flavor: return a specific error for forward compatibility
                // This allows callers to distinguish between "current-thread doesn't support
                // block_in_place" and "unknown runtime flavor that may or may not support it"
                Err(BlockingError::UnsupportedRuntimeFlavor)
            }
        }
    } else {
        // Outside a runtime: use global runtime's block_on
        Ok(global().block_on(future))
    }
}

/// Executes a future synchronously, blocking the current thread.
///
/// This is a convenience wrapper around [`try_run_blocking`] that panics
/// on error. Use this when you know you're in a multi-thread runtime context
/// or when a panic is acceptable.
///
/// # Arguments
///
/// * `future` - The future to execute.
///
/// # Returns
///
/// The output of the future.
///
/// # Panics
///
/// - Panics if called from within a current-thread runtime.
/// - Panics if the future panics.
///
/// # Examples
///
/// ```rust,ignore
/// use lambars::effect::async_io::runtime::run_blocking;
///
/// // From synchronous code (outside any runtime)
/// let result = run_blocking(async {
///     tokio::time::sleep(std::time::Duration::from_millis(10)).await;
///     42
/// });
/// assert_eq!(result, 42);
/// ```
///
/// ```rust,ignore
/// use lambars::effect::async_io::runtime::run_blocking;
///
/// // From inside a multi-thread runtime's spawn_blocking
/// #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
/// async fn test() {
///     let result = tokio::task::spawn_blocking(|| {
///         run_blocking(async { 42 })
///     }).await.unwrap();
///     assert_eq!(result, 42);
/// }
/// ```
#[inline]
pub fn run_blocking<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    try_run_blocking(future).expect("run_blocking failed")
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::ptr;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    // =========================================================================
    // global() Tests
    // =========================================================================

    #[rstest]
    fn global_returns_same_instance() {
        let runtime1 = global();
        let runtime2 = global();
        assert!(ptr::eq(runtime1, runtime2));
    }

    #[rstest]
    fn global_runtime_is_multi_threaded() {
        // Verify we can spawn multiple concurrent tasks
        let counter = Arc::new(AtomicUsize::new(0));
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let counter = counter.clone();
                global().spawn(async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                })
            })
            .collect();

        global().block_on(async {
            for handle in handles {
                handle.await.unwrap();
            }
        });

        assert_eq!(counter.load(Ordering::SeqCst), 4);
    }

    // =========================================================================
    // handle() Tests
    // =========================================================================

    #[rstest]
    fn handle_works_from_outside_runtime() {
        let obtained_handle = handle();
        let result = obtained_handle.block_on(async { 42 });
        assert_eq!(result, 42);
    }

    #[rstest]
    #[tokio::test]
    async fn handle_works_from_inside_runtime() {
        let obtained_handle = handle();
        let result = obtained_handle.spawn(async { 42 }).await.unwrap();
        assert_eq!(result, 42);
    }

    #[rstest]
    fn handle_caching_works() {
        // Call handle multiple times from the same thread
        let handle1 = handle();
        let handle2 = handle();

        // Both should work
        let result1 = handle1.block_on(async { 1 });
        let result2 = handle2.block_on(async { 2 });

        assert_eq!(result1, 1);
        assert_eq!(result2, 2);
    }

    // =========================================================================
    // BlockingError Tests
    // =========================================================================

    #[rstest]
    fn blocking_error_display() {
        let error = BlockingError::CurrentThreadRuntime;
        let message = error.to_string();
        assert!(message.contains("current-thread runtime"));
        assert!(message.contains("block_in_place"));
    }

    #[rstest]
    fn blocking_error_debug() {
        let error = BlockingError::CurrentThreadRuntime;
        let debug = format!("{error:?}");
        assert!(debug.contains("CurrentThreadRuntime"));
    }

    #[rstest]
    fn blocking_error_equality() {
        let error1 = BlockingError::CurrentThreadRuntime;
        let error2 = BlockingError::CurrentThreadRuntime;
        assert_eq!(error1, error2);
    }

    #[rstest]
    fn blocking_error_clone() {
        let error = BlockingError::CurrentThreadRuntime;
        let cloned = error;
        assert_eq!(error, cloned);
    }

    #[rstest]
    fn blocking_error_unsupported_runtime_flavor_display() {
        let error = BlockingError::UnsupportedRuntimeFlavor;
        let message = error.to_string();
        assert!(message.contains("runtime flavor"));
        assert!(message.contains("not supported"));
    }

    #[rstest]
    fn blocking_error_unsupported_runtime_flavor_debug() {
        let error = BlockingError::UnsupportedRuntimeFlavor;
        let debug = format!("{error:?}");
        assert!(debug.contains("UnsupportedRuntimeFlavor"));
    }

    #[rstest]
    fn blocking_error_variants_are_distinct() {
        let current_thread = BlockingError::CurrentThreadRuntime;
        let unsupported = BlockingError::UnsupportedRuntimeFlavor;
        assert_ne!(current_thread, unsupported);
    }

    // =========================================================================
    // try_run_blocking() Tests
    // =========================================================================

    #[rstest]
    fn try_run_blocking_from_outside_runtime() {
        let result = try_run_blocking(async { 42 });
        assert_eq!(result, Ok(42));
    }

    #[rstest]
    fn try_run_blocking_with_complex_future() {
        let result = try_run_blocking(async {
            let value1 = async { 10 }.await;
            let value2 = async { 20 }.await;
            value1 + value2
        });
        assert_eq!(result, Ok(30));
    }

    #[rstest]
    fn try_run_blocking_preserves_result_types() {
        let ok_result: Result<Result<i32, &str>, BlockingError> =
            try_run_blocking(async { Ok(42) });
        assert_eq!(ok_result, Ok(Ok(42)));

        let err_result: Result<Result<i32, &str>, BlockingError> =
            try_run_blocking(async { Err("error") });
        assert_eq!(err_result, Ok(Err("error")));
    }

    #[rstest]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn try_run_blocking_inside_multi_thread_runtime() {
        let result = tokio::task::spawn_blocking(|| try_run_blocking(async { 42 }))
            .await
            .unwrap();
        assert_eq!(result, Ok(42));
    }

    #[rstest]
    #[tokio::test(flavor = "current_thread")]
    async fn try_run_blocking_inside_current_thread_runtime() {
        let result = tokio::task::spawn_blocking(|| try_run_blocking(async { 42 }))
            .await
            .unwrap();
        assert_eq!(result, Err(BlockingError::CurrentThreadRuntime));
    }

    #[rstest]
    fn try_run_blocking_multiple_calls() {
        let results: Vec<Result<i32, BlockingError>> = (0..10)
            .map(|i| try_run_blocking(async move { i }))
            .collect();

        let expected: Vec<Result<i32, BlockingError>> = (0..10).map(Ok).collect();
        assert_eq!(results, expected);
    }

    // =========================================================================
    // run_blocking() Tests
    // =========================================================================

    #[rstest]
    fn run_blocking_from_outside_runtime() {
        let result = run_blocking(async { 42 });
        assert_eq!(result, 42);
    }

    #[rstest]
    fn run_blocking_with_complex_future() {
        let result = run_blocking(async {
            let value1 = async { 10 }.await;
            let value2 = async { 20 }.await;
            value1 + value2
        });
        assert_eq!(result, 30);
    }

    #[rstest]
    fn run_blocking_preserves_result_types() {
        let ok_result: Result<i32, &str> = run_blocking(async { Ok(42) });
        assert_eq!(ok_result, Ok(42));

        let err_result: Result<i32, &str> = run_blocking(async { Err("error") });
        assert_eq!(err_result, Err("error"));
    }

    #[rstest]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn run_blocking_inside_multi_thread_spawn_blocking() {
        let result = tokio::task::spawn_blocking(|| run_blocking(async { 42 }))
            .await
            .unwrap();
        assert_eq!(result, 42);
    }

    #[rstest]
    fn run_blocking_multiple_calls() {
        let results: Vec<i32> = (0..10).map(|i| run_blocking(async move { i })).collect();

        let expected: Vec<i32> = (0..10).collect();
        assert_eq!(results, expected);
    }

    // =========================================================================
    // Thread Safety Tests
    // =========================================================================

    #[rstest]
    fn global_accessible_from_multiple_threads() {
        let results: Vec<i32> = (0..4)
            .map(|i| thread::spawn(move || run_blocking(async move { i })))
            .map(|h| h.join().unwrap())
            .collect();

        // All threads should have executed successfully
        assert_eq!(results.len(), 4);
        for (i, result) in results.into_iter().enumerate() {
            assert_eq!(result, i32::try_from(i).unwrap());
        }
    }
}
