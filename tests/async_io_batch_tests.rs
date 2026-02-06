#![cfg(feature = "async")]
//! Integration tests for AsyncIO batch execution API.
//!
//! This module tests the batch_run and batch_run_buffered functions that
//! enable efficient parallel execution of multiple AsyncIO operations
//! with minimal runtime Enter/Drop overhead.

use lambars::effect::{AsyncIO, BatchError};
use rstest::rstest;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

// =============================================================================
// batch_run Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_batch_run_empty() {
    let items: Vec<AsyncIO<i32>> = vec![];
    let results = AsyncIO::batch_run(items).await;
    assert!(results.is_empty());
}

#[rstest]
#[tokio::test]
async fn test_batch_run_single() {
    let items = vec![AsyncIO::pure(42)];
    let results = AsyncIO::batch_run(items).await;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], 42);
}

#[rstest]
#[tokio::test]
async fn test_batch_run_multiple() {
    let items = vec![
        AsyncIO::pure(1),
        AsyncIO::pure(2),
        AsyncIO::pure(3),
        AsyncIO::pure(4),
        AsyncIO::pure(5),
    ];
    let mut results = AsyncIO::batch_run(items).await;
    results.sort();
    assert_eq!(results, vec![1, 2, 3, 4, 5]);
}

#[rstest]
#[tokio::test]
async fn test_batch_run_parallel_execution() {
    // Verify parallel execution: each task takes 50ms, so sequential execution
    // would take 250ms+, but parallel should complete in ~50ms.
    let start = Instant::now();

    let items: Vec<AsyncIO<i32>> = (0..5)
        .map(|index| {
            AsyncIO::new(move || async move {
                tokio::time::sleep(Duration::from_millis(50)).await;
                index
            })
        })
        .collect();

    let results = AsyncIO::batch_run(items).await;
    let elapsed = start.elapsed();

    assert_eq!(results.len(), 5);

    // Parallel execution should complete in <150ms (with margin);
    // sequential would take 250ms+.
    assert!(
        elapsed < Duration::from_millis(150),
        "Expected parallel execution to complete in <150ms, but took {:?}",
        elapsed
    );
}

#[rstest]
#[tokio::test]
async fn test_batch_run_with_deferred_computation() {
    // Verify that deferred computations are actually executed
    let counter = Arc::new(AtomicUsize::new(0));

    let items: Vec<AsyncIO<usize>> = (0..3)
        .map(|index| {
            let counter = Arc::clone(&counter);
            AsyncIO::new(move || {
                let counter = Arc::clone(&counter);
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    index
                }
            })
        })
        .collect();

    // Not yet executed before batch_run
    assert_eq!(counter.load(Ordering::SeqCst), 0);

    let results = AsyncIO::batch_run(items).await;

    // All executed after batch_run
    assert_eq!(counter.load(Ordering::SeqCst), 3);
    assert_eq!(results.len(), 3);
}

#[rstest]
#[tokio::test]
async fn test_batch_run_preserves_all_values() {
    // Verify correct behavior with different types (strings)
    let items = vec![
        AsyncIO::pure("hello".to_string()),
        AsyncIO::pure("world".to_string()),
        AsyncIO::pure("batch".to_string()),
    ];

    let results = AsyncIO::batch_run(items).await;
    assert_eq!(results.len(), 3);
    assert!(results.contains(&"hello".to_string()));
    assert!(results.contains(&"world".to_string()));
    assert!(results.contains(&"batch".to_string()));
}

// =============================================================================
// batch_run_buffered Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_batch_run_buffered_empty() {
    let items: Vec<AsyncIO<i32>> = vec![];
    let results = AsyncIO::batch_run_buffered(items, 2).await.unwrap();
    assert!(results.is_empty());
}

#[rstest]
#[tokio::test]
async fn test_batch_run_buffered_single() {
    let items = vec![AsyncIO::pure(42)];
    let results = AsyncIO::batch_run_buffered(items, 2).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], 42);
}

