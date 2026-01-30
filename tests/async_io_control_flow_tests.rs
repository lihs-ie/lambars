//! Integration tests for AsyncIO control flow utilities.
//!
//! This module tests the advanced control flow operations:
//! - Retry operations (retry_with_factory, retry_with_backoff_factory)
//! - Parallel execution (par, par3, race_result)
//! - Resource management (bracket, finally_async, on_error)
//! - Timeout extensions (timeout_result, TimeoutError)

#![cfg(feature = "async")]
#![allow(deprecated)]

use lambars::effect::AsyncIO;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

// =============================================================================
// Retry Operation Integration Tests
// =============================================================================

mod retry_tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_combined_with_timeout() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        // Combine retry with timeout
        let action = AsyncIO::retry_with_factory(
            move || {
                let counter = counter_clone.clone();
                AsyncIO::new(move || {
                    let counter = counter.clone();
                    async move {
                        let count = counter.fetch_add(1, Ordering::SeqCst);
                        if count < 2 {
                            Err("temporary failure")
                        } else {
                            Ok(42)
                        }
                    }
                })
            },
            5,
        )
        .timeout_result(Duration::from_secs(5));

        let result = action.run_async().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ok(42));
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_with_finally_cleanup() {
        let cleanup_called = Arc::new(AtomicBool::new(false));
        let cleanup_clone = cleanup_called.clone();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let action = AsyncIO::retry_with_factory(
            move || {
                let counter = counter_clone.clone();
                AsyncIO::new(move || {
                    let counter = counter.clone();
                    async move {
                        counter.fetch_add(1, Ordering::SeqCst);
                        Err::<i32, _>("permanent failure")
                    }
                })
            },
            3,
        )
        .finally_async(move || {
            let cleanup = cleanup_clone.clone();
            async move {
                cleanup.store(true, Ordering::SeqCst);
            }
        });

        let result = action.run_async().await;
        assert_eq!(result, Err("permanent failure"));
        assert!(cleanup_called.load(Ordering::SeqCst));
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_with_on_error_logging() {
        let error_logged = Arc::new(AtomicBool::new(false));
        let error_clone = error_logged.clone();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let factory_counter = Arc::new(AtomicUsize::new(0));
        let factory_counter_clone = factory_counter.clone();

        let action = AsyncIO::retry_with_factory(
            move || {
                let counter = counter_clone.clone();
                AsyncIO::new(move || {
                    let counter = counter.clone();
                    async move {
                        counter.fetch_add(1, Ordering::SeqCst);
                        Err::<i32, String>("retry error".to_string())
                    }
                })
            },
            3,
        )
        .on_error(move |_error| {
            let error_logged = error_clone.clone();
            let factory_counter = factory_counter_clone.clone();
            async move {
                error_logged.store(true, Ordering::SeqCst);
                factory_counter.fetch_add(1, Ordering::SeqCst);
            }
        });

        let result = action.run_async().await;
        assert!(result.is_err());
        assert!(error_logged.load(Ordering::SeqCst));
        // on_error should be called once after all retries fail
        assert_eq!(factory_counter.load(Ordering::SeqCst), 1);
    }
}

// =============================================================================
// Parallel Execution Integration Tests
// =============================================================================

mod parallel_tests {
    use super::*;

