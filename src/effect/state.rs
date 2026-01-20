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
//! # Pure/Deferred Pattern
//!
//! `State` uses a Pure/Deferred pattern for performance optimization:
//!
//! - **Pure**: Holds an immediate value without Rc allocation. Used by `pure()`.
//! - **Deferred**: Holds a state transition function wrapped in `Rc<dyn Fn>`.
//!   Used by `new()` and operations that depend on state.
//!
//! This pattern eliminates Rc allocation overhead for pure values while
//! maintaining full functionality for stateful computations.
//!
//! # Important: Pure Function Assumption
//!
//! Functions passed to `fmap`, `flat_map`, etc. are assumed to be **pure and total**:
//! - No side effects (I/O, global state, etc.)
//! - Always return a value (no panics, infinite loops)
//!
//! For Pure variants, `fmap`/`flat_map` evaluate immediately (eager evaluation).
//! This is semantically equivalent to lazy evaluation for pure functions.
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
/// # Variants
///
/// - `Pure`: Holds an immediate value. No Rc allocation overhead.
/// - `Deferred`: Holds a state transition function wrapped in `Rc<dyn Fn>`.
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
#[non_exhaustive]
pub enum State<S, A>
where
    S: 'static,
    A: 'static,
{
    /// Pure value that requires no state computation.
    /// This variant avoids Rc allocation for pure values.
    Pure {
        /// The pure value.
        value: A,
    },

    /// Deferred state transition function.
    /// Uses Rc to allow cloning of the State for `flat_map`.
    Deferred {
        /// The wrapped state transition function.
        run_function: Rc<dyn Fn(S) -> (A, S)>,
    },
}

