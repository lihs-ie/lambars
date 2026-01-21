//! Reader Monad - environment reading computation.
//!
//! The Reader monad represents computations that depend on an environment.
//! It is useful for dependency injection, configuration access, and other
//! patterns where computations need read-only access to some shared context.
//!
//! # Overview
//!
//! A `Reader<R, A>` encapsulates a function `R -> A`, where `R` is the
//! environment type and `A` is the result type. The key insight is that
//! by wrapping this function in a monad, we can compose multiple such
//! computations while implicitly threading the environment through all of them.
//!
//! # Pure/Deferred Pattern
//!
//! `Reader` uses a Pure/Deferred pattern for performance optimization:
//!
//! - **Pure**: Holds an immediate value without Rc allocation. Used by `pure()`.
//! - **Deferred**: Holds an environment reading function wrapped in `Rc<dyn Fn>`.
//!   Used by `new()` and operations that depend on the environment.
//!
//! This pattern eliminates Rc allocation overhead for pure values while
//! maintaining full functionality for environment-dependent computations.
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
//! Reader provides its own `fmap`, `flat_map`, `map2`, etc. methods directly
//! on the type, rather than implementing the Functor/Applicative/Monad traits.
//! This is because Rust's type system requires 'static bounds on trait
//! implementations when using `Rc<dyn Fn>`, and the standard type class traits
//! don't have these bounds. The methods work identically to their type class
//! counterparts.
//!
//! # Laws
//!
//! Reader satisfies all Functor, Applicative, and Monad laws, plus the
//! MonadReader-specific laws:
//!
//! ## Functor Laws
//!
//! - Identity: `reader.fmap(|x| x) == reader`
//! - Composition: `reader.fmap(f).fmap(g) == reader.fmap(|x| g(f(x)))`
//!
//! ## Monad Laws
//!
//! - Left Identity: `Reader::pure(a).flat_map(f) == f(a)`
//! - Right Identity: `m.flat_map(Reader::pure) == m`
//! - Associativity: `m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))`
//!
//! ## `MonadReader` Laws
//!
//! - Ask Local Identity: `Reader::local(|r| r, m) == m`
//! - Ask Local Composition: `Reader::local(f, Reader::local(g, m)) == Reader::local(|r| g(f(r)), m)`
//! - Ask Retrieval: `Reader::ask().run(r) == r`
//!
//! # Examples
//!
//! Basic usage:
//!
//! ```rust
//! use lambars::effect::Reader;
//!
//! // Create a reader that doubles the environment
//! let reader: Reader<i32, i32> = Reader::new(|environment| environment * 2);
//! assert_eq!(reader.run(21), 42);
//!
//! // Transform the result
//! let string_reader = Reader::new(|environment: i32| environment)
//!     .fmap(|value| value.to_string());
//! assert_eq!(string_reader.run(42), "42");
//! ```
//!
//! Dependency injection pattern:
//!
//! ```rust
//! use lambars::effect::Reader;
//!
//! #[derive(Clone)]
//! struct Config {
//!     port: u16,
//!     host: String,
//! }
//!
//! fn get_port() -> Reader<Config, u16> {
//!     Reader::asks(|config: Config| config.port)
//! }
//!
//! fn get_host() -> Reader<Config, String> {
//!     Reader::asks(|config: Config| config.host)
//! }
//!
//! fn get_address() -> Reader<Config, String> {
//!     get_host().map2(get_port(), |host, port| {
//!         format!("{}:{}", host, port)
//!     })
//! }
//!
//! let config = Config {
//!     port: 8080,
//!     host: "localhost".to_string(),
//! };
//!
//! assert_eq!(get_address().run(config), "localhost:8080");
//! ```

#![forbid(unsafe_code)]

use std::rc::Rc;

