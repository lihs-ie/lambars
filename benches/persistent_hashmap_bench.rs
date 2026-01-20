//! Benchmark for PersistentHashMap vs standard HashMap.
//!
//! Compares the performance of lambars' PersistentHashMap against Rust's standard HashMap
//! for common operations.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lambars::persistent::PersistentHashMap;
use std::collections::HashMap;
use std::hint::black_box;

// =============================================================================
// insert Benchmark
// =============================================================================

fn benchmark_insert(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("insert");

    for size in [1_000, 10_000, 100_000] {
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

// =============================================================================
// get Benchmark
// =============================================================================

fn benchmark_get(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("get");

    for size in [100, 1000, 10000] {
        // Prepare data
        let persistent_map: PersistentHashMap<i32, i32> =
            (0..size).map(|index| (index, index * 2)).collect();
        let standard_map: HashMap<i32, i32> = (0..size).map(|index| (index, index * 2)).collect();

        // PersistentHashMap get
        group.bench_with_input(
            BenchmarkId::new("PersistentHashMap", size),
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

        // Standard HashMap get
        group.bench_with_input(
            BenchmarkId::new("HashMap", size),
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
// remove Benchmark
// =============================================================================

fn benchmark_remove(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("remove");

    for size in [100, 1000, 10000] {
        // Prepare data
        let persistent_map: PersistentHashMap<i32, i32> =
            (0..size).map(|index| (index, index * 2)).collect();
        let standard_map: HashMap<i32, i32> = (0..size).map(|index| (index, index * 2)).collect();

        // PersistentHashMap remove (single key, immutable)
        group.bench_with_input(
            BenchmarkId::new("PersistentHashMap_single", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let key = size / 2;
                    let removed = persistent_map.remove(&black_box(key));
                    black_box(removed)
                });
            },
        );

        // Standard HashMap clone + remove (to compare fair immutable remove)
        group.bench_with_input(
            BenchmarkId::new("HashMap_clone_single", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut cloned = standard_map.clone();
                    let key = size / 2;
                    cloned.remove(&black_box(key));
                    black_box(cloned)
                });
            },
        );

        // PersistentHashMap remove all (sequential)
        group.bench_with_input(
            BenchmarkId::new("PersistentHashMap_all", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut map = persistent_map.clone();
                    for key in 0..size {
                        map = map.remove(&black_box(key));
                    }
                    black_box(map)
                });
            },
        );

        // Standard HashMap remove all (mutable)
        group.bench_with_input(
            BenchmarkId::new("HashMap_all", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut map = standard_map.clone();
                    for key in 0..size {
                        map.remove(&black_box(key));
                    }
                    black_box(map)
                });
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

    for size in [100, 1000, 10000] {
        // Prepare data
        let persistent_map: PersistentHashMap<i32, i32> =
            (0..size).map(|index| (index, index * 2)).collect();
        let standard_map: HashMap<i32, i32> = (0..size).map(|index| (index, index * 2)).collect();

        // PersistentHashMap iteration
        group.bench_with_input(
            BenchmarkId::new("PersistentHashMap", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let sum: i32 = persistent_map.iter().map(|(_, &value)| value).sum();
                    black_box(sum)
                });
            },
        );

        // Standard HashMap iteration
        group.bench_with_input(BenchmarkId::new("HashMap", size), &size, |bencher, _| {
            bencher.iter(|| {
                let sum: i32 = standard_map.values().sum();
                black_box(sum)
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
        let persistent_map: PersistentHashMap<i32, i32> =
            (0..size).map(|index| (index, index * 2)).collect();
        let standard_map: HashMap<i32, i32> = (0..size).map(|index| (index, index * 2)).collect();

        for take_count in [1, 10, 100] {
            let label = format!("{}/take_{}", size, take_count);

            group.bench_with_input(
                BenchmarkId::new("PersistentHashMap", &label),
                &take_count,
                |bencher, &take_count| {
                    bencher.iter(|| {
                        let result: Vec<_> = persistent_map.iter().take(take_count).collect();
                        black_box(result)
                    });
                },
            );

            group.bench_with_input(
                BenchmarkId::new("HashMap", &label),
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
        let persistent_map: PersistentHashMap<i32, i32> =
            (0..size).map(|index| (index, index * 2)).collect();
        let standard_map: HashMap<i32, i32> = (0..size).map(|index| (index, index * 2)).collect();

        group.bench_with_input(
            BenchmarkId::new("PersistentHashMap", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let first = persistent_map.iter().next();
                    black_box(first)
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("HashMap", size), &size, |bencher, _| {
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
        let persistent_map: PersistentHashMap<i32, i32> =
            (0..size).map(|index| (index, index * 2)).collect();
        let standard_map: HashMap<i32, i32> = (0..size).map(|index| (index, index * 2)).collect();

        group.bench_with_input(
            BenchmarkId::new("PersistentHashMap", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let iterator = persistent_map.iter();
                    black_box(iterator)
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("HashMap", size), &size, |bencher, _| {
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
        let persistent_map: PersistentHashMap<i32, i32> =
            (0..size).map(|index| (index, index * 2)).collect();
        let standard_map: HashMap<i32, i32> = (0..size).map(|index| (index, index * 2)).collect();

        let target = size / 2;

        group.bench_with_input(
            BenchmarkId::new("PersistentHashMap", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let found = persistent_map.iter().find(|(key, _)| **key == target);
                    black_box(found)
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("HashMap", size), &size, |bencher, _| {
            bencher.iter(|| {
                let found = standard_map.iter().find(|(key, _)| **key == target);
                black_box(found)
            });
        });
    }

    group.finish();
}

// =============================================================================
// insert_large_scale Benchmark (Phase 5: Issue #221)
// =============================================================================

#[allow(clippy::single_element_loop)]
fn benchmark_insert_large_scale(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("insert_large_scale");

    // Large scale benchmarks require reduced sampling to avoid timeout
    group.sample_size(10);

    // Note: Using a loop for extensibility - additional scales can be added here
    for size in [1_000_000] {
        // PersistentHashMap insert
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

// =============================================================================
// collect Benchmark (Phase 5: Issue #221)
// =============================================================================

fn benchmark_collect(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("collect");

    for size in [1_000, 10_000, 100_000] {
        // PersistentHashMap collect (FromIterator)
        group.bench_with_input(
            BenchmarkId::new("PersistentHashMap", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let map: PersistentHashMap<i32, i32> =
                        (0..size).map(|index| (index, index * 2)).collect();
                    black_box(map)
                });
            },
        );

        // Standard HashMap collect (FromIterator)
        group.bench_with_input(
            BenchmarkId::new("HashMap", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let map: HashMap<i32, i32> =
                        (0..size).map(|index| (index, index * 2)).collect();
                    black_box(map)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// transient_build Benchmark (Phase 5: Issue #221)
// =============================================================================

fn benchmark_transient_build(criterion: &mut Criterion) {
    use lambars::persistent::TransientHashMap;

    let mut group = criterion.benchmark_group("transient_build");

    for size in [1_000, 10_000, 100_000] {
        // TransientHashMap batch insert and convert to persistent
        group.bench_with_input(
            BenchmarkId::new("TransientHashMap", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut transient: TransientHashMap<i32, i32> =
                        TransientHashMap::with_capacity_hint(size as usize);
                    for index in 0..size {
                        transient.insert(black_box(index), black_box(index * 2));
                    }
                    let persistent = transient.persistent();
                    black_box(persistent)
                });
            },
        );

        // Standard HashMap with_capacity for comparison
        group.bench_with_input(
            BenchmarkId::new("HashMap_with_capacity", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut map: HashMap<i32, i32> = HashMap::with_capacity(size as usize);
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
// Criterion Group and Main
// =============================================================================

criterion_group!(
    benches,
    benchmark_insert,
    benchmark_get,
    benchmark_remove,
    benchmark_iteration,
    benchmark_iteration_early_exit,
    benchmark_iteration_first,
    benchmark_iteration_create,
    benchmark_iteration_find,
    benchmark_insert_large_scale,
    benchmark_collect,
    benchmark_transient_build
);

criterion_main!(benches);