#[rstest]
#[tokio::test]
async fn test_batch_run_buffered_respects_limit() {
    // Verify concurrency does not exceed limit.
    // Run 5 tasks with limit=2; each task increments concurrent_count
    // on start and decrements on finish.
    let concurrent_count = Arc::new(AtomicUsize::new(0));
    let max_concurrent = Arc::new(AtomicUsize::new(0));

    let items: Vec<AsyncIO<i32>> = (0..5)
        .map(|index| {
            let concurrent_count = Arc::clone(&concurrent_count);
            let max_concurrent = Arc::clone(&max_concurrent);
            AsyncIO::new(move || {
                let concurrent_count = Arc::clone(&concurrent_count);
                let max_concurrent = Arc::clone(&max_concurrent);
                async move {
                    let current = concurrent_count.fetch_add(1, Ordering::SeqCst) + 1;
                    max_concurrent.fetch_max(current, Ordering::SeqCst);

                    // Sleep to allow concurrent execution overlap
                    tokio::time::sleep(Duration::from_millis(30)).await;

                    concurrent_count.fetch_sub(1, Ordering::SeqCst);

                    index
                }
            })
        })
        .collect();

    let results = AsyncIO::batch_run_buffered(items, 2).await.unwrap();

    assert_eq!(results.len(), 5);

    // Max concurrency should not exceed the limit
    let observed_max = max_concurrent.load(Ordering::SeqCst);
    assert!(
        observed_max <= 2,
        "Expected max concurrent <= 2, but observed {}",
        observed_max
    );
}

#[rstest]
#[tokio::test]
async fn test_batch_run_buffered_completes_all() {
    // Verify all tasks complete
    let counter = Arc::new(AtomicUsize::new(0));

    let items: Vec<AsyncIO<i32>> = (0..10)
        .map(|index| {
            let counter = Arc::clone(&counter);
            AsyncIO::new(move || {
                let counter = Arc::clone(&counter);
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    index
                }
            })
        })
        .collect();

    let results = AsyncIO::batch_run_buffered(items, 3).await.unwrap();

    assert_eq!(results.len(), 10);

    // All tasks were executed
    assert_eq!(counter.load(Ordering::SeqCst), 10);
}

#[rstest]
#[tokio::test]
async fn test_batch_run_buffered_with_large_limit() {
    // Works correctly even when limit exceeds number of tasks
    let items = vec![AsyncIO::pure(1), AsyncIO::pure(2), AsyncIO::pure(3)];

    let mut results = AsyncIO::batch_run_buffered(items, 100).await.unwrap();
    results.sort();
    assert_eq!(results, vec![1, 2, 3]);
}

#[rstest]
#[tokio::test]
async fn test_batch_run_buffered_limit_one() {
    // limit=1 effectively serializes execution
    let execution_order = Arc::new(std::sync::Mutex::new(Vec::new()));

    let items: Vec<AsyncIO<i32>> = (0..3)
        .map(|index| {
            let execution_order = Arc::clone(&execution_order);
            AsyncIO::new(move || {
                let execution_order = Arc::clone(&execution_order);
                async move {
                    execution_order.lock().unwrap().push(index);
                    index
                }
            })
        })
        .collect();

    let results = AsyncIO::batch_run_buffered(items, 1).await.unwrap();

    assert_eq!(results.len(), 3);
    // With limit=1, execution order is guaranteed
    let order = execution_order.lock().unwrap();
    assert_eq!(*order, vec![0, 1, 2]);
}

