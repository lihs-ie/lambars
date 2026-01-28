#![cfg(feature = "async")]
//! AsyncIO Error Handling Optimization Tests - Phase 3: aio-03-handlers-zero-alloc
//!
//! These tests verify that `finally_async`, `on_error`, and `retry_with_factory`
//! are correctly implemented using the state machine approach without additional
//! Box allocations from `AsyncIO::new()`.
//!
//! Test coverage:
//! - finally_async: cleanup execution on both success and error paths
//! - on_error: callback execution only on error, preserving the original error
//! - retry_with_factory: retry logic with max attempts

use lambars::effect::AsyncIO;
use rstest::rstest;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// =============================================================================
// finally_async Tests
// =============================================================================

/// finally_async should execute cleanup on success path.
#[rstest]
#[tokio::test]
async fn test_finally_async_success_path() {
    let cleanup_executed = Arc::new(AtomicBool::new(false));
    let cleanup_executed_clone = cleanup_executed.clone();

    let result = AsyncIO::pure(42)
        .finally_async(move || {
            let flag = cleanup_executed_clone.clone();
            async move {
                flag.store(true, Ordering::SeqCst);
            }
        })
        .await;

    assert_eq!(result, 42);
    assert!(
        cleanup_executed.load(Ordering::SeqCst),
        "cleanup should be executed on success path"
    );
}

/// finally_async should execute cleanup on error path (via panic handling).
#[rstest]
#[tokio::test]
async fn test_finally_async_error_path() {
    let cleanup_executed = Arc::new(AtomicBool::new(false));
    let cleanup_executed_clone = cleanup_executed.clone();

    // Create a result type to represent error
    let result: Result<i32, &str> = AsyncIO::pure(Err("error"))
        .finally_async(move || {
            let flag = cleanup_executed_clone.clone();
            async move {
                flag.store(true, Ordering::SeqCst);
            }
        })
        .await;

    assert!(result.is_err());
    assert!(
        cleanup_executed.load(Ordering::SeqCst),
        "cleanup should be executed on error path"
    );
}

/// finally_async should preserve the original result value.
#[rstest]
#[tokio::test]
async fn test_finally_async_preserves_result() {
    let result = AsyncIO::pure(100)
        .finally_async(|| async {
            // Cleanup does something but doesn't affect the result
        })
        .await;

    assert_eq!(result, 100);
}

/// finally_async should be lazy - cleanup should not execute until awaited.
#[rstest]
#[tokio::test]
async fn test_finally_async_is_lazy() {
    let cleanup_executed = Arc::new(AtomicBool::new(false));
    let cleanup_executed_clone = cleanup_executed.clone();

    let _async_io = AsyncIO::pure(42).finally_async(move || {
        let flag = cleanup_executed_clone.clone();
        async move {
            flag.store(true, Ordering::SeqCst);
        }
    });

    // Not yet awaited - cleanup should not have executed
    assert!(
        !cleanup_executed.load(Ordering::SeqCst),
        "cleanup should not execute before await"
    );
}

/// finally_async should work with chained operations.
#[rstest]
#[tokio::test]
async fn test_finally_async_with_chain() {
    let cleanup_executed = Arc::new(AtomicBool::new(false));
    let cleanup_executed_clone = cleanup_executed.clone();

    let result = AsyncIO::pure(10)
        .fmap(|x| x * 2)
        .finally_async(move || {
            let flag = cleanup_executed_clone.clone();
            async move {
                flag.store(true, Ordering::SeqCst);
            }
        })
        .await;

    assert_eq!(result, 20);
    assert!(cleanup_executed.load(Ordering::SeqCst));
}

// =============================================================================
// on_error Tests
// =============================================================================

/// on_error should NOT call handler on success.
#[rstest]
#[tokio::test]
async fn test_on_error_success_no_call() {
    let handler_called = Arc::new(AtomicBool::new(false));
    let handler_called_clone = handler_called.clone();

    let result: Result<i32, String> = AsyncIO::pure(Ok(42))
        .on_error(move |_error: &String| {
            let flag = handler_called_clone.clone();
            async move {
                flag.store(true, Ordering::SeqCst);
            }
        })
        .await;

    assert_eq!(result, Ok(42));
    assert!(
        !handler_called.load(Ordering::SeqCst),
        "handler should NOT be called on success"
    );
}

/// on_error should call handler on error.
#[rstest]
#[tokio::test]
async fn test_on_error_calls_handler() {
    let handler_called = Arc::new(AtomicBool::new(false));
    let handler_called_clone = handler_called.clone();
    let error_received = Arc::new(std::sync::Mutex::new(None));
    let error_received_clone = error_received.clone();

    let result: Result<i32, String> = AsyncIO::pure(Err("test error".to_string()))
        .on_error(move |error: &String| {
            let flag = handler_called_clone.clone();
            let err_store = error_received_clone.clone();
            let error_clone = error.clone();
            async move {
                flag.store(true, Ordering::SeqCst);
                *err_store.lock().unwrap() = Some(error_clone);
            }
        })
        .await;

    assert!(result.is_err());
    assert!(
        handler_called.load(Ordering::SeqCst),
        "handler should be called on error"
    );
    assert_eq!(
        *error_received.lock().unwrap(),
        Some("test error".to_string())
    );
}