/// A monad for computations that read from an environment.
///
/// `Reader<R, A>` represents a computation that, given an environment of type `R`,
/// produces a value of type `A`. The environment is immutable and shared across
/// all composed computations.
///
/// # Type Parameters
///
/// - `R`: The environment type (read-only context)
/// - `A`: The result type
///
/// # Variants
///
/// - `Pure`: Holds an immediate value. No Rc allocation overhead.
/// - `Deferred`: Holds an environment reading function wrapped in `Rc<dyn Fn>`.
///
/// # Examples
///
/// ```rust
/// use lambars::effect::Reader;
///
/// let computation: Reader<i32, i32> = Reader::ask()
///     .flat_map(|environment| Reader::pure(environment * 2));
///
/// assert_eq!(computation.run(21), 42);
/// ```
#[non_exhaustive]
pub enum Reader<R, A>
where
    R: 'static,
    A: 'static,
{
    /// Pure value that requires no environment access.
    /// This variant avoids Rc allocation for pure values.
    Pure {
        /// The pure value.
        value: A,
    },

    /// Deferred environment reading function.
    /// Uses Rc to allow cloning of the Reader for `flat_map`.
    Deferred {
        /// The wrapped function from environment to result.
        run_function: Rc<dyn Fn(R) -> A>,
    },
}

