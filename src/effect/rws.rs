//! RWS Monad - Reader + Writer + State combined monad.
//!
//! The RWS monad combines three effects into a single monad:
//! - **Reader**: Read-only access to an environment
//! - **Writer**: Accumulated output (logs, metrics, etc.)
//! - **State**: Mutable state threading
//!
//! This is equivalent to stacking `ReaderT<R, WriterT<W, State<S, A>>>` but
//! provides a more efficient and ergonomic implementation.
//!
//! # Overview
//!
//! An `RWS<R, W, S, A>` encapsulates a function `(R, S) -> (A, S, W)`:
//! - Takes an environment `R` and initial state `S`
//! - Produces a result `A`, new state `S`, and accumulated output `W`
//!
//! # Note on Type Classes
//!
//! RWS provides its own `fmap`, `flat_map`, `map2`, etc. methods directly
//! on the type, rather than implementing the Functor/Applicative/Monad traits.
//! This follows the same pattern as Reader, Writer, and State in this library.
//!
//! # Laws
//!
//! RWS satisfies all Functor, Applicative, and Monad laws, plus the laws of
//! `MonadReader`, `MonadWriter`, and `MonadState`.
//!
//! ## Functor Laws
//!
//! - Identity: `rws.fmap(|x| x) == rws`
//! - Composition: `rws.fmap(f).fmap(g) == rws.fmap(|x| g(f(x)))`
//!
//! ## Monad Laws
//!
//! - Left Identity: `RWS::pure(a).flat_map(f) == f(a)`
//! - Right Identity: `m.flat_map(RWS::pure) == m`
//! - Associativity: `m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))`
//!
//! ## `MonadReader` Laws
//!
//! - Ask Local Identity: `RWS::local(|r| r, m) == m`
//! - Ask Local Composition: `RWS::local(f, RWS::local(g, m)) == RWS::local(|r| g(f(r)), m)`
//!
//! ## `MonadWriter` Laws
//!
//! - Tell Monoid Law: `tell(w1).then(tell(w2)) == tell(w1.combine(w2))`
//!
//! ## `MonadState` Laws
//!
//! - Get Put Law: `get().flat_map(|s| put(s)) == pure(())`
//! - Put Get Law: `put(s).then(get())` returns `s`
//! - Put Put Law: `put(s1).then(put(s2)) == put(s2)`
//!
//! # Examples
//!
//! ```rust
//! use lambars::effect::RWS;
//!
//! // A computation that reads config, updates state, and logs
//! #[derive(Clone)]
//! struct Config { multiplier: i32 }
//!
//! let computation: RWS<Config, Vec<String>, i32, i32> = RWS::ask()
//!     .flat_map(|config: Config| RWS::get().flat_map(move |state| {
//!         let result = state * config.multiplier;
//!         RWS::put(state + 1)
//!             .then(RWS::tell(vec![format!("result: {}", result)]))
//!             .then(RWS::pure(result))
//!     }));
//!
//! let (result, final_state, logs) = computation.run(Config { multiplier: 3 }, 10);
//! assert_eq!(result, 30);
//! assert_eq!(final_state, 11);
//! assert_eq!(logs, vec!["result: 30"]);
//! ```

#![forbid(unsafe_code)]

use std::rc::Rc;

use crate::typeclass::Monoid;

/// A monad combining Reader, Writer, and State effects.
///
/// `RWS<R, W, S, A>` represents a computation that:
/// - Reads from an environment of type `R`
/// - Accumulates output of type `W` (must be a `Monoid`)
/// - Threads state of type `S`
/// - Produces a result of type `A`
///
/// # Type Parameters
///
/// - `R`: Environment type (read-only)
/// - `W`: Output type (must implement `Monoid`)
/// - `S`: State type
/// - `A`: Result type
///
/// # Examples
///
/// ```rust
/// use lambars::effect::RWS;
///
/// let rws: RWS<i32, Vec<String>, i32, i32> = RWS::new(|environment, state| {
///     let result = environment + state;
///     let new_state = state + 1;
///     let output = vec![format!("computed: {}", result)];
///     (result, new_state, output)
/// });
///
/// let (result, final_state, output) = rws.run(10, 5);
/// assert_eq!(result, 15);
/// assert_eq!(final_state, 6);
/// assert_eq!(output, vec!["computed: 15"]);
/// ```
pub struct RWS<R, W, S, A>
where
    R: 'static,
    W: Monoid + 'static,
    S: 'static,
    A: 'static,
{
    run_function: Rc<dyn Fn(R, S) -> (A, S, W)>,
}

