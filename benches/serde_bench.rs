//! Benchmark for serde serialization/deserialization of persistent data structures.
//!
//! Compares the performance of lambars' persistent data structures against
//! standard library collections for serde operations.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lambars::control::Either;
use lambars::persistent::{
    PersistentHashMap, PersistentHashSet, PersistentList, PersistentTreeMap, PersistentVector,
};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::hint::black_box;

// =============================================================================
// PersistentList vs VecDeque - Serialize
// =============================================================================

fn benchmark_list_serialize(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serde_serialize_list");

    for size in [100, 1000, 10000] {
        let persistent_list: PersistentList<i32> = (0..size).collect();
        let standard_deque: VecDeque<i32> = (0..size).collect();

        group.bench_with_input(
            BenchmarkId::new("PersistentList", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let json = serde_json::to_string(&persistent_list).unwrap();
                    black_box(json)
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("VecDeque", size), &size, |bencher, _| {
            bencher.iter(|| {
                let json = serde_json::to_string(&standard_deque).unwrap();
                black_box(json)
            });
        });
    }

    group.finish();
}

// =============================================================================
// PersistentList vs VecDeque - Deserialize
// =============================================================================

fn benchmark_list_deserialize(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serde_deserialize_list");

    for size in [100, 1000, 10000] {
        let standard_vec: Vec<i32> = (0..size).collect();
        let json = serde_json::to_string(&standard_vec).unwrap();

        group.bench_with_input(
            BenchmarkId::new("PersistentList", size),
            &json,
            |bencher, json| {
                bencher.iter(|| {
                    let list: PersistentList<i32> = serde_json::from_str(json).unwrap();
                    black_box(list)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("VecDeque", size),
            &json,
            |bencher, json| {
                bencher.iter(|| {
                    let deque: VecDeque<i32> = serde_json::from_str(json).unwrap();
                    black_box(deque)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// PersistentVector vs Vec - Serialize
// =============================================================================

fn benchmark_vector_serialize(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serde_serialize_vector");

    for size in [100, 1000, 10000] {
        let persistent_vector: PersistentVector<i32> = (0..size).collect();
        let standard_vec: Vec<i32> = (0..size).collect();

        group.bench_with_input(
            BenchmarkId::new("PersistentVector", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let json = serde_json::to_string(&persistent_vector).unwrap();
                    black_box(json)
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("Vec", size), &size, |bencher, _| {
            bencher.iter(|| {
                let json = serde_json::to_string(&standard_vec).unwrap();
                black_box(json)
            });
        });
    }

    group.finish();
}

// =============================================================================
// PersistentVector vs Vec - Deserialize
// =============================================================================

fn benchmark_vector_deserialize(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serde_deserialize_vector");

    for size in [100, 1000, 10000] {
        let standard_vec: Vec<i32> = (0..size).collect();
        let json = serde_json::to_string(&standard_vec).unwrap();

        group.bench_with_input(
            BenchmarkId::new("PersistentVector", size),
            &json,
            |bencher, json| {
                bencher.iter(|| {
                    let vector: PersistentVector<i32> = serde_json::from_str(json).unwrap();
                    black_box(vector)
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("Vec", size), &json, |bencher, json| {
            bencher.iter(|| {
                let vec: Vec<i32> = serde_json::from_str(json).unwrap();
                black_box(vec)
            });
        });
    }

    group.finish();
}

// =============================================================================
// PersistentHashSet vs HashSet - Serialize
// =============================================================================

fn benchmark_hashset_serialize(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serde_serialize_hashset");

    for size in [100, 1000, 10000] {
        let persistent_set: PersistentHashSet<i32> = (0..size).collect();
        let standard_set: HashSet<i32> = (0..size).collect();

        group.bench_with_input(
            BenchmarkId::new("PersistentHashSet", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let json = serde_json::to_string(&persistent_set).unwrap();
                    black_box(json)
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("HashSet", size), &size, |bencher, _| {
            bencher.iter(|| {
                let json = serde_json::to_string(&standard_set).unwrap();
                black_box(json)
            });
        });
    }

    group.finish();
}

// =============================================================================
// PersistentHashSet vs HashSet - Deserialize
// =============================================================================

fn benchmark_hashset_deserialize(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serde_deserialize_hashset");

    for size in [100, 1000, 10000] {
        let standard_vec: Vec<i32> = (0..size).collect();
        let json = serde_json::to_string(&standard_vec).unwrap();

        group.bench_with_input(
            BenchmarkId::new("PersistentHashSet", size),
            &json,
            |bencher, json| {
                bencher.iter(|| {
                    let set: PersistentHashSet<i32> = serde_json::from_str(json).unwrap();
                    black_box(set)
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("HashSet", size), &json, |bencher, json| {
            bencher.iter(|| {
                let set: HashSet<i32> = serde_json::from_str(json).unwrap();
                black_box(set)
            });
        });
    }

    group.finish();
}

// =============================================================================
// PersistentHashMap vs HashMap - Serialize
// =============================================================================

fn benchmark_hashmap_serialize(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serde_serialize_hashmap");

    for size in [100, 1000, 10000] {
        let persistent_map: PersistentHashMap<String, i32> =
            (0..size).map(|i| (format!("key{i}"), i)).collect();
        let standard_map: HashMap<String, i32> =
            (0..size).map(|i| (format!("key{i}"), i)).collect();

        group.bench_with_input(
            BenchmarkId::new("PersistentHashMap", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let json = serde_json::to_string(&persistent_map).unwrap();
                    black_box(json)
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("HashMap", size), &size, |bencher, _| {
            bencher.iter(|| {
                let json = serde_json::to_string(&standard_map).unwrap();
                black_box(json)
            });
        });
    }

    group.finish();
}

// =============================================================================
// PersistentHashMap vs HashMap - Deserialize
// =============================================================================

fn benchmark_hashmap_deserialize(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serde_deserialize_hashmap");

    for size in [100, 1000, 10000] {
        let standard_map: HashMap<String, i32> =
            (0..size).map(|i| (format!("key{i}"), i)).collect();
        let json = serde_json::to_string(&standard_map).unwrap();

        group.bench_with_input(
            BenchmarkId::new("PersistentHashMap", size),
            &json,
            |bencher, json| {
                bencher.iter(|| {
                    let map: PersistentHashMap<String, i32> = serde_json::from_str(json).unwrap();
                    black_box(map)
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("HashMap", size), &json, |bencher, json| {
            bencher.iter(|| {
                let map: HashMap<String, i32> = serde_json::from_str(json).unwrap();
                black_box(map)
            });
        });
    }

    group.finish();
}

// =============================================================================
// PersistentTreeMap vs BTreeMap - Serialize
// =============================================================================

fn benchmark_treemap_serialize(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serde_serialize_treemap");

    for size in [100, 1000, 10000] {
        let persistent_map: PersistentTreeMap<String, i32> =
            (0..size).map(|i| (format!("key{i:05}"), i)).collect();
        let standard_map: BTreeMap<String, i32> =
            (0..size).map(|i| (format!("key{i:05}"), i)).collect();

        group.bench_with_input(
            BenchmarkId::new("PersistentTreeMap", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let json = serde_json::to_string(&persistent_map).unwrap();
                    black_box(json)
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("BTreeMap", size), &size, |bencher, _| {
            bencher.iter(|| {
                let json = serde_json::to_string(&standard_map).unwrap();
                black_box(json)
            });
        });
    }

    group.finish();
}

// =============================================================================
// PersistentTreeMap vs BTreeMap - Deserialize
// =============================================================================

fn benchmark_treemap_deserialize(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serde_deserialize_treemap");

    for size in [100, 1000, 10000] {
        let standard_map: BTreeMap<String, i32> =
            (0..size).map(|i| (format!("key{i:05}"), i)).collect();
        let json = serde_json::to_string(&standard_map).unwrap();

        group.bench_with_input(
            BenchmarkId::new("PersistentTreeMap", size),
            &json,
            |bencher, json| {
                bencher.iter(|| {
                    let map: PersistentTreeMap<String, i32> = serde_json::from_str(json).unwrap();
                    black_box(map)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("BTreeMap", size),
            &json,
            |bencher, json| {
                bencher.iter(|| {
                    let map: BTreeMap<String, i32> = serde_json::from_str(json).unwrap();
                    black_box(map)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Either - Serialize/Deserialize
// =============================================================================

fn benchmark_either_serialize(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serde_either");

    let left: Either<String, i32> = Either::Left("error message".to_string());
    let right: Either<String, i32> = Either::Right(42);

    group.bench_function("serialize_left", |bencher| {
        bencher.iter(|| {
            let json = serde_json::to_string(&left).unwrap();
            black_box(json)
        });
    });

    group.bench_function("serialize_right", |bencher| {
        bencher.iter(|| {
            let json = serde_json::to_string(&right).unwrap();
            black_box(json)
        });
    });

    let left_json = serde_json::to_string(&left).unwrap();
    let right_json = serde_json::to_string(&right).unwrap();

    group.bench_function("deserialize_left", |bencher| {
        bencher.iter(|| {
            let either: Either<String, i32> = serde_json::from_str(&left_json).unwrap();
            black_box(either)
        });
    });

    group.bench_function("deserialize_right", |bencher| {
        bencher.iter(|| {
            let either: Either<String, i32> = serde_json::from_str(&right_json).unwrap();
            black_box(either)
        });
    });

    group.finish();
}

// =============================================================================
// Roundtrip Benchmark
// =============================================================================

fn benchmark_roundtrip(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("serde_roundtrip");

    for size in [100, 1000] {
        let persistent_list: PersistentList<i32> = (0..size).collect();
        let persistent_vector: PersistentVector<i32> = (0..size).collect();
        let persistent_hashmap: PersistentHashMap<String, i32> =
            (0..size).map(|i| (format!("key{i}"), i)).collect();

        group.bench_with_input(
            BenchmarkId::new("PersistentList", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let json = serde_json::to_string(&persistent_list).unwrap();
                    let restored: PersistentList<i32> = serde_json::from_str(&json).unwrap();
                    black_box(restored)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("PersistentVector", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let json = serde_json::to_string(&persistent_vector).unwrap();
                    let restored: PersistentVector<i32> = serde_json::from_str(&json).unwrap();
                    black_box(restored)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("PersistentHashMap", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let json = serde_json::to_string(&persistent_hashmap).unwrap();
                    let restored: PersistentHashMap<String, i32> =
                        serde_json::from_str(&json).unwrap();
                    black_box(restored)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_list_serialize,
    benchmark_list_deserialize,
    benchmark_vector_serialize,
    benchmark_vector_deserialize,
    benchmark_hashset_serialize,
    benchmark_hashset_deserialize,
    benchmark_hashmap_serialize,
    benchmark_hashmap_deserialize,
    benchmark_treemap_serialize,
    benchmark_treemap_deserialize,
    benchmark_either_serialize,
    benchmark_roundtrip,
);

criterion_main!(benches);