impl<R, A> Reader<R, A>
where
    R: 'static,
    A: 'static,
{
    /// Creates a new Reader from a function.
    ///
    /// This creates a `Deferred` variant.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes an environment and produces a result
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::Reader;
    ///
    /// let reader: Reader<i32, i32> = Reader::new(|environment| environment * 2);
    /// assert_eq!(reader.run(21), 42);
    /// ```
    pub fn new<F>(function: F) -> Self
    where
        F: Fn(R) -> A + 'static,
    {
        Self::Deferred {
            run_function: Rc::new(function),
        }
    }

    /// Runs the Reader computation with the given environment, consuming `self`.
    ///
    /// This method consumes the `Reader` and does not require `A: Clone`.
    /// Use `run_cloned` if you need to run the computation multiple times.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment to run the computation with
    ///
    /// # Returns
    ///
    /// The result of running the computation with the environment.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::Reader;
    ///
    /// let reader: Reader<i32, i32> = Reader::new(|environment| environment + 1);
    /// assert_eq!(reader.run(41), 42);
    /// ```
    pub fn run(self, environment: R) -> A {
        match self {
            Self::Pure { value } => value,
            Self::Deferred { run_function } => run_function(environment),
        }
    }

    /// Runs the Reader computation with the given environment by reference.
    ///
    /// This method borrows `self` and requires `A: Clone` for the `Pure` variant.
    /// Use this when you need to run the computation multiple times.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment to run the computation with
    ///
    /// # Returns
    ///
    /// The result of running the computation with the environment.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::Reader;
    ///
    /// let reader: Reader<i32, i32> = Reader::new(|environment| environment + 1);
    /// assert_eq!(reader.run_cloned(41), 42);
    /// // Reader can be run multiple times
    /// assert_eq!(reader.run_cloned(0), 1);
    /// ```
    pub fn run_cloned(&self, environment: R) -> A
    where
        A: Clone,
    {
        match self {
            Self::Pure { value } => value.clone(),
            Self::Deferred { run_function } => run_function(environment),
        }
    }

    /// Creates a Reader that returns a constant value, ignoring the environment.
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
    /// use lambars::effect::Reader;
    ///
    /// let reader: Reader<i32, &str> = Reader::pure("constant");
    /// // run consumes self, so clone if you need to call it multiple times
    /// assert_eq!(reader.clone().run(0), "constant");
    /// assert_eq!(reader.run(100), "constant");
    /// ```
    pub const fn pure(value: A) -> Self {
        Self::Pure { value }
    }

    /// Maps a function over the result of this Reader.
    ///
    /// This is the Functor operation for Reader.
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
    /// use lambars::effect::Reader;
    ///
    /// let reader: Reader<i32, i32> = Reader::new(|environment| environment);
    /// let mapped = reader.fmap(|value| value * 2);
    /// assert_eq!(mapped.run(21), 42);
    /// ```
    pub fn fmap<B, F>(self, function: F) -> Reader<R, B>
    where
        F: Fn(A) -> B + 'static,
        B: 'static,
    {
        match self {
            Self::Pure { value } => Reader::Pure {
                value: function(value),
            },
            Self::Deferred { run_function } => Reader::Deferred {
                run_function: Rc::new(move |environment| function(run_function(environment))),
            },
        }
    }

    /// Chains this Reader with a function that produces another Reader.
    ///
    /// This is the Monad operation for Reader.
    ///
    /// For `Pure` variants, the function is applied immediately (eager evaluation).
    /// For `Deferred` variants, evaluation is deferred until `run()` is called.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes the result and produces a new Reader
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::Reader;
    ///
    /// let reader: Reader<i32, i32> = Reader::new(|environment| environment);
    /// let chained = reader.flat_map(|value| Reader::new(move |environment| value + environment));
    /// assert_eq!(chained.run(10), 20); // 10 + 10
    /// ```
    pub fn flat_map<B, F>(self, function: F) -> Reader<R, B>
    where
        F: Fn(A) -> Reader<R, B> + 'static,
        B: 'static,
        R: Clone,
    {
        match self {
            Self::Pure { value } => function(value),
            Self::Deferred { run_function } => Reader::Deferred {
                run_function: Rc::new(move |environment: R| {
                    let a = run_function(environment.clone());
                    let next_reader = function(a);
                    // Handle Pure/Deferred without requiring B: Clone
                    match next_reader {
                        Reader::Pure { value } => value,
                        Reader::Deferred {
                            run_function: next_run,
                        } => next_run(environment),
                    }
                }),
            },
        }
    }

    /// Alias for `flat_map` to match Rust's naming conventions.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes the result and produces a new Reader
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::Reader;
    ///
    /// let reader: Reader<i32, i32> = Reader::new(|environment| environment);
    /// let chained = reader.and_then(|value| Reader::new(move |environment| value + environment));
    /// assert_eq!(chained.run(10), 20);
    /// ```
    pub fn and_then<B, F>(self, function: F) -> Reader<R, B>
    where
        F: Fn(A) -> Reader<R, B> + 'static,
        B: 'static,
        R: Clone,
    {
        self.flat_map(function)
    }

    /// Sequences two Readers, discarding the first result.
    ///
    /// For `Pure` variants (no environment access), returns next directly.
    /// For `Deferred` variants, chains the computations without `Rc::new(next)`.
    ///
    /// # Arguments
    ///
    /// * `next` - The Reader to execute after this one
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::Reader;
    ///
    /// let reader1: Reader<i32, i32> = Reader::new(|environment| environment);
    /// let reader2: Reader<i32, &str> = Reader::pure("result");
    /// let sequenced = reader1.then(reader2);
    /// assert_eq!(sequenced.run(42), "result");
    /// ```
    #[must_use]
    pub fn then<B>(self, next: Reader<R, B>) -> Reader<R, B>
    where
        B: Clone + 'static,
        R: Clone,
    {
        match self {
            Self::Pure { .. } => next, // Pure has no environment access, return next directly
            Self::Deferred { run_function } => {
                // Decompose next to avoid Rc::new(next)
                match next {
                    Reader::Pure { value } => Reader::Deferred {
                        run_function: Rc::new(move |environment: R| {
                            let _ = run_function(environment);
                            value.clone()
                        }),
                    },
                    Reader::Deferred {
                        run_function: next_run,
                    } => Reader::Deferred {
                        run_function: Rc::new(move |environment: R| {
                            let _ = run_function(environment.clone());
                            next_run(environment)
                        }),
                    },
                }
            }
        }
    }

    /// Combines two Readers using a binary function.
    ///
    /// This is the Applicative map2 operation for Reader.
    ///
    /// # Arguments
    ///
    /// * `other` - The second Reader
    /// * `function` - A function that combines the results
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::Reader;
    ///
    /// let reader1: Reader<i32, i32> = Reader::new(|environment| environment);
    /// let reader2: Reader<i32, i32> = Reader::new(|environment| environment * 2);
    /// let combined = reader1.map2(reader2, |a, b| a + b);
    /// assert_eq!(combined.run(10), 30); // 10 + 20
    /// ```
    pub fn map2<B, C, F>(self, other: Reader<R, B>, function: F) -> Reader<R, C>
    where
        F: Fn(A, B) -> C + 'static,
        A: Clone,
        B: Clone + 'static,
        C: 'static,
        R: Clone,
    {
        match (self, other) {
            (Self::Pure { value: a }, Reader::Pure { value: b }) => Reader::Pure {
                value: function(a, b),
            },
            (Self::Pure { value: a }, Reader::Deferred { run_function }) => Reader::Deferred {
                run_function: Rc::new(move |environment| {
                    let b = run_function(environment);
                    function(a.clone(), b)
                }),
            },
            (Self::Deferred { run_function }, Reader::Pure { value: b }) => Reader::Deferred {
                run_function: Rc::new(move |environment| {
                    let a = run_function(environment);
                    function(a, b.clone())
                }),
            },
            (
                Self::Deferred {
                    run_function: self_function,
                },
                Reader::Deferred {
                    run_function: other_function,
                },
            ) => Reader::Deferred {
                run_function: Rc::new(move |environment: R| {
                    let a = self_function(environment.clone());
                    let b = other_function(environment);
                    function(a, b)
                }),
            },
        }
    }

    /// Combines three Readers using a ternary function.
    ///
    /// # Arguments
    ///
    /// * `second` - The second Reader
    /// * `third` - The third Reader
    /// * `function` - A function that combines the results
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::Reader;
    ///
    /// let reader1: Reader<i32, i32> = Reader::new(|environment| environment);
    /// let reader2: Reader<i32, i32> = Reader::new(|environment| environment * 2);
    /// let reader3: Reader<i32, i32> = Reader::new(|environment| environment * 3);
    /// let combined = reader1.map3(reader2, reader3, |a, b, c| a + b + c);
    /// assert_eq!(combined.run(10), 60); // 10 + 20 + 30
    /// ```
    pub fn map3<B, C, D, F>(
        self,
        second: Reader<R, B>,
        third: Reader<R, C>,
        function: F,
    ) -> Reader<R, D>
    where
        F: Fn(A, B, C) -> D + 'static,
        A: Clone,
        B: Clone + 'static,
        C: Clone + 'static,
        D: 'static,
        R: Clone,
    {
        // For map3, we compose using map2 to avoid complex match combinations
        self.map2(second, |a, b| (a, b))
            .map2(third, move |(a, b), c| function(a, b, c))
    }

    /// Combines two Readers into a tuple.
    ///
    /// # Arguments
    ///
    /// * `other` - The second Reader
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::Reader;
    ///
    /// let reader1: Reader<i32, i32> = Reader::new(|environment| environment);
    /// let reader2: Reader<i32, &str> = Reader::pure("hello");
    /// let product = reader1.product(reader2);
    /// assert_eq!(product.run(42), (42, "hello"));
    /// ```
    #[must_use]
    pub fn product<B>(self, other: Reader<R, B>) -> Reader<R, (A, B)>
    where
        A: Clone,
        B: Clone + 'static,
        R: Clone,
    {
        self.map2(other, |a, b| (a, b))
    }

    /// Applies a function inside a Reader to a value inside another Reader.
    ///
    /// # Arguments
    ///
    /// * `other` - The Reader containing the value to apply the function to
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::Reader;
    ///
    /// let function_reader: Reader<i32, fn(i32) -> i32> = Reader::pure(|x| x + 1);
    /// let value_reader: Reader<i32, i32> = Reader::new(|environment| environment);
    /// let result = function_reader.apply(value_reader);
    /// assert_eq!(result.run(41), 42);
    /// ```
    #[must_use]
    pub fn apply<B, Output>(self, other: Reader<R, B>) -> Reader<R, Output>
    where
        A: Fn(B) -> Output + Clone + 'static,
        B: Clone + 'static,
        Output: 'static,
        R: Clone,
    {
        self.map2(other, |function, b| function(b))
    }
}