impl<S, A> State<S, A>
where
    S: 'static,
    A: 'static,
{
    /// Creates a new State from a state transition function.
    ///
    /// This creates a `Deferred` variant.
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
        Self::Deferred {
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

    /// Runs the State computation with the given initial state, consuming `self`.
    ///
    /// This method consumes the `State` and does not require `A: Clone`.
    /// Use `run_cloned` if you need to run the computation multiple times.
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
    pub fn run(self, initial_state: S) -> (A, S) {
        match self {
            Self::Pure { value } => (value, initial_state),
            Self::Deferred { run_function } => run_function(initial_state),
        }
    }

    /// Runs the State computation with the given initial state by reference.
    ///
    /// This method borrows `self` and requires `A: Clone` for the `Pure` variant.
    /// Use this when you need to run the computation multiple times.
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
    /// let (result1, _) = state.run_cloned(10);
    /// let (result2, _) = state.run_cloned(20);
    /// assert_eq!(result1, 11);
    /// assert_eq!(result2, 21);
    /// ```
    pub fn run_cloned(&self, initial_state: S) -> (A, S)
    where
        A: Clone,
    {
        match self {
            Self::Pure { value } => (value.clone(), initial_state),
            Self::Deferred { run_function } => run_function(initial_state),
        }
    }

    /// Runs the State computation and returns only the result, consuming `self`.
    ///
    /// This method consumes the `State` and does not require `A: Clone`.
    /// Use `eval_cloned` if you need to evaluate the computation multiple times.
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
    pub fn eval(self, initial_state: S) -> A {
        let (result, _) = self.run(initial_state);
        result
    }

    /// Runs the State computation and returns only the result by reference.
    ///
    /// This method borrows `self` and requires `A: Clone` for the `Pure` variant.
    /// Use this when you need to evaluate the computation multiple times.
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
    /// assert_eq!(state.eval_cloned(10), 20);
    /// assert_eq!(state.eval_cloned(20), 40);
    /// ```
    pub fn eval_cloned(&self, initial_state: S) -> A
    where
        A: Clone,
    {
        let (result, _) = self.run_cloned(initial_state);
        result
    }

    /// Runs the State computation and returns only the final state, consuming `self`.
    ///
    /// This method consumes the `State` and does not require `A: Clone`.
    /// Use `exec_cloned` if you need to execute the computation multiple times.
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
    pub fn exec(self, initial_state: S) -> S {
        let (_, final_state) = self.run(initial_state);
        final_state
    }

    /// Runs the State computation and returns only the final state by reference.
    ///
    /// This method borrows `self` and requires `A: Clone` for the `Pure` variant.
    /// Use this when you need to execute the computation multiple times.
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
    /// assert_eq!(state.exec_cloned(10), 11);
    /// assert_eq!(state.exec_cloned(20), 21);
    /// ```
    pub fn exec_cloned(&self, initial_state: S) -> S
    where
        A: Clone,
    {
        let (_, final_state) = self.run_cloned(initial_state);
        final_state
    }

    /// Creates a State that returns a constant value without modifying the state.
    ///
    /// This creates a `Pure` variant, avoiding Rc allocation.
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
    pub const fn pure(value: A) -> Self {
        Self::Pure { value }
    }

    /// Maps a function over the result of this State.
    ///
    /// This is the Functor operation for State.
    ///
    /// For `Pure` variants, the function is applied immediately (eager evaluation).
    /// For `Deferred` variants, evaluation is deferred until `run()` is called.
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
        match self {
            Self::Pure { value } => State::Pure {
                value: function(value),
            },
            Self::Deferred { run_function } => State::Deferred {
                run_function: Rc::new(move |state| {
                    let (result, new_state) = run_function(state);
                    (function(result), new_state)
                }),
            },
        }
    }

    /// Chains this State with a function that produces another State.
    ///
    /// This is the Monad operation for State.
    ///
    /// For `Pure` variants, the function is applied immediately (eager evaluation).
    /// For `Deferred` variants, evaluation is deferred until `run()` is called.
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
        match self {
            Self::Pure { value } => function(value),
            Self::Deferred { run_function } => State::Deferred {
                run_function: Rc::new(move |state| {
                    let (result, intermediate_state) = run_function(state);
                    let next_state = function(result);
                    // Handle Pure/Deferred without requiring B: Clone
                    match next_state {
                        State::Pure { value } => (value, intermediate_state),
                        State::Deferred {
                            run_function: next_run,
                        } => next_run(intermediate_state),
                    }
                }),
            },
        }
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
    /// For `Pure` variants (no state change), returns next directly.
    /// For `Deferred` variants, chains the computations without `Rc::new(next)`.
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
        B: Clone + 'static,
    {
        match self {
            Self::Pure { .. } => next, // Pure has no state change, return next directly
            Self::Deferred { run_function } => {
                // Decompose next to avoid Rc::new(next)
                match next {
                    State::Pure { value } => State::Deferred {
                        run_function: Rc::new(move |state| {
                            let (_, intermediate_state) = run_function(state);
                            (value.clone(), intermediate_state)
                        }),
                    },
                    State::Deferred {
                        run_function: next_run,
                    } => State::Deferred {
                        run_function: Rc::new(move |state| {
                            let (_, intermediate_state) = run_function(state);
                            next_run(intermediate_state)
                        }),
                    },
                }
            }
        }
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
        A: Clone,
        B: Clone + 'static,
        C: 'static,
    {
        match (self, other) {
            (Self::Pure { value: a }, State::Pure { value: b }) => State::Pure {
                value: function(a, b),
            },
            (Self::Pure { value: a }, State::Deferred { run_function }) => State::Deferred {
                run_function: Rc::new(move |state| {
                    let (result_b, final_state) = run_function(state);
                    (function(a.clone(), result_b), final_state)
                }),
            },
            (Self::Deferred { run_function }, State::Pure { value: b }) => State::Deferred {
                run_function: Rc::new(move |state| {
                    let (result_a, final_state) = run_function(state);
                    (function(result_a, b.clone()), final_state)
                }),
            },
            (
                Self::Deferred {
                    run_function: self_function,
                },
                State::Deferred {
                    run_function: other_function,
                },
            ) => State::Deferred {
                run_function: Rc::new(move |state| {
                    let (result_a, intermediate_state) = self_function(state);
                    let (result_b, final_state) = other_function(intermediate_state);
                    (function(result_a, result_b), final_state)
                }),
            },
        }
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
        A: Clone,
        B: Clone + 'static,
    {
        self.map2(other, |a, b| (a, b))
    }
}

impl<St> State<St, St>
where
    St: Clone + 'static,
{
    /// Creates a State that returns the current state without modifying it.
    ///
    /// This is the fundamental "get" operation of `MonadState`.
    /// This creates a `Deferred` variant since it depends on the state.
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
        Self::Deferred {
            run_function: Rc::new(|state: St| (state.clone(), state)),
        }
    }
}

impl<S> State<S, ()>
where
    S: 'static,
{
    /// Creates a State that replaces the current state with a new value.
    ///
    /// This is the fundamental "put" operation of `MonadState`.
    /// This creates a `Deferred` variant since it modifies the state.
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
        Self::Deferred {
            run_function: Rc::new(move |_| ((), new_state.clone())),
        }
    }

    /// Creates a State that modifies the current state using a function.
    ///
    /// This creates a `Deferred` variant since it modifies the state.
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
        Self::Deferred {
            run_function: Rc::new(move |state| ((), modifier(state))),
        }
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
    /// This creates a `Deferred` variant since it depends on the state.
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
        Self::Deferred {
            run_function: Rc::new(move |state| {
                let result = projection(&state);
                (result, state)
            }),
        }
    }
}

