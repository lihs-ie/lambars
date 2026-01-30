//! Benchmark for the `eff_async!` macro with ExceptT<E, AsyncIO<Result<A, E>>>.
//!
//! Measures the performance overhead of the `eff_async!` macro compared to
//! traditional flat_map chain style for monadic computations with error handling.
//!
//! # Benchmark Categories
//!
//! 1. **Chain Depth**: Measures overhead at different nesting depths (1, 5, 10, 20)
//! 2. **Pure Ratio**: Measures performance with different ratios of pure vs lifted operations
//! 3. **Error Path**: Measures performance for success, early error, and late error cases
//!
//! Each category compares `eff_async!` macro style against traditional `flat_map` chain style.

#![cfg(feature = "async")]
#![allow(deprecated)]

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lambars::eff_async;
use lambars::effect::{AsyncIO, ExceptT};
use std::hint::black_box;

// =============================================================================
// Chain Depth Benchmarks
// =============================================================================

/// Benchmarks the overhead of `eff_async!` macro at different chain depths.
///
/// The chain depth determines the number of monadic bindings in the computation.
/// Higher depths result in more nested closures for the macro-generated code.
fn benchmark_chain_depth(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let mut group = criterion.benchmark_group("eff_async_chain_depth");

    // Depth 1: Single bind
    group.bench_function("macro_depth_1", |bencher| {
        bencher.iter(|| {
            runtime.block_on(async {
                let result = eff_async! {
                    a <= ExceptT::<String, _>::pure_async_io(black_box(1));
                    ExceptT::pure_async_io(a)
                };
                black_box(result.run_async().await)
            })
        });
    });

    group.bench_function("traditional_depth_1", |bencher| {
        bencher.iter(|| {
            runtime.block_on(async {
                let result = ExceptT::<String, _>::pure_async_io(black_box(1))
                    .flat_map(ExceptT::pure_async_io);
                black_box(result.run_async().await)
            })
        });
    });

    // Depth 5: Five binds
    group.bench_function("macro_depth_5", |bencher| {
        bencher.iter(|| {
            runtime.block_on(async {
                let result = eff_async! {
                    a <= ExceptT::<String, _>::pure_async_io(black_box(1));
                    b <= ExceptT::pure_async_io(a + 1);
                    c <= ExceptT::pure_async_io(b + 1);
                    d <= ExceptT::pure_async_io(c + 1);
                    e <= ExceptT::pure_async_io(d + 1);
                    ExceptT::pure_async_io(e)
                };
                black_box(result.run_async().await)
            })
        });
    });

    group.bench_function("traditional_depth_5", |bencher| {
        bencher.iter(|| {
            runtime.block_on(async {
                let result = ExceptT::<String, _>::pure_async_io(black_box(1))
                    .flat_map(|a| ExceptT::pure_async_io(a + 1))
                    .flat_map(|b| ExceptT::pure_async_io(b + 1))
                    .flat_map(|c| ExceptT::pure_async_io(c + 1))
                    .flat_map(|d| ExceptT::pure_async_io(d + 1))
                    .flat_map(ExceptT::pure_async_io);
                black_box(result.run_async().await)
            })
        });
    });

    // Depth 10: Ten binds
    group.bench_function("macro_depth_10", |bencher| {
        bencher.iter(|| {
            runtime.block_on(async {
                let result = eff_async! {
                    a <= ExceptT::<String, _>::pure_async_io(black_box(1));
                    b <= ExceptT::pure_async_io(a + 1);
                    c <= ExceptT::pure_async_io(b + 1);
                    d <= ExceptT::pure_async_io(c + 1);
                    e <= ExceptT::pure_async_io(d + 1);
                    f <= ExceptT::pure_async_io(e + 1);
                    g <= ExceptT::pure_async_io(f + 1);
                    h <= ExceptT::pure_async_io(g + 1);
                    i <= ExceptT::pure_async_io(h + 1);
                    j <= ExceptT::pure_async_io(i + 1);
                    ExceptT::pure_async_io(j)
                };
                black_box(result.run_async().await)
            })
        });
    });

    group.bench_function("traditional_depth_10", |bencher| {
        bencher.iter(|| {
            runtime.block_on(async {
                let result = ExceptT::<String, _>::pure_async_io(black_box(1))
                    .flat_map(|a| ExceptT::pure_async_io(a + 1))
                    .flat_map(|b| ExceptT::pure_async_io(b + 1))
                    .flat_map(|c| ExceptT::pure_async_io(c + 1))
                    .flat_map(|d| ExceptT::pure_async_io(d + 1))
                    .flat_map(|e| ExceptT::pure_async_io(e + 1))
                    .flat_map(|f| ExceptT::pure_async_io(f + 1))
                    .flat_map(|g| ExceptT::pure_async_io(g + 1))
                    .flat_map(|h| ExceptT::pure_async_io(h + 1))
                    .flat_map(|i| ExceptT::pure_async_io(i + 1))
                    .flat_map(ExceptT::pure_async_io);
                black_box(result.run_async().await)
            })
        });
    });

    // Depth 20: Twenty binds
    group.bench_function("macro_depth_20", |bencher| {
        bencher.iter(|| {
            runtime.block_on(async {
                let result = eff_async! {
                    a <= ExceptT::<String, _>::pure_async_io(black_box(1));
                    b <= ExceptT::pure_async_io(a + 1);
                    c <= ExceptT::pure_async_io(b + 1);
                    d <= ExceptT::pure_async_io(c + 1);
                    e <= ExceptT::pure_async_io(d + 1);
                    f <= ExceptT::pure_async_io(e + 1);
                    g <= ExceptT::pure_async_io(f + 1);
                    h <= ExceptT::pure_async_io(g + 1);
                    i <= ExceptT::pure_async_io(h + 1);
                    j <= ExceptT::pure_async_io(i + 1);
                    k <= ExceptT::pure_async_io(j + 1);
                    l <= ExceptT::pure_async_io(k + 1);
                    m <= ExceptT::pure_async_io(l + 1);
                    n <= ExceptT::pure_async_io(m + 1);
                    o <= ExceptT::pure_async_io(n + 1);
                    p <= ExceptT::pure_async_io(o + 1);
                    q <= ExceptT::pure_async_io(p + 1);
                    r <= ExceptT::pure_async_io(q + 1);
                    s <= ExceptT::pure_async_io(r + 1);
                    t <= ExceptT::pure_async_io(s + 1);
                    ExceptT::pure_async_io(t)
                };
                black_box(result.run_async().await)
            })
        });
    });

    group.bench_function("traditional_depth_20", |bencher| {
        bencher.iter(|| {
            runtime.block_on(async {
                let result = ExceptT::<String, _>::pure_async_io(black_box(1))
                    .flat_map(|a| ExceptT::pure_async_io(a + 1))
                    .flat_map(|b| ExceptT::pure_async_io(b + 1))
                    .flat_map(|c| ExceptT::pure_async_io(c + 1))
                    .flat_map(|d| ExceptT::pure_async_io(d + 1))
                    .flat_map(|e| ExceptT::pure_async_io(e + 1))
                    .flat_map(|f| ExceptT::pure_async_io(f + 1))
                    .flat_map(|g| ExceptT::pure_async_io(g + 1))
                    .flat_map(|h| ExceptT::pure_async_io(h + 1))
                    .flat_map(|i| ExceptT::pure_async_io(i + 1))
                    .flat_map(|j| ExceptT::pure_async_io(j + 1))
                    .flat_map(|k| ExceptT::pure_async_io(k + 1))
                    .flat_map(|l| ExceptT::pure_async_io(l + 1))
                    .flat_map(|m| ExceptT::pure_async_io(m + 1))
                    .flat_map(|n| ExceptT::pure_async_io(n + 1))
                    .flat_map(|o| ExceptT::pure_async_io(o + 1))
                    .flat_map(|p| ExceptT::pure_async_io(p + 1))
                    .flat_map(|q| ExceptT::pure_async_io(q + 1))
                    .flat_map(|r| ExceptT::pure_async_io(r + 1))
                    .flat_map(|s| ExceptT::pure_async_io(s + 1))
                    .flat_map(ExceptT::pure_async_io);
                black_box(result.run_async().await)
            })
        });
    });

    group.finish();
}