impl<Env> Reader<Env, Env>
where
    Env: Clone + 'static,
{
    /// Creates a Reader that returns the entire environment.
    ///
    /// This is the fundamental operation of `MonadReader`.
    /// This creates a `Deferred` variant since it depends on the environment.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::Reader;
    ///
    /// let reader: Reader<i32, i32> = Reader::ask();
    /// assert_eq!(reader.run(42), 42);
    /// ```
    #[must_use]
    pub fn ask() -> Self {
        Self::Deferred {
            run_function: Rc::new(|environment| environment),
        }
    }
}

impl<R, A> Reader<R, A>
where
    R: 'static,
    A: 'static,
{
    /// Creates a Reader that projects a value from the environment.
    ///
    /// This is a convenience method that combines `ask` with a projection.
    /// This creates a `Deferred` variant since it depends on the environment.
    ///
    /// # Arguments
    ///
    /// * `projection` - A function that extracts a value from the environment
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::Reader;
    ///
    /// #[derive(Clone)]
    /// struct Config { port: u16 }
    ///
    /// let reader: Reader<Config, u16> = Reader::asks(|config: Config| config.port);
    /// let config = Config { port: 8080 };
    /// assert_eq!(reader.run(config), 8080);
    /// ```
    pub fn asks<F>(projection: F) -> Self
    where
        F: Fn(R) -> A + 'static,
    {
        Self::Deferred {
            run_function: Rc::new(projection),
        }
    }

    /// Runs a computation with a modified environment.
    ///
    /// The modifier function transforms the outer environment into the
    /// environment seen by the inner computation.
    ///
    /// For `Pure` variants, the computation is returned unchanged since
    /// it doesn't depend on the environment.
    ///
    /// # Arguments
    ///
    /// * `modifier` - A function that transforms the environment
    /// * `computation` - The computation to run with the modified environment
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::Reader;
    ///
    /// let reader: Reader<i32, i32> = Reader::new(|environment| environment * 2);
    /// let local_reader = Reader::local(|environment| environment + 10, reader);
    /// assert_eq!(local_reader.run(5), 30); // (5 + 10) * 2
    /// ```
    pub fn local<F>(modifier: F, computation: Self) -> Self
    where
        F: Fn(R) -> R + 'static,
    {
        match computation {
            Self::Pure { value } => Self::Pure { value },
            Self::Deferred { run_function } => Self::Deferred {
                run_function: Rc::new(move |environment| {
                    let modified_environment = modifier(environment);
                    run_function(modified_environment)
                }),
            },
        }
    }
}