/// on_error should propagate the original error after handler execution.
#[rstest]
#[tokio::test]
async fn test_on_error_propagates_error() {
    let result: Result<i32, &str> = AsyncIO::pure(Err("original error"))
        .on_error(|_error| async {
            // Handler does something but error should still propagate
        })
        .await;

    assert_eq!(result, Err("original error"));
}

/// on_error should be lazy - handler should not execute until awaited.
#[rstest]
#[tokio::test]
async fn test_on_error_is_lazy() {
    let handler_called = Arc::new(AtomicBool::new(false));
    let handler_called_clone = handler_called.clone();

    let _async_io: AsyncIO<Result<i32, String>> =
        AsyncIO::pure(Err("error".to_string())).on_error(move |_error: &String| {
            let flag = handler_called_clone.clone();
            async move {
                flag.store(true, Ordering::SeqCst);
            }
        });

    // Not yet awaited - handler should not have executed
    assert!(
        !handler_called.load(Ordering::SeqCst),
        "handler should not execute before await"
    );
}

// =============================================================================
// retry_with_factory Tests
// =============================================================================

/// retry_with_factory should succeed on first attempt.
#[rstest]
#[tokio::test]
async fn test_retry_success_first_attempt() {
    let attempt_count = Arc::new(AtomicUsize::new(0));
    let attempt_count_clone = attempt_count.clone();

    let result: Result<i32, &str> = AsyncIO::retry_with_factory(
        move || {
            let count = attempt_count_clone.clone();
            AsyncIO::new(move || {
                let count = count.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Ok(42)
                }
            })
        },
        3,
    )
    .await;

    assert_eq!(result, Ok(42));
    assert_eq!(
        attempt_count.load(Ordering::SeqCst),
        1,
        "should only attempt once on success"
    );
}

/// retry_with_factory should succeed after failures.
#[rstest]
#[tokio::test]
async fn test_retry_success_after_failures() {
    let attempt_count = Arc::new(AtomicUsize::new(0));
    let attempt_count_clone = attempt_count.clone();

    let result: Result<i32, &str> = AsyncIO::retry_with_factory(
        move || {
            let count = attempt_count_clone.clone();
            AsyncIO::new(move || {
                let count = count.clone();
                async move {
                    let current = count.fetch_add(1, Ordering::SeqCst);
                    if current < 2 { Err("fail") } else { Ok(42) }
                }
            })
        },
        5,
    )
    .await;

    assert_eq!(result, Ok(42));
    assert_eq!(
        attempt_count.load(Ordering::SeqCst),
        3,
        "should retry until success"
    );
}

/// retry_with_factory should return error after max attempts.
#[rstest]
#[tokio::test]
async fn test_retry_fails_after_max_attempts() {
    let attempt_count = Arc::new(AtomicUsize::new(0));
    let attempt_count_clone = attempt_count.clone();

    let result: Result<i32, &str> = AsyncIO::retry_with_factory(
        move || {
            let count = attempt_count_clone.clone();
            AsyncIO::new(move || {
                let count = count.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Err("always fail")
                }
            })
        },
        3,
    )
    .await;

    assert_eq!(result, Err("always fail"));
    assert_eq!(
        attempt_count.load(Ordering::SeqCst),
        3,
        "should attempt exactly max_attempts times"
    );
}

/// retry_with_factory should be lazy - factory should not execute until awaited.
#[rstest]
#[tokio::test]
async fn test_retry_is_lazy() {
    let factory_called = Arc::new(AtomicBool::new(false));
    let factory_called_clone = factory_called.clone();

    let _async_io: AsyncIO<Result<i32, &str>> = AsyncIO::retry_with_factory(
        move || {
            factory_called_clone.store(true, Ordering::SeqCst);
            AsyncIO::pure(Ok(42))
        },
        3,
    );

    // Not yet awaited - factory should not have executed
    assert!(
        !factory_called.load(Ordering::SeqCst),
        "factory should not execute before await"
    );
}

/// retry_with_factory with max_attempts=1 should not retry.
#[rstest]
#[tokio::test]
async fn test_retry_with_single_attempt() {
    let attempt_count = Arc::new(AtomicUsize::new(0));
    let attempt_count_clone = attempt_count.clone();

    let result: Result<i32, &str> = AsyncIO::retry_with_factory(
        move || {
            let count = attempt_count_clone.clone();
            AsyncIO::new(move || {
                let count = count.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Err("fail")
                }
            })
        },
        1,
    )
    .await;

    assert_eq!(result, Err("fail"));
    assert_eq!(
        attempt_count.load(Ordering::SeqCst),
        1,
        "should only attempt once"
    );
}

