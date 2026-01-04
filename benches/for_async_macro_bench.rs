//! Benchmark for the `for_async!` macro.
//!
//! Compares the performance of the `for_async!` macro against
//! the synchronous `for_!` macro and hand-written async code.

#![cfg(feature = "async")]

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lambars::effect::AsyncIO;
use lambars::for_;
use lambars::for_async;
use std::hint::black_box;

// =============================================================================
// Single Iteration Benchmark
// =============================================================================

fn benchmark_async_single_iteration(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    let mut group = criterion.benchmark_group("for_async_single_iteration");

    for size in [100, 1_000, 10_000] {
        let data: Vec<i32> = (0..size).collect();

        // for_async! macro
        group.bench_with_input(
            BenchmarkId::new("for_async_macro", size),
            &size,
            |bencher, _| {
                let data_clone = data.clone();
                bencher.iter(|| {
                    let data_inner = data_clone.clone();
                    runtime.block_on(async move {
                        let result = for_async! {
                            x <= data_inner;
                            yield black_box(x * 2)
                        };
                        black_box(result.run_async().await)
                    })
                });
            },
        );

        // for_! macro (sync baseline)
        group.bench_with_input(
            BenchmarkId::new("for_macro_sync", size),
            &size,
            |bencher, _| {
                let data_clone = data.clone();
                bencher.iter(|| {
                    let result = for_! {
                        x <= data_clone.clone();
                        yield black_box(x * 2)
                    };
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Two-Level Nested Iteration Benchmark
// =============================================================================

fn benchmark_async_nested_two_levels(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    let mut group = criterion.benchmark_group("for_async_nested_two_levels");

    for size in [10, 50, 100] {
        let outer: Vec<i32> = (0..size).collect();
        let inner: Vec<i32> = (0..size).collect();

        // for_async! macro
        group.bench_with_input(
            BenchmarkId::new("for_async_macro", size * size),
            &size,
            |bencher, _| {
                let outer_clone = outer.clone();
                let inner_clone = inner.clone();
                bencher.iter(|| {
                    let outer_inner = outer_clone.clone();
                    let inner_inner = inner_clone.clone();
                    runtime.block_on(async move {
                        let result = for_async! {
                            x <= outer_inner;
                            y <= inner_inner.clone();
                            yield black_box(x + y)
                        };
                        black_box(result.run_async().await)
                    })
                });
            },
        );

        // for_! macro (sync baseline)
        group.bench_with_input(
            BenchmarkId::new("for_macro_sync", size * size),
            &size,
            |bencher, _| {
                let outer_clone = outer.clone();
                let inner_clone = inner.clone();
                bencher.iter(|| {
                    let inner_cloned = inner_clone.clone();
                    let result = for_! {
                        x <= outer_clone.clone();
                        y <= inner_cloned.clone();
                        yield black_box(x + y)
                    };
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// AsyncIO Bind (<~) Benchmark
// =============================================================================

fn benchmark_async_io_bind(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    let mut group = criterion.benchmark_group("for_async_io_bind");

    for size in [100, 1_000, 10_000] {
        let data: Vec<i32> = (0..size).collect();

        // for_async! macro with AsyncIO bind
        group.bench_with_input(
            BenchmarkId::new("with_async_bind", size),
            &size,
            |bencher, _| {
                let data_clone = data.clone();
                bencher.iter(|| {
                    let data_inner = data_clone.clone();
                    runtime.block_on(async move {
                        let result = for_async! {
                            x <= data_inner;
                            doubled <~ AsyncIO::pure(x * 2);
                            yield black_box(doubled)
                        };
                        black_box(result.run_async().await)
                    })
                });
            },
        );

        // for_async! macro without AsyncIO bind (baseline)
        group.bench_with_input(
            BenchmarkId::new("without_async_bind", size),
            &size,
            |bencher, _| {
                let data_clone = data.clone();
                bencher.iter(|| {
                    let data_inner = data_clone.clone();
                    runtime.block_on(async move {
                        let result = for_async! {
                            x <= data_inner;
                            yield black_box(x * 2)
                        };
                        black_box(result.run_async().await)
                    })
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Let Binding Benchmark
// =============================================================================

fn benchmark_async_let_binding(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    let mut group = criterion.benchmark_group("for_async_let_binding");

    for size in [100, 1_000, 10_000] {
        let data: Vec<i32> = (0..size).collect();

        // for_async! macro with let binding
        group.bench_with_input(
            BenchmarkId::new("for_async_macro", size),
            &size,
            |bencher, _| {
                let data_clone = data.clone();
                bencher.iter(|| {
                    let data_inner = data_clone.clone();
                    runtime.block_on(async move {
                        let result = for_async! {
                            x <= data_inner;
                            let doubled = x * 2;
                            let squared = doubled * doubled;
                            yield black_box(squared)
                        };
                        black_box(result.run_async().await)
                    })
                });
            },
        );

        // for_! macro with let binding (sync baseline)
        group.bench_with_input(
            BenchmarkId::new("for_macro_sync", size),
            &size,
            |bencher, _| {
                let data_clone = data.clone();
                bencher.iter(|| {
                    let result = for_! {
                        x <= data_clone.clone();
                        let doubled = x * 2;
                        let squared = doubled * doubled;
                        yield black_box(squared)
                    };
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Multiple AsyncIO Binds Benchmark
// =============================================================================

fn benchmark_multiple_async_binds(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    let mut group = criterion.benchmark_group("for_async_multiple_binds");

    for size in [100, 1_000, 5_000] {
        let data: Vec<i32> = (0..size).collect();

        // for_async! macro with two AsyncIO binds
        group.bench_with_input(BenchmarkId::new("two_binds", size), &size, |bencher, _| {
            let data_clone = data.clone();
            bencher.iter(|| {
                let data_inner = data_clone.clone();
                runtime.block_on(async move {
                    let result = for_async! {
                        x <= data_inner;
                        doubled <~ AsyncIO::pure(x * 2);
                        squared <~ AsyncIO::pure(doubled * doubled);
                        yield black_box(squared)
                    };
                    black_box(result.run_async().await)
                })
            });
        });

        // for_async! macro with three AsyncIO binds
        group.bench_with_input(
            BenchmarkId::new("three_binds", size),
            &size,
            |bencher, _| {
                let data_clone = data.clone();
                bencher.iter(|| {
                    let data_inner = data_clone.clone();
                    runtime.block_on(async move {
                        let result = for_async! {
                            x <= data_inner;
                            a <~ AsyncIO::pure(x + 1);
                            b <~ AsyncIO::pure(a * 2);
                            c <~ AsyncIO::pure(b + 10);
                            yield black_box(c)
                        };
                        black_box(result.run_async().await)
                    })
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Criterion Group and Main
// =============================================================================

criterion_group!(
    benches,
    benchmark_async_single_iteration,
    benchmark_async_nested_two_levels,
    benchmark_async_io_bind,
    benchmark_async_let_binding,
    benchmark_multiple_async_binds
);

criterion_main!(benches);