impl<R, A> Clone for Reader<R, A>
where
    R: 'static,
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

impl<R, A> std::fmt::Display for Reader<R, A>
where
    R: 'static,
    A: 'static,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pure { .. } => write!(formatter, "<Reader::Pure>"),
            Self::Deferred { .. } => write!(formatter, "<Reader::Deferred>"),
        }
    }
}

impl<R: 'static, A: 'static> crate::typeclass::ReaderLike for Reader<R, A> {
    type Environment = R;
    type Value = A;

    fn into_reader(self) -> Self
    where
        R: Clone + 'static,
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
    fn test_display_reader_pure() {
        let reader: Reader<i32, i32> = Reader::pure(42);
        assert_eq!(format!("{reader}"), "<Reader::Pure>");
    }

    #[rstest]
    fn test_display_reader_deferred() {
        let reader: Reader<i32, i32> = Reader::new(|environment| environment * 2);
        assert_eq!(format!("{reader}"), "<Reader::Deferred>");
    }

    // =========================================================================
    // Pure/Deferred Pattern Tests
    // =========================================================================

    #[rstest]
    fn test_pure_creates_pure_variant() {
        let reader: Reader<i32, i32> = Reader::pure(42);
        match reader {
            Reader::Pure { value } => assert_eq!(value, 42),
            Reader::Deferred { .. } => panic!("Expected Pure variant"),
        }
    }

    #[rstest]
    fn test_new_creates_deferred_variant() {
        let reader: Reader<i32, i32> = Reader::new(|environment| environment);
        match reader {
            Reader::Pure { .. } => panic!("Expected Deferred variant"),
            Reader::Deferred { .. } => {} // OK
        }
    }

    #[rstest]
    fn test_pure_run_returns_value_ignoring_environment() {
        let reader: Reader<i32, &str> = Reader::pure("constant");
        assert_eq!(reader.run_cloned(0), "constant");
        assert_eq!(reader.run_cloned(100), "constant");
    }

