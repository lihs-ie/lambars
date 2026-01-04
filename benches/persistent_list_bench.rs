//! Benchmark for PersistentList vs standard VecDeque.
//!
//! Compares the performance of lambars' PersistentList against Rust's standard VecDeque
//! for common operations.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lambars::persistent::PersistentList;
use std::collections::VecDeque;
use std::hint::black_box;

// =============================================================================
// cons Benchmark (prepend)
// =============================================================================

fn benchmark_cons(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("cons");

    for size in [100, 1000, 10000] {
        // PersistentList cons (O(1))
        group.bench_with_input(
            BenchmarkId::new("PersistentList", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut list = PersistentList::new();
                    for index in 0..size {
                        list = list.cons(black_box(index));
                    }
                    black_box(list)
                });
            },
        );

        // VecDeque push_front
        group.bench_with_input(
            BenchmarkId::new("VecDeque", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut deque = VecDeque::new();
                    for index in 0..size {
                        deque.push_front(black_box(index));
                    }
                    black_box(deque)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// head/tail Benchmark
// =============================================================================

fn benchmark_head_tail(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("head_tail");

    for size in [100, 1000, 10000] {
        // Prepare data
        let persistent_list: PersistentList<i32> = (0..size).collect();
        let standard_deque: VecDeque<i32> = (0..size).collect();

        // PersistentList head (O(1))
        group.bench_with_input(
            BenchmarkId::new("PersistentList_head", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let head = persistent_list.head();
                    black_box(head)
                });
            },
        );

        // VecDeque front (O(1))
        group.bench_with_input(
            BenchmarkId::new("VecDeque_front", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let front = standard_deque.front();
                    black_box(front)
                });
            },
        );

        // PersistentList tail (O(1))
        group.bench_with_input(
            BenchmarkId::new("PersistentList_tail", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let tail = persistent_list.tail();
                    black_box(tail)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// consume (head + tail repeatedly) Benchmark
// =============================================================================

fn benchmark_consume(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("consume");

    for size in [100, 1000] {
        // Prepare data
        let persistent_list: PersistentList<i32> = (0..size).collect();
        let standard_deque: VecDeque<i32> = (0..size).collect();

        // PersistentList consume via head/tail
        group.bench_with_input(
            BenchmarkId::new("PersistentList", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let mut sum = 0;
                    let mut current = persistent_list.clone();
                    while let Some(&head) = current.head() {
                        sum += head;
                        current = current.tail();
                    }
                    black_box(sum)
                });
            },
        );

        // VecDeque consume via pop_front (clone first for fair comparison)
        group.bench_with_input(BenchmarkId::new("VecDeque", size), &size, |bencher, _| {
            bencher.iter(|| {
                let mut sum = 0;
                let mut deque = standard_deque.clone();
                while let Some(value) = deque.pop_front() {
                    sum += value;
                }
                black_box(sum)
            });
        });
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
        let persistent_list: PersistentList<i32> = (0..size).collect();
        let standard_deque: VecDeque<i32> = (0..size).collect();

        // PersistentList iteration
        group.bench_with_input(
            BenchmarkId::new("PersistentList", size),
            &size,
            |bencher, _| {
                bencher.iter(|| {
                    let sum: i32 = persistent_list.iter().sum();
                    black_box(sum)
                });
            },
        );

        // VecDeque iteration
        group.bench_with_input(BenchmarkId::new("VecDeque", size), &size, |bencher, _| {
            bencher.iter(|| {
                let sum: i32 = standard_deque.iter().sum();
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
        // PersistentList from_iter
        group.bench_with_input(
            BenchmarkId::new("PersistentList", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let list: PersistentList<i32> = (0..size).collect();
                    black_box(list)
                });
            },
        );

        // VecDeque from_iter
        group.bench_with_input(
            BenchmarkId::new("VecDeque", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let deque: VecDeque<i32> = (0..size).collect();
                    black_box(deque)
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
    benchmark_cons,
    benchmark_head_tail,
    benchmark_consume,
    benchmark_iteration,
    benchmark_from_iter
);

criterion_main!(benches);
