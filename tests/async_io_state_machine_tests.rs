#![cfg(feature = "async")]
//! AsyncIO State Machine Tests - Phase 2: impl Future
//!
//! These tests verify that AsyncIO can be directly awaited via impl Future.

use lambars::effect::AsyncIO;
use rstest::rstest;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

// =============================================================================
// Direct await tests (impl Future)
// =============================================================================

/// Pure values should return immediately when awaited directly.
#[rstest]
#[tokio::test]
async fn test_pure_returns_immediately() {
    let async_io = AsyncIO::pure(42);

    // Direct await
    let result = async_io.await;
    assert_eq!(result, 42);
}

/// Pure values with string type should work correctly.
#[rstest]
#[tokio::test]
async fn test_pure_with_string_returns_immediately() {
    let async_io = AsyncIO::pure("hello".to_string());

    let result = async_io.await;
    assert_eq!(result, "hello");
}

/// Deferred computations should execute on first poll.
#[rstest]
#[tokio::test]
async fn test_defer_executes_on_first_poll() {
    let executed = Arc::new(AtomicBool::new(false));
    let executed_clone = executed.clone();

    let async_io = AsyncIO::new(move || {
        let flag = executed_clone.clone();
        async move {
            flag.store(true, Ordering::SeqCst);
            42
        }
    });

    // Not executed before awaiting
    assert!(!executed.load(Ordering::SeqCst));

    // Direct await triggers execution
    let result = async_io.await;
    assert!(executed.load(Ordering::SeqCst));
    assert_eq!(result, 42);
}

/// fmap on pure should be immediate when awaited.
#[rstest]
#[tokio::test]
async fn test_fmap_on_pure_is_immediate() {
    let async_io = AsyncIO::pure(21).fmap(|x| x * 2);

    let result = async_io.await;
    assert_eq!(result, 42);
}

/// fmap chain should work correctly with direct await.
#[rstest]
#[tokio::test]
async fn test_fmap_chain_with_direct_await() {
    let async_io = AsyncIO::pure(2)
        .fmap(|x| x * 3) // 6
        .fmap(|x| x + 4) // 10
        .fmap(|x| x * 5); // 50

    let result = async_io.await;
    assert_eq!(result, 50);
}

/// flat_map should maintain laziness until awaited.
#[rstest]
#[tokio::test]
async fn test_flat_map_maintains_laziness() {
    let executed_outer = Arc::new(AtomicBool::new(false));
    let executed_inner = Arc::new(AtomicBool::new(false));
    let executed_outer_clone = executed_outer.clone();
    let executed_inner_clone = executed_inner.clone();

    let async_io = AsyncIO::new(move || {
        let flag = executed_outer_clone.clone();
        async move {
            flag.store(true, Ordering::SeqCst);
            10
        }
    })
    .flat_map(move |x| {
        let flag = executed_inner_clone.clone();
        AsyncIO::new(move || async move {
            flag.store(true, Ordering::SeqCst);
            x * 2
        })
    });

    // Nothing executed yet
    assert!(!executed_outer.load(Ordering::SeqCst));
    assert!(!executed_inner.load(Ordering::SeqCst));

    // Direct await executes both
    let result = async_io.await;
    assert!(executed_outer.load(Ordering::SeqCst));
    assert!(executed_inner.load(Ordering::SeqCst));
    assert_eq!(result, 20);
}

/// Direct await produces correct results.
#[rstest]
#[tokio::test]
async fn test_impl_future_direct_await() {
    // Test that direct await produces correct results
    let async_io = AsyncIO::pure(100);
    let result = async_io.await;
    assert_eq!(result, 100);

    // With computation
    let async_io = AsyncIO::new(|| async { 10 + 20 });
    let result = async_io.await;
    assert_eq!(result, 30);

    // With fmap
    let async_io = AsyncIO::pure(5).fmap(|x| x * x);
    let result = async_io.await;
    assert_eq!(result, 25);

    // With flat_map
    let async_io = AsyncIO::pure(7).flat_map(|x| AsyncIO::pure(x + 3));
    let result = async_io.await;
    assert_eq!(result, 10);
}

