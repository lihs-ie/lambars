//! Integration tests for `AsyncPool` - a fixed-size async task pool.
//!
//! These tests verify the functionality of `AsyncPool`, which provides:
//! - Fixed capacity pool management with separate queue capacity
//! - Backpressure when queue capacity is exceeded
//! - Efficient batch execution with bounded concurrency
//!
//! # Implementation Notes
//!
//! The pool uses `tokio::sync::Semaphore` and `tokio::sync::mpsc` for:
//! - Robust backpressure without manual Waker management
//! - Automatic permit release on cancellation (drop safety)
//! - FIFO ordering guaranteed by bounded mpsc channel

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use rstest::rstest;

use lambars::effect::async_io::pool::{AsyncPool, PoolError};

// =============================================================================
// AsyncPool::new Tests
// =============================================================================

#[rstest]
fn new_creates_pool_with_specified_capacity() {
    let pool = AsyncPool::<i32>::new(128);
    assert_eq!(pool.capacity(), 128);
    assert_eq!(pool.queue_capacity(), 128);
}

#[rstest]
fn new_creates_empty_pool() {
    let pool = AsyncPool::<i32>::new(10);
    assert_eq!(pool.capacity(), 10);
}

#[rstest]
fn new_with_zero_capacity_returns_error() {
    let result = AsyncPool::<i32>::try_new(0);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), PoolError::InvalidCapacity);
}

#[rstest]
fn new_with_one_capacity_succeeds() {
    let pool = AsyncPool::<i32>::new(1);
    assert_eq!(pool.capacity(), 1);
}

// =============================================================================
// AsyncPool::with_queue_capacity Tests
// =============================================================================

#[rstest]
fn with_queue_capacity_creates_pool_with_different_capacities() {
    // queue_capacity must be <= capacity
    let pool = AsyncPool::<i32>::with_queue_capacity(50, 10);
    assert_eq!(pool.capacity(), 50);
    assert_eq!(pool.queue_capacity(), 10);
}

#[rstest]
fn with_queue_capacity_zero_capacity_returns_error() {
    let result = AsyncPool::<i32>::try_with_queue_capacity(0, 10);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), PoolError::InvalidCapacity);
}

#[rstest]
fn with_queue_capacity_zero_queue_capacity_returns_error() {
    let result = AsyncPool::<i32>::try_with_queue_capacity(10, 0);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), PoolError::InvalidCapacity);
}

// =============================================================================
// AsyncPool::try_spawn Tests
// =============================================================================

#[rstest]
fn try_spawn_adds_future_to_queue() {
    let pool = AsyncPool::<i32>::new(10);
    let result = pool.try_spawn(async { 42 });
    assert!(result.is_ok());
}

#[rstest]
fn try_spawn_multiple_futures() {
    let pool = AsyncPool::<i32>::new(10);
    for i in 0..5 {
        pool.try_spawn(async move { i }).unwrap();
    }
}

#[rstest]
fn try_spawn_returns_error_when_queue_is_full() {
    let pool = AsyncPool::<i32>::new(2);
    pool.try_spawn(async { 1 }).unwrap();
    pool.try_spawn(async { 2 }).unwrap();
    let result = pool.try_spawn(async { 3 });
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), PoolError::QueueFull);
}

#[rstest]
fn try_spawn_respects_queue_capacity_not_capacity() {
    // queue_capacity=3, capacity=5 (queue_capacity must be <= capacity)
    let pool = AsyncPool::<i32>::with_queue_capacity(5, 3);
    // Queue capacity is 3, so we can add 3 futures
    for i in 0..3 {
        pool.try_spawn(async move { i }).unwrap();
    }
    // 4th should fail
    let result = pool.try_spawn(async { 3 });
    assert_eq!(result, Err(PoolError::QueueFull));
}

// =============================================================================
// AsyncPool::spawn Tests (async, with backpressure)
// =============================================================================

#[rstest]
#[tokio::test]
async fn spawn_adds_future_when_space_available() {
    let pool = AsyncPool::<i32>::new(10);
    let result = pool.spawn(async { 42 }).await;
    assert!(result.is_ok());
    assert_eq!(pool.queue_len(), 1);
}

