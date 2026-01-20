use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use lambars::control::Trampoline;
use lambars::effect::{IO, Reader, State};
use lambars::for_;
use lambars::lens;
use lambars::optics::Lens;
use lambars::persistent::{PersistentHashMap, PersistentVector};
use lambars::pipe;
use lambars::typeclass::{Foldable, Functor};
use std::hint::black_box;

#[library_benchmark]
fn monad_transformer_chain() -> i32 {
    let reader: Reader<i32, i32> = Reader::ask().flat_map(|env| Reader::pure(env * 2));

    let state: State<i32, i32> = State::get()
        .flat_map(move |s| State::put(s + reader.run_cloned(black_box(10))).then(State::get()));

    let io = IO::pure(state.run(black_box(0))).fmap(|(result, _)| result);

    black_box(io.run_unsafe())
}

fn setup_vector_100() -> PersistentVector<i32> {
    (0..100).collect()
}

#[library_benchmark]
#[bench::with_setup(setup_vector_100())]
fn persistent_data_pipeline(vector: PersistentVector<i32>) -> Option<i32> {
    let vector = black_box(vector);
    let updated = vector
        .update(black_box(50), black_box(999))
        .expect("index valid")
        .push_back(black_box(100))
        .push_back(black_box(101));

    let map: PersistentHashMap<i32, i32> = updated
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as i32, v))
        .collect();

    black_box(map.get(&black_box(50)).copied())
}

fn setup_data_100() -> Vec<i32> {
    (0..100).collect()
}

#[library_benchmark]
#[bench::with_setup(setup_data_100())]
fn for_macro_pipeline(data: Vec<i32>) -> Vec<i32> {
    let data = black_box(data);
    let result = for_! {
        x <= data;
        let y = x * 2;
        let z = y + 1;
        yield z
    };
    black_box(result)
}

#[derive(Clone)]
enum TaskStatus {
    Todo,
    Done,
}

#[derive(Clone)]
#[allow(dead_code)]
struct Task {
    id: u64,
    title: String,
    status: TaskStatus,
}

fn setup_task() -> Task {
    Task {
        id: 1,
        title: "Test".to_string(),
        status: TaskStatus::Todo,
    }
}

#[library_benchmark]
#[bench::with_setup(setup_task())]
fn optics_update(task: Task) -> Task {
    let task = black_box(task);
    let title_lens = lens!(Task, title);
    let status_lens = lens!(Task, status);

    let updated = pipe!(task, |t| title_lens.set(t, "Updated".to_string()), |t| {
        status_lens.set(t, TaskStatus::Done)
    });
    black_box(updated)
}

fn sum_trampoline(n: i32, acc: i32) -> Trampoline<i32> {
    if n <= 0 {
        Trampoline::done(acc)
    } else {
        Trampoline::suspend(move || sum_trampoline(n - 1, acc + n))
    }
}

#[library_benchmark]
fn trampoline_recursion_1000() -> i32 {
    let n = black_box(1000);
    let acc = black_box(0);
    black_box(sum_trampoline(n, acc).run())
}

fn setup_vector_1000() -> PersistentVector<i32> {
    (0..1000).collect()
}

#[library_benchmark]
#[bench::with_setup(setup_vector_1000())]
fn foldable_aggregation(vector: PersistentVector<i32>) -> i32 {
    let vector = black_box(vector);
    let initial = black_box(0);
    black_box(vector.fold_left(initial, |acc, x| acc + x))
}

library_benchmark_group!(
    name = scenario_group;
    benchmarks = monad_transformer_chain, persistent_data_pipeline, for_macro_pipeline,
                 optics_update, trampoline_recursion_1000, foldable_aggregation
);

main!(library_benchmark_groups = scenario_group);
