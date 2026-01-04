//! State Monad - stateful computation.
//!
//! The State monad represents computations that thread a state through
//! a sequence of operations. It is useful for maintaining mutable state
//! in a pure functional way.
//!
//! # Overview
//!
//! A `State<S, A>` encapsulates a function `S -> (A, S)`, where `S` is the
//! state type and `A` is the result type. The function takes the current
//! state, produces a result, and returns a potentially modified state.
//!
//! # Note on Type Classes
//!
//! State provides its own `fmap`, `flat_map`, `map2`, etc. methods directly
//! on the type, rather than implementing the Functor/Applicative/Monad traits.
//! This is because Rust's type system requires 'static bounds on trait
//! implementations when using `Rc<dyn Fn>`, and the standard type class traits
//! don't have these bounds. The methods work identically to their type class
//! counterparts.
//!
//! # Laws
//!
//! State satisfies all Functor, Applicative, and Monad laws, plus the
//! MonadState-specific laws:
//!
//! ## Functor Laws
//!
//! - Identity: `state.fmap(|x| x) == state`
//! - Composition: `state.fmap(f).fmap(g) == state.fmap(|x| g(f(x)))`
//!
//! ## Monad Laws
//!
//! - Left Identity: `State::pure(a).flat_map(f) == f(a)`
//! - Right Identity: `m.flat_map(State::pure) == m`
//! - Associativity: `m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))`
//!
//! ## `MonadState` Laws
//!
//! - Get Put Law: `get().flat_map(|s| put(s)) == pure(())`
//! - Put Get Law: `put(s).then(get())` returns `s`
//! - Put Put Law: `put(s1).then(put(s2)) == put(s2)`
//! - Modify Composition: `modify(f).then(modify(g)) == modify(|s| g(f(s)))`
//!
//! # Examples
//!
//! Basic usage:
//!
//! ```rust
//! use lambars::effect::State;
//!
//! // Create a state that doubles the result and increments the state
//! let state: State<i32, i32> = State::new(|s: i32| (s * 2, s + 1));
//! let (result, final_state) = state.run(10);
//! assert_eq!(result, 20);
//! assert_eq!(final_state, 11);
//! ```
//!
//! Counter pattern:
//!
//! ```rust
//! use lambars::effect::State;
//!
//! fn increment() -> State<i32, ()> {
//!     State::modify(|count| count + 1)
//! }
//!
//! fn get_count() -> State<i32, i32> {
//!     State::get()
//! }
//!
//! let computation = increment()
//!     .then(increment())
//!     .then(increment())
//!     .then(get_count());
//!
//! let (count, _) = computation.run(0);
//! assert_eq!(count, 3);
//! ```

#![forbid(unsafe_code)]

use std::rc::Rc;

/// A monad for computations that thread state through a sequence of operations.
///
/// `State<S, A>` represents a computation that, given an initial state of type `S`,
/// produces a result of type `A` and a new state of type `S`.
///
/// # Type Parameters
///
/// - `S`: The state type
/// - `A`: The result type
///
/// # Examples
///
/// ```rust
/// use lambars::effect::State;
///
/// let computation: State<i32, i32> = State::get()
///     .flat_map(|current| {
///         State::put(current + 1).then(State::pure(current))
///     });
///
/// let (result, final_state) = computation.run(10);
/// assert_eq!(result, 10);
/// assert_eq!(final_state, 11);
/// ```
pub struct State<S, A>
where
    S: 'static,
    A: 'static,
{
    /// The wrapped state transition function.
    /// Uses Rc to allow cloning of the State for `flat_map`.
    run_function: Rc<dyn Fn(S) -> (A, S)>,
}

