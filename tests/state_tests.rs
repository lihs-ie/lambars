#![cfg(feature = "effect")]
//! Unit tests for State Monad.
//!
//! Tests basic functionality of the State monad including:
//! - Creation and execution (run, eval, exec)
//! - Functor operations (fmap)
//! - Monad operations (flat_map, pure)
//! - State-specific operations (get, put, modify, gets, state)

use lambars::effect::State;
use rstest::rstest;

// =============================================================================
// Basic Construction and Execution Tests
// =============================================================================

#[rstest]
fn state_new_and_run_basic() {
    let state: State<i32, i32> = State::new(|s: i32| (s * 2, s + 1));
    let (result, final_state) = state.run(10);
    assert_eq!(result, 20);
    assert_eq!(final_state, 11);
}

#[rstest]
fn state_eval_returns_result() {
    let state: State<i32, i32> = State::new(|s: i32| (s * 2, s + 1));
    let result = state.eval(10);
    assert_eq!(result, 20);
}

#[rstest]
fn state_exec_returns_final_state() {
    let state: State<i32, i32> = State::new(|s: i32| (s * 2, s + 1));
    let final_state = state.exec(10);
    assert_eq!(final_state, 11);
}

#[rstest]
fn state_new_and_run_with_string_state() {
    let state: State<String, usize> = State::new(|s: String| (s.len(), s + " modified"));
    let (result, final_state) = state.run("hello".to_string());
    assert_eq!(result, 5);
    assert_eq!(final_state, "hello modified");
}

#[rstest]
fn state_new_and_run_with_struct_state() {
    #[derive(Clone, Debug, PartialEq)]
    struct Counter {
        value: i32,
        increments: u32,
    }

    let state: State<Counter, i32> = State::new(|s: Counter| {
        let old_value = s.value;
        let new_counter = Counter {
            value: s.value + 1,
            increments: s.increments + 1,
        };
        (old_value, new_counter)
    });

    let initial = Counter {
        value: 10,
        increments: 0,
    };

    let (result, final_state) = state.run(initial);
    assert_eq!(result, 10);
    assert_eq!(
        final_state,
        Counter {
            value: 11,
            increments: 1
        }
    );
}

// =============================================================================
// Pure Tests
// =============================================================================

#[rstest]
fn state_pure_creates_constant_state() {
    let state: State<i32, &str> = State::pure("constant");
    let (result, final_state) = state.run(42);
    assert_eq!(result, "constant");
    assert_eq!(final_state, 42);
}

#[rstest]
fn state_pure_does_not_modify_state() {
    let state: State<String, i32> = State::pure(100);
    let (result, final_state) = state.run("initial".to_string());
    assert_eq!(result, 100);
    assert_eq!(final_state, "initial");
}

// =============================================================================
// Functor (fmap) Tests
// =============================================================================

#[rstest]
fn state_fmap_transforms_result() {
    let state: State<i32, i32> = State::new(|s: i32| (s, s));
    let mapped = state.fmap(|value| value * 2);
    let (result, final_state) = mapped.run(21);
    assert_eq!(result, 42);
    assert_eq!(final_state, 21);
}

#[rstest]
fn state_fmap_changes_type() {
    let state: State<i32, i32> = State::new(|s: i32| (s, s));
    let mapped = state.fmap(|value| value.to_string());
    let (result, final_state) = mapped.run(42);
    assert_eq!(result, "42");
    assert_eq!(final_state, 42);
}

#[rstest]
fn state_fmap_chained() {
    let state: State<i32, i32> = State::new(|s: i32| (s, s));
    let mapped = state
        .fmap(|value| value + 1)
        .fmap(|value| value * 2)
        .fmap(|value| value.to_string());
    let (result, final_state) = mapped.run(5);
    assert_eq!(result, "12"); // (5 + 1) * 2 = 12
    assert_eq!(final_state, 5);
}

// =============================================================================
// Applicative Tests
// =============================================================================

#[rstest]
fn state_map2_combines_two_states() {
    let state1: State<i32, i32> = State::new(|s: i32| (s, s + 1));
    let state2: State<i32, i32> = State::new(|s: i32| (s * 2, s + 1));
    let combined = state1.map2(state2, |a, b| a + b);
    let (result, final_state) = combined.run(10);
    // state1: (10, 11), state2 runs with 11: (22, 12)
    assert_eq!(result, 10 + 22);
    assert_eq!(final_state, 12);
}

