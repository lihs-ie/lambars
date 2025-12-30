//! Writer Monad - computation with accumulated output.
//!
//! The Writer monad represents computations that produce a value along
//! with accumulated output (logs, metrics, etc.). It is useful for
//! logging, debugging, and collecting auxiliary data during computation.
//!
//! # Overview
//!
//! A `Writer<W, A>` encapsulates a pair `(A, W)`, where `A` is the result
//! type and `W` is the output/log type. The output type must implement
//! `Monoid` to support combining outputs from sequential computations.
//!
//! # Note on Type Classes
//!
//! Writer provides its own `fmap`, `flat_map`, `map2`, etc. methods directly
//! on the type, rather than implementing the Functor/Applicative/Monad traits.
//! This is because Rust's type system requires 'static bounds on trait
//! implementations when using internal function types, and the standard type
//! class traits don't have these bounds. The methods work identically to their
//! type class counterparts.
//!
//! # Laws
//!
//! Writer satisfies all Functor, Applicative, and Monad laws, plus the
//! MonadWriter-specific laws:
//!
//! ## Functor Laws
//!
//! - Identity: `writer.fmap(|x| x) == writer`
//! - Composition: `writer.fmap(f).fmap(g) == writer.fmap(|x| g(f(x)))`
//!
//! ## Monad Laws
//!
//! - Left Identity: `Writer::pure(a).flat_map(f) == f(a)`
//! - Right Identity: `m.flat_map(Writer::pure) == m`
//! - Associativity: `m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))`
//!
//! ## MonadWriter Laws
//!
//! - Tell Monoid Law: `tell(w1).then(tell(w2)) == tell(w1.combine(w2))`
//! - Listen Tell Law: `listen(tell(w))` captures the output correctly
//! - Pass Identity Law: `pass(m.fmap(|a| (a, |w| w))) == m`
//! - Censor Definition: `censor(f, m) == pass(m.fmap(|a| (a, f)))`
//!
//! # Examples
//!
//! Basic usage:
//!
//! ```rust
//! use functional_rusty::effect::Writer;
//!
//! let writer: Writer<Vec<String>, i32> =
//!     Writer::new(42, vec!["computation completed".to_string()]);
//! let (result, logs) = writer.run();
//! assert_eq!(result, 42);
//! assert_eq!(logs, vec!["computation completed"]);
//! ```
//!
//! Logging pattern:
//!
//! ```rust
//! use functional_rusty::effect::Writer;
//!
//! fn log(message: &str) -> Writer<Vec<String>, ()> {
//!     Writer::tell(vec![message.to_string()])
//! }
//!
//! let computation = log("step 1")
//!     .then(log("step 2"))
//!     .then(Writer::pure(42));
//!
//! let (result, logs) = computation.run();
//! assert_eq!(result, 42);
//! assert_eq!(logs, vec!["step 1", "step 2"]);
//! ```

#![forbid(unsafe_code)]

use crate::typeclass::Monoid;

/// A monad for computations that produce accumulated output alongside a result.
///
/// `Writer<W, A>` represents a computation that produces a value of type `A`
/// and accumulates output of type `W`. The output type must be a `Monoid`
/// to support combining outputs from sequential computations.
///
/// # Type Parameters
///
/// - `W`: The output type (must implement `Monoid`)
/// - `A`: The result type
///
/// # Examples
///
/// ```rust
/// use functional_rusty::effect::Writer;
///
/// let computation: Writer<Vec<String>, i32> = Writer::tell(vec!["log".to_string()])
///     .then(Writer::pure(42));
///
/// let (result, output) = computation.run();
/// assert_eq!(result, 42);
/// assert_eq!(output, vec!["log"]);
/// ```
#[derive(Debug)]
pub struct Writer<W, A>
where
    W: Monoid + 'static,
    A: 'static,
{
    /// The result value.
    result: A,
    /// The accumulated output.
    output: W,
}