// =============================================================================
// Pure Ratio Benchmarks
// =============================================================================

/// Benchmarks the performance with different ratios of pure vs lifted (deferred) operations.
///
/// - 0% pure: All operations are lifted from deferred `AsyncIO`
/// - 50% pure: Half pure, half lifted operations
/// - 100% pure: All operations are pure (no deferred execution)
fn benchmark_pure_ratio(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let mut group = criterion.benchmark_group("eff_async_pure_ratio");

    // 100% pure operations (all pure_async_io)
    group.bench_function("macro_pure_100", |bencher| {
        bencher.iter(|| {
            runtime.block_on(async {
                let result = eff_async! {
                    a <= ExceptT::<String, _>::pure_async_io(black_box(1));
                    b <= ExceptT::pure_async_io(a + 1);
                    c <= ExceptT::pure_async_io(b + 1);
                    d <= ExceptT::pure_async_io(c + 1);
                    e <= ExceptT::pure_async_io(d + 1);
                    ExceptT::pure_async_io(e)
                };
                black_box(result.run_async().await)
            })
        });
    });

    group.bench_function("traditional_pure_100", |bencher| {
        bencher.iter(|| {
            runtime.block_on(async {
                let result = ExceptT::<String, _>::pure_async_io(black_box(1))
                    .flat_map(|a| ExceptT::pure_async_io(a + 1))
                    .flat_map(|b| ExceptT::pure_async_io(b + 1))
                    .flat_map(|c| ExceptT::pure_async_io(c + 1))
                    .flat_map(|d| ExceptT::pure_async_io(d + 1))
                    .flat_map(ExceptT::pure_async_io);
                black_box(result.run_async().await)
            })
        });
    });

    // 50% pure operations (alternating pure and lifted)
    group.bench_function("macro_pure_50", |bencher| {
        bencher.iter(|| {
            runtime.block_on(async {
                let result = eff_async! {
                    a <= ExceptT::<String, _>::pure_async_io(black_box(1));
                    b <= ExceptT::lift_async_io(AsyncIO::new(move || async move { a + 1 }));
                    c <= ExceptT::pure_async_io(b + 1);
                    d <= ExceptT::lift_async_io(AsyncIO::new(move || async move { c + 1 }));
                    e <= ExceptT::pure_async_io(d + 1);
                    ExceptT::lift_async_io(AsyncIO::new(move || async move { e }))
                };
                black_box(result.run_async().await)
            })
        });
    });

    group.bench_function("traditional_pure_50", |bencher| {
        bencher.iter(|| {
            runtime.block_on(async {
                let result = ExceptT::<String, _>::pure_async_io(black_box(1))
                    .flat_map(|a| {
                        ExceptT::lift_async_io(AsyncIO::new(move || async move { a + 1 }))
                    })
                    .flat_map(|b| ExceptT::pure_async_io(b + 1))
                    .flat_map(|c| {
                        ExceptT::lift_async_io(AsyncIO::new(move || async move { c + 1 }))
                    })
                    .flat_map(|d| ExceptT::pure_async_io(d + 1))
                    .flat_map(|e| ExceptT::lift_async_io(AsyncIO::new(move || async move { e })));
                black_box(result.run_async().await)
            })
        });
    });

    // 0% pure operations (all lifted/deferred)
    group.bench_function("macro_pure_0", |bencher| {
        bencher.iter(|| {
            runtime.block_on(async {
                let initial = black_box(1);
                let result = eff_async! {
                    a <= ExceptT::<String, _>::lift_async_io(AsyncIO::new(move || async move { initial }));
                    b <= ExceptT::lift_async_io(AsyncIO::new(move || async move { a + 1 }));
                    c <= ExceptT::lift_async_io(AsyncIO::new(move || async move { b + 1 }));
                    d <= ExceptT::lift_async_io(AsyncIO::new(move || async move { c + 1 }));
                    e <= ExceptT::lift_async_io(AsyncIO::new(move || async move { d + 1 }));
                    ExceptT::lift_async_io(AsyncIO::new(move || async move { e }))
                };
                black_box(result.run_async().await)
            })
        });
    });

    group.bench_function("traditional_pure_0", |bencher| {
        bencher.iter(|| {
            runtime.block_on(async {
                let initial = black_box(1);
                let result = ExceptT::<String, _>::lift_async_io(AsyncIO::new(
                    move || async move { initial },
                ))
                .flat_map(|a| ExceptT::lift_async_io(AsyncIO::new(move || async move { a + 1 })))
                .flat_map(|b| ExceptT::lift_async_io(AsyncIO::new(move || async move { b + 1 })))
                .flat_map(|c| ExceptT::lift_async_io(AsyncIO::new(move || async move { c + 1 })))
                .flat_map(|d| ExceptT::lift_async_io(AsyncIO::new(move || async move { d + 1 })))
                .flat_map(|e| ExceptT::lift_async_io(AsyncIO::new(move || async move { e })));
                black_box(result.run_async().await)
            })
        });
    });

    group.finish();
}

