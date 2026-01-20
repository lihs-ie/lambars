use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use lambars::eff;
use lambars::effect::{ExceptT, IO, Reader, State};
use lambars::typeclass::Monad;
use std::hint::black_box;

#[library_benchmark]
fn io_pure_chain_10() -> i32 {
    let initial = black_box(1);
    let io = IO::pure(initial)
        .flat_map(|x| IO::pure(x + 1))
        .flat_map(|x| IO::pure(x * 2))
        .flat_map(|x| IO::pure(x + 1))
        .flat_map(|x| IO::pure(x * 2))
        .flat_map(|x| IO::pure(x + 1))
        .flat_map(|x| IO::pure(x * 2))
        .flat_map(|x| IO::pure(x + 1))
        .flat_map(|x| IO::pure(x * 2))
        .flat_map(|x| IO::pure(x + 1))
        .flat_map(|x| IO::pure(x + 10));
    black_box(io.run_unsafe())
}

#[library_benchmark]
fn reader_chain_10() -> i32 {
    let reader: Reader<i32, i32> = Reader::ask()
        .flat_map(|x: i32| Reader::pure(x + 1))
        .flat_map(|x| Reader::pure(x * 2))
        .flat_map(|x| Reader::pure(x + 1))
        .flat_map(|x| Reader::pure(x * 2))
        .flat_map(|x| Reader::pure(x + 1))
        .flat_map(|x| Reader::pure(x * 2))
        .flat_map(|x| Reader::pure(x + 1))
        .flat_map(|x| Reader::pure(x * 2))
        .flat_map(|x| Reader::pure(x + 1))
        .flat_map(|x| Reader::pure(x + 10));
    black_box(reader.run(black_box(10)))
}

#[library_benchmark]
fn state_chain_10() -> (i32, i32) {
    let state: State<i32, i32> = State::get()
        .flat_map(|s: i32| State::put(s + 1).then(State::get()))
        .flat_map(|s| State::put(s * 2).then(State::get()))
        .flat_map(|s| State::put(s + 1).then(State::get()))
        .flat_map(|s| State::put(s * 2).then(State::get()))
        .flat_map(|s| State::put(s + 1).then(State::get()))
        .flat_map(|s| State::put(s * 2).then(State::get()))
        .flat_map(|s| State::put(s + 1).then(State::get()))
        .flat_map(|s| State::put(s * 2).then(State::get()))
        .flat_map(|s| State::put(s + 1).then(State::get()))
        .flat_map(State::pure);
    black_box(state.run(black_box(0)))
}

#[library_benchmark]
fn exceptt_chain_10() -> Result<i32, String> {
    let initial = black_box(1);
    let exceptt: ExceptT<String, Option<Result<i32, String>>> = ExceptT::pure_option(initial)
        .flat_map_option(|x| ExceptT::pure_option(x + 1))
        .flat_map_option(|x| ExceptT::pure_option(x * 2))
        .flat_map_option(|x| ExceptT::pure_option(x + 1))
        .flat_map_option(|x| ExceptT::pure_option(x * 2))
        .flat_map_option(|x| ExceptT::pure_option(x + 1))
        .flat_map_option(|x| ExceptT::pure_option(x * 2))
        .flat_map_option(|x| ExceptT::pure_option(x + 1))
        .flat_map_option(|x| ExceptT::pure_option(x * 2))
        .flat_map_option(|x| ExceptT::pure_option(x + 1))
        .flat_map_option(|x| ExceptT::pure_option(x + 10));
    black_box(exceptt.run().expect("Option should be Some"))
}

#[library_benchmark]
fn eff_macro_10() -> i32 {
    let initial = black_box(1);
    let io: IO<i32> = eff! {
        x <= IO::pure(initial);
        y <= IO::pure(x + 1);
        z <= IO::pure(y * 2);
        a <= IO::pure(z + 1);
        b <= IO::pure(a * 2);
        c <= IO::pure(b + 1);
        d <= IO::pure(c * 2);
        e <= IO::pure(d + 1);
        f <= IO::pure(e * 2);
        g <= IO::pure(f + 1);
        IO::pure(g + 10)
    };
    black_box(io.run_unsafe())
}

