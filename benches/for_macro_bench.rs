//! Benchmark for the `for_!` macro.
//!
//! Compares the performance of the `for_!` macro against
//! hand-written `flat_map` chains.

#![cfg(feature = "compose")]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use lambars::for_;

// =============================================================================
// Single Iteration Benchmark
// =============================================================================

fn benchmark_single_iteration(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("for_single_iteration");

    for size in [100, 1_000, 10_000] {
        let data: Vec<i32> = (0..size).collect();

        // for_! macro
        group.bench_with_input(
            BenchmarkId::new("for_macro", size),
            &data,
            |bencher, data| {
                bencher.iter(|| {
                    let result = for_! {
                        x <= data.clone();
                        yield black_box(x * 2)
                    };
                    black_box(result)
                });
            },
        );

        // Hand-written map
        group.bench_with_input(
            BenchmarkId::new("map_collect", size),
            &data,
            |bencher, data| {
                bencher.iter(|| {
                    let result: Vec<i32> = data.clone().into_iter().map(|x| black_box(x * 2)).collect();
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

fn benchmark_nested_two_levels(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("for_nested_two_levels");

    for size in [10, 50, 100] {
        let outer: Vec<i32> = (0..size).collect();
        let inner: Vec<i32> = (0..size).collect();

        // for_! macro
        group.bench_with_input(
            BenchmarkId::new("for_macro", size * size),
            &(outer.clone(), inner.clone()),
            |bencher, (outer, inner)| {
                bencher.iter(|| {
                    let inner_clone = inner.clone();
                    let result = for_! {
                        x <= outer.clone();
                        y <= inner_clone.clone();
                        yield black_box(x + y)
                    };
                    black_box(result)
                });
            },
        );

        // Hand-written flat_map
        group.bench_with_input(
            BenchmarkId::new("flat_map", size * size),
            &(outer.clone(), inner.clone()),
            |bencher, (outer, inner)| {
                bencher.iter(|| {
                    let result: Vec<i32> = outer
                        .clone()
                        .into_iter()
                        .flat_map(|x| inner.clone().into_iter().map(move |y| black_box(x + y)))
                        .collect();
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Three-Level Nested Iteration Benchmark
// =============================================================================

fn benchmark_nested_three_levels(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("for_nested_three_levels");

    for size in [5, 10, 20] {
        let first: Vec<i32> = (0..size).collect();
        let second: Vec<i32> = (0..size).collect();
        let third: Vec<i32> = (0..size).collect();

        // for_! macro
        group.bench_with_input(
            BenchmarkId::new("for_macro", size * size * size),
            &(first.clone(), second.clone(), third.clone()),
            |bencher, (first, second, third)| {
                bencher.iter(|| {
                    let second_clone = second.clone();
                    let third_clone = third.clone();
                    let result = for_! {
                        x <= first.clone();
                        y <= second_clone.clone();
                        z <= third_clone.clone();
                        yield black_box(x + y + z)
                    };
                    black_box(result)
                });
            },
        );

        // Hand-written flat_map chain
        group.bench_with_input(
            BenchmarkId::new("flat_map", size * size * size),
            &(first.clone(), second.clone(), third.clone()),
            |bencher, (first, second, third)| {
                bencher.iter(|| {
                    let result: Vec<i32> = first
                        .clone()
                        .into_iter()
                        .flat_map(|x| {
                            let third_inner = third.clone();
                            second.clone().into_iter().flat_map(move |y| {
                                third_inner
                                    .clone()
                                    .into_iter()
                                    .map(move |z| black_box(x + y + z))
                            })
                        })
                        .collect();
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Let Binding Benchmark
// =============================================================================

fn benchmark_with_let_binding(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("for_with_let_binding");

    for size in [100, 1_000, 10_000] {
        let data: Vec<i32> = (0..size).collect();

        // for_! macro with let binding
        group.bench_with_input(
            BenchmarkId::new("for_macro", size),
            &data,
            |bencher, data| {
                bencher.iter(|| {
                    let result = for_! {
                        x <= data.clone();
                        let doubled = x * 2;
                        let squared = doubled * doubled;
                        yield black_box(squared)
                    };
                    black_box(result)
                });
            },
        );

        // Hand-written map with same computation
        group.bench_with_input(
            BenchmarkId::new("map_collect", size),
            &data,
            |bencher, data| {
                bencher.iter(|| {
                    let result: Vec<i32> = data
                        .clone()
                        .into_iter()
                        .map(|x| {
                            let doubled = x * 2;
                            let squared = doubled * doubled;
                            black_box(squared)
                        })
                        .collect();
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Filter Simulation Benchmark
// =============================================================================

fn benchmark_filter_simulation(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("for_filter_simulation");

    for size in [100, 1_000, 10_000] {
        let data: Vec<i32> = (0..size).collect();

        // for_! macro with filter simulation (empty vec for false)
        group.bench_with_input(
            BenchmarkId::new("for_macro", size),
            &data,
            |bencher, data| {
                bencher.iter(|| {
                    let result = for_! {
                        x <= data.clone();
                        y <= if x % 2 == 0 { vec![x] } else { vec![] };
                        yield black_box(y)
                    };
                    black_box(result)
                });
            },
        );

        // Hand-written filter + collect
        group.bench_with_input(
            BenchmarkId::new("filter_collect", size),
            &data,
            |bencher, data| {
                bencher.iter(|| {
                    let result: Vec<i32> = data
                        .clone()
                        .into_iter()
                        .filter(|&x| x % 2 == 0)
                        .map(|x| black_box(x))
                        .collect();
                    black_box(result)
                });
            },
        );

        // Hand-written flat_map (same as for_! expansion)
        group.bench_with_input(
            BenchmarkId::new("flat_map", size),
            &data,
            |bencher, data| {
                bencher.iter(|| {
                    let result: Vec<i32> = data
                        .clone()
                        .into_iter()
                        .flat_map(|x| {
                            if x % 2 == 0 {
                                vec![black_box(x)]
                            } else {
                                vec![]
                            }
                        })
                        .collect();
                    black_box(result)
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
    benchmark_single_iteration,
    benchmark_nested_two_levels,
    benchmark_nested_three_levels,
    benchmark_with_let_binding,
    benchmark_filter_simulation
);

criterion_main!(benches);
