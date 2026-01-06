//! `StateT` - State Monad Transformer.
//!
//! `StateT` adds state manipulation capability to any monad.
//! It transforms a monad M into a monad that can read and write state S.
//!
//! # Overview
//!
//! `StateT<S, M>` encapsulates a function `S -> M<(A, S)>` where `S` is the state
//! type and `M` is the inner monad. This allows composing stateful computations
//! while also using the capabilities of the inner monad.
//!
//! # Design Note
//!
//! Due to Rust's lack of Higher-Kinded Types (HKT), we cannot write a single
//! generic implementation that works for all monads. Instead, we provide
//! specific methods for common monads (Option, Result, IO) using the naming
//! convention `method_option`, `method_result`, `method_io`.
//!
//! # Examples
//!
//! With Option:
//!
//! ```rust
//! use lambars::effect::StateT;
//!
//! let state: StateT<i32, Option<(i32, i32)>> = StateT::new(|s| Some((s * 2, s + 1)));
//! assert_eq!(state.run(10), Some((20, 11)));
//! ```
//!
//! With Result:
//!
//! ```rust
//! use lambars::effect::StateT;
//!
//! let state: StateT<i32, Result<(i32, i32), String>> = StateT::new(|s| Ok((s * 2, s + 1)));
//! assert_eq!(state.run(10), Ok((20, 11)));
//! ```

#![forbid(unsafe_code)]

use std::rc::Rc;

use super::IO;
use super::error::{AlreadyConsumedError, EffectError, EffectType};

/// A monad transformer that adds state manipulation capability.
///
/// `StateT<S, M>` represents a computation that, given an initial state of type `S`,
/// produces a value and a new state wrapped in monad `M`.
///
/// # Type Parameters
///
/// - `S`: The state type
/// - `M`: The inner monad type (e.g., `Option<(A, S)>`, `Result<(A, S), E>`, `IO<(A, S)>`)
///
/// # Examples
///
/// ```rust
/// use lambars::effect::StateT;
///
/// fn increment() -> StateT<i32, Option<((), i32)>> {
///     StateT::<i32, Option<((), i32)>>::modify_option(|count| count + 1)
/// }
///
/// let computation = increment()
///     .flat_map_option(|_| increment())
///     .flat_map_option(|_| StateT::<i32, Option<(i32, i32)>>::get_option());
///
/// assert_eq!(computation.run(0), Some((2, 2)));
/// ```
pub struct StateT<S, M>
where
    S: 'static,
{
    /// The wrapped state transition function.
    /// Uses Rc to allow cloning of the `StateT` for `flat_map`.
    run_function: Rc<dyn Fn(S) -> M>,
}

impl<S, M> StateT<S, M>
where
    S: 'static,
    M: 'static,
{
    /// Creates a new `StateT` from a state transition function.
    ///
    /// # Arguments
    ///
    /// * `transition` - A function that takes the current state and returns
    ///   a wrapped tuple of (result, `new_state`)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::StateT;
    ///
    /// let state: StateT<i32, Option<(i32, i32)>> = StateT::new(|s| Some((s * 2, s + 1)));
    /// assert_eq!(state.run(10), Some((20, 11)));
    /// ```
    pub fn new<F>(transition: F) -> Self
    where
        F: Fn(S) -> M + 'static,
    {
        Self {
            run_function: Rc::new(transition),
        }
    }

    /// Runs the `StateT` computation with the given initial state.
    ///
    /// # Arguments
    ///
    /// * `initial_state` - The initial state to run the computation with
    ///
    /// # Returns
    ///
    /// The result wrapped in the inner monad.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::StateT;
    ///
    /// let state: StateT<i32, Option<(i32, i32)>> = StateT::new(|s| Some((s + 1, s * 2)));
    /// assert_eq!(state.run(10), Some((11, 20)));
    /// ```
    pub fn run(&self, initial_state: S) -> M {
        (self.run_function)(initial_state)
    }
}

// =============================================================================
// Clone Implementation
// =============================================================================

impl<S, M> Clone for StateT<S, M>
where
    S: 'static,
{
    fn clone(&self) -> Self {
        Self {
            run_function: self.run_function.clone(),
        }
    }
}

// =============================================================================
// Option-specific Methods
// =============================================================================

