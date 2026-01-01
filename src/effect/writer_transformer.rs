//! WriterT - Writer Monad Transformer.
//!
//! WriterT adds output accumulation capability to any monad.
//! It transforms a monad M into a monad that can accumulate output W.
//!
//! # Overview
//!
//! `WriterT<W, M>` encapsulates `M<(A, W)>` where `W` is the output type (must be
//! a Monoid for combining outputs) and `M` is the inner monad. This allows
//! composing computations that produce output while also using the capabilities
//! of the inner monad.
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
//! use lambars::effect::WriterT;
//!
//! let writer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
//!     WriterT::new(Some((42, vec!["log".to_string()])));
//! assert_eq!(writer.run(), Some((42, vec!["log".to_string()])));
//! ```

#![forbid(unsafe_code)]

use crate::typeclass::Monoid;

use super::IO;

/// A monad transformer that adds output accumulation capability.
///
/// `WriterT<W, M>` represents a computation that produces a value and output
/// wrapped in monad `M`. The output type `W` must be a `Monoid` to support
/// combining outputs from sequential computations.
///
/// # Type Parameters
///
/// - `W`: The output type (must implement `Monoid`)
/// - `M`: The inner monad type (e.g., `Option<(A, W)>`, `Result<(A, W), E>`, `IO<(A, W)>`)
///
/// # Examples
///
/// ```rust
/// use lambars::effect::WriterT;
///
/// fn log(msg: &str) -> WriterT<Vec<String>, Option<((), Vec<String>)>> {
///     WriterT::<Vec<String>, Option<((), Vec<String>)>>::tell_option(vec![msg.to_string()])
/// }
///
/// let computation = log("step 1")
///     .flat_map_option(|_| log("step 2"))
///     .flat_map_option(|_| WriterT::<Vec<String>, Option<(i32, Vec<String>)>>::pure_option(42));
///
/// assert_eq!(computation.run(), Some((42, vec!["step 1".to_string(), "step 2".to_string()])));
/// ```
pub struct WriterT<W, M>
where
    W: Monoid + 'static,
{
    /// The wrapped monad containing (value, output).
    inner: M,
    /// Phantom data to hold the output type.
    _marker: std::marker::PhantomData<W>,
}

impl<W, M> WriterT<W, M>
where
    W: Monoid + 'static,
{
    /// Creates a new WriterT from an inner monad.
    ///
    /// # Arguments
    ///
    /// * `inner` - The inner monad containing (value, output)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::WriterT;
    ///
    /// let writer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
    ///     WriterT::new(Some((42, vec!["log".to_string()])));
    /// assert_eq!(writer.run(), Some((42, vec!["log".to_string()])));
    /// ```
    pub fn new(inner: M) -> Self {
        WriterT {
            inner,
            _marker: std::marker::PhantomData,
        }
    }

    /// Runs the WriterT computation, returning the inner monad.
    ///
    /// # Returns
    ///
    /// The inner monad containing (value, output).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::WriterT;
    ///
    /// let writer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
    ///     WriterT::new(Some((42, vec!["log".to_string()])));
    /// assert_eq!(writer.run(), Some((42, vec!["log".to_string()])));
    /// ```
    pub fn run(self) -> M {
        self.inner
    }
}

// =============================================================================
// Clone Implementation
// =============================================================================

