//! Benchmark for AsyncIO runtime overhead.
//!
//! This benchmark suite measures the performance of AsyncIO operations to ensure
//! that runtime enter/drop operations are not dominating the execution profile.
//!
//! # Benchmark Categories
//!
//! 1. **Runtime Reuse**: Measures overhead of runtime access patterns
//! 2. **AsyncIO Pure**: Pure value evaluation (zero-allocation path)
//! 3. **AsyncIO fmap**: Chain of fmap operations
//! 4. **AsyncIO flat_map**: Chain of flat_map operations
//! 5. **AsyncPool**: Spawn-drain cycle performance
//!
//! # Comparison with Legacy Benchmarks
//!
//! Most benchmarks in this file use `criterion::to_async()` instead of
//! `runtime.block_on()` to measure the actual async operation overhead
//! with reduced Runtime enter/drop impact (block_on calls are batched per sample).
//!
//! **Exception**: The `async_io_runtime_reuse` group intentionally uses
//! `block_on` to measure Runtime access patterns directly.
//!
//! ## Key Differences from `effect_bench.rs`
//!
//! | Aspect | effect_bench.rs | async_io_runtime_bench.rs |
//! |--------|-----------------|---------------------------|
//! | Execution | `runtime.block_on()` per iter | `to_async(&runtime)` (batched per sample) |
//! | Runtime overhead | Per-iteration | Reduced (batched, except reuse group) |
//! | Group prefix | `async_io_*` | `async_io_runtime_*` |
//!
//! ## How to Compare
//!
//! To verify REQ-ASYNC-RT-001 (Runtime reuse) effectiveness:
//!
//! 1. Compare `async_io_runtime_batch_run` with `async_io_batch_run` (effect_bench)
//! 2. Both use the same input sizes (10, 100, 1000 items)
//! 3. The difference shows Runtime enter/drop overhead reduction
//!
//! Note: Direct comparison is only meaningful for groups with matching
//! input sizes and operations. The `async_io_runtime_reuse` group
//! measures different aspects (Runtime access patterns).
//!
//! # Reference
//!
//! See `docs/internal/requirements/20260124_1204_async_io_runtime_overhead.yaml`
//! for the requirement specification (REQ-ASYNC-BENCH-001).

#![cfg(feature = "async")]

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lambars::effect::AsyncIO;
use lambars::effect::async_io::pool::AsyncPool;
use lambars::effect::async_io::runtime::{global, handle, runtime_id, try_run_blocking};
use std::hint::black_box;

// =============================================================================
// Runtime Reuse Benchmarks
// =============================================================================

/// Benchmarks runtime access pattern overhead.
fn benchmark_runtime_reuse(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("async_io_runtime_reuse");

    group.bench_function("runtime_id", |bencher| {
        bencher.iter(|| {
            let id = runtime_id();
            black_box(id)
        });
    });

    group.bench_function("global_access", |bencher| {
        bencher.iter(|| {
            let runtime = global();
            black_box(runtime)
        });
    });

    group.bench_function("handle_access", |bencher| {
        bencher.iter(|| {
            let obtained_handle = handle();
            black_box(obtained_handle)
        });
    });

    group.bench_function("try_run_blocking_simple", |bencher| {
        bencher.iter(|| {
            let result = try_run_blocking(async { black_box(42) });
            black_box(result)
        });
    });

    group.bench_function("global_block_on_simple", |bencher| {
        bencher.iter(|| {
            let result = global().block_on(async { black_box(42) });
            black_box(result)
        });
    });

    group.finish();
}

// =============================================================================
// AsyncIO Pure Benchmarks
// =============================================================================

/// Benchmarks AsyncIO::pure operations (zero-allocation path).
fn benchmark_async_io_pure(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let mut group = criterion.benchmark_group("async_io_runtime_pure");

    group.bench_function("pure_await", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let async_io = AsyncIO::pure(black_box(42));
            black_box(async_io.await)
        });
    });

    group.bench_function("pure_run_async", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let async_io = AsyncIO::pure(black_box(42));
            black_box(async_io.run_async().await)
        });
    });

    group.bench_function("pure_sequence_5", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let a = AsyncIO::pure(black_box(1)).await;
            let b = AsyncIO::pure(black_box(2)).await;
            let c = AsyncIO::pure(black_box(3)).await;
            let d = AsyncIO::pure(black_box(4)).await;
            let e = AsyncIO::pure(black_box(5)).await;
            black_box(a + b + c + d + e)
        });
    });

    group.bench_function("pure_sequence_5_run_async", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let a = AsyncIO::pure(black_box(1)).run_async().await;
            let b = AsyncIO::pure(black_box(2)).run_async().await;
            let c = AsyncIO::pure(black_box(3)).run_async().await;
            let d = AsyncIO::pure(black_box(4)).run_async().await;
            let e = AsyncIO::pure(black_box(5)).run_async().await;
            black_box(a + b + c + d + e)
        });
    });

    group.finish();
}