/// Direct await works with various operations.
#[rstest]
#[tokio::test]
async fn test_direct_await_various_operations() {
    // Pure value
    let async_io = AsyncIO::pure(42);
    let result = async_io.await;
    assert_eq!(result, 42);

    // With computation
    let async_io = AsyncIO::new(|| async { 5 * 5 });
    let result = async_io.await;
    assert_eq!(result, 25);

    // With fmap
    let async_io = AsyncIO::pure(10).fmap(|x| x + 5);
    let result = async_io.await;
    assert_eq!(result, 15);

    // With flat_map
    let async_io = AsyncIO::pure(3).flat_map(|x| AsyncIO::pure(x * 4));
    let result = async_io.await;
    assert_eq!(result, 12);
}

// =============================================================================
// State transition tests
// =============================================================================

/// Pure state should complete immediately without state transition.
#[rstest]
#[tokio::test]
async fn test_pure_state_completes_immediately() {
    let async_io = AsyncIO::pure(42);
    let result = async_io.await;
    assert_eq!(result, 42);
}

/// Deferred state should transition from thunk to running to completed.
#[rstest]
#[tokio::test]
async fn test_defer_state_transitions() {
    let poll_count = Arc::new(AtomicUsize::new(0));
    let poll_count_clone = poll_count.clone();

    let async_io = AsyncIO::new(move || {
        let counter = poll_count_clone.clone();
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            // Yield to ensure multiple polls might be needed
            tokio::task::yield_now().await;
            42
        }
    });

    let result = async_io.await;
    assert_eq!(result, 42);
    // The async closure should only be called once
    assert_eq!(poll_count.load(Ordering::SeqCst), 1);
}

/// from_future should work correctly with direct await.
#[rstest]
#[tokio::test]
async fn test_from_future_with_direct_await() {
    let future = async { 100 };
    let async_io = AsyncIO::from_future(future);
    let result = async_io.await;
    assert_eq!(result, 100);
}

// =============================================================================
// Applicative operations tests
// =============================================================================

/// apply should work correctly with direct await.
#[rstest]
#[tokio::test]
async fn test_apply_with_direct_await() {
    let function_io = AsyncIO::pure(|x: i32| x * 2);
    let value_io = AsyncIO::pure(21);
    let result = value_io.apply(function_io).await;
    assert_eq!(result, 42);
}

/// map2 should work correctly with direct await.
#[rstest]
#[tokio::test]
async fn test_map2_with_direct_await() {
    let io1 = AsyncIO::pure(10);
    let io2 = AsyncIO::pure(20);
    let combined = io1.map2(io2, |a, b| a + b);
    let result = combined.await;
    assert_eq!(result, 30);
}

/// product should work correctly with direct await.
#[rstest]
#[tokio::test]
async fn test_product_with_direct_await() {
    let io1 = AsyncIO::pure(10);
    let io2 = AsyncIO::pure(20);
    let result = io1.product(io2).await;
    assert_eq!(result, (10, 20));
}

// =============================================================================
// Monad operations tests
// =============================================================================

/// and_then should work correctly with direct await.
#[rstest]
#[tokio::test]
async fn test_and_then_with_direct_await() {
    let async_io = AsyncIO::pure(10).and_then(|x| AsyncIO::pure(x + 5));
    let result = async_io.await;
    assert_eq!(result, 15);
}

/// then should work correctly with direct await.
#[rstest]
#[tokio::test]
async fn test_then_with_direct_await() {
    let async_io = AsyncIO::pure(10).then(AsyncIO::pure(20));
    let result = async_io.await;
    assert_eq!(result, 20);
}

/// Chained flat_map should work correctly with direct await.
#[rstest]
#[tokio::test]
async fn test_flat_map_chain_with_direct_await() {
    let async_io = AsyncIO::pure(1)
        .flat_map(|x| AsyncIO::pure(x + 1)) // 2
        .flat_map(|x| AsyncIO::pure(x * 3)) // 6
        .flat_map(|x| AsyncIO::pure(x + 4)); // 10

    let result = async_io.await;
    assert_eq!(result, 10);
}

// =============================================================================
// Utility method tests with direct await
// =============================================================================

/// delay_async should work with direct await.
#[rstest]
#[tokio::test]
async fn test_delay_async_with_direct_await() {
    let start = std::time::Instant::now();
    let async_io = AsyncIO::delay_async(Duration::from_millis(50));
    async_io.await;
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(45));
}

/// timeout should work with direct await.
#[rstest]
#[tokio::test]
async fn test_timeout_with_direct_await() {
    let async_io = AsyncIO::pure(42).timeout(Duration::from_millis(100));
    let result = async_io.await;
    assert_eq!(result, Some(42));
}

