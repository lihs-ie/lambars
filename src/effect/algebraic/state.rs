//! State effect for stateful computations.
//!
//! This module provides the `StateEffect<S>` type that represents computations
//! that can read and modify a state of type `S`.
//!
//! # Operations
//!
//! - [`StateEffect::get`]: Retrieves the current state
//! - [`StateEffect::put`]: Replaces the current state
//! - [`StateEffect::modify`]: Modifies the state using a function
//! - [`StateEffect::gets`]: Retrieves a projected value from the state
//!
//! # Examples
//!
//! ```rust
//! use lambars::effect::algebraic::{StateEffect, StateHandler, Handler};
//!
//! // Counter example
//! let computation = StateEffect::<i32>::get()
//!     .flat_map(|x| StateEffect::put(x + 1))
//!     .then(StateEffect::get());
//!
//! let (result, final_state) = StateHandler::new(0).run(computation);
//! assert_eq!(result, 1);
//! assert_eq!(final_state, 1);
//! ```

use super::eff::{Eff, EffInner, OperationTag};
use super::effect::Effect;
use super::handler::Handler;
use std::cell::RefCell;
use std::marker::PhantomData;

mod state_operations {
    use super::OperationTag;
    pub const GET: OperationTag = OperationTag::new(10);
    pub const PUT: OperationTag = OperationTag::new(11);
}

/// State effect: provides stateful computations.
///
/// `StateEffect<S>` represents the capability to read and modify
/// a state of type `S`.
///
/// # Type Parameters
///
/// - `S`: The type of the state
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{StateEffect, StateHandler, Handler};
///
/// let computation = StateEffect::<i32>::modify(|x| x + 1)
///     .then(StateEffect::get());
///
/// let (result, final_state) = StateHandler::new(10).run(computation);
/// assert_eq!(result, 11);
/// assert_eq!(final_state, 11);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StateEffect<S>(PhantomData<S>);

impl<S: 'static> Effect for StateEffect<S> {
    const NAME: &'static str = "State";
}

impl<S: Clone + 'static> StateEffect<S> {
    /// Retrieves the current state.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::algebraic::{StateEffect, StateHandler, Handler};
    ///
    /// let computation = StateEffect::<i32>::get();
    /// let (result, final_state) = StateHandler::new(42).run(computation);
    /// assert_eq!(result, 42);
    /// assert_eq!(final_state, 42);
    /// ```
    #[must_use]
    pub fn get() -> Eff<Self, S> {
        Eff::<Self, S>::perform_raw::<S>(state_operations::GET, ())
    }

    /// Replaces the current state with a new value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::algebraic::{StateEffect, StateHandler, Handler};
    ///
    /// let computation = StateEffect::put(100);
    /// let ((), final_state) = StateHandler::new(0).run(computation);
    /// assert_eq!(final_state, 100);
    /// ```
    pub fn put(state: S) -> Eff<Self, ()>
    where
        S: Send + Sync,
    {
        Eff::<Self, ()>::perform_raw::<()>(state_operations::PUT, state)
    }

    /// Modifies the state using a function.
    ///
    /// This is equivalent to `get().flat_map(|s| put(modifier(s)))`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::algebraic::{StateEffect, StateHandler, Handler};
    ///
    /// let computation = StateEffect::<i32>::modify(|x| x * 2);
    /// let ((), final_state) = StateHandler::new(21).run(computation);
    /// assert_eq!(final_state, 42);
    /// ```
    pub fn modify<F>(modifier: F) -> Eff<Self, ()>
    where
        S: Send + Sync,
        F: FnOnce(S) -> S + 'static,
    {
        Self::get().flat_map(move |state| Self::put(modifier(state)))
    }

    /// Retrieves a projected value from the state.
    ///
    /// This is equivalent to `get().fmap(|s| projection(&s))`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::algebraic::{StateEffect, StateHandler, Handler};
    ///
    /// let computation = StateEffect::<Vec<i32>>::gets(|v| v.len());
    /// let (result, _) = StateHandler::new(vec![1, 2, 3]).run(computation);
    /// assert_eq!(result, 3);
    /// ```
    pub fn gets<A: 'static, F>(projection: F) -> Eff<Self, A>
    where
        F: FnOnce(&S) -> A + 'static,
    {
        Self::get().fmap(|state| projection(&state))
    }
}