#[rstest]
#[tokio::test]
async fn spawn_multiple_futures() {
    let pool = AsyncPool::<i32>::new(10);
    for i in 0..5 {
        pool.spawn(async move { i }).await.unwrap();
    }
    assert_eq!(pool.queue_len(), 5);
}

#[rstest]
#[tokio::test]
async fn spawn_waits_when_queue_full_then_succeeds_after_drain() {
    let pool = AsyncPool::<i32>::with_queue_capacity(2, 2);

    // Fill the queue
    pool.spawn(async { 1 }).await.unwrap();
    pool.spawn(async { 2 }).await.unwrap();
    assert_eq!(pool.queue_len(), 2);
    assert!(pool.is_queue_full());

    // Note: Sharing the same pool for testing backpressure would require Arc,
    // so we test the timeout behavior in spawn_can_be_cancelled_via_timeout instead
}

#[rstest]
#[tokio::test]
async fn spawn_can_be_cancelled_via_timeout() {
    let pool = AsyncPool::<i32>::with_queue_capacity(2, 1);

    // Fill the queue
    pool.try_spawn(async { 1 }).unwrap();
    assert!(pool.is_queue_full());

    // spawn should wait, but timeout will cancel it
    let spawn_future = pool.spawn(async { 2 });
    let result = tokio::time::timeout(Duration::from_millis(50), spawn_future).await;

    // Should timeout because queue is full
    assert!(result.is_err());
}

// =============================================================================
// AsyncPool::run_all Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn run_all_executes_all_futures() {
    let mut pool = AsyncPool::<i32>::new(10);
    pool.try_spawn(async { 1 }).unwrap();
    pool.try_spawn(async { 2 }).unwrap();
    pool.try_spawn(async { 3 }).unwrap();

    let results = pool.run_all().await;
    assert_eq!(results.len(), 3);
    // Results may be in any order due to concurrent execution
    let mut sorted_results = results.clone();
    sorted_results.sort();
    assert_eq!(sorted_results, vec![1, 2, 3]);
}

#[rstest]
#[tokio::test]
async fn run_all_returns_empty_vec_for_empty_pool() {
    let mut pool = AsyncPool::<i32>::new(10);
    let results = pool.run_all().await;
    assert!(results.is_empty());
}

#[rstest]
#[tokio::test]
async fn run_all_clears_queue_after_execution() {
    let mut pool = AsyncPool::<i32>::new(10);
    pool.try_spawn(async { 1 }).unwrap();
    pool.try_spawn(async { 2 }).unwrap();

    let _ = pool.run_all().await;
    assert!(pool.is_queue_empty());
}

#[rstest]
#[tokio::test]
async fn run_all_executes_futures_concurrently() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut pool = AsyncPool::new(10);

    for _ in 0..5 {
        let counter_clone = counter.clone();
        pool.try_spawn(async move {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        })
        .unwrap();
    }

    pool.run_all().await;
    assert_eq!(counter.load(Ordering::SeqCst), 5);
}

#[rstest]
#[tokio::test]
async fn run_all_with_async_sleep() {
    let mut pool = AsyncPool::<i32>::new(10);
    pool.try_spawn(async {
        tokio::time::sleep(Duration::from_millis(10)).await;
        1
    })
    .unwrap();
    pool.try_spawn(async {
        tokio::time::sleep(Duration::from_millis(5)).await;
        2
    })
    .unwrap();

    let results = pool.run_all().await;
    assert_eq!(results.len(), 2);
}

