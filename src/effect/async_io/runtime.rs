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
//!    efficiently by using `block_in_place` when already inside a runtime,
//!    avoiding nested runtime panics.
//!
//! # Performance Characteristics
//!
//! - `global()`: O(1) after first initialization (static `LazyLock`)
//! - `handle()`: O(1) with thread-local caching
//! - `run_blocking()`: No additional Enter/Drop overhead when inside runtime
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
//! // Execute a future blocking
//! let result = run_blocking(async { 42 });
//! ```

use std::cell::RefCell;
use std::future::Future;
use std::sync::LazyLock;

use tokio::runtime::{Builder, Handle, Runtime};

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
// Blocking Execution
// =============================================================================

/// Executes a future synchronously, blocking the current thread.
///
/// This function provides an efficient way to run async code from synchronous
/// contexts. It handles the complexity of being inside or outside a tokio
/// runtime automatically:
///
/// - **Inside a runtime**: Uses `block_in_place` to avoid nested runtime panics
/// - **Outside a runtime**: Uses the global runtime's `block_on`
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
/// Panics if the future panics.
///
/// # Examples
///
/// ```rust,ignore
/// use lambars::effect::async_io::runtime::run_blocking;
///
/// // From synchronous code
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
/// // From inside a spawn_blocking task
/// #[tokio::test(flavor = "multi_thread")]
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
    // Check if we're inside a tokio runtime
    if Handle::try_current().is_ok() {
        // Inside a runtime: use block_in_place to avoid nested runtime issues
        // This moves the current task to a blocking thread and runs the future there
        tokio::task::block_in_place(|| {
            // We need to create a new runtime here because block_in_place
            // doesn't give us a way to run futures directly.
            // However, we can use the global runtime's handle to run the future.
            global().handle().block_on(future)
        })
    } else {
        // Outside a runtime: use global runtime's block_on
        global().block_on(future)
    }
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
    async fn run_blocking_inside_spawn_blocking() {
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
