//! Benchmark for Transient data structures.
//!
//! Compares the performance of TransientVector, TransientHashMap, and TransientHashSet
//! against their Persistent counterparts and standard library equivalents for batch operations.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lambars::persistent::{
    PersistentHashMap, PersistentHashSet, PersistentVector, TransientHashMap, TransientHashSet,
    TransientVector,
};
use std::collections::{HashMap, HashSet};
use std::hint::black_box;

// =============================================================================
// TransientVector Benchmarks
// =============================================================================

fn benchmark_transient_vector_push_back(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("transient_vector_push_back");

    for size in [1_000, 10_000, 100_000] {
        // TransientVector push_back
        group.bench_with_input(
            BenchmarkId::new("TransientVector", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut transient = TransientVector::new();
                    for index in 0..size {
                        transient.push_back(black_box(index));
                    }
                    black_box(transient.persistent())
                });
            },
        );

        // PersistentVector push_back (immutable)
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

fn benchmark_transient_vector_update(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("transient_vector_update");

    for size in [1_000, 10_000, 100_000] {
        // Prepare initial data
        let persistent_vector: PersistentVector<i32> = (0..size).collect();

        // TransientVector update (batch updates)
        group.bench_with_input(
            BenchmarkId::new("TransientVector", size),
            &size,
            |bencher, &size| {
                let vector = persistent_vector.clone();
                bencher.iter_batched(
                    || vector.clone().transient(),
                    |mut transient| {
                        for index in (0..size as usize).step_by(10) {
                            transient.update(black_box(index), black_box(999));
                        }
                        black_box(transient.persistent())
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        // PersistentVector update (immutable, creates new vector each time)
        group.bench_with_input(
            BenchmarkId::new("PersistentVector", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut vector = persistent_vector.clone();
                    for index in (0..size as usize).step_by(10) {
                        vector = vector.update(black_box(index), black_box(999)).unwrap();
                    }
                    black_box(vector)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_transient_vector_roundtrip(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("transient_vector_roundtrip");

    for size in [1_000, 10_000] {
        let persistent_vector: PersistentVector<i32> = (0..size).collect();

        // Measure roundtrip: persistent -> transient -> persistent
        group.bench_with_input(BenchmarkId::new("roundtrip", size), &size, |bencher, _| {
            bencher.iter(|| {
                let transient = persistent_vector.clone().transient();
                black_box(transient.persistent())
            });
        });
    }

    group.finish();
}

// =============================================================================
// TransientHashMap Benchmarks
// =============================================================================

fn benchmark_transient_hashmap_insert(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("transient_hashmap_insert");

    for size in [1_000, 10_000, 100_000] {
        // TransientHashMap insert
        group.bench_with_input(
            BenchmarkId::new("TransientHashMap", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut transient = TransientHashMap::new();
                    for index in 0..size {
                        transient.insert(black_box(index), black_box(index * 2));
                    }
                    black_box(transient.persistent())
                });
            },
        );

        // PersistentHashMap insert (immutable)
        group.bench_with_input(
            BenchmarkId::new("PersistentHashMap", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut map = PersistentHashMap::new();
                    for index in 0..size {
                        map = map.insert(black_box(index), black_box(index * 2));
                    }
                    black_box(map)
                });
            },
        );

        // Standard HashMap insert
        group.bench_with_input(
            BenchmarkId::new("HashMap", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut map = HashMap::new();
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

fn benchmark_transient_hashmap_update(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("transient_hashmap_update");

    for size in [1_000, 10_000] {
        let persistent_map: PersistentHashMap<i32, i32> =
            (0..size).map(|index| (index, index * 2)).collect();

        // TransientHashMap update_with (batch updates)
        group.bench_with_input(
            BenchmarkId::new("TransientHashMap", size),
            &size,
            |bencher, &size| {
                let map = persistent_map.clone();
                bencher.iter_batched(
                    || map.clone().transient(),
                    |mut transient| {
                        for key in (0..size).step_by(10) {
                            transient.update_with(&black_box(key), |value| value + 1);
                        }
                        black_box(transient.persistent())
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        // PersistentHashMap update (immutable)
        group.bench_with_input(
            BenchmarkId::new("PersistentHashMap", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut map = persistent_map.clone();
                    for key in (0..size).step_by(10) {
                        if let Some(new_map) = map.update(&black_box(key), |value| value + 1) {
                            map = new_map;
                        }
                    }
                    black_box(map)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_transient_hashmap_remove(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("transient_hashmap_remove");

    for size in [1_000, 10_000] {
        let persistent_map: PersistentHashMap<i32, i32> =
            (0..size).map(|index| (index, index * 2)).collect();

        // TransientHashMap remove (batch removes)
        group.bench_with_input(
            BenchmarkId::new("TransientHashMap", size),
            &size,
            |bencher, &size| {
                let map = persistent_map.clone();
                bencher.iter_batched(
                    || map.clone().transient(),
                    |mut transient| {
                        for key in (0..size).step_by(10) {
                            transient.remove(&black_box(key));
                        }
                        black_box(transient.persistent())
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        // PersistentHashMap remove (immutable)
        group.bench_with_input(
            BenchmarkId::new("PersistentHashMap", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut map = persistent_map.clone();
                    for key in (0..size).step_by(10) {
                        map = map.remove(&black_box(key));
                    }
                    black_box(map)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// TransientHashSet Benchmarks
// =============================================================================

fn benchmark_transient_hashset_insert(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("transient_hashset_insert");

    for size in [1_000, 10_000, 100_000] {
        // TransientHashSet insert
        group.bench_with_input(
            BenchmarkId::new("TransientHashSet", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut transient = TransientHashSet::new();
                    for value in 0..size {
                        transient.insert(black_box(value));
                    }
                    black_box(transient.persistent())
                });
            },
        );

        // PersistentHashSet insert (immutable)
        group.bench_with_input(
            BenchmarkId::new("PersistentHashSet", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut set = PersistentHashSet::new();
                    for value in 0..size {
                        set = set.insert(black_box(value));
                    }
                    black_box(set)
                });
            },
        );

        // Standard HashSet insert
        group.bench_with_input(
            BenchmarkId::new("HashSet", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut set = HashSet::new();
                    for value in 0..size {
                        set.insert(black_box(value));
                    }
                    black_box(set)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// FromIterator Optimization Benchmark
// =============================================================================

fn benchmark_collect_optimization(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("collect_optimization");

    for size in [10_000, 100_000] {
        // PersistentVector FromIterator (uses Transient internally)
        group.bench_with_input(
            BenchmarkId::new("PersistentVector_collect", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let vector: PersistentVector<i32> = (0..size).collect();
                    black_box(vector)
                });
            },
        );

        // PersistentHashMap FromIterator (uses Transient internally)
        group.bench_with_input(
            BenchmarkId::new("PersistentHashMap_collect", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let map: PersistentHashMap<i32, i32> =
                        (0..size).map(|index| (index, index * 2)).collect();
                    black_box(map)
                });
            },
        );

        // PersistentHashSet FromIterator (uses Transient internally)
        group.bench_with_input(
            BenchmarkId::new("PersistentHashSet_collect", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let set: PersistentHashSet<i32> = (0..size).collect();
                    black_box(set)
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
    benchmark_transient_vector_push_back,
    benchmark_transient_vector_update,
    benchmark_transient_vector_roundtrip,
    benchmark_transient_hashmap_insert,
    benchmark_transient_hashmap_update,
    benchmark_transient_hashmap_remove,
    benchmark_transient_hashset_insert,
    benchmark_collect_optimization,
);

criterion_main!(benches);