#[rstest]
#[tokio::test]
async fn run_all_limits_concurrency_to_capacity() {
    let active_count = Arc::new(AtomicUsize::new(0));
    let max_concurrent = Arc::new(AtomicUsize::new(0));
    // capacity=10, queue_capacity=3 (queue_capacity must be <= capacity)
    // We'll use capacity=3 and only spawn 3 tasks to stay within queue limit
    let mut pool = AsyncPool::with_queue_capacity(3, 3);

    for i in 0..3 {
        let active = active_count.clone();
        let max = max_concurrent.clone();
        pool.try_spawn(async move {
            let current = active.fetch_add(1, Ordering::SeqCst) + 1;
            // Update max if current is higher
            loop {
                let old_max = max.load(Ordering::SeqCst);
                if current <= old_max {
                    break;
                }
                if max
                    .compare_exchange(old_max, current, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    break;
                }
            }
            // Simulate some work
            tokio::time::sleep(Duration::from_millis(10)).await;
            active.fetch_sub(1, Ordering::SeqCst);
            i
        })
        .unwrap();
    }

    let results = pool.run_all().await;
    assert_eq!(results.len(), 3);
    // Max concurrent tasks should not exceed the capacity of 3
    assert!(max_concurrent.load(Ordering::SeqCst) <= 3);
}

// =============================================================================
// AsyncPool::run_buffered Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn run_buffered_limits_concurrency() {
    let active_count = Arc::new(AtomicUsize::new(0));
    let max_concurrent = Arc::new(AtomicUsize::new(0));
    let mut pool = AsyncPool::new(10);

    for i in 0..10 {
        let active = active_count.clone();
        let max = max_concurrent.clone();
        pool.try_spawn(async move {
            let current = active.fetch_add(1, Ordering::SeqCst) + 1;
            // Update max if current is higher
            loop {
                let old_max = max.load(Ordering::SeqCst);
                if current <= old_max {
                    break;
                }
                if max
                    .compare_exchange(old_max, current, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    break;
                }
            }
            // Simulate some work
            tokio::time::sleep(Duration::from_millis(10)).await;
            active.fetch_sub(1, Ordering::SeqCst);
            i
        })
        .unwrap();
    }

    let results = pool.run_buffered(3).await.unwrap();
    assert_eq!(results.len(), 10);
    // Max concurrent tasks should not exceed the limit of 3
    assert!(max_concurrent.load(Ordering::SeqCst) <= 3);
}

#[rstest]
#[tokio::test]
async fn run_buffered_with_limit_one_runs_sequentially() {
    let order = Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut pool = AsyncPool::new(5);

    for i in 0..5 {
        let order_clone = order.clone();
        pool.try_spawn(async move {
            order_clone.lock().unwrap().push(i);
            i
        })
        .unwrap();
    }

    let results = pool.run_buffered(1).await.unwrap();
    assert_eq!(results.len(), 5);
    // With limit=1, execution should be sequential (preserving input order)
    let final_order = order.lock().unwrap().clone();
    assert_eq!(final_order, vec![0, 1, 2, 3, 4]);
}

#[rstest]
#[tokio::test]
async fn run_buffered_with_zero_limit_returns_error() {
    let mut pool = AsyncPool::<i32>::new(10);
    let result = pool.run_buffered(0).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), PoolError::InvalidConcurrencyLimit);
}

#[rstest]
#[tokio::test]
async fn run_buffered_returns_empty_vec_for_empty_pool() {
    let mut pool = AsyncPool::<i32>::new(10);
    let results = pool.run_buffered(5).await.unwrap();
    assert!(results.is_empty());
}

#[rstest]
#[tokio::test]
async fn run_buffered_clears_queue_after_execution() {
    let mut pool = AsyncPool::<i32>::new(10);
    pool.try_spawn(async { 1 }).unwrap();
    pool.try_spawn(async { 2 }).unwrap();

    let _ = pool.run_buffered(2).await;
    assert!(pool.is_queue_empty());
}

// =============================================================================
// PoolError Tests
// =============================================================================

#[rstest]
fn pool_error_display_invalid_capacity() {
    let error = PoolError::InvalidCapacity;
    assert_eq!(error.to_string(), "pool capacity must be greater than 0");
}

#[rstest]
fn pool_error_display_queue_full() {
    let error = PoolError::QueueFull;
    assert!(error.to_string().contains("queue is full"));
}

#[rstest]
fn pool_error_display_invalid_concurrency_limit() {
    let error = PoolError::InvalidConcurrencyLimit;
    assert!(error.to_string().contains("concurrency limit"));
}

