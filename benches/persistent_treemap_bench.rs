//! Benchmark for PersistentTreeMap vs standard BTreeMap.
//!
//! Compares the performance of lambars' PersistentTreeMap against Rust's standard BTreeMap
//! for common operations.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use lambars::persistent::PersistentTreeMap;
use std::collections::BTreeMap;

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
// Criterion Group and Main
// =============================================================================

criterion_group!(
    benches,
    benchmark_insert,
    benchmark_get,
    benchmark_range,
    benchmark_iteration,
    benchmark_min_max
);

criterion_main!(benches);
