//! Benchmark for control structures: Lazy and Trampoline.
//!
//! Measures the performance of lambars' control flow abstractions.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use lambars::control::{Lazy, Trampoline};

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
    benchmark_lazy_force,
    benchmark_lazy_cached,
    benchmark_lazy_chain,
    benchmark_trampoline_shallow,
    benchmark_trampoline_deep,
    benchmark_trampoline_very_deep,
    benchmark_trampoline_factorial,
    benchmark_trampoline_map_flatmap
);

criterion_main!(benches);