// =============================================================================
// Combined error handling tests
// =============================================================================

/// Combining on_error with retry should work correctly.
#[rstest]
#[tokio::test]
async fn test_retry_with_on_error() {
    let error_handler_count = Arc::new(AtomicUsize::new(0));
    let error_handler_count_clone = error_handler_count.clone();
    let attempt_count = Arc::new(AtomicUsize::new(0));
    let attempt_count_clone = attempt_count.clone();

    let result: Result<i32, &str> = AsyncIO::retry_with_factory(
        move || {
            let count = attempt_count_clone.clone();
            AsyncIO::new(move || {
                let count = count.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Err("fail")
                }
            })
        },
        2,
    )
    .on_error(move |_error| {
        let handler_count = error_handler_count_clone.clone();
        async move {
            handler_count.fetch_add(1, Ordering::SeqCst);
        }
    })
    .await;

    assert!(result.is_err());
    assert_eq!(attempt_count.load(Ordering::SeqCst), 2);
    // on_error is called once after all retries have been exhausted
    assert_eq!(error_handler_count.load(Ordering::SeqCst), 1);
}

/// Combining finally_async with retry should work correctly.
#[rstest]
#[tokio::test]
async fn test_retry_with_finally() {
    let cleanup_executed = Arc::new(AtomicBool::new(false));
    let cleanup_executed_clone = cleanup_executed.clone();
    let attempt_count = Arc::new(AtomicUsize::new(0));
    let attempt_count_clone = attempt_count.clone();

    let result: Result<i32, &str> = AsyncIO::retry_with_factory(
        move || {
            let count = attempt_count_clone.clone();
            AsyncIO::new(move || {
                let count = count.clone();
                async move {
                    let current = count.fetch_add(1, Ordering::SeqCst);
                    if current < 1 { Err("fail") } else { Ok(42) }
                }
            })
        },
        3,
    )
    .finally_async(move || {
        let flag = cleanup_executed_clone.clone();
        async move {
            flag.store(true, Ordering::SeqCst);
        }
    })
    .await;

    assert_eq!(result, Ok(42));
    assert!(cleanup_executed.load(Ordering::SeqCst));
    assert_eq!(attempt_count.load(Ordering::SeqCst), 2);
}

// =============================================================================
// finally_async Panic Handling Tests
// =============================================================================

/// finally_async should handle synchronous panic in cleanup closure
/// (panic before Future is returned).
#[rstest]
#[tokio::test]
async fn test_finally_async_synchronous_panic_in_closure() {
    // This tests the case where the cleanup closure panics BEFORE returning
    // the async block (synchronous panic in closure body).
    let result = AsyncIO::pure(42)
        .finally_async(|| {
            panic!("synchronous panic before returning Future");
            #[allow(unreachable_code)]
            async {}
        })
        .await;

    // Original result should be returned despite synchronous cleanup panic
    assert_eq!(result, 42);
}

/// finally_async should handle asynchronous panic in cleanup Future.
#[rstest]
#[tokio::test]
async fn test_finally_async_asynchronous_panic_in_future() {
    // This tests the case where the panic occurs during the async execution.
    let result = AsyncIO::pure(100)
        .finally_async(|| async {
            panic!("asynchronous panic during Future execution");
        })
        .await;

    // Original result should be returned despite asynchronous cleanup panic
    assert_eq!(result, 100);
}

/// finally_async should preserve Err result when synchronous panic occurs.
#[rstest]
#[tokio::test]
async fn test_finally_async_synchronous_panic_preserves_error() {
    let result: Result<i32, &str> = AsyncIO::pure(Err("original error"))
        .finally_async(|| {
            panic!("synchronous panic");
            #[allow(unreachable_code)]
            async {}
        })
        .await;

    assert_eq!(result, Err("original error"));
}

/// finally_async should handle panic with String message (not &str).
#[rstest]
#[tokio::test]
async fn test_finally_async_synchronous_panic_with_string_message() {
    let result = AsyncIO::pure(200)
        .finally_async(|| {
            panic!("{}", "synchronous panic with String".to_string());
            #[allow(unreachable_code)]
            async {}
        })
        .await;

    assert_eq!(result, 200);
}

/// finally_async should still execute cleanup normally when no panic occurs.
#[rstest]
#[tokio::test]
async fn test_finally_async_normal_execution_after_panic_handling_fix() {
    let cleanup_executed = Arc::new(AtomicBool::new(false));
    let cleanup_executed_clone = cleanup_executed.clone();

    let result = AsyncIO::pure(300)
        .finally_async(move || {
            let flag = cleanup_executed_clone.clone();
            async move {
                flag.store(true, Ordering::SeqCst);
            }
        })
        .await;

    assert_eq!(result, 300);
    assert!(
        cleanup_executed.load(Ordering::SeqCst),
        "cleanup should execute normally when no panic occurs"
    );
}