impl<S, A> Clone for State<S, A>
where
    S: 'static,
    A: Clone + 'static,
{
    fn clone(&self) -> Self {
        match self {
            Self::Pure { value } => Self::Pure {
                value: value.clone(),
            },
            Self::Deferred { run_function } => Self::Deferred {
                run_function: run_function.clone(),
            },
        }
    }
}

impl<S, A> std::fmt::Display for State<S, A>
where
    S: 'static,
    A: 'static,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pure { .. } => write!(formatter, "<State::Pure>"),
            Self::Deferred { .. } => write!(formatter, "<State::Deferred>"),
        }
    }
}

impl<S: 'static, A: 'static> crate::typeclass::StateLike for State<S, A> {
    type StateType = S;
    type Value = A;

    fn into_state(self) -> Self
    where
        S: Clone + 'static,
        A: 'static,
    {
        self
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
    fn test_display_state_pure() {
        let state: State<i32, i32> = State::pure(42);
        assert_eq!(format!("{state}"), "<State::Pure>");
    }

    #[rstest]
    fn test_display_state_deferred() {
        let state: State<i32, i32> = State::new(|s: i32| (s * 2, s + 1));
        assert_eq!(format!("{state}"), "<State::Deferred>");
    }

    // =========================================================================
    // Pure/Deferred Pattern Tests
    // =========================================================================

    #[rstest]
    fn test_pure_creates_pure_variant() {
        let state: State<i32, i32> = State::pure(42);
        match state {
            State::Pure { value } => assert_eq!(value, 42),
            State::Deferred { .. } => panic!("Expected Pure variant"),
        }
    }

    #[rstest]
    fn test_new_creates_deferred_variant() {
        let state: State<i32, i32> = State::new(|s| (s, s));
        match state {
            State::Pure { .. } => panic!("Expected Deferred variant"),
            State::Deferred { .. } => {} // OK
        }
    }

    #[rstest]
    fn test_pure_run_returns_value_and_unchanged_state() {
        let state: State<i32, &str> = State::pure("constant");
        let (result, final_state) = state.run(42);
        assert_eq!(result, "constant");
        assert_eq!(final_state, 42);
    }

    #[rstest]
    fn test_fmap_on_pure_returns_pure() {
        let state: State<i32, i32> = State::pure(21);
        let mapped = state.fmap(|x| x * 2);
        match mapped {
            State::Pure { value } => assert_eq!(value, 42),
            State::Deferred { .. } => panic!("Expected Pure variant"),
        }
    }

    #[rstest]
    fn test_fmap_on_deferred_returns_deferred() {
        let state: State<i32, i32> = State::new(|s| (s, s));
        let mapped = state.fmap(|x| x * 2);
        match mapped {
            State::Pure { .. } => panic!("Expected Deferred variant"),
            State::Deferred { .. } => {} // OK
        }
    }

    #[rstest]
    fn test_flat_map_on_pure_returns_function_result() {
        let state: State<i32, i32> = State::pure(21);
        let chained = state.flat_map(|x| State::pure(x * 2));
        match chained {
            State::Pure { value } => assert_eq!(value, 42),
            State::Deferred { .. } => panic!("Expected Pure variant"),
        }
    }

    #[rstest]
    fn test_flat_map_on_pure_to_deferred() {
        let state: State<i32, i32> = State::pure(21);
        let chained = state.flat_map(|x| State::new(move |s| (x + s, s)));
        match chained {
            State::Pure { .. } => panic!("Expected Deferred variant"),
            State::Deferred { .. } => {} // OK
        }
        let (result, final_state) = chained.run(10);
        assert_eq!(result, 31);
        assert_eq!(final_state, 10);
    }

    #[rstest]
    fn test_then_pure_to_next() {
        let state1: State<i32, i32> = State::pure(1);
        let state2: State<i32, &str> = State::pure("result");
        let sequenced = state1.then(state2);
        // Pure.then(anything) should return next directly
        let (result, final_state) = sequenced.run(42);
        assert_eq!(result, "result");
        assert_eq!(final_state, 42);
    }

    #[rstest]
    fn test_then_deferred_to_pure() {
        let state1: State<i32, i32> = State::new(|s| (s, s + 10));
        let state2: State<i32, &str> = State::pure("result");
        let sequenced = state1.then(state2);
        let (result, final_state) = sequenced.run(42);
        assert_eq!(result, "result");
        assert_eq!(final_state, 52);
    }

    #[rstest]
    fn test_then_deferred_to_deferred() {
        let state1: State<i32, i32> = State::new(|s| (s, s + 10));
        let state2: State<i32, i32> = State::new(|s| (s * 2, s));
        let sequenced = state1.then(state2);
        let (result, final_state) = sequenced.run(5);
        // state1: (5, 15), state2 with 15: (30, 15)
        assert_eq!(result, 30);
        assert_eq!(final_state, 15);
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

    #[rstest]
    fn state_clone_pure_works() {
        let state: State<i32, i32> = State::pure(42);
        let cloned = state.clone();
        let (r1, f1) = state.run(10);
        let (r2, f2) = cloned.run(10);
        assert_eq!(r1, r2);
        assert_eq!(f1, f2);
    }

    // =========================================================================
    // Monad Laws Tests
    // =========================================================================

    mod monad_laws {
        use super::*;

        // Left Identity: pure(a).flat_map(f).run(s) == f(a).run(s)
        #[rstest]
        #[case(0, 42)]
        #[case(10, 100)]
        #[case(-5, 0)]
        fn left_identity_law(#[case] initial_state: i32, #[case] value: i32) {
            let f = |x: i32| State::new(move |s: i32| (x + s, s * 2));

            let lhs = State::pure(value).flat_map(f);
            let rhs = f(value);

            assert_eq!(lhs.run(initial_state), rhs.run(initial_state));
        }

        // Right Identity: m.flat_map(pure).run(s) == m.run(s)
        #[rstest]
        fn right_identity_law_pure() {
            let state: State<i32, i32> = State::pure(42);
            let lhs = state.clone().flat_map(State::pure);
            assert_eq!(lhs.run(10), state.run(10));
        }

        #[rstest]
        fn right_identity_law_deferred() {
            let state: State<i32, i32> = State::new(|s| (s * 2, s + 1));
            let lhs = state.clone().flat_map(State::pure);
            assert_eq!(lhs.run(10), state.run(10));
        }

        // Associativity: m.flat_map(f).flat_map(g).run(s) == m.flat_map(|x| f(x).flat_map(g)).run(s)
        #[rstest]
        fn associativity_law_pure() {
            let m: State<i32, i32> = State::pure(5);
            let f = |x: i32| State::new(move |s: i32| (x + s, s + 1));
            let g = |x: i32| State::new(move |s: i32| (x * s, s * 2));

            let lhs = m.clone().flat_map(f).flat_map(g);
            let rhs = m.flat_map(move |x| f(x).flat_map(g));

            assert_eq!(lhs.run(10), rhs.run(10));
        }

        #[rstest]
        fn associativity_law_deferred() {
            let m: State<i32, i32> = State::new(|s| (s, s + 1));
            let f = |x: i32| State::new(move |s: i32| (x + s, s + 1));
            let g = |x: i32| State::new(move |s: i32| (x * s, s * 2));

            let lhs = m.clone().flat_map(f).flat_map(g);
            let rhs = m.flat_map(move |x| f(x).flat_map(g));

            assert_eq!(lhs.run(10), rhs.run(10));
        }
    }

    // =========================================================================
    // Functor Laws Tests
    // =========================================================================

    mod functor_laws {
        use super::*;

        // Identity: state.fmap(|x| x).run(s) == state.run(s)
        #[rstest]
        fn identity_law_pure() {
            let state: State<i32, i32> = State::pure(42);
            let mapped = state.clone().fmap(|x| x);
            assert_eq!(mapped.run(10), state.run(10));
        }

        #[rstest]
        fn identity_law_deferred() {
            let state: State<i32, i32> = State::new(|s| (s * 2, s + 1));
            let mapped = state.clone().fmap(|x| x);
            assert_eq!(mapped.run(10), state.run(10));
        }

        // Composition: state.fmap(f).fmap(g).run(s) == state.fmap(|x| g(f(x))).run(s)
        #[rstest]
        fn composition_law_pure() {
            let state: State<i32, i32> = State::pure(5);
            let f = |x: i32| x + 1;
            let g = |x: i32| x * 2;

            let lhs = state.clone().fmap(f).fmap(g);
            let rhs = state.fmap(move |x| g(f(x)));

            assert_eq!(lhs.run(10), rhs.run(10));
        }

        #[rstest]
        fn composition_law_deferred() {
            let state: State<i32, i32> = State::new(|s| (s, s + 1));
            let f = |x: i32| x + 1;
            let g = |x: i32| x * 2;

            let lhs = state.clone().fmap(f).fmap(g);
            let rhs = state.fmap(move |x| g(f(x)));

            assert_eq!(lhs.run(10), rhs.run(10));
        }
    }

    // =========================================================================
    // MonadState Laws Tests
    // =========================================================================

    mod monad_state_laws {
        use super::*;

        // Get Put: get().flat_map(|s| put(s)).run(s) == pure(()).run(s)
        #[rstest]
        #[case(0)]
        #[case(42)]
        #[case(-100)]
        fn get_put_law(#[case] initial_state: i32) {
            let lhs: State<i32, ()> = State::get().flat_map(State::put);
            let rhs: State<i32, ()> = State::pure(());
            assert_eq!(lhs.run(initial_state), rhs.run(initial_state));
        }

        // Put Get: put(s).then(get()).run(_) returns s
        #[rstest]
        #[case(0)]
        #[case(42)]
        #[case(-100)]
        fn put_get_law(#[case] new_state: i32) {
            let computation: State<i32, i32> = State::put(new_state).then(State::get());
            let (result, final_state) = computation.run(999);
            assert_eq!(result, new_state);
            assert_eq!(final_state, new_state);
        }

        // Put Put: put(s1).then(put(s2)).run(_) == put(s2).run(_)
        #[rstest]
        fn put_put_law() {
            let lhs: State<i32, ()> = State::put(10).then(State::put(20));
            let rhs: State<i32, ()> = State::put(20);
            assert_eq!(lhs.run(0), rhs.run(0));
        }

        // Modify Composition: modify(f).then(modify(g)).run(s) == modify(|s| g(f(s))).run(s)
        #[rstest]
        fn modify_composition_law() {
            let f = |x: i32| x + 1;
            let g = |x: i32| x * 2;

            let lhs: State<i32, ()> = State::modify(f).then(State::modify(g));
            let rhs: State<i32, ()> = State::modify(move |x| g(f(x)));

            assert_eq!(lhs.run(10), rhs.run(10));
        }
    }

    // =========================================================================
    // Mixed Pure/Deferred Tests
    // =========================================================================

    mod mixed_pure_deferred {
        use super::*;

        #[rstest]
        fn mixed_chain_pure_deferred_pure() {
            let chain = State::pure(10)
                .flat_map(|x| State::new(move |s: i32| (x + s, s + 1)))
                .fmap(|x| x * 2);

            let (result, final_state) = chain.run(5);
            // pure(10) -> Deferred(|s| (10 + s, s + 1)) with s=5 -> (15, 6)
            // fmap(|x| x * 2) -> (30, 6)
            assert_eq!(result, 30);
            assert_eq!(final_state, 6);
        }

        #[rstest]
        fn mixed_map2_pure_pure() {
            let s1: State<i32, i32> = State::pure(10);
            let s2: State<i32, i32> = State::pure(20);
            let combined = s1.map2(s2, |a, b| a + b);

            match combined {
                State::Pure { value } => assert_eq!(value, 30),
                State::Deferred { .. } => panic!("Expected Pure variant"),
            }
        }

        #[rstest]
        fn mixed_map2_pure_deferred() {
            let s1: State<i32, i32> = State::pure(10);
            let s2: State<i32, i32> = State::new(|s| (s * 2, s + 1));
            let combined = s1.map2(s2, |a, b| a + b);

            let (result, final_state) = combined.run(5);
            assert_eq!(result, 20); // 10 + (5 * 2)
            assert_eq!(final_state, 6);
        }

        #[rstest]
        fn counter_example() {
            fn increment() -> State<i32, ()> {
                State::modify(|count| count + 1)
            }

            fn get_count() -> State<i32, i32> {
                State::get()
            }

            let computation = increment()
                .then(increment())
                .then(increment())
                .then(get_count());

            let (count, _) = computation.run(0);
            assert_eq!(count, 3);
        }
    }
}