impl<S, A> StateT<S, Option<(A, S)>>
where
    S: 'static,
    A: 'static,
{
    /// Runs the `StateT` and returns only the result value.
    ///
    /// # Arguments
    ///
    /// * `initial_state` - The initial state to run the computation with
    ///
    /// # Returns
    ///
    /// The result value wrapped in Option.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::StateT;
    ///
    /// let state: StateT<i32, Option<(i32, i32)>> = StateT::new(|s| Some((s * 2, s + 1)));
    /// assert_eq!(state.eval(10), Some(20));
    /// ```
    pub fn eval(&self, initial_state: S) -> Option<A> {
        self.run(initial_state).map(|(value, _)| value)
    }

    /// Runs the `StateT` and returns only the final state.
    ///
    /// # Arguments
    ///
    /// * `initial_state` - The initial state to run the computation with
    ///
    /// # Returns
    ///
    /// The final state wrapped in Option.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::StateT;
    ///
    /// let state: StateT<i32, Option<(i32, i32)>> = StateT::new(|s| Some((s * 2, s + 1)));
    /// assert_eq!(state.exec(10), Some(11));
    /// ```
    pub fn exec(&self, initial_state: S) -> Option<S> {
        self.run(initial_state).map(|(_, state)| state)
    }

    /// Creates a `StateT` that returns a constant value without modifying the state.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to return
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::StateT;
    ///
    /// let state: StateT<i32, Option<(String, i32)>> = StateT::pure_option("hello".to_string());
    /// assert_eq!(state.run(42), Some(("hello".to_string(), 42)));
    /// ```
    pub fn pure_option(value: A) -> Self
    where
        A: Clone,
    {
        Self::new(move |state| Some((value.clone(), state)))
    }

    /// Lifts an Option into `StateT`.
    ///
    /// # Arguments
    ///
    /// * `inner` - The Option to lift
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::StateT;
    ///
    /// let inner: Option<String> = Some("hello".to_string());
    /// let state: StateT<i32, Option<(String, i32)>> = StateT::lift_option(inner);
    /// assert_eq!(state.run(42), Some(("hello".to_string(), 42)));
    /// ```
    pub fn lift_option(inner: Option<A>) -> Self
    where
        A: Clone,
    {
        Self::new(move |state| inner.clone().map(|value| (value, state)))
    }

    /// Maps a function over the value inside the Option.
    ///
    /// # Arguments
    ///
    /// * `function` - The function to apply to the value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::StateT;
    ///
    /// let state: StateT<i32, Option<(i32, i32)>> = StateT::new(|s| Some((s, s + 1)));
    /// let mapped = state.fmap_option(|v| v * 2);
    /// assert_eq!(mapped.run(10), Some((20, 11)));
    /// ```
    pub fn fmap_option<B, F>(self, function: F) -> StateT<S, Option<(B, S)>>
    where
        F: Fn(A) -> B + 'static,
        B: 'static,
    {
        let original = self.run_function;
        StateT::new(move |state| {
            (original)(state).map(|(value, new_state)| (function(value), new_state))
        })
    }

    /// Chains `StateT` computations with Option.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes the value and returns a new `StateT`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::StateT;
    ///
    /// let state: StateT<i32, Option<(i32, i32)>> = StateT::new(|s| Some((s, s + 1)));
    /// let chained = state.flat_map_option(|v| {
    ///     StateT::new(move |s| Some((v + s, s * 2)))
    /// });
    /// // Initial state 10: first (10, 11), then (10 + 11, 22) = (21, 22)
    /// assert_eq!(chained.run(10), Some((21, 22)));
    /// ```
    pub fn flat_map_option<B, F>(self, function: F) -> StateT<S, Option<(B, S)>>
    where
        F: Fn(A) -> StateT<S, Option<(B, S)>> + 'static,
        B: 'static,
    {
        let original = self.run_function;
        StateT::new(move |state| match (original)(state) {
            Some((value, intermediate_state)) => {
                let next = function(value);
                next.run(intermediate_state)
            }
            None => None,
        })
    }

    /// Returns the current state as the result.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::StateT;
    ///
    /// let state: StateT<i32, Option<(i32, i32)>> = StateT::get_option();
    /// assert_eq!(state.run(42), Some((42, 42)));
    /// ```
    #[must_use]
    pub fn get_option() -> Self
    where
        S: Clone,
        A: From<S>,
    {
        Self::new(|state: S| Some((A::from(state.clone()), state)))
    }

    /// Replaces the current state with a new value.
    ///
    /// # Arguments
    ///
    /// * `new_state` - The new state value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::StateT;
    ///
    /// let state: StateT<i32, Option<((), i32)>> =
    ///     StateT::<i32, Option<((), i32)>>::put_option(100);
    /// assert_eq!(state.run(42), Some(((), 100)));
    /// ```
    pub fn put_option(new_state: S) -> StateT<S, Option<((), S)>>
    where
        S: Clone,
    {
        StateT::new(move |_| Some(((), new_state.clone())))
    }

    /// Modifies the current state using a function.
    ///
    /// # Arguments
    ///
    /// * `modifier` - A function that transforms the state
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::StateT;
    ///
    /// let state: StateT<i32, Option<((), i32)>> =
    ///     StateT::<i32, Option<((), i32)>>::modify_option(|s| s * 2);
    /// assert_eq!(state.run(21), Some(((), 42)));
    /// ```
    pub fn modify_option<F>(modifier: F) -> StateT<S, Option<((), S)>>
    where
        F: Fn(S) -> S + 'static,
    {
        StateT::new(move |state| Some(((), modifier(state))))
    }
}

