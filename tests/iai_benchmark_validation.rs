//! Validation tests for Iai-Callgrind benchmark functions.

use lambars::control::Trampoline;
use lambars::eff;
use lambars::effect::{ExceptT, IO, Reader, State};
use lambars::for_;
use lambars::lens;
use lambars::optics::Lens;
use lambars::persistent::{OrderedUniqueSet, PersistentHashMap, PersistentVector};
use lambars::pipe;
use lambars::typeclass::{Foldable, Functor, Monad};
use rstest::rstest;
use std::hint::black_box;

#[rstest]
fn test_push_back_1000() {
    let mut vector = PersistentVector::new();
    for index in 0..1000 {
        vector = vector.push_back(black_box(index));
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
    for index in 0..1000 {
        if let Some(&value) = vector.get(black_box(index)) {
            sum += value;
        }
    }
    assert_eq!(black_box(sum), 499500);
}

#[rstest]
fn test_update_1000() {
    let vector: PersistentVector<i32> = (0..1000).collect();
    let mut vector = black_box(vector);
    for index in 0..1000 {
        if let Some(updated) = vector.update(black_box(index), black_box(index as i32 * 2)) {
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
    assert_eq!(black_box(black_box(vector).iter().sum::<i32>()), 499500);
}

#[rstest]
fn test_io_pure_chain_10() {
    let io = IO::pure(black_box(1))
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
    assert_eq!(black_box(io.run_unsafe()), 57);
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
    assert_eq!(black_box(reader.run(black_box(10))), 201);
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
    assert_eq!(result, 31);
    assert_eq!(final_state, 31);
}

#[rstest]
fn test_exceptt_chain_10() {
    let exceptt: ExceptT<String, Option<Result<i32, String>>> = ExceptT::pure_option(black_box(1))
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
    assert_eq!(
        black_box(exceptt.run().expect("Option should be Some")),
        Ok(57)
    );
}

#[rstest]
fn test_eff_macro_10() {
    let io: IO<i32> = eff! {
        x <= IO::pure(black_box(1));
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
    assert_eq!(black_box(io.run_unsafe()), 57);
}

#[rstest]
fn test_monad_transformer_chain() {
    let reader: Reader<i32, i32> = Reader::ask().flat_map(|env| Reader::pure(env * 2));
    let state: State<i32, i32> = State::get()
        .flat_map(move |s| State::put(s + reader.run_cloned(black_box(10))).then(State::get()));
    let io = IO::pure(state.run(black_box(0))).fmap(|(result, _)| result);
    assert_eq!(black_box(io.run_unsafe()), 20);
}

#[rstest]
fn test_persistent_data_pipeline() {
    let vector: PersistentVector<i32> = (0..100).collect();
    let updated = black_box(vector)
        .update(black_box(50), black_box(999))
        .expect("index valid")
        .push_back(black_box(100))
        .push_back(black_box(101));

    let map: PersistentHashMap<i32, i32> = updated
        .iter()
        .enumerate()
        .map(|(index, &value)| (index as i32, value))
        .collect();

    assert_eq!(black_box(map.get(&black_box(50)).copied()), Some(999));
}

#[rstest]
fn test_for_macro_pipeline() {
    let data: Vec<i32> = (0..100).collect();
    let result = for_! {
        x <= black_box(data);
        let y = x * 2;
        let z = y + 1;
        yield z
    };
    let result = black_box(result);
    assert_eq!(result.len(), 100);
    assert_eq!(result[0], 1);
    assert_eq!(result[50], 101);
    assert_eq!(result[99], 199);
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
    let task = black_box(Task {
        id: 1,
        title: "Test".to_string(),
        status: TaskStatus::Todo,
    });
    let title_lens = lens!(Task, title);
    let status_lens = lens!(Task, status);

    let result = black_box(pipe!(
        task,
        |t| title_lens.set(t, "Updated".to_string()),
        |t| { status_lens.set(t, TaskStatus::Done) }
    ));
    assert_eq!(result.title, "Updated");
    assert!(matches!(result.status, TaskStatus::Done));
}

fn sum_trampoline(n: i32, accumulator: i32) -> Trampoline<i32> {
    if n <= 0 {
        Trampoline::done(accumulator)
    } else {
        Trampoline::suspend(move || sum_trampoline(n - 1, accumulator + n))
    }
}

#[rstest]
fn test_trampoline_recursion_1000() {
    assert_eq!(
        black_box(sum_trampoline(black_box(1000), black_box(0)).run()),
        500500
    );
}

#[rstest]
fn test_foldable_aggregation() {
    let vector: PersistentVector<i32> = (0..1000).collect();
    assert_eq!(
        black_box(black_box(vector).fold_left(black_box(0), |accumulator, x| accumulator + x)),
        499500
    );
}

#[rstest]
fn test_ordered_unique_set_from_sorted_iter_1000() {
    let elements: Vec<i32> = (0..1000).collect();
    let result = black_box(OrderedUniqueSet::from_sorted_iter(black_box(elements)));
    assert_eq!(result.len(), 1000);
    assert!(result.contains(&0));
    assert!(result.contains(&500));
    assert!(result.contains(&999));
}

#[rstest]
fn test_ordered_unique_set_from_sorted_vec_1000() {
    let elements: Vec<i32> = (0..1000).collect();
    let result = black_box(OrderedUniqueSet::from_sorted_vec(black_box(elements)));
    assert_eq!(result.len(), 1000);
    assert!(result.contains(&0));
    assert!(result.contains(&500));
    assert!(result.contains(&999));
}

#[rstest]
fn test_ordered_unique_set_fold_insert_1000() {
    let elements: Vec<i32> = (0..1000).collect();
    let result = black_box(
        black_box(elements)
            .into_iter()
            .fold(OrderedUniqueSet::new(), |accumulator, element| {
                accumulator.insert(element)
            }),
    );
    assert_eq!(result.len(), 1000);
    assert!(result.contains(&0));
    assert!(result.contains(&500));
    assert!(result.contains(&999));
}

#[rstest]
fn test_persistent_vector_from_vec_1000() {
    let elements: Vec<i32> = (0..1000).collect();
    let result = black_box(PersistentVector::from_vec(black_box(elements)));
    assert_eq!(result.len(), 1000);
    assert_eq!(result.get(0), Some(&0));
    assert_eq!(result.get(500), Some(&500));
    assert_eq!(result.get(999), Some(&999));
}

#[rstest]
fn test_persistent_vector_collect_1000() {
    let result = black_box(black_box(0..1000).collect::<PersistentVector<i32>>());
    assert_eq!(result.len(), 1000);
    assert_eq!(result.get(0), Some(&0));
    assert_eq!(result.get(500), Some(&500));
    assert_eq!(result.get(999), Some(&999));
}