// =============================================================================
// AsyncIO fmap Benchmarks
// =============================================================================

/// Benchmarks fmap chain performance.
fn benchmark_async_io_fmap(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let mut group = criterion.benchmark_group("async_io_runtime_fmap");

    group.bench_function("fmap_1", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let async_io = AsyncIO::pure(black_box(1)).fmap(|x| x + 1);
            black_box(async_io.await)
        });
    });

    group.bench_function("fmap_1_run_async", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let async_io = AsyncIO::pure(black_box(1)).fmap(|x| x + 1);
            black_box(async_io.run_async().await)
        });
    });

    group.bench_function("fmap_5", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let async_io = AsyncIO::pure(black_box(1))
                .fmap(|x| x + 1)
                .fmap(|x| x * 2)
                .fmap(|x| x + 3)
                .fmap(|x| x * 4)
                .fmap(|x| x + 5);
            black_box(async_io.await)
        });
    });

    group.bench_function("fmap_5_run_async", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let async_io = AsyncIO::pure(black_box(1))
                .fmap(|x| x + 1)
                .fmap(|x| x * 2)
                .fmap(|x| x + 3)
                .fmap(|x| x * 4)
                .fmap(|x| x + 5);
            black_box(async_io.run_async().await)
        });
    });

    group.bench_function("fmap_10", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let async_io = AsyncIO::pure(black_box(0))
                .fmap(|x| x + 1)
                .fmap(|x| x + 1)
                .fmap(|x| x + 1)
                .fmap(|x| x + 1)
                .fmap(|x| x + 1)
                .fmap(|x| x + 1)
                .fmap(|x| x + 1)
                .fmap(|x| x + 1)
                .fmap(|x| x + 1)
                .fmap(|x| x + 1);
            black_box(async_io.await)
        });
    });

    group.bench_function("fmap_10_run_async", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let async_io = AsyncIO::pure(black_box(0))
                .fmap(|x| x + 1)
                .fmap(|x| x + 1)
                .fmap(|x| x + 1)
                .fmap(|x| x + 1)
                .fmap(|x| x + 1)
                .fmap(|x| x + 1)
                .fmap(|x| x + 1)
                .fmap(|x| x + 1)
                .fmap(|x| x + 1)
                .fmap(|x| x + 1);
            black_box(async_io.run_async().await)
        });
    });

    for depth in [1, 5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::new("fmap_chain", depth),
            &depth,
            |bencher, &depth| {
                bencher.to_async(&runtime).iter(|| async move {
                    let mut async_io = AsyncIO::pure(black_box(0));
                    for _ in 0..depth {
                        async_io = async_io.fmap(|x| x + 1);
                    }
                    black_box(async_io.await)
                });
            },
        );
    }

    for depth in [1, 5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::new("fmap_chain_run_async", depth),
            &depth,
            |bencher, &depth| {
                bencher.to_async(&runtime).iter(|| async move {
                    let mut async_io = AsyncIO::pure(black_box(0));
                    for _ in 0..depth {
                        async_io = async_io.fmap(|x| x + 1);
                    }
                    black_box(async_io.run_async().await)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// AsyncIO flat_map Benchmarks
// =============================================================================

/// Benchmarks flat_map chain performance.
fn benchmark_async_io_flat_map(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let mut group = criterion.benchmark_group("async_io_runtime_flat_map");

    group.bench_function("flat_map_1", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let async_io = AsyncIO::pure(black_box(1)).flat_map(|x| AsyncIO::pure(x + 1));
            black_box(async_io.await)
        });
    });

    group.bench_function("flat_map_1_run_async", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let async_io = AsyncIO::pure(black_box(1)).flat_map(|x| AsyncIO::pure(x + 1));
            black_box(async_io.run_async().await)
        });
    });

    group.bench_function("flat_map_5", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let async_io = AsyncIO::pure(black_box(1))
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x * 2))
                .flat_map(|x| AsyncIO::pure(x + 3))
                .flat_map(|x| AsyncIO::pure(x * 4))
                .flat_map(|x| AsyncIO::pure(x + 5));
            black_box(async_io.await)
        });
    });

    group.bench_function("flat_map_5_run_async", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let async_io = AsyncIO::pure(black_box(1))
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x * 2))
                .flat_map(|x| AsyncIO::pure(x + 3))
                .flat_map(|x| AsyncIO::pure(x * 4))
                .flat_map(|x| AsyncIO::pure(x + 5));
            black_box(async_io.run_async().await)
        });
    });

    group.bench_function("flat_map_10", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let async_io = AsyncIO::pure(black_box(0))
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x + 1));
            black_box(async_io.await)
        });
    });

    group.bench_function("flat_map_10_run_async", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let async_io = AsyncIO::pure(black_box(0))
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x + 1));
            black_box(async_io.run_async().await)
        });
    });

    for depth in [1, 5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::new("flat_map_chain", depth),
            &depth,
            |bencher, &depth| {
                bencher.to_async(&runtime).iter(|| async move {
                    let mut async_io = AsyncIO::pure(black_box(0));
                    for _ in 0..depth {
                        async_io = async_io.flat_map(|x| AsyncIO::pure(x + 1));
                    }
                    black_box(async_io.await)
                });
            },
        );
    }

    for depth in [1, 5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::new("flat_map_chain_run_async", depth),
            &depth,
            |bencher, &depth| {
                bencher.to_async(&runtime).iter(|| async move {
                    let mut async_io = AsyncIO::pure(black_box(0));
                    for _ in 0..depth {
                        async_io = async_io.flat_map(|x| AsyncIO::pure(x + 1));
                    }
                    black_box(async_io.run_async().await)
                });
            },
        );
    }

    group.bench_function("deferred_flat_map_5", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let async_io = AsyncIO::new(|| async { black_box(1) })
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x * 2))
                .flat_map(|x| AsyncIO::pure(x + 3))
                .flat_map(|x| AsyncIO::pure(x * 4))
                .flat_map(|x| AsyncIO::pure(x + 5));
            black_box(async_io.await)
        });
    });

    group.bench_function("deferred_flat_map_5_run_async", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let async_io = AsyncIO::new(|| async { black_box(1) })
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x * 2))
                .flat_map(|x| AsyncIO::pure(x + 3))
                .flat_map(|x| AsyncIO::pure(x * 4))
                .flat_map(|x| AsyncIO::pure(x + 5));
            black_box(async_io.run_async().await)
        });
    });

    group.finish();
}