    #[rstest]
    fn test_fmap_on_pure_returns_pure() {
        let reader: Reader<i32, i32> = Reader::pure(21);
        let mapped = reader.fmap(|x| x * 2);
        match mapped {
            Reader::Pure { value } => assert_eq!(value, 42),
            Reader::Deferred { .. } => panic!("Expected Pure variant"),
        }
    }

    #[rstest]
    fn test_fmap_on_deferred_returns_deferred() {
        let reader: Reader<i32, i32> = Reader::new(|environment| environment);
        let mapped = reader.fmap(|x| x * 2);
        match mapped {
            Reader::Pure { .. } => panic!("Expected Deferred variant"),
            Reader::Deferred { .. } => {} // OK
        }
    }

    #[rstest]
    fn test_flat_map_on_pure_returns_function_result() {
        let reader: Reader<i32, i32> = Reader::pure(21);
        let chained = reader.flat_map(|x| Reader::pure(x * 2));
        match chained {
            Reader::Pure { value } => assert_eq!(value, 42),
            Reader::Deferred { .. } => panic!("Expected Pure variant"),
        }
    }

    #[rstest]
    fn test_flat_map_on_pure_to_deferred() {
        let reader: Reader<i32, i32> = Reader::pure(21);
        let chained = reader.flat_map(|x| Reader::new(move |environment| x + environment));
        match chained {
            Reader::Pure { .. } => panic!("Expected Deferred variant"),
            Reader::Deferred { .. } => {} // OK
        }
        assert_eq!(chained.run(10), 31);
    }

    #[rstest]
    fn test_then_pure_to_next() {
        let reader1: Reader<i32, i32> = Reader::pure(1);
        let reader2: Reader<i32, &str> = Reader::pure("result");
        let sequenced = reader1.then(reader2);
        // Pure.then(anything) should return next directly
        assert_eq!(sequenced.run(42), "result");
    }

    #[rstest]
    fn test_then_deferred_to_pure() {
        let reader1: Reader<i32, i32> = Reader::new(|environment| environment);
        let reader2: Reader<i32, &str> = Reader::pure("result");
        let sequenced = reader1.then(reader2);
        assert_eq!(sequenced.run(42), "result");
    }

    #[rstest]
    fn test_then_deferred_to_deferred() {
        let reader1: Reader<i32, i32> = Reader::new(|environment| environment);
        let reader2: Reader<i32, i32> = Reader::new(|environment| environment * 2);
        let sequenced = reader1.then(reader2);
        assert_eq!(sequenced.run(5), 10);
    }

    // =========================================================================
    // Original Tests
    // =========================================================================

    #[rstest]
    fn reader_new_and_run() {
        let reader: Reader<i32, i32> = Reader::new(|environment| environment * 2);
        assert_eq!(reader.run(21), 42);
    }

    #[rstest]
    fn reader_pure_ignores_environment() {
        let reader: Reader<i32, &str> = Reader::pure("constant");
        assert_eq!(reader.run(0), "constant");
    }

    #[rstest]
    fn reader_ask_returns_environment() {
        let reader: Reader<i32, i32> = Reader::ask();
        assert_eq!(reader.run(42), 42);
    }

    #[rstest]
    fn reader_asks_projects_environment() {
        let reader: Reader<i32, String> = Reader::asks(|environment: i32| environment.to_string());
        assert_eq!(reader.run(42), "42");
    }

    #[rstest]
    fn reader_fmap_transforms_result() {
        let reader: Reader<i32, i32> = Reader::new(|environment| environment);
        let mapped = reader.fmap(|value| value * 2);
        assert_eq!(mapped.run(21), 42);
    }

    #[rstest]
    fn reader_flat_map_chains_readers() {
        let reader: Reader<i32, i32> = Reader::new(|environment| environment);
        let chained = reader.flat_map(|value| Reader::new(move |environment| value + environment));
        assert_eq!(chained.run(10), 20);
    }

    #[rstest]
    fn reader_local_modifies_environment() {
        let reader: Reader<i32, i32> = Reader::new(|environment| environment * 2);
        let local_reader = Reader::local(|environment| environment + 10, reader);
        assert_eq!(local_reader.run(5), 30);
    }

