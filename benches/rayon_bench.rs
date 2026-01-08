//! Benchmark for parallel iteration with rayon.
//!
//! Compares the performance of parallel iteration (`par_iter()`) against
//! sequential iteration (`iter()`) for lambars' persistent data structures.
//!
//! Requires the `rayon` feature to be enabled.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lambars::persistent::{
    PersistentHashMap, PersistentHashSet, PersistentList, PersistentTreeMap, PersistentVector,
};
use rayon::prelude::*;
use std::hint::black_box;

// =============================================================================
// PersistentVector Benchmarks
// =============================================================================

fn benchmark_vector_sum(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("vector_sum");

    for size in [1000, 10000, 100000, 1000000] {
        let vector: PersistentVector<i64> = (0..size).collect();

        // Sequential sum
        group.bench_with_input(BenchmarkId::new("sequential", size), &size, |bencher, _| {
            bencher.iter(|| {
                let sum: i64 = vector.iter().sum();
                black_box(sum)
            });
        });

        // Parallel sum
        group.bench_with_input(BenchmarkId::new("parallel", size), &size, |bencher, _| {
            bencher.iter(|| {
                let sum: i64 = vector.par_iter().sum();
                black_box(sum)
            });
        });
    }

    group.finish();
}

fn benchmark_vector_map(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("vector_map");

    for size in [1000, 10000, 100000] {
        let vector: PersistentVector<i64> = (0..size).collect();

        // Sequential map
        group.bench_with_input(BenchmarkId::new("sequential", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<i64> = vector.iter().map(|x| x * 2 + 1).collect();
                black_box(result)
            });
        });

        // Parallel map
        group.bench_with_input(BenchmarkId::new("parallel", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<i64> = vector.par_iter().map(|x| x * 2 + 1).collect();
                black_box(result)
            });
        });
    }

    group.finish();
}

fn benchmark_vector_filter(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("vector_filter");

    for size in [1000, 10000, 100000] {
        let vector: PersistentVector<i64> = (0..size).collect();

        // Sequential filter
        group.bench_with_input(BenchmarkId::new("sequential", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<i64> = vector.iter().filter(|x| *x % 2 == 0).copied().collect();
                black_box(result)
            });
        });

        // Parallel filter
        group.bench_with_input(BenchmarkId::new("parallel", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<i64> = vector.par_iter().filter(|x| *x % 2 == 0).copied().collect();
                black_box(result)
            });
        });
    }

    group.finish();
}

fn benchmark_vector_map_reduce(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("vector_map_reduce");

    for size in [1000, 10000, 100000, 1000000] {
        let vector: PersistentVector<i64> = (0..size).collect();

        // Sequential map + reduce
        group.bench_with_input(BenchmarkId::new("sequential", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: i64 = vector.iter().map(|x| x * x).sum();
                black_box(result)
            });
        });

        // Parallel map + reduce
        group.bench_with_input(BenchmarkId::new("parallel", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: i64 = vector.par_iter().map(|x| x * x).sum();
                black_box(result)
            });
        });
    }

    group.finish();
}

// =============================================================================
// PersistentHashMap Benchmarks
// =============================================================================

fn benchmark_hashmap_sum_values(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("hashmap_sum_values");

    for size in [1000, 10000, 100000] {
        let map: PersistentHashMap<i64, i64> = (0..size).map(|i| (i, i * 2)).collect();

        // Sequential sum
        group.bench_with_input(BenchmarkId::new("sequential", size), &size, |bencher, _| {
            bencher.iter(|| {
                let sum: i64 = map.iter().map(|(_, v)| v).sum();
                black_box(sum)
            });
        });

        // Parallel sum
        group.bench_with_input(BenchmarkId::new("parallel", size), &size, |bencher, _| {
            bencher.iter(|| {
                let sum: i64 = map.par_iter().map(|(_, v)| v).sum();
                black_box(sum)
            });
        });
    }

    group.finish();
}

fn benchmark_hashmap_filter(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("hashmap_filter");

    for size in [1000, 10000, 100000] {
        let map: PersistentHashMap<i64, i64> = (0..size).map(|i| (i, i * 2)).collect();

        // Sequential filter
        group.bench_with_input(BenchmarkId::new("sequential", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<(&i64, &i64)> = map.iter().filter(|(k, _)| *k % 2 == 0).collect();
                black_box(result)
            });
        });

        // Parallel filter
        group.bench_with_input(BenchmarkId::new("parallel", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<(&i64, &i64)> =
                    map.par_iter().filter(|(k, _)| *k % 2 == 0).collect();
                black_box(result)
            });
        });
    }

    group.finish();
}