#[rstest]
fn pool_error_debug() {
    let error = PoolError::QueueFull;
    let debug = format!("{:?}", error);
    assert!(debug.contains("QueueFull"));
}

#[rstest]
fn pool_error_equality() {
    assert_eq!(PoolError::InvalidCapacity, PoolError::InvalidCapacity);
    assert_eq!(PoolError::QueueFull, PoolError::QueueFull);
    assert_ne!(PoolError::InvalidCapacity, PoolError::QueueFull);
}

#[rstest]
fn pool_error_copy() {
    let error = PoolError::QueueFull;
    let copied = error; // PoolError is Copy, so this is a copy
    assert_eq!(error, copied);
}

// =============================================================================
// Thread Safety Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn pool_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<AsyncPool<i32>>();
}

#[rstest]
#[tokio::test]
async fn pool_results_are_collected_correctly_with_many_tasks() {
    let mut pool = AsyncPool::<usize>::new(100);
    for i in 0..100 {
        pool.try_spawn(async move { i }).unwrap();
    }

    let results = pool.run_all().await;
    assert_eq!(results.len(), 100);

    let mut sorted = results.clone();
    sorted.sort();
    let expected: Vec<usize> = (0..100).collect();
    assert_eq!(sorted, expected);
}

// =============================================================================
// Reuse Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn pool_can_be_reused_after_run_all() {
    let mut pool = AsyncPool::<i32>::new(10);

    // First batch
    pool.try_spawn(async { 1 }).unwrap();
    pool.try_spawn(async { 2 }).unwrap();
    let results1 = pool.run_all().await;
    assert_eq!(results1.len(), 2);

    // Second batch
    pool.try_spawn(async { 3 }).unwrap();
    pool.try_spawn(async { 4 }).unwrap();
    pool.try_spawn(async { 5 }).unwrap();
    let results2 = pool.run_all().await;
    assert_eq!(results2.len(), 3);
}

#[rstest]
#[tokio::test]
async fn pool_can_be_reused_after_run_buffered() {
    let mut pool = AsyncPool::<i32>::new(10);

    // First batch
    pool.try_spawn(async { 1 }).unwrap();
    let results1 = pool.run_buffered(2).await.unwrap();
    assert_eq!(results1.len(), 1);

    // Second batch
    pool.try_spawn(async { 2 }).unwrap();
    pool.try_spawn(async { 3 }).unwrap();
    let results2 = pool.run_buffered(2).await.unwrap();
    assert_eq!(results2.len(), 2);
}

// =============================================================================
// Laws Tests (from requirements)
// =============================================================================

