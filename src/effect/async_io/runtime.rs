//! Runtime sharing mechanism for `AsyncIO`.
//!
//! Provides a global tokio runtime and utilities for efficient async execution
//! without creating new runtimes or `EnterGuard`s on each call.
//!
//! # Components
//!
//! - **Global Runtime**: Lazily-initialized multi-thread runtime (static lifetime)
//! - **Handle Caching**: Thread-local handle caching for O(1) access
//! - **Blocking Execution**: `run_blocking`/`try_run_blocking` for sync contexts
//!
//! # Runtime Behavior
//!
//! | Context | `try_run_blocking` Result |
//! |---------|---------------------------|
//! | Outside runtime | `Ok(T)` via `global().block_on()` |
//! | Multi-thread runtime | `Ok(T)` via `block_in_place` |
//! | Current-thread runtime | `Err(CurrentThreadRuntime)` |
//! | Unknown flavor | `Err(UnsupportedRuntimeFlavor)` |
//!
//! # Limitations
//!
//! `block_in_place` panics in multi-thread runtime when called from:
//! - `LocalSet::run_until()`
//! - Contexts with `disallow_block_in_place` enabled
//!
//! **Workaround**: Use `spawn_blocking` to move to a worker thread first.
//!
//! # Security
//!
//! `runtime_id()` returns an opaque counter value, not a memory address,
//! avoiding ASLR information leakage.
//!
//! # Example
//!
//! ```rust,ignore
//! use lambars::effect::async_io::runtime::{run_blocking, try_run_blocking};
//!
//! let result = run_blocking(async { 42 });
//! assert_eq!(result, 42);
//!
//! let result = try_run_blocking(async { 42 });
//! assert_eq!(result, Ok(42));
//! ```

use std::cell::RefCell;
use std::error::Error;
use std::fmt;
use std::future::Future;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::runtime::{Builder, Handle, Runtime, RuntimeFlavor};

// =============================================================================
// Global Runtime
// =============================================================================

/// Counter for generating unique, ASLR-safe runtime IDs.
static RUNTIME_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Unique ID assigned to the global runtime (initialized once on first access).
static GLOBAL_RUNTIME_ID: LazyLock<u64> =
    LazyLock::new(|| RUNTIME_ID_COUNTER.fetch_add(1, Ordering::Relaxed) + 1);

/// Global multi-thread tokio runtime (static lifetime, never dropped).
static GLOBAL_RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    // Ensure runtime ID is initialized before the runtime itself
    let _ = *GLOBAL_RUNTIME_ID;

    Builder::new_multi_thread()
        .worker_threads(num_cpus::get())
        .enable_all()
        .build()
        .expect("Failed to create global tokio runtime")
});

/// Returns a reference to the lazily-initialized global runtime.
#[inline]
#[must_use]
pub fn global() -> &'static Runtime {
    &GLOBAL_RUNTIME
}

/// Returns an opaque, ASLR-safe identifier for the global runtime.
#[inline]
#[must_use]
pub fn runtime_id() -> u64 {
    *GLOBAL_RUNTIME_ID
}

// =============================================================================
// Handle Caching
// =============================================================================

thread_local! {
    /// Thread-local cache for the global runtime's handle (avoids repeated clones).
    static CACHED_HANDLE: RefCell<Option<Handle>> = const { RefCell::new(None) };
}

