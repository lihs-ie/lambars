#![cfg(feature = "effect")]
#![allow(deprecated)]
//! Tests for StateT (State Transformer).
//!
//! StateT adds state manipulation capability to any monad.

use lambars::effect::{IO, StateT};
use rstest::rstest;

// =============================================================================
// Basic Structure Tests
// =============================================================================

#[rstest]
fn state_transformer_new_and_run_with_option() {
    let state_transformer: StateT<i32, Option<(String, i32)>> =
        StateT::new(|state: i32| Some((format!("state: {}", state), state + 1)));
    let result = state_transformer.run(10);
    assert_eq!(result, Some(("state: 10".to_string(), 11)));
}

#[rstest]
fn state_transformer_new_and_run_with_result() {
    let state_transformer: StateT<i32, Result<(String, i32), String>> =
        StateT::new(|state: i32| Ok((format!("state: {}", state), state + 1)));
    let result = state_transformer.run(10);
    assert_eq!(result, Ok(("state: 10".to_string(), 11)));
}

#[rstest]
fn state_transformer_run_returns_none_when_inner_is_none() {
    let state_transformer: StateT<i32, Option<(String, i32)>> =
        StateT::new(|_state: i32| None::<(String, i32)>);
    let result = state_transformer.run(10);
    assert_eq!(result, None);
}

// =============================================================================
// eval and exec Tests
// =============================================================================

#[rstest]
fn state_transformer_eval_returns_value_only() {
    let state_transformer: StateT<i32, Option<(i32, i32)>> =
        StateT::new(|state: i32| Some((state * 2, state + 1)));
    let result = state_transformer.eval(10);
    assert_eq!(result, Some(20));
}

#[rstest]
fn state_transformer_exec_returns_state_only() {
    let state_transformer: StateT<i32, Option<(i32, i32)>> =
        StateT::new(|state: i32| Some((state * 2, state + 1)));
    let result = state_transformer.exec(10);
    assert_eq!(result, Some(11));
}

// =============================================================================
// pure Tests
// =============================================================================

#[rstest]
fn state_transformer_pure_with_option() {
    let state_transformer: StateT<i32, Option<(String, i32)>> =
        StateT::pure_option("hello".to_string());
    let result = state_transformer.run(42);
    assert_eq!(result, Some(("hello".to_string(), 42)));
}

#[rstest]
fn state_transformer_pure_with_result() {
    let state_transformer: StateT<i32, Result<(String, i32), String>> =
        StateT::pure_result("hello".to_string());
    let result = state_transformer.run(42);
    assert_eq!(result, Ok(("hello".to_string(), 42)));
}

// =============================================================================
// lift Tests
// =============================================================================

#[rstest]
fn state_transformer_lift_option() {
    let inner: Option<String> = Some("hello".to_string());
    let state_transformer: StateT<i32, Option<(String, i32)>> = StateT::lift_option(inner);
    let result = state_transformer.run(42);
    assert_eq!(result, Some(("hello".to_string(), 42)));
}

#[rstest]
fn state_transformer_lift_option_none() {
    let inner: Option<String> = None;
    let state_transformer: StateT<i32, Option<(String, i32)>> = StateT::lift_option(inner);
    let result = state_transformer.run(42);
    assert_eq!(result, None);
}

#[rstest]
fn state_transformer_lift_result() {
    let inner: Result<String, String> = Ok("hello".to_string());
    let state_transformer: StateT<i32, Result<(String, i32), String>> = StateT::lift_result(inner);
    let result = state_transformer.run(42);
    assert_eq!(result, Ok(("hello".to_string(), 42)));
}

#[rstest]
fn state_transformer_lift_result_error() {
    let inner: Result<String, String> = Err("error".to_string());
    let state_transformer: StateT<i32, Result<(String, i32), String>> = StateT::lift_result(inner);
    let result = state_transformer.run(42);
    assert_eq!(result, Err("error".to_string()));
}

// =============================================================================
// fmap (Functor) Tests
// =============================================================================

#[rstest]
fn state_transformer_fmap_option_some() {
    let state_transformer: StateT<i32, Option<(i32, i32)>> =
        StateT::new(|state: i32| Some((state, state + 1)));
    let mapped = state_transformer.fmap_option(|value| value * 2);
    let result = mapped.run(10);
    assert_eq!(result, Some((20, 11)));
}

#[rstest]
fn state_transformer_fmap_option_none() {
    let state_transformer: StateT<i32, Option<(i32, i32)>> =
        StateT::new(|_state: i32| None::<(i32, i32)>);
    let mapped = state_transformer.fmap_option(|value| value * 2);
    let result = mapped.run(10);
    assert_eq!(result, None);
}