/// Handler for the State effect.
///
/// `StateHandler<S>` holds an initial state and interprets State operations
/// by maintaining and modifying the state throughout the computation.
///
/// The handler returns a tuple `(A, S)` containing the computation result
/// and the final state.
///
/// # Type Parameters
///
/// - `S`: The type of the state
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::{StateEffect, StateHandler, Handler};
///
/// let handler = StateHandler::new(0);
/// let computation = StateEffect::<i32>::modify(|x| x + 1)
///     .then(StateEffect::modify(|x| x + 1))
///     .then(StateEffect::get());
///
/// let (result, final_state) = handler.run(computation);
/// assert_eq!(result, 2);
/// assert_eq!(final_state, 2);
/// ```
#[derive(Debug, Clone)]
pub struct StateHandler<S> {
    initial_state: S,
}

impl<S: Clone + 'static> StateHandler<S> {
    /// Creates a new `StateHandler` with the given initial state.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::algebraic::StateHandler;
    ///
    /// let handler = StateHandler::new(0);
    /// ```
    #[must_use]
    pub const fn new(initial_state: S) -> Self {
        Self { initial_state }
    }

    /// Returns a reference to the initial state.
    #[must_use]
    pub const fn initial_state(&self) -> &S {
        &self.initial_state
    }

    /// Runs the computation with a mutable state cell (internal).
    ///
    /// Uses an iterative approach for stack safety.
    fn run_with_state<A: 'static>(computation: Eff<StateEffect<S>, A>, state: &RefCell<S>) -> A {
        let mut current_computation = computation;

        loop {
            let normalized = current_computation.normalize();

            match normalized.inner {
                EffInner::Pure(value) => return value,
                EffInner::Impure(operation) => match operation.operation_tag {
                    state_operations::GET => {
                        let current = state.borrow().clone();
                        let continuation = operation.continuation;
                        current_computation = continuation(Box::new(current));
                    }
                    state_operations::PUT => {
                        let new_state = *operation
                            .arguments
                            .downcast::<S>()
                            .expect("Type mismatch in State::put");
                        *state.borrow_mut() = new_state;
                        let continuation = operation.continuation;
                        current_computation = continuation(Box::new(()));
                    }
                    _ => panic!("Unknown State operation: {:?}", operation.operation_tag),
                },
                EffInner::FlatMap(_) => {
                    unreachable!("FlatMap should be normalized by normalize()")
                }
            }
        }
    }
}

impl<S: Clone + 'static> Handler<StateEffect<S>> for StateHandler<S> {
    type Output<A> = (A, S);

    fn run<A: 'static>(self, computation: Eff<StateEffect<S>, A>) -> (A, S) {
        let state = RefCell::new(self.initial_state);
        let result = Self::run_with_state(computation, &state);
        (result, state.into_inner())
    }
}