/// Returns a handle to the current runtime (if inside one) or the cached global runtime handle.
///
/// Priority: current runtime handle > cached global runtime handle.
/// The global handle is cloned once per thread and cached thereafter.
#[inline]
#[must_use]
pub fn handle() -> Handle {
    if let Ok(current_handle) = Handle::try_current() {
        return current_handle;
    }

    CACHED_HANDLE.with(|cell| {
        let mut cached = cell.borrow_mut();
        if let Some(ref cached_handle) = *cached {
            return cached_handle.clone();
        }

        let global_handle = global().handle().clone();
        *cached = Some(global_handle.clone());
        global_handle
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

/// Executes a future synchronously, blocking the current thread.
///
/// Automatically handles different runtime contexts:
/// - **Outside runtime**: Uses `global().block_on()`
/// - **Multi-thread runtime**: Uses `block_in_place` + `handle.block_on()`
/// - **Current-thread runtime**: Returns `Err(CurrentThreadRuntime)`
///
/// # Errors
///
/// - `CurrentThreadRuntime`: Called from a current-thread runtime
/// - `UnsupportedRuntimeFlavor`: Called from an unknown runtime flavor
///
/// # Panics
///
/// In multi-thread runtime, panics if called from `LocalSet::run_until()`
/// or when `disallow_block_in_place` is enabled.
///
/// # Example
///
/// ```rust,ignore
/// use lambars::effect::async_io::runtime::try_run_blocking;
///
/// let result = try_run_blocking(async { 42 });
/// assert_eq!(result, Ok(42));
/// ```
#[inline]
pub fn try_run_blocking<F, T>(future: F) -> Result<T, BlockingError>
where
    F: Future<Output = T>,
{
    if let Ok(current_handle) = Handle::try_current() {
        match current_handle.runtime_flavor() {
            RuntimeFlavor::MultiThread => {
                // Preserve caller's runtime context (tracing, metrics, etc.)
                // Panics in LocalSet::run_until() or disallow_block_in_place contexts
                Ok(tokio::task::block_in_place(|| {
                    current_handle.block_on(future)
                }))
            }
            RuntimeFlavor::CurrentThread => Err(BlockingError::CurrentThreadRuntime),
            _ => Err(BlockingError::UnsupportedRuntimeFlavor),
        }
    } else {
        Ok(global().block_on(future))
    }
}

/// Convenience wrapper around [`try_run_blocking`] that panics on error.
///
/// # Panics
///
/// - In current-thread runtime (`BlockingError::CurrentThreadRuntime`)
/// - In unsupported runtime flavor (`BlockingError::UnsupportedRuntimeFlavor`)
/// - In `LocalSet::run_until()` or `disallow_block_in_place` contexts
/// - If the future panics
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
    // runtime_id() Tests
    // =========================================================================

    #[rstest]
    fn runtime_id_is_consistent() {
        let id1 = runtime_id();
        let id2 = runtime_id();
        assert_eq!(id1, id2);
    }

    #[rstest]
    fn runtime_id_is_nonzero() {
        // The runtime ID should be a positive number (generated from counter starting at 1)
        let id = runtime_id();
        assert!(id > 0, "runtime_id should be nonzero");
    }

    #[rstest]
    fn runtime_id_same_across_threads() {
        let main_id = runtime_id();
        let thread_id = thread::spawn(runtime_id).join().unwrap();
        assert_eq!(main_id, thread_id);
    }

    #[rstest]
    fn runtime_id_does_not_expose_memory_address() {
        // The runtime ID should NOT be the memory address of the global runtime.
        // This test ensures we're not leaking ASLR information.
        let id = runtime_id();
        let pointer_value = std::ptr::from_ref::<Runtime>(global()) as u64;

        // The ID should NOT equal the pointer address.
        // This is the primary security requirement - we don't want to leak
        // memory address information through the runtime ID.
        assert_ne!(
            id, pointer_value,
            "runtime_id should not equal the pointer value (would leak ASLR information)"
        );

        // Additionally verify that the ID is generated from a counter mechanism
        // by checking it's a reasonable positive value (counter starts at 1).
        assert!(
            id > 0,
            "runtime_id should be a positive counter value, got {id}"
        );
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

    #[rstest]
    fn handle_caching_is_thread_local() {
        // Each thread should have its own cached handle
        let results: Vec<i32> = (0..4)
            .map(|i| {
                thread::spawn(move || {
                    // First call caches the handle
                    let obtained_handle = handle();
                    // Second call should return the cached handle
                    let _ = handle();
                    obtained_handle.block_on(async move { i })
                })
            })
            .map(|h| h.join().unwrap())
            .collect();

        assert_eq!(results, vec![0, 1, 2, 3]);
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
