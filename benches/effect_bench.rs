//! Benchmark for effect system: IO, AsyncIO, Reader, State.
//!
//! Measures the performance of lambars' effect monads.

use criterion::{Criterion, criterion_group, criterion_main};
use lambars::effect::{IO, Reader, State};
use std::hint::black_box;

// =============================================================================
// IO Benchmarks
// =============================================================================

fn benchmark_io_pure(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("io_pure");

    group.bench_function("pure", |bencher| {
        bencher.iter(|| {
            let io = IO::pure(black_box(42));
            let result = io.run_unsafe();
            black_box(result)
        });
    });

    group.bench_function("new", |bencher| {
        bencher.iter(|| {
            let io = IO::new(|| 42);
            let result = io.run_unsafe();
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_io_map_chain(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("io_map_chain");

    // Single map
    group.bench_function("map_1", |bencher| {
        bencher.iter(|| {
            let io = IO::pure(1).fmap(|x| x + 1);
            black_box(io.run_unsafe())
        });
    });

    // Chain of 5 maps
    group.bench_function("map_5", |bencher| {
        bencher.iter(|| {
            let io = IO::pure(1)
                .fmap(|x| x + 1)
                .fmap(|x| x * 2)
                .fmap(|x| x + 3)
                .fmap(|x| x * 4)
                .fmap(|x| x + 5);
            black_box(io.run_unsafe())
        });
    });

    // Chain of 10 maps
    group.bench_function("map_10", |bencher| {
        bencher.iter(|| {
            let io = IO::pure(1)
                .fmap(|x| x + 1)
                .fmap(|x| x * 2)
                .fmap(|x| x + 3)
                .fmap(|x| x * 4)
                .fmap(|x| x + 5)
                .fmap(|x| x - 1)
                .fmap(|x| x / 2)
                .fmap(|x| x + 7)
                .fmap(|x| x * 8)
                .fmap(|x| x - 9);
            black_box(io.run_unsafe())
        });
    });

    group.finish();
}

fn benchmark_io_flat_map_chain(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("io_flat_map_chain");

    // Single flat_map
    group.bench_function("flat_map_1", |bencher| {
        bencher.iter(|| {
            let io = IO::pure(1).flat_map(|x| IO::pure(x + 1));
            black_box(io.run_unsafe())
        });
    });

    // Chain of 5 flat_maps
    group.bench_function("flat_map_5", |bencher| {
        bencher.iter(|| {
            let io = IO::pure(1)
                .flat_map(|x| IO::pure(x + 1))
                .flat_map(|x| IO::pure(x * 2))
                .flat_map(|x| IO::pure(x + 3))
                .flat_map(|x| IO::pure(x * 4))
                .flat_map(|x| IO::pure(x + 5));
            black_box(io.run_unsafe())
        });
    });

    // Chain of 10 flat_maps
    group.bench_function("flat_map_10", |bencher| {
        bencher.iter(|| {
            let io = IO::pure(1)
                .flat_map(|x| IO::pure(x + 1))
                .flat_map(|x| IO::pure(x * 2))
                .flat_map(|x| IO::pure(x + 3))
                .flat_map(|x| IO::pure(x * 4))
                .flat_map(|x| IO::pure(x + 5))
                .flat_map(|x| IO::pure(x - 1))
                .flat_map(|x| IO::pure(x / 2))
                .flat_map(|x| IO::pure(x + 7))
                .flat_map(|x| IO::pure(x * 8))
                .flat_map(|x| IO::pure(x - 9));
            black_box(io.run_unsafe())
        });
    });

    group.finish();
}

// =============================================================================
// AsyncIO Benchmarks (requires tokio runtime)
// =============================================================================

fn benchmark_async_io_pure(criterion: &mut Criterion) {
    use lambars::effect::AsyncIO;

    let mut group = criterion.benchmark_group("async_io_pure");

    // Create tokio runtime for benchmarks
    let runtime = tokio::runtime::Runtime::new().unwrap();

    group.bench_function("pure", |bencher| {
        bencher.iter(|| {
            let async_io = AsyncIO::pure(black_box(42));
            let result = runtime.block_on(async_io.run_async());
            black_box(result)
        });
    });

    group.bench_function("new", |bencher| {
        bencher.iter(|| {
            let async_io = AsyncIO::new(|| async { 42 });
            let result = runtime.block_on(async_io.run_async());
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_async_io_chain(criterion: &mut Criterion) {
    use lambars::effect::AsyncIO;

    let mut group = criterion.benchmark_group("async_io_chain");

    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Chain of flat_maps
    group.bench_function("flat_map_5", |bencher| {
        bencher.iter(|| {
            let async_io = AsyncIO::pure(1)
                .flat_map(|x| AsyncIO::pure(x + 1))
                .flat_map(|x| AsyncIO::pure(x * 2))
                .flat_map(|x| AsyncIO::pure(x + 3))
                .flat_map(|x| AsyncIO::pure(x * 4))
                .flat_map(|x| AsyncIO::pure(x + 5));
            let result = runtime.block_on(async_io.run_async());
            black_box(result)
        });
    });

    group.finish();
}

// =============================================================================
// Reader Benchmarks
// =============================================================================

fn benchmark_reader_run(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("reader_run");

    // Simple reader
    group.bench_function("simple", |bencher| {
        bencher.iter(|| {
            let reader: Reader<i32, i32> = Reader::new(|environment| environment * 2);
            let result = reader.run(black_box(21));
            black_box(result)
        });
    });

    // Reader::ask
    group.bench_function("ask", |bencher| {
        bencher.iter(|| {
            let reader: Reader<i32, i32> = Reader::ask();
            let result = reader.run(black_box(42));
            black_box(result)
        });
    });

    // Reader::pure
    group.bench_function("pure", |bencher| {
        bencher.iter(|| {
            let reader: Reader<i32, i32> = Reader::pure(black_box(42));
            let result = reader.run(0);
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_reader_chain(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("reader_chain");

    // Chain of flat_maps
    group.bench_function("flat_map_chain", |bencher| {
        bencher.iter(|| {
            let reader: Reader<i32, i32> = Reader::ask()
                .flat_map(|x: i32| Reader::pure(x + 1))
                .flat_map(|x| Reader::pure(x * 2))
                .flat_map(|x| Reader::pure(x + 3));
            let result = reader.run(black_box(10));
            black_box(result)
        });
    });

    // fmap chain
    group.bench_function("fmap_chain", |bencher| {
        bencher.iter(|| {
            let reader: Reader<i32, i32> = Reader::ask()
                .fmap(|x| x + 1)
                .fmap(|x| x * 2)
                .fmap(|x| x + 3);
            let result = reader.run(black_box(10));
            black_box(result)
        });
    });

    // local
    group.bench_function("local", |bencher| {
        bencher.iter(|| {
            let reader: Reader<i32, i32> = Reader::ask();
            let local_reader = Reader::local(|x: i32| x * 2, reader);
            let result = local_reader.run(black_box(21));
            black_box(result)
        });
    });

    group.finish();
}

// =============================================================================
// State Benchmarks
// =============================================================================

fn benchmark_state_run(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("state_run");

    // Simple state
    group.bench_function("simple", |bencher| {
        bencher.iter(|| {
            let state: State<i32, i32> = State::new(|s: i32| (s * 2, s + 1));
            let (result, final_state) = state.run(black_box(10));
            black_box((result, final_state))
        });
    });

    // State::get
    group.bench_function("get", |bencher| {
        bencher.iter(|| {
            let state: State<i32, i32> = State::get();
            let (result, final_state) = state.run(black_box(42));
            black_box((result, final_state))
        });
    });

    // State::put
    group.bench_function("put", |bencher| {
        bencher.iter(|| {
            let state: State<i32, ()> = State::put(black_box(100));
            let (result, final_state) = state.run(0);
            black_box((result, final_state))
        });
    });

    // State::pure
    group.bench_function("pure", |bencher| {
        bencher.iter(|| {
            let state: State<i32, i32> = State::pure(black_box(42));
            let (result, final_state) = state.run(0);
            black_box((result, final_state))
        });
    });

    group.finish();
}

fn benchmark_state_chain(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("state_chain");

    // Chain of operations simulating a counter
    group.bench_function("counter_chain", |bencher| {
        bencher.iter(|| {
            let state = State::modify(|count: i32| count + 1)
                .then(State::modify(|count: i32| count + 1))
                .then(State::modify(|count: i32| count + 1))
                .then(State::get());
            let (result, final_state) = state.run(black_box(0));
            black_box((result, final_state))
        });
    });

    // flat_map chain
    group.bench_function("flat_map_chain", |bencher| {
        bencher.iter(|| {
            let state: State<i32, i32> = State::get()
                .flat_map(|current: i32| State::put(current + 1).then(State::pure(current)))
                .flat_map(|old| State::get().fmap(move |new| old + new));
            let (result, final_state) = state.run(black_box(10));
            black_box((result, final_state))
        });
    });

    // fmap chain
    group.bench_function("fmap_chain", |bencher| {
        bencher.iter(|| {
            let state: State<i32, i32> =
                State::get().fmap(|x| x + 1).fmap(|x| x * 2).fmap(|x| x + 3);
            let (result, final_state) = state.run(black_box(10));
            black_box((result, final_state))
        });
    });

    group.finish();
}

fn benchmark_state_modify(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("state_modify");

    // Single modify
    group.bench_function("modify_1", |bencher| {
        bencher.iter(|| {
            let state: State<i32, ()> = State::modify(|count| count + 1);
            let (result, final_state) = state.run(black_box(0));
            black_box((result, final_state))
        });
    });

    // Chain of 10 modifies
    group.bench_function("modify_10", |bencher| {
        bencher.iter(|| {
            let state: State<i32, ()> = State::modify(|count: i32| count + 1)
                .then(State::modify(|count| count + 1))
                .then(State::modify(|count| count + 1))
                .then(State::modify(|count| count + 1))
                .then(State::modify(|count| count + 1))
                .then(State::modify(|count| count + 1))
                .then(State::modify(|count| count + 1))
                .then(State::modify(|count| count + 1))
                .then(State::modify(|count| count + 1))
                .then(State::modify(|count| count + 1));
            let (result, final_state) = state.run(black_box(0));
            black_box((result, final_state))
        });
    });

    group.finish();
}

// =============================================================================
// Criterion Group and Main
// =============================================================================

criterion_group!(
    benches,
    benchmark_io_pure,
    benchmark_io_map_chain,
    benchmark_io_flat_map_chain,
    benchmark_async_io_pure,
    benchmark_async_io_chain,
    benchmark_reader_run,
    benchmark_reader_chain,
    benchmark_state_run,
    benchmark_state_chain,
    benchmark_state_modify
);

criterion_main!(benches);