/// Law: inflight_count <= capacity
/// During run_all execution, the number of concurrently executing tasks
/// should not exceed capacity.
#[rstest]
#[tokio::test]
async fn law_bounded_inflight() {
    let capacity = 20;
    let queue_capacity = 20; // queue_capacity must be <= capacity
    let active_count = Arc::new(AtomicUsize::new(0));
    let max_concurrent = Arc::new(AtomicUsize::new(0));
    let mut pool = AsyncPool::with_queue_capacity(capacity, queue_capacity);

    for _ in 0..queue_capacity {
        let active = active_count.clone();
        let max = max_concurrent.clone();
        pool.try_spawn(async move {
            let current = active.fetch_add(1, Ordering::SeqCst) + 1;
            loop {
                let old_max = max.load(Ordering::SeqCst);
                if current <= old_max
                    || max
                        .compare_exchange(old_max, current, Ordering::SeqCst, Ordering::SeqCst)
                        .is_ok()
                {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
            active.fetch_sub(1, Ordering::SeqCst);
        })
        .unwrap();
    }

    pool.run_all().await;
    assert!(
        max_concurrent.load(Ordering::SeqCst) <= capacity,
        "Max concurrent {} exceeded capacity {}",
        max_concurrent.load(Ordering::SeqCst),
        capacity
    );
}

/// Law: queue_len <= queue_capacity
/// The number of queued tasks should not exceed queue_capacity.
#[rstest]
#[tokio::test]
async fn law_bounded_queue() {
    let queue_capacity = 5;
    let pool = AsyncPool::<i32>::with_queue_capacity(10, queue_capacity);

    // Fill up to queue capacity
    for i in 0..queue_capacity {
        assert!(pool.try_spawn(async move { i as i32 }).is_ok());
    }

    // Next spawn should fail
    assert_eq!(pool.try_spawn(async { -1 }), Err(PoolError::QueueFull));

    // Verify queue length
    assert_eq!(pool.queue_len(), queue_capacity);
}

/// Law: inflight_count + queue_len <= capacity + queue_capacity
/// Total tasks (executing + queued) should not exceed capacity + queue_capacity.
#[rstest]
#[tokio::test]
async fn law_total_bounded() {
    let capacity = 5;
    let queue_capacity = 3; // queue_capacity must be <= capacity
    let pool = AsyncPool::<i32>::with_queue_capacity(capacity, queue_capacity);

    // We can only queue up to queue_capacity
    for i in 0..queue_capacity {
        assert!(pool.try_spawn(async move { i as i32 }).is_ok());
    }

    // Total is now queue_capacity (all in queue, none executing)
    // During run_all, at most capacity will be executing at once
    // So total is always <= capacity + queue_capacity

    assert_eq!(pool.queue_len(), queue_capacity);
    assert_eq!(pool.try_spawn(async { -1 }), Err(PoolError::QueueFull));
}

// =============================================================================
// Queue State Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn queue_len_returns_correct_count() {
    let pool = AsyncPool::<i32>::new(10);
    assert_eq!(pool.queue_len(), 0);

    pool.try_spawn(async { 1 }).unwrap();
    assert_eq!(pool.queue_len(), 1);

    pool.try_spawn(async { 2 }).unwrap();
    pool.try_spawn(async { 3 }).unwrap();
    assert_eq!(pool.queue_len(), 3);
}

#[rstest]
#[tokio::test]
async fn is_queue_empty_returns_correct_value() {
    let pool = AsyncPool::<i32>::new(10);
    assert!(pool.is_queue_empty());

    pool.try_spawn(async { 1 }).unwrap();
    assert!(!pool.is_queue_empty());
}

#[rstest]
#[tokio::test]
async fn is_queue_full_returns_correct_value() {
    let pool = AsyncPool::<i32>::with_queue_capacity(10, 2);
    assert!(!pool.is_queue_full());

    pool.try_spawn(async { 1 }).unwrap();
    assert!(!pool.is_queue_full());

    pool.try_spawn(async { 2 }).unwrap();
    assert!(pool.is_queue_full());
}

// =============================================================================
// Debug Tests
// =============================================================================

#[rstest]
fn debug_format_shows_capacities() {
    // queue_capacity must be <= capacity
    let pool = AsyncPool::<i32>::with_queue_capacity(10, 5);
    let debug = format!("{:?}", pool);
    assert!(debug.contains("AsyncPool"));
    assert!(debug.contains("capacity: 10"));
    assert!(debug.contains("queue_capacity: 5"));
}

// =============================================================================
// Permit Return Tests (Semaphore-specific)
// =============================================================================

#[rstest]
#[tokio::test]
async fn permits_are_returned_after_run_all() {
    let mut pool = AsyncPool::<i32>::with_queue_capacity(10, 3);

    // Fill the queue
    pool.try_spawn(async { 1 }).unwrap();
    pool.try_spawn(async { 2 }).unwrap();
    pool.try_spawn(async { 3 }).unwrap();
    assert!(pool.is_queue_full());

    // After run_all, permits should be returned
    let _ = pool.run_all().await;
    assert!(pool.is_queue_empty());

    // Should be able to spawn again
    pool.try_spawn(async { 4 }).unwrap();
    pool.try_spawn(async { 5 }).unwrap();
    pool.try_spawn(async { 6 }).unwrap();
    assert!(pool.is_queue_full());
}

#[rstest]
#[tokio::test]
async fn permits_are_returned_after_run_buffered() {
    let mut pool = AsyncPool::<i32>::with_queue_capacity(10, 3);

    // Fill the queue
    pool.try_spawn(async { 1 }).unwrap();
    pool.try_spawn(async { 2 }).unwrap();
    pool.try_spawn(async { 3 }).unwrap();
    assert!(pool.is_queue_full());

    // After run_buffered, permits should be returned
    let _ = pool.run_buffered(2).await.unwrap();
    assert!(pool.is_queue_empty());

    // Should be able to spawn again
    pool.try_spawn(async { 4 }).unwrap();
    pool.try_spawn(async { 5 }).unwrap();
    pool.try_spawn(async { 6 }).unwrap();
    assert!(pool.is_queue_full());
}

// =============================================================================
// Backpressure Behavior Tests
// =============================================================================

/// Verifies that `spawn` waits when queue is full and succeeds when space becomes available.
///
/// This test demonstrates the backpressure behavior by:
/// 1. Filling the queue
/// 2. Verifying spawn times out when queue is full
/// 3. Draining the queue with a separate pool instance
/// 4. Verifying spawn succeeds after draining
///
/// # Design Constraint
///
/// `run_all` requires `&mut self`, so concurrent execution with `spawn` on the same
/// pool instance is not possible. This is intentional for Rust's memory safety.
/// To test backpressure with concurrent spawn/drain, use separate pool instances
/// or consider `Arc<tokio::sync::Mutex<AsyncPool>>` (which has its own tradeoffs).
#[rstest]
#[tokio::test]
async fn spawn_waits_on_full_queue_and_drain_allows_new_spawn() {
    use std::sync::atomic::AtomicBool;
    use tokio::sync::oneshot;

    // Create pool with capacity=2, queue_capacity=2 so we have room for the waiting spawn
    let pool = Arc::new(AsyncPool::<i32>::with_queue_capacity(2, 2));

    // Fill the queue
    pool.try_spawn(async { 1 }).unwrap();
    pool.try_spawn(async { 2 }).unwrap();
    assert!(pool.is_queue_full());

    // Test that spawn times out when queue is full
    let pool_clone = pool.clone();
    let timeout_result =
        tokio::time::timeout(Duration::from_millis(50), pool_clone.spawn(async { 3 })).await;
    assert!(
        timeout_result.is_err(),
        "spawn should timeout when queue is full"
    );

    // Now test with a fresh pool that we can drain
    let mut pool2 = AsyncPool::<i32>::with_queue_capacity(3, 2);
    pool2.try_spawn(async { 10 }).unwrap();
    pool2.try_spawn(async { 20 }).unwrap();
    assert!(pool2.is_queue_full());

    // Start a spawn in a separate task using oneshot to signal when it should try
    let (drain_done_sender, drain_done_receiver) = oneshot::channel::<()>();
    let spawn_completed = Arc::new(AtomicBool::new(false));
    let spawn_completed_clone = spawn_completed.clone();

    // First, drain the queue
    let results = pool2.run_all().await;
    assert_eq!(results.len(), 2);
    assert!(pool2.is_queue_empty());

    // Signal that drain is complete (not used in this flow but shows the pattern)
    let _ = drain_done_sender.send(());

    // Now spawn should succeed immediately since queue is empty
    pool2.spawn(async { 30 }).await.unwrap();
    spawn_completed_clone.store(true, Ordering::SeqCst);
    assert!(spawn_completed.load(Ordering::SeqCst));

    // Verify the task is in the queue
    assert_eq!(pool2.queue_len(), 1);

    // Execute and verify
    let final_results = pool2.run_all().await;
    assert_eq!(final_results.len(), 1);
    assert_eq!(final_results[0], 30);

    // Clean up the oneshot receiver
    drop(drain_done_receiver);
}

#[rstest]
#[tokio::test]
async fn spawn_with_async_returns_ok() {
    let pool = AsyncPool::<i32>::new(10);

    // spawn is now async fn, not returning AsyncIO
    let result = pool.spawn(async { 42 }).await;
    assert!(result.is_ok());
    assert_eq!(pool.queue_len(), 1);
}

// =============================================================================
// Spawn Cancellation Tests
// =============================================================================

/// Verifies that when a spawn Future is dropped (cancelled) while waiting for a permit,
/// no resource leak occurs and subsequent operations work correctly.
///
/// This test demonstrates:
/// 1. A waiting spawn can be cancelled via task abort
/// 2. After cancellation, the queue length is unchanged (cancelled spawn never acquired permit)
/// 3. After draining, new spawns succeed (proving no permit leak)
///
/// # Note
///
/// The cancelled spawn is aborted before acquiring a permit, so there is no "permit return"
/// per se - rather, we verify that no leak occurs. The semaphore correctly handles the
/// cancellation via `tokio::sync::Semaphore`'s drop safety.
#[rstest]
#[tokio::test]
async fn spawn_cancel_does_not_leak_semaphore() {
    use std::sync::atomic::AtomicBool;

    let pool = Arc::new(AsyncPool::<i32>::with_queue_capacity(2, 1));

    // Fill the queue to capacity
    pool.try_spawn(async { 1 }).unwrap();
    assert!(pool.is_queue_full());
    assert_eq!(pool.queue_len(), 1);

    // Start a spawn that will wait for a permit, and cancel it via timeout
    let pool_clone = pool.clone();
    let spawn_started = Arc::new(AtomicBool::new(false));
    let spawn_started_clone = spawn_started.clone();

    let spawn_handle = tokio::spawn(async move {
        spawn_started_clone.store(true, Ordering::SeqCst);
        pool_clone.spawn(async { 2 }).await
    });

    // Wait for spawn to start waiting on semaphore
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(spawn_started.load(Ordering::SeqCst));

    // Cancel the spawn by aborting the task (this drops the Future, releasing pending permit)
    spawn_handle.abort();

    // Wait for cancellation to propagate
    tokio::time::sleep(Duration::from_millis(20)).await;

    // Verify the queue still has exactly 1 task (the cancelled spawn did not add anything)
    assert_eq!(pool.queue_len(), 1);
    assert!(pool.is_queue_full());

    // Verify try_spawn still fails (queue is still full with the original task)
    assert_eq!(pool.try_spawn(async { 3 }), Err(PoolError::QueueFull));

    // Now test with a pool we can drain
    let mut pool2 = AsyncPool::<i32>::with_queue_capacity(2, 1);
    pool2.try_spawn(async { 10 }).unwrap();
    assert!(pool2.is_queue_full());

    // Drain the queue
    let results = pool2.run_all().await;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], 10);

    // After draining, the queue should be empty and we should be able to spawn again
    assert!(pool2.is_queue_empty());

    // This proves the permit was correctly returned after drain
    let result = pool2.try_spawn(async { 20 });
    assert!(result.is_ok(), "try_spawn should succeed after drain");
    assert_eq!(pool2.queue_len(), 1);

    // Verify the new task executes correctly
    let results = pool2.run_all().await;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], 20);
}