#[rstest]
fn state_transformer_fmap_result_ok() {
    let state_transformer: StateT<i32, Result<(i32, i32), String>> =
        StateT::new(|state: i32| Ok((state, state + 1)));
    let mapped = state_transformer.fmap_result(|value| value * 2);
    let result = mapped.run(10);
    assert_eq!(result, Ok((20, 11)));
}

#[rstest]
fn state_transformer_fmap_result_error() {
    let state_transformer: StateT<i32, Result<(i32, i32), String>> =
        StateT::new(|_state: i32| Err::<(i32, i32), String>("error".to_string()));
    let mapped = state_transformer.fmap_result(|value| value * 2);
    let result: Result<(i32, i32), String> = mapped.run(10);
    assert_eq!(result, Err("error".to_string()));
}

// =============================================================================
// flat_map (Monad) Tests
// =============================================================================

#[rstest]
fn state_transformer_flat_map_option_some_to_some() {
    let state_transformer: StateT<i32, Option<(i32, i32)>> =
        StateT::new(|state: i32| Some((state, state + 1)));

    let chained = state_transformer
        .flat_map_option(|value| StateT::new(move |state: i32| Some((value + state, state * 2))));

    // Initial state: 10
    // First: value = 10, new_state = 11
    // Second: value = 10 + 11 = 21, new_state = 11 * 2 = 22
    let result = chained.run(10);
    assert_eq!(result, Some((21, 22)));
}

#[rstest]
fn state_transformer_flat_map_option_some_to_none() {
    let state_transformer: StateT<i32, Option<(i32, i32)>> =
        StateT::new(|state: i32| Some((state, state + 1)));

    let chained =
        state_transformer.flat_map_option(|_value| StateT::new(|_state: i32| None::<(i32, i32)>));

    let result: Option<(i32, i32)> = chained.run(10);
    assert_eq!(result, None);
}

#[rstest]
fn state_transformer_flat_map_option_none_short_circuits() {
    let state_transformer: StateT<i32, Option<(i32, i32)>> =
        StateT::new(|_state: i32| None::<(i32, i32)>);

    let chained = state_transformer
        .flat_map_option(|value| StateT::new(move |state: i32| Some((value + state, state * 2))));

    let result: Option<(i32, i32)> = chained.run(10);
    assert_eq!(result, None);
}

#[rstest]
fn state_transformer_flat_map_result_ok_to_ok() {
    let state_transformer: StateT<i32, Result<(i32, i32), String>> =
        StateT::new(|state: i32| Ok((state, state + 1)));

    let chained = state_transformer
        .flat_map_result(|value| StateT::new(move |state: i32| Ok((value + state, state * 2))));

    let result = chained.run(10);
    assert_eq!(result, Ok((21, 22)));
}

#[rstest]
fn state_transformer_flat_map_result_ok_to_error() {
    let state_transformer: StateT<i32, Result<(i32, i32), String>> =
        StateT::new(|state: i32| Ok((state, state + 1)));

    let chained = state_transformer.flat_map_result(|_value| {
        StateT::new(|_state: i32| Err::<(i32, i32), String>("error".to_string()))
    });

    let result: Result<(i32, i32), String> = chained.run(10);
    assert_eq!(result, Err("error".to_string()));
}

#[rstest]
fn state_transformer_flat_map_result_error_short_circuits() {
    let state_transformer: StateT<i32, Result<(i32, i32), String>> =
        StateT::new(|_state: i32| Err::<(i32, i32), String>("error".to_string()));

    let chained = state_transformer
        .flat_map_result(|value| StateT::new(move |state: i32| Ok((value + state, state * 2))));

    let result: Result<(i32, i32), String> = chained.run(10);
    assert_eq!(result, Err("error".to_string()));
}

// =============================================================================
// get/put/modify (MonadState) Tests
// =============================================================================

#[rstest]
fn state_transformer_get_option() {
    let state_transformer: StateT<i32, Option<(i32, i32)>> = StateT::get_option();
    let result = state_transformer.run(42);
    assert_eq!(result, Some((42, 42)));
}

#[rstest]
fn state_transformer_get_result() {
    let state_transformer: StateT<i32, Result<(i32, i32), String>> = StateT::get_result();
    let result = state_transformer.run(42);
    assert_eq!(result, Ok((42, 42)));
}

#[rstest]
fn state_transformer_put_option() {
    let state_transformer: StateT<i32, Option<((), i32)>> =
        StateT::<i32, Option<((), i32)>>::put_option(100);
    let result = state_transformer.run(42);
    assert_eq!(result, Some(((), 100)));
}

#[rstest]
fn state_transformer_put_result() {
    let state_transformer: StateT<i32, Result<((), i32), String>> =
        StateT::<i32, Result<((), i32), String>>::put_result(100);
    let result = state_transformer.run(42);
    assert_eq!(result, Ok(((), 100)));
}