// =============================================================================
// Result-specific Methods
// =============================================================================

impl<S, A, E> StateT<S, Result<(A, S), E>>
where
    S: 'static,
    A: 'static,
    E: 'static,
{
    /// Runs the `StateT` and returns only the result value.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the inner computation fails.
    pub fn eval(&self, initial_state: S) -> Result<A, E> {
        self.run(initial_state).map(|(value, _)| value)
    }

    /// Runs the `StateT` and returns only the final state.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the inner computation fails.
    pub fn exec(&self, initial_state: S) -> Result<S, E> {
        self.run(initial_state).map(|(_, state)| state)
    }

    /// Creates a `StateT` that returns a constant value without modifying the state.
    pub fn pure_result(value: A) -> Self
    where
        A: Clone,
    {
        Self::new(move |state| Ok((value.clone(), state)))
    }

    /// Lifts a Result into `StateT`.
    pub fn lift_result(inner: Result<A, E>) -> Self
    where
        A: Clone,
        E: Clone,
    {
        Self::new(move |state| inner.clone().map(|value| (value, state)))
    }

    /// Maps a function over the value inside the Result.
    pub fn fmap_result<B, F>(self, function: F) -> StateT<S, Result<(B, S), E>>
    where
        F: Fn(A) -> B + 'static,
        B: 'static,
    {
        let original = self.run_function;
        StateT::new(move |state| {
            (original)(state).map(|(value, new_state)| (function(value), new_state))
        })
    }

    /// Chains `StateT` computations with Result.
    pub fn flat_map_result<B, F>(self, function: F) -> StateT<S, Result<(B, S), E>>
    where
        F: Fn(A) -> StateT<S, Result<(B, S), E>> + 'static,
        B: 'static,
    {
        let original = self.run_function;
        StateT::new(move |state| match (original)(state) {
            Ok((value, intermediate_state)) => {
                let next = function(value);
                next.run(intermediate_state)
            }
            Err(error) => Err(error),
        })
    }

    /// Returns the current state as the result.
    #[must_use]
    pub fn get_result() -> Self
    where
        S: Clone,
        A: From<S>,
    {
        Self::new(|state: S| Ok((A::from(state.clone()), state)))
    }

    /// Replaces the current state with a new value.
    pub fn put_result(new_state: S) -> StateT<S, Result<((), S), E>>
    where
        S: Clone,
    {
        StateT::new(move |_| Ok(((), new_state.clone())))
    }

    /// Modifies the current state using a function.
    pub fn modify_result<F>(modifier: F) -> StateT<S, Result<((), S), E>>
    where
        F: Fn(S) -> S + 'static,
    {
        StateT::new(move |state| Ok(((), modifier(state))))
    }
}

// =============================================================================
// IO-specific Methods
// =============================================================================