impl<W, A> Writer<W, A>
where
    W: Monoid + 'static,
    A: 'static,
{
    /// Creates a new Writer with the given result and output.
    ///
    /// # Arguments
    ///
    /// * `result` - The result value
    /// * `output` - The initial output
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::effect::Writer;
    ///
    /// let writer: Writer<Vec<String>, i32> =
    ///     Writer::new(42, vec!["initial".to_string()]);
    /// let (result, output) = writer.run();
    /// assert_eq!(result, 42);
    /// assert_eq!(output, vec!["initial"]);
    /// ```
    pub fn new(result: A, output: W) -> Self {
        Writer { result, output }
    }

    /// Runs the Writer computation, returning the result and output.
    ///
    /// # Returns
    ///
    /// A tuple of (result, output).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::effect::Writer;
    ///
    /// let writer: Writer<Vec<String>, i32> =
    ///     Writer::new(42, vec!["log".to_string()]);
    /// let (result, output) = writer.run();
    /// assert_eq!(result, 42);
    /// assert_eq!(output, vec!["log"]);
    /// ```
    pub fn run(&self) -> (A, W)
    where
        A: Clone,
        W: Clone,
    {
        (self.result.clone(), self.output.clone())
    }

    /// Runs the Writer computation and returns only the result.
    ///
    /// # Returns
    ///
    /// The result value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::effect::Writer;
    ///
    /// let writer: Writer<Vec<String>, i32> =
    ///     Writer::new(42, vec!["log".to_string()]);
    /// assert_eq!(writer.eval(), 42);
    /// ```
    pub fn eval(&self) -> A
    where
        A: Clone,
    {
        self.result.clone()
    }

    /// Runs the Writer computation and returns only the output.
    ///
    /// # Returns
    ///
    /// The accumulated output.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::effect::Writer;
    ///
    /// let writer: Writer<Vec<String>, i32> =
    ///     Writer::new(42, vec!["log".to_string()]);
    /// assert_eq!(writer.exec(), vec!["log"]);
    /// ```
    pub fn exec(&self) -> W
    where
        W: Clone,
    {
        self.output.clone()
    }

    /// Creates a Writer that returns a constant value with empty output.
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
    /// use functional_rusty::effect::Writer;
    ///
    /// let writer: Writer<Vec<String>, i32> = Writer::pure(42);
    /// let (result, output) = writer.run();
    /// assert_eq!(result, 42);
    /// assert!(output.is_empty());
    /// ```
    pub fn pure(value: A) -> Self {
        Writer {
            result: value,
            output: W::empty(),
        }
    }

    /// Maps a function over the result of this Writer.
    ///
    /// This is the Functor operation for Writer.
    ///
    /// # Arguments
    ///
    /// * `function` - A function to apply to the result
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::effect::Writer;
    ///
    /// let writer: Writer<Vec<String>, i32> =
    ///     Writer::new(21, vec!["log".to_string()]);
    /// let mapped = writer.fmap(|value| value * 2);
    /// let (result, output) = mapped.run();
    /// assert_eq!(result, 42);
    /// assert_eq!(output, vec!["log"]);
    /// ```
    pub fn fmap<B, F>(self, function: F) -> Writer<W, B>
    where
        F: FnOnce(A) -> B,
        B: 'static,
    {
        Writer {
            result: function(self.result),
            output: self.output,
        }
    }

    /// Chains this Writer with a function that produces another Writer.
    ///
    /// This is the Monad operation for Writer. The outputs are combined
    /// using the `Monoid::combine` operation.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes the result and produces a new Writer
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::effect::Writer;
    ///
    /// let writer: Writer<Vec<String>, i32> =
    ///     Writer::new(10, vec!["first".to_string()]);
    /// let chained = writer.flat_map(|value| {
    ///     Writer::new(value * 2, vec!["second".to_string()])
    /// });
    /// let (result, output) = chained.run();
    /// assert_eq!(result, 20);
    /// assert_eq!(output, vec!["first", "second"]);
    /// ```
    pub fn flat_map<B, F>(self, function: F) -> Writer<W, B>
    where
        F: FnOnce(A) -> Writer<W, B>,
        B: 'static,
    {
        let next = function(self.result);
        Writer {
            result: next.result,
            output: self.output.combine(next.output),
        }
    }

    /// Alias for `flat_map` to match Rust's naming conventions.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes the result and produces a new Writer
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::effect::Writer;
    ///
    /// let writer: Writer<Vec<String>, i32> =
    ///     Writer::new(10, vec!["first".to_string()]);
    /// let chained = writer.and_then(|value| {
    ///     Writer::new(value + 5, vec!["second".to_string()])
    /// });
    /// let (result, output) = chained.run();
    /// assert_eq!(result, 15);
    /// assert_eq!(output, vec!["first", "second"]);
    /// ```
    pub fn and_then<B, F>(self, function: F) -> Writer<W, B>
    where
        F: FnOnce(A) -> Writer<W, B>,
        B: 'static,
    {
        self.flat_map(function)
    }

    /// Sequences two Writers, discarding the first result.
    ///
    /// # Arguments
    ///
    /// * `next` - The Writer to execute after this one
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::effect::Writer;
    ///
    /// let writer1: Writer<Vec<String>, i32> =
    ///     Writer::new(42, vec!["first".to_string()]);
    /// let writer2: Writer<Vec<String>, &str> =
    ///     Writer::new("result", vec!["second".to_string()]);
    /// let sequenced = writer1.then(writer2);
    /// let (result, output) = sequenced.run();
    /// assert_eq!(result, "result");
    /// assert_eq!(output, vec!["first", "second"]);
    /// ```
    pub fn then<B>(self, next: Writer<W, B>) -> Writer<W, B>
    where
        B: 'static,
    {
        Writer {
            result: next.result,
            output: self.output.combine(next.output),
        }
    }

    /// Combines two Writers using a binary function.
    ///
    /// This is the Applicative map2 operation for Writer.
    ///
    /// # Arguments
    ///
    /// * `other` - The second Writer
    /// * `function` - A function that combines the results
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::effect::Writer;
    ///
    /// let writer1: Writer<Vec<String>, i32> =
    ///     Writer::new(10, vec!["first".to_string()]);
    /// let writer2: Writer<Vec<String>, i32> =
    ///     Writer::new(20, vec!["second".to_string()]);
    /// let combined = writer1.map2(writer2, |a, b| a + b);
    /// let (result, output) = combined.run();
    /// assert_eq!(result, 30);
    /// assert_eq!(output, vec!["first", "second"]);
    /// ```
    pub fn map2<B, C, F>(self, other: Writer<W, B>, function: F) -> Writer<W, C>
    where
        F: FnOnce(A, B) -> C,
        B: 'static,
        C: 'static,
    {
        Writer {
            result: function(self.result, other.result),
            output: self.output.combine(other.output),
        }
    }

    /// Combines two Writers into a tuple.
    ///
    /// # Arguments
    ///
    /// * `other` - The second Writer
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::effect::Writer;
    ///
    /// let writer1: Writer<Vec<String>, i32> =
    ///     Writer::new(42, vec!["first".to_string()]);
    /// let writer2: Writer<Vec<String>, &str> =
    ///     Writer::new("hello", vec!["second".to_string()]);
    /// let product = writer1.product(writer2);
    /// let ((first, second), output) = product.run();
    /// assert_eq!(first, 42);
    /// assert_eq!(second, "hello");
    /// assert_eq!(output, vec!["first", "second"]);
    /// ```
    pub fn product<B>(self, other: Writer<W, B>) -> Writer<W, (A, B)>
    where
        B: 'static,
    {
        self.map2(other, |a, b| (a, b))
    }
}

