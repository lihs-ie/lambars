//! Benchmark for the `for_!` macro.
//!
//! Compares the performance of the `for_!` macro against
//! hand-written `flat_map` chains.

#![cfg(feature = "compose")]

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lambars::for_;
use std::hint::black_box;

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
                    let result: Vec<i32> =
                        data.clone().into_iter().map(|x| black_box(x * 2)).collect();
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
                        let second_inner = second_clone.clone();
                        let third_for_y = third_clone.clone();
                        y <= second_inner;
                        let third_inner = third_for_y.clone();
                        z <= third_inner;
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
                        .map(black_box)
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
// Large Scale 100k Elements Benchmark
// =============================================================================

fn benchmark_large_scale_100k(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("for_large_scale_100k");
    group.sample_size(50);

    let data: Vec<i32> = (0..100_000).collect();

    // for_! macro
    group.bench_function("for_macro_single_100k", |bencher| {
        bencher.iter(|| {
            let result = for_! {
                x <= black_box(data.clone());
                yield black_box(x * 2)
            };
            black_box(result)
        });
    });

    // Hand-written map
    group.bench_function("map_collect_100k", |bencher| {
        bencher.iter(|| {
            let result: Vec<i32> = black_box(data.clone())
                .into_iter()
                .map(|x| black_box(x * 2))
                .collect();
            black_box(result)
        });
    });

    group.finish();
}

// =============================================================================
// Nested Two Levels with Guard (100 x 1000 = 100k candidates)
// =============================================================================

fn benchmark_nested_with_guard_100k(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("for_nested_guard_100k");
    group.sample_size(30);

    let outer: Vec<i32> = (0..100).collect();
    let inner: Vec<i32> = (0..1000).collect();

    // for_! macro with guard
    group.bench_function("for_macro", |bencher| {
        let inner_clone = inner.clone();
        bencher.iter(|| {
            let inner_for_iter = inner_clone.clone();
            let result = for_! {
                x <= black_box(outer.clone());
                let inner_inner = inner_for_iter.clone();
                y <= black_box(inner_inner);
                if (x + y) % 3 == 0;
                yield black_box(x + y)
            };
            black_box(result)
        });
    });

    // Hand-written flat_map + filter
    group.bench_function("flat_map_filter", |bencher| {
        let inner_clone = inner.clone();
        bencher.iter(|| {
            let result: Vec<i32> = black_box(outer.clone())
                .into_iter()
                .flat_map(|x| {
                    black_box(inner_clone.clone())
                        .into_iter()
                        .filter(move |&y| (x + y) % 3 == 0)
                        .map(move |y| black_box(x + y))
                })
                .collect();
            black_box(result)
        });
    });

    group.finish();
}

// =============================================================================
// Three-Level Nested with Pattern Guard (20 x 25 x 100 = 50k candidates)
// =============================================================================

fn benchmark_nested_three_pattern_guard(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("for_nested_3_pattern_guard");
    group.sample_size(30);

    let first: Vec<i32> = (0..20).collect();
    let second: Vec<Option<i32>> = (0..50)
        .map(|i| if i % 2 == 0 { Some(i) } else { None })
        .collect(); // 25 Some values
    let third: Vec<i32> = (0..100).collect();

    // for_! macro with pattern guard
    group.bench_function("for_macro", |bencher| {
        let second_clone = second.clone();
        let third_clone = third.clone();
        bencher.iter(|| {
            let second_for_iter = second_clone.clone();
            let third_for_iter = third_clone.clone();
            let result = for_! {
                x <= black_box(first.clone());
                let second_inner = second_for_iter.clone();
                let third_for_y = third_for_iter.clone();
                opt <= black_box(second_inner);
                if let Some(y) = opt;
                let third_inner = third_for_y.clone();
                z <= black_box(third_inner);
                yield black_box(x + y + z)
            };
            black_box(result)
        });
    });

    group.finish();
}

// =============================================================================
// Concat (Flatten) Benchmark (1000 x 100 = 100k)
// =============================================================================

fn benchmark_concat_100k(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("for_concat_100k");
    group.sample_size(30);

    let nested: Vec<Vec<i32>> = (0..1000)
        .map(|i| ((i * 100)..((i + 1) * 100)).collect())
        .collect();

    // for_! macro concat
    group.bench_function("for_macro", |bencher| {
        bencher.iter(|| {
            let result = for_! {
                inner <= black_box(nested.clone());
                x <= inner;
                yield black_box(x)
            };
            black_box(result)
        });
    });

    // Hand-written flatten
    group.bench_function("flatten", |bencher| {
        bencher.iter(|| {
            let result: Vec<i32> = black_box(nested.clone())
                .into_iter()
                .flatten()
                .map(black_box)
                .collect();
            black_box(result)
        });
    });

    group.finish();
}

