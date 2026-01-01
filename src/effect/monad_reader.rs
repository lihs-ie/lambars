//! `MonadReader` type class - environment reading capability.
//!
//! This module provides the `MonadReader` trait which abstracts
//! the ability to read from an environment. This is the core
//! abstraction of the Reader monad pattern.
//!
//! # Laws
//!
//! All `MonadReader` implementations must satisfy these laws:
//!
//! ## Ask Local Identity Law
//!
//! Applying the identity modifier should not change the computation:
//!
//! ```text
//! local(|r| r, m) == m
//! ```
//!
//! ## Ask Local Composition Law
//!
//! Local modifiers should compose correctly:
//!
//! ```text
//! local(f, local(g, m)) == local(|r| g(f(r)), m)
//! ```
//!
//! ## Ask Retrieval Law
//!
//! Ask should return the environment unchanged when run:
//!
//! ```text
//! ask().run(r) == r
//! ```
//!
//! # Examples
//!
//! ```rust,ignore
//! use lambars::effect::MonadReader;
//!
//! // Reader implementation will provide concrete examples
//! ```

use crate::typeclass::Monad;

/// A type class for monads that can read from an environment.
///
/// `MonadReader<R>` extends `Monad` with the ability to access
/// an environment of type `R`. This is useful for dependency injection,
/// configuration access, and other patterns where computations need
/// read-only access to some shared context.
///
/// # Type Parameters
///
/// - `R`: The environment type (read-only context)
///
/// # Laws
///
/// ## Ask Local Identity Law
///
/// Applying the identity modifier should not change the computation:
///
/// ```text
/// local(|r| r, m) == m
/// ```
///
/// ## Ask Local Composition Law
///
/// Local modifiers should compose correctly:
///
/// ```text
/// local(f, local(g, m)) == local(|r| g(f(r)), m)
/// ```
///
/// ## Ask Retrieval Law
///
/// Ask should return the environment unchanged when run:
///
/// ```text
/// ask().run(r) == r
/// ```
///
/// # Examples
///
/// ```rust,ignore
/// use lambars::effect::MonadReader;
///
/// struct Config {
///     port: u16,
///     host: String,
/// }
///
/// fn get_port<M: MonadReader<Config>>() -> M::WithType<u16>
/// where
///     Config: Clone + 'static,
/// {
///     M::asks(|config| config.port)
/// }
/// ```
pub trait MonadReader<R>: Monad {
    /// Retrieves the entire environment.
    ///
    /// This is the fundamental operation of `MonadReader`. It returns
    /// a computation that, when run with an environment, produces
    /// that environment as its result.
    ///
    /// # Returns
    ///
    /// A computation that produces the environment.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{MonadReader, Reader};
    ///
    /// let reader: Reader<i32, i32> = Reader::ask();
    /// assert_eq!(reader.run(42), 42);
    /// ```
    fn ask() -> Self;

    /// Executes a computation with a modified environment.
    ///
    /// This allows temporarily changing the environment for a
    /// sub-computation. The modifier function transforms the
    /// outer environment into the environment seen by the
    /// inner computation.
    ///
    /// # Arguments
    ///
    /// * `modifier` - A function that transforms the environment
    /// * `computation` - The computation to run with the modified environment
    ///
    /// # Returns
    ///
    /// A computation that runs with the modified environment.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{MonadReader, Reader};
    ///
    /// let reader = Reader::ask().map(|x: i32| x * 2);
    /// let modified = Reader::local(|x| x + 10, reader);
    /// assert_eq!(modified.run(5), 30); // (5 + 10) * 2
    /// ```
    fn local<F>(modifier: F, computation: Self) -> Self
    where
        F: FnOnce(R) -> R + 'static;

    /// Projects a value from the environment.
    ///
    /// This is a convenience method that combines `ask` with a projection
    /// function. It's equivalent to `ask().map(projection)` but may be
    /// more efficient in some implementations.
    ///
    /// # Arguments
    ///
    /// * `projection` - A function that extracts a value from the environment
    ///
    /// # Returns
    ///
    /// A computation that produces the projected value.
    ///
    /// # Type Parameters
    ///
    /// * `B` - The type of the projected value
    /// * `F` - The projection function type
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::effect::{MonadReader, Reader};
    ///
    /// struct Config { port: u16, host: String }
    ///
    /// let port_reader: Reader<Config, u16> = Reader::asks(|c| c.port);
    /// ```
    fn asks<B, F>(projection: F) -> Self::WithType<B>
    where
        F: FnOnce(R) -> B + 'static,
        R: Clone + 'static,
        B: 'static;
}

#[cfg(test)]
mod tests {
    // =========================================================================
    // Trait Definition Tests
    // =========================================================================

    #[test]
    fn monad_reader_trait_compiles() {
        // Just verify the trait module compiles and is accessible
        // The trait requires Monad as a supertrait
        use super::MonadReader;

        // This function signature proves the trait is properly defined
        fn _requires_monad_reader<R, M: MonadReader<R>>() {}

        // The test passes if this file compiles
    }
}