/// timeout_result should work with direct await.
#[rstest]
#[tokio::test]
async fn test_timeout_result_with_direct_await() {
    let async_io = AsyncIO::pure(42).timeout_result(Duration::from_millis(100));
    let result = async_io.await;
    assert_eq!(result, Ok(42));
}

/// par should work with direct await.
#[rstest]
#[tokio::test]
async fn test_par_with_direct_await() {
    let first = AsyncIO::pure(1);
    let second = AsyncIO::pure(2);
    let result = first.par(second).await;
    assert_eq!(result, (1, 2));
}

/// par3 should work with direct await.
#[rstest]
#[tokio::test]
async fn test_par3_with_direct_await() {
    let first = AsyncIO::pure(1);
    let second = AsyncIO::pure(2);
    let third = AsyncIO::pure(3);
    let result = first.par3(second, third).await;
    assert_eq!(result, (1, 2, 3));
}

/// race should work with direct await.
#[rstest]
#[tokio::test]
async fn test_race_with_direct_await() {
    use lambars::control::Either;

    let slow = AsyncIO::delay_async(Duration::from_millis(100)).fmap(|_| "slow");
    let fast = AsyncIO::pure("fast");

    let result = slow.race(fast).await;
    assert!(matches!(result, Either::Right("fast")));
}

/// race_result should work with direct await.
#[rstest]
#[tokio::test]
async fn test_race_result_with_direct_await() {
    let slow = AsyncIO::delay_async(Duration::from_millis(100)).fmap(|_| 1);
    let fast = AsyncIO::pure(2);

    let result = slow.race_result(fast).await;
    assert_eq!(result, 2);
}

/// bracket should work with direct await.
#[rstest]
#[tokio::test]
async fn test_bracket_with_direct_await() {
    let released = Arc::new(AtomicBool::new(false));
    let released_clone = released.clone();

    let result = AsyncIO::bracket(
        || AsyncIO::pure(42),
        |value| AsyncIO::pure(value * 2),
        move |_| {
            AsyncIO::new(move || async move {
                released_clone.store(true, Ordering::SeqCst);
            })
        },
    )
    .await;

    assert_eq!(result, 84);
    assert!(released.load(Ordering::SeqCst));
}

/// finally_async should work with direct await.
#[rstest]
#[tokio::test]
async fn test_finally_async_with_direct_await() {
    let executed = Arc::new(AtomicBool::new(false));
    let executed_clone = executed.clone();

    let result = AsyncIO::pure(42)
        .finally_async(move || async move {
            executed_clone.store(true, Ordering::SeqCst);
        })
        .await;

    assert_eq!(result, 42);
    assert!(executed.load(Ordering::SeqCst));
}

/// on_error should work with direct await.
#[rstest]
#[tokio::test]
async fn test_on_error_with_direct_await() {
    let called = Arc::new(AtomicBool::new(false));
    let called_clone = called.clone();

    let action: AsyncIO<Result<i32, String>> = AsyncIO::pure(Err("error".to_string()));
    let result = action
        .on_error(move |_| async move {
            called_clone.store(true, Ordering::SeqCst);
        })
        .await;

    assert_eq!(result, Err("error".to_string()));
    assert!(called.load(Ordering::SeqCst));
}

/// catch_async should work with direct await.
#[rstest]
#[tokio::test]
async fn test_catch_async_with_direct_await() {
    let async_io = AsyncIO::pure(42).catch_async(|_| "error".to_string());
    let result = async_io.await;
    assert_eq!(result, Ok(42));
}

/// retry_with_factory should work with direct await.
#[rstest]
#[tokio::test]
async fn test_retry_with_factory_with_direct_await() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    let result = AsyncIO::retry_with_factory(
        move || {
            let counter = counter_clone.clone();
            AsyncIO::new(move || async move {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                if count < 2 { Err("temporary") } else { Ok(42) }
            })
        },
        5,
    )
    .await;

    assert_eq!(result, Ok(42));
    assert_eq!(counter.load(Ordering::SeqCst), 3);
}

// =============================================================================
// Monad law tests with direct await
// =============================================================================

/// Left identity: pure(a).flat_map(f) == f(a)
#[rstest]
#[tokio::test]
async fn test_monad_left_identity_with_direct_await() {
    let value = 5;
    let f = |x: i32| AsyncIO::pure(x * 2);

    let left = AsyncIO::pure(value).flat_map(f).await;
    let right = f(value).await;

    assert_eq!(left, right);
}

