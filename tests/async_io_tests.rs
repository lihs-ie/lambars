#![cfg(feature = "async")]
#![allow(deprecated)]
//! Unit tests for AsyncIO monad.
//!
//! This module tests the AsyncIO type that represents deferred asynchronous
//! side effects. Tests cover:
//! - Basic construction and execution
//! - Lazy evaluation verification
//! - Functor operations (fmap)
//! - Applicative operations (apply, map2, product)
//! - Monad operations (flat_map, and_then, then)
//! - IO <-> AsyncIO conversion
//! - Utility methods (delay_async, timeout, race, catch_async)

use lambars::effect::AsyncIO;
use rstest::rstest;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

// =============================================================================
// Basic Construction and Execution Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_async_io_pure_creates_value() {
    // AsyncIO::pure wraps a pure value
    let async_io = AsyncIO::pure(42);
    let result = async_io.await;
    assert_eq!(result, 42);
}

#[rstest]
#[tokio::test]
async fn test_async_io_pure_with_string() {
    // Verify it works with String type
    let async_io = AsyncIO::pure("hello".to_string());
    let result = async_io.await;
    assert_eq!(result, "hello");
}

#[rstest]
#[tokio::test]
async fn test_async_io_pure_with_struct() {
    // Verify it works with a struct type
    #[derive(Debug, Clone, PartialEq)]
    struct TestData {
        value: i32,
        name: String,
    }

    let data = TestData {
        value: 42,
        name: "test".to_string(),
    };

    let async_io = AsyncIO::pure(data.clone());
    let result = async_io.await;
    assert_eq!(result, data);
}

#[rstest]
#[tokio::test]
async fn test_async_io_new_with_async_closure() {
    // AsyncIO::new accepts an async closure
    let async_io = AsyncIO::new(|| async { 10 + 20 });
    let result = async_io.await;
    assert_eq!(result, 30);
}

#[rstest]
#[tokio::test]
async fn test_async_io_new_with_delay() {
    // Verify that an actual async operation (delay) works
    let async_io = AsyncIO::new(|| async {
        tokio::time::sleep(Duration::from_millis(10)).await;
        "delayed"
    });
    let result = async_io.await;
    assert_eq!(result, "delayed");
}

#[rstest]
#[tokio::test]
async fn test_async_io_from_future_basic() {
    // Create AsyncIO from an existing Future
    let future = async { 100 };
    let async_io = AsyncIO::from_future(future);
    let result = async_io.await;
    assert_eq!(result, 100);
}

#[rstest]
#[tokio::test]
async fn test_async_io_impl_future_can_be_awaited() {
    // Can be directly awaited via impl Future
    let async_io = AsyncIO::pure(42);
    let result = async_io.await;
    assert_eq!(result, 42);
}

// =============================================================================
// Lazy Evaluation Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_async_io_new_is_lazy() {
    // AsyncIO::new does not produce side effects until execution
    let executed = Arc::new(AtomicBool::new(false));
    let executed_clone = executed.clone();

    let async_io = AsyncIO::new(move || {
        let flag = executed_clone.clone();
        async move {
            flag.store(true, Ordering::SeqCst);
            42
        }
    });

    // Not yet executed at this point
    assert!(!executed.load(Ordering::SeqCst));

    // Execute via await
    let result = async_io.await;
    assert!(executed.load(Ordering::SeqCst));
    assert_eq!(result, 42);
}

#[rstest]
#[tokio::test]
async fn test_async_io_side_effect_not_executed_on_creation() {
    // No side effects occur at creation time
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    let _async_io = AsyncIO::new(move || {
        let cnt = counter_clone.clone();
        async move {
            cnt.fetch_add(1, Ordering::SeqCst);
        }
    });

    // Creating AsyncIO alone does not trigger side effects
    assert_eq!(counter.load(Ordering::SeqCst), 0);
}

#[rstest]
#[tokio::test]
async fn test_async_io_fmap_is_lazy() {
    // fmap is also lazily evaluated
    let executed = Arc::new(AtomicBool::new(false));
    let executed_clone = executed.clone();

    let async_io = AsyncIO::new(move || {
        let flag = executed_clone.clone();
        async move {
            flag.store(true, Ordering::SeqCst);
            42
        }
    });

    let mapped = async_io.fmap(|x| x * 2);

    // fmap alone does not trigger execution
    assert!(!executed.load(Ordering::SeqCst));

    let result = mapped.await;
    assert!(executed.load(Ordering::SeqCst));
    assert_eq!(result, 84);
}