    #[tokio::test]
    async fn test_par_with_different_durations() {
        let start = Instant::now();

        let slow = AsyncIO::delay_async(Duration::from_millis(50)).fmap(|_| "slow");
        let fast = AsyncIO::pure("fast");

        let (slow_result, fast_result) = slow.par(fast).run_async().await;
        let elapsed = start.elapsed();

        assert_eq!(slow_result, "slow");
        assert_eq!(fast_result, "fast");
        // Should complete in about 50ms (parallel), not 50ms + 0ms (sequential)
        assert!(elapsed < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_par3_timing() {
        let start = Instant::now();

        let action_first = AsyncIO::delay_async(Duration::from_millis(50)).fmap(|_| 1);
        let action_second = AsyncIO::delay_async(Duration::from_millis(50)).fmap(|_| 2);
        let action_third = AsyncIO::delay_async(Duration::from_millis(50)).fmap(|_| 3);

        let (first, second, third) = action_first
            .par3(action_second, action_third)
            .run_async()
            .await;
        let elapsed = start.elapsed();

        assert_eq!((first, second, third), (1, 2, 3));
        // All three run in parallel, should be around 50ms, not 150ms
        assert!(elapsed < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_race_result_with_both_slow() {
        let start = Instant::now();

        let first = AsyncIO::delay_async(Duration::from_millis(50)).fmap(|_| 1);
        let second = AsyncIO::delay_async(Duration::from_millis(100)).fmap(|_| 2);

        let result = first.race_result(second).run_async().await;
        let elapsed = start.elapsed();

        assert_eq!(result, 1); // first should win
        // Should complete around 50ms (first wins), not wait for second
        assert!(elapsed < Duration::from_millis(75));
    }

    #[tokio::test]
    async fn test_par_combined_with_fmap() {
        let first = AsyncIO::pure(10);
        let second = AsyncIO::pure(20);

        let result = first
            .par(second)
            .fmap(|(first_value, second_value)| first_value + second_value)
            .run_async()
            .await;

        assert_eq!(result, 30);
    }

    #[tokio::test]
    async fn test_race_result_combined_with_finally() {
        let cleanup_called = Arc::new(AtomicBool::new(false));
        let cleanup_clone = cleanup_called.clone();

        let slow = AsyncIO::delay_async(Duration::from_secs(10)).fmap(|_| 1);
        let fast = AsyncIO::pure(2);

        let result = slow
            .race_result(fast)
            .finally_async(move || {
                let cleanup = cleanup_clone.clone();
                async move {
                    cleanup.store(true, Ordering::SeqCst);
                }
            })
            .run_async()
            .await;

        assert_eq!(result, 2);
        assert!(cleanup_called.load(Ordering::SeqCst));
    }
}

// =============================================================================
// Resource Management Integration Tests
// =============================================================================

mod resource_tests {
    use super::*;

    #[tokio::test]
    async fn test_bracket_with_async_operations() {
        let acquire_count = Arc::new(AtomicUsize::new(0));
        let use_count = Arc::new(AtomicUsize::new(0));
        let release_count = Arc::new(AtomicUsize::new(0));

        let acquire_clone = acquire_count.clone();
        let use_clone = use_count.clone();
        let release_clone = release_count.clone();

        let result = AsyncIO::bracket(
            move || {
                let acquire = acquire_clone.clone();
                AsyncIO::new(move || async move {
                    acquire.fetch_add(1, Ordering::SeqCst);
                    "resource"
                })
            },
            move |resource| {
                let use_count = use_clone.clone();
                AsyncIO::new(move || async move {
                    use_count.fetch_add(1, Ordering::SeqCst);
                    format!("used {}", resource)
                })
            },
            move |_resource| {
                let release = release_clone.clone();
                AsyncIO::new(move || async move {
                    release.fetch_add(1, Ordering::SeqCst);
                })
            },
        );

        let value = result.run_async().await;
        assert_eq!(value, "used resource");
        assert_eq!(acquire_count.load(Ordering::SeqCst), 1);
        assert_eq!(use_count.load(Ordering::SeqCst), 1);
        assert_eq!(release_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_nested_bracket() {
        let outer_released = Arc::new(AtomicBool::new(false));
        let inner_released = Arc::new(AtomicBool::new(false));

        let outer_clone = outer_released.clone();
        let inner_clone = inner_released.clone();

        let result: i32 = AsyncIO::bracket(
            || AsyncIO::pure(1),
            move |outer| {
                let inner_released = inner_clone.clone();
                AsyncIO::bracket(
                    move || AsyncIO::pure(outer + 10),
                    move |inner| AsyncIO::pure(inner * 2),
                    move |_| {
                        let inner_released = inner_released.clone();
                        AsyncIO::new(move || async move {
                            inner_released.store(true, Ordering::SeqCst);
                        })
                    },
                )
            },
            move |_| {
                let outer_released = outer_clone.clone();
                AsyncIO::new(move || async move {
                    outer_released.store(true, Ordering::SeqCst);
                })
            },
        )
        .run_async()
        .await;

        assert_eq!(result, 22); // (1 + 10) * 2
        assert!(outer_released.load(Ordering::SeqCst));
        assert!(inner_released.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_finally_async_chaining() {
        let first_cleanup = Arc::new(AtomicBool::new(false));
        let second_cleanup = Arc::new(AtomicBool::new(false));

        let first_clone = first_cleanup.clone();
        let second_clone = second_cleanup.clone();

        let result = AsyncIO::pure(42)
            .finally_async(move || {
                let first = first_clone.clone();
                async move {
                    first.store(true, Ordering::SeqCst);
                }
            })
            .finally_async(move || {
                let second = second_clone.clone();
                async move {
                    second.store(true, Ordering::SeqCst);
                }
            })
            .run_async()
            .await;

        assert_eq!(result, 42);
        assert!(first_cleanup.load(Ordering::SeqCst));
        assert!(second_cleanup.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_on_error_with_successful_operation() {
        let callback_called = Arc::new(AtomicBool::new(false));
        let callback_clone = callback_called.clone();

        let action: AsyncIO<Result<i32, String>> = AsyncIO::pure(Ok(42));
        let result = action
            .on_error(move |_| {
                let called = callback_clone.clone();
                async move {
                    called.store(true, Ordering::SeqCst);
                }
            })
            .run_async()
            .await;

        assert_eq!(result, Ok(42));
        assert!(!callback_called.load(Ordering::SeqCst)); // Should not be called
    }
}

// =============================================================================
// Timeout Extension Integration Tests
// =============================================================================

mod timeout_tests {
    use super::*;

    #[tokio::test]
    async fn test_timeout_error_provides_duration_info() {
        let timeout_duration = Duration::from_millis(50);
        let slow = AsyncIO::delay_async(Duration::from_secs(10))
            .fmap(|_| 42)
            .timeout_result(timeout_duration);

        let result = slow.run_async().await;

        match result {
            Err(error) => {
                assert_eq!(error.duration, timeout_duration);
                assert!(format!("{}", error).contains("timed out"));
            }
            Ok(_) => panic!("Expected timeout error"),
        }
    }

    #[tokio::test]
    async fn test_timeout_result_combined_with_retry() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        // Each retry has its own timeout
        let action = AsyncIO::retry_with_factory(
            move || {
                let counter = counter_clone.clone();
                AsyncIO::new(move || {
                    let counter = counter.clone();
                    async move {
                        let count = counter.fetch_add(1, Ordering::SeqCst);
                        // First two attempts take too long
                        if count < 2 {
                            tokio::time::sleep(Duration::from_millis(500)).await;
                        }
                        42i32
                    }
                })
                .timeout_result(Duration::from_millis(100))
                .fmap(|result| match result {
                    Ok(value) => Ok(value),
                    Err(_timeout_error) => Err("timeout"),
                })
            },
            5,
        );

        let result = action.run_async().await;
        assert_eq!(result, Ok(42));
        assert_eq!(counter.load(Ordering::SeqCst), 3); // First 2 timed out, 3rd succeeded
    }

    #[tokio::test]
    async fn test_timeout_result_with_par() {
        let slow = AsyncIO::delay_async(Duration::from_secs(10))
            .fmap(|_| 1)
            .timeout_result(Duration::from_millis(50));

        let fast = AsyncIO::pure(2).timeout_result(Duration::from_secs(1));

        let (slow_result, fast_result) = slow.par(fast).run_async().await;

        assert!(slow_result.is_err()); // Timed out
        assert_eq!(fast_result, Ok(2)); // Completed in time
    }
}

// =============================================================================
// Combined Operation Integration Tests
// =============================================================================

mod combined_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_pipeline_with_all_features() {
        let operation_count = Arc::new(AtomicUsize::new(0));
        let cleanup_count = Arc::new(AtomicUsize::new(0));
        let error_count = Arc::new(AtomicUsize::new(0));

        let op_clone = operation_count.clone();
        let cleanup_clone = cleanup_count.clone();
        let error_clone = error_count.clone();

        // Complex pipeline: retry with backoff -> bracket for resource -> timeout -> finally
        let result = AsyncIO::retry_with_backoff_factory(
            {
                let op_clone = op_clone.clone();
                move || {
                    let op_count = op_clone.clone();
                    AsyncIO::new(move || {
                        let op_count = op_count.clone();
                        async move {
                            let count = op_count.fetch_add(1, Ordering::SeqCst);
                            if count < 1 {
                                Err::<String, &str>("first failure")
                            } else {
                                Ok("success".to_string())
                            }
                        }
                    })
                }
            },
            3,
            Duration::from_millis(10),
        )
        .on_error(move |_| {
            let error_count = error_clone.clone();
            async move {
                error_count.fetch_add(1, Ordering::SeqCst);
            }
        })
        .finally_async(move || {
            let cleanup = cleanup_clone.clone();
            async move {
                cleanup.fetch_add(1, Ordering::SeqCst);
            }
        });

        let final_result = result.run_async().await;

        assert_eq!(final_result, Ok("success".to_string()));
        assert_eq!(operation_count.load(Ordering::SeqCst), 2); // 1 failure + 1 success
        assert_eq!(cleanup_count.load(Ordering::SeqCst), 1); // Finally called once
        assert_eq!(error_count.load(Ordering::SeqCst), 0); // No error callback (final result is Ok)
    }

    #[tokio::test]
    async fn test_lazy_evaluation_throughout_pipeline() {
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        // Build complex pipeline but don't execute
        let pipeline = AsyncIO::retry_with_factory(
            move || {
                let executed = executed_clone.clone();
                AsyncIO::new(move || {
                    let executed = executed.clone();
                    async move {
                        executed.store(true, Ordering::SeqCst);
                        Ok::<i32, &str>(42)
                    }
                })
            },
            3,
        )
        .timeout_result(Duration::from_secs(1))
        .finally_async(|| async {});

        // Verify no execution happened yet
        assert!(!executed.load(Ordering::SeqCst));

        // Now execute
        let _ = pipeline.run_async().await;
        assert!(executed.load(Ordering::SeqCst));
    }
}

// =============================================================================
// Property-Like Tests (Law Verification)
// =============================================================================

mod law_tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_zero_equals_one() {
        // Retry Zero Law: retry(0) == retry(1) == f()
        let result_zero: Result<i32, &str> =
            AsyncIO::retry_with_factory(|| AsyncIO::pure(Ok::<i32, &str>(42)), 0)
                .run_async()
                .await;

        let result_one: Result<i32, &str> =
            AsyncIO::retry_with_factory(|| AsyncIO::pure(Ok::<i32, &str>(42)), 1)
                .run_async()
                .await;

        assert_eq!(result_zero, result_one);
        assert_eq!(result_zero, Ok(42));
    }

    #[tokio::test]
    async fn test_retry_success_no_retry() {
        // Retry Success Law: successful operations don't retry
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let _result: Result<i32, &str> = AsyncIO::retry_with_factory(
            move || {
                let counter = counter_clone.clone();
                AsyncIO::new(move || {
                    let counter = counter.clone();
                    async move {
                        counter.fetch_add(1, Ordering::SeqCst);
                        Ok(42)
                    }
                })
            },
            10,
        )
        .run_async()
        .await;

        // Should only be called once despite 10 max attempts
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_bracket_always_releases() {
        // Bracket Release Guarantee Law
        let released = Arc::new(AtomicBool::new(false));
        let released_clone = released.clone();

        // Even when use returns an error, release is called
        let result: Result<i32, &str> = AsyncIO::bracket(
            || AsyncIO::pure(42),
            |_| AsyncIO::pure(Err("error")),
            move |_| {
                let released = released_clone.clone();
                AsyncIO::new(move || async move {
                    released.store(true, Ordering::SeqCst);
                })
            },
        )
        .run_async()
        .await;

        assert_eq!(result, Err("error"));
        assert!(released.load(Ordering::SeqCst)); // Release was called
    }

    #[tokio::test]
    async fn test_par_results_order() {
        // Par should return results in order (first, second)
        let first = AsyncIO::delay_async(Duration::from_millis(50)).fmap(|_| "first");
        let second = AsyncIO::pure("second");

        let (first_result, second_result) = first.par(second).run_async().await;

        // Even though second finishes first, order is preserved
        assert_eq!(first_result, "first");
        assert_eq!(second_result, "second");
    }
}