#[rstest]
fn state_product_creates_tuple() {
    let state1: State<i32, i32> = State::new(|s: i32| (s, s + 1));
    let state2: State<i32, &str> = State::pure("hello");
    let product = state1.product(state2);
    let ((first, second), final_state) = product.run(42);
    assert_eq!(first, 42);
    assert_eq!(second, "hello");
    assert_eq!(final_state, 43);
}

// =============================================================================
// Monad (flat_map) Tests
// =============================================================================

#[rstest]
fn state_flat_map_chains_states() {
    let state: State<i32, i32> = State::new(|s: i32| (s, s + 1));
    let chained = state.flat_map(|value| State::new(move |s: i32| (value + s, s * 2)));
    let (result, final_state) = chained.run(10);
    // First: (10, 11), then with state 11: (10 + 11, 22)
    assert_eq!(result, 21);
    assert_eq!(final_state, 22);
}

#[rstest]
fn state_flat_map_with_pure() {
    let state: State<i32, i32> = State::new(|s: i32| (s, s));
    let chained = state.flat_map(|value| State::pure(value * 2));
    let (result, final_state) = chained.run(21);
    assert_eq!(result, 42);
    assert_eq!(final_state, 21);
}

#[rstest]
fn state_and_then_is_alias_for_flat_map() {
    let state: State<i32, i32> = State::new(|s: i32| (s, s + 1));
    let chained = state.and_then(|value| State::new(move |s: i32| (value + s, s)));
    let (result, final_state) = chained.run(10);
    assert_eq!(result, 21); // 10 + 11
    assert_eq!(final_state, 11);
}

#[rstest]
fn state_then_sequences_and_discards_first() {
    let state1: State<i32, i32> = State::new(|s: i32| (s, s + 10));
    let state2: State<i32, &str> = State::pure("result");
    let sequenced = state1.then(state2);
    let (result, final_state) = sequenced.run(42);
    assert_eq!(result, "result");
    assert_eq!(final_state, 52);
}

// =============================================================================
// MonadState - get Tests
// =============================================================================

#[rstest]
fn state_get_returns_current_state() {
    let state: State<i32, i32> = State::get();
    let (result, final_state) = state.run(42);
    assert_eq!(result, 42);
    assert_eq!(final_state, 42);
}

#[rstest]
fn state_get_with_struct_state() {
    #[derive(Clone, PartialEq, Debug)]
    struct Config {
        value: i32,
    }

    let state: State<Config, Config> = State::get();
    let config = Config { value: 100 };
    let (result, final_state) = state.run(config.clone());
    assert_eq!(result, config);
    assert_eq!(final_state, config);
}

// =============================================================================
// MonadState - put Tests
// =============================================================================

#[rstest]
fn state_put_replaces_state() {
    let state: State<i32, ()> = State::put(100);
    let (result, final_state) = state.run(42);
    assert_eq!(result, ());
    assert_eq!(final_state, 100);
}

#[rstest]
fn state_put_with_struct_state() {
    #[derive(Clone, PartialEq, Debug)]
    struct Config {
        value: i32,
    }

    let new_config = Config { value: 200 };
    let state: State<Config, ()> = State::put(new_config.clone());
    let (result, final_state) = state.run(Config { value: 100 });
    assert_eq!(result, ());
    assert_eq!(final_state, new_config);
}

// =============================================================================
// MonadState - modify Tests
// =============================================================================

#[rstest]
fn state_modify_transforms_state() {
    let state: State<i32, ()> = State::modify(|x| x * 2);
    let (result, final_state) = state.run(21);
    assert_eq!(result, ());
    assert_eq!(final_state, 42);
}

#[rstest]
fn state_modify_with_struct_state() {
    #[derive(Clone, PartialEq, Debug)]
    struct Counter {
        value: i32,
    }

    let state: State<Counter, ()> = State::modify(|c: Counter| Counter { value: c.value + 1 });
    let (_, final_state) = state.run(Counter { value: 10 });
    assert_eq!(final_state, Counter { value: 11 });
}

// =============================================================================
// MonadState - gets Tests
// =============================================================================

#[rstest]
fn state_gets_projects_from_state() {
    #[derive(Clone)]
    struct Config {
        port: u16,
        #[allow(dead_code)]
        host: String,
    }

    let port_state: State<Config, u16> = State::gets(|config: &Config| config.port);
    let config = Config {
        port: 8080,
        host: "localhost".to_string(),
    };
    let (result, final_state) = port_state.run(config.clone());
    assert_eq!(result, 8080);
    assert_eq!(final_state.port, config.port);
}