// =============================================================================
// MonadWriter Operations (as inherent methods)
// =============================================================================

impl<W> Writer<W, ()>
where
    W: Monoid + 'static,
{
    /// Creates a Writer that appends output without producing a meaningful result.
    ///
    /// This is the fundamental "tell" operation of MonadWriter.
    ///
    /// # Arguments
    ///
    /// * `output` - The output to append
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::effect::Writer;
    ///
    /// let writer: Writer<Vec<String>, ()> =
    ///     Writer::tell(vec!["log message".to_string()]);
    /// let (result, output) = writer.run();
    /// assert_eq!(result, ());
    /// assert_eq!(output, vec!["log message"]);
    /// ```
    pub fn tell(output: W) -> Self {
        Writer { result: (), output }
    }
}

impl<W, A> Writer<W, A>
where
    W: Monoid + Clone + 'static,
    A: 'static,
{
    /// Executes a computation and also returns its output.
    ///
    /// This allows inspecting the output that a computation produces.
    ///
    /// # Arguments
    ///
    /// * `computation` - The computation whose output to capture
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::effect::Writer;
    ///
    /// let writer: Writer<Vec<String>, i32> =
    ///     Writer::new(42, vec!["log".to_string()]);
    /// let listened = Writer::listen(writer);
    /// let ((result, captured), output) = listened.run();
    /// assert_eq!(result, 42);
    /// assert_eq!(captured, vec!["log"]);
    /// assert_eq!(output, vec!["log"]);
    /// ```
    pub fn listen(computation: Writer<W, A>) -> Writer<W, (A, W)> {
        Writer {
            result: (computation.result, computation.output.clone()),
            output: computation.output,
        }
    }

    /// Executes a computation with a function that modifies the output.
    ///
    /// The computation produces a tuple of a result and a function.
    /// The function is applied to transform the accumulated output.
    ///
    /// # Arguments
    ///
    /// * `computation` - A computation that produces (result, output_modifier)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::effect::Writer;
    ///
    /// let writer: Writer<Vec<String>, (i32, fn(Vec<String>) -> Vec<String>)> =
    ///     Writer::new(
    ///         (42, (|output: Vec<String>| {
    ///             output.into_iter().map(|s| s.to_uppercase()).collect()
    ///         }) as fn(Vec<String>) -> Vec<String>),
    ///         vec!["hello".to_string()],
    ///     );
    /// let passed = Writer::pass(writer);
    /// let (result, output) = passed.run();
    /// assert_eq!(result, 42);
    /// assert_eq!(output, vec!["HELLO"]);
    /// ```
    pub fn pass<F>(computation: Writer<W, (A, F)>) -> Writer<W, A>
    where
        F: FnOnce(W) -> W,
    {
        let (result, modifier) = computation.result;
        Writer {
            result,
            output: modifier(computation.output),
        }
    }

    /// Modifies the output of a computation.
    ///
    /// This is a convenience function that applies a modifier function
    /// to transform the accumulated output of a computation.
    ///
    /// # Arguments
    ///
    /// * `modifier` - A function that transforms the output
    /// * `computation` - The computation whose output to modify
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::effect::Writer;
    ///
    /// let writer: Writer<Vec<String>, i32> =
    ///     Writer::new(42, vec!["hello".to_string()]);
    /// let censored = Writer::censor(
    ///     |output: Vec<String>| output.into_iter().map(|s| s.to_uppercase()).collect(),
    ///     writer,
    /// );
    /// let (result, output) = censored.run();
    /// assert_eq!(result, 42);
    /// assert_eq!(output, vec!["HELLO"]);
    /// ```
    pub fn censor<F>(modifier: F, computation: Writer<W, A>) -> Writer<W, A>
    where
        F: FnOnce(W) -> W,
    {
        Writer {
            result: computation.result,
            output: modifier(computation.output),
        }
    }
}