// =============================================================================
// PersistentHashSet Benchmarks
// =============================================================================

fn benchmark_hashset_sum(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("hashset_sum");

    for size in [1000, 10000, 100000] {
        let set: PersistentHashSet<i64> = (0..size).collect();

        // Sequential sum
        group.bench_with_input(BenchmarkId::new("sequential", size), &size, |bencher, _| {
            bencher.iter(|| {
                let sum: i64 = set.iter().sum();
                black_box(sum)
            });
        });

        // Parallel sum
        group.bench_with_input(BenchmarkId::new("parallel", size), &size, |bencher, _| {
            bencher.iter(|| {
                let sum: i64 = set.par_iter().sum();
                black_box(sum)
            });
        });
    }

    group.finish();
}

// =============================================================================
// PersistentList Benchmarks
// =============================================================================

fn benchmark_list_sum(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("list_sum");

    for size in [1000, 10000, 100000] {
        let list: PersistentList<i64> = (0..size).collect();

        // Sequential sum
        group.bench_with_input(BenchmarkId::new("sequential", size), &size, |bencher, _| {
            bencher.iter(|| {
                let sum: i64 = list.iter().sum();
                black_box(sum)
            });
        });

        // Parallel sum
        group.bench_with_input(BenchmarkId::new("parallel", size), &size, |bencher, _| {
            bencher.iter(|| {
                let sum: i64 = list.par_iter().sum();
                black_box(sum)
            });
        });
    }

    group.finish();
}

// =============================================================================
// PersistentTreeMap Benchmarks
// =============================================================================

fn benchmark_treemap_sum_values(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("treemap_sum_values");

    for size in [1000, 10000, 100000] {
        let map: PersistentTreeMap<i64, i64> = (0..size).map(|i| (i, i * 2)).collect();

        // Sequential sum
        group.bench_with_input(BenchmarkId::new("sequential", size), &size, |bencher, _| {
            bencher.iter(|| {
                let sum: i64 = map.iter().map(|(_, v)| v).sum();
                black_box(sum)
            });
        });

        // Parallel sum
        group.bench_with_input(BenchmarkId::new("parallel", size), &size, |bencher, _| {
            bencher.iter(|| {
                let sum: i64 = map.par_iter().map(|(_, v)| v).sum();
                black_box(sum)
            });
        });
    }

    group.finish();
}

// =============================================================================
// Heavy Computation Benchmarks
// =============================================================================

/// Simulates a computationally expensive operation
fn expensive_computation(x: i64) -> i64 {
    let mut result = x;
    for _ in 0..100 {
        result = result.wrapping_mul(result).wrapping_add(1);
    }
    result
}

fn benchmark_vector_heavy_map(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("vector_heavy_map");

    for size in [1000, 10000, 50000] {
        let vector: PersistentVector<i64> = (0..size).collect();

        // Sequential heavy map
        group.bench_with_input(BenchmarkId::new("sequential", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<i64> = vector.iter().map(|x| expensive_computation(*x)).collect();
                black_box(result)
            });
        });

        // Parallel heavy map
        group.bench_with_input(BenchmarkId::new("parallel", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: Vec<i64> = vector
                    .par_iter()
                    .map(|x| expensive_computation(*x))
                    .collect();
                black_box(result)
            });
        });
    }

    group.finish();
}

// =============================================================================
// FromParallelIterator Benchmarks
// =============================================================================

fn benchmark_from_par_iter_vector(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("from_par_iter_vector");

    for size in [1000, 10000, 100000] {
        let source: Vec<i64> = (0..size).collect();

        // Sequential collect
        group.bench_with_input(BenchmarkId::new("sequential", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: PersistentVector<i64> = source.iter().copied().collect();
                black_box(result)
            });
        });

        // Parallel collect
        group.bench_with_input(BenchmarkId::new("parallel", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result: PersistentVector<i64> = source.par_iter().copied().collect();
                black_box(result)
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
    // PersistentVector
    benchmark_vector_sum,
    benchmark_vector_map,
    benchmark_vector_filter,
    benchmark_vector_map_reduce,
    // PersistentHashMap
    benchmark_hashmap_sum_values,
    benchmark_hashmap_filter,
    // PersistentHashSet
    benchmark_hashset_sum,
    // PersistentList
    benchmark_list_sum,
    // PersistentTreeMap
    benchmark_treemap_sum_values,
    // Heavy computation
    benchmark_vector_heavy_map,
    // FromParallelIterator
    benchmark_from_par_iter_vector,
);

criterion_main!(benches);