// --- Basic Constructors and Executors ---

impl<R, W, S, A> RWS<R, W, S, A>
where
    R: 'static,
    W: Monoid + 'static,
    S: 'static,
    A: 'static,
{
    /// Creates a new RWS from a function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws: RWS<i32, String, i32, i32> = RWS::new(|environment, state| {
    ///     (environment + state, state + 1, format!("sum: {}", environment + state))
    /// });
    /// ```
    pub fn new<F>(function: F) -> Self
    where
        F: Fn(R, S) -> (A, S, W) + 'static,
    {
        Self {
            run_function: Rc::new(function),
        }
    }

    /// Creates an RWS that returns a constant value without modifying
    /// state or producing output.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws: RWS<i32, Vec<String>, i32, i32> = RWS::pure(42);
    /// let (result, state, output) = rws.run(0, 0);
    /// assert_eq!(result, 42);
    /// assert_eq!(state, 0);
    /// assert!(output.is_empty());
    /// ```
    pub fn pure(value: A) -> Self
    where
        A: Clone,
    {
        Self::new(move |_, state| (value.clone(), state, W::empty()))
    }

    /// Runs the RWS computation with the given environment and initial state.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws: RWS<i32, String, i32, i32> = RWS::new(|env, state| {
    ///     (env + state, state * 2, format!("env={}, state={}", env, state))
    /// });
    /// let (result, final_state, output) = rws.run(10, 5);
    /// assert_eq!(result, 15);
    /// assert_eq!(final_state, 10);
    /// assert_eq!(output, "env=10, state=5");
    /// ```
    pub fn run(&self, environment: R, initial_state: S) -> (A, S, W) {
        (self.run_function)(environment, initial_state)
    }

    /// Runs the RWS computation and returns only the result and output.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws: RWS<i32, String, i32, i32> = RWS::pure(42);
    /// let (result, output) = rws.eval(0, 0);
    /// assert_eq!(result, 42);
    /// assert!(output.is_empty());
    /// ```
    pub fn eval(&self, environment: R, initial_state: S) -> (A, W) {
        let (result, _, output) = self.run(environment, initial_state);
        (result, output)
    }

    /// Runs the RWS computation and returns only the final state and output.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws: RWS<i32, String, i32, ()> = RWS::new(|_, state| {
    ///     ((), state + 1, "incremented".to_string())
    /// });
    /// let (final_state, output) = rws.exec(0, 10);
    /// assert_eq!(final_state, 11);
    /// assert_eq!(output, "incremented");
    /// ```
    pub fn exec(&self, environment: R, initial_state: S) -> (S, W) {
        let (_, final_state, output) = self.run(environment, initial_state);
        (final_state, output)
    }
}

// --- Functor/Monad Operations ---