// =============================================================================
// AsyncIO Sync Execution Benchmarks
// =============================================================================

#[library_benchmark]
fn async_io_run_sync_lightweight() -> i32 {
    use lambars::effect::AsyncIO;

    let initial = black_box(1);
    let async_io = AsyncIO::pure(initial)
        .flat_map(|x| AsyncIO::pure(x + 1))
        .flat_map(|x| AsyncIO::pure(x * 2))
        .flat_map(|x| AsyncIO::pure(x + 1))
        .flat_map(|x| AsyncIO::pure(x * 2))
        .flat_map(|x| AsyncIO::pure(x + 1))
        .flat_map(|x| AsyncIO::pure(x * 2))
        .flat_map(|x| AsyncIO::pure(x + 1))
        .flat_map(|x| AsyncIO::pure(x * 2))
        .flat_map(|x| AsyncIO::pure(x + 1))
        .flat_map(|x| AsyncIO::pure(x + 10));
    black_box(async_io.run_sync_lightweight())
}

#[library_benchmark]
fn async_io_run_sync() -> i32 {
    use lambars::effect::AsyncIO;

    let initial = black_box(1);
    let async_io = AsyncIO::pure(initial)
        .flat_map(|x| AsyncIO::pure(x + 1))
        .flat_map(|x| AsyncIO::pure(x * 2))
        .flat_map(|x| AsyncIO::pure(x + 1))
        .flat_map(|x| AsyncIO::pure(x * 2))
        .flat_map(|x| AsyncIO::pure(x + 1))
        .flat_map(|x| AsyncIO::pure(x * 2))
        .flat_map(|x| AsyncIO::pure(x + 1))
        .flat_map(|x| AsyncIO::pure(x * 2))
        .flat_map(|x| AsyncIO::pure(x + 1))
        .flat_map(|x| AsyncIO::pure(x + 10));
    black_box(async_io.run_sync())
}

#[library_benchmark]
fn async_io_to_sync_lightweight() -> i32 {
    use lambars::effect::AsyncIO;

    let initial = black_box(1);
    let async_io = AsyncIO::pure(initial)
        .flat_map(|x| AsyncIO::pure(x + 1))
        .flat_map(|x| AsyncIO::pure(x * 2))
        .flat_map(|x| AsyncIO::pure(x + 1))
        .flat_map(|x| AsyncIO::pure(x * 2))
        .flat_map(|x| AsyncIO::pure(x + 1))
        .flat_map(|x| AsyncIO::pure(x * 2))
        .flat_map(|x| AsyncIO::pure(x + 1))
        .flat_map(|x| AsyncIO::pure(x * 2))
        .flat_map(|x| AsyncIO::pure(x + 1))
        .flat_map(|x| AsyncIO::pure(x + 10));
    let io = async_io.to_sync_lightweight();
    black_box(io.run_unsafe())
}

#[library_benchmark]
fn async_io_to_sync() -> i32 {
    use lambars::effect::AsyncIO;

    let initial = black_box(1);
    let async_io = AsyncIO::pure(initial)
        .flat_map(|x| AsyncIO::pure(x + 1))
        .flat_map(|x| AsyncIO::pure(x * 2))
        .flat_map(|x| AsyncIO::pure(x + 1))
        .flat_map(|x| AsyncIO::pure(x * 2))
        .flat_map(|x| AsyncIO::pure(x + 1))
        .flat_map(|x| AsyncIO::pure(x * 2))
        .flat_map(|x| AsyncIO::pure(x + 1))
        .flat_map(|x| AsyncIO::pure(x * 2))
        .flat_map(|x| AsyncIO::pure(x + 1))
        .flat_map(|x| AsyncIO::pure(x + 10));
    let io = async_io.to_sync();
    black_box(io.run_unsafe())
}

library_benchmark_group!(
    name = effect_group;
    benchmarks = io_pure_chain_10, reader_chain_10, state_chain_10, exceptt_chain_10, eff_macro_10
);

library_benchmark_group!(
    name = async_io_sync_group;
    benchmarks = async_io_run_sync_lightweight, async_io_run_sync, async_io_to_sync_lightweight, async_io_to_sync
);

main!(library_benchmark_groups = effect_group, async_io_sync_group);