impl<S, A> StateT<S, IO<(A, S)>>
where
    S: 'static,
    A: 'static,
{
    /// Creates a `StateT` that returns a constant value without modifying the state.
    pub fn pure_io(value: A) -> Self
    where
        A: Clone,
    {
        Self::new(move |state| IO::pure((value.clone(), state)))
    }

    /// Lifts an IO into `StateT`.
    ///
    /// # Panics
    ///
    /// Panics if the resulting `StateT` is run more than once.
    #[deprecated(
        since = "0.2.0",
        note = "Use try_lift_io instead, which returns Result instead of panicking"
    )]
    #[must_use]
    pub fn lift_io(inner: IO<A>) -> Self {
        let inner_rc = Rc::new(std::cell::RefCell::new(Some(inner)));
        Self::new(move |state| {
            let io = inner_rc.borrow_mut().take().unwrap_or_else(|| {
                panic!("StateT::lift_io: IO already consumed. Use the StateT only once.")
            });
            io.fmap(move |value| (value, state))
        })
    }

    /// Lifts an `IO` into `StateT`, returning `Result` instead of panicking.
    ///
    /// This is the safe version of [`lift_io`](Self::lift_io) that returns
    /// an error instead of panicking when the IO is consumed more than once.
    ///
    /// # Arguments
    ///
    /// * `inner` - The IO computation to lift
    ///
    /// # Returns
    ///
    /// A `StateT` that, when run, returns `IO<Result<(A, S), EffectError>>`.
    /// The first execution returns `Ok((value, state))`, subsequent executions
    /// return `Err(EffectError::AlreadyConsumed)`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::{StateT, IO, EffectError};
    ///
    /// let io = IO::pure(42);
    /// let state: StateT<String, IO<Result<(i32, String), EffectError>>> =
    ///     StateT::try_lift_io(io);
    ///
    /// let result = state.run("initial".to_string()).run_unsafe();
    /// assert_eq!(result, Ok((42, "initial".to_string())));
    /// ```
    #[must_use]
    #[allow(clippy::option_if_let_else, clippy::type_complexity)]
    pub fn try_lift_io(inner: IO<A>) -> StateT<S, IO<Result<(A, S), EffectError>>> {
        let inner_rc = Rc::new(std::cell::RefCell::new(Some(inner)));
        StateT::new(move |state: S| match inner_rc.borrow_mut().take() {
            Some(io) => io.fmap(move |value| Ok((value, state))),
            None => IO::pure(Err(EffectError::AlreadyConsumed(AlreadyConsumedError {
                transformer_name: "StateT",
                method_name: "try_lift_io",
                effect_type: EffectType::IO,
            }))),
        })
    }

    /// Maps a function over the value inside the IO.
    pub fn fmap_io<B, F>(self, function: F) -> StateT<S, IO<(B, S)>>
    where
        F: Fn(A) -> B + 'static,
        B: 'static,
    {
        let original = self.run_function;
        let function_rc = Rc::new(function);
        StateT::new(move |state| {
            let io = (original)(state);
            let function_clone = function_rc.clone();
            io.fmap(move |(value, new_state)| (function_clone(value), new_state))
        })
    }

    /// Chains `StateT` computations with IO.
    pub fn flat_map_io<B, F>(self, function: F) -> StateT<S, IO<(B, S)>>
    where
        F: Fn(A) -> StateT<S, IO<(B, S)>> + 'static,
        B: 'static,
    {
        let original = self.run_function;
        let function_rc = Rc::new(function);
        StateT::new(move |state| {
            let io = (original)(state);
            let function_clone = function_rc.clone();
            io.flat_map(move |(value, intermediate_state)| {
                let next = function_clone(value);
                next.run(intermediate_state)
            })
        })
    }

    /// Returns the current state as the result.
    #[must_use]
    pub fn get_io() -> Self
    where
        S: Clone,
        A: From<S>,
    {
        Self::new(|state: S| IO::pure((A::from(state.clone()), state)))
    }

    /// Replaces the current state with a new value.
    pub fn put_io(new_state: S) -> StateT<S, IO<((), S)>>
    where
        S: Clone,
    {
        StateT::new(move |_| IO::pure(((), new_state.clone())))
    }

    /// Modifies the current state using a function.
    pub fn modify_io<F>(modifier: F) -> StateT<S, IO<((), S)>>
    where
        F: Fn(S) -> S + 'static,
    {
        StateT::new(move |state| IO::pure(((), modifier(state))))
    }
}

// =============================================================================
// AsyncIO-specific Methods (requires async feature)
// =============================================================================

#[cfg(feature = "async")]
use super::AsyncIO;