// =============================================================================
// Clone Implementation
// =============================================================================

impl<W, A> Clone for Writer<W, A>
where
    W: Monoid + Clone + 'static,
    A: Clone + 'static,
{
    fn clone(&self) -> Self {
        Writer {
            result: self.result.clone(),
            output: self.output.clone(),
        }
    }
}

// =============================================================================
// PartialEq Implementation
// =============================================================================

impl<W, A> PartialEq for Writer<W, A>
where
    W: Monoid + PartialEq + 'static,
    A: PartialEq + 'static,
{
    fn eq(&self, other: &Self) -> bool {
        self.result == other.result && self.output == other.output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn writer_new_and_run() {
        let writer: Writer<Vec<String>, i32> = Writer::new(42, vec!["log".to_string()]);
        let (result, output) = writer.run();
        assert_eq!(result, 42);
        assert_eq!(output, vec!["log"]);
    }

    #[rstest]
    fn writer_pure_has_empty_output() {
        let writer: Writer<Vec<String>, i32> = Writer::pure(42);
        let (result, output) = writer.run();
        assert_eq!(result, 42);
        assert!(output.is_empty());
    }

    #[rstest]
    fn writer_tell_appends_output() {
        let writer: Writer<Vec<String>, ()> = Writer::tell(vec!["message".to_string()]);
        let (result, output) = writer.run();
        assert_eq!(result, ());
        assert_eq!(output, vec!["message"]);
    }

    #[rstest]
    fn writer_fmap_transforms_result() {
        let writer: Writer<Vec<String>, i32> = Writer::new(21, vec!["log".to_string()]);
        let mapped = writer.fmap(|value| value * 2);
        let (result, output) = mapped.run();
        assert_eq!(result, 42);
        assert_eq!(output, vec!["log"]);
    }

    #[rstest]
    fn writer_flat_map_combines_outputs() {
        let writer: Writer<Vec<String>, i32> = Writer::new(10, vec!["first".to_string()]);
        let chained = writer.flat_map(|value| Writer::new(value * 2, vec!["second".to_string()]));
        let (result, output) = chained.run();
        assert_eq!(result, 20);
        assert_eq!(output, vec!["first", "second"]);
    }

    #[rstest]
    fn writer_listen_captures_output() {
        let writer: Writer<Vec<String>, i32> = Writer::new(42, vec!["log".to_string()]);
        let listened = Writer::listen(writer);
        let ((result, captured), output) = listened.run();
        assert_eq!(result, 42);
        assert_eq!(captured, vec!["log"]);
        assert_eq!(output, vec!["log"]);
    }

    #[rstest]
    fn writer_censor_modifies_output() {
        let writer: Writer<Vec<String>, i32> = Writer::new(42, vec!["hello".to_string()]);
        let censored = Writer::censor(
            |output: Vec<String>| output.into_iter().map(|s| s.to_uppercase()).collect(),
            writer,
        );
        let (result, output) = censored.run();
        assert_eq!(result, 42);
        assert_eq!(output, vec!["HELLO"]);
    }

    #[rstest]
    fn writer_map2_combines_outputs() {
        let writer1: Writer<Vec<String>, i32> = Writer::new(10, vec!["first".to_string()]);
        let writer2: Writer<Vec<String>, i32> = Writer::new(20, vec!["second".to_string()]);
        let combined = writer1.map2(writer2, |a, b| a + b);
        let (result, output) = combined.run();
        assert_eq!(result, 30);
        assert_eq!(output, vec!["first", "second"]);
    }
}
