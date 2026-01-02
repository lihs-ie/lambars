//! Benchmark for PersistentVector vs standard Vec.
//!
//! Compares the performance of lambars' PersistentVector against Rust's standard Vec
//! for common operations.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use lambars::persistent::PersistentVector;

// =============================================================================
// push_back Benchmark
// =============================================================================

fn benchmark_push_back(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("push_back");

    for size in [100, 1000, 10000] {
        // PersistentVector push_back
        group.bench_with_input(
            BenchmarkId::new("PersistentVector", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut vector = PersistentVector::new();
                    for index in 0..size {
                        vector = vector.push_back(black_box(index));
                    }
                    black_box(vector)
                });
            },
        );

        // Standard Vec push
        group.bench_with_input(BenchmarkId::new("Vec", size), &size, |bencher, &size| {
            bencher.iter(|| {
                let mut vector = Vec::new();
                for index in 0..size {
                    vector.push(black_box(index));
                }
                black_box(vector)
            });
        });
    }

    group.finish();
}

// =============================================================================
// get Benchmark (Random Access)
// =============================================================================

fn benchmark_get(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("get");

    for size in [100, 1000, 10000] {
        // Prepare data
        let persistent_vector: PersistentVector<i32> = (0..size).collect();
        let standard_vector: Vec<i32> = (0..size).collect();

        // PersistentVector get
        group.bench_with_input(
            BenchmarkId::new("PersistentVector", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut sum = 0;
                    for index in 0..size as usize {
                        if let Some(&value) = persistent_vector.get(black_box(index)) {
                            sum += value;
                        }
                    }
                    black_box(sum)
                });
            },
        );

        // Standard Vec get
        group.bench_with_input(BenchmarkId::new("Vec", size), &size, |bencher, &size| {
            bencher.iter(|| {
                let mut sum = 0;
                for index in 0..size as usize {
                    if let Some(&value) = standard_vector.get(black_box(index)) {
                        sum += value;
                    }
                }
                black_box(sum)
            });
        });
    }

    group.finish();
}

// =============================================================================
// update Benchmark
// =============================================================================

fn benchmark_update(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("update");

    for size in [100, 1000, 10000] {
        // Prepare data
        let persistent_vector: PersistentVector<i32> = (0..size).collect();
        let standard_vector: Vec<i32> = (0..size).collect();

        // PersistentVector update (immutable, creates new vector)
        group.bench_with_input(
            BenchmarkId::new("PersistentVector", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let index = (size / 2) as usize;
                    let updated = persistent_vector.update(black_box(index), black_box(999));
                    black_box(updated)
                });
            },
        );

        // Standard Vec clone + update (to compare fair immutable update)
        group.bench_with_input(
            BenchmarkId::new("Vec_clone", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut cloned = standard_vector.clone();
                    let index = (size / 2) as usize;
                    cloned[black_box(index)] = black_box(999);
                    black_box(cloned)
                });
            },
        );

        // Standard Vec mutable update (in-place, for reference)
        group.bench_with_input(
            BenchmarkId::new("Vec_inplace", size),
            &size,
            |bencher, &size| {
                bencher.iter_batched(
                    || standard_vector.clone(),
                    |mut mutable_vector| {
                        let index = (size / 2) as usize;
                        mutable_vector[black_box(index)] = black_box(999);
                        black_box(mutable_vector)
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

// =============================================================================
// iteration Benchmark
// =============================================================================

fn benchmark_iteration(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("iteration");

    // Extended sizes for optimized iterator performance verification
    // As per Phase 8.1 requirements (TR-002): 1,000 / 100,000 / 1,000,000 elements
    for size in [1_000, 100_000, 1_000_000] {
        // Prepare data
        let persistent_vector: PersistentVector<i32> = (0..size).collect();
        let standard_vector: Vec<i32> = (0..size).collect();

        // PersistentVector iteration
        group.bench_with_input(
            BenchmarkId::new("PersistentVector", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let sum: i32 = persistent_vector.iter().sum();
                    black_box(sum)
                });
            },
        );

        // Standard Vec iteration
        group.bench_with_input(BenchmarkId::new("Vec", size), &size, |bencher, _| {
            bencher.iter(|| {
                let sum: i32 = standard_vector.iter().sum();
                black_box(sum)
            });
        });
    }

    group.finish();
}

// =============================================================================
// from_iter Benchmark (Construction)
// =============================================================================

fn benchmark_from_iter(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("from_iter");

    for size in [100, 1000, 10000] {
        // PersistentVector from_iter
        group.bench_with_input(
            BenchmarkId::new("PersistentVector", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let vector: PersistentVector<i32> = (0..size).collect();
                    black_box(vector)
                });
            },
        );

        // Standard Vec from_iter
        group.bench_with_input(BenchmarkId::new("Vec", size), &size, |bencher, &size| {
            bencher.iter(|| {
                let vector: Vec<i32> = (0..size).collect();
                black_box(vector)
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
    benchmark_push_back,
    benchmark_get,
    benchmark_update,
    benchmark_iteration,
    benchmark_from_iter
);

criterion_main!(benches);