#[cfg(feature = "async")]
impl<S, A> StateT<S, AsyncIO<(A, S)>>
where
    S: Send + 'static,
    A: Send + 'static,
{
    /// Runs the `StateT` and returns only the result value.
    ///
    /// # Arguments
    ///
    /// * `initial_state` - The initial state to run the computation with
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{StateT, AsyncIO};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let state: StateT<i32, AsyncIO<(i32, i32)>> = StateT::new(|s| AsyncIO::pure((s * 2, s + 1)));
    ///     assert_eq!(state.eval_async(10).run_async().await, 20);
    /// }
    /// ```
    pub fn eval_async(&self, initial_state: S) -> AsyncIO<A> {
        self.run(initial_state).fmap(|(value, _)| value)
    }

    /// Runs the `StateT` and returns only the final state.
    ///
    /// # Arguments
    ///
    /// * `initial_state` - The initial state to run the computation with
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{StateT, AsyncIO};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let state: StateT<i32, AsyncIO<(i32, i32)>> = StateT::new(|s| AsyncIO::pure((s * 2, s + 1)));
    ///     assert_eq!(state.exec_async(10).run_async().await, 11);
    /// }
    /// ```
    pub fn exec_async(&self, initial_state: S) -> AsyncIO<S> {
        self.run(initial_state).fmap(|(_, state)| state)
    }

    /// Creates a `StateT` that returns a constant value without modifying the state.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{StateT, AsyncIO};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let state: StateT<i32, AsyncIO<(String, i32)>> = StateT::pure_async_io("hello".to_string());
    ///     assert_eq!(state.run(42).run_async().await, ("hello".to_string(), 42));
    /// }
    /// ```
    pub fn pure_async_io(value: A) -> Self
    where
        A: Clone,
    {
        Self::new(move |state| AsyncIO::pure((value.clone(), state)))
    }

    /// Maps a function over the value inside the `AsyncIO`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{StateT, AsyncIO};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let state: StateT<i32, AsyncIO<(i32, i32)>> = StateT::new(|s| AsyncIO::pure((s, s + 1)));
    ///     let mapped = state.fmap_async_io(|v| v * 2);
    ///     assert_eq!(mapped.run(10).run_async().await, (20, 11));
    /// }
    /// ```
    pub fn fmap_async_io<B, F>(self, function: F) -> StateT<S, AsyncIO<(B, S)>>
    where
        F: Fn(A) -> B + Send + Sync + 'static,
        B: Send + 'static,
    {
        let original = self.run_function;
        let function_rc = std::sync::Arc::new(function);
        StateT::new(move |state| {
            let async_io = (original)(state);
            let function_clone = function_rc.clone();
            async_io.fmap(move |(value, new_state)| (function_clone(value), new_state))
        })
    }

    /// Chains `StateT` computations with `AsyncIO`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{StateT, AsyncIO};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let state: StateT<i32, AsyncIO<(i32, i32)>> = StateT::new(|s| AsyncIO::pure((s, s + 1)));
    ///     let chained = state.flat_map_async_io(|v| {
    ///         StateT::new(move |s| AsyncIO::pure((v + s, s * 2)))
    ///     });
    ///     // Initial state 10: first (10, 11), then (10 + 11, 22) = (21, 22)
    ///     assert_eq!(chained.run(10).run_async().await, (21, 22));
    /// }
    /// ```
    pub fn flat_map_async_io<B, F>(self, function: F) -> StateT<S, AsyncIO<(B, S)>>
    where
        F: Fn(A) -> StateT<S, AsyncIO<(B, S)>> + Send + Sync + 'static,
        B: Send + 'static,
    {
        let original = self.run_function;
        let function_arc = std::sync::Arc::new(function);
        StateT::new(move |state| {
            let async_io = (original)(state);
            let function_clone = function_arc.clone();
            async_io.flat_map(move |(value, intermediate_state)| {
                let next = function_clone(value);
                next.run(intermediate_state)
            })
        })
    }

    /// Returns the current state as the result.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{StateT, AsyncIO};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let state: StateT<i32, AsyncIO<(i32, i32)>> = StateT::get_async_io();
    ///     assert_eq!(state.run(42).run_async().await, (42, 42));
    /// }
    /// ```
    #[must_use]
    pub fn get_async_io() -> Self
    where
        S: Clone,
        A: From<S>,
    {
        Self::new(|state: S| AsyncIO::pure((A::from(state.clone()), state)))
    }

    /// Replaces the current state with a new value.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{StateT, AsyncIO};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let state: StateT<i32, AsyncIO<((), i32)>> =
    ///         StateT::<i32, AsyncIO<((), i32)>>::put_async_io(100);
    ///     assert_eq!(state.run(42).run_async().await, ((), 100));
    /// }
    /// ```
    pub fn put_async_io(new_state: S) -> StateT<S, AsyncIO<((), S)>>
    where
        S: Clone,
    {
        StateT::new(move |_| AsyncIO::pure(((), new_state.clone())))
    }

    /// Modifies the current state using a function.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{StateT, AsyncIO};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let state: StateT<i32, AsyncIO<((), i32)>> =
    ///         StateT::<i32, AsyncIO<((), i32)>>::modify_async_io(|s| s * 2);
    ///     assert_eq!(state.run(21).run_async().await, ((), 42));
    /// }
    /// ```
    pub fn modify_async_io<F>(modifier: F) -> StateT<S, AsyncIO<((), S)>>
    where
        F: Fn(S) -> S + Send + 'static,
    {
        let modifier_arc = std::sync::Arc::new(modifier);
        StateT::new(move |state| {
            let modifier_clone = modifier_arc.clone();
            AsyncIO::pure(((), modifier_clone(state)))
        })
    }

    /// Lifts an `AsyncIO` into `StateT`.
    ///
    /// The state is not modified; the resulting `StateT` returns
    /// the `AsyncIO`'s result paired with the unchanged state.
    ///
    /// # Important: Single Use Only
    ///
    /// The resulting `StateT` can only be run **once**. Running it multiple
    /// times will cause a panic. This is because `AsyncIO` is not `Clone`,
    /// so we cannot share the inner computation across multiple runs.
    ///
    /// # Panics
    ///
    /// Panics if the `StateT` is run more than once.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{StateT, AsyncIO};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let async_io = AsyncIO::pure(42);
    ///     #[allow(deprecated)]
    ///     let state: StateT<i32, AsyncIO<(i32, i32)>> = StateT::lift_async_io(async_io);
    ///     let (result, final_state) = state.run(100).run_async().await;
    ///     assert_eq!(result, 42);
    ///     assert_eq!(final_state, 100);
    /// }
    /// ```
    #[deprecated(
        since = "0.2.0",
        note = "Use try_lift_async_io instead, which returns Result instead of panicking"
    )]
    #[must_use]
    pub fn lift_async_io(inner: AsyncIO<A>) -> Self
    where
        S: Clone,
    {
        let inner_arc = std::sync::Arc::new(std::sync::Mutex::new(Some(inner)));
        Self::new(move |state: S| {
            let async_io = inner_arc.lock().unwrap().take().unwrap_or_else(|| {
                panic!("StateT::lift_async_io: AsyncIO already consumed. Use the StateT only once.")
            });
            async_io.fmap(move |value| (value, state))
        })
    }

    /// Lifts an `AsyncIO` into `StateT`, returning `Result` instead of panicking.
    ///
    /// This is the safe version of [`lift_async_io`](Self::lift_async_io) that returns
    /// an error instead of panicking when the `AsyncIO` is consumed more than once.
    ///
    /// # Arguments
    ///
    /// * `inner` - The `AsyncIO` computation to lift
    ///
    /// # Returns
    ///
    /// A `StateT` that, when run, returns `AsyncIO<Result<(A, S), EffectError>>`.
    /// The first execution returns `Ok((value, state))`, subsequent executions
    /// return `Err(EffectError::AlreadyConsumed)`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{StateT, AsyncIO, EffectError};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let async_io = AsyncIO::pure(42);
    ///     let state: StateT<String, AsyncIO<Result<(i32, String), EffectError>>> =
    ///         StateT::try_lift_async_io(async_io);
    ///
    ///     let result = state.run("initial".to_string()).run_async().await;
    ///     assert_eq!(result, Ok((42, "initial".to_string())));
    /// }
    /// ```
    ///
    /// # Panics
    ///
    /// This function will not panic under normal circumstances. The internal
    /// `Mutex::lock` is only used for interior mutability in single-threaded
    /// contexts and will not be poisoned.
    #[must_use]
    #[allow(clippy::option_if_let_else, clippy::type_complexity)]
    pub fn try_lift_async_io(inner: AsyncIO<A>) -> StateT<S, AsyncIO<Result<(A, S), EffectError>>>
    where
        S: Clone,
    {
        let inner_arc = std::sync::Arc::new(std::sync::Mutex::new(Some(inner)));
        StateT::new(move |state: S| {
            let mut guard = inner_arc.lock().unwrap();
            match guard.take() {
                Some(async_io) => async_io.fmap(move |value| Ok((value, state))),
                None => AsyncIO::pure(Err(EffectError::AlreadyConsumed(AlreadyConsumedError {
                    transformer_name: "StateT",
                    method_name: "try_lift_async_io",
                    effect_type: EffectType::AsyncIO,
                }))),
            }
        })
    }

    /// Projects a value from the state without modifying it.
    ///
    /// This is a convenient way to read a part of the state. The projection
    /// function receives a reference to the state and returns the projected value.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{StateT, AsyncIO};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let state: StateT<i32, AsyncIO<(i32, i32)>> =
    ///         StateT::gets_async_io(|s: &i32| s * 2);
    ///     let (result, final_state) = state.run(21).run_async().await;
    ///     assert_eq!(result, 42);
    ///     assert_eq!(final_state, 21);
    /// }
    /// ```
    #[must_use]
    pub fn gets_async_io<F>(projection: F) -> Self
    where
        S: Clone,
        F: Fn(&S) -> A + Send + Sync + 'static,
    {
        let projection_arc = std::sync::Arc::new(projection);
        Self::new(move |state: S| {
            let projection_clone = projection_arc.clone();
            let value = projection_clone(&state);
            AsyncIO::pure((value, state))
        })
    }

    /// Executes a state transition function.
    ///
    /// The transition function takes the current state and returns both
    /// a result value and the new state.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{StateT, AsyncIO};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let state: StateT<i32, AsyncIO<(String, i32)>> =
    ///         StateT::state_async_io(|s| (format!("was: {}", s), s + 1));
    ///     let (result, final_state) = state.run(41).run_async().await;
    ///     assert_eq!(result, "was: 41");
    ///     assert_eq!(final_state, 42);
    /// }
    /// ```
    #[must_use]
    pub fn state_async_io<F>(transition: F) -> Self
    where
        F: Fn(S) -> (A, S) + Send + Sync + 'static,
    {
        let transition_arc = std::sync::Arc::new(transition);
        Self::new(move |state: S| {
            let transition_clone = transition_arc.clone();
            AsyncIO::pure(transition_clone(state))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // IO-specific Tests
    // =========================================================================

    #[allow(clippy::type_complexity)]
    mod io_tests {
        use super::*;
        use crate::effect::{AlreadyConsumedError, EffectError, EffectType};
        use rstest::rstest;

        #[rstest]
        #[case(42, "initial", Ok((42, "initial".to_string())))]
        #[case(0, "test", Ok((0, "test".to_string())))]
        #[case(-1, "negative", Ok((-1, "negative".to_string())))]
        fn state_transformer_try_lift_io_success(
            #[case] value: i32,
            #[case] initial_state: &str,
            #[case] expected: Result<(i32, String), EffectError>,
        ) {
            let io = IO::pure(value);
            let state: StateT<String, IO<Result<(i32, String), EffectError>>> =
                StateT::try_lift_io(io);
            let result = state.run(initial_state.to_string()).run_unsafe();
            assert_eq!(result, expected);
        }

        #[rstest]
        fn state_transformer_try_lift_io_already_consumed() {
            let io = IO::pure(42);
            let state: StateT<String, IO<Result<(i32, String), EffectError>>> =
                StateT::try_lift_io(io);

            let cloned = state.clone();

            let result1 = state.run("state1".to_string()).run_unsafe();
            assert_eq!(result1, Ok((42, "state1".to_string())));

            let result2 = cloned.run("state2".to_string()).run_unsafe();
            assert!(matches!(
                result2,
                Err(EffectError::AlreadyConsumed(AlreadyConsumedError {
                    transformer_name: "StateT",
                    method_name: "try_lift_io",
                    effect_type: EffectType::IO,
                }))
            ));
        }

        #[rstest]
        fn state_transformer_try_lift_io_error_message() {
            let io = IO::pure(42);
            let state: StateT<String, IO<Result<(i32, String), EffectError>>> =
                StateT::try_lift_io(io);

            let cloned = state.clone();
            let _ = state.run("state".to_string()).run_unsafe();
            let result = cloned.run("state".to_string()).run_unsafe();

            match result {
                Err(error) => {
                    assert_eq!(
                        error.to_string(),
                        "StateT::try_lift_io: IO already consumed. Use the transformer only once."
                    );
                }
                Ok(_) => panic!("Expected error"),
            }
        }
    }

    // =========================================================================
    // Option-specific Tests
    // =========================================================================

    #[test]
    fn state_transformer_new_and_run() {
        let state: StateT<i32, Option<(i32, i32)>> = StateT::new(|s| Some((s * 2, s + 1)));
        assert_eq!(state.run(10), Some((20, 11)));
    }

    #[test]
    fn state_transformer_clone() {
        let state: StateT<i32, Option<(i32, i32)>> = StateT::new(|s| Some((s * 2, s + 1)));
        let cloned = state.clone();
        assert_eq!(state.run(10), Some((20, 11)));
        assert_eq!(cloned.run(10), Some((20, 11)));
    }

    #[test]
    fn state_transformer_pure_option() {
        let state: StateT<i32, Option<(i32, i32)>> = StateT::pure_option(42);
        assert_eq!(state.run(10), Some((42, 10)));
    }

    #[test]
    fn state_transformer_get_option() {
        let state: StateT<i32, Option<(i32, i32)>> = StateT::get_option();
        assert_eq!(state.run(42), Some((42, 42)));
    }

    #[test]
    fn state_transformer_flat_map_option() {
        let state: StateT<i32, Option<(i32, i32)>> = StateT::new(|s| Some((s, s + 1)));
        let chained = state.flat_map_option(|v| StateT::new(move |s| Some((v + s, s * 2))));
        assert_eq!(chained.run(10), Some((21, 22)));
    }

    // =========================================================================
    // AsyncIO-specific Tests (requires async feature)
    // =========================================================================

    #[cfg(feature = "async")]
    #[allow(clippy::type_complexity, clippy::future_not_send)]
    mod async_io_tests {
        use super::*;
        use crate::effect::{AlreadyConsumedError, EffectError, EffectType};
        use rstest::rstest;

        #[rstest]
        #[case(42, "initial", Ok((42, "initial".to_string())))]
        #[case(0, "test", Ok((0, "test".to_string())))]
        #[case(-1, "negative", Ok((-1, "negative".to_string())))]
        #[tokio::test]
        async fn state_transformer_try_lift_async_io_success(
            #[case] value: i32,
            #[case] initial_state: &str,
            #[case] expected: Result<(i32, String), EffectError>,
        ) {
            let async_io = AsyncIO::pure(value);
            let state: StateT<String, AsyncIO<Result<(i32, String), EffectError>>> =
                StateT::try_lift_async_io(async_io);
            let result = state.run(initial_state.to_string()).run_async().await;
            assert_eq!(result, expected);
        }

        #[rstest]
        #[tokio::test]
        async fn state_transformer_try_lift_async_io_already_consumed() {
            let async_io = AsyncIO::pure(42);
            let state: StateT<String, AsyncIO<Result<(i32, String), EffectError>>> =
                StateT::try_lift_async_io(async_io);

            let cloned = state.clone();

            let result1 = state.run("state1".to_string()).run_async().await;
            assert_eq!(result1, Ok((42, "state1".to_string())));

            let result2 = cloned.run("state2".to_string()).run_async().await;
            assert!(matches!(
                result2,
                Err(EffectError::AlreadyConsumed(AlreadyConsumedError {
                    transformer_name: "StateT",
                    method_name: "try_lift_async_io",
                    effect_type: EffectType::AsyncIO,
                }))
            ));
        }

        #[rstest]
        #[tokio::test]
        async fn state_transformer_try_lift_async_io_error_message() {
            let async_io = AsyncIO::pure(42);
            let state: StateT<String, AsyncIO<Result<(i32, String), EffectError>>> =
                StateT::try_lift_async_io(async_io);

            let cloned = state.clone();
            let _ = state.run("state".to_string()).run_async().await;
            let result = cloned.run("state".to_string()).run_async().await;

            match result {
                Err(error) => {
                    assert_eq!(
                        error.to_string(),
                        "StateT::try_lift_async_io: AsyncIO already consumed. Use the transformer only once."
                    );
                }
                Ok(_) => panic!("Expected error"),
            }
        }
    }
}