impl<R, W, S, A> RWS<R, W, S, A>
where
    R: 'static,
    W: Monoid + 'static,
    S: 'static,
    A: 'static,
{
    /// Maps a function over the result of this RWS (Functor operation).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws: RWS<i32, String, i32, i32> = RWS::pure(21);
    /// let mapped = rws.fmap(|x| x * 2);
    /// let (result, _, _) = mapped.run(0, 0);
    /// assert_eq!(result, 42);
    /// ```
    pub fn fmap<B, F>(self, function: F) -> RWS<R, W, S, B>
    where
        F: Fn(A) -> B + 'static,
        B: 'static,
    {
        let original_function = self.run_function;
        RWS::new(move |environment, state| {
            let (result, new_state, output) = (original_function)(environment, state);
            (function(result), new_state, output)
        })
    }

    /// Chains this RWS with a function that produces another RWS (Monad operation).
    ///
    /// The outputs are combined using the `Monoid::combine` operation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws1: RWS<i32, Vec<String>, i32, i32> = RWS::new(|env, state| {
    ///     (env, state, vec!["first".to_string()])
    /// });
    /// let rws2 = rws1.flat_map(|x| RWS::new(move |_, state| {
    ///     (x + state, state + 1, vec!["second".to_string()])
    /// }));
    /// let (result, final_state, output) = rws2.run(10, 5);
    /// assert_eq!(result, 15);
    /// assert_eq!(final_state, 6);
    /// assert_eq!(output, vec!["first", "second"]);
    /// ```
    pub fn flat_map<B, F>(self, function: F) -> RWS<R, W, S, B>
    where
        F: Fn(A) -> RWS<R, W, S, B> + 'static,
        B: 'static,
        R: Clone,
    {
        let original_function = self.run_function;
        RWS::new(move |environment: R, state: S| {
            let (result_a, intermediate_state, output_a) =
                (original_function)(environment.clone(), state);
            let next_rws = function(result_a);
            let (result_b, final_state, output_b) = next_rws.run(environment, intermediate_state);
            (result_b, final_state, output_a.combine(output_b))
        })
    }

    /// Alias for `flat_map`.
    pub fn and_then<B, F>(self, function: F) -> RWS<R, W, S, B>
    where
        F: Fn(A) -> RWS<R, W, S, B> + 'static,
        B: 'static,
        R: Clone,
    {
        self.flat_map(function)
    }

    /// Sequences two RWS computations, discarding the first result.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let log1: RWS<(), Vec<String>, (), ()> = RWS::new(|_, _| {
    ///     ((), (), vec!["step 1".to_string()])
    /// });
    /// let log2: RWS<(), Vec<String>, (), i32> = RWS::new(|_, _| {
    ///     (42, (), vec!["step 2".to_string()])
    /// });
    /// let combined = log1.then(log2);
    /// let (result, _, output) = combined.run((), ());
    /// assert_eq!(result, 42);
    /// assert_eq!(output, vec!["step 1", "step 2"]);
    /// ```
    #[must_use]
    pub fn then<B>(self, next: RWS<R, W, S, B>) -> RWS<R, W, S, B>
    where
        B: 'static,
        R: Clone,
    {
        self.flat_map(move |_| next.clone())
    }

    /// Combines two RWS computations using a binary function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws1: RWS<i32, String, i32, i32> = RWS::pure(10);
    /// let rws2: RWS<i32, String, i32, i32> = RWS::pure(20);
    /// let combined = rws1.map2(rws2, |a, b| a + b);
    /// let (result, _, _) = combined.run(0, 0);
    /// assert_eq!(result, 30);
    /// ```
    pub fn map2<B, C, F>(self, other: RWS<R, W, S, B>, function: F) -> RWS<R, W, S, C>
    where
        F: Fn(A, B) -> C + 'static,
        B: 'static,
        C: 'static,
        R: Clone,
    {
        let self_function = self.run_function;
        let other_function = other.run_function;
        RWS::new(move |environment: R, state: S| {
            let (result_a, intermediate_state, output_a) =
                (self_function)(environment.clone(), state);
            let (result_b, final_state, output_b) =
                (other_function)(environment, intermediate_state);
            (
                function(result_a, result_b),
                final_state,
                output_a.combine(output_b),
            )
        })
    }

    /// Combines two RWS computations into a tuple.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws1: RWS<i32, String, i32, i32> = RWS::pure(42);
    /// let rws2: RWS<i32, String, i32, &str> = RWS::pure("hello");
    /// let product = rws1.product(rws2);
    /// let ((first, second), _, _) = product.run(0, 0);
    /// assert_eq!(first, 42);
    /// assert_eq!(second, "hello");
    /// ```
    #[must_use]
    pub fn product<B>(self, other: RWS<R, W, S, B>) -> RWS<R, W, S, (A, B)>
    where
        B: 'static,
        R: Clone,
    {
        self.map2(other, |a, b| (a, b))
    }

    /// Applies a function inside an RWS to a value inside another RWS.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let function_rws: RWS<i32, String, i32, fn(i32) -> i32> = RWS::pure(|x| x * 2);
    /// let value_rws: RWS<i32, String, i32, i32> = RWS::pure(21);
    /// let applied = function_rws.apply(value_rws);
    /// let (result, _, _) = applied.run(0, 0);
    /// assert_eq!(result, 42);
    /// ```
    #[must_use]
    pub fn apply<B, Output>(self, other: RWS<R, W, S, B>) -> RWS<R, W, S, Output>
    where
        A: Fn(B) -> Output + 'static,
        B: 'static,
        Output: 'static,
        R: Clone,
    {
        self.map2(other, |function, b| function(b))
    }
}