// =============================================================================
// Deep Chain (4 levels with guard + let)
// =============================================================================

fn benchmark_deep_chain(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("for_deep_chain");
    group.sample_size(50);

    let data: Vec<i32> = (0..1000).collect();

    // for_! macro with 4 levels of nesting (guard + let)
    group.bench_function("for_macro_4_levels", |bencher| {
        bencher.iter(|| {
            let result = for_! {
                a <= black_box(data.clone());
                if a % 2 == 0;
                let b = a * 2;
                if b < 1000;
                let c = b + 1;
                yield black_box(c)
            };
            black_box(result)
        });
    });

    // Hand-written equivalent
    group.bench_function("hand_written", |bencher| {
        bencher.iter(|| {
            let result: Vec<i32> = black_box(data.clone())
                .into_iter()
                .filter(|&a| a % 2 == 0)
                .map(|a| a * 2)
                .filter(|&b| b < 1000)
                .map(|b| black_box(b + 1))
                .collect();
            black_box(result)
        });
    });

    group.finish();
}

// =============================================================================
// Three-Level Nested 100k (10 x 100 x 100 = 100k elements)
// =============================================================================

fn benchmark_nested_three_100k(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("for_nested_3_100k");
    group.sample_size(20);

    let first: Vec<i32> = (0..10).collect();
    let second: Vec<i32> = (0..100).collect();
    let third: Vec<i32> = (0..100).collect();

    // for_! macro
    group.bench_function("for_macro", |bencher| {
        let second_clone = second.clone();
        let third_clone = third.clone();
        bencher.iter(|| {
            let second_for_iter = second_clone.clone();
            let third_for_iter = third_clone.clone();
            let result = for_! {
                x <= black_box(first.clone());
                let second_inner = second_for_iter.clone();
                let third_for_y = third_for_iter.clone();
                y <= black_box(second_inner);
                let third_inner = third_for_y.clone();
                z <= black_box(third_inner);
                yield black_box(x + y + z)
            };
            black_box(result)
        });
    });

    // Hand-written flat_map
    group.bench_function("flat_map", |bencher| {
        let second_clone = second.clone();
        let third_clone = third.clone();
        bencher.iter(|| {
            let result: Vec<i32> = black_box(first.clone())
                .into_iter()
                .flat_map(|x| {
                    let third_inner = third_clone.clone();
                    second_clone.clone().into_iter().flat_map(move |y| {
                        third_inner
                            .clone()
                            .into_iter()
                            .map(move |z| black_box(x + y + z))
                    })
                })
                .collect();
            black_box(result)
        });
    });

    group.finish();
}

// =============================================================================
// Three-Level Nested with Pattern Guard 100k (20 x 50 x 100 = 100k candidates)
// =============================================================================

fn benchmark_nested_three_pattern_guard_100k(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("for_nested_3_pattern_guard_100k");
    group.sample_size(20);

    let first: Vec<i32> = (0..20).collect();
    let second: Vec<Option<i32>> = (0..100)
        .map(|i| if i % 2 == 0 { Some(i) } else { None })
        .collect(); // 50 Some values
    let third: Vec<i32> = (0..100).collect();

    // for_! macro with pattern guard
    group.bench_function("for_macro", |bencher| {
        let second_clone = second.clone();
        let third_clone = third.clone();
        bencher.iter(|| {
            let second_for_iter = second_clone.clone();
            let third_for_iter = third_clone.clone();
            let result = for_! {
                x <= black_box(first.clone());
                let second_inner = second_for_iter.clone();
                let third_for_y = third_for_iter.clone();
                opt <= black_box(second_inner);
                if let Some(y) = opt;
                let third_inner = third_for_y.clone();
                z <= black_box(third_inner);
                yield black_box(x + y + z)
            };
            black_box(result)
        });
    });

    group.finish();
}

// =============================================================================
// Three-Level Nested with Multiple Guards 100k
// =============================================================================