/// Right identity: m.flat_map(pure) == m
#[rstest]
#[tokio::test]
async fn test_monad_right_identity_with_direct_await() {
    let async_io = AsyncIO::pure(42);
    let result = async_io.flat_map(AsyncIO::pure).await;
    assert_eq!(result, 42);
}

/// Associativity: m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
#[rstest]
#[tokio::test]
async fn test_monad_associativity_with_direct_await() {
    let f = |x: i32| AsyncIO::pure(x + 1);
    let g = |x: i32| AsyncIO::pure(x * 2);

    let async_io1 = AsyncIO::pure(5);
    let async_io2 = AsyncIO::pure(5);

    let left = async_io1.flat_map(f).flat_map(g).await;
    let right = async_io2.flat_map(move |x| f(x).flat_map(g)).await;

    assert_eq!(left, right);
}

// =============================================================================
// Functor law tests with direct await
// =============================================================================

/// Identity: fmap(|x| x) should not change the value
#[rstest]
#[tokio::test]
async fn test_functor_identity_with_direct_await() {
    let async_io = AsyncIO::pure(42);
    let result = async_io.fmap(|x| x).await;
    assert_eq!(result, 42);
}

/// Composition: fmap(f).fmap(g) == fmap(|x| g(f(x)))
#[rstest]
#[tokio::test]
async fn test_functor_composition_with_direct_await() {
    let f = |x: i32| x + 1;
    let g = |x: i32| x * 2;

    let async_io1 = AsyncIO::pure(5);
    let async_io2 = AsyncIO::pure(5);

    let left = async_io1.fmap(f).fmap(g).await;
    let right = async_io2.fmap(move |x| g(f(x))).await;

    assert_eq!(left, right);
}

// =============================================================================
// impl Future tests (direct await support)
// =============================================================================

/// AsyncIO implements Future directly and can be awaited.
#[rstest]
#[tokio::test]
async fn test_impl_future_with_direct_await() {
    let async_io = AsyncIO::pure(42);
    // AsyncIO implements Future directly via pin_project_lite
    let result = async_io.await;
    assert_eq!(result, 42);
}

// =============================================================================
// Complex scenario tests
// =============================================================================

/// Complex chaining should work with direct await.
#[rstest]
#[tokio::test]
async fn test_complex_chaining_with_direct_await() {
    let result = AsyncIO::pure(5)
        .fmap(|x| x + 1) // 6
        .flat_map(|x| AsyncIO::pure(x * 2)) // 12
        .fmap(|x| x.to_string()) // "12"
        .await;

    assert_eq!(result, "12");
}

/// Multiple AsyncIO operations in sequence should work.
#[rstest]
#[tokio::test]
async fn test_multiple_operations_in_sequence() {
    let a = AsyncIO::pure(10).await;
    let b = AsyncIO::pure(20).await;
    let c = AsyncIO::pure(a + b).await;
    assert_eq!(c, 30);
}

/// Nested AsyncIO should work correctly.
#[rstest]
#[tokio::test]
async fn test_nested_async_io() {
    let inner = AsyncIO::pure(42);
    let outer: AsyncIO<AsyncIO<i32>> = AsyncIO::pure(inner);

    let inner_result = outer.await;
    let final_result = inner_result.await;

    assert_eq!(final_result, 42);
}

// =============================================================================
// Execution order tests
// =============================================================================

