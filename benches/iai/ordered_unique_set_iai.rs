//! IAI-Callgrind benchmark for OrderedUniqueSet bulk construction APIs.
//!
//! Measures instruction counts for bulk construction methods vs incremental insert.
//! Data sizes: 100 (Small state), 1000/10000/100000 (Large state).

use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use lambars::persistent::OrderedUniqueSet;
use std::hint::black_box;

// Setup functions for different data sizes
fn setup_sorted_vec_100() -> Vec<i32> {
    (0..100).collect()
}

fn setup_sorted_vec_1000() -> Vec<i32> {
    (0..1000).collect()
}

fn setup_sorted_vec_10000() -> Vec<i32> {
    (0..10000).collect()
}

fn setup_sorted_vec_100000() -> Vec<i32> {
    (0..100000).collect()
}

// from_sorted_iter benchmarks
#[library_benchmark]
#[bench::with_setup(setup_sorted_vec_100())]
fn from_sorted_iter_100(elements: Vec<i32>) -> OrderedUniqueSet<i32> {
    black_box(OrderedUniqueSet::from_sorted_iter(black_box(elements)))
}

#[library_benchmark]
#[bench::with_setup(setup_sorted_vec_1000())]
fn from_sorted_iter_1000(elements: Vec<i32>) -> OrderedUniqueSet<i32> {
    black_box(OrderedUniqueSet::from_sorted_iter(black_box(elements)))
}

#[library_benchmark]
#[bench::with_setup(setup_sorted_vec_10000())]
fn from_sorted_iter_10000(elements: Vec<i32>) -> OrderedUniqueSet<i32> {
    black_box(OrderedUniqueSet::from_sorted_iter(black_box(elements)))
}

#[library_benchmark]
#[bench::with_setup(setup_sorted_vec_100000())]
fn from_sorted_iter_100000(elements: Vec<i32>) -> OrderedUniqueSet<i32> {
    black_box(OrderedUniqueSet::from_sorted_iter(black_box(elements)))
}

// from_sorted_vec benchmarks
#[library_benchmark]
#[bench::with_setup(setup_sorted_vec_100())]
fn from_sorted_vec_100(elements: Vec<i32>) -> OrderedUniqueSet<i32> {
    black_box(OrderedUniqueSet::from_sorted_vec(black_box(elements)))
}

#[library_benchmark]
#[bench::with_setup(setup_sorted_vec_1000())]
fn from_sorted_vec_1000(elements: Vec<i32>) -> OrderedUniqueSet<i32> {
    black_box(OrderedUniqueSet::from_sorted_vec(black_box(elements)))
}

#[library_benchmark]
#[bench::with_setup(setup_sorted_vec_10000())]
fn from_sorted_vec_10000(elements: Vec<i32>) -> OrderedUniqueSet<i32> {
    black_box(OrderedUniqueSet::from_sorted_vec(black_box(elements)))
}

#[library_benchmark]
#[bench::with_setup(setup_sorted_vec_100000())]
fn from_sorted_vec_100000(elements: Vec<i32>) -> OrderedUniqueSet<i32> {
    black_box(OrderedUniqueSet::from_sorted_vec(black_box(elements)))
}

// fold + insert benchmarks (baseline for comparison)
#[library_benchmark]
#[bench::with_setup(setup_sorted_vec_100())]
fn fold_insert_100(elements: Vec<i32>) -> OrderedUniqueSet<i32> {
    black_box(
        black_box(elements)
            .into_iter()
            .fold(OrderedUniqueSet::new(), |accumulator, element| {
                accumulator.insert(black_box(element))
            }),
    )
}

#[library_benchmark]
#[bench::with_setup(setup_sorted_vec_1000())]
fn fold_insert_1000(elements: Vec<i32>) -> OrderedUniqueSet<i32> {
    black_box(
        black_box(elements)
            .into_iter()
            .fold(OrderedUniqueSet::new(), |accumulator, element| {
                accumulator.insert(black_box(element))
            }),
    )
}

#[library_benchmark]
#[bench::with_setup(setup_sorted_vec_10000())]
fn fold_insert_10000(elements: Vec<i32>) -> OrderedUniqueSet<i32> {
    black_box(
        black_box(elements)
            .into_iter()
            .fold(OrderedUniqueSet::new(), |accumulator, element| {
                accumulator.insert(black_box(element))
            }),
    )
}

#[library_benchmark]
#[bench::with_setup(setup_sorted_vec_100000())]
fn fold_insert_100000(elements: Vec<i32>) -> OrderedUniqueSet<i32> {
    black_box(
        black_box(elements)
            .into_iter()
            .fold(OrderedUniqueSet::new(), |accumulator, element| {
                accumulator.insert(black_box(element))
            }),
    )
}

library_benchmark_group!(
    name = ordered_unique_set_bulk_group;
    benchmarks =
        from_sorted_iter_100, from_sorted_iter_1000, from_sorted_iter_10000, from_sorted_iter_100000,
        from_sorted_vec_100, from_sorted_vec_1000, from_sorted_vec_10000, from_sorted_vec_100000,
        fold_insert_100, fold_insert_1000, fold_insert_10000, fold_insert_100000
);

main!(library_benchmark_groups = ordered_unique_set_bulk_group);
