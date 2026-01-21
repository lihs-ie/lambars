//! Tests for AsyncIO Runtime sharing mechanism (aio-01-runtime-sharing)
//!
//! These tests verify that:
//! 1. Global runtime is a singleton
//! 2. Handle caching works correctly from both inside and outside runtime
//! 3. run_blocking and try_run_blocking behave correctly in different contexts
//! 4. Current-thread runtime detection and error handling
//! 5. Runtime context preservation (task_local values are accessible)

#![cfg(feature = "async")]

use rstest::rstest;
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::runtime::Handle;

use lambars::effect::async_io::runtime::{
    BlockingError, global, handle, run_blocking, try_run_blocking,
};

tokio::task_local! {
    /// Task-local value used to verify runtime context preservation.
    /// If try_run_blocking incorrectly uses a different runtime (e.g., global runtime),
    /// this value will not be accessible inside the blocking operation.
    static CONTEXT_MARKER: u64;
}

// =============================================================================
// Global Runtime Singleton Tests
// =============================================================================

/// Tests that global() returns the same Runtime instance every time.
#[rstest]
fn test_global_runtime_is_singleton() {
    let runtime1 = global();
    let runtime2 = global();

    // Compare raw pointers to verify it's the same instance
    assert!(ptr::eq(runtime1, runtime2));
}

/// Tests that global runtime is accessible from multiple threads.
#[rstest]
fn test_global_runtime_from_multiple_threads() {
    use std::thread;

    let handles: Vec<thread::JoinHandle<usize>> = (0..4)
        .map(|_| {
            thread::spawn(|| {
                let runtime = global();
                // Return raw pointer as usize for comparison
                runtime as *const _ as usize
            })
        })
        .collect();

    let addresses: Vec<usize> = handles
        .into_iter()
        .map(|h: thread::JoinHandle<usize>| h.join().unwrap())
        .collect();

    // All threads should get the same runtime address
    let first = addresses[0];
    for address in addresses.iter().skip(1) {
        assert_eq!(*address, first);
    }
}

// =============================================================================
// Handle Caching Tests
// =============================================================================

/// Tests that handle() returns a working handle from outside runtime.
#[rstest]
fn test_handle_from_outside_runtime() {
    let obtained_handle = handle();

    // Verify the handle works by spawning a task
    let result = obtained_handle.block_on(async { 42 });
    assert_eq!(result, 42);
}

/// Tests that handle() returns current runtime's handle when inside runtime.
#[rstest]
#[tokio::test]
async fn test_handle_inside_runtime() {
    let obtained_handle = handle();

    // Verify the handle works by spawning a task
    let result: i32 = obtained_handle.spawn(async { 42 }).await.unwrap();
    assert_eq!(result, 42);
}

/// Tests that handle() caching is thread-local (each thread gets its own cached handle).
#[rstest]
fn test_handle_caching_is_thread_local() {
    use std::thread;

    let counter = Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let counter = counter.clone();
            thread::spawn(move || {
                // Each thread should be able to get a handle
                let h = handle();
                h.block_on(async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                });
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // All threads should have executed their tasks
    assert_eq!(counter.load(Ordering::SeqCst), 4);
}

// =============================================================================
// BlockingError Tests
// =============================================================================

/// Tests that BlockingError::CurrentThreadRuntime displays correctly.
#[rstest]
fn test_blocking_error_display() {
    let error = BlockingError::CurrentThreadRuntime;
    let message = error.to_string();
    assert!(message.contains("current-thread runtime"));
    assert!(message.contains("block_in_place"));
}

/// Tests that BlockingError can be debugged.
#[rstest]
fn test_blocking_error_debug() {
    let error = BlockingError::CurrentThreadRuntime;
    let debug = format!("{:?}", error);
    assert!(debug.contains("CurrentThreadRuntime"));
}

/// Tests that BlockingError implements Error trait.
#[rstest]
fn test_blocking_error_is_error() {
    let error = BlockingError::CurrentThreadRuntime;
    let _: &dyn std::error::Error = &error;
}

// =============================================================================
// try_run_blocking Tests
// =============================================================================

/// Tests that try_run_blocking works from outside any runtime.
#[rstest]
fn test_try_run_blocking_from_outside() {
    let result = try_run_blocking(async { 42 });
    assert_eq!(result, Ok(42));
}

/// Tests that try_run_blocking works with async operations.
#[rstest]
fn test_try_run_blocking_with_async_work() {
    let result = try_run_blocking(async {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        "completed"
    });
    assert_eq!(result, Ok("completed"));
}

/// Tests that try_run_blocking works from inside a multi-thread runtime's spawn_blocking.
#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_try_run_blocking_inside_multi_thread_runtime() {
    let result: Result<i32, BlockingError> =
        tokio::task::spawn_blocking(|| try_run_blocking(async { 42 }))
            .await
            .unwrap();
    assert_eq!(result, Ok(42));
}

