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
pub struct Reader<R, A>
where
    R: 'static,
    A: 'static,
{
    /// The wrapped function from environment to result.
    /// Uses Rc to allow cloning of the Reader for `flat_map`.
    run_function: Rc<dyn Fn(R) -> A>,
}

impl<R, A> Reader<R, A>
where
    R: 'static,
    A: 'static,
{
    /// Creates a new Reader from a function.
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
        Self {
            run_function: Rc::new(function),
        }
    }

    /// Runs the Reader computation with the given environment.
    ///
    /// This is the primary way to extract a value from a Reader.
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
    /// // Reader can be run multiple times
    /// assert_eq!(reader.run(0), 1);
    /// ```
    pub fn run(&self, environment: R) -> A {
        (self.run_function)(environment)
    }

    /// Creates a Reader that returns a constant value, ignoring the environment.
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
    /// use lambars::effect::Reader;
    ///
    /// let reader: Reader<i32, &str> = Reader::pure("constant");
    /// assert_eq!(reader.run(0), "constant");
    /// assert_eq!(reader.run(100), "constant");
    /// ```
    pub fn pure(value: A) -> Self
    where
        A: Clone,
    {
        Self::new(move |_| value.clone())
    }

    /// Maps a function over the result of this Reader.
    ///
    /// This is the Functor operation for Reader.
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
        let original_function = self.run_function;
        Reader::new(move |environment| {
            let result = (original_function)(environment);
            function(result)
        })
    }

    /// Chains this Reader with a function that produces another Reader.
    ///
    /// This is the Monad operation for Reader.
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
        let original_function = self.run_function;
        Reader::new(move |environment: R| {
            let a = (original_function)(environment.clone());
            let next_reader = function(a);
            next_reader.run(environment)
        })
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
        B: 'static,
        R: Clone,
    {
        self.flat_map(move |_| next.clone())
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
        B: 'static,
        C: 'static,
        R: Clone,
    {
        let self_function = self.run_function;
        let other_function = other.run_function;
        Reader::new(move |environment: R| {
            let a = (self_function)(environment.clone());
            let b = (other_function)(environment);
            function(a, b)
        })
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
        B: 'static,
        C: 'static,
        D: 'static,
        R: Clone,
    {
        let self_function = self.run_function;
        let second_function = second.run_function;
        let third_function = third.run_function;
        Reader::new(move |environment: R| {
            let a = (self_function)(environment.clone());
            let b = (second_function)(environment.clone());
            let c = (third_function)(environment);
            function(a, b, c)
        })
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
        B: 'static,
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
        A: Fn(B) -> Output + 'static,
        B: 'static,
        Output: 'static,
        R: Clone,
    {
        self.map2(other, |function, b| function(b))
    }
}

// =============================================================================
// MonadReader Operations (as inherent methods)
// =============================================================================

impl<Env> Reader<Env, Env>
where
    Env: Clone + 'static,
{
    /// Creates a Reader that returns the entire environment.
    ///
    /// This is the fundamental operation of `MonadReader`.
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
        Self::new(|environment| environment)
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
        Self::new(projection)
    }

    /// Runs a computation with a modified environment.
    ///
    /// The modifier function transforms the outer environment into the
    /// environment seen by the inner computation.
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
        let computation_function = computation.run_function;
        Self::new(move |environment| {
            let modified_environment = modifier(environment);
            (computation_function)(modified_environment)
        })
    }
}

// =============================================================================
// Clone Implementation
// =============================================================================

impl<R, A> Clone for Reader<R, A>
where
    R: 'static,
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

impl<R, A> std::fmt::Display for Reader<R, A>
where
    R: 'static,
    A: 'static,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "<Reader>")
    }
}

// =============================================================================
// ReaderLike Implementation
// =============================================================================

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
    fn test_display_reader() {
        let reader: Reader<i32, i32> = Reader::new(|environment| environment * 2);
        assert_eq!(format!("{reader}"), "<Reader>");
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
}