#[rstest]
fn state_transformer_modify_option() {
    let state_transformer: StateT<i32, Option<((), i32)>> =
        StateT::<i32, Option<((), i32)>>::modify_option(|state| state * 2);
    let result = state_transformer.run(21);
    assert_eq!(result, Some(((), 42)));
}

#[rstest]
fn state_transformer_modify_result() {
    let state_transformer: StateT<i32, Result<((), i32), String>> =
        StateT::<i32, Result<((), i32), String>>::modify_result(|state| state * 2);
    let result = state_transformer.run(21);
    assert_eq!(result, Ok(((), 42)));
}

// =============================================================================
// StateT with IO Tests
// =============================================================================

#[rstest]
fn state_transformer_with_io_basic() {
    let state_transformer: StateT<i32, IO<(String, i32)>> =
        StateT::new(|state: i32| IO::pure((format!("state: {}", state), state + 1)));

    let io_result = state_transformer.run(10);
    let (value, final_state) = io_result.run_unsafe();
    assert_eq!(value, "state: 10");
    assert_eq!(final_state, 11);
}

#[rstest]
#[allow(deprecated)]
fn state_transformer_lift_io() {
    let inner = IO::pure("hello".to_string());
    let state_transformer: StateT<i32, IO<(String, i32)>> = StateT::lift_io(inner);

    let io_result = state_transformer.run(42);
    let (value, final_state) = io_result.run_unsafe();
    assert_eq!(value, "hello");
    assert_eq!(final_state, 42);
}

#[rstest]
fn state_transformer_fmap_io() {
    let state_transformer: StateT<i32, IO<(i32, i32)>> =
        StateT::new(|state: i32| IO::pure((state, state + 1)));

    let mapped = state_transformer.fmap_io(|value| value * 2);

    let io_result = mapped.run(10);
    let (value, final_state) = io_result.run_unsafe();
    assert_eq!(value, 20);
    assert_eq!(final_state, 11);
}

#[rstest]
fn state_transformer_flat_map_io() {
    let state_transformer: StateT<i32, IO<(i32, i32)>> =
        StateT::new(|state: i32| IO::pure((state, state + 1)));

    let chained = state_transformer
        .flat_map_io(|value| StateT::new(move |state: i32| IO::pure((value + state, state * 2))));

    let io_result = chained.run(10);
    let (value, final_state) = io_result.run_unsafe();
    assert_eq!(value, 21);
    assert_eq!(final_state, 22);
}

#[rstest]
fn state_transformer_get_io() {
    let state_transformer: StateT<i32, IO<(i32, i32)>> = StateT::get_io();

    let io_result = state_transformer.run(42);
    let (value, final_state) = io_result.run_unsafe();
    assert_eq!(value, 42);
    assert_eq!(final_state, 42);
}

#[rstest]
fn state_transformer_put_io() {
    let state_transformer: StateT<i32, IO<((), i32)>> = StateT::<i32, IO<((), i32)>>::put_io(100);

    let io_result = state_transformer.run(42);
    let (value, final_state) = io_result.run_unsafe();
    assert_eq!(value, ());
    assert_eq!(final_state, 100);
}

#[rstest]
fn state_transformer_modify_io() {
    let state_transformer: StateT<i32, IO<((), i32)>> =
        StateT::<i32, IO<((), i32)>>::modify_io(|state| state * 2);

    let io_result = state_transformer.run(21);
    let (value, final_state) = io_result.run_unsafe();
    assert_eq!(value, ());
    assert_eq!(final_state, 42);
}

// =============================================================================
// Clone Tests
// =============================================================================

#[rstest]
fn state_transformer_clone() {
    let state_transformer: StateT<i32, Option<(i32, i32)>> =
        StateT::new(|state: i32| Some((state * 2, state + 1)));
    let cloned = state_transformer.clone();

    assert_eq!(state_transformer.run(10), Some((20, 11)));
    assert_eq!(cloned.run(10), Some((20, 11)));
}

// =============================================================================
// Practical Examples
// =============================================================================

#[rstest]
fn state_transformer_counter_example() {
    fn increment() -> StateT<i32, Option<((), i32)>> {
        StateT::<i32, Option<((), i32)>>::modify_option(|count| count + 1)
    }

    fn get_count() -> StateT<i32, Option<(i32, i32)>> {
        StateT::get_option()
    }

    let computation = increment()
        .flat_map_option(|_| increment())
        .flat_map_option(|_| increment())
        .flat_map_option(|_| get_count());

    let result = computation.run(0);
    assert_eq!(result, Some((3, 3)));
}