impl<W, M> Clone for WriterT<W, M>
where
    W: Monoid + 'static,
    M: Clone,
{
    fn clone(&self) -> Self {
        WriterT {
            inner: self.inner.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}

// =============================================================================
// Option-specific Methods
// =============================================================================

impl<W, A> WriterT<W, Option<(A, W)>>
where
    W: Monoid + Clone + 'static,
    A: 'static,
{
    /// Creates a WriterT that returns a constant value with empty output.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to return
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::WriterT;
    /// use lambars::typeclass::Monoid;
    ///
    /// let writer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
    ///     WriterT::pure_option(42);
    /// assert_eq!(writer.run(), Some((42, Vec::<String>::empty())));
    /// ```
    pub fn pure_option(value: A) -> Self {
        WriterT::new(Some((value, W::empty())))
    }

    /// Lifts an Option into WriterT with empty output.
    ///
    /// # Arguments
    ///
    /// * `inner` - The Option to lift
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::WriterT;
    /// use lambars::typeclass::Monoid;
    ///
    /// let inner: Option<i32> = Some(42);
    /// let writer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
    ///     WriterT::lift_option(inner);
    /// assert_eq!(writer.run(), Some((42, Vec::<String>::empty())));
    /// ```
    pub fn lift_option(inner: Option<A>) -> Self {
        WriterT::new(inner.map(|value| (value, W::empty())))
    }

    /// Creates a WriterT that appends output without producing a meaningful result.
    ///
    /// # Arguments
    ///
    /// * `output` - The output to append
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::WriterT;
    ///
    /// let writer: WriterT<Vec<String>, Option<((), Vec<String>)>> =
    ///     WriterT::<Vec<String>, Option<((), Vec<String>)>>::tell_option(vec!["message".to_string()]);
    /// assert_eq!(writer.run(), Some(((), vec!["message".to_string()])));
    /// ```
    pub fn tell_option(output: W) -> WriterT<W, Option<((), W)>> {
        WriterT::new(Some(((), output)))
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
    /// use lambars::effect::WriterT;
    ///
    /// let writer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
    ///     WriterT::new(Some((21, vec!["log".to_string()])));
    /// let mapped = writer.fmap_option(|v| v * 2);
    /// assert_eq!(mapped.run(), Some((42, vec!["log".to_string()])));
    /// ```
    pub fn fmap_option<B, F>(self, function: F) -> WriterT<W, Option<(B, W)>>
    where
        F: FnOnce(A) -> B,
        B: 'static,
    {
        WriterT::new(self.inner.map(|(value, output)| (function(value), output)))
    }

    /// Chains WriterT computations with Option.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes the value and returns a new WriterT
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::WriterT;
    ///
    /// let writer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
    ///     WriterT::new(Some((10, vec!["first".to_string()])));
    /// let chained = writer.flat_map_option(|v| {
    ///     WriterT::new(Some((v * 2, vec!["second".to_string()])))
    /// });
    /// assert_eq!(chained.run(), Some((20, vec!["first".to_string(), "second".to_string()])));
    /// ```
    pub fn flat_map_option<B, F>(self, function: F) -> WriterT<W, Option<(B, W)>>
    where
        F: FnOnce(A) -> WriterT<W, Option<(B, W)>>,
        B: 'static,
    {
        match self.inner {
            Some((value, output1)) => {
                let next = function(value);
                match next.inner {
                    Some((result, output2)) => {
                        WriterT::new(Some((result, output1.combine(output2))))
                    }
                    None => WriterT::new(None),
                }
            }
            None => WriterT::new(None),
        }
    }

    /// Executes a computation and also returns its output.
    ///
    /// # Arguments
    ///
    /// * `computation` - The computation whose output to capture
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::effect::WriterT;
    ///
    /// let writer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
    ///     WriterT::new(Some((42, vec!["log".to_string()])));
    /// let listened = WriterT::listen_option(writer);
    /// assert_eq!(listened.run(), Some(((42, vec!["log".to_string()]), vec!["log".to_string()])));
    /// ```
    pub fn listen_option(computation: Self) -> WriterT<W, Option<((A, W), W)>> {
        match computation.inner {
            Some((value, output)) => WriterT::new(Some(((value, output.clone()), output))),
            None => WriterT::new(None),
        }
    }
}

// =============================================================================
// Result-specific Methods
// =============================================================================

impl<W, A, E> WriterT<W, Result<(A, W), E>>
where
    W: Monoid + Clone + 'static,
    A: 'static,
    E: 'static,
{
    /// Creates a WriterT that returns a constant value with empty output.
    pub fn pure_result(value: A) -> Self {
        WriterT::new(Ok((value, W::empty())))
    }

    /// Lifts a Result into WriterT with empty output.
    pub fn lift_result(inner: Result<A, E>) -> Self {
        WriterT::new(inner.map(|value| (value, W::empty())))
    }

    /// Creates a WriterT that appends output without producing a meaningful result.
    pub fn tell_result(output: W) -> WriterT<W, Result<((), W), E>> {
        WriterT::new(Ok(((), output)))
    }

    /// Maps a function over the value inside the Result.
    pub fn fmap_result<B, F>(self, function: F) -> WriterT<W, Result<(B, W), E>>
    where
        F: FnOnce(A) -> B,
        B: 'static,
    {
        WriterT::new(self.inner.map(|(value, output)| (function(value), output)))
    }

    /// Chains WriterT computations with Result.
    pub fn flat_map_result<B, F>(self, function: F) -> WriterT<W, Result<(B, W), E>>
    where
        F: FnOnce(A) -> WriterT<W, Result<(B, W), E>>,
        B: 'static,
    {
        match self.inner {
            Ok((value, output1)) => {
                let next = function(value);
                match next.inner {
                    Ok((result, output2)) => WriterT::new(Ok((result, output1.combine(output2)))),
                    Err(error) => WriterT::new(Err(error)),
                }
            }
            Err(error) => WriterT::new(Err(error)),
        }
    }

    /// Executes a computation and also returns its output.
    pub fn listen_result(computation: Self) -> WriterT<W, Result<((A, W), W), E>> {
        match computation.inner {
            Ok((value, output)) => WriterT::new(Ok(((value, output.clone()), output))),
            Err(error) => WriterT::new(Err(error)),
        }
    }
}

// =============================================================================
// IO-specific Methods
// =============================================================================

impl<W, A> WriterT<W, IO<(A, W)>>
where
    W: Monoid + Clone + 'static,
    A: 'static,
{
    /// Creates a WriterT that returns a constant value with empty output.
    pub fn pure_io(value: A) -> Self {
        WriterT::new(IO::pure((value, W::empty())))
    }

    /// Lifts an IO into WriterT with empty output.
    pub fn lift_io(inner: IO<A>) -> Self {
        WriterT::new(inner.fmap(|value| (value, W::empty())))
    }

    /// Creates a WriterT that appends output without producing a meaningful result.
    pub fn tell_io(output: W) -> WriterT<W, IO<((), W)>> {
        WriterT::new(IO::pure(((), output)))
    }

    /// Maps a function over the value inside the IO.
    pub fn fmap_io<B, F>(self, function: F) -> WriterT<W, IO<(B, W)>>
    where
        F: FnOnce(A) -> B + 'static,
        B: 'static,
    {
        WriterT::new(
            self.inner
                .fmap(move |(value, output)| (function(value), output)),
        )
    }

    /// Chains WriterT computations with IO.
    pub fn flat_map_io<B, F>(self, function: F) -> WriterT<W, IO<(B, W)>>
    where
        F: FnOnce(A) -> WriterT<W, IO<(B, W)>> + 'static,
        B: 'static,
    {
        WriterT::new(self.inner.flat_map(move |(value, output1)| {
            let next = function(value);
            next.inner
                .fmap(move |(result, output2)| (result, output1.combine(output2)))
        }))
    }

    /// Executes a computation and also returns its output.
    pub fn listen_io(computation: Self) -> WriterT<W, IO<((A, W), W)>> {
        WriterT::new(
            computation
                .inner
                .fmap(|(value, output)| ((value, output.clone()), output)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writer_transformer_new_and_run() {
        let writer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
            WriterT::new(Some((42, vec!["log".to_string()])));
        assert_eq!(writer.run(), Some((42, vec!["log".to_string()])));
    }

    #[test]
    fn writer_transformer_clone() {
        let writer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
            WriterT::new(Some((42, vec!["log".to_string()])));
        let cloned = writer.clone();
        assert_eq!(writer.run(), Some((42, vec!["log".to_string()])));
        assert_eq!(cloned.run(), Some((42, vec!["log".to_string()])));
    }

    #[test]
    fn writer_transformer_pure_option() {
        let writer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> = WriterT::pure_option(42);
        assert_eq!(writer.run(), Some((42, vec![])));
    }

    #[test]
    fn writer_transformer_tell_option() {
        let writer: WriterT<Vec<String>, Option<((), Vec<String>)>> =
            WriterT::<Vec<String>, Option<((), Vec<String>)>>::tell_option(vec!["log".to_string()]);
        assert_eq!(writer.run(), Some(((), vec!["log".to_string()])));
    }

    #[test]
    fn writer_transformer_flat_map_option() {
        let writer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
            WriterT::new(Some((10, vec!["first".to_string()])));
        let chained =
            writer.flat_map_option(|v| WriterT::new(Some((v * 2, vec!["second".to_string()]))));
        assert_eq!(
            chained.run(),
            Some((20, vec!["first".to_string(), "second".to_string()]))
        );
    }
}