// =============================================================================
// Error Path Benchmarks
// =============================================================================

/// Benchmarks the performance for different error scenarios.
///
/// - Success: All operations succeed
/// - Early Error: Error occurs at the beginning (step 1)
/// - Late Error: Error occurs near the end (step 4 of 5)
fn benchmark_error_path(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let mut group = criterion.benchmark_group("eff_async_error_path");

    // Success path: All operations succeed
    group.bench_function("macro_success", |bencher| {
        bencher.iter(|| {
            runtime.block_on(async {
                let result = eff_async! {
                    a <= ExceptT::<String, _>::pure_async_io(black_box(1));
                    b <= ExceptT::pure_async_io(a + 1);
                    c <= ExceptT::pure_async_io(b + 1);
                    d <= ExceptT::pure_async_io(c + 1);
                    e <= ExceptT::pure_async_io(d + 1);
                    ExceptT::pure_async_io(e)
                };
                black_box(result.run_async().await)
            })
        });
    });

    group.bench_function("traditional_success", |bencher| {
        bencher.iter(|| {
            runtime.block_on(async {
                let result = ExceptT::<String, _>::pure_async_io(black_box(1))
                    .flat_map(|a| ExceptT::pure_async_io(a + 1))
                    .flat_map(|b| ExceptT::pure_async_io(b + 1))
                    .flat_map(|c| ExceptT::pure_async_io(c + 1))
                    .flat_map(|d| ExceptT::pure_async_io(d + 1))
                    .flat_map(ExceptT::pure_async_io);
                black_box(result.run_async().await)
            })
        });
    });

    // Early error: Error at step 1
    group.bench_function("macro_early_error", |bencher| {
        bencher.iter(|| {
            runtime.block_on(async {
                let result: ExceptT<String, _> = eff_async! {
                    _a <= ExceptT::<String, AsyncIO<Result<i32, String>>>::throw_async_io(black_box("early error".to_string()));
                    b <= ExceptT::pure_async_io(1);
                    c <= ExceptT::pure_async_io(b + 1);
                    d <= ExceptT::pure_async_io(c + 1);
                    e <= ExceptT::pure_async_io(d + 1);
                    ExceptT::pure_async_io(e)
                };
                black_box(result.run_async().await)
            })
        });
    });

    group.bench_function("traditional_early_error", |bencher| {
        bencher.iter(|| {
            runtime.block_on(async {
                let result = ExceptT::<String, AsyncIO<Result<i32, String>>>::throw_async_io(
                    black_box("early error".to_string()),
                )
                .flat_map(|_a: i32| ExceptT::pure_async_io(1))
                .flat_map(|b| ExceptT::pure_async_io(b + 1))
                .flat_map(|c| ExceptT::pure_async_io(c + 1))
                .flat_map(|d| ExceptT::pure_async_io(d + 1))
                .flat_map(ExceptT::pure_async_io);
                black_box(result.run_async().await)
            })
        });
    });

    // Late error: Error at step 4 (of 5)
    group.bench_function("macro_late_error", |bencher| {
        bencher.iter(|| {
            runtime.block_on(async {
                let result: ExceptT<String, _> = eff_async! {
                    a <= ExceptT::<String, _>::pure_async_io(black_box(1));
                    b <= ExceptT::pure_async_io(a + 1);
                    c <= ExceptT::pure_async_io(b + 1);
                    _d <= ExceptT::<String, AsyncIO<Result<i32, String>>>::throw_async_io("late error".to_string());
                    e <= ExceptT::pure_async_io(c + 1);
                    ExceptT::pure_async_io(e)
                };
                black_box(result.run_async().await)
            })
        });
    });

    group.bench_function("traditional_late_error", |bencher| {
        bencher.iter(|| {
            runtime.block_on(async {
                let result = ExceptT::<String, _>::pure_async_io(black_box(1))
                    .flat_map(|a| ExceptT::pure_async_io(a + 1))
                    .flat_map(|b| ExceptT::pure_async_io(b + 1))
                    .flat_map(|_c| {
                        ExceptT::<String, AsyncIO<Result<i32, String>>>::throw_async_io(
                            "late error".to_string(),
                        )
                    })
                    .flat_map(|d: i32| ExceptT::pure_async_io(d + 1))
                    .flat_map(ExceptT::pure_async_io);
                black_box(result.run_async().await)
            })
        });
    });

    group.finish();
}

