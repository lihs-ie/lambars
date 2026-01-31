//! OrderedUniqueSet bulk construction benchmark.
//!
//! Compares `from_sorted_iter`, `from_sorted_vec` vs `fold + insert` (baseline).
//! Expected: bulk APIs should be at least 2x faster than incremental construction.
//!
//! Pre-generated Vec is reused via clone() in setup to avoid regeneration overhead
//! and ensure consistent benchmark data across iterations.

use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use lambars::persistent::OrderedUniqueSet;
use std::hint::black_box;

const SIZES: [i32; 4] = [100, 1000, 10000, 100000];

/// Pre-generates sorted Vec for each size to be reused in benchmarks.
fn generate_sorted_vec(size: i32) -> Vec<i32> {
    (0..size).collect()
}

/// Returns the appropriate BatchSize based on input size.
/// - SmallInput: for sizes < 1000 (Small state, fast setup, many iterations)
/// - LargeInput: for sizes >= 1000 (Large state, slower setup, fewer iterations, better cache behavior)
fn batch_size_for(size: i32) -> BatchSize {
    if size < 1000 {
        BatchSize::SmallInput
    } else {
        BatchSize::LargeInput
    }
}

fn benchmark_from_sorted_iter(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("ordered_unique_set_from_sorted_iter");

    for size in SIZES {
        let base_vec = generate_sorted_vec(size);
        group.bench_with_input(
            BenchmarkId::new("from_sorted_iter", size),
            &size,
            |bencher, &size| {
                bencher.iter_batched(
                    || base_vec.clone(),
                    |elements| black_box(OrderedUniqueSet::from_sorted_iter(black_box(elements))),
                    batch_size_for(size),
                );
            },
        );
    }

    group.finish();
}

fn benchmark_from_sorted_vec(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("ordered_unique_set_from_sorted_vec");

    for size in SIZES {
        let base_vec = generate_sorted_vec(size);
        group.bench_with_input(
            BenchmarkId::new("from_sorted_vec", size),
            &size,
            |bencher, &size| {
                bencher.iter_batched(
                    || base_vec.clone(),
                    |elements| black_box(OrderedUniqueSet::from_sorted_vec(black_box(elements))),
                    batch_size_for(size),
                );
            },
        );
    }

    group.finish();
}

fn benchmark_fold_insert(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("ordered_unique_set_fold_insert");

    for size in SIZES {
        let base_vec = generate_sorted_vec(size);
        group.bench_with_input(
            BenchmarkId::new("fold_insert", size),
            &size,
            |bencher, &size| {
                bencher.iter_batched(
                    || base_vec.clone(),
                    |elements| {
                        black_box(
                            elements
                                .into_iter()
                                .fold(OrderedUniqueSet::new(), |accumulator, element| {
                                    accumulator.insert(black_box(element))
                                }),
                        )
                    },
                    batch_size_for(size),
                );
            },
        );
    }

    group.finish();
}

fn benchmark_bulk_construction_comparison(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("ordered_unique_set_bulk_comparison");

    for size in [1000, 10000] {
        let base_vec = generate_sorted_vec(size);

        group.bench_with_input(
            BenchmarkId::new("from_sorted_iter", size),
            &size,
            |bencher, &size| {
                bencher.iter_batched(
                    || base_vec.clone(),
                    |elements| black_box(OrderedUniqueSet::from_sorted_iter(black_box(elements))),
                    batch_size_for(size),
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("from_sorted_vec", size),
            &size,
            |bencher, &size| {
                bencher.iter_batched(
                    || base_vec.clone(),
                    |elements| black_box(OrderedUniqueSet::from_sorted_vec(black_box(elements))),
                    batch_size_for(size),
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("fold_insert", size),
            &size,
            |bencher, &size| {
                bencher.iter_batched(
                    || base_vec.clone(),
                    |elements| {
                        black_box(
                            elements
                                .into_iter()
                                .fold(OrderedUniqueSet::new(), |accumulator, element| {
                                    accumulator.insert(black_box(element))
                                }),
                        )
                    },
                    batch_size_for(size),
                );
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_from_sorted_iter,
    benchmark_from_sorted_vec,
    benchmark_fold_insert,
    benchmark_bulk_construction_comparison
);

criterion_main!(benches);
