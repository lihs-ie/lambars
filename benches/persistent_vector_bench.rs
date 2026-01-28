//! Benchmark for PersistentVector vs standard Vec.
//!
//! Compares the performance of lambars' PersistentVector against Rust's standard Vec
//! for common operations.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lambars::persistent::PersistentVector;
use std::hint::black_box;

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

    for size in [100, 1000, 10000, 100000] {
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
// transient_update Benchmark
// =============================================================================

fn benchmark_transient_update(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("transient_update");

    for size in [1000, 10000, 100000] {
        let persistent_vector: PersistentVector<i32> = (0..size).collect();

        // TransientVector batch updates (issue #222 target: 100k ≤ 1.0ms)
        group.bench_with_input(
            BenchmarkId::new("TransientVector", size),
            &size,
            |bencher, &size| {
                bencher.iter_batched(
                    || persistent_vector.clone(),
                    |vector| {
                        let mut transient = vector.transient();
                        for index in (0..size as usize).step_by(10) {
                            transient.update(black_box(index), black_box(999));
                        }
                        black_box(transient.persistent())
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        // PersistentVector sequential updates for comparison
        group.bench_with_input(
            BenchmarkId::new("PersistentVector", size),
            &size,
            |bencher, &size| {
                bencher.iter_batched(
                    || persistent_vector.clone(),
                    |mut vector| {
                        for index in (0..size as usize).step_by(10) {
                            if let Some(updated) = vector.update(black_box(index), black_box(999)) {
                                vector = updated;
                            }
                        }
                        black_box(vector)
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
// concat Benchmark
// =============================================================================

fn benchmark_concat(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("concat");

    for size in [100, 1_000, 10_000, 100_000] {
        let left_persistent: PersistentVector<i32> = (0..size).collect();
        let right_persistent: PersistentVector<i32> = (size..size * 2).collect();
        let left_vec: Vec<i32> = (0..size).collect();
        let right_vec: Vec<i32> = (size..size * 2).collect();

        // PersistentVector concat - O(log n) RRB-Tree merge
        group.bench_with_input(
            BenchmarkId::new("PersistentVector_concat", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let result = left_persistent.concat(black_box(&right_persistent));
                    black_box(result)
                });
            },
        );

        // Naive approach: iter().chain().collect() - O(n)
        group.bench_with_input(
            BenchmarkId::new("PersistentVector_naive", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let result: PersistentVector<i32> = left_persistent
                        .iter()
                        .chain(right_persistent.iter())
                        .copied()
                        .collect();
                    black_box(result)
                });
            },
        );

        // Standard Vec clone + extend - O(n)
        group.bench_with_input(
            BenchmarkId::new("Vec_clone_extend", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let mut result = left_vec.clone();
                    result.extend(right_vec.iter().copied());
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// concat Scaling Benchmark (to verify O(log n) complexity)
// =============================================================================

fn benchmark_concat_scaling(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("concat_scaling");
    group.sample_size(50);

    // Test with exponentially increasing sizes to verify O(log n) vs O(n)
    for exponent in [10, 12, 14, 16, 18, 20] {
        let size = 1 << exponent; // 1K, 4K, 16K, 64K, 256K, 1M

        let left: PersistentVector<i32> = (0..size).collect();
        let right: PersistentVector<i32> = (size..size * 2).collect();

        group.bench_with_input(BenchmarkId::new("concat", size), &size, |bencher, _| {
            bencher.iter(|| {
                let result = left.concat(black_box(&right));
                black_box(result)
            });
        });
    }

    group.finish();
}

// =============================================================================
// concat Chain Benchmark (multiple concatenations)
// =============================================================================

fn benchmark_concat_chain(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("concat_chain");

    for num_vectors in [4, 8, 16, 32] {
        let vector_size = 1000;
        let vectors: Vec<PersistentVector<i32>> = (0..num_vectors)
            .map(|index| {
                let start = index * vector_size;
                (start..start + vector_size).collect()
            })
            .collect();

        // concat chain using fold
        group.bench_with_input(
            BenchmarkId::new("PersistentVector_fold", num_vectors),
            &num_vectors,
            |bencher, _| {
                bencher.iter(|| {
                    let result = vectors
                        .iter()
                        .skip(1)
                        .fold(vectors[0].clone(), |accumulator, vector| {
                            accumulator.concat(black_box(vector))
                        });
                    black_box(result)
                });
            },
        );

        // naive chain using iter().flatten().collect()
        group.bench_with_input(
            BenchmarkId::new("PersistentVector_naive", num_vectors),
            &num_vectors,
            |bencher, _| {
                bencher.iter(|| {
                    let result: PersistentVector<i32> = vectors
                        .iter()
                        .flat_map(|vector| vector.iter().copied())
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
    benchmark_push_back,
    benchmark_get,
    benchmark_update,
    benchmark_transient_update,
    benchmark_iteration,
    benchmark_from_iter,
    benchmark_concat,
    benchmark_concat_scaling,
    benchmark_concat_chain
);

criterion_main!(benches);