// =============================================================================
// Parameterized Chain Depth Benchmark
// =============================================================================

/// Benchmarks chain depth with parameterized inputs for more detailed analysis.
fn benchmark_chain_depth_parameterized(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let mut group = criterion.benchmark_group("eff_async_chain_depth_parameterized");

    for depth in [1, 5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::new("macro", depth),
            &depth,
            |bencher, &depth| {
                bencher.iter(|| {
                    runtime.block_on(async {
                        match depth {
                            1 => {
                                let result = eff_async! {
                                    a <= ExceptT::<String, _>::pure_async_io(black_box(1));
                                    ExceptT::pure_async_io(a)
                                };
                                black_box(result.run_async().await)
                            }
                            5 => {
                                let result = eff_async! {
                                    a <= ExceptT::<String, _>::pure_async_io(black_box(1));
                                    b <= ExceptT::pure_async_io(a + 1);
                                    c <= ExceptT::pure_async_io(b + 1);
                                    d <= ExceptT::pure_async_io(c + 1);
                                    e <= ExceptT::pure_async_io(d + 1);
                                    ExceptT::pure_async_io(e)
                                };
                                black_box(result.run_async().await)
                            }
                            10 => {
                                let result = eff_async! {
                                    a <= ExceptT::<String, _>::pure_async_io(black_box(1));
                                    b <= ExceptT::pure_async_io(a + 1);
                                    c <= ExceptT::pure_async_io(b + 1);
                                    d <= ExceptT::pure_async_io(c + 1);
                                    e <= ExceptT::pure_async_io(d + 1);
                                    f <= ExceptT::pure_async_io(e + 1);
                                    g <= ExceptT::pure_async_io(f + 1);
                                    h <= ExceptT::pure_async_io(g + 1);
                                    i <= ExceptT::pure_async_io(h + 1);
                                    j <= ExceptT::pure_async_io(i + 1);
                                    ExceptT::pure_async_io(j)
                                };
                                black_box(result.run_async().await)
                            }
                            20 => {
                                let result = eff_async! {
                                    a <= ExceptT::<String, _>::pure_async_io(black_box(1));
                                    b <= ExceptT::pure_async_io(a + 1);
                                    c <= ExceptT::pure_async_io(b + 1);
                                    d <= ExceptT::pure_async_io(c + 1);
                                    e <= ExceptT::pure_async_io(d + 1);
                                    f <= ExceptT::pure_async_io(e + 1);
                                    g <= ExceptT::pure_async_io(f + 1);
                                    h <= ExceptT::pure_async_io(g + 1);
                                    i <= ExceptT::pure_async_io(h + 1);
                                    j <= ExceptT::pure_async_io(i + 1);
                                    k <= ExceptT::pure_async_io(j + 1);
                                    l <= ExceptT::pure_async_io(k + 1);
                                    m <= ExceptT::pure_async_io(l + 1);
                                    n <= ExceptT::pure_async_io(m + 1);
                                    o <= ExceptT::pure_async_io(n + 1);
                                    p <= ExceptT::pure_async_io(o + 1);
                                    q <= ExceptT::pure_async_io(p + 1);
                                    r <= ExceptT::pure_async_io(q + 1);
                                    s <= ExceptT::pure_async_io(r + 1);
                                    t <= ExceptT::pure_async_io(s + 1);
                                    ExceptT::pure_async_io(t)
                                };
                                black_box(result.run_async().await)
                            }
                            _ => unreachable!(),
                        }
                    })
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("traditional", depth),
            &depth,
            |bencher, &depth| {
                bencher.iter(|| {
                    runtime.block_on(async {
                        match depth {
                            1 => {
                                let result = ExceptT::<String, _>::pure_async_io(black_box(1))
                                    .flat_map(ExceptT::pure_async_io);
                                black_box(result.run_async().await)
                            }
                            5 => {
                                let result = ExceptT::<String, _>::pure_async_io(black_box(1))
                                    .flat_map(|a| ExceptT::pure_async_io(a + 1))
                                    .flat_map(|b| ExceptT::pure_async_io(b + 1))
                                    .flat_map(|c| ExceptT::pure_async_io(c + 1))
                                    .flat_map(|d| ExceptT::pure_async_io(d + 1))
                                    .flat_map(ExceptT::pure_async_io);
                                black_box(result.run_async().await)
                            }
                            10 => {
                                let result = ExceptT::<String, _>::pure_async_io(black_box(1))
                                    .flat_map(|a| ExceptT::pure_async_io(a + 1))
                                    .flat_map(|b| ExceptT::pure_async_io(b + 1))
                                    .flat_map(|c| ExceptT::pure_async_io(c + 1))
                                    .flat_map(|d| ExceptT::pure_async_io(d + 1))
                                    .flat_map(|e| ExceptT::pure_async_io(e + 1))
                                    .flat_map(|f| ExceptT::pure_async_io(f + 1))
                                    .flat_map(|g| ExceptT::pure_async_io(g + 1))
                                    .flat_map(|h| ExceptT::pure_async_io(h + 1))
                                    .flat_map(|i| ExceptT::pure_async_io(i + 1))
                                    .flat_map(ExceptT::pure_async_io);
                                black_box(result.run_async().await)
                            }
                            20 => {
                                let result = ExceptT::<String, _>::pure_async_io(black_box(1))
                                    .flat_map(|a| ExceptT::pure_async_io(a + 1))
                                    .flat_map(|b| ExceptT::pure_async_io(b + 1))
                                    .flat_map(|c| ExceptT::pure_async_io(c + 1))
                                    .flat_map(|d| ExceptT::pure_async_io(d + 1))
                                    .flat_map(|e| ExceptT::pure_async_io(e + 1))
                                    .flat_map(|f| ExceptT::pure_async_io(f + 1))
                                    .flat_map(|g| ExceptT::pure_async_io(g + 1))
                                    .flat_map(|h| ExceptT::pure_async_io(h + 1))
                                    .flat_map(|i| ExceptT::pure_async_io(i + 1))
                                    .flat_map(|j| ExceptT::pure_async_io(j + 1))
                                    .flat_map(|k| ExceptT::pure_async_io(k + 1))
                                    .flat_map(|l| ExceptT::pure_async_io(l + 1))
                                    .flat_map(|m| ExceptT::pure_async_io(m + 1))
                                    .flat_map(|n| ExceptT::pure_async_io(n + 1))
                                    .flat_map(|o| ExceptT::pure_async_io(o + 1))
                                    .flat_map(|p| ExceptT::pure_async_io(p + 1))
                                    .flat_map(|q| ExceptT::pure_async_io(q + 1))
                                    .flat_map(|r| ExceptT::pure_async_io(r + 1))
                                    .flat_map(|s| ExceptT::pure_async_io(s + 1))
                                    .flat_map(ExceptT::pure_async_io);
                                black_box(result.run_async().await)
                            }
                            _ => unreachable!(),
                        }
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
    benchmark_chain_depth,
    benchmark_pure_ratio,
    benchmark_error_path,
    benchmark_chain_depth_parameterized
);

criterion_main!(benches);