    #[rstest]
    fn reader_local_preserves_pure() {
        let reader: Reader<i32, i32> = Reader::pure(42);
        let local_reader = Reader::local(|environment| environment + 10, reader);
        // Pure should be preserved, environment modification has no effect
        match local_reader {
            Reader::Pure { value } => assert_eq!(value, 42),
            Reader::Deferred { .. } => panic!("Expected Pure variant"),
        }
    }

    #[rstest]
    fn reader_map2_combines_readers() {
        let reader1: Reader<i32, i32> = Reader::new(|environment| environment);
        let reader2: Reader<i32, i32> = Reader::new(|environment| environment * 2);
        let combined = reader1.map2(reader2, |a, b| a + b);
        assert_eq!(combined.run(10), 30);
    }

    #[rstest]
    fn reader_clone_works() {
        let reader: Reader<i32, i32> = Reader::new(|environment| environment * 2);
        let cloned = reader.clone();
        assert_eq!(reader.run(21), 42);
        assert_eq!(cloned.run(21), 42);
    }

    #[rstest]
    fn reader_clone_pure_works() {
        let reader: Reader<i32, i32> = Reader::pure(42);
        let cloned = reader.clone();
        assert_eq!(reader.run(0), 42);
        assert_eq!(cloned.run(0), 42);
    }

    // =========================================================================
    // Monad Laws Tests
    // =========================================================================

    mod monad_laws {
        use super::*;

        // Left Identity: pure(a).flat_map(f).run(r) == f(a).run(r)
        #[rstest]
        #[case(0, 42)]
        #[case(10, 100)]
        #[case(-5, 0)]
        fn left_identity_law(#[case] environment: i32, #[case] value: i32) {
            let f = |x: i32| Reader::new(move |r: i32| x + r);

            let lhs = Reader::pure(value).flat_map(f);
            let rhs = f(value);

            assert_eq!(lhs.run(environment), rhs.run(environment));
        }

        // Right Identity: m.flat_map(pure).run(r) == m.run(r)
        #[rstest]
        fn right_identity_law_pure() {
            let reader: Reader<i32, i32> = Reader::pure(42);
            let lhs = reader.clone().flat_map(Reader::pure);
            assert_eq!(lhs.run(10), reader.run(10));
        }

        #[rstest]
        fn right_identity_law_deferred() {
            let reader: Reader<i32, i32> = Reader::new(|r| r * 2);
            let lhs = reader.clone().flat_map(Reader::pure);
            assert_eq!(lhs.run(10), reader.run(10));
        }

        // Associativity: m.flat_map(f).flat_map(g).run(r) == m.flat_map(|x| f(x).flat_map(g)).run(r)
        #[rstest]
        fn associativity_law_pure() {
            let m: Reader<i32, i32> = Reader::pure(5);
            let f = |x: i32| Reader::new(move |r: i32| x + r);
            let g = |x: i32| Reader::new(move |r: i32| x * r);

            let lhs = m.clone().flat_map(f).flat_map(g);
            let rhs = m.flat_map(move |x| f(x).flat_map(g));

            assert_eq!(lhs.run(10), rhs.run(10));
        }

        #[rstest]
        fn associativity_law_deferred() {
            let m: Reader<i32, i32> = Reader::new(|r| r);
            let f = |x: i32| Reader::new(move |r: i32| x + r);
            let g = |x: i32| Reader::new(move |r: i32| x * r);

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

        // Identity: reader.fmap(|x| x).run(r) == reader.run(r)
        #[rstest]
        fn identity_law_pure() {
            let reader: Reader<i32, i32> = Reader::pure(42);
            let mapped = reader.clone().fmap(|x| x);
            assert_eq!(mapped.run(10), reader.run(10));
        }

        #[rstest]
        fn identity_law_deferred() {
            let reader: Reader<i32, i32> = Reader::new(|r| r * 2);
            let mapped = reader.clone().fmap(|x| x);
            assert_eq!(mapped.run(10), reader.run(10));
        }

        // Composition: reader.fmap(f).fmap(g).run(r) == reader.fmap(|x| g(f(x))).run(r)
        #[rstest]
        fn composition_law_pure() {
            let reader: Reader<i32, i32> = Reader::pure(5);
            let f = |x: i32| x + 1;
            let g = |x: i32| x * 2;

            let lhs = reader.clone().fmap(f).fmap(g);
            let rhs = reader.fmap(move |x| g(f(x)));

            assert_eq!(lhs.run(10), rhs.run(10));
        }

        #[rstest]
        fn composition_law_deferred() {
            let reader: Reader<i32, i32> = Reader::new(|r| r);
            let f = |x: i32| x + 1;
            let g = |x: i32| x * 2;

            let lhs = reader.clone().fmap(f).fmap(g);
            let rhs = reader.fmap(move |x| g(f(x)));

            assert_eq!(lhs.run(10), rhs.run(10));
        }
    }