impl<S, A> State<S, A>
where
    S: 'static,
    A: 'static,
{
    /// Creates a new State from a state transition function.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes the current state and returns
    ///   a tuple of (result, `new_state`)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::State;
    ///
    /// let state: State<i32, i32> = State::new(|s: i32| (s * 2, s + 1));
    /// let (result, final_state) = state.run(10);
    /// assert_eq!(result, 20);
    /// assert_eq!(final_state, 11);
    /// ```
    pub fn new<F>(function: F) -> Self
    where
        F: Fn(S) -> (A, S) + 'static,
    {
        Self {
            run_function: Rc::new(function),
        }
    }

    /// Creates a new State from a state transition function.
    ///
    /// This is an alias for `new` that is more descriptive for state transitions.
    ///
    /// # Arguments
    ///
    /// * `transition` - A function that takes the current state and returns
    ///   a tuple of (result, `new_state`)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::State;
    ///
    /// let computation: State<i32, String> = State::from_transition(|s: i32| {
    ///     (format!("was: {}", s), s + 1)
    /// });
    /// let (result, final_state) = computation.run(10);
    /// assert_eq!(result, "was: 10");
    /// assert_eq!(final_state, 11);
    /// ```
    pub fn from_transition<F>(transition: F) -> Self
    where
        F: Fn(S) -> (A, S) + 'static,
    {
        Self::new(transition)
    }

    /// Runs the State computation with the given initial state.
    ///
    /// Returns both the result and the final state.
    ///
    /// # Arguments
    ///
    /// * `initial_state` - The initial state to run the computation with
    ///
    /// # Returns
    ///
    /// A tuple of (result, `final_state`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::State;
    ///
    /// let state: State<i32, i32> = State::new(|s: i32| (s + 1, s * 2));
    /// let (result, final_state) = state.run(10);
    /// assert_eq!(result, 11);
    /// assert_eq!(final_state, 20);
    /// ```
    pub fn run(&self, initial_state: S) -> (A, S) {
        (self.run_function)(initial_state)
    }

    /// Runs the State computation and returns only the result.
    ///
    /// # Arguments
    ///
    /// * `initial_state` - The initial state to run the computation with
    ///
    /// # Returns
    ///
    /// The result of the computation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::State;
    ///
    /// let state: State<i32, i32> = State::new(|s: i32| (s * 2, s + 1));
    /// assert_eq!(state.eval(10), 20);
    /// ```
    pub fn eval(&self, initial_state: S) -> A {
        let (result, _) = self.run(initial_state);
        result
    }

    /// Runs the State computation and returns only the final state.
    ///
    /// # Arguments
    ///
    /// * `initial_state` - The initial state to run the computation with
    ///
    /// # Returns
    ///
    /// The final state after running the computation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::State;
    ///
    /// let state: State<i32, i32> = State::new(|s: i32| (s * 2, s + 1));
    /// assert_eq!(state.exec(10), 11);
    /// ```
    pub fn exec(&self, initial_state: S) -> S {
        let (_, final_state) = self.run(initial_state);
        final_state
    }

    /// Creates a State that returns a constant value without modifying the state.
    ///
    /// This is equivalent to `Applicative::pure`.
    ///
    /// # Arguments
    ///
    /// * `value` - The constant value to return
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::State;
    ///
    /// let state: State<i32, &str> = State::pure("constant");
    /// let (result, final_state) = state.run(42);
    /// assert_eq!(result, "constant");
    /// assert_eq!(final_state, 42);
    /// ```
    pub fn pure(value: A) -> Self
    where
        A: Clone,
    {
        Self::new(move |state| (value.clone(), state))
    }

    /// Maps a function over the result of this State.
    ///
    /// This is the Functor operation for State.
    ///
    /// # Arguments
    ///
    /// * `function` - A function to apply to the result
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::State;
    ///
    /// let state: State<i32, i32> = State::new(|s: i32| (s, s));
    /// let mapped = state.fmap(|value| value * 2);
    /// let (result, final_state) = mapped.run(21);
    /// assert_eq!(result, 42);
    /// assert_eq!(final_state, 21);
    /// ```
    pub fn fmap<B, F>(self, function: F) -> State<S, B>
    where
        F: Fn(A) -> B + 'static,
        B: 'static,
    {
        let original_function = self.run_function;
        State::new(move |state| {
            let (result, new_state) = (original_function)(state);
            (function(result), new_state)
        })
    }

    /// Chains this State with a function that produces another State.
    ///
    /// This is the Monad operation for State.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes the result and produces a new State
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::State;
    ///
    /// let state: State<i32, i32> = State::new(|s: i32| (s, s + 1));
    /// let chained = state.flat_map(|value| {
    ///     State::new(move |s: i32| (value + s, s * 2))
    /// });
    /// let (result, final_state) = chained.run(10);
    /// // First: (10, 11), then with state 11: (10 + 11, 22)
    /// assert_eq!(result, 21);
    /// assert_eq!(final_state, 22);
    /// ```
    pub fn flat_map<B, F>(self, function: F) -> State<S, B>
    where
        F: Fn(A) -> State<S, B> + 'static,
        B: 'static,
    {
        let original_function = self.run_function;
        State::new(move |state| {
            let (result, intermediate_state) = (original_function)(state);
            let next_state = function(result);
            next_state.run(intermediate_state)
        })
    }

    /// Alias for `flat_map` to match Rust's naming conventions.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes the result and produces a new State
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::State;
    ///
    /// let state: State<i32, i32> = State::new(|s: i32| (s, s + 1));
    /// let chained = state.and_then(|value| {
    ///     State::new(move |s: i32| (value + s, s))
    /// });
    /// let (result, final_state) = chained.run(10);
    /// assert_eq!(result, 21); // 10 + 11
    /// assert_eq!(final_state, 11);
    /// ```
    pub fn and_then<B, F>(self, function: F) -> State<S, B>
    where
        F: Fn(A) -> State<S, B> + 'static,
        B: 'static,
    {
        self.flat_map(function)
    }

    /// Sequences two States, discarding the first result.
    ///
    /// # Arguments
    ///
    /// * `next` - The State to execute after this one
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::State;
    ///
    /// let state1: State<i32, i32> = State::new(|s: i32| (s, s + 10));
    /// let state2: State<i32, &str> = State::pure("result");
    /// let sequenced = state1.then(state2);
    /// let (result, final_state) = sequenced.run(42);
    /// assert_eq!(result, "result");
    /// assert_eq!(final_state, 52);
    /// ```
    #[must_use]
    pub fn then<B>(self, next: State<S, B>) -> State<S, B>
    where
        B: 'static,
    {
        self.flat_map(move |_| next.clone())
    }

    /// Combines two States using a binary function.
    ///
    /// This is the Applicative map2 operation for State.
    ///
    /// # Arguments
    ///
    /// * `other` - The second State
    /// * `function` - A function that combines the results
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::State;
    ///
    /// let state1: State<i32, i32> = State::new(|s: i32| (s, s + 1));
    /// let state2: State<i32, i32> = State::new(|s: i32| (s * 2, s + 1));
    /// let combined = state1.map2(state2, |a, b| a + b);
    /// let (result, final_state) = combined.run(10);
    /// // state1: (10, 11), state2 with 11: (22, 12)
    /// assert_eq!(result, 32);
    /// assert_eq!(final_state, 12);
    /// ```
    pub fn map2<B, C, F>(self, other: State<S, B>, function: F) -> State<S, C>
    where
        F: Fn(A, B) -> C + 'static,
        B: 'static,
        C: 'static,
    {
        let self_function = self.run_function;
        let other_function = other.run_function;
        State::new(move |state| {
            let (result_a, intermediate_state) = (self_function)(state);
            let (result_b, final_state) = (other_function)(intermediate_state);
            (function(result_a, result_b), final_state)
        })
    }

    /// Combines two States into a tuple.
    ///
    /// # Arguments
    ///
    /// * `other` - The second State
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::State;
    ///
    /// let state1: State<i32, i32> = State::new(|s: i32| (s, s + 1));
    /// let state2: State<i32, &str> = State::pure("hello");
    /// let product = state1.product(state2);
    /// let ((first, second), final_state) = product.run(42);
    /// assert_eq!(first, 42);
    /// assert_eq!(second, "hello");
    /// assert_eq!(final_state, 43);
    /// ```
    #[must_use]
    pub fn product<B>(self, other: State<S, B>) -> State<S, (A, B)>
    where
        B: 'static,
    {
        self.map2(other, |a, b| (a, b))
    }
}

