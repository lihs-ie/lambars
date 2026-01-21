//! Tests for AsyncIO Runtime sharing mechanism (aio-01-runtime-sharing)
//!
//! These tests verify that:
//! 1. Global runtime is a singleton
//! 2. Handle caching works correctly from both inside and outside runtime
//! 3. run_blocking behaves correctly in different contexts

#![cfg(feature = "async")]

use rstest::rstest;
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use lambars::effect::async_io::runtime::{global, handle, run_blocking};

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

/// Tests that run_blocking works from inside a runtime (uses block_in_place).
#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_run_blocking_inside_runtime() {
    // This should use block_in_place internally
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