#[rstest]
fn state_gets_with_transformation() {
    let state: State<i32, String> = State::gets(|s| format!("value: {}", s));
    let (result, final_state) = state.run(42);
    assert_eq!(result, "value: 42");
    assert_eq!(final_state, 42);
}

// =============================================================================
// MonadState - state function Tests
// =============================================================================

#[rstest]
fn state_new_function_basic() {
    let computation: State<i32, String> = State::new(|s: i32| (format!("was: {}", s), s + 1));
    let (result, final_state) = computation.run(10);
    assert_eq!(result, "was: 10");
    assert_eq!(final_state, 11);
}

#[rstest]
fn state_new_and_from_transition_are_equivalent() {
    let via_new: State<i32, i32> = State::new(|s: i32| (s * 2, s + 1));
    let via_from_transition: State<i32, i32> = State::from_transition(|s: i32| (s * 2, s + 1));

    let (new_result, new_final) = via_new.run(10);
    let (transition_result, transition_final) = via_from_transition.run(10);

    assert_eq!(new_result, transition_result);
    assert_eq!(new_final, transition_final);
}

// =============================================================================
// Complex Use Cases
// =============================================================================

#[rstest]
fn state_counter_pattern() {
    #[derive(Clone, Debug, PartialEq)]
    struct Counter {
        count: i32,
    }

    fn increment() -> State<Counter, ()> {
        State::modify(|c: Counter| Counter { count: c.count + 1 })
    }

    fn get_count() -> State<Counter, i32> {
        State::gets(|c: &Counter| c.count)
    }

    fn increment_and_get() -> State<Counter, i32> {
        increment().then(get_count())
    }

    let computation = increment_and_get()
        .flat_map(|_| increment_and_get())
        .flat_map(|_| increment_and_get());

    let (result, final_state) = computation.run(Counter { count: 0 });
    assert_eq!(result, 3);
    assert_eq!(final_state, Counter { count: 3 });
}

#[rstest]
fn state_stack_pattern() {
    fn push(value: i32) -> State<Vec<i32>, ()> {
        State::modify(move |mut stack: Vec<i32>| {
            stack.push(value);
            stack
        })
    }

    fn pop() -> State<Vec<i32>, Option<i32>> {
        State::new(|mut stack: Vec<i32>| {
            let value = stack.pop();
            (value, stack)
        })
    }

    let computation = push(1)
        .then(push(2))
        .then(push(3))
        .then(pop())
        .flat_map(|popped| pop().fmap(move |next| (popped, next)));

    let ((first_pop, second_pop), final_stack) = computation.run(vec![]);
    assert_eq!(first_pop, Some(3));
    assert_eq!(second_pop, Some(2));
    assert_eq!(final_stack, vec![1]);
}

#[rstest]
fn state_multiple_flat_map_chains() {
    let computation: State<i32, i32> = State::get().flat_map(|a| {
        State::modify(|s| s + 1)
            .then(State::get())
            .fmap(move |b| a + b)
    });

    let (result, final_state) = computation.run(5);
    assert_eq!(result, 11); // 5 + 6
    assert_eq!(final_state, 6);
}

// =============================================================================
// Edge Cases
// =============================================================================

#[rstest]
fn state_with_unit_state() {
    let state: State<(), i32> = State::pure(42);
    let (result, final_state) = state.run(());
    assert_eq!(result, 42);
    assert_eq!(final_state, ());
}

#[rstest]
fn state_with_unit_result() {
    let state: State<i32, ()> = State::modify(|s| s + 1);
    let (result, final_state) = state.run(41);
    assert_eq!(result, ());
    assert_eq!(final_state, 42);
}

#[rstest]
fn state_with_vector_state() {
    let state: State<Vec<i32>, usize> = State::gets(|s: &Vec<i32>| s.len());
    let (result, final_state) = state.run(vec![1, 2, 3]);
    assert_eq!(result, 3);
    assert_eq!(final_state, vec![1, 2, 3]);
}

#[rstest]
fn state_run_can_be_called_multiple_times() {
    let state: State<i32, i32> = State::get();
    assert_eq!(state.run(1), (1, 1));
    assert_eq!(state.run(2), (2, 2));
    assert_eq!(state.run(3), (3, 3));
}