#[rstest]
#[tokio::test]
async fn test_async_io_flat_map_is_lazy() {
    // flat_map is also lazily evaluated
    let executed = Arc::new(AtomicBool::new(false));
    let executed_clone = executed.clone();

    let async_io = AsyncIO::new(move || {
        let flag = executed_clone.clone();
        async move {
            flag.store(true, Ordering::SeqCst);
            42
        }
    });

    let chained = async_io.flat_map(|x| AsyncIO::pure(x * 2));

    // flat_map alone does not trigger execution
    assert!(!executed.load(Ordering::SeqCst));

    let result = chained.await;
    assert!(executed.load(Ordering::SeqCst));
    assert_eq!(result, 84);
}

// =============================================================================
// Functor Tests (fmap)
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_async_io_fmap_basic() {
    // Basic fmap operation
    let async_io = AsyncIO::pure(21).fmap(|x| x * 2);
    let result = async_io.await;
    assert_eq!(result, 42);
}

#[rstest]
#[tokio::test]
async fn test_async_io_fmap_chain() {
    // Chaining fmap
    let async_io = AsyncIO::pure(2)
        .fmap(|x| x * 3) // 6
        .fmap(|x| x + 4) // 10
        .fmap(|x| x * 5); // 50
    let result = async_io.await;
    assert_eq!(result, 50);
}

#[rstest]
#[tokio::test]
async fn test_async_io_fmap_type_change() {
    // Type conversion via fmap
    let async_io = AsyncIO::pure(42).fmap(|x| format!("value: {}", x));
    let result = async_io.await;
    assert_eq!(result, "value: 42");
}

#[rstest]
#[tokio::test]
async fn test_async_io_fmap_identity() {
    // fmap with identity function does not change the value
    let async_io = AsyncIO::pure(42).fmap(|x| x);
    let result = async_io.await;
    assert_eq!(result, 42);
}

// =============================================================================
// Applicative Tests (apply, map2, product)
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_async_io_map2_basic() {
    // Combine two AsyncIO values
    let io1 = AsyncIO::pure(10);
    let io2 = AsyncIO::pure(20);
    let combined = io1.map2(io2, |a, b| a + b);
    let result = combined.await;
    assert_eq!(result, 30);
}

#[rstest]
#[tokio::test]
async fn test_async_io_map2_with_different_types() {
    // Combine AsyncIO values of different types
    let io1 = AsyncIO::pure(42);
    let io2 = AsyncIO::pure("hello".to_string());
    let combined = io1.map2(io2, |n, s| format!("{}: {}", s, n));
    let result = combined.await;
    assert_eq!(result, "hello: 42");
}

#[rstest]
#[tokio::test]
async fn test_async_io_product_basic() {
    // product returns a tuple
    let io1 = AsyncIO::pure(10);
    let io2 = AsyncIO::pure(20);
    let result = io1.product(io2).await;
    assert_eq!(result, (10, 20));
}

#[rstest]
#[tokio::test]
async fn test_async_io_product_tuple_type() {
    // Verify product type
    let io1 = AsyncIO::pure(1);
    let io2 = AsyncIO::pure("hello");
    let result = io1.product(io2).await;
    assert_eq!(result, (1, "hello"));
}

#[rstest]
#[tokio::test]
async fn test_async_io_apply_basic() {
    // Basic apply operation
    let function_io = AsyncIO::pure(|x: i32| x * 2);
    let value_io = AsyncIO::pure(21);
    let result = value_io.apply(function_io).await;
    assert_eq!(result, 42);
}

// =============================================================================
// Monad Tests (flat_map, and_then, then)
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_async_io_flat_map_basic() {
    // Basic flat_map operation
    let async_io = AsyncIO::pure(10).flat_map(|x| AsyncIO::pure(x * 2));
    let result = async_io.await;
    assert_eq!(result, 20);
}

#[rstest]
#[tokio::test]
async fn test_async_io_flat_map_chain() {
    // Chaining flat_map
    let async_io = AsyncIO::pure(1)
        .flat_map(|x| AsyncIO::pure(x + 1)) // 2
        .flat_map(|x| AsyncIO::pure(x * 3)) // 6
        .flat_map(|x| AsyncIO::pure(x + 4)); // 10
    let result = async_io.await;
    assert_eq!(result, 10);
}