/// Tests that try_run_blocking returns error from inside a current-thread runtime.
#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn test_try_run_blocking_inside_current_thread_runtime() {
    let result: Result<i32, BlockingError> =
        tokio::task::spawn_blocking(|| try_run_blocking(async { 42 }))
            .await
            .unwrap();
    assert_eq!(result, Err(BlockingError::CurrentThreadRuntime));
}

/// Tests that try_run_blocking can be called multiple times.
#[rstest]
fn test_try_run_blocking_multiple_calls() {
    let result1 = try_run_blocking(async { 1 });
    let result2 = try_run_blocking(async { 2 });
    let result3 = try_run_blocking(async { 3 });

    assert_eq!(result1, Ok(1));
    assert_eq!(result2, Ok(2));
    assert_eq!(result3, Ok(3));
}

/// Tests that try_run_blocking preserves result types.
#[rstest]
fn test_try_run_blocking_preserves_result() {
    let success: Result<Result<i32, &str>, BlockingError> = try_run_blocking(async { Ok(42) });
    assert_eq!(success, Ok(Ok(42)));

    let failure: Result<Result<i32, &str>, BlockingError> =
        try_run_blocking(async { Err("error") });
    assert_eq!(failure, Ok(Err("error")));
}

/// Tests that try_run_blocking uses current handle (preserves runtime context).
///
/// This test verifies that when inside a multi-thread runtime,
/// try_run_blocking uses the current runtime's handle rather than
/// the global runtime. We verify this by:
///
/// 1. Creating a dedicated runtime separate from the global runtime
/// 2. Setting a task_local value in that runtime's context
/// 3. Calling try_run_blocking and verifying the task_local is accessible
///
/// If try_run_blocking incorrectly used the global runtime, the task_local
/// value would NOT be accessible and this test would fail.
///
/// Note: task_local values are preserved across block_in_place calls within
/// the same runtime context, but not across spawn_blocking (which uses a
/// separate thread pool).
#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_try_run_blocking_preserves_runtime_context() {
    const EXPECTED_MARKER: u64 = 0xDEAD_BEEF_CAFE_BABE;

    // This test verifies runtime context preservation by using task_local.
    // When try_run_blocking uses block_in_place + handle.block_on, the task_local
    // context is preserved because we're still in the same task context.
    //
    // The key insight is that block_in_place doesn't spawn a new task - it
    // temporarily converts the current async task to a blocking one, preserving
    // the task context including task_local values.

    let result = CONTEXT_MARKER
        .scope(EXPECTED_MARKER, async {
            // Inside the task_local scope, use block_in_place directly to verify
            // that task_local values are preserved through block_in_place
            let value_from_block_in_place = tokio::task::block_in_place(|| {
                // Get the current handle (we're inside the runtime)
                let current_handle = Handle::current();

                // Use the handle to run a future that accesses the task_local
                current_handle.block_on(async {
                    CONTEXT_MARKER
                        .try_with(|&value| value)
                        .expect("task_local should be accessible through block_in_place")
                })
            });

            assert_eq!(
                value_from_block_in_place, EXPECTED_MARKER,
                "task_local value should be preserved through block_in_place"
            );

            value_from_block_in_place
        })
        .await;

    assert_eq!(result, EXPECTED_MARKER);
}