// =============================================================================
// MonadState Operations (as inherent methods)
// =============================================================================

impl<St> State<St, St>
where
    St: Clone + 'static,
{
    /// Creates a State that returns the current state without modifying it.
    ///
    /// This is the fundamental "get" operation of `MonadState`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::State;
    ///
    /// let state: State<i32, i32> = State::get();
    /// let (result, final_state) = state.run(42);
    /// assert_eq!(result, 42);
    /// assert_eq!(final_state, 42);
    /// ```
    #[must_use]
    pub fn get() -> Self {
        Self::new(|state: St| (state.clone(), state))
    }
}

impl<S> State<S, ()>
where
    S: 'static,
{
    /// Creates a State that replaces the current state with a new value.
    ///
    /// This is the fundamental "put" operation of `MonadState`.
    ///
    /// # Arguments
    ///
    /// * `new_state` - The new state value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::State;
    ///
    /// let state: State<i32, ()> = State::put(100);
    /// let (_, final_state) = state.run(42);
    /// assert_eq!(final_state, 100);
    /// ```
    pub fn put(new_state: S) -> Self
    where
        S: Clone,
    {
        Self::new(move |_| ((), new_state.clone()))
    }

    /// Creates a State that modifies the current state using a function.
    ///
    /// # Arguments
    ///
    /// * `modifier` - A function that transforms the state
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::State;
    ///
    /// let state: State<i32, ()> = State::modify(|x| x * 2);
    /// let (_, final_state) = state.run(21);
    /// assert_eq!(final_state, 42);
    /// ```
    pub fn modify<F>(modifier: F) -> Self
    where
        F: Fn(S) -> S + 'static,
    {
        Self::new(move |state| ((), modifier(state)))
    }
}