#[cfg(test)]
#[allow(
    clippy::no_effect_underscore_binding,
    clippy::redundant_clone,
    clippy::ignored_unit_patterns
)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn state_effect_name_is_state() {
        assert_eq!(StateEffect::<i32>::NAME, "State");
    }

    #[rstest]
    fn state_effect_is_debug() {
        let effect: StateEffect<i32> = StateEffect(PhantomData);
        let debug_string = format!("{effect:?}");
        assert!(debug_string.contains("StateEffect"));
    }

    #[rstest]
    fn state_effect_is_clone() {
        let effect: StateEffect<i32> = StateEffect(PhantomData);
        let _cloned = effect;
    }

    #[rstest]
    fn state_effect_is_copy() {
        let effect: StateEffect<i32> = StateEffect(PhantomData);
        let _copied = effect;
    }

    #[rstest]
    fn state_handler_new_creates_handler() {
        let handler = StateHandler::new(42);
        assert_eq!(*handler.initial_state(), 42);
    }

    #[rstest]
    fn state_handler_is_debug() {
        let handler = StateHandler::new(42);
        let debug_string = format!("{handler:?}");
        assert!(debug_string.contains("StateHandler"));
        assert!(debug_string.contains("42"));
    }

    #[rstest]
    fn state_handler_is_clone() {
        let handler = StateHandler::new(42);
        let cloned = handler.clone();
        assert_eq!(*cloned.initial_state(), 42);
    }

    // get Operation Tests

    #[rstest]
    fn state_get_returns_current_state() {
        let handler = StateHandler::new(42);
        let (result, final_state) = handler.run(StateEffect::<i32>::get());
        assert_eq!(result, 42);
        assert_eq!(final_state, 42);
    }

    #[rstest]
    fn state_get_with_string() {
        let handler = StateHandler::new("hello".to_string());
        let (result, final_state) = handler.run(StateEffect::<String>::get());
        assert_eq!(result, "hello");
        assert_eq!(final_state, "hello");
    }

    #[rstest]
    fn state_get_with_complex_type() {
        #[derive(Clone, Debug, PartialEq)]
        struct AppState {
            count: i32,
            name: String,
        }

        let initial = AppState {
            count: 10,
            name: "test".to_string(),
        };
        let handler = StateHandler::new(initial.clone());
        let (result, final_state) = handler.run(StateEffect::<AppState>::get());
        assert_eq!(result, initial);
        assert_eq!(final_state, initial);
    }

    // put Operation Tests

    #[rstest]
    fn state_put_changes_state() {
        let handler = StateHandler::new(0);
        let ((), final_state) = handler.run(StateEffect::put(100));
        assert_eq!(final_state, 100);
    }

    #[rstest]
    fn state_put_with_string() {
        let handler = StateHandler::new("initial".to_string());
        let ((), final_state) = handler.run(StateEffect::put("updated".to_string()));
        assert_eq!(final_state, "updated");
    }

    #[rstest]
    fn state_put_multiple_times() {
        let handler = StateHandler::new(0);
        let computation = StateEffect::put(1)
            .then(StateEffect::put(2))
            .then(StateEffect::put(3));
        let ((), final_state) = handler.run(computation);
        assert_eq!(final_state, 3);
    }

    // modify Operation Tests

    #[rstest]
    fn state_modify_transforms_state() {
        let handler = StateHandler::new(10);
        let ((), final_state) = handler.run(StateEffect::modify(|x: i32| x * 2));
        assert_eq!(final_state, 20);
    }

    #[rstest]
    fn state_modify_with_closure() {
        let handler = StateHandler::new(vec![1, 2, 3]);
        let computation = StateEffect::modify(|mut v: Vec<i32>| {
            v.push(4);
            v
        });
        let ((), final_state) = handler.run(computation);
        assert_eq!(final_state, vec![1, 2, 3, 4]);
    }

    #[rstest]
    fn state_modify_multiple_times() {
        let handler = StateHandler::new(1);
        let computation = StateEffect::modify(|x: i32| x + 1)
            .then(StateEffect::modify(|x: i32| x * 2))
            .then(StateEffect::modify(|x: i32| x + 10));
        let ((), final_state) = handler.run(computation);
        assert_eq!(final_state, 14); // ((1 + 1) * 2) + 10
    }

    // gets Operation Tests

    #[rstest]
    fn state_gets_projects_state() {
        let handler = StateHandler::new(vec![1, 2, 3, 4, 5]);
        let (result, _) = handler.run(StateEffect::gets(|v: &Vec<i32>| v.len()));
        assert_eq!(result, 5);
    }

    #[rstest]
    fn state_gets_with_struct_field() {
        #[derive(Clone)]
        struct Config {
            value: i32,
        }

        let handler = StateHandler::new(Config { value: 42 });
        let (result, _) = handler.run(StateEffect::gets(|c: &Config| c.value));
        assert_eq!(result, 42);
    }

    #[rstest]
    fn state_get_then_put() {
        let handler = StateHandler::new(10);
        let computation = StateEffect::<i32>::get().flat_map(|x| StateEffect::put(x + 5));
        let ((), final_state) = handler.run(computation);
        assert_eq!(final_state, 15);
    }

    #[rstest]
    fn state_put_then_get() {
        let handler = StateHandler::new(0);
        let computation = StateEffect::put(42).then(StateEffect::get());
        let (result, final_state) = handler.run(computation);
        assert_eq!(result, 42);
        assert_eq!(final_state, 42);
    }

    #[rstest]
    fn state_counter_pattern() {
        let handler = StateHandler::new(0);

        let increment = || StateEffect::modify(|x: i32| x + 1);
        let computation = increment()
            .then(increment())
            .then(increment())
            .then(StateEffect::get());

        let (result, final_state) = handler.run(computation);
        assert_eq!(result, 3);
        assert_eq!(final_state, 3);
    }

    #[rstest]
    fn state_accumulator_pattern() {
        let handler = StateHandler::new(Vec::new());

        let computation = StateEffect::modify(|mut v: Vec<i32>| {
            v.push(1);
            v
        })
        .then(StateEffect::modify(|mut v: Vec<i32>| {
            v.push(2);
            v
        }))
        .then(StateEffect::modify(|mut v: Vec<i32>| {
            v.push(3);
            v
        }))
        .then(StateEffect::get());

        let (result, final_state) = handler.run(computation);
        assert_eq!(result, vec![1, 2, 3]);
        assert_eq!(final_state, vec![1, 2, 3]);
    }

    #[rstest]
    fn state_pure_value_does_not_change_state() {
        let handler = StateHandler::new(42);
        let computation: Eff<StateEffect<i32>, &str> = Eff::pure("constant");
        let (result, final_state) = handler.run(computation);
        assert_eq!(result, "constant");
        assert_eq!(final_state, 42);
    }

    #[rstest]
    fn state_operations_can_be_chained() {
        let handler = StateHandler::new(0);
        let computation = StateEffect::<i32>::get()
            .flat_map(|a| StateEffect::put(a + 10).then(StateEffect::get()))
            .flat_map(|b| StateEffect::put(b * 2).then(StateEffect::get()));
        let (result, final_state) = handler.run(computation);
        assert_eq!(result, 20); // (0 + 10) * 2
        assert_eq!(final_state, 20);
    }

    #[rstest]
    fn state_fmap_transforms_result() {
        let handler = StateHandler::new(21);
        let computation = StateEffect::<i32>::get().fmap(|x| x * 2);
        let (result, final_state) = handler.run(computation);
        assert_eq!(result, 42);
        assert_eq!(final_state, 21); // State unchanged by fmap
    }

    #[rstest]
    fn state_deep_chain_is_stack_safe() {
        let handler = StateHandler::new(0);
        let mut computation: Eff<StateEffect<i32>, ()> = Eff::pure(());
        for _ in 0..1000 {
            computation = computation.then(StateEffect::modify(|x: i32| x + 1));
        }
        let ((), final_state) = handler.run(computation);
        assert_eq!(final_state, 1000);
    }

    #[rstest]
    fn state_deep_flat_map_is_stack_safe() {
        let handler = StateHandler::new(0);
        let mut computation: Eff<StateEffect<i32>, i32> = StateEffect::get();
        for _ in 0..1000 {
            computation = computation
                .flat_map(|_| StateEffect::modify(|x: i32| x + 1).then(StateEffect::get()));
        }
        let (result, final_state) = handler.run(computation);
        assert_eq!(result, 1000);
        assert_eq!(final_state, 1000);
    }

    #[rstest]
    fn state_stack_operations() {
        let handler = StateHandler::new(Vec::<i32>::new());

        let push = |value: i32| {
            StateEffect::modify(move |mut stack: Vec<i32>| {
                stack.push(value);
                stack
            })
        };

        let pop = || {
            StateEffect::<Vec<i32>>::get().flat_map(|mut stack| {
                let value = stack.pop();
                StateEffect::put(stack).fmap(move |_| value)
            })
        };

        let computation = push(1)
            .then(push(2))
            .then(push(3))
            .then(pop())
            .flat_map(|popped| StateEffect::get().fmap(move |stack| (popped, stack)));

        let ((popped, remaining_stack), final_state) = handler.run(computation);
        assert_eq!(popped, Some(3));
        assert_eq!(remaining_stack, vec![1, 2]);
        assert_eq!(final_state, vec![1, 2]);
    }
}