fn benchmark_nested_three_multi_guard_100k(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("for_nested_3_multi_guard_100k");
    group.sample_size(20);

    let first: Vec<i32> = (0..50).collect();
    let second: Vec<i32> = (0..100).collect();
    let third: Vec<i32> = (0..100).collect();

    // for_! macro with multiple guards
    group.bench_function("for_macro", |bencher| {
        let second_clone = second.clone();
        let third_clone = third.clone();
        bencher.iter(|| {
            let second_for_iter = second_clone.clone();
            let third_for_iter = third_clone.clone();
            let result = for_! {
                x <= black_box(first.clone());
                if x % 2 == 0;
                let second_inner = second_for_iter.clone();
                let third_for_y = third_for_iter.clone();
                y <= black_box(second_inner);
                let third_inner = third_for_y.clone();
                z <= black_box(third_inner);
                if (x + y + z) % 3 == 0;
                yield black_box(x + y + z)
            };
            black_box(result)
        });
    });

    // Hand-written equivalent
    group.bench_function("hand_written", |bencher| {
        let second_clone = second.clone();
        let third_clone = third.clone();
        bencher.iter(|| {
            let result: Vec<i32> = black_box(first.clone())
                .into_iter()
                .filter(|&x| x % 2 == 0)
                .flat_map(|x| {
                    let third_inner = third_clone.clone();
                    second_clone.clone().into_iter().flat_map(move |y| {
                        third_inner
                            .clone()
                            .into_iter()
                            .filter(move |&z| (x + y + z) % 3 == 0)
                            .map(move |z| black_box(x + y + z))
                    })
                })
                .collect();
            black_box(result)
        });
    });

    group.finish();
}

// =============================================================================
// for_macro_alloc Benchmark
// =============================================================================

/// Compare allocation-optimized `for_!` macro performance against
/// `Vec::with_capacity` (optimal baseline) and `map().collect()`.
fn benchmark_for_macro_alloc(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("for_macro_alloc");

    for size in [100, 1_000, 10_000, 100_000] {
        let data: Vec<i32> = (0..size).collect();

        // for_! macro (optimized)
        group.bench_function(BenchmarkId::new("for_macro", size), |bencher| {
            bencher.iter(|| {
                let result = for_! {
                    x <= data.clone();
                    yield black_box(x * 2)
                };
                black_box(result)
            });
        });

        // Hand-written Vec::with_capacity (optimal baseline)
        group.bench_function(BenchmarkId::new("vec_with_capacity", size), |bencher| {
            bencher.iter(|| {
                let data_clone = data.clone();
                let mut result = Vec::with_capacity(size as usize);
                for x in data_clone.iter() {
                    result.push(black_box(*x * 2));
                }
                black_box(result)
            });
        });

        // Hand-written map().collect() (comparison)
        group.bench_function(BenchmarkId::new("map_collect", size), |bencher| {
            bencher.iter(|| {
                let result: Vec<_> = data.clone().into_iter().map(|x| black_box(x * 2)).collect();
                black_box(result)
            });
        });
    }

    group.finish();
}

// =============================================================================
// for_macro_alloc_nested Benchmark
// =============================================================================

/// Compare 2-level nested `for_!` macro performance against `flat_map` chain.
fn benchmark_for_macro_alloc_nested(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("for_macro_alloc_nested");

    for size in [10, 50, 100] {
        let outer: Vec<i32> = (0..size).collect();
        let inner: Vec<i32> = (0..size).collect();

        group.bench_function(
            BenchmarkId::new("for_macro_2level", size * size),
            |bencher| {
                let inner_clone = inner.clone();
                bencher.iter(|| {
                    let inner_for_iter = inner_clone.clone();
                    let result = for_! {
                        x <= outer.clone();
                        y <= inner_for_iter.clone();
                        yield black_box(x + y)
                    };
                    black_box(result)
                });
            },
        );

        group.bench_function(
            BenchmarkId::new("flat_map_2level", size * size),
            |bencher| {
                let inner_clone = inner.clone();
                bencher.iter(|| {
                    let result: Vec<_> = outer
                        .clone()
                        .into_iter()
                        .flat_map(|x| {
                            inner_clone
                                .clone()
                                .into_iter()
                                .map(move |y| black_box(x + y))
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
    benchmark_filter_simulation,
    benchmark_large_scale_100k,
    benchmark_nested_with_guard_100k,
    benchmark_nested_three_pattern_guard,
    benchmark_concat_100k,
    benchmark_deep_chain,
    benchmark_nested_three_100k,
    benchmark_nested_three_pattern_guard_100k,
    benchmark_nested_three_multi_guard_100k,
    benchmark_for_macro_alloc,
    benchmark_for_macro_alloc_nested
);

criterion_main!(benches);