#[rstest]
#[tokio::test]
async fn test_batch_run_buffered_backpressure() {
    // Verify backpressure: run 6 tasks with limit=2, recording start times
    // to confirm that only 2 tasks start at a time.
    let start = Instant::now();
    let start_times = Arc::new(std::sync::Mutex::new(Vec::new()));

    let items: Vec<AsyncIO<i32>> = (0..6)
        .map(|index| {
            let start_times = Arc::clone(&start_times);
            AsyncIO::new(move || {
                let start_times = Arc::clone(&start_times);
                async move {
                    let elapsed = start.elapsed();
                    start_times.lock().unwrap().push((index, elapsed));
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    index
                }
            })
        })
        .collect();

    let results = AsyncIO::batch_run_buffered(items, 2).await.unwrap();

    assert_eq!(results.len(), 6);

    // Analyze start times: first 2 start immediately (<50ms),
    // next 2 at ~50ms, last 2 at ~100ms, confirming backpressure.
    let times = start_times.lock().unwrap();
    assert_eq!(times.len(), 6);
    let mut early_count = 0;
    let mut mid_count = 0;
    let mut late_count = 0;

    for (_, elapsed) in times.iter() {
        if *elapsed < Duration::from_millis(30) {
            early_count += 1;
        } else if *elapsed < Duration::from_millis(80) {
            mid_count += 1;
        } else {
            late_count += 1;
        }
    }

    // Expect 2 early, 2 mid, 2 late (with timing tolerance)
    assert!(
        (1..=3).contains(&early_count),
        "Expected 1-3 early tasks, got {}",
        early_count
    );
    assert!(
        early_count + mid_count + late_count == 6,
        "Total should be 6"
    );
}

// =============================================================================
// Edge Cases
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_batch_run_with_mixed_durations() {
    // Works correctly with mixed-duration tasks
    let items = vec![
        AsyncIO::new(|| async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            1
        }),
        AsyncIO::pure(2), // completes immediately
        AsyncIO::new(|| async {
            tokio::time::sleep(Duration::from_millis(30)).await;
            3
        }),
        AsyncIO::pure(4), // completes immediately
    ];

    let mut results = AsyncIO::batch_run(items).await;
    results.sort();
    assert_eq!(results, vec![1, 2, 3, 4]);
}

#[rstest]
#[tokio::test]
async fn test_batch_run_buffered_with_mixed_durations() {
    // buffered also handles mixed-duration tasks correctly
    let items = vec![
        AsyncIO::new(|| async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            1
        }),
        AsyncIO::pure(2),
        AsyncIO::new(|| async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            3
        }),
    ];

    let mut results = AsyncIO::batch_run_buffered(items, 2).await.unwrap();
    results.sort();
    assert_eq!(results, vec![1, 2, 3]);
}

// =============================================================================
// Type Inference Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_batch_run_type_inference_with_iterator() {
    // Type inference works when calling directly from an iterator
    let results: Vec<i32> = AsyncIO::batch_run((0..5).map(AsyncIO::pure)).await;
    assert_eq!(results.len(), 5);
}

#[rstest]
#[tokio::test]
async fn test_batch_run_buffered_type_inference_with_iterator() {
    // Type inference also works with buffered when calling from an iterator
    let results: Vec<i32> = AsyncIO::batch_run_buffered((0..5).map(AsyncIO::pure), 2)
        .await
        .unwrap();
    assert_eq!(results.len(), 5);
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_batch_run_buffered_limit_zero_returns_error() {
    // limit == 0 returns an InvalidLimit error
    let items = vec![AsyncIO::pure(1), AsyncIO::pure(2)];
    let result = AsyncIO::batch_run_buffered(items, 0).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), BatchError::InvalidLimit);
}

#[rstest]
#[tokio::test]
async fn test_batch_run_buffered_limit_zero_with_empty_items() {
    // limit == 0 returns an error even with an empty iterator
    let items: Vec<AsyncIO<i32>> = vec![];
    let result = AsyncIO::batch_run_buffered(items, 0).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), BatchError::InvalidLimit);
}

#[rstest]
fn test_batch_error_display() {
    // Verify BatchError Display implementation
    let error = BatchError::InvalidLimit;
    let message = format!("{}", error);
    assert!(message.contains("limit must be greater than 0"));
}

#[rstest]
fn test_batch_error_debug() {
    // Verify BatchError Debug implementation
    let error = BatchError::InvalidLimit;
    let debug = format!("{:?}", error);
    assert_eq!(debug, "InvalidLimit");
}

#[rstest]
fn test_batch_error_is_error() {
    // Verify BatchError implements std::error::Error
    fn assert_error<E: std::error::Error>(_: E) {}
    assert_error(BatchError::InvalidLimit);
}