#[rstest]
fn state_transformer_stack_example() {
    fn push(value: i32) -> StateT<Vec<i32>, Option<((), Vec<i32>)>> {
        StateT::new(move |mut stack: Vec<i32>| {
            stack.push(value);
            Some(((), stack))
        })
    }

    fn pop() -> StateT<Vec<i32>, Option<(i32, Vec<i32>)>> {
        StateT::new(|mut stack: Vec<i32>| stack.pop().map(|value| (value, stack)))
    }

    let computation = push(1)
        .flat_map_option(|_| push(2))
        .flat_map_option(|_| push(3))
        .flat_map_option(|_| pop())
        .flat_map_option(|popped| StateT::pure_option(popped * 10));

    let result = computation.run(vec![]);
    // Stack: [1, 2, 3] -> pop returns 3, final stack [1, 2], result 30
    assert_eq!(result, Some((30, vec![1, 2])));
}

// =============================================================================
// AsyncIO-specific Tests (requires async feature)
// =============================================================================

#[cfg(feature = "async")]
mod async_io_tests {
    use lambars::effect::{AsyncIO, StateT};
    use rstest::rstest;

    // =========================================================================
    // lift_async_io Tests
    // =========================================================================

    #[rstest]
    #[tokio::test]
    #[allow(deprecated)]
    async fn state_lift_async_io_preserves_state() {
        let async_io = AsyncIO::pure(42);
        let state: StateT<i32, AsyncIO<(i32, i32)>> = StateT::lift_async_io(async_io);
        let (result, final_state) = state.run(100).await;
        assert_eq!(result, 42);
        assert_eq!(final_state, 100);
    }

    #[rstest]
    #[tokio::test]
    #[allow(deprecated)]
    async fn state_lift_async_io_preserves_async_value() {
        let async_io = AsyncIO::new(|| async { "hello".to_string() });
        let state: StateT<(), AsyncIO<(String, ())>> = StateT::lift_async_io(async_io);
        let (result, _) = state.run(()).await;
        assert_eq!(result, "hello");
    }

    #[rstest]
    #[tokio::test]
    #[allow(deprecated)]
    async fn state_lift_state_law() {
        let initial_state = 100;
        let async_io = AsyncIO::pure(42);
        let state: StateT<i32, AsyncIO<(i32, i32)>> = StateT::lift_async_io(async_io);
        let final_state = state.exec_async(initial_state).await;
        assert_eq!(final_state, initial_state);
    }

    // =========================================================================
    // gets_async_io Tests
    // =========================================================================

    #[rstest]
    #[tokio::test]
    async fn state_gets_async_io_projects_value() {
        let state: StateT<i32, AsyncIO<(i32, i32)>> = StateT::gets_async_io(|s: &i32| s * 2);
        let (result, final_state) = state.run(21).await;
        assert_eq!(result, 42);
        assert_eq!(final_state, 21);
    }

    #[rstest]
    #[tokio::test]
    async fn state_gets_async_io_does_not_modify_state() {
        let state: StateT<String, AsyncIO<(usize, String)>> =
            StateT::gets_async_io(|s: &String| s.len());
        let (result, final_state) = state.run("hello".to_string()).await;
        assert_eq!(result, 5);
        assert_eq!(final_state, "hello");
    }

    #[rstest]
    #[tokio::test]
    async fn state_gets_get_law() {
        let projection = |s: &i32| s * 2;

        let state1: StateT<i32, AsyncIO<(i32, i32)>> = StateT::gets_async_io(projection);
        let (result1, _) = state1.run(21).await;

        let state2: StateT<i32, AsyncIO<(i32, i32)>> =
            StateT::<i32, AsyncIO<(i32, i32)>>::get_async_io()
                .fmap_async_io(move |s| projection(&s));
        let (result2, _) = state2.run(21).await;

        assert_eq!(result1, result2);
    }

    // =========================================================================
    // state_async_io Tests
    // =========================================================================

    #[rstest]
    #[tokio::test]
    async fn state_state_async_io_transitions() {
        let state: StateT<i32, AsyncIO<(String, i32)>> =
            StateT::state_async_io(|s| (format!("was: {}", s), s + 1));
        let (result, final_state) = state.run(41).await;
        assert_eq!(result, "was: 41");
        assert_eq!(final_state, 42);
    }

    #[rstest]
    #[tokio::test]
    async fn state_state_async_io_can_read_and_write() {
        let state: StateT<Vec<i32>, AsyncIO<(i32, Vec<i32>)>> =
            StateT::state_async_io(|mut s: Vec<i32>| {
                let sum: i32 = s.iter().sum();
                s.push(sum);
                (sum, s)
            });
        let (result, final_state) = state.run(vec![1, 2, 3]).await;
        assert_eq!(result, 6);
        assert_eq!(final_state, vec![1, 2, 3, 6]);
    }
}