// =============================================================================
// AsyncPool Benchmarks
// =============================================================================

/// Benchmarks AsyncPool spawn-drain cycle performance.
fn benchmark_async_pool_spawn_drain(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let mut group = criterion.benchmark_group("async_io_runtime_pool");

    group.bench_function("spawn_drain_10", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let mut pool = AsyncPool::new(10);
            for i in 0..10 {
                pool.try_spawn(async move { black_box(i) }).unwrap();
            }
            let results = pool.run_all().await;
            black_box(results)
        });
    });

    group.bench_function("spawn_drain_100", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let mut pool = AsyncPool::new(100);
            for i in 0..100 {
                pool.try_spawn(async move { black_box(i) }).unwrap();
            }
            let results = pool.run_all().await;
            black_box(results)
        });
    });

    group.bench_function("spawn_drain_1000", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let mut pool = AsyncPool::new(1000);
            for i in 0..1000 {
                pool.try_spawn(async move { black_box(i) }).unwrap();
            }
            let results = pool.run_all().await;
            black_box(results)
        });
    });

    for task_count in [10, 50, 100, 500, 1000] {
        group.bench_with_input(
            BenchmarkId::new("spawn_drain", task_count),
            &task_count,
            |bencher, &task_count| {
                bencher.to_async(&runtime).iter(|| async move {
                    let mut pool = AsyncPool::new(task_count);
                    for i in 0..task_count {
                        pool.try_spawn(async move { black_box(i) }).unwrap();
                    }
                    let results = pool.run_all().await;
                    black_box(results)
                });
            },
        );
    }

    group.bench_function("buffered_100_limit_10", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let mut pool = AsyncPool::new(100);
            for i in 0..100 {
                pool.try_spawn(async move { black_box(i) }).unwrap();
            }
            let results = pool.run_buffered(10).await.unwrap();
            black_box(results)
        });
    });

    group.bench_function("buffered_100_limit_50", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let mut pool = AsyncPool::new(100);
            for i in 0..100 {
                pool.try_spawn(async move { black_box(i) }).unwrap();
            }
            let results = pool.run_buffered(50).await.unwrap();
            black_box(results)
        });
    });

    group.bench_function("buffered_1000_limit_100", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let mut pool = AsyncPool::new(1000);
            for i in 0..1000 {
                pool.try_spawn(async move { black_box(i) }).unwrap();
            }
            let results = pool.run_buffered(100).await.unwrap();
            black_box(results)
        });
    });

    group.bench_function("sequential_10", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let mut results = Vec::with_capacity(10);
            for i in 0..10 {
                results.push(AsyncIO::pure(i).await);
            }
            black_box(results)
        });
    });

    group.bench_function("sequential_10_run_async", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let mut results = Vec::with_capacity(10);
            for i in 0..10 {
                results.push(AsyncIO::pure(i).run_async().await);
            }
            black_box(results)
        });
    });

    group.finish();
}