impl<S, A> State<S, A>
where
    S: 'static,
    A: 'static,
{
    /// Creates a State that projects a value from the current state.
    ///
    /// This is a convenience method that combines `get` with a projection.
    ///
    /// # Arguments
    ///
    /// * `projection` - A function that extracts a value from the state
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::State;
    ///
    /// #[derive(Clone)]
    /// struct Config { port: u16 }
    ///
    /// let state: State<Config, u16> = State::gets(|c: &Config| c.port);
    /// let config = Config { port: 8080 };
    /// let (result, _) = state.run(config);
    /// assert_eq!(result, 8080);
    /// ```
    pub fn gets<F>(projection: F) -> Self
    where
        F: Fn(&S) -> A + 'static,
    {
        Self::new(move |state| {
            let result = projection(&state);
            (result, state)
        })
    }
}

// =============================================================================
// Clone Implementation
// =============================================================================

impl<S, A> Clone for State<S, A>
where
    S: 'static,
    A: 'static,
{
    fn clone(&self) -> Self {
        Self {
            run_function: self.run_function.clone(),
        }
    }
}

// =============================================================================
// Display Implementation
// =============================================================================

impl<S, A> std::fmt::Display for State<S, A>
where
    S: 'static,
    A: 'static,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "<State>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[rstest]
    fn test_display_state() {
        let state: State<i32, i32> = State::new(|s: i32| (s * 2, s + 1));
        assert_eq!(format!("{state}"), "<State>");
    }

    // =========================================================================
    // Original Tests
    // =========================================================================

    #[rstest]
    fn state_new_and_run() {
        let state: State<i32, i32> = State::new(|s: i32| (s * 2, s + 1));
        let (result, final_state) = state.run(10);
        assert_eq!(result, 20);
        assert_eq!(final_state, 11);
    }

    #[rstest]
    fn state_pure_does_not_modify_state() {
        let state: State<i32, &str> = State::pure("constant");
        let (result, final_state) = state.run(42);
        assert_eq!(result, "constant");
        assert_eq!(final_state, 42);
    }

    #[rstest]
    fn state_get_returns_current_state() {
        let state: State<i32, i32> = State::get();
        let (result, final_state) = state.run(42);
        assert_eq!(result, 42);
        assert_eq!(final_state, 42);
    }

    #[rstest]
    fn state_put_replaces_state() {
        let state: State<i32, ()> = State::put(100);
        let ((), final_state) = state.run(42);
        assert_eq!(final_state, 100);
    }

    #[rstest]
    fn state_modify_transforms_state() {
        let state: State<i32, ()> = State::modify(|x| x * 2);
        let ((), final_state) = state.run(21);
        assert_eq!(final_state, 42);
    }

    #[rstest]
    fn state_fmap_transforms_result() {
        let state: State<i32, i32> = State::new(|s: i32| (s, s));
        let mapped = state.fmap(|value| value * 2);
        let (result, final_state) = mapped.run(21);
        assert_eq!(result, 42);
        assert_eq!(final_state, 21);
    }

    #[rstest]
    fn state_flat_map_chains_states() {
        let state: State<i32, i32> = State::new(|s: i32| (s, s + 1));
        let chained = state.flat_map(|value| State::new(move |s: i32| (value + s, s)));
        let (result, final_state) = chained.run(10);
        assert_eq!(result, 21); // 10 + 11
        assert_eq!(final_state, 11);
    }

    #[rstest]
    fn state_map2_combines_states() {
        let state1: State<i32, i32> = State::new(|s: i32| (s, s + 1));
        let state2: State<i32, i32> = State::new(|s: i32| (s * 2, s + 1));
        let combined = state1.map2(state2, |a, b| a + b);
        let (result, final_state) = combined.run(10);
        assert_eq!(result, 32); // 10 + 22
        assert_eq!(final_state, 12);
    }

    #[rstest]
    fn state_clone_works() {
        let state: State<i32, i32> = State::new(|s: i32| (s * 2, s + 1));
        let cloned = state.clone();
        let (r1, f1) = state.run(10);
        let (r2, f2) = cloned.run(10);
        assert_eq!(r1, r2);
        assert_eq!(f1, f2);
    }
}
