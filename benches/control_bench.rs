//! Benchmark for control structures: Lazy, ConcurrentLazy, and Trampoline.
//!
//! Measures the performance of lambars' control flow abstractions.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lambars::control::{ConcurrentLazy, Lazy, Trampoline};
use std::hint::black_box;
use std::sync::Arc;
use std::thread;

// =============================================================================
// Lazy Benchmarks
// =============================================================================

fn benchmark_lazy_force(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("lazy_force");

    // Benchmark initial evaluation (cold path)
    group.bench_function("initial_evaluation", |bencher| {
        bencher.iter(|| {
            let lazy = Lazy::new(|| {
                // Simulate some computation
                let mut sum = 0;
                for index in 0..100 {
                    sum += index;
                }
                sum
            });
            let value = lazy.force();
            black_box(*value)
        });
    });

    // Benchmark with different computation sizes
    for size in [10, 100, 1000] {
        group.bench_with_input(
            BenchmarkId::new("computation_size", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let lazy = Lazy::new(move || {
                        let mut sum = 0;
                        for index in 0..size {
                            sum += index;
                        }
                        sum
                    });
                    let value = lazy.force();
                    black_box(*value)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_lazy_cached(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("lazy_cached");

    // Pre-create and force the lazy value, then benchmark cached access
    let lazy = Lazy::new(|| {
        let mut sum = 0;
        for index in 0..1000 {
            sum += index;
        }
        sum
    });
    // Force initial evaluation
    let _ = lazy.force();

    // Benchmark cached access (hot path)
    group.bench_function("cached_access", |bencher| {
        bencher.iter(|| {
            let value = lazy.force();
            black_box(*value)
        });
    });

    // Compare with direct value access (baseline)
    let direct_value: i32 = (0..1000).sum();
    group.bench_function("direct_access", |bencher| {
        bencher.iter(|| black_box(direct_value));
    });

    group.finish();
}

fn benchmark_lazy_chain(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("lazy_chain");

    // Benchmark chained lazy evaluations
    for chain_length in [2, 5, 10] {
        group.bench_with_input(
            BenchmarkId::new("chain_length", chain_length),
            &chain_length,
            |bencher, &length| {
                bencher.iter(|| {
                    let base = Lazy::new(|| 1);
                    let mut result = *base.force();

                    for _ in 0..length {
                        let lazy = Lazy::new(move || result * 2);
                        result = *lazy.force();
                    }

                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// ConcurrentLazy Benchmarks
// =============================================================================

/// Benchmark comparing initial evaluation overhead between Lazy and ConcurrentLazy
fn benchmark_concurrent_lazy_vs_lazy_initial(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("concurrent_lazy_vs_lazy_initial");

    group.bench_function("Lazy", |bencher| {
        bencher.iter(|| {
            let lazy = Lazy::new(|| {
                let mut sum = 0;
                for index in 0..100 {
                    sum += index;
                }
                sum
            });
            let value = lazy.force();
            black_box(*value)
        });
    });

    group.bench_function("ConcurrentLazy", |bencher| {
        bencher.iter(|| {
            let lazy = ConcurrentLazy::new(|| {
                let mut sum = 0;
                for index in 0..100 {
                    sum += index;
                }
                sum
            });
            let value = lazy.force();
            black_box(*value)
        });
    });

    group.finish();
}

/// Benchmark comparing cached access overhead between Lazy and ConcurrentLazy
fn benchmark_concurrent_lazy_vs_lazy_cached(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("concurrent_lazy_vs_lazy_cached");

    let lazy = Lazy::new(|| {
        let mut sum = 0;
        for index in 0..1000 {
            sum += index;
        }
        sum
    });
    let _ = lazy.force();

    group.bench_function("Lazy", |bencher| {
        bencher.iter(|| {
            let value = lazy.force();
            black_box(*value)
        });
    });

    let concurrent_lazy = ConcurrentLazy::new(|| {
        let mut sum = 0;
        for index in 0..1000 {
            sum += index;
        }
        sum
    });
    let _ = concurrent_lazy.force();

    group.bench_function("ConcurrentLazy", |bencher| {
        bencher.iter(|| {
            let value = concurrent_lazy.force();
            black_box(*value)
        });
    });

    group.finish();
}

/// Benchmark ConcurrentLazy with different computation sizes
fn benchmark_concurrent_lazy_computation_size(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("concurrent_lazy_computation_size");

    for size in [10, 100, 1000] {
        group.bench_with_input(
            BenchmarkId::new("computation_size", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let lazy = ConcurrentLazy::new(move || {
                        let mut sum = 0;
                        for index in 0..size {
                            sum += index;
                        }
                        sum
                    });
                    let value = lazy.force();
                    black_box(*value)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark initialization contention when multiple threads try to initialize simultaneously
fn benchmark_concurrent_lazy_init_contention(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("concurrent_lazy_init_contention");

    for thread_count in [2, 4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::new("thread_count", thread_count),
            &thread_count,
            |bencher, &thread_count| {
                bencher.iter(|| {
                    let lazy = Arc::new(ConcurrentLazy::new(|| {
                        let mut sum = 0;
                        for index in 0..100 {
                            sum += index;
                        }
                        sum
                    }));

                    let handles: Vec<_> = (0..thread_count)
                        .map(|_| {
                            let lazy = Arc::clone(&lazy);
                            thread::spawn(move || *lazy.force())
                        })
                        .collect();

                    for handle in handles {
                        black_box(handle.join().unwrap());
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark cached access from multiple threads (should be lock-free)
fn benchmark_concurrent_lazy_cached_access(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("concurrent_lazy_cached_access");

    for thread_count in [2, 4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::new("thread_count", thread_count),
            &thread_count,
            |bencher, &thread_count| {
                let lazy = Arc::new(ConcurrentLazy::new(|| {
                    let mut sum = 0;
                    for index in 0..1000 {
                        sum += index;
                    }
                    sum
                }));
                // Pre-initialize
                let _ = lazy.force();

                bencher.iter(|| {
                    let handles: Vec<_> = (0..thread_count)
                        .map(|_| {
                            let lazy = Arc::clone(&lazy);
                            thread::spawn(move || *lazy.force())
                        })
                        .collect();

                    for handle in handles {
                        black_box(handle.join().unwrap());
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark thread scalability with increasing thread counts
fn benchmark_concurrent_lazy_thread_scalability(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("concurrent_lazy_thread_scalability");

    for thread_count in [2, 4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::new("thread_count", thread_count),
            &thread_count,
            |bencher, &thread_count| {
                bencher.iter(|| {
                    // Each thread gets its own lazy value to measure pure scaling
                    let handles: Vec<_> = (0..thread_count)
                        .map(|_| {
                            thread::spawn(|| {
                                let lazy = ConcurrentLazy::new(|| {
                                    let mut sum = 0;
                                    for index in 0..100 {
                                        sum += index;
                                    }
                                    sum
                                });
                                *lazy.force()
                            })
                        })
                        .collect();

                    for handle in handles {
                        black_box(handle.join().unwrap());
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark map chain overhead
fn benchmark_concurrent_lazy_map_chain(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("concurrent_lazy_map_chain");

    // Chain length 2
    group.bench_function("chain_length_2", |bencher| {
        bencher.iter(|| {
            let lazy = ConcurrentLazy::new(|| 1).map(|x| x * 2).map(|x| x * 2);
            let value = lazy.force();
            black_box(*value)
        });
    });

    // Chain length 5
    group.bench_function("chain_length_5", |bencher| {
        bencher.iter(|| {
            let lazy = ConcurrentLazy::new(|| 1)
                .map(|x| x * 2)
                .map(|x| x * 2)
                .map(|x| x * 2)
                .map(|x| x * 2)
                .map(|x| x * 2);
            let value = lazy.force();
            black_box(*value)
        });
    });

    // Chain length 10
    group.bench_function("chain_length_10", |bencher| {
        bencher.iter(|| {
            let lazy = ConcurrentLazy::new(|| 1)
                .map(|x| x * 2)
                .map(|x| x * 2)
                .map(|x| x * 2)
                .map(|x| x * 2)
                .map(|x| x * 2)
                .map(|x| x * 2)
                .map(|x| x * 2)
                .map(|x| x * 2)
                .map(|x| x * 2)
                .map(|x| x * 2);
            let value = lazy.force();
            black_box(*value)
        });
    });

    group.finish();
}

/// Benchmark flat_map chain overhead
fn benchmark_concurrent_lazy_flat_map_chain(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("concurrent_lazy_flat_map_chain");

    // Chain length 2
    group.bench_function("chain_length_2", |bencher| {
        bencher.iter(|| {
            let lazy = ConcurrentLazy::new(|| 1)
                .flat_map(|x| ConcurrentLazy::new(move || x * 2))
                .flat_map(|x| ConcurrentLazy::new(move || x * 2));
            let value = lazy.force();
            black_box(*value)
        });
    });

    // Chain length 5
    group.bench_function("chain_length_5", |bencher| {
        bencher.iter(|| {
            let lazy = ConcurrentLazy::new(|| 1)
                .flat_map(|x| ConcurrentLazy::new(move || x * 2))
                .flat_map(|x| ConcurrentLazy::new(move || x * 2))
                .flat_map(|x| ConcurrentLazy::new(move || x * 2))
                .flat_map(|x| ConcurrentLazy::new(move || x * 2))
                .flat_map(|x| ConcurrentLazy::new(move || x * 2));
            let value = lazy.force();
            black_box(*value)
        });
    });

    // Chain length 10
    group.bench_function("chain_length_10", |bencher| {
        bencher.iter(|| {
            let lazy = ConcurrentLazy::new(|| 1)
                .flat_map(|x| ConcurrentLazy::new(move || x * 2))
                .flat_map(|x| ConcurrentLazy::new(move || x * 2))
                .flat_map(|x| ConcurrentLazy::new(move || x * 2))
                .flat_map(|x| ConcurrentLazy::new(move || x * 2))
                .flat_map(|x| ConcurrentLazy::new(move || x * 2))
                .flat_map(|x| ConcurrentLazy::new(move || x * 2))
                .flat_map(|x| ConcurrentLazy::new(move || x * 2))
                .flat_map(|x| ConcurrentLazy::new(move || x * 2))
                .flat_map(|x| ConcurrentLazy::new(move || x * 2))
                .flat_map(|x| ConcurrentLazy::new(move || x * 2));
            let value = lazy.force();
            black_box(*value)
        });
    });

    group.finish();
}

/// Benchmark zip and zip_with operations
fn benchmark_concurrent_lazy_zip_operations(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("concurrent_lazy_zip_operations");

    group.bench_function("zip", |bencher| {
        bencher.iter(|| {
            let lazy1 = ConcurrentLazy::new(|| 21);
            let lazy2 = ConcurrentLazy::new(|| 21);
            let zipped = lazy1.zip(lazy2);
            let value = zipped.force();
            black_box(*value)
        });
    });

    group.bench_function("zip_with", |bencher| {
        bencher.iter(|| {
            let lazy1 = ConcurrentLazy::new(|| 21);
            let lazy2 = ConcurrentLazy::new(|| 21);
            let combined = lazy1.zip_with(lazy2, |a, b| a + b);
            let value = combined.force();
            black_box(*value)
        });
    });

    group.finish();
}

/// Benchmark for Lazy/ConcurrentLazy hot path performance.
///
/// This benchmark measures the performance of `force()` on already-initialized
/// lazy values, isolating the STATE_READY fast path which should have zero
/// allocations.
///
/// # Purpose
///
/// - `force()` on cached values performs zero heap allocations
/// - Performance is comparable to direct value access (baseline)
///
/// # Profiling Results
///
/// Performance improvement has been confirmed through profiling results:
///
/// | Metric | Before (f6a64a71) | After (3b56e3a6) | Improvement |
/// |--------|-------------------|------------------|-------------|
/// | malloc | 6,171,514,377 | 4,878,635,776 | 21% reduction |
/// | Lazy::force | 4,599,799,274 | 4,046,138,306 | 12% reduction |
///
/// Evidence files:
/// - `benches/results/before/criterion-profiling-all-before/criterion-profiling-f6a64a719b721328c34aeaa3d8dcf995cdc38900-control_bench/top_functions.txt`
/// - `benches/results/after/criterion-profiling-all-3b56e3a624aa888db0a671b1db529c761103f6e4/criterion-profiling-3b56e3a624aa888db0a671b1db529c761103f6e4-control_bench/top_functions.txt`
///
/// The improvement from OnceLock-based to AtomicU8+MaybeUninit design has been
/// achieved. This benchmark guards against performance regressions.
///
/// # Allocation Verification
///
/// For detailed allocation analysis, use memory profiling tools:
/// ```sh
/// # Run benchmark with cargo bench
/// cargo bench --bench control_bench -- lazy_force_hot_path
///
/// # Linux: Analyze heap allocations with valgrind massif
/// valgrind --tool=massif cargo bench --bench control_bench -- lazy_force_hot_path --profile-time 5
/// ms_print massif.out.*
///
/// # Linux: Alternative - use heaptrack for allocation tracking
/// heaptrack cargo bench --bench control_bench -- lazy_force_hot_path --profile-time 5
/// heaptrack_gui heaptrack.*.zst
///
/// # Linux: CPU profiling with perf (for hotspot analysis, not allocations)
/// perf record -g -- cargo bench --bench control_bench -- lazy_force_hot_path --profile-time 5
/// perf report
///
/// # macOS: Use Instruments Allocations template for heap analysis
/// # macOS: Use Instruments Time Profiler for CPU hotspot analysis
/// ```
///
/// Expected: For this benchmark (`lazy_force_hot_path`), malloc/cfree should NOT
/// appear in top functions when accessing cached values.
///
/// # Baseline Comparison
///
/// - Lazy_cached should be within 2x of direct_access
/// - ConcurrentLazy_cached should be within 3x of direct_access (due to Acquire load)
fn benchmark_lazy_force_hot_path(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("lazy_force_hot_path");

    // Use consistent iteration count with bench_force_cached for comparability
    const ITERATIONS: u64 = 10_000_000;

    // 1. Baseline: direct value access (no indirection, no atomic)
    // This establishes the absolute minimum possible latency.
    let direct_value = 42i64;
    group.bench_function("direct_access", |bencher| {
        bencher.iter(|| {
            for _ in 0..ITERATIONS {
                // Prevent loop elimination and constant propagation
                black_box(black_box(direct_value));
            }
        })
    });

    // 2. Lazy STATE_READY path
    // Expected: 1 Acquire load + pointer dereference
    let lazy = Lazy::new(|| 42i64);
    let _ = lazy.force(); // Complete initialization outside measurement

    group.bench_function("Lazy_cached", |bencher| {
        bencher.iter(|| {
            for _ in 0..ITERATIONS {
                // Wrap lazy reference in black_box to prevent hoisting
                let value = black_box(&lazy).force();
                black_box(*value);
            }
        })
    });

    // 3. ConcurrentLazy STATE_READY path
    // Expected: 1 Acquire load + pointer dereference (same as Lazy for cached case)
    let concurrent_lazy = ConcurrentLazy::new(|| 42i64);
    let _ = concurrent_lazy.force();

    group.bench_function("ConcurrentLazy_cached", |bencher| {
        bencher.iter(|| {
            for _ in 0..ITERATIONS {
                let value = black_box(&concurrent_lazy).force();
                black_box(*value);
            }
        })
    });

    group.finish();
}

/// Benchmark for force() on cached values (p95 measurement).
///
/// This benchmark measures the performance of accessing already-initialized
/// lazy values with 1e7 (10 million) iterations per sample for p95 measurement.
/// Target: < 20ns per force() call.
///
/// # Viewing p95 Results
///
/// Criterion collects p95 data but does not display it in console output.
/// To view p95 measurements, open the HTML report after running:
/// ```sh
/// cargo bench --bench control_bench -- --save-baseline latest
/// open target/criterion/force_cached_p95/report/index.html
/// ```
///
/// The HTML report includes percentile distribution (p5, p25, p50, p75, p95).
fn bench_force_cached(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("force_cached_p95");
    group.sample_size(1000); // Large sample for accurate p95

    // Lazy cached access - 1e7 iterations
    let lazy = Lazy::new(|| 42i64);
    let _ = lazy.force();

    group.bench_function("Lazy_1e7_cached", |bencher| {
        bencher.iter(|| {
            for _ in 0..10_000_000 {
                // Wrap lazy reference in black_box to prevent hoisting
                let value = black_box(&lazy).force();
                black_box(*value);
            }
        })
    });

    // ConcurrentLazy cached access - 1e7 iterations
    let concurrent_lazy = ConcurrentLazy::new(|| 42i64);
    let _ = concurrent_lazy.force();

    group.bench_function("ConcurrentLazy_1e7_cached", |bencher| {
        bencher.iter(|| {
            for _ in 0..10_000_000 {
                let value = black_box(&concurrent_lazy).force();
                black_box(*value);
            }
        })
    });

    group.finish();
}

/// Benchmark for concurrent initialization contention.
///
/// This benchmark measures ConcurrentLazy performance under high contention
/// with 16 and 32 threads all trying to initialize simultaneously.
/// The benchmark should complete without warnings.
fn bench_concurrent_contention(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("concurrent_contention");
    group.measurement_time(std::time::Duration::from_secs(10));

    // 16 threads contention
    group.bench_function("16_threads", |bencher| {
        bencher.iter(|| {
            let lazy = Arc::new(ConcurrentLazy::new(|| 42i64));
            let handles: Vec<_> = (0..16)
                .map(|_| {
                    let l = Arc::clone(&lazy);
                    thread::spawn(move || *l.force())
                })
                .collect();
            for h in handles {
                black_box(h.join().unwrap());
            }
        })
    });

    // 32 threads contention
    group.bench_function("32_threads", |bencher| {
        bencher.iter(|| {
            let lazy = Arc::new(ConcurrentLazy::new(|| 42i64));
            let handles: Vec<_> = (0..32)
                .map(|_| {
                    let l = Arc::clone(&lazy);
                    thread::spawn(move || *l.force())
                })
                .collect();
            for h in handles {
                black_box(h.join().unwrap());
            }
        })
    });

    group.finish();
}

// =============================================================================
// Trampoline Benchmarks
// =============================================================================

/// Helper function: factorial using trampoline
fn factorial_trampoline(number: u64) -> Trampoline<u64> {
    factorial_helper(number, 1)
}

fn factorial_helper(number: u64, accumulator: u64) -> Trampoline<u64> {
    if number <= 1 {
        Trampoline::done(accumulator)
    } else {
        Trampoline::suspend(move || factorial_helper(number - 1, number * accumulator))
    }
}

/// Helper function: sum using trampoline
fn sum_trampoline(number: u64) -> Trampoline<u64> {
    sum_helper(number, 0)
}

fn sum_helper(number: u64, accumulator: u64) -> Trampoline<u64> {
    if number == 0 {
        Trampoline::done(accumulator)
    } else {
        Trampoline::suspend(move || sum_helper(number - 1, accumulator + number))
    }
}

/// Helper function: direct recursive factorial (for comparison, only safe for small numbers)
fn factorial_direct(number: u64) -> u64 {
    if number <= 1 {
        1
    } else {
        number * factorial_direct(number - 1)
    }
}

/// Helper function: iterative sum (for comparison)
fn sum_iterative(number: u64) -> u64 {
    (0..=number).sum()
}

fn benchmark_trampoline_shallow(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("trampoline_shallow");

    let depth = 100;

    // Trampoline sum (shallow recursion)
    group.bench_function("Trampoline", |bencher| {
        bencher.iter(|| {
            let result = sum_trampoline(black_box(depth)).run();
            black_box(result)
        });
    });

    // Iterative sum (baseline)
    group.bench_function("Iterative", |bencher| {
        bencher.iter(|| {
            let result = sum_iterative(black_box(depth));
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_trampoline_deep(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("trampoline_deep");

    let depth = 10000;

    // Trampoline sum (deep recursion)
    group.bench_function("Trampoline", |bencher| {
        bencher.iter(|| {
            let result = sum_trampoline(black_box(depth)).run();
            black_box(result)
        });
    });

    // Iterative sum (baseline)
    group.bench_function("Iterative", |bencher| {
        bencher.iter(|| {
            let result = sum_iterative(black_box(depth));
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_trampoline_very_deep(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("trampoline_very_deep");

    let depth = 100_000;

    // Trampoline sum (very deep recursion - would stack overflow without trampoline)
    group.bench_function("Trampoline", |bencher| {
        bencher.iter(|| {
            let result = sum_trampoline(black_box(depth)).run();
            black_box(result)
        });
    });

    // Iterative sum (baseline)
    group.bench_function("Iterative", |bencher| {
        bencher.iter(|| {
            let result = sum_iterative(black_box(depth));
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_trampoline_factorial(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("trampoline_factorial");

    // Small factorial (direct recursion is safe)
    let small_number = 20;

    group.bench_function("Trampoline_small", |bencher| {
        bencher.iter(|| {
            let result = factorial_trampoline(black_box(small_number)).run();
            black_box(result)
        });
    });

    group.bench_function("Direct_small", |bencher| {
        bencher.iter(|| {
            let result = factorial_direct(black_box(small_number));
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_trampoline_map_flatmap(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("trampoline_map_flatmap");

    // Benchmark map operation
    group.bench_function("map", |bencher| {
        bencher.iter(|| {
            let trampoline = Trampoline::done(42);
            let result = trampoline.map(|x| x * 2).run();
            black_box(result)
        });
    });

    // Benchmark flat_map operation
    group.bench_function("flat_map", |bencher| {
        bencher.iter(|| {
            let trampoline = Trampoline::done(42);
            let result = trampoline.flat_map(|x| Trampoline::done(x * 2)).run();
            black_box(result)
        });
    });

    // Benchmark chained flat_maps
    group.bench_function("flat_map_chain", |bencher| {
        bencher.iter(|| {
            let result = Trampoline::done(1)
                .flat_map(|x| Trampoline::done(x + 1))
                .flat_map(|x| Trampoline::done(x * 2))
                .flat_map(|x| Trampoline::done(x + 10))
                .run();
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
    // Lazy benchmarks
    benchmark_lazy_force,
    benchmark_lazy_cached,
    benchmark_lazy_chain,
    // ConcurrentLazy benchmarks
    benchmark_concurrent_lazy_vs_lazy_initial,
    benchmark_concurrent_lazy_vs_lazy_cached,
    benchmark_concurrent_lazy_computation_size,
    benchmark_concurrent_lazy_init_contention,
    benchmark_concurrent_lazy_cached_access,
    benchmark_concurrent_lazy_thread_scalability,
    benchmark_concurrent_lazy_map_chain,
    benchmark_concurrent_lazy_flat_map_chain,
    benchmark_concurrent_lazy_zip_operations,
    // Requirements-specified benchmarks (Issue #224)
    benchmark_lazy_force_hot_path,
    bench_force_cached,
    bench_concurrent_contention,
    // Trampoline benchmarks
    benchmark_trampoline_shallow,
    benchmark_trampoline_deep,
    benchmark_trampoline_very_deep,
    benchmark_trampoline_factorial,
    benchmark_trampoline_map_flatmap
);

criterion_main!(benches);
