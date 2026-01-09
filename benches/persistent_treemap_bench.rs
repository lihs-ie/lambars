//! Benchmark for PersistentTreeMap vs standard BTreeMap.
//!
//! Compares the performance of lambars' PersistentTreeMap against Rust's standard BTreeMap
//! for common operations.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lambars::persistent::PersistentTreeMap;
use std::collections::BTreeMap;
use std::hint::black_box;

// =============================================================================
// insert Benchmark
// =============================================================================

fn benchmark_insert(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("insert");

    for size in [100, 1000, 10000] {
        // PersistentTreeMap insert
        group.bench_with_input(
            BenchmarkId::new("PersistentTreeMap", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut map = PersistentTreeMap::new();
                    for index in 0..size {
                        map = map.insert(black_box(index), black_box(index * 2));
                    }
                    black_box(map)
                });
            },
        );

        // Standard BTreeMap insert
        group.bench_with_input(
            BenchmarkId::new("BTreeMap", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut map = BTreeMap::new();
                    for index in 0..size {
                        map.insert(black_box(index), black_box(index * 2));
                    }
                    black_box(map)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// get Benchmark
// =============================================================================

fn benchmark_get(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("get");

    for size in [100, 1000, 10000] {
        // Prepare data
        let persistent_map: PersistentTreeMap<i32, i32> =
            (0..size).map(|index| (index, index * 2)).collect();
        let standard_map: BTreeMap<i32, i32> = (0..size).map(|index| (index, index * 2)).collect();

        // PersistentTreeMap get
        group.bench_with_input(
            BenchmarkId::new("PersistentTreeMap", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut sum = 0;
                    for key in 0..size {
                        if let Some(&value) = persistent_map.get(&black_box(key)) {
                            sum += value;
                        }
                    }
                    black_box(sum)
                });
            },
        );

        // Standard BTreeMap get
        group.bench_with_input(
            BenchmarkId::new("BTreeMap", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut sum = 0;
                    for key in 0..size {
                        if let Some(&value) = standard_map.get(&black_box(key)) {
                            sum += value;
                        }
                    }
                    black_box(sum)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// range Benchmark
// =============================================================================

fn benchmark_range(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("range");

    for size in [100, 1000, 10000] {
        // Prepare data
        let persistent_map: PersistentTreeMap<i32, i32> =
            (0..size).map(|index| (index, index * 2)).collect();
        let standard_map: BTreeMap<i32, i32> = (0..size).map(|index| (index, index * 2)).collect();

        let range_start = size / 4;
        let range_end = size * 3 / 4;

        // PersistentTreeMap range
        group.bench_with_input(
            BenchmarkId::new("PersistentTreeMap", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let sum: i32 = persistent_map
                        .range(black_box(range_start)..black_box(range_end))
                        .map(|(_, &value)| value)
                        .sum();
                    black_box(sum)
                });
            },
        );

        // Standard BTreeMap range
        group.bench_with_input(BenchmarkId::new("BTreeMap", size), &size, |bencher, _| {
            bencher.iter(|| {
                let sum: i32 = standard_map
                    .range(black_box(range_start)..black_box(range_end))
                    .map(|(_, &value)| value)
                    .sum();
                black_box(sum)
            });
        });
    }

    group.finish();
}

// =============================================================================
// iteration Benchmark
// =============================================================================

fn benchmark_iteration(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("iteration");

    for size in [100, 1000, 10000] {
        // Prepare data
        let persistent_map: PersistentTreeMap<i32, i32> =
            (0..size).map(|index| (index, index * 2)).collect();
        let standard_map: BTreeMap<i32, i32> = (0..size).map(|index| (index, index * 2)).collect();

        // PersistentTreeMap iteration
        group.bench_with_input(
            BenchmarkId::new("PersistentTreeMap", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let sum: i32 = persistent_map.iter().map(|(_, &value)| value).sum();
                    black_box(sum)
                });
            },
        );

        // Standard BTreeMap iteration
        group.bench_with_input(BenchmarkId::new("BTreeMap", size), &size, |bencher, _| {
            bencher.iter(|| {
                let sum: i32 = standard_map.values().sum();
                black_box(sum)
            });
        });
    }

    group.finish();
}

// =============================================================================
// min/max Benchmark
// =============================================================================

