use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use lambars::persistent::PersistentVector;
use std::hint::black_box;

fn setup_vector_1000() -> PersistentVector<i32> {
    (0..1000).collect()
}

#[library_benchmark]
fn push_back_1000() -> PersistentVector<i32> {
    let mut vector = PersistentVector::new();
    for i in 0..1000 {
        vector = vector.push_back(black_box(i));
    }
    black_box(vector)
}

#[library_benchmark]
#[bench::with_setup(setup_vector_1000())]
fn get_sequential_1000(vector: PersistentVector<i32>) -> i32 {
    let vector = black_box(vector);
    let mut sum = 0;
    for i in 0..1000 {
        if let Some(&v) = vector.get(black_box(i)) {
            sum += v;
        }
    }
    black_box(sum)
}

#[library_benchmark]
#[bench::with_setup(setup_vector_1000())]
fn update_1000(vector: PersistentVector<i32>) -> PersistentVector<i32> {
    let mut vector = black_box(vector);
    for i in 0..1000 {
        if let Some(updated) = vector.update(black_box(i), black_box(i as i32 * 2)) {
            vector = updated;
        }
    }
    black_box(vector)
}

#[library_benchmark]
#[bench::with_setup(setup_vector_1000())]
fn iter_1000(vector: PersistentVector<i32>) -> i32 {
    let vector = black_box(vector);
    black_box(vector.iter().sum())
}

library_benchmark_group!(
    name = persistent_vector_group;
    benchmarks = push_back_1000, get_sequential_1000, update_1000, iter_1000
);

main!(library_benchmark_groups = persistent_vector_group);
