//! PersistentVector vs standard Vec benchmark.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lambars::persistent::PersistentVector;
use std::hint::black_box;

fn benchmark_push_back(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("push_back");

    for size in [100, 1000, 10000] {
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

fn benchmark_get(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("get");

    for size in [100, 1000, 10000] {
        let persistent_vector: PersistentVector<i32> = (0..size).collect();
        let standard_vector: Vec<i32> = (0..size).collect();

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

fn benchmark_update(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("update");

    for size in [100, 1000, 10000, 100000] {
        let persistent_vector: PersistentVector<i32> = (0..size).collect();
        let standard_vector: Vec<i32> = (0..size).collect();

        group.bench_with_input(
            BenchmarkId::new("PersistentVector", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let index = (size / 2) as usize;
                    black_box(persistent_vector.update(black_box(index), black_box(999)))
                });
            },
        );

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

fn benchmark_transient_update(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("transient_update");

    for size in [1000, 10000, 100000] {
        let persistent_vector: PersistentVector<i32> = (0..size).collect();

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

fn benchmark_iteration(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("iteration");

    for size in [1_000, 100_000, 1_000_000] {
        let persistent_vector: PersistentVector<i32> = (0..size).collect();
        let standard_vector: Vec<i32> = (0..size).collect();

        group.bench_with_input(
            BenchmarkId::new("PersistentVector", size),
            &size,
            |bencher, _| {
                bencher.iter(|| black_box(persistent_vector.iter().sum::<i32>()));
            },
        );

        group.bench_with_input(BenchmarkId::new("Vec", size), &size, |bencher, _| {
            bencher.iter(|| black_box(standard_vector.iter().sum::<i32>()));
        });
    }

    group.finish();
}

fn benchmark_from_iter(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("from_iter");

    for size in [100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("PersistentVector", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| black_box((0..size).collect::<PersistentVector<i32>>()));
            },
        );

        group.bench_with_input(BenchmarkId::new("Vec", size), &size, |bencher, &size| {
            bencher.iter(|| black_box((0..size).collect::<Vec<i32>>()));
        });
    }

    group.finish();
}

fn benchmark_concat(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("concat");

    for size in [100, 1_000, 10_000, 100_000] {
        let left_persistent: PersistentVector<i32> = (0..size).collect();
        let right_persistent: PersistentVector<i32> = (size..size * 2).collect();
        let left_vec: Vec<i32> = (0..size).collect();
        let right_vec: Vec<i32> = (size..size * 2).collect();

        group.bench_with_input(
            BenchmarkId::new("PersistentVector_concat", size),
            &size,
            |bencher, _| {
                bencher.iter(|| black_box(left_persistent.concat(black_box(&right_persistent))));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("PersistentVector_naive", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(
                        left_persistent
                            .iter()
                            .chain(right_persistent.iter())
                            .copied()
                            .collect::<PersistentVector<i32>>(),
                    )
                });
            },
        );

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

fn benchmark_concat_scaling(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("concat_scaling");
    group.sample_size(50);

    for exponent in [10, 12, 14, 16, 18, 20] {
        let size = 1 << exponent;
        let left: PersistentVector<i32> = (0..size).collect();
        let right: PersistentVector<i32> = (size..size * 2).collect();

        group.bench_with_input(BenchmarkId::new("concat", size), &size, |bencher, _| {
            bencher.iter(|| black_box(left.concat(black_box(&right))));
        });
    }

    group.finish();
}

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

        group.bench_with_input(
            BenchmarkId::new("PersistentVector_fold", num_vectors),
            &num_vectors,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(
                        vectors
                            .iter()
                            .skip(1)
                            .fold(vectors[0].clone(), |accumulator, vector| {
                                accumulator.concat(black_box(vector))
                            }),
                    )
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("PersistentVector_naive", num_vectors),
            &num_vectors,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(
                        vectors
                            .iter()
                            .flat_map(|vector| vector.iter().copied())
                            .collect::<PersistentVector<i32>>(),
                    )
                });
            },
        );
    }

    group.finish();
}

/// Pre-generates Vec for each size to be reused in benchmarks.
fn generate_vec(size: i32) -> Vec<i32> {
    (0..size).collect()
}

/// Returns the appropriate BatchSize based on input size.
/// - SmallInput: for sizes < 1000 (Small state, fast setup, many iterations)
/// - LargeInput: for sizes >= 1000 (Large state, slower setup, fewer iterations, better cache behavior)
fn batch_size_for(size: i32) -> criterion::BatchSize {
    if size < 1000 {
        criterion::BatchSize::SmallInput
    } else {
        criterion::BatchSize::LargeInput
    }
}

fn benchmark_from_vec(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("from_vec");

    for size in [100, 1000, 10000, 100000] {
        let base_vec = generate_vec(size);

        group.bench_with_input(
            BenchmarkId::new("PersistentVector_from_vec", size),
            &size,
            |bencher, &size| {
                bencher.iter_batched(
                    || base_vec.clone(),
                    |elements| black_box(PersistentVector::from_vec(black_box(elements))),
                    batch_size_for(size),
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("PersistentVector_collect", size),
            &size,
            |bencher, &size| {
                bencher.iter_batched(
                    || 0..size,
                    |range| black_box(black_box(range).collect::<PersistentVector<i32>>()),
                    batch_size_for(size),
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("Vec_collect", size),
            &size,
            |bencher, &size| {
                bencher.iter_batched(
                    || 0..size,
                    |range| black_box(black_box(range).collect::<Vec<i32>>()),
                    batch_size_for(size),
                );
            },
        );
    }

    group.finish();
}

fn benchmark_bulk_construction_comparison(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("persistent_vector_bulk_comparison");

    for size in [1000, 10000] {
        let base_vec = generate_vec(size);

        group.bench_with_input(
            BenchmarkId::new("from_vec", size),
            &size,
            |bencher, &size| {
                bencher.iter_batched(
                    || base_vec.clone(),
                    |elements| black_box(PersistentVector::from_vec(black_box(elements))),
                    batch_size_for(size),
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("collect", size),
            &size,
            |bencher, &size| {
                bencher.iter_batched(
                    || 0..size,
                    |range| black_box(black_box(range).collect::<PersistentVector<i32>>()),
                    batch_size_for(size),
                );
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_push_back,
    benchmark_get,
    benchmark_update,
    benchmark_transient_update,
    benchmark_iteration,
    benchmark_from_iter,
    benchmark_from_vec,
    benchmark_bulk_construction_comparison,
    benchmark_concat,
    benchmark_concat_scaling,
    benchmark_concat_chain
);

criterion_main!(benches);