// --- MonadReader Operations ---

#[allow(clippy::mismatching_type_param_order)]
impl<R, W, S> RWS<R, W, S, R>
where
    R: Clone + 'static,
    W: Monoid + 'static,
    S: 'static,
{
    /// Creates an RWS that returns the entire environment.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws: RWS<i32, String, (), i32> = RWS::ask();
    /// let (result, _, _) = rws.run(42, ());
    /// assert_eq!(result, 42);
    /// ```
    #[must_use]
    pub fn ask() -> Self {
        Self::new(|environment, state| (environment, state, W::empty()))
    }
}

impl<R, W, S, A> RWS<R, W, S, A>
where
    R: 'static,
    W: Monoid + 'static,
    S: 'static,
    A: 'static,
{
    /// Creates an RWS that projects a value from the environment.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// #[derive(Clone)]
    /// struct Config { port: u16 }
    ///
    /// let rws: RWS<Config, String, (), u16> = RWS::asks(|c: Config| c.port);
    /// let (result, _, _) = rws.run(Config { port: 8080 }, ());
    /// assert_eq!(result, 8080);
    /// ```
    pub fn asks<F>(projection: F) -> Self
    where
        F: Fn(R) -> A + 'static,
    {
        Self::new(move |environment, state| (projection(environment), state, W::empty()))
    }

    /// Runs a computation with a modified environment.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws: RWS<i32, String, (), i32> = RWS::ask();
    /// let modified = RWS::local(|env| env * 2, rws);
    /// let (result, _, _) = modified.run(21, ());
    /// assert_eq!(result, 42);
    /// ```
    pub fn local<F>(modifier: F, computation: Self) -> Self
    where
        F: Fn(R) -> R + 'static,
    {
        let computation_function = computation.run_function;
        Self::new(move |environment, state| {
            let modified_environment = modifier(environment);
            (computation_function)(modified_environment, state)
        })
    }
}

// --- MonadWriter Operations ---

impl<R, W, S> RWS<R, W, S, ()>
where
    R: 'static,
    W: Monoid + Clone + 'static,
    S: 'static,
{
    /// Creates an RWS that appends output without producing a meaningful result.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws: RWS<(), Vec<String>, (), ()> = RWS::tell(vec!["log message".to_string()]);
    /// let (_, _, output) = rws.run((), ());
    /// assert_eq!(output, vec!["log message"]);
    /// ```
    pub fn tell(output: W) -> Self {
        Self::new(move |_, state| ((), state, output.clone()))
    }
}

impl<R, W, S, A> RWS<R, W, S, A>
where
    R: 'static,
    W: Monoid + Clone + 'static,
    S: 'static,
    A: 'static,
{
    /// Executes a computation and also returns its output.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws: RWS<(), Vec<String>, (), i32> = RWS::new(|_, _| {
    ///     (42, (), vec!["computed".to_string()])
    /// });
    /// let listened = RWS::listen(rws);
    /// let ((result, captured_output), _, total_output) = listened.run((), ());
    /// assert_eq!(result, 42);
    /// assert_eq!(captured_output, vec!["computed"]);
    /// assert_eq!(total_output, vec!["computed"]);
    /// ```
    #[must_use]
    pub fn listen(computation: Self) -> RWS<R, W, S, (A, W)> {
        let computation_function = computation.run_function;
        RWS::new(move |environment, state| {
            let (result, new_state, output) = (computation_function)(environment, state);
            ((result, output.clone()), new_state, output)
        })
    }

    /// Executes a computation and projects a value from its output.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws: RWS<(), Vec<String>, (), i32> = RWS::new(|_, _| {
    ///     (42, (), vec!["a".to_string(), "b".to_string()])
    /// });
    /// let listened = RWS::listens(|output: &Vec<String>| output.len(), rws);
    /// let ((result, count), _, output) = listened.run((), ());
    /// assert_eq!(result, 42);
    /// assert_eq!(count, 2);
    /// assert_eq!(output, vec!["a", "b"]);
    /// ```
    pub fn listens<B, F>(projection: F, computation: Self) -> RWS<R, W, S, (A, B)>
    where
        F: Fn(&W) -> B + 'static,
        B: 'static,
    {
        let computation_function = computation.run_function;
        RWS::new(move |environment, state| {
            let (result, new_state, output) = (computation_function)(environment, state);
            let projected = projection(&output);
            ((result, projected), new_state, output)
        })
    }
}