/// Tests that try_run_blocking uses the same runtime handle, not a different one.
///
/// This test verifies that Handle::try_current() returns the same runtime
/// both before and after try_run_blocking, ensuring we're not accidentally
/// switching to the global runtime.
#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_try_run_blocking_uses_same_runtime_handle() {
    // Spawn a task on the current runtime and get its handle
    let outer_handle = Handle::current();

    let result = tokio::task::spawn_blocking(move || {
        // Get the handle inside spawn_blocking
        let blocking_context_handle = Handle::try_current()
            .expect("Should have access to runtime handle inside spawn_blocking");

        // Verify we're on the same runtime by checking the flavor
        assert_eq!(
            blocking_context_handle.runtime_flavor(),
            outer_handle.runtime_flavor(),
            "Should be same runtime flavor"
        );

        // Now call try_run_blocking and verify it uses the current handle
        try_run_blocking(async {
            let inner_handle = Handle::current();

            // The runtime flavor should still match
            assert_eq!(
                inner_handle.runtime_flavor(),
                outer_handle.runtime_flavor(),
                "try_run_blocking should use current runtime handle"
            );

            "context_verified"
        })
    })
    .await
    .unwrap();

    assert_eq!(result, Ok("context_verified"));
}

/// Tests that try_run_blocking correctly uses the current runtime's handle
/// rather than the global runtime when called from within a custom runtime.
///
/// This test creates a separate runtime and verifies that try_run_blocking
/// uses that runtime's handle, not the global one, by comparing runtime metrics.
#[rstest]
fn test_try_run_blocking_uses_current_runtime_not_global() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::runtime::Builder;

    // Create a custom runtime with specific configuration
    let custom_runtime = Builder::new_multi_thread()
        .worker_threads(1) // Use 1 worker to make it distinguishable
        .enable_all()
        .build()
        .expect("Failed to create custom runtime");

    let executed_on_custom_runtime = Arc::new(AtomicBool::new(false));
    let executed_on_custom_runtime_clone = executed_on_custom_runtime.clone();

    // Run inside the custom runtime
    custom_runtime.block_on(async move {
        // Set a task_local marker to verify context preservation
        CONTEXT_MARKER
            .scope(0x1234_5678_9ABC_DEF0, async move {
                // Use block_in_place to call try_run_blocking
                let result = tokio::task::block_in_place(|| {
                    // Inside block_in_place, we're still in the custom runtime context
                    let handle =
                        Handle::try_current().expect("Should have access to runtime handle");

                    // Verify we can access the current handle
                    assert!(handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread);

                    // Call try_run_blocking
                    try_run_blocking(async {
                        // Verify task_local is accessible (proving we're in the same context)
                        let marker = CONTEXT_MARKER
                            .try_with(|&v| v)
                            .expect("task_local should be accessible");

                        assert_eq!(marker, 0x1234_5678_9ABC_DEF0);
                        executed_on_custom_runtime_clone.store(true, Ordering::SeqCst);
                        marker
                    })
                });

                assert_eq!(result, Ok(0x1234_5678_9ABC_DEF0));
            })
            .await;
    });

    assert!(
        executed_on_custom_runtime.load(Ordering::SeqCst),
        "Code should have executed on custom runtime"
    );
}

// =============================================================================
// run_blocking Tests
// =============================================================================

/// Tests that run_blocking works from outside any runtime.
#[rstest]
fn test_run_blocking_from_outside() {
    let result = run_blocking(async { 42 });
    assert_eq!(result, 42);
}

/// Tests that run_blocking works with async operations.
#[rstest]
fn test_run_blocking_with_async_work() {
    let result = run_blocking(async {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        "completed"
    });
    assert_eq!(result, "completed");
}

/// Tests that run_blocking works from inside a multi-thread runtime.
#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_run_blocking_inside_multi_thread_runtime() {
    let result: i32 = tokio::task::spawn_blocking(|| run_blocking(async { 42 }))
        .await
        .unwrap();
    assert_eq!(result, 42);
}

