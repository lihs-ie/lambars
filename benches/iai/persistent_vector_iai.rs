//! IAI-Callgrind benchmark for PersistentVector operations.
//!
//! Measures instruction counts for construction, access, update, and iteration operations.
//!
//! # Data Sizes
//!
//! - **get_sequential / update**: 100, 1000, 10000 (multi-size for regression detection)
//! - **from_vec / collect**: 100, 1000, 10000, 100000 (construction benchmarks)
//! - **push_back / iter**: 1000 (single size)
//!
//! # Design Notes
//!
//! - **FromIterator path**: For exact-size iterators with >= 64 elements, `collect()` uses
//!   `from_vec()` internally.
//!
//!   **Note on measurement scope difference**: `from_vec_*` uses `#[bench::with_setup]` to
//!   exclude Vec construction from measurement, while `collect_*` includes Range iteration
//!   and Vec construction in the measurement. Therefore, Ir values are NOT directly comparable.
//!   The `collect_*` benchmarks exist solely for tracking collection-based construction
//!   performance over time, not for comparing against `from_vec_*`.
//!
//! - **Setup functions**: The `#[bench::with_setup]` attribute ensures setup costs are NOT
//!   attributed to the benchmark measurement. This is confirmed by the iai-callgrind
//!   documentation: "the setup costs and event counts aren't attributed to the benchmark."
//!   See: <https://docs.rs/iai-callgrind/latest/iai_callgrind/>
//!
//! - **Multi-size operations**: get_sequential and update benchmarks at multiple sizes
//!   detect size-dependent performance characteristics and cache effects.
//!   See: docs/internal/analysis/REQ-BOTTLENECK-003-analysis.yaml

use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use lambars::persistent::PersistentVector;
use std::hint::black_box;

fn setup_vector_100() -> PersistentVector<i32> {
    (0..100).collect()
}

fn setup_vector_1000() -> PersistentVector<i32> {
    (0..1000).collect()
}

fn setup_vector_10000() -> PersistentVector<i32> {
    (0..10000).collect()
}

fn setup_vec_100() -> Vec<i32> {
    (0..100).collect()
}

fn setup_vec_1000() -> Vec<i32> {
    (0..1000).collect()
}

fn setup_vec_10000() -> Vec<i32> {
    (0..10000).collect()
}

fn setup_vec_100000() -> Vec<i32> {
    (0..100000).collect()
}

#[library_benchmark]
fn push_back_1000() -> PersistentVector<i32> {
    let mut vector = PersistentVector::new();
    for index in 0..1000 {
        vector = vector.push_back(black_box(index));
    }
    black_box(vector)
}

#[library_benchmark]
#[bench::with_setup(setup_vector_100())]
fn get_sequential_100(vector: PersistentVector<i32>) -> i32 {
    let vector = black_box(vector);
    let mut sum = 0;
    for index in 0..100 {
        if let Some(&value) = vector.get(black_box(index)) {
            sum += value;
        }
    }
    black_box(sum)
}

#[library_benchmark]
#[bench::with_setup(setup_vector_1000())]
fn get_sequential_1000(vector: PersistentVector<i32>) -> i32 {
    let vector = black_box(vector);
    let mut sum = 0;
    for index in 0..1000 {
        if let Some(&value) = vector.get(black_box(index)) {
            sum += value;
        }
    }
    black_box(sum)
}

#[library_benchmark]
#[bench::with_setup(setup_vector_10000())]
fn get_sequential_10000(vector: PersistentVector<i32>) -> i32 {
    let vector = black_box(vector);
    let mut sum = 0;
    for index in 0..10000 {
        if let Some(&value) = vector.get(black_box(index)) {
            sum += value;
        }
    }
    black_box(sum)
}

#[library_benchmark]
#[bench::with_setup(setup_vector_100())]
fn update_100(vector: PersistentVector<i32>) -> PersistentVector<i32> {
    let mut vector = black_box(vector);
    for index in 0..100 {
        if let Some(updated) = vector.update(black_box(index), black_box(index as i32 * 2)) {
            vector = updated;
        }
    }
    black_box(vector)
}

#[library_benchmark]
#[bench::with_setup(setup_vector_1000())]
fn update_1000(vector: PersistentVector<i32>) -> PersistentVector<i32> {
    let mut vector = black_box(vector);
    for index in 0..1000 {
        if let Some(updated) = vector.update(black_box(index), black_box(index as i32 * 2)) {
            vector = updated;
        }
    }
    black_box(vector)
}

#[library_benchmark]
#[bench::with_setup(setup_vector_10000())]
fn update_10000(vector: PersistentVector<i32>) -> PersistentVector<i32> {
    let mut vector = black_box(vector);
    for index in 0..10000 {
        if let Some(updated) = vector.update(black_box(index), black_box(index as i32 * 2)) {
            vector = updated;
        }
    }
    black_box(vector)
}

#[library_benchmark]
#[bench::with_setup(setup_vector_1000())]
fn iter_1000(vector: PersistentVector<i32>) -> i32 {
    black_box(black_box(vector).iter().sum())
}

#[library_benchmark]
#[bench::with_setup(setup_vec_100())]
fn from_vec_100(elements: Vec<i32>) -> PersistentVector<i32> {
    black_box(PersistentVector::from_vec(black_box(elements)))
}

#[library_benchmark]
#[bench::with_setup(setup_vec_1000())]
fn from_vec_1000(elements: Vec<i32>) -> PersistentVector<i32> {
    black_box(PersistentVector::from_vec(black_box(elements)))
}

#[library_benchmark]
#[bench::with_setup(setup_vec_10000())]
fn from_vec_10000(elements: Vec<i32>) -> PersistentVector<i32> {
    black_box(PersistentVector::from_vec(black_box(elements)))
}

#[library_benchmark]
#[bench::with_setup(setup_vec_100000())]
fn from_vec_100000(elements: Vec<i32>) -> PersistentVector<i32> {
    black_box(PersistentVector::from_vec(black_box(elements)))
}

#[library_benchmark]
fn collect_100() -> PersistentVector<i32> {
    black_box(black_box(0..100).collect::<PersistentVector<i32>>())
}

#[library_benchmark]
fn collect_1000() -> PersistentVector<i32> {
    black_box(black_box(0..1000).collect::<PersistentVector<i32>>())
}

#[library_benchmark]
fn collect_10000() -> PersistentVector<i32> {
    black_box(black_box(0..10000).collect::<PersistentVector<i32>>())
}

#[library_benchmark]
fn collect_100000() -> PersistentVector<i32> {
    black_box(black_box(0..100000).collect::<PersistentVector<i32>>())
}

library_benchmark_group!(
    name = persistent_vector_group;
    benchmarks =
        push_back_1000,
        get_sequential_100, get_sequential_1000, get_sequential_10000,
        update_100, update_1000, update_10000,
        iter_1000,
        from_vec_100, from_vec_1000, from_vec_10000, from_vec_100000,
        collect_100, collect_1000, collect_10000, collect_100000
);

main!(library_benchmark_groups = persistent_vector_group);