fn benchmark_min_max(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("min_max");

    for size in [100, 1000, 10000] {
        // Prepare data
        let persistent_map: PersistentTreeMap<i32, i32> =
            (0..size).map(|index| (index, index * 2)).collect();
        let standard_map: BTreeMap<i32, i32> = (0..size).map(|index| (index, index * 2)).collect();

        // PersistentTreeMap min/max
        group.bench_with_input(
            BenchmarkId::new("PersistentTreeMap", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let min = persistent_map.min();
                    let max = persistent_map.max();
                    black_box((min, max))
                });
            },
        );

        // Standard BTreeMap first_key_value/last_key_value
        group.bench_with_input(BenchmarkId::new("BTreeMap", size), &size, |bencher, _| {
            bencher.iter(|| {
                let min = standard_map.first_key_value();
                let max = standard_map.last_key_value();
                black_box((min, max))
            });
        });
    }

    group.finish();
}

// =============================================================================
// iteration_early_exit Benchmark (Issue #108)
// =============================================================================

fn benchmark_iteration_early_exit(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("iteration_early_exit");

    for size in [1000, 10000, 100000] {
        let persistent_map: PersistentTreeMap<i32, i32> =
            (0..size).map(|index| (index, index * 2)).collect();
        let standard_map: BTreeMap<i32, i32> = (0..size).map(|index| (index, index * 2)).collect();

        for take_count in [1, 10, 100] {
            let label = format!("{}/take_{}", size, take_count);

            group.bench_with_input(
                BenchmarkId::new("PersistentTreeMap", &label),
                &take_count,
                |bencher, &take_count| {
                    bencher.iter(|| {
                        let result: Vec<_> = persistent_map.iter().take(take_count).collect();
                        black_box(result)
                    });
                },
            );

            group.bench_with_input(
                BenchmarkId::new("BTreeMap", &label),
                &take_count,
                |bencher, &take_count| {
                    bencher.iter(|| {
                        let result: Vec<_> = standard_map.iter().take(take_count).collect();
                        black_box(result)
                    });
                },
            );
        }
    }

    group.finish();
}

// =============================================================================
// iteration_first Benchmark (Issue #108)
// =============================================================================

fn benchmark_iteration_first(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("iteration_first");

    for size in [1000, 10000, 100000] {
        let persistent_map: PersistentTreeMap<i32, i32> =
            (0..size).map(|index| (index, index * 2)).collect();
        let standard_map: BTreeMap<i32, i32> = (0..size).map(|index| (index, index * 2)).collect();

        group.bench_with_input(
            BenchmarkId::new("PersistentTreeMap", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let first = persistent_map.iter().next();
                    black_box(first)
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("BTreeMap", size), &size, |bencher, _| {
            bencher.iter(|| {
                let first = standard_map.iter().next();
                black_box(first)
            });
        });
    }

    group.finish();
}

// =============================================================================
// iteration_create Benchmark (Issue #108)
// =============================================================================

fn benchmark_iteration_create(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("iteration_create");

    for size in [1000, 10000, 100000] {
        let persistent_map: PersistentTreeMap<i32, i32> =
            (0..size).map(|index| (index, index * 2)).collect();
        let standard_map: BTreeMap<i32, i32> = (0..size).map(|index| (index, index * 2)).collect();

        group.bench_with_input(
            BenchmarkId::new("PersistentTreeMap", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let iterator = persistent_map.iter();
                    black_box(iterator)
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("BTreeMap", size), &size, |bencher, _| {
            bencher.iter(|| {
                let iterator = standard_map.iter();
                black_box(iterator)
            });
        });
    }

    group.finish();
}

// =============================================================================
// iteration_find Benchmark (Issue #108)
// =============================================================================

fn benchmark_iteration_find(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("iteration_find");

    for size in [1000, 10000, 100000] {
        let persistent_map: PersistentTreeMap<i32, i32> =
            (0..size).map(|index| (index, index * 2)).collect();
        let standard_map: BTreeMap<i32, i32> = (0..size).map(|index| (index, index * 2)).collect();

        let target = size / 2;

        group.bench_with_input(
            BenchmarkId::new("PersistentTreeMap", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let found = persistent_map.iter().find(|(key, _)| **key == target);
                    black_box(found)
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("BTreeMap", size), &size, |bencher, _| {
            bencher.iter(|| {
                let found = standard_map.iter().find(|(key, _)| **key == target);
                black_box(found)
            });
        });
    }

    group.finish();
}

// =============================================================================
// Criterion Group and Main
// =============================================================================

criterion_group!(
    benches,
    benchmark_insert,
    benchmark_get,
    benchmark_range,
    benchmark_iteration,
    benchmark_min_max,
    benchmark_iteration_early_exit,
    benchmark_iteration_first,
    benchmark_iteration_create,
    benchmark_iteration_find
);

criterion_main!(benches);