#[rstest]
#[tokio::test]
async fn test_async_io_flat_map_with_effect() {
    // flat_map chain with side effects
    let counter = Arc::new(AtomicUsize::new(0));
    let counter1 = counter.clone();
    let counter2 = counter.clone();

    let async_io = AsyncIO::new(move || {
        let cnt = counter1.clone();
        async move {
            cnt.fetch_add(1, Ordering::SeqCst);
            10
        }
    })
    .flat_map(move |x| {
        let cnt = counter2.clone();
        AsyncIO::new(move || {
            let cnt_inner = cnt.clone();
            async move {
                cnt_inner.fetch_add(1, Ordering::SeqCst);
                x * 2
            }
        })
    });

    let result = async_io.await;
    assert_eq!(result, 20);
    assert_eq!(counter.load(Ordering::SeqCst), 2);
}

#[rstest]
#[tokio::test]
async fn test_async_io_and_then_is_flat_map_alias() {
    // and_then is an alias for flat_map
    let async_io1 = AsyncIO::pure(10).flat_map(|x| AsyncIO::pure(x + 5));
    let async_io2 = AsyncIO::pure(10).and_then(|x| AsyncIO::pure(x + 5));

    let result1 = async_io1.await;
    let result2 = async_io2.await;
    assert_eq!(result1, result2);
}

#[rstest]
#[tokio::test]
async fn test_async_io_then_discards_first() {
    // then discards the first result
    let async_io = AsyncIO::pure(10).then(AsyncIO::pure(20));
    let result = async_io.await;
    assert_eq!(result, 20);
}

#[rstest]
#[tokio::test]
async fn test_async_io_then_executes_first_for_side_effect() {
    // then executes the first AsyncIO for its side effect
    let executed = Arc::new(AtomicBool::new(false));
    let executed_clone = executed.clone();

    let async_io = AsyncIO::new(move || {
        let flag = executed_clone.clone();
        async move {
            flag.store(true, Ordering::SeqCst);
            "side effect"
        }
    })
    .then(AsyncIO::pure(42));

    let result = async_io.await;
    assert_eq!(result, 42);
    assert!(executed.load(Ordering::SeqCst));
}

// =============================================================================
// IO <-> AsyncIO Conversion Tests
// =============================================================================

#[cfg(feature = "async")]
mod conversion_tests {
    use super::*;
    use lambars::effect::IO;

    #[rstest]
    #[tokio::test]
    async fn test_io_to_async_basic() {
        // Convert IO to AsyncIO
        let io = IO::pure(42);
        let async_io = io.to_async();
        let result = async_io.await;
        assert_eq!(result, 42);
    }

    #[rstest]
    #[tokio::test]
    async fn test_io_to_async_preserves_value() {
        // Value is preserved after conversion
        let io = IO::new(|| "hello".to_string());
        let async_io = io.to_async();
        let result = async_io.await;
        assert_eq!(result, "hello");
    }

    #[rstest]
    #[tokio::test]
    async fn test_io_to_async_executes_immediately() {
        // IO::to_async executes the IO immediately (because IO is not Send)
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let io = IO::new(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            42
        });

        // IO is executed when to_async is called
        let async_io = io.to_async();
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // await simply returns the result
        let result = async_io.await;
        assert_eq!(result, 42);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[rstest]
    #[allow(deprecated)]
    fn test_async_io_to_sync_basic() {
        // Convert AsyncIO to IO (executed in a synchronous context)
        let async_io = AsyncIO::pure(42);
        let io = async_io.to_sync();
        let result = io.run_unsafe();
        assert_eq!(result, 42);
    }

    #[rstest]
    #[allow(deprecated)]
    fn test_async_io_to_sync_preserves_value() {
        // Value is preserved after conversion
        let async_io = AsyncIO::pure("hello".to_string());
        let io = async_io.to_sync();
        let result = io.run_unsafe();
        assert_eq!(result, "hello");
    }

    #[rstest]
    #[allow(deprecated)]
    fn test_io_to_async_to_sync_roundtrip() {
        // IO -> AsyncIO -> IO round-trip
        let original = 42;
        let io = IO::pure(original);
        let async_io = io.to_async();
        let io_back = async_io.to_sync();
        let result = io_back.run_unsafe();
        assert_eq!(result, original);
    }

