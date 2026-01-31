//! IAI-Callgrind benchmark for PersistentVector operations.
//!
//! Measures instruction counts for various operations including bulk construction.
//! Data sizes for bulk construction: 100, 1000, 10000, 100000.

use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use lambars::persistent::PersistentVector;
use std::hint::black_box;

fn setup_vector_1000() -> PersistentVector<i32> {
    (0..1000).collect()
}

// Setup functions for bulk construction benchmarks
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
#[bench::with_setup(setup_vector_1000())]
fn iter_1000(vector: PersistentVector<i32>) -> i32 {
    black_box(black_box(vector).iter().sum())
}

// from_vec benchmarks at different sizes
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

// collect benchmarks at different sizes (baseline for comparison)
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
        push_back_1000, get_sequential_1000, update_1000, iter_1000,
        from_vec_100, from_vec_1000, from_vec_10000, from_vec_100000,
        collect_100, collect_1000, collect_10000, collect_100000
);

main!(library_benchmark_groups = persistent_vector_group);
