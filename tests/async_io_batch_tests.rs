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
    // 空のイテレータを渡した場合、空の Vec が返される
    let items: Vec<AsyncIO<i32>> = vec![];
    let results = AsyncIO::batch_run(items).await;
    assert!(results.is_empty());
}

#[rstest]
#[tokio::test]
async fn test_batch_run_single() {
    // 単一の AsyncIO を実行
    let items = vec![AsyncIO::pure(42)];
    let results = AsyncIO::batch_run(items).await;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], 42);
}

#[rstest]
#[tokio::test]
async fn test_batch_run_multiple() {
    // 複数の AsyncIO を実行し、全ての結果が含まれることを確認
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
    // 並列実行されることを確認（全てが同時に開始される）
    // 各タスクが 50ms かかる場合、直列では 250ms 以上かかるが、
    // 並列では 50ms 程度で完了するはず
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

    // 5 つの結果が返される
    assert_eq!(results.len(), 5);

    // 並列実行されていれば 150ms 以内に完了するはず（余裕を持って）
    // 直列なら 250ms 以上かかる
    assert!(
        elapsed < Duration::from_millis(150),
        "Expected parallel execution to complete in <150ms, but took {:?}",
        elapsed
    );
}

#[rstest]
#[tokio::test]
async fn test_batch_run_with_deferred_computation() {
    // defer された計算が実際に実行されることを確認
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

    // batch_run 前はまだ実行されていない
    assert_eq!(counter.load(Ordering::SeqCst), 0);

    let results = AsyncIO::batch_run(items).await;

    // batch_run 後は全て実行されている
    assert_eq!(counter.load(Ordering::SeqCst), 3);
    assert_eq!(results.len(), 3);
}

#[rstest]
#[tokio::test]
async fn test_batch_run_preserves_all_values() {
    // 異なる型の値（文字列）でも正しく動作することを確認
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
    // 空のイテレータを渡した場合、空の Vec が返される
    let items: Vec<AsyncIO<i32>> = vec![];
    let results = AsyncIO::batch_run_buffered(items, 2).await.unwrap();
    assert!(results.is_empty());
}

#[rstest]
#[tokio::test]
async fn test_batch_run_buffered_single() {
    // 単一の AsyncIO を実行
    let items = vec![AsyncIO::pure(42)];
    let results = AsyncIO::batch_run_buffered(items, 2).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], 42);
}

#[rstest]
#[tokio::test]
async fn test_batch_run_buffered_respects_limit() {
    // limit を超える同時実行が行われないことを確認
    // 5 つのタスクを limit=2 で実行
    // 各タスクは実行中に concurrent_count をインクリメントし、
    // 終了時にデクリメントする
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
                    // 実行開始
                    let current = concurrent_count.fetch_add(1, Ordering::SeqCst) + 1;

                    // 最大同時実行数を記録
                    max_concurrent.fetch_max(current, Ordering::SeqCst);

                    // 少し待機して同時実行の機会を作る
                    tokio::time::sleep(Duration::from_millis(30)).await;

                    // 実行終了
                    concurrent_count.fetch_sub(1, Ordering::SeqCst);

                    index
                }
            })
        })
        .collect();

    let results = AsyncIO::batch_run_buffered(items, 2).await.unwrap();

    // 全ての結果が返される
    assert_eq!(results.len(), 5);

    // 最大同時実行数が limit を超えていないことを確認
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
    // 全てのタスクが完了することを確認
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

    // 全ての結果が返される
    assert_eq!(results.len(), 10);

    // 全てのタスクが実行された
    assert_eq!(counter.load(Ordering::SeqCst), 10);
}

#[rstest]
#[tokio::test]
async fn test_batch_run_buffered_with_large_limit() {
    // limit がタスク数より大きい場合も正しく動作する
    let items = vec![AsyncIO::pure(1), AsyncIO::pure(2), AsyncIO::pure(3)];

    let mut results = AsyncIO::batch_run_buffered(items, 100).await.unwrap();
    results.sort();
    assert_eq!(results, vec![1, 2, 3]);
}

#[rstest]
#[tokio::test]
async fn test_batch_run_buffered_limit_one() {
    // limit=1 の場合は実質直列実行
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
    // limit=1 では順序が保証される
    let order = execution_order.lock().unwrap();
    assert_eq!(*order, vec![0, 1, 2]);
}

#[rstest]
#[tokio::test]
async fn test_batch_run_buffered_backpressure() {
    // Backpressure が機能することを確認
    // limit=2 で 6 つのタスクを実行し、
    // 各タスクの開始時刻を記録して、一度に 2 つずつ開始されることを確認
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

    // 開始時刻を分析
    let times = start_times.lock().unwrap();
    assert_eq!(times.len(), 6);

    // 最初の 2 つは即座に開始される（50ms 未満）
    // 次の 2 つは約 50ms 後に開始される
    // 最後の 2 つは約 100ms 後に開始される
    // これにより Backpressure が機能していることを確認
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

    // 最初に 2 つ、50ms 後に 2 つ、100ms 後に 2 つ開始されるはず
    // タイミングに余裕を持たせて検証
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
    // 異なる実行時間のタスクを混在させた場合も正しく動作する
    let items = vec![
        AsyncIO::new(|| async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            1
        }),
        AsyncIO::pure(2), // 即座に完了
        AsyncIO::new(|| async {
            tokio::time::sleep(Duration::from_millis(30)).await;
            3
        }),
        AsyncIO::pure(4), // 即座に完了
    ];

    let mut results = AsyncIO::batch_run(items).await;
    results.sort();
    assert_eq!(results, vec![1, 2, 3, 4]);
}

#[rstest]
#[tokio::test]
async fn test_batch_run_buffered_with_mixed_durations() {
    // buffered でも異なる実行時間のタスクが正しく処理される
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
    // イテレータから直接呼び出しても型推論が動作する
    let results: Vec<i32> = AsyncIO::batch_run((0..5).map(AsyncIO::pure)).await;
    assert_eq!(results.len(), 5);
}

#[rstest]
#[tokio::test]
async fn test_batch_run_buffered_type_inference_with_iterator() {
    // buffered でもイテレータから直接呼び出しても型推論が動作する
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
    // limit == 0 の場合は InvalidLimit エラーを返す
    let items = vec![AsyncIO::pure(1), AsyncIO::pure(2)];
    let result = AsyncIO::batch_run_buffered(items, 0).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), BatchError::InvalidLimit);
}

#[rstest]
#[tokio::test]
async fn test_batch_run_buffered_limit_zero_with_empty_items() {
    // 空のイテレータでも limit == 0 はエラーを返す
    let items: Vec<AsyncIO<i32>> = vec![];
    let result = AsyncIO::batch_run_buffered(items, 0).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), BatchError::InvalidLimit);
}

#[rstest]
fn test_batch_error_display() {
    // BatchError の Display 実装を確認
    let error = BatchError::InvalidLimit;
    let message = format!("{}", error);
    assert!(message.contains("limit must be greater than 0"));
}

#[rstest]
fn test_batch_error_debug() {
    // BatchError の Debug 実装を確認
    let error = BatchError::InvalidLimit;
    let debug = format!("{:?}", error);
    assert_eq!(debug, "InvalidLimit");
}

#[rstest]
fn test_batch_error_is_error() {
    // BatchError が std::error::Error を実装していることを確認
    fn assert_error<E: std::error::Error>(_: E) {}
    assert_error(BatchError::InvalidLimit);
}