// =============================================================================
// AsyncIO batch_run Benchmarks
// =============================================================================

/// Benchmarks batch_run operations (parallel execution using FuturesUnordered).
fn benchmark_async_io_batch_run(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let mut group = criterion.benchmark_group("async_io_runtime_batch_run");

    group.bench_function("batch_run_10", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let items: Vec<AsyncIO<i32>> = (0..10).map(AsyncIO::pure).collect();
            let result = AsyncIO::batch_run(items).await;
            black_box(result)
        });
    });

    group.bench_function("batch_run_100", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let items: Vec<AsyncIO<i32>> = (0..100).map(AsyncIO::pure).collect();
            let result = AsyncIO::batch_run(items).await;
            black_box(result)
        });
    });

    group.bench_function("batch_run_1000", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let items: Vec<AsyncIO<i32>> = (0..1000).map(AsyncIO::pure).collect();
            let result = AsyncIO::batch_run(items).await;
            black_box(result)
        });
    });

    group.bench_function("batch_run_buffered_100_limit_10", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let items: Vec<AsyncIO<i32>> = (0..100).map(AsyncIO::pure).collect();
            let result = AsyncIO::batch_run_buffered(items, 10).await.unwrap();
            black_box(result)
        });
    });

    group.bench_function("batch_run_buffered_100_limit_50", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let items: Vec<AsyncIO<i32>> = (0..100).map(AsyncIO::pure).collect();
            let result = AsyncIO::batch_run_buffered(items, 50).await.unwrap();
            black_box(result)
        });
    });

    group.bench_function("batch_run_buffered_1000_limit_100", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let items: Vec<AsyncIO<i32>> = (0..1000).map(AsyncIO::pure).collect();
            let result = AsyncIO::batch_run_buffered(items, 100).await.unwrap();
            black_box(result)
        });
    });

    group.bench_function("sequential_10_vs_batch", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let mut results = Vec::with_capacity(10);
            for i in 0..10 {
                results.push(AsyncIO::pure(i).run_async().await);
            }
            black_box(results)
        });
    });

    group.finish();
}

// =============================================================================
// Comparison Benchmarks
// =============================================================================

/// Benchmarks comparing AsyncIO with direct async operations.
fn benchmark_comparison(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let mut group = criterion.benchmark_group("async_io_runtime_comparison");

    group.bench_function("direct_async", |bencher| {
        bencher.to_async(&runtime).iter(|| async { black_box(42) });
    });

    group.bench_function("async_io_pure", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let result = AsyncIO::pure(black_box(42)).await;
            black_box(result)
        });
    });

    group.bench_function("async_io_pure_run_async", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let result = AsyncIO::pure(black_box(42)).run_async().await;
            black_box(result)
        });
    });

    group.bench_function("async_io_new", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let result = AsyncIO::new(|| async { black_box(42) }).await;
            black_box(result)
        });
    });

    group.bench_function("async_io_new_run_async", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let result = AsyncIO::new(|| async { black_box(42) }).run_async().await;
            black_box(result)
        });
    });

    group.bench_function("direct_async_chain_5", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let a = black_box(1);
            let b = a + 1;
            let c = b * 2;
            let d = c + 3;
            let e = d * 4;
            let f = e + 5;
            black_box(f)
        });
    });

    group.bench_function("async_io_chain_5", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let result = AsyncIO::pure(black_box(1))
                .fmap(|x| x + 1)
                .fmap(|x| x * 2)
                .fmap(|x| x + 3)
                .fmap(|x| x * 4)
                .fmap(|x| x + 5)
                .await;
            black_box(result)
        });
    });

    group.bench_function("async_io_chain_5_run_async", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let result = AsyncIO::pure(black_box(1))
                .fmap(|x| x + 1)
                .fmap(|x| x * 2)
                .fmap(|x| x + 3)
                .fmap(|x| x * 4)
                .fmap(|x| x + 5)
                .run_async()
                .await;
            black_box(result)
        });
    });

    group.finish();
}

// =============================================================================
// Criterion Group and Main
// =============================================================================

criterion_group!(
    benches,
    benchmark_runtime_reuse,
    benchmark_async_io_pure,
    benchmark_async_io_fmap,
    benchmark_async_io_flat_map,
    benchmark_async_pool_spawn_drain,
    benchmark_async_io_batch_run,
    benchmark_comparison
);

criterion_main!(benches);