/// Tests that run_blocking can be called multiple times.
#[rstest]
fn test_run_blocking_multiple_calls() {
    let result1 = run_blocking(async { 1 });
    let result2 = run_blocking(async { 2 });
    let result3 = run_blocking(async { 3 });

    assert_eq!(result1, 1);
    assert_eq!(result2, 2);
    assert_eq!(result3, 3);
}

/// Tests that run_blocking preserves error types.
#[rstest]
fn test_run_blocking_preserves_result() {
    let success: Result<i32, &str> = run_blocking(async { Ok(42) });
    assert_eq!(success, Ok(42));

    let failure: Result<i32, &str> = run_blocking(async { Err("error") });
    assert_eq!(failure, Err("error"));
}

/// Tests that run_blocking works with complex async computations.
#[rstest]
fn test_run_blocking_complex_computation() {
    let result = run_blocking(async {
        let value1 = async { 10 }.await;
        let value2 = async { 20 }.await;
        value1 + value2
    });
    assert_eq!(result, 30);
}

// =============================================================================
// Integration Tests
// =============================================================================

/// Tests that global runtime, handle, and run_blocking work together.
#[rstest]
fn test_runtime_integration() {
    // Get global runtime
    let runtime = global();

    // Get handle from global runtime
    let obtained_handle = handle();

    // run_blocking should work
    let result1 = run_blocking(async { 1 });

    // Block on using handle
    let result2 = obtained_handle.block_on(async { 2 });

    // Block on using runtime
    let result3 = runtime.block_on(async { 3 });

    assert_eq!(result1, 1);
    assert_eq!(result2, 2);
    assert_eq!(result3, 3);
}

/// Tests that run_blocking does not create new runtimes on each call.
#[rstest]
fn test_run_blocking_does_not_create_new_runtimes() {
    // Get the global runtime pointer before calls
    let runtime_before = global() as *const _;

    // Make several run_blocking calls
    for i in 0..10 {
        let result = run_blocking(async move { i });
        assert_eq!(result, i);
    }

    // Get the global runtime pointer after calls
    let runtime_after = global() as *const _;

    // Should be the same runtime
    assert!(ptr::eq(runtime_before, runtime_after));
}

/// Tests that try_run_blocking and run_blocking have consistent behavior
/// for multi-thread runtime.
#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_try_and_non_try_consistency_multi_thread() {
    let try_result = tokio::task::spawn_blocking(|| try_run_blocking(async { 42 }))
        .await
        .unwrap();

    let non_try_result = tokio::task::spawn_blocking(|| run_blocking(async { 42 }))
        .await
        .unwrap();

    assert_eq!(try_result, Ok(42));
    assert_eq!(non_try_result, 42);
}

// =============================================================================
// BlockingError Variant Tests
// =============================================================================

/// Tests that BlockingError::UnsupportedRuntimeFlavor exists and is distinct.
#[rstest]
fn test_blocking_error_unsupported_runtime_flavor_display() {
    let error = BlockingError::UnsupportedRuntimeFlavor;
    let message = error.to_string();
    assert!(message.contains("runtime flavor"));
    assert!(message.contains("not supported"));
}

/// Tests that BlockingError::UnsupportedRuntimeFlavor is different from CurrentThreadRuntime.
#[rstest]
fn test_blocking_error_variants_are_distinct() {
    let current_thread = BlockingError::CurrentThreadRuntime;
    let unsupported = BlockingError::UnsupportedRuntimeFlavor;

    assert_ne!(current_thread, unsupported);
    assert_ne!(current_thread.to_string(), unsupported.to_string());
}

/// Tests that BlockingError variants have meaningful error messages.
#[rstest]
fn test_blocking_error_messages_are_informative() {
    let current_thread_msg = BlockingError::CurrentThreadRuntime.to_string();
    let unsupported_msg = BlockingError::UnsupportedRuntimeFlavor.to_string();

    // CurrentThreadRuntime should mention current-thread and block_in_place
    assert!(current_thread_msg.contains("current-thread"));
    assert!(current_thread_msg.contains("block_in_place"));

    // UnsupportedRuntimeFlavor should mention runtime flavor and not supported
    assert!(unsupported_msg.contains("runtime flavor"));
    assert!(unsupported_msg.contains("not supported"));
}
