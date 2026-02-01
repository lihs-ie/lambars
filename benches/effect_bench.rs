//! Benchmark for effect system: IO, AsyncIO, Reader, State.
//!
//! Measures the performance of lambars' effect monads.

#![allow(deprecated)]

use criterion::{Criterion, criterion_group, criterion_main};
use lambars::effect::{IO, Reader, State};
use lambars::typeclass::{Functor, Monad};
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
            let result = runtime.block_on(async_io);
            black_box(result)
        });
    });

    group.bench_function("new", |bencher| {
        bencher.iter(|| {
            let async_io = AsyncIO::new(|| async { 42 });
            let result = runtime.block_on(async_io);
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
            let result = runtime.block_on(async_io);
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
// AsyncIO Control Flow Benchmarks
// =============================================================================

fn benchmark_async_io_retry(criterion: &mut Criterion) {
    use lambars::effect::AsyncIO;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let mut group = criterion.benchmark_group("async_io_retry");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    // retry_with_factory: success on first attempt (no actual retry)
    group.bench_function("retry_success_first", |bencher| {
        bencher.iter(|| {
            let result = runtime.block_on(async {
                AsyncIO::retry_with_factory(|| AsyncIO::pure(Ok::<i32, &str>(42)), 3).await
            });
            black_box(result)
        });
    });

    // retry_with_factory: success after 2 failures
    group.bench_function("retry_success_third", |bencher| {
        bencher.iter(|| {
            let counter = Arc::new(AtomicUsize::new(0));
            let counter_clone = counter.clone();

            let result = runtime.block_on(async {
                AsyncIO::retry_with_factory(
                    move || {
                        let c = counter_clone.clone();
                        AsyncIO::new(move || {
                            let c = c.clone();
                            async move {
                                let count = c.fetch_add(1, Ordering::SeqCst);
                                if count < 2 { Err("fail") } else { Ok(42) }
                            }
                        })
                    },
                    5,
                )
                .await
            });
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_async_io_par(criterion: &mut Criterion) {
    use lambars::effect::AsyncIO;

    let mut group = criterion.benchmark_group("async_io_par");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    // par: two pure values
    group.bench_function("par_pure_2", |bencher| {
        bencher.iter(|| {
            let result = runtime.block_on(async { AsyncIO::pure(1).par(AsyncIO::pure(2)).await });
            black_box(result)
        });
    });

    // par3: three pure values
    group.bench_function("par3_pure_3", |bencher| {
        bencher.iter(|| {
            let result = runtime.block_on(async {
                AsyncIO::pure(1)
                    .par3(AsyncIO::pure(2), AsyncIO::pure(3))
                    .await
            });
            black_box(result)
        });
    });

    // race_result: two pure values
    group.bench_function("race_result_pure", |bencher| {
        bencher.iter(|| {
            let result =
                runtime.block_on(async { AsyncIO::pure(1).race_result(AsyncIO::pure(2)).await });
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_async_io_bracket(criterion: &mut Criterion) {
    use lambars::effect::AsyncIO;

    let mut group = criterion.benchmark_group("async_io_bracket");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    // bracket: simple acquire-use-release
    group.bench_function("bracket_simple", |bencher| {
        bencher.iter(|| {
            let result = runtime.block_on(async {
                AsyncIO::bracket(
                    || AsyncIO::pure(42),
                    |resource| AsyncIO::pure(resource * 2),
                    |_| AsyncIO::pure(()),
                )
                .await
            });
            black_box(result)
        });
    });

    // finally_async: simple cleanup
    group.bench_function("finally_async_simple", |bencher| {
        bencher.iter(|| {
            let result =
                runtime.block_on(async { AsyncIO::pure(42).finally_async(|| async {}).await });
            black_box(result)
        });
    });

    // on_error: success case (callback not called)
    group.bench_function("on_error_success", |bencher| {
        bencher.iter(|| {
            let result = runtime.block_on(async {
                AsyncIO::pure(Ok::<i32, &str>(42))
                    .on_error(|_| async {})
                    .await
            });
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_async_io_timeout(criterion: &mut Criterion) {
    use lambars::effect::AsyncIO;
    use std::time::Duration;

    let mut group = criterion.benchmark_group("async_io_timeout");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    // timeout_result: completes before timeout
    group.bench_function("timeout_result_success", |bencher| {
        bencher.iter(|| {
            let result = runtime.block_on(async {
                AsyncIO::pure(42)
                    .timeout_result(Duration::from_secs(10))
                    .await
            });
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_async_io_overhead_comparison(criterion: &mut Criterion) {
    use lambars::effect::AsyncIO;

    let mut group = criterion.benchmark_group("async_io_overhead");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Baseline: pure value
    group.bench_function("baseline_pure", |bencher| {
        bencher.iter(|| {
            let result = runtime.block_on(async { AsyncIO::pure(42).await });
            black_box(result)
        });
    });

    // With finally_async (overhead of catch_unwind)
    group.bench_function("with_finally_async", |bencher| {
        bencher.iter(|| {
            let result =
                runtime.block_on(async { AsyncIO::pure(42).finally_async(|| async {}).await });
            black_box(result)
        });
    });

    // With on_error (overhead of pattern matching)
    group.bench_function("with_on_error", |bencher| {
        bencher.iter(|| {
            let result = runtime.block_on(async {
                AsyncIO::pure(Ok::<i32, &str>(42))
                    .on_error(|_| async {})
                    .await
            });
            black_box(result)
        });
    });

    // With retry (no actual retry, just setup overhead)
    group.bench_function("with_retry_no_retry", |bencher| {
        bencher.iter(|| {
            let result = runtime.block_on(async {
                AsyncIO::retry_with_factory(|| AsyncIO::pure(Ok::<i32, &str>(42)), 1).await
            });
            black_box(result)
        });
    });

    // map chain on Pure: measures eager evaluation optimization
    // Pure(x).fmap(f).fmap(g).fmap(h) should be equivalent to Pure(h(g(f(x))))
    // with no intermediate Box allocations
    group.bench_function("map_chain_pure_3", |bencher| {
        bencher.iter(|| {
            let result = runtime.block_on(async {
                AsyncIO::pure(1)
                    .fmap(|x| x + 1)
                    .fmap(|x| x * 2)
                    .fmap(|x| x + 10)
                    .await
            });
            black_box(result)
        });
    });

    // map chain on Pure: 10 chained fmaps
    group.bench_function("map_chain_pure_10", |bencher| {
        bencher.iter(|| {
            let result = runtime.block_on(async {
                AsyncIO::pure(0)
                    .fmap(|x| x + 1)
                    .fmap(|x| x + 1)
                    .fmap(|x| x + 1)
                    .fmap(|x| x + 1)
                    .fmap(|x| x + 1)
                    .fmap(|x| x + 1)
                    .fmap(|x| x + 1)
                    .fmap(|x| x + 1)
                    .fmap(|x| x + 1)
                    .fmap(|x| x + 1)
                    .await
            });
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_async_io_batch_run(criterion: &mut Criterion) {
    use lambars::effect::AsyncIO;

    let mut group = criterion.benchmark_group("async_io_batch_run");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    // batch_run: 10 pure items
    group.bench_function("batch_run_10", |bencher| {
        bencher.iter(|| {
            let items: Vec<AsyncIO<i32>> = (0..10).map(AsyncIO::pure).collect();
            let result = runtime.block_on(async { AsyncIO::batch_run(items).await });
            black_box(result)
        });
    });

    // batch_run: 100 pure items
    group.bench_function("batch_run_100", |bencher| {
        bencher.iter(|| {
            let items: Vec<AsyncIO<i32>> = (0..100).map(AsyncIO::pure).collect();
            let result = runtime.block_on(async { AsyncIO::batch_run(items).await });
            black_box(result)
        });
    });

    // batch_run: 1000 pure items
    group.bench_function("batch_run_1000", |bencher| {
        bencher.iter(|| {
            let items: Vec<AsyncIO<i32>> = (0..1000).map(AsyncIO::pure).collect();
            let result = runtime.block_on(async { AsyncIO::batch_run(items).await });
            black_box(result)
        });
    });

    // batch_run_buffered: 100 items with limit 10
    group.bench_function("batch_run_buffered_100_limit_10", |bencher| {
        bencher.iter(|| {
            let items: Vec<AsyncIO<i32>> = (0..100).map(AsyncIO::pure).collect();
            let result =
                runtime.block_on(async { AsyncIO::batch_run_buffered(items, 10).await.unwrap() });
            black_box(result)
        });
    });

    // batch_run_buffered: 100 items with limit 50
    group.bench_function("batch_run_buffered_100_limit_50", |bencher| {
        bencher.iter(|| {
            let items: Vec<AsyncIO<i32>> = (0..100).map(AsyncIO::pure).collect();
            let result =
                runtime.block_on(async { AsyncIO::batch_run_buffered(items, 50).await.unwrap() });
            black_box(result)
        });
    });

    // batch_run_buffered: 1000 items with limit 100
    group.bench_function("batch_run_buffered_1000_limit_100", |bencher| {
        bencher.iter(|| {
            let items: Vec<AsyncIO<i32>> = (0..1000).map(AsyncIO::pure).collect();
            let result =
                runtime.block_on(async { AsyncIO::batch_run_buffered(items, 100).await.unwrap() });
            black_box(result)
        });
    });

    // Comparison: sequential vs batch for 10 items
    group.bench_function("sequential_10_vs_batch", |bencher| {
        bencher.iter(|| {
            let result = runtime.block_on(async {
                let mut results = Vec::with_capacity(10);
                for i in 0..10 {
                    results.push(AsyncIO::pure(i).await);
                }
                results
            });
            black_box(result)
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
    benchmark_state_modify,
    benchmark_async_io_retry,
    benchmark_async_io_par,
    benchmark_async_io_bracket,
    benchmark_async_io_timeout,
    benchmark_async_io_overhead_comparison,
    benchmark_async_io_batch_run
);

criterion_main!(benches);