impl<R, W, S, A> RWS<R, W, S, A>
where
    R: 'static,
    W: Monoid + 'static,
    S: 'static,
    A: 'static,
{
    /// Executes a computation with a function that modifies the output.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws: RWS<(), Vec<String>, (), (i32, fn(Vec<String>) -> Vec<String>)> =
    ///     RWS::new(|_, _| {
    ///         let modifier: fn(Vec<String>) -> Vec<String> = |output| {
    ///             output.into_iter().map(|s| s.to_uppercase()).collect()
    ///         };
    ///         ((42, modifier), (), vec!["hello".to_string()])
    ///     });
    /// let passed = RWS::pass(rws);
    /// let (result, _, output) = passed.run((), ());
    /// assert_eq!(result, 42);
    /// assert_eq!(output, vec!["HELLO"]);
    /// ```
    #[must_use]
    pub fn pass<F>(computation: RWS<R, W, S, (A, F)>) -> Self
    where
        F: Fn(W) -> W + 'static,
    {
        let computation_function = computation.run_function;
        Self::new(move |environment, state| {
            let ((result, modifier), new_state, output) =
                (computation_function)(environment, state);
            (result, new_state, modifier(output))
        })
    }

    /// Modifies the output of a computation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws: RWS<(), Vec<String>, (), i32> = RWS::new(|_, _| {
    ///     (42, (), vec!["hello".to_string()])
    /// });
    /// let censored = RWS::censor(
    ///     |output: Vec<String>| output.into_iter().map(|s| s.to_uppercase()).collect(),
    ///     rws
    /// );
    /// let (result, _, output) = censored.run((), ());
    /// assert_eq!(result, 42);
    /// assert_eq!(output, vec!["HELLO"]);
    /// ```
    pub fn censor<F>(modifier: F, computation: Self) -> Self
    where
        F: Fn(W) -> W + 'static,
    {
        let computation_function = computation.run_function;
        Self::new(move |environment, state| {
            let (result, new_state, output) = (computation_function)(environment, state);
            (result, new_state, modifier(output))
        })
    }
}

// --- MonadState Operations ---

#[allow(clippy::mismatching_type_param_order)]
impl<R, W, S> RWS<R, W, S, S>
where
    R: 'static,
    W: Monoid + 'static,
    S: Clone + 'static,
{
    /// Creates an RWS that returns the current state without modifying it.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws: RWS<(), String, i32, i32> = RWS::get();
    /// let (result, final_state, _) = rws.run((), 42);
    /// assert_eq!(result, 42);
    /// assert_eq!(final_state, 42);
    /// ```
    #[must_use]
    pub fn get() -> Self {
        Self::new(|_, state: S| (state.clone(), state, W::empty()))
    }
}

impl<R, W, S> RWS<R, W, S, ()>
where
    R: 'static,
    W: Monoid + 'static,
    S: Clone + 'static,
{
    /// Creates an RWS that replaces the current state with a new value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws: RWS<(), String, i32, ()> = RWS::put(100);
    /// let (_, final_state, _) = rws.run((), 42);
    /// assert_eq!(final_state, 100);
    /// ```
    pub fn put(new_state: S) -> Self {
        Self::new(move |_, _| ((), new_state.clone(), W::empty()))
    }
}

impl<R, W, S> RWS<R, W, S, ()>
where
    R: 'static,
    W: Monoid + 'static,
    S: 'static,
{
    /// Creates an RWS that modifies the current state using a function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws: RWS<(), String, i32, ()> = RWS::modify(|x| x * 2);
    /// let (_, final_state, _) = rws.run((), 21);
    /// assert_eq!(final_state, 42);
    /// ```
    pub fn modify<F>(modifier: F) -> Self
    where
        F: Fn(S) -> S + 'static,
    {
        Self::new(move |_, state| ((), modifier(state), W::empty()))
    }
}