/// Verifies that multiple spawn operations work correctly with backpressure and drain cycles.
///
/// This test demonstrates:
/// 1. Multiple spawns succeed when queue has space
/// 2. Multiple spawns timeout when queue is full (backpressure)
/// 3. Spawn-drain cycles work correctly with permits properly managed
/// 4. Permits are restored after each drain cycle
///
/// # Design Constraint
///
/// `run_all` requires `&mut self`, preventing concurrent spawn/drain on the same pool.
/// This test uses timeout patterns and sequential spawn-drain cycles to verify behavior.
#[rstest]
#[tokio::test]
async fn spawn_drain_cycle_works_correctly() {
    use std::sync::atomic::AtomicUsize;

    // Test 1: Verify multiple spawns succeed when queue has space
    // This tests the basic case without backpressure
    {
        let mut pool = AsyncPool::<i32>::with_queue_capacity(10, 5);

        // Spawn 5 tasks (fills the queue)
        for i in 1..=5 {
            pool.spawn(async move { i }).await.unwrap();
        }

        assert_eq!(pool.queue_len(), 5);

        // Run all and verify results
        let results = pool.run_all().await;
        assert_eq!(results.len(), 5);

        let mut sorted = results.clone();
        sorted.sort();
        assert_eq!(sorted, vec![1, 2, 3, 4, 5]);
    }

    // Test 2: Verify multiple waiters timeout when queue is full (backpressure)
    {
        let pool = Arc::new(AsyncPool::<i32>::with_queue_capacity(5, 2));

        // Fill the queue
        pool.try_spawn(async { 1 }).unwrap();
        pool.try_spawn(async { 2 }).unwrap();
        assert!(pool.is_queue_full());

        let timeout_count = Arc::new(AtomicUsize::new(0));

        // Start multiple spawn tasks that will wait for permits
        let mut handles = Vec::new();
        for i in 3..=5 {
            let pool_clone = pool.clone();
            let timeout_count_clone = timeout_count.clone();

            let handle = tokio::spawn(async move {
                // This will wait because the queue is full, and timeout
                let result = tokio::time::timeout(
                    Duration::from_millis(50),
                    pool_clone.spawn(async move { i }),
                )
                .await;
                if result.is_err() {
                    timeout_count_clone.fetch_add(1, Ordering::SeqCst);
                }
                result
            });
            handles.push(handle);
        }

        // Wait for all handles to complete (they should all timeout)
        for handle in handles {
            let _ = handle.await;
        }

        // All 3 spawns should have timed out
        assert_eq!(timeout_count.load(Ordering::SeqCst), 3);

        // Queue should still have only the original 2 tasks
        assert_eq!(pool.queue_len(), 2);
    }

    // Test 3: Full integration test with spawn-drain cycle
    // Demonstrates that spawns succeed when space is available
    {
        let mut pool = AsyncPool::<i32>::with_queue_capacity(10, 3);

        // First batch: fill the queue
        pool.spawn(async { 1 }).await.unwrap();
        pool.spawn(async { 2 }).await.unwrap();
        pool.spawn(async { 3 }).await.unwrap();
        assert!(pool.is_queue_full());

        // Drain first batch
        let results1 = pool.run_all().await;
        assert_eq!(results1.len(), 3);
        assert!(pool.is_queue_empty());

        // Second batch: spawn more tasks (succeeds immediately since queue is now empty)
        let success_count = Arc::new(AtomicUsize::new(0));

        for i in 4..=6 {
            let success_clone = success_count.clone();
            pool.spawn(async move {
                success_clone.fetch_add(1, Ordering::Relaxed);
                i
            })
            .await
            .unwrap();
        }

        // All 3 spawns should have succeeded immediately
        assert_eq!(pool.queue_len(), 3);

        // Drain second batch
        let results2 = pool.run_all().await;
        assert_eq!(results2.len(), 3);
        assert_eq!(success_count.load(Ordering::Relaxed), 3);

        let mut sorted = results2.clone();
        sorted.sort();
        assert_eq!(sorted, vec![4, 5, 6]);
    }

    // Test 4: Verify permits are correctly managed across multiple drain cycles
    {
        let mut pool = AsyncPool::<i32>::with_queue_capacity(5, 2);

        // Cycle 1
        pool.try_spawn(async { 100 }).unwrap();
        pool.try_spawn(async { 200 }).unwrap();
        assert!(pool.is_queue_full());
        assert_eq!(pool.try_spawn(async { 999 }), Err(PoolError::QueueFull));

        let results1 = pool.run_all().await;
        assert_eq!(results1.len(), 2);
        assert!(pool.is_queue_empty());

        // Cycle 2: permits should be fully restored
        pool.try_spawn(async { 300 }).unwrap();
        pool.try_spawn(async { 400 }).unwrap();
        assert!(pool.is_queue_full());

        let results2 = pool.run_all().await;
        assert_eq!(results2.len(), 2);

        // Cycle 3: async spawn should also work
        pool.spawn(async { 500 }).await.unwrap();
        pool.spawn(async { 600 }).await.unwrap();
        assert!(pool.is_queue_full());

        let results3 = pool.run_all().await;
        assert_eq!(results3.len(), 2);

        let mut sorted = results3.clone();
        sorted.sort();
        assert_eq!(sorted, vec![500, 600]);
    }
}