    // =========================================================================
    // MonadReader Laws Tests
    // =========================================================================

    mod monad_reader_laws {
        use super::*;

        // Ask Local Identity: local(|r| r, m).run(r) == m.run(r)
        #[rstest]
        fn ask_local_identity_pure() {
            let reader: Reader<i32, i32> = Reader::pure(42);
            let local_reader = Reader::local(|r| r, reader.clone());
            assert_eq!(local_reader.run(10), reader.run(10));
        }

        #[rstest]
        fn ask_local_identity_deferred() {
            let reader: Reader<i32, i32> = Reader::new(|r| r * 2);
            let local_reader = Reader::local(|r| r, reader.clone());
            assert_eq!(local_reader.run(10), reader.run(10));
        }

        // Ask Retrieval: ask().run(r) == r
        #[rstest]
        #[case(0)]
        #[case(42)]
        #[case(-100)]
        fn ask_retrieval(#[case] environment: i32) {
            let reader: Reader<i32, i32> = Reader::ask();
            assert_eq!(reader.run(environment), environment);
        }

        // Local Composition: local(f, local(g, m)).run(r) == local(|r| g(f(r)), m).run(r)
        #[rstest]
        fn local_composition() {
            let m: Reader<i32, i32> = Reader::new(|r| r * 2);
            let f = |r: i32| r + 1;
            let g = |r: i32| r * 3;

            let lhs = Reader::local(f, Reader::local(g, m.clone()));
            let rhs = Reader::local(move |r| g(f(r)), m);

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
            let chain = Reader::pure(10)
                .flat_map(|x| Reader::new(move |r: i32| x + r))
                .fmap(|x| x * 2);

            // pure(10) -> Deferred(|r| 10 + r) with r=5 -> 15
            // fmap(|x| x * 2) -> 30
            assert_eq!(chain.run(5), 30);
        }

        #[rstest]
        fn mixed_map2_pure_pure() {
            let r1: Reader<i32, i32> = Reader::pure(10);
            let r2: Reader<i32, i32> = Reader::pure(20);
            let combined = r1.map2(r2, |a, b| a + b);

            match combined {
                Reader::Pure { value } => assert_eq!(value, 30),
                Reader::Deferred { .. } => panic!("Expected Pure variant"),
            }
        }

        #[rstest]
        fn mixed_map2_pure_deferred() {
            let r1: Reader<i32, i32> = Reader::pure(10);
            let r2: Reader<i32, i32> = Reader::new(|r| r * 2);
            let combined = r1.map2(r2, |a, b| a + b);

            assert_eq!(combined.run(5), 20); // 10 + (5 * 2)
        }

        #[rstest]
        fn config_example() {
            #[derive(Clone)]
            struct Config {
                port: u16,
                host: String,
            }

            fn get_port() -> Reader<Config, u16> {
                Reader::asks(|config: Config| config.port)
            }

            fn get_host() -> Reader<Config, String> {
                Reader::asks(|config: Config| config.host)
            }

            fn get_address() -> Reader<Config, String> {
                get_host().map2(get_port(), |host, port| format!("{host}:{port}"))
            }

            let config = Config {
                port: 8080,
                host: "localhost".to_string(),
            };

            assert_eq!(get_address().run(config), "localhost:8080");
        }
    }
}