    #[rstest]
    #[allow(deprecated)]
    fn test_async_io_to_sync_is_lazy() {
        // IO converted via AsyncIO::to_sync maintains lazy evaluation
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let async_io = AsyncIO::new(move || {
            let cnt = counter_clone.clone();
            async move {
                cnt.fetch_add(1, Ordering::SeqCst);
                42
            }
        });

        // Not executed when to_sync is called
        let io = async_io.to_sync();
        assert_eq!(counter.load(Ordering::SeqCst), 0);

        // Executed via run_unsafe
        let result = io.run_unsafe();
        assert_eq!(result, 42);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}

// =============================================================================
// Utility Method Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_async_io_delay_async_waits() {
    // delay_async waits for the specified duration
    let start = std::time::Instant::now();
    let async_io = AsyncIO::delay_async(Duration::from_millis(50));
    async_io.await;
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(45)); // Allow some tolerance
}

#[rstest]
#[tokio::test]
async fn test_async_io_delay_async_is_lazy() {
    // delay_async is also lazily evaluated
    let start = std::time::Instant::now();
    let _async_io = AsyncIO::delay_async(Duration::from_millis(100));
    // No time passes by just creating it
    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_millis(50));
}

#[rstest]
#[tokio::test]
async fn test_async_io_timeout_completes_in_time() {
    // When completed before timeout
    let async_io = AsyncIO::pure(42).timeout(Duration::from_millis(100));
    let result = async_io.await;
    assert_eq!(result, Some(42));
}

#[rstest]
#[tokio::test]
async fn test_async_io_timeout_returns_none_on_timeout() {
    // Returns None on timeout
    let async_io =
        AsyncIO::delay_async(Duration::from_millis(200)).timeout(Duration::from_millis(50));
    let result = async_io.await;
    assert_eq!(result, None);
}

#[rstest]
#[tokio::test]
async fn test_async_io_race_returns_first_completed() {
    use lambars::control::Either;

    // race returns the first one to complete
    let slow = AsyncIO::delay_async(Duration::from_millis(100)).fmap(|_| "slow");
    let fast = AsyncIO::pure("fast");

    let result = slow.race(fast).await;
    assert!(matches!(result, Either::Right("fast")));
}

#[rstest]
#[tokio::test]
async fn test_async_io_race_with_immediate_value() {
    use lambars::control::Either;

    // When both complete immediately
    let io1 = AsyncIO::pure(1);
    let io2 = AsyncIO::pure(2);

    let result = io1.race(io2).await;
    // Both complete immediately, so either one is returned
    assert!(matches!(result, Either::Left(1) | Either::Right(2)));
}

#[rstest]
#[tokio::test]
async fn test_async_io_catch_async_on_success() {
    // Returns Ok on success
    let async_io = AsyncIO::pure(42).catch_async(|_| "error".to_string());
    let result = async_io.await;
    assert_eq!(result, Ok(42));
}

#[rstest]
#[tokio::test]
async fn test_async_io_catch_async_recovers_panic() {
    // Catches a panic and converts it to Err
    let async_io = AsyncIO::new(|| async {
        panic!("test panic");
        #[allow(unreachable_code)]
        42
    })
    .catch_async(|_| "caught panic".to_string());

    let result = async_io.await;
    assert_eq!(result, Err("caught panic".to_string()));
}

// =============================================================================
// Execution Order Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_async_io_execution_order() {
    // Execution order test
    let order = Arc::new(std::sync::Mutex::new(Vec::new()));

    let order1 = order.clone();
    let order2 = order.clone();
    let order3 = order.clone();

    let async_io = AsyncIO::new(move || {
        let o = order1.clone();
        async move {
            o.lock().unwrap().push(1);
            10
        }
    })
    .flat_map(move |x| {
        let o = order2.clone();
        AsyncIO::new(move || {
            let o_inner = o.clone();
            async move {
                o_inner.lock().unwrap().push(2);
                x + 10
            }
        })
    })
    .fmap(move |x| {
        order3.lock().unwrap().push(3);
        x + 10
    });

    let result = async_io.await;
    assert_eq!(result, 30);

    let execution_order = order.lock().unwrap().clone();
    assert_eq!(execution_order, vec![1, 2, 3]);
}
