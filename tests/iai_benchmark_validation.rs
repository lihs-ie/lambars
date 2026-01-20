//! Validation tests for Iai-Callgrind benchmark functions.
//!
//! These tests verify that the benchmark functions produce correct results.

use lambars::control::Trampoline;
use lambars::eff;
use lambars::effect::{ExceptT, IO, Reader, State};
use lambars::for_;
use lambars::lens;
use lambars::optics::Lens;
use lambars::persistent::{PersistentHashMap, PersistentVector};
use lambars::pipe;
use lambars::typeclass::{Foldable, Functor, Monad};
use rstest::rstest;
use std::hint::black_box;

// =============================================================================
// persistent_vector_iai.rs validation
// =============================================================================

#[rstest]
fn test_push_back_1000() {
    let mut vector = PersistentVector::new();
    for i in 0..1000 {
        vector = vector.push_back(black_box(i));
    }
    let result = black_box(vector);
    assert_eq!(result.len(), 1000);
    assert_eq!(result.get(0), Some(&0));
    assert_eq!(result.get(999), Some(&999));
}

#[rstest]
fn test_get_sequential_1000() {
    let vector: PersistentVector<i32> = (0..1000).collect();
    let vector = black_box(vector);
    let mut sum = 0;
    for i in 0..1000 {
        if let Some(&v) = vector.get(black_box(i)) {
            sum += v;
        }
    }
    let result = black_box(sum);
    // Sum of 0..1000 = 499500
    assert_eq!(result, 499500);
}

#[rstest]
fn test_update_1000() {
    let vector: PersistentVector<i32> = (0..1000).collect();
    let mut vector = black_box(vector);
    for i in 0..1000 {
        if let Some(updated) = vector.update(black_box(i), black_box(i as i32 * 2)) {
            vector = updated;
        }
    }
    let result = black_box(vector);
    assert_eq!(result.get(0), Some(&0));
    assert_eq!(result.get(500), Some(&1000));
    assert_eq!(result.get(999), Some(&1998));
}

#[rstest]
fn test_iter_1000() {
    let vector: PersistentVector<i32> = (0..1000).collect();
    let vector = black_box(vector);
    let result = black_box(vector.iter().sum::<i32>());
    assert_eq!(result, 499500);
}

// =============================================================================
// effect_iai.rs validation
// =============================================================================

#[rstest]
fn test_io_pure_chain_10() {
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
    let result = black_box(io.run_unsafe());
    // 1 -> 2 -> 4 -> 5 -> 10 -> 11 -> 22 -> 23 -> 46 -> 47 -> 57
    assert_eq!(result, 57);
}

#[rstest]
fn test_reader_chain_10() {
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
    let result = black_box(reader.run(black_box(10)));
    // 10 -> 11 -> 22 -> 23 -> 46 -> 47 -> 94 -> 95 -> 190 -> 191 -> 201
    assert_eq!(result, 201);
}

#[rstest]
fn test_state_chain_10() {
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
    let (result, final_state) = black_box(state.run(black_box(0)));
    // Starting from 0:
    // 0 -> 1 -> 2 -> 3 -> 6 -> 7 -> 14 -> 15 -> 30 -> 31
    assert_eq!(result, 31);
    assert_eq!(final_state, 31);
}

#[rstest]
fn test_exceptt_chain_10() {
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
    let result = black_box(exceptt.run().expect("Option should be Some"));
    // Same as io_pure_chain_10: 57
    assert_eq!(result, Ok(57));
}

#[rstest]
fn test_eff_macro_10() {
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
    let result = black_box(io.run_unsafe());
    // Same as io_pure_chain_10: 57
    assert_eq!(result, 57);
}

// =============================================================================
// scenario_iai.rs validation
// =============================================================================

#[rstest]
fn test_monad_transformer_chain() {
    let reader: Reader<i32, i32> = Reader::ask().flat_map(|env| Reader::pure(env * 2));

    let state: State<i32, i32> = State::get()
        .flat_map(move |s| State::put(s + reader.run_cloned(black_box(10))).then(State::get()));

    let io = IO::pure(state.run(black_box(0))).fmap(|(result, _)| result);

    let result = black_box(io.run_unsafe());
    // reader.run(10) = 10 * 2 = 20
    // state: 0 -> put(0 + 20) -> get() = 20
    assert_eq!(result, 20);
}

#[rstest]
fn test_persistent_data_pipeline() {
    let vector: PersistentVector<i32> = (0..100).collect();
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

    let result = black_box(map.get(&black_box(50)).copied());
    assert_eq!(result, Some(999));
}

#[rstest]
fn test_for_macro_pipeline() {
    let data: Vec<i32> = (0..100).collect();
    let data = black_box(data);
    let result = for_! {
        x <= data;
        let y = x * 2;
        let z = y + 1;
        yield z
    };
    let result = black_box(result);
    assert_eq!(result.len(), 100);
    assert_eq!(result[0], 1); // 0 * 2 + 1 = 1
    assert_eq!(result[50], 101); // 50 * 2 + 1 = 101
    assert_eq!(result[99], 199); // 99 * 2 + 1 = 199
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

#[rstest]
fn test_optics_update() {
    let task = Task {
        id: 1,
        title: "Test".to_string(),
        status: TaskStatus::Todo,
    };
    let task = black_box(task);
    let title_lens = lens!(Task, title);
    let status_lens = lens!(Task, status);

    let updated = pipe!(task, |t| title_lens.set(t, "Updated".to_string()), |t| {
        status_lens.set(t, TaskStatus::Done)
    });
    let result = black_box(updated);
    assert_eq!(result.title, "Updated");
    assert!(matches!(result.status, TaskStatus::Done));
}

fn sum_trampoline(n: i32, acc: i32) -> Trampoline<i32> {
    if n <= 0 {
        Trampoline::done(acc)
    } else {
        Trampoline::suspend(move || sum_trampoline(n - 1, acc + n))
    }
}

#[rstest]
fn test_trampoline_recursion_1000() {
    let n = black_box(1000);
    let acc = black_box(0);
    let result = black_box(sum_trampoline(n, acc).run());
    // Sum of 1..=1000 = 500500
    assert_eq!(result, 500500);
}

#[rstest]
fn test_foldable_aggregation() {
    let vector: PersistentVector<i32> = (0..1000).collect();
    let vector = black_box(vector);
    let initial = black_box(0);
    let result = black_box(vector.fold_left(initial, |acc, x| acc + x));
    // Sum of 0..1000 = 499500
    assert_eq!(result, 499500);
}