impl<R, W, S, A> RWS<R, W, S, A>
where
    R: 'static,
    W: Monoid + 'static,
    S: 'static,
    A: 'static,
{
    /// Creates an RWS from a state transition function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws: RWS<(), String, i32, String> = RWS::state(|s| {
    ///     (format!("was: {}", s), s + 1)
    /// });
    /// let (result, final_state, _) = rws.run((), 41);
    /// assert_eq!(result, "was: 41");
    /// assert_eq!(final_state, 42);
    /// ```
    pub fn state<F>(transition: F) -> Self
    where
        F: Fn(S) -> (A, S) + 'static,
    {
        Self::new(move |_, current_state| {
            let (result, new_state) = transition(current_state);
            (result, new_state, W::empty())
        })
    }

    /// Creates an RWS that projects a value from the current state.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// #[derive(Clone)]
    /// struct AppState { counter: i32 }
    ///
    /// let rws: RWS<(), String, AppState, i32> = RWS::gets(|s: &AppState| s.counter);
    /// let (result, _, _) = rws.run((), AppState { counter: 42 });
    /// assert_eq!(result, 42);
    /// ```
    pub fn gets<F>(projection: F) -> Self
    where
        F: Fn(&S) -> A + 'static,
    {
        Self::new(move |_, state| {
            let result = projection(&state);
            (result, state, W::empty())
        })
    }
}

// --- Utility Methods ---

impl<R, W, S, A> RWS<R, W, S, A>
where
    R: 'static,
    W: Monoid + 'static,
    S: 'static,
    A: 'static,
{
    /// Transforms the result, state, and output of this RWS.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws: RWS<i32, String, i32, i32> = RWS::new(|env, state| {
    ///     (env + state, state, "log".to_string())
    /// });
    /// let mapped = rws.map_rws(|(result, state, output): (i32, i32, String)| {
    ///     (result * 2, state + 1, output.to_uppercase())
    /// });
    /// let (result, final_state, output) = mapped.run(10, 5);
    /// assert_eq!(result, 30);
    /// assert_eq!(final_state, 6);
    /// assert_eq!(output, "LOG");
    /// ```
    pub fn map_rws<B, W2, F>(self, function: F) -> RWS<R, W2, S, B>
    where
        F: Fn((A, S, W)) -> (B, S, W2) + 'static,
        W2: Monoid + 'static,
        B: 'static,
    {
        let original_function = self.run_function;
        RWS::new(move |environment, state| {
            let (result, new_state, output) = (original_function)(environment, state);
            function((result, new_state, output))
        })
    }

    /// Transforms the initial environment and state before running this RWS.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::RWS;
    ///
    /// let rws: RWS<i32, String, i32, i32> = RWS::ask();
    /// let with_transformed = rws.with_rws(|env: String, state| {
    ///     (env.len() as i32, state)
    /// });
    /// let (result, _, _) = with_transformed.run("hello".to_string(), 0);
    /// assert_eq!(result, 5);
    /// ```
    pub fn with_rws<R2, F>(self, function: F) -> RWS<R2, W, S, A>
    where
        F: Fn(R2, S) -> (R, S) + 'static,
        R2: 'static,
    {
        let original_function = self.run_function;
        RWS::new(move |environment, state| {
            let (transformed_environment, transformed_state) = function(environment, state);
            (original_function)(transformed_environment, transformed_state)
        })
    }
}

// --- Clone Implementation ---

impl<R, W, S, A> Clone for RWS<R, W, S, A>
where
    R: 'static,
    W: Monoid + 'static,
    S: 'static,
    A: 'static,
{
    fn clone(&self) -> Self {
        Self {
            run_function: self.run_function.clone(),
        }
    }
}

// --- Display Implementation ---

impl<R, W, S, A> std::fmt::Display for RWS<R, W, S, A>
where
    R: 'static,
    W: Monoid + 'static,
    S: 'static,
    A: 'static,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "<RWS>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn test_display_rws() {
        let rws: RWS<i32, String, i32, i32> = RWS::pure(42);
        assert_eq!(format!("{rws}"), "<RWS>");
    }

    #[rstest]
    fn test_clone_rws() {
        let rws: RWS<i32, String, i32, i32> = RWS::pure(42);
        let cloned = rws.clone();
        assert_eq!(rws.run(0, 0), cloned.run(0, 0));
    }
}