/// Execution order should be preserved with direct await.
#[rstest]
#[tokio::test]
async fn test_execution_order_with_direct_await() {
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
        AsyncIO::new(move || async move {
            o.lock().unwrap().push(2);
            x + 10
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

// =============================================================================
// FlatMap state machine tests (REQ-ASYNC-FUT-001)
// =============================================================================

/// FlatMap should use enum state instead of Box<dyn Future>.
/// This test verifies that flat_map chains work correctly with state transitions.
#[rstest]
#[tokio::test]
async fn test_flatmap_state_pure_to_pure() {
    // Pure -> FlatMap -> Pure transition
    let async_io = AsyncIO::pure(10).flat_map(|x| AsyncIO::pure(x * 2));
    let result = async_io.await;
    assert_eq!(result, 20);
}

/// Multiple flat_map calls should chain correctly via state transitions.
#[rstest]
#[tokio::test]
async fn test_flatmap_state_chain() {
    let async_io = AsyncIO::pure(1)
        .flat_map(|x| AsyncIO::pure(x + 1)) // 2
        .flat_map(|x| AsyncIO::pure(x * 2)) // 4
        .flat_map(|x| AsyncIO::pure(x + 3)); // 7

    let result = async_io.await;
    assert_eq!(result, 7);
}

/// FlatMap with deferred computation should work correctly.
#[rstest]
#[tokio::test]
async fn test_flatmap_state_with_defer() {
    let executed = Arc::new(AtomicBool::new(false));
    let executed_clone = executed.clone();

    let async_io = AsyncIO::pure(5).flat_map(move |x| {
        let flag = executed_clone.clone();
        AsyncIO::new(move || async move {
            flag.store(true, Ordering::SeqCst);
            x * 3
        })
    });

    // Not executed yet
    assert!(!executed.load(Ordering::SeqCst));

    let result = async_io.await;
    assert!(executed.load(Ordering::SeqCst));
    assert_eq!(result, 15);
}

/// FlatMap should preserve laziness until polled.
#[rstest]
#[tokio::test]
async fn test_flatmap_state_preserves_laziness() {
    let first_executed = Arc::new(AtomicBool::new(false));
    let second_executed = Arc::new(AtomicBool::new(false));
    let first_clone = first_executed.clone();
    let second_clone = second_executed.clone();

    let async_io = AsyncIO::new(move || {
        let flag = first_clone.clone();
        async move {
            flag.store(true, Ordering::SeqCst);
            10
        }
    })
    .flat_map(move |x| {
        let flag = second_clone.clone();
        AsyncIO::new(move || async move {
            flag.store(true, Ordering::SeqCst);
            x + 5
        })
    });

    // Neither should be executed yet
    assert!(!first_executed.load(Ordering::SeqCst));
    assert!(!second_executed.load(Ordering::SeqCst));

    let result = async_io.await;

    // Both should be executed after await
    assert!(first_executed.load(Ordering::SeqCst));
    assert!(second_executed.load(Ordering::SeqCst));
    assert_eq!(result, 15);
}

/// FlatMap followed by fmap should work correctly.
#[rstest]
#[tokio::test]
async fn test_flatmap_state_followed_by_fmap() {
    let async_io = AsyncIO::pure(7)
        .flat_map(|x| AsyncIO::pure(x * 2)) // 14
        .fmap(|x| x + 1); // 15

    let result = async_io.await;
    assert_eq!(result, 15);
}

/// fmap followed by flat_map should work correctly.
#[rstest]
#[tokio::test]
async fn test_fmap_followed_by_flatmap_state() {
    let async_io = AsyncIO::pure(3)
        .fmap(|x| x * 3) // 9
        .flat_map(|x| AsyncIO::pure(x + 1)); // 10

    let result = async_io.await;
    assert_eq!(result, 10);
}

/// Deep flat_map chain should not overflow stack (tests tail-call-like behavior).
#[rstest]
#[tokio::test]
async fn test_flatmap_state_deep_chain() {
    let mut async_io = AsyncIO::pure(0);
    for _ in 0..100 {
        async_io = async_io.flat_map(|x| AsyncIO::pure(x + 1));
    }
    let result = async_io.await;
    assert_eq!(result, 100);
}

/// FlatMap with type transformation should work correctly.
#[rstest]
#[tokio::test]
async fn test_flatmap_state_type_transformation() {
    let async_io: AsyncIO<String> = AsyncIO::pure(42)
        .flat_map(|x| AsyncIO::pure(x.to_string()))
        .flat_map(|s| AsyncIO::pure(format!("Value: {}", s)));

    let result = async_io.await;
    assert_eq!(result, "Value: 42");
}

// =============================================================================
// Pure Chain Optimization Tests (REQ-ASYNC-FUT-001)
// =============================================================================

/// Pure chains should be evaluated immediately without deferred state.
/// This test verifies that Pure -> Pure chains don't create FlatMap states.
#[rstest]
#[tokio::test]
async fn test_pure_chain_immediate_evaluation() {
    // This chain should be evaluated immediately during construction,
    // not deferred to poll time.
    let execution_order = Arc::new(std::sync::Mutex::new(Vec::new()));

    let order1 = execution_order.clone();
    let order2 = execution_order.clone();

    // Pure chain: all operations should execute during construction
    let _async_io = AsyncIO::pure(1)
        .fmap(move |x| {
            order1.lock().unwrap().push("fmap1");
            x + 1
        })
        .fmap(move |x| {
            order2.lock().unwrap().push("fmap2");
            x * 2
        });

    // Since these are pure operations, they should have already executed
    let order = execution_order.lock().unwrap().clone();
    assert_eq!(
        order,
        vec!["fmap1", "fmap2"],
        "Pure fmap chain should execute immediately"
    );
}

/// Pure flat_map chain should also be evaluated immediately.
#[rstest]
#[tokio::test]
async fn test_pure_flatmap_chain_immediate_evaluation() {
    let execution_order = Arc::new(std::sync::Mutex::new(Vec::new()));

    let order1 = execution_order.clone();
    let order2 = execution_order.clone();

    // Pure flat_map chain: continuations should execute during construction
    let async_io = AsyncIO::pure(1)
        .flat_map(move |x| {
            order1.lock().unwrap().push("flat_map1");
            AsyncIO::pure(x + 1)
        })
        .flat_map(move |x| {
            order2.lock().unwrap().push("flat_map2");
            AsyncIO::pure(x * 2)
        });

    // Since source is Pure, flat_map continuations should have already executed
    let order = execution_order.lock().unwrap().clone();
    assert_eq!(
        order,
        vec!["flat_map1", "flat_map2"],
        "Pure flat_map chain should execute immediately"
    );

    // Final result should be correct
    let result = async_io.await;
    assert_eq!(result, 4);
}

/// Deferred source should delay flat_map continuation execution.
#[rstest]
#[tokio::test]
async fn test_deferred_flatmap_delays_continuation() {
    let continuation_executed = Arc::new(AtomicBool::new(false));
    let continuation_clone = continuation_executed.clone();

    // Deferred source: flat_map continuation should NOT execute during construction
    let async_io = AsyncIO::new(|| async { 10 }).flat_map(move |x| {
        continuation_clone.store(true, Ordering::SeqCst);
        AsyncIO::pure(x * 2)
    });

    // Continuation should NOT have executed yet (deferred)
    assert!(
        !continuation_executed.load(Ordering::SeqCst),
        "Deferred flat_map continuation should not execute during construction"
    );

    // Execute by awaiting
    let result = async_io.await;
    assert_eq!(result, 20);

    // Now continuation should have executed
    assert!(
        continuation_executed.load(Ordering::SeqCst),
        "Deferred flat_map continuation should execute after await"
    );
}

/// Mixed chain: Pure followed by deferred should work correctly.
#[rstest]
#[tokio::test]
async fn test_mixed_pure_and_deferred_chain() {
    let pure_executed = Arc::new(AtomicBool::new(false));
    let deferred_executed = Arc::new(AtomicBool::new(false));
    let pure_clone = pure_executed.clone();
    let deferred_clone = deferred_executed.clone();

    // Start with Pure (immediate)
    let async_io = AsyncIO::pure(5).flat_map(move |x| {
        pure_clone.store(true, Ordering::SeqCst);
        // Return a deferred AsyncIO
        AsyncIO::new(move || async move {
            deferred_clone.store(true, Ordering::SeqCst);
            x * 3
        })
    });

    // Pure part should have executed (source is Pure)
    assert!(
        pure_executed.load(Ordering::SeqCst),
        "Pure continuation should execute immediately"
    );

    // Deferred part should NOT have executed yet
    assert!(
        !deferred_executed.load(Ordering::SeqCst),
        "Deferred inner AsyncIO should not execute during construction"
    );

    // Execute by awaiting
    let result = async_io.await;
    assert_eq!(result, 15);

    // Now deferred should have executed
    assert!(
        deferred_executed.load(Ordering::SeqCst),
        "Deferred inner AsyncIO should execute after await"
    );
}

/// Long Pure chain should not cause stack overflow.
#[rstest]
#[tokio::test]
async fn test_long_pure_chain_no_overflow() {
    // Build a long chain of pure fmap operations
    let mut async_io = AsyncIO::pure(0i64);
    for i in 0..1000 {
        async_io = async_io.fmap(move |x| x + i);
    }

    // Expected: 0 + 0 + 1 + 2 + ... + 999 = 999 * 1000 / 2 = 499500
    let result = async_io.await;
    assert_eq!(result, 499500);
}

/// Long Pure flat_map chain should not cause stack overflow.
#[rstest]
#[tokio::test]
async fn test_long_pure_flatmap_chain_no_overflow() {
    // Build a long chain of pure flat_map operations
    let mut async_io = AsyncIO::pure(0i64);
    for i in 0..1000 {
        async_io = async_io.flat_map(move |x| AsyncIO::pure(x + i));
    }

    // Expected: 0 + 0 + 1 + 2 + ... + 999 = 999 * 1000 / 2 = 499500
    let result = async_io.await;
    assert_eq!(result, 499500);
}
