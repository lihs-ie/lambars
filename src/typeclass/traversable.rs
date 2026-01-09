//! Traversable type class - mapping with effects and collecting results.
//!
//! This module provides the `Traversable` trait, which represents types that can
//! have an effectful function applied to each element while collecting the results
//! inside the effect.
//!
//! A `Traversable` is a combination of `Functor` and `Foldable` with the additional
//! ability to "turn the structure inside out" with respect to effects.
//!
//! # Motivation
//!
//! Consider a `Vec<String>` where you want to parse each string as an integer.
//! The parsing function returns `Option<i32>` (or `Result<i32, E>`). You want:
//! - If all parses succeed: `Some(Vec<i32>)` containing all results
//! - If any parse fails: `None` (or the first error)
//!
//! This is exactly what `traverse` does.
//!
//! # Limitations in Rust
//!
//! Rust lacks Higher-Kinded Types (HKT), which would allow us to define a single
//! generic `traverse` method for any `Applicative`. Instead, we provide specialized
//! methods for the most common effect types:
//!
//! - `traverse_option`: For functions returning `Option<B>`
//! - `traverse_result`: For functions returning `Result<B, E>`
//!
//! # Examples
//!
//! ```rust
//! use lambars::typeclass::Traversable;
//!
//! // Parse a vector of strings into integers
//! let strings = vec!["1", "2", "3"];
//! let numbers: Option<Vec<i32>> = strings.traverse_option(|s| s.parse().ok());
//! assert_eq!(numbers, Some(vec![1, 2, 3]));
//!
//! // If any parse fails, the whole result is None
//! let with_error = vec!["1", "not a number", "3"];
//! let result: Option<Vec<i32>> = with_error.traverse_option(|s| s.parse().ok());
//! assert_eq!(result, None);
//! ```

use super::foldable::Foldable;
use super::functor::Functor;
use super::higher::TypeConstructor;
use super::identity::Identity;

#[cfg(feature = "effect")]
use crate::effect::{IO, Reader, State};

#[cfg(all(feature = "effect", feature = "async"))]
use crate::effect::AsyncIO;

/// A trait for types that represent Reader-like computations.
///
/// This trait allows `sequence_reader` to extract environment and value types
/// from the inner elements of a traversable structure.
///
/// # Type Parameters
///
/// * `Environment` - The environment type (Reader's R)
/// * `Value` - The value type (Reader's A)
#[cfg(feature = "effect")]
pub trait ReaderLike {
    /// The environment type of the Reader.
    type Environment;
    /// The value type of the Reader.
    type Value;

    /// Converts this value into a Reader.
    ///
    /// This method is used by `sequence_reader` to convert elements
    /// into Reader values that can be traversed.
    fn into_reader(self) -> Reader<Self::Environment, Self::Value>
    where
        Self::Environment: Clone + 'static,
        Self::Value: 'static;
}

/// A trait for types that represent State-like computations.
///
/// This trait allows `sequence_state` to extract state and value types
/// from the inner elements of a traversable structure.
///
/// # Type Parameters
///
/// * `StateType` - The state type (State's S)
/// * `Value` - The value type (State's A)
#[cfg(feature = "effect")]
pub trait StateLike {
    /// The state type of the State computation.
    type StateType;
    /// The value type of the State computation.
    type Value;

    /// Converts this value into a State.
    ///
    /// This method is used by `sequence_state` to convert elements
    /// into State values that can be traversed.
    fn into_state(self) -> State<Self::StateType, Self::Value>
    where
        Self::StateType: Clone + 'static,
        Self::Value: 'static;
}

/// A trait for types that represent IO-like computations.
///
/// This trait allows `sequence_io` to extract the value type
/// from the inner elements of a traversable structure.
///
/// # Type Parameters
///
/// * `Value` - The value type (IO's A)
#[cfg(feature = "effect")]
pub trait IOLike {
    /// The value type of the IO computation.
    type Value;

    /// Converts this value into an IO.
    ///
    /// This method is used by `sequence_io` to convert elements
    /// into IO values that can be traversed.
    fn into_io(self) -> IO<Self::Value>
    where
        Self::Value: 'static;
}

/// A trait for types that represent AsyncIO-like computations.
///
/// This trait allows `sequence_async_io` to extract the value type
/// from the inner elements of a traversable structure.
///
/// # Type Parameters
///
/// * `Value` - The value type (`AsyncIO`'s A)
#[cfg(all(feature = "effect", feature = "async"))]
pub trait AsyncIOLike {
    /// The value type of the `AsyncIO` computation.
    type Value;

    /// Converts this value into an `AsyncIO`.
    ///
    /// This method is used by `sequence_async_io` to convert elements
    /// into `AsyncIO` values that can be traversed.
    fn into_async_io(self) -> AsyncIO<Self::Value>
    where
        Self::Value: Send + 'static;
}

/// A type class for structures that can be traversed with effects.
///
/// `Traversable` combines the capabilities of `Functor` and `Foldable` with
/// the ability to sequence effects. It allows you to apply an effectful
/// function to each element and collect all the effects together.
///
/// # Type Class Laws
///
/// Implementations should satisfy these laws (expressed informally since we
/// cannot directly express them without HKT):
///
/// ## Identity
///
/// Traversing with the identity effect is the same as mapping:
/// ```text
/// traverse(Identity) == fmap(Identity)  // conceptually
/// ```
///
/// ## Naturality
///
/// The result of traversing is preserved by natural transformations:
/// ```text
/// transform(traverse(f)) == traverse(transform . f)  // for natural transformation `transform`
/// ```
///
/// ## Composition
///
/// Traversing with composed effects is the same as composing traversals:
/// ```text
/// traverse(Compose . fmap(g) . f) == Compose . fmap(traverse(g)) . traverse(f)
/// ```
///
/// # Provided Methods
///
/// In addition to the required `traverse_option` and `traverse_result` methods,
/// this trait provides:
///
/// - `sequence_option`: Turn `F<Option<A>>` into `Option<F<A>>`
/// - `sequence_result`: Turn `F<Result<A, E>>` into `Result<F<A>, E>`
/// - `traverse_option_`: Traverse for effects only, discarding results
/// - `traverse_result_`: Traverse for effects only, discarding results
/// - `for_each_option`: Alias for `traverse_option_`
/// - `for_each_result`: Alias for `traverse_result_`
///
/// # Examples
///
/// ```rust
/// use lambars::typeclass::Traversable;
///
/// // Validate all elements in a vector
/// fn validate_positive(number: i32) -> Result<i32, &'static str> {
///     if number > 0 { Ok(number) } else { Err("must be positive") }
/// }
///
/// let valid = vec![1, 2, 3];
/// assert_eq!(valid.traverse_result(validate_positive), Ok(vec![1, 2, 3]));
///
/// let invalid = vec![1, -2, 3];
/// assert_eq!(invalid.traverse_result(validate_positive), Err("must be positive"));
/// ```
pub trait Traversable: Functor + Foldable {
    /// Applies a function returning `Option` to each element and collects the results.
    ///
    /// If all function applications return `Some`, the result is `Some` containing
    /// the collected values. If any application returns `None`, the entire result
    /// is `None`.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that transforms each element to an `Option<B>`
    ///
    /// # Returns
    ///
    /// `Option<Self::WithType<B>>` - `Some` if all elements succeed, `None` otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Traversable;
    ///
    /// // All succeed
    /// let values = vec!["1", "2", "3"];
    /// let result: Option<Vec<i32>> = values.traverse_option(|s| s.parse().ok());
    /// assert_eq!(result, Some(vec![1, 2, 3]));
    ///
    /// // One fails
    /// let values = vec!["1", "invalid", "3"];
    /// let result: Option<Vec<i32>> = values.traverse_option(|s| s.parse().ok());
    /// assert_eq!(result, None);
    /// ```
    fn traverse_option<B, F>(self, function: F) -> Option<Self::WithType<B>>
    where
        F: FnMut(Self::Inner) -> Option<B>;

    /// Applies a function returning `Result` to each element and collects the results.
    ///
    /// If all function applications return `Ok`, the result is `Ok` containing
    /// the collected values. If any application returns `Err`, the entire result
    /// is that `Err`.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that transforms each element to a `Result<B, E>`
    ///
    /// # Returns
    ///
    /// `Result<Self::WithType<B>, E>` - `Ok` if all elements succeed, `Err` otherwise
    ///
    /// # Errors
    ///
    /// Returns the first `Err` encountered when applying `function` to elements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Traversable;
    ///
    /// fn parse_positive(s: &str) -> Result<i32, &'static str> {
    ///     s.parse::<i32>()
    ///         .map_err(|_| "parse error")
    ///         .and_then(|n| if n > 0 { Ok(n) } else { Err("must be positive") })
    /// }
    ///
    /// let values = vec!["1", "2", "3"];
    /// let result: Result<Vec<i32>, _> = values.traverse_result(parse_positive);
    /// assert_eq!(result, Ok(vec![1, 2, 3]));
    /// ```
    fn traverse_result<B, E, F>(self, function: F) -> Result<Self::WithType<B>, E>
    where
        F: FnMut(Self::Inner) -> Result<B, E>;

    /// Turns a structure of `Option`s inside out.
    ///
    /// Converts `Self<Option<A>>` to `Option<Self<A>>`.
    ///
    /// This is equivalent to `traverse_option(|x| x)` but may be more efficient.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Traversable;
    ///
    /// // All Some values
    /// let values: Vec<Option<i32>> = vec![Some(1), Some(2), Some(3)];
    /// let result: Option<Vec<i32>> = values.sequence_option();
    /// assert_eq!(result, Some(vec![1, 2, 3]));
    ///
    /// // Contains a None
    /// let values: Vec<Option<i32>> = vec![Some(1), None, Some(3)];
    /// let result: Option<Vec<i32>> = values.sequence_option();
    /// assert_eq!(result, None);
    /// ```
    fn sequence_option(self) -> Option<Self::WithType<<Self::Inner as TypeConstructor>::Inner>>
    where
        Self: Sized,
        Self::Inner: TypeConstructor + Into<Option<<Self::Inner as TypeConstructor>::Inner>>,
    {
        self.traverse_option(Into::into)
    }

    /// Turns a structure of `Result`s inside out.
    ///
    /// Converts `Self<Result<A, E>>` to `Result<Self<A>, E>`.
    ///
    /// This is equivalent to `traverse_result(|x| x)` but may be more efficient.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Traversable;
    ///
    /// // All Ok values
    /// let values: Vec<Result<i32, &str>> = vec![Ok(1), Ok(2), Ok(3)];
    /// let result: Result<Vec<i32>, _> = values.sequence_result();
    /// assert_eq!(result, Ok(vec![1, 2, 3]));
    ///
    /// // Contains an Err
    /// let values: Vec<Result<i32, &str>> = vec![Ok(1), Err("error"), Ok(3)];
    /// let result: Result<Vec<i32>, _> = values.sequence_result();
    /// assert_eq!(result, Err("error"));
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the first `Err` value found in the structure.
    fn sequence_result<E>(
        self,
    ) -> Result<Self::WithType<<Self::Inner as TypeConstructor>::Inner>, E>
    where
        Self: Sized,
        Self::Inner: TypeConstructor + Into<Result<<Self::Inner as TypeConstructor>::Inner, E>>,
    {
        self.traverse_result(Into::into)
    }

    /// Applies an effectful function for its effects only, discarding results.
    ///
    /// This is useful when you want to perform side effects on each element
    /// but don't need to collect the results.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that performs an effect returning `Option<()>`
    ///
    /// # Returns
    ///
    /// `Option<()>` - `Some(())` if all effects succeed, `None` otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Traversable;
    /// use std::cell::RefCell;
    ///
    /// let log = RefCell::new(Vec::new());
    /// let values = vec![1, 2, 3];
    ///
    /// let result = values.traverse_option_(|element| {
    ///     if element > 0 {
    ///         log.borrow_mut().push(element);
    ///         Some(())
    ///     } else {
    ///         None
    ///     }
    /// });
    ///
    /// assert_eq!(result, Some(()));
    /// assert_eq!(*log.borrow(), vec![1, 2, 3]);
    /// ```
    fn traverse_option_<F>(self, function: F) -> Option<()>
    where
        F: FnMut(Self::Inner) -> Option<()>,
        Self: Sized,
    {
        self.traverse_option(function).map(|_| ())
    }

    /// Alias for `traverse_option_`.
    ///
    /// The naming follows Haskell's convention of `for_` / `forM_` for
    /// flipped traverse that discards results.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Traversable;
    ///
    /// let values = vec![1, 2, 3];
    /// let result = values.for_each_option(|element| {
    ///     if element > 0 { Some(()) } else { None }
    /// });
    /// assert_eq!(result, Some(()));
    /// ```
    fn for_each_option<F>(self, function: F) -> Option<()>
    where
        F: FnMut(Self::Inner) -> Option<()>,
        Self: Sized,
    {
        self.traverse_option_(function)
    }

    /// Applies an effectful function for its effects only, discarding results.
    ///
    /// This is useful when you want to perform side effects on each element
    /// but don't need to collect the results.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that performs an effect returning `Result<(), E>`
    ///
    /// # Returns
    ///
    /// `Result<(), E>` - `Ok(())` if all effects succeed, `Err(e)` otherwise
    ///
    /// # Errors
    ///
    /// Returns the first `Err` encountered when applying `function` to elements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Traversable;
    /// use std::cell::RefCell;
    ///
    /// let log = RefCell::new(Vec::new());
    /// let values = vec![1, 2, 3];
    ///
    /// let result: Result<(), &str> = values.traverse_result_(|element| {
    ///     if element > 0 {
    ///         log.borrow_mut().push(element);
    ///         Ok(())
    ///     } else {
    ///         Err("must be positive")
    ///     }
    /// });
    ///
    /// assert_eq!(result, Ok(()));
    /// assert_eq!(*log.borrow(), vec![1, 2, 3]);
    /// ```
    fn traverse_result_<E, F>(self, function: F) -> Result<(), E>
    where
        F: FnMut(Self::Inner) -> Result<(), E>,
        Self: Sized,
    {
        self.traverse_result(function).map(|_| ())
    }

    /// Alias for `traverse_result_`.
    ///
    /// # Errors
    ///
    /// Returns the first `Err` encountered when applying `function` to elements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Traversable;
    ///
    /// let values = vec![1, 2, 3];
    /// let result: Result<(), &str> = values.for_each_result(|element| {
    ///     if element > 0 { Ok(()) } else { Err("must be positive") }
    /// });
    /// assert_eq!(result, Ok(()));
    /// ```
    fn for_each_result<E, F>(self, function: F) -> Result<(), E>
    where
        F: FnMut(Self::Inner) -> Result<(), E>,
        Self: Sized,
    {
        self.traverse_result_(function)
    }

    /// Applies a function returning `Reader` to each element and collects the results.
    ///
    /// The Reader computations are combined, and all will receive the same environment
    /// when the resulting Reader is run.
    ///
    /// # Type Parameters
    ///
    /// * `R` - The environment type for the Reader
    /// * `B` - The result type of each Reader computation
    ///
    /// # Arguments
    ///
    /// * `function` - A function that transforms each element to a `Reader<R, B>`
    ///
    /// # Returns
    ///
    /// `Reader<R, Self::WithType<B>>` - A Reader that, when run with an environment,
    /// produces the structure with all transformed values.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Traversable;
    /// use lambars::effect::Reader;
    ///
    /// let values = vec![1, 2, 3];
    /// let reader = values.traverse_reader(|value| {
    ///     Reader::asks(move |multiplier: i32| value * multiplier)
    /// });
    /// assert_eq!(reader.run(10), vec![10, 20, 30]);
    /// ```
    #[cfg(feature = "effect")]
    fn traverse_reader<R, B, F>(self, function: F) -> Reader<R, Self::WithType<B>>
    where
        F: FnMut(Self::Inner) -> Reader<R, B>,
        R: Clone + 'static,
        B: 'static,
        Self::WithType<B>: 'static;

    /// Turns a structure of `Reader`s inside out.
    ///
    /// Converts `Self<Reader<R, A>>` to `Reader<R, Self<A>>`.
    ///
    /// This is equivalent to `traverse_reader(ReaderLike::into_reader)`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Traversable;
    /// use lambars::effect::Reader;
    ///
    /// let values: Vec<Reader<i32, i32>> = vec![
    ///     Reader::asks(|environment: i32| environment),
    ///     Reader::asks(|environment: i32| environment * 2),
    /// ];
    /// let reader = values.sequence_reader();
    /// assert_eq!(reader.run(5), vec![5, 10]);
    /// ```
    #[cfg(feature = "effect")]
    fn sequence_reader<R>(self) -> Reader<R, Self::WithType<<Self::Inner as ReaderLike>::Value>>
    where
        Self: Sized,
        R: Clone + 'static,
        Self::Inner: ReaderLike<Environment = R> + 'static,
        <Self::Inner as ReaderLike>::Value: 'static,
        Self::WithType<<Self::Inner as ReaderLike>::Value>: 'static,
    {
        self.traverse_reader(ReaderLike::into_reader)
    }

    /// Applies an effectful function for its effects only, discarding results.
    ///
    /// This is useful when you want to perform Reader effects on each element
    /// but don't need to collect the results.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that performs an effect returning `Reader<R, ()>`
    ///
    /// # Returns
    ///
    /// `Reader<R, ()>` - A Reader that performs all effects when run
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Traversable;
    /// use lambars::effect::Reader;
    ///
    /// let values = vec![1, 2, 3];
    /// let reader = values.traverse_reader_(|_| Reader::pure(()));
    /// assert_eq!(reader.run(0), ());
    /// ```
    #[cfg(feature = "effect")]
    fn traverse_reader_<R, F>(self, function: F) -> Reader<R, ()>
    where
        F: FnMut(Self::Inner) -> Reader<R, ()>,
        R: Clone + 'static,
        Self: Sized,
        Self::WithType<()>: 'static,
    {
        self.traverse_reader(function).fmap(|_| ())
    }

    /// Alias for `traverse_reader_`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Traversable;
    /// use lambars::effect::Reader;
    ///
    /// let values = vec![1, 2, 3];
    /// let reader = values.for_each_reader(|_| Reader::pure(()));
    /// assert_eq!(reader.run(0), ());
    /// ```
    #[cfg(feature = "effect")]
    fn for_each_reader<R, F>(self, function: F) -> Reader<R, ()>
    where
        F: FnMut(Self::Inner) -> Reader<R, ()>,
        R: Clone + 'static,
        Self: Sized,
        Self::WithType<()>: 'static,
    {
        self.traverse_reader_(function)
    }

    /// Applies a function returning `State` to each element and collects the results.
    ///
    /// The state is threaded from left to right through each element's computation.
    ///
    /// # Type Parameters
    ///
    /// * `S` - The state type for the State monad
    /// * `B` - The result type of each State computation
    ///
    /// # Arguments
    ///
    /// * `function` - A function that transforms each element to a `State<S, B>`
    ///
    /// # Returns
    ///
    /// `State<S, Self::WithType<B>>` - A State that, when run with an initial state,
    /// produces the structure with all transformed values and the final state.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Traversable;
    /// use lambars::effect::State;
    ///
    /// let items = vec!["a", "b", "c"];
    /// let state = items.traverse_state(|item| {
    ///     State::new(move |index: usize| ((index, item), index + 1))
    /// });
    /// let (result, final_index) = state.run(0);
    /// assert_eq!(result, vec![(0, "a"), (1, "b"), (2, "c")]);
    /// assert_eq!(final_index, 3);
    /// ```
    #[cfg(feature = "effect")]
    fn traverse_state<S, B, F>(self, function: F) -> State<S, Self::WithType<B>>
    where
        F: FnMut(Self::Inner) -> State<S, B>,
        S: Clone + 'static,
        B: 'static,
        Self::WithType<B>: 'static;

    /// Turns a structure of `State`s inside out.
    ///
    /// Converts `Self<State<S, A>>` to `State<S, Self<A>>`.
    ///
    /// This is equivalent to `traverse_state(StateLike::into_state)`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Traversable;
    /// use lambars::effect::State;
    ///
    /// let states: Vec<State<i32, i32>> = vec![
    ///     State::new(|state: i32| (state, state + 1)),
    ///     State::new(|state: i32| (state * 2, state + 1)),
    /// ];
    /// let combined = states.sequence_state();
    /// let (result, final_state) = combined.run(1);
    /// assert_eq!(result, vec![1, 4]);
    /// assert_eq!(final_state, 3);
    /// ```
    #[cfg(feature = "effect")]
    fn sequence_state<S>(self) -> State<S, Self::WithType<<Self::Inner as StateLike>::Value>>
    where
        Self: Sized,
        S: Clone + 'static,
        Self::Inner: StateLike<StateType = S> + 'static,
        <Self::Inner as StateLike>::Value: 'static,
        Self::WithType<<Self::Inner as StateLike>::Value>: 'static,
    {
        self.traverse_state(StateLike::into_state)
    }

    /// Applies an effectful function for its effects only, discarding results.
    ///
    /// This is useful when you want to perform State effects on each element
    /// but don't need to collect the results.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that performs an effect returning `State<S, ()>`
    ///
    /// # Returns
    ///
    /// `State<S, ()>` - A State that performs all effects when run
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Traversable;
    /// use lambars::effect::State;
    ///
    /// let values = vec![1, 2, 3];
    /// let state = values.traverse_state_(|value| {
    ///     State::modify(move |current: i32| current + value)
    /// });
    /// let ((), final_state) = state.run(0);
    /// assert_eq!(final_state, 6);
    /// ```
    #[cfg(feature = "effect")]
    fn traverse_state_<S, F>(self, function: F) -> State<S, ()>
    where
        F: FnMut(Self::Inner) -> State<S, ()>,
        S: Clone + 'static,
        Self: Sized,
        Self::WithType<()>: 'static,
    {
        self.traverse_state(function).fmap(|_| ())
    }

    /// Alias for `traverse_state_`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Traversable;
    /// use lambars::effect::State;
    ///
    /// let values = vec![1, 2, 3];
    /// let state = values.for_each_state(|value| {
    ///     State::modify(move |current: i32| current + value)
    /// });
    /// let ((), final_state) = state.run(0);
    /// assert_eq!(final_state, 6);
    /// ```
    #[cfg(feature = "effect")]
    fn for_each_state<S, F>(self, function: F) -> State<S, ()>
    where
        F: FnMut(Self::Inner) -> State<S, ()>,
        S: Clone + 'static,
        Self: Sized,
        Self::WithType<()>: 'static,
    {
        self.traverse_state_(function)
    }

    /// Applies a function returning `IO` to each element and collects the results.
    ///
    /// The IO actions are executed sequentially from left to right.
    ///
    /// # Type Parameters
    ///
    /// * `B` - The result type of each IO computation
    ///
    /// # Arguments
    ///
    /// * `function` - A function that transforms each element to an `IO<B>`
    ///
    /// # Returns
    ///
    /// `IO<Self::WithType<B>>` - An IO that, when executed, produces the structure
    /// with all transformed values.
    ///
    /// # Type Constraints (Result)
    ///
    /// When traversing `Result<T, E>`, the error type `E` must satisfy:
    /// - `E: Clone + Send + 'static` - The `Send` constraint is required by `traverse_async_io`,
    ///   and due to Rust's type system limitations, this constraint applies to the entire
    ///   `Traversable` implementation for `Result<T, E>`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Traversable;
    /// use lambars::effect::IO;
    ///
    /// let paths = vec!["a.txt", "b.txt"];
    /// let io = paths.traverse_io(|path| {
    ///     let path = path.to_string();
    ///     IO::new(move || format!("content of {}", path))
    /// });
    /// let contents = io.run_unsafe();
    /// assert_eq!(contents, vec!["content of a.txt".to_string(), "content of b.txt".to_string()]);
    /// ```
    #[cfg(feature = "effect")]
    fn traverse_io<B, F>(self, function: F) -> IO<Self::WithType<B>>
    where
        F: FnMut(Self::Inner) -> IO<B>,
        B: 'static,
        Self::WithType<B>: 'static;

    /// Turns a structure of `IO`s inside out.
    ///
    /// Converts `Self<IO<A>>` to `IO<Self<A>>`.
    ///
    /// This is equivalent to `traverse_io(IOLike::into_io)`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Traversable;
    /// use lambars::effect::IO;
    ///
    /// let ios: Vec<IO<i32>> = vec![IO::pure(1), IO::pure(2), IO::pure(3)];
    /// let combined = ios.sequence_io();
    /// let result = combined.run_unsafe();
    /// assert_eq!(result, vec![1, 2, 3]);
    /// ```
    #[cfg(feature = "effect")]
    fn sequence_io(self) -> IO<Self::WithType<<Self::Inner as IOLike>::Value>>
    where
        Self: Sized,
        Self::Inner: IOLike + 'static,
        <Self::Inner as IOLike>::Value: 'static,
        Self::WithType<<Self::Inner as IOLike>::Value>: 'static,
    {
        self.traverse_io(IOLike::into_io)
    }

    /// Applies an effectful function for its effects only, discarding results.
    ///
    /// This is useful when you want to perform IO effects on each element
    /// but don't need to collect the results.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that performs an effect returning `IO<()>`
    ///
    /// # Returns
    ///
    /// `IO<()>` - An IO that performs all effects when executed
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Traversable;
    /// use lambars::effect::IO;
    ///
    /// let values = vec![1, 2, 3];
    /// let io = values.traverse_io_(|_value| {
    ///     IO::new(|| {
    ///         // In real code, this would be a side effect
    ///     })
    /// });
    /// let () = io.run_unsafe();
    /// ```
    #[cfg(feature = "effect")]
    fn traverse_io_<F>(self, function: F) -> IO<()>
    where
        F: FnMut(Self::Inner) -> IO<()>,
        Self: Sized,
        Self::WithType<()>: 'static,
    {
        self.traverse_io(function).fmap(|_| ())
    }

    /// Alias for `traverse_io_`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Traversable;
    /// use lambars::effect::IO;
    ///
    /// let values = vec![1, 2, 3];
    /// let io = values.for_each_io(|_| IO::pure(()));
    /// io.run_unsafe();
    /// ```
    #[cfg(feature = "effect")]
    fn for_each_io<F>(self, function: F) -> IO<()>
    where
        F: FnMut(Self::Inner) -> IO<()>,
        Self: Sized,
        Self::WithType<()>: 'static,
    {
        self.traverse_io_(function)
    }

    /// Applies a function returning `AsyncIO` to each element and collects the results.
    ///
    /// The `AsyncIO` actions are executed sequentially from left to right.
    ///
    /// # Type Parameters
    ///
    /// * `B` - The result type of each `AsyncIO` computation
    ///
    /// # Arguments
    ///
    /// * `function` - A function that transforms each element to an `AsyncIO<B>`
    ///
    /// # Returns
    ///
    /// `AsyncIO<Self::WithType<B>>` - An `AsyncIO` that, when executed, produces the
    /// structure with all transformed values.
    ///
    /// # Type Constraints (Result)
    ///
    /// When traversing `Result<T, E>`, the error type `E` must satisfy:
    /// - `E: Clone + Send + 'static` - Required because the error is captured by the async
    ///   closure and may be moved between threads by the async runtime.
    ///   This `Send` constraint is the reason why the entire `Traversable` implementation
    ///   for `Result<T, E>` requires `E: Send`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::typeclass::Traversable;
    /// use lambars::effect::AsyncIO;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let urls = vec!["http://a.com", "http://b.com"];
    ///     let async_io = urls.traverse_async_io(|url| {
    ///         let url = url.to_string();
    ///         AsyncIO::new(move || async move { format!("response from {}", url) })
    ///     });
    ///     let responses = async_io.run_async().await;
    ///     assert_eq!(responses, vec!["response from http://a.com", "response from http://b.com"]);
    /// }
    /// ```
    #[cfg(all(feature = "effect", feature = "async"))]
    fn traverse_async_io<B, F>(self, function: F) -> AsyncIO<Self::WithType<B>>
    where
        F: FnMut(Self::Inner) -> AsyncIO<B>,
        B: Send + 'static,
        Self::WithType<B>: Send + 'static;

    /// Turns a structure of `AsyncIO`s inside out.
    ///
    /// Converts `Self<AsyncIO<A>>` to `AsyncIO<Self<A>>`.
    ///
    /// This is equivalent to `traverse_async_io(AsyncIOLike::into_async_io)`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::typeclass::Traversable;
    /// use lambars::effect::AsyncIO;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let async_ios: Vec<AsyncIO<i32>> = vec![
    ///         AsyncIO::pure(1),
    ///         AsyncIO::pure(2),
    ///         AsyncIO::pure(3),
    ///     ];
    ///     let combined = async_ios.sequence_async_io();
    ///     let result = combined.run_async().await;
    ///     assert_eq!(result, vec![1, 2, 3]);
    /// }
    /// ```
    #[cfg(all(feature = "effect", feature = "async"))]
    fn sequence_async_io(self) -> AsyncIO<Self::WithType<<Self::Inner as AsyncIOLike>::Value>>
    where
        Self: Sized,
        Self::Inner: AsyncIOLike + 'static,
        <Self::Inner as AsyncIOLike>::Value: Send + 'static,
        Self::WithType<<Self::Inner as AsyncIOLike>::Value>: Send + 'static,
    {
        self.traverse_async_io(AsyncIOLike::into_async_io)
    }

    /// Applies an effectful function for its effects only, discarding results.
    ///
    /// This is useful when you want to perform `AsyncIO` effects on each element
    /// but don't need to collect the results.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that performs an effect returning `AsyncIO<()>`
    ///
    /// # Returns
    ///
    /// `AsyncIO<()>` - An `AsyncIO` that performs all effects when executed
    #[cfg(all(feature = "effect", feature = "async"))]
    fn traverse_async_io_<F>(self, function: F) -> AsyncIO<()>
    where
        F: FnMut(Self::Inner) -> AsyncIO<()>,
        Self: Sized,
        Self::WithType<()>: Send + 'static,
    {
        self.traverse_async_io(function).fmap(|_| ())
    }

    /// Alias for `traverse_async_io_`.
    #[cfg(all(feature = "effect", feature = "async"))]
    fn for_each_async_io<F>(self, function: F) -> AsyncIO<()>
    where
        F: FnMut(Self::Inner) -> AsyncIO<()>,
        Self: Sized,
        Self::WithType<()>: Send + 'static,
    {
        self.traverse_async_io_(function)
    }

    /// Applies a function returning `AsyncIO` to each element and collects the results in parallel.
    ///
    /// All `AsyncIO` actions are spawned concurrently using `tokio::spawn`, and the results
    /// are collected in the original order (not completion order).
    ///
    /// # Type Parameters
    ///
    /// * `B` - The result type of each `AsyncIO` computation. Must be `Send + 'static`
    ///   because values are returned from spawned tasks.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that transforms each element to an `AsyncIO<B>`.
    ///   The `'static` bound is required because the function is captured in the `AsyncIO` closure.
    ///   `Send` is **not** required because the function is applied to all elements before spawning.
    ///
    /// # Returns
    ///
    /// `AsyncIO<Self::WithType<B>>` - An `AsyncIO` that, when executed, produces the
    /// structure with all transformed values in the original order.
    ///
    /// # Semantics
    ///
    /// - All `AsyncIO` tasks are started simultaneously
    /// - All tasks are awaited (no fail-fast on panic)
    /// - Results maintain input order (not completion order)
    /// - Side effect execution order is non-deterministic
    ///
    /// # Panics
    ///
    /// - If called outside of a tokio runtime, `run_async()` will panic
    /// - If any task panics, the panic is re-thrown after all tasks complete
    /// - Only the first panic is re-thrown; others are lost
    ///
    /// # Resource Usage Warning
    ///
    /// This method spawns one tokio task per element without any concurrency limit.
    /// For large input collections, this can lead to:
    /// - High memory usage (each task has its own stack)
    /// - Thread pool exhaustion
    /// - File descriptor limits being reached
    ///
    /// For bounded concurrency, consider using a semaphore or chunking the input manually.
    /// A bounded variant (`traverse_async_io_parallel_n`) is planned for future releases.
    ///
    /// # Type Constraints (Result)
    ///
    /// When traversing `Result<T, E>`, the error type `E` must satisfy:
    /// - `E: Clone + Send + 'static` - Required because the error is captured by the async
    ///   closure and may be moved between threads by the async runtime.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::typeclass::Traversable;
    /// use lambars::effect::AsyncIO;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let urls = vec!["http://a.com", "http://b.com", "http://c.com"];
    ///     let async_io = urls.traverse_async_io_parallel(|url| {
    ///         let url = url.to_string();
    ///         AsyncIO::new(move || async move { format!("response from {}", url) })
    ///     });
    ///     let responses = async_io.run_async().await;
    ///     // All URLs fetched in parallel; results in original order
    ///     assert_eq!(responses.len(), 3);
    /// }
    /// ```
    #[cfg(all(feature = "effect", feature = "async"))]
    fn traverse_async_io_parallel<B, F>(self, function: F) -> AsyncIO<Self::WithType<B>>
    where
        F: FnMut(Self::Inner) -> AsyncIO<B> + 'static,
        B: Send + 'static,
        Self::Inner: Send + 'static,
        Self::WithType<B>: Send + 'static;

    /// Turns a structure of `AsyncIO`s inside out, executing them in parallel.
    ///
    /// Converts `Self<AsyncIO<A>>` to `AsyncIO<Self<A>>`, running all `AsyncIO` actions
    /// concurrently.
    ///
    /// This is equivalent to `traverse_async_io_parallel(AsyncIOLike::into_async_io)`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use lambars::typeclass::Traversable;
    /// use lambars::effect::AsyncIO;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let async_ios: Vec<AsyncIO<i32>> = vec![
    ///         AsyncIO::pure(1),
    ///         AsyncIO::pure(2),
    ///         AsyncIO::pure(3),
    ///     ];
    ///     let combined = async_ios.sequence_async_io_parallel();
    ///     let result = combined.run_async().await;
    ///     assert_eq!(result, vec![1, 2, 3]);
    /// }
    /// ```
    #[cfg(all(feature = "effect", feature = "async"))]
    fn sequence_async_io_parallel(
        self,
    ) -> AsyncIO<Self::WithType<<Self::Inner as AsyncIOLike>::Value>>
    where
        Self: Sized,
        Self::Inner: AsyncIOLike + Send + 'static,
        <Self::Inner as AsyncIOLike>::Value: Send + 'static,
        Self::WithType<<Self::Inner as AsyncIOLike>::Value>: Send + 'static,
    {
        self.traverse_async_io_parallel(AsyncIOLike::into_async_io)
    }

    /// Applies an effectful function for its effects only, discarding results, in parallel.
    ///
    /// This is useful when you want to perform `AsyncIO` effects on each element
    /// concurrently but don't need to collect the results.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that performs an effect returning `AsyncIO<()>`
    ///
    /// # Returns
    ///
    /// `AsyncIO<()>` - An `AsyncIO` that performs all effects in parallel when executed
    #[cfg(all(feature = "effect", feature = "async"))]
    fn traverse_async_io_parallel_<F>(self, function: F) -> AsyncIO<()>
    where
        F: FnMut(Self::Inner) -> AsyncIO<()> + 'static,
        Self: Sized,
        Self::Inner: Send + 'static,
        Self::WithType<()>: Send + 'static,
    {
        self.traverse_async_io_parallel(function).fmap(|_| ())
    }

    /// Alias for `traverse_async_io_parallel_`.
    #[cfg(all(feature = "effect", feature = "async"))]
    fn for_each_async_io_parallel<F>(self, function: F) -> AsyncIO<()>
    where
        F: FnMut(Self::Inner) -> AsyncIO<()> + 'static,
        Self: Sized,
        Self::Inner: Send + 'static,
        Self::WithType<()>: Send + 'static,
    {
        self.traverse_async_io_parallel_(function)
    }
}

// =============================================================================
// Option<A> Implementation
// =============================================================================

impl<A> Traversable for Option<A> {
    fn traverse_option<B, F>(self, mut function: F) -> Option<Option<B>>
    where
        F: FnMut(A) -> Option<B>,
    {
        self.map_or_else(|| Some(None), |element| function(element).map(Some))
    }

    fn traverse_result<B, E, F>(self, mut function: F) -> Result<Option<B>, E>
    where
        F: FnMut(A) -> Result<B, E>,
    {
        self.map_or_else(|| Ok(None), |element| function(element).map(Some))
    }

    #[cfg(feature = "effect")]
    fn traverse_reader<R, B, F>(self, mut function: F) -> Reader<R, Option<B>>
    where
        F: FnMut(A) -> Reader<R, B>,
        R: Clone + 'static,
        B: 'static,
        Option<B>: 'static,
    {
        self.map_or_else(
            || Reader::new(|_| None),
            |element| function(element).fmap(Some),
        )
    }

    #[cfg(feature = "effect")]
    fn traverse_state<S, B, F>(self, mut function: F) -> State<S, Option<B>>
    where
        F: FnMut(A) -> State<S, B>,
        S: Clone + 'static,
        B: 'static,
        Option<B>: 'static,
    {
        self.map_or_else(
            || State::new(|state| (None, state)),
            |element| function(element).fmap(Some),
        )
    }

    #[cfg(feature = "effect")]
    fn traverse_io<B, F>(self, mut function: F) -> IO<Option<B>>
    where
        F: FnMut(A) -> IO<B>,
        B: 'static,
        Option<B>: 'static,
    {
        self.map_or_else(|| IO::pure(None), |element| function(element).fmap(Some))
    }

    #[cfg(all(feature = "effect", feature = "async"))]
    fn traverse_async_io<B, F>(self, mut function: F) -> AsyncIO<Option<B>>
    where
        F: FnMut(A) -> AsyncIO<B>,
        B: Send + 'static,
        Option<B>: Send + 'static,
    {
        self.map_or_else(
            || AsyncIO::pure(None),
            |element| function(element).fmap(Some),
        )
    }

    #[cfg(all(feature = "effect", feature = "async"))]
    fn traverse_async_io_parallel<B, F>(self, function: F) -> AsyncIO<Option<B>>
    where
        F: FnMut(A) -> AsyncIO<B> + 'static,
        B: Send + 'static,
        A: Send + 'static,
        Option<B>: Send + 'static,
    {
        self.traverse_async_io(function)
    }
}

// =============================================================================
// Result<T, E> Implementation
// =============================================================================

impl<T, E: Clone + Send + 'static> Traversable for Result<T, E> {
    fn traverse_option<B, F>(self, mut function: F) -> Option<Result<B, E>>
    where
        F: FnMut(T) -> Option<B>,
    {
        match self {
            Ok(element) => function(element).map(Ok),
            Err(error) => Some(Err(error)),
        }
    }

    fn traverse_result<B, E2, F>(self, mut function: F) -> Result<Result<B, E>, E2>
    where
        F: FnMut(T) -> Result<B, E2>,
    {
        match self {
            Ok(element) => function(element).map(Ok),
            Err(error) => Ok(Err(error)),
        }
    }

    #[cfg(feature = "effect")]
    fn traverse_reader<R, B, F>(self, mut function: F) -> Reader<R, Result<B, E>>
    where
        F: FnMut(T) -> Reader<R, B>,
        R: Clone + 'static,
        B: 'static,
        E: 'static,
    {
        match self {
            Ok(element) => function(element).fmap(Ok),
            Err(error) => Reader::new(move |_| Err(error.clone())),
        }
    }

    #[cfg(feature = "effect")]
    fn traverse_state<S, B, F>(self, mut function: F) -> State<S, Result<B, E>>
    where
        F: FnMut(T) -> State<S, B>,
        S: Clone + 'static,
        B: 'static,
        E: 'static,
    {
        match self {
            Ok(element) => function(element).fmap(Ok),
            Err(error) => State::new(move |state| (Err(error.clone()), state)),
        }
    }

    #[cfg(feature = "effect")]
    fn traverse_io<B, F>(self, mut function: F) -> IO<Result<B, E>>
    where
        F: FnMut(T) -> IO<B>,
        B: 'static,
    {
        match self {
            Ok(element) => function(element).fmap(Ok),
            Err(error) => IO::new(move || Err(error)),
        }
    }

    #[cfg(all(feature = "effect", feature = "async"))]
    fn traverse_async_io<B, F>(self, mut function: F) -> AsyncIO<Result<B, E>>
    where
        F: FnMut(T) -> AsyncIO<B>,
        B: Send + 'static,
    {
        match self {
            Ok(element) => function(element).fmap(Ok),
            Err(error) => AsyncIO::new(move || async move { Err(error) }),
        }
    }

    #[cfg(all(feature = "effect", feature = "async"))]
    fn traverse_async_io_parallel<B, F>(self, function: F) -> AsyncIO<Result<B, E>>
    where
        F: FnMut(T) -> AsyncIO<B> + 'static,
        B: Send + 'static,
        T: Send + 'static,
        Result<B, E>: Send + 'static,
    {
        self.traverse_async_io(function)
    }
}

// =============================================================================
// Vec<T> Implementation
// =============================================================================

impl<T> Traversable for Vec<T> {
    fn traverse_option<B, F>(self, mut function: F) -> Option<Vec<B>>
    where
        F: FnMut(T) -> Option<B>,
    {
        let mut result = Vec::with_capacity(self.len());
        for element in self {
            match function(element) {
                Some(value) => result.push(value),
                None => return None,
            }
        }
        Some(result)
    }

    fn traverse_result<B, E, F>(self, mut function: F) -> Result<Vec<B>, E>
    where
        F: FnMut(T) -> Result<B, E>,
    {
        let mut result = Vec::with_capacity(self.len());
        for element in self {
            match function(element) {
                Ok(value) => result.push(value),
                Err(error) => return Err(error),
            }
        }
        Ok(result)
    }

    #[cfg(feature = "effect")]
    fn traverse_reader<R, B, F>(self, mut function: F) -> Reader<R, Vec<B>>
    where
        F: FnMut(T) -> Reader<R, B>,
        R: Clone + 'static,
        B: 'static,
        Vec<B>: 'static,
    {
        let readers: Vec<Reader<R, B>> = self.into_iter().map(&mut function).collect();
        Reader::new(move |environment: R| {
            readers
                .iter()
                .map(|reader| reader.run(environment.clone()))
                .collect()
        })
    }

    #[cfg(feature = "effect")]
    fn traverse_state<S, B, F>(self, mut function: F) -> State<S, Vec<B>>
    where
        F: FnMut(T) -> State<S, B>,
        S: Clone + 'static,
        B: 'static,
        Vec<B>: 'static,
    {
        let capacity = self.len();
        if capacity == 0 {
            return State::new(|state| (Vec::new(), state));
        }

        let states: Vec<State<S, B>> = self.into_iter().map(&mut function).collect();
        State::new(move |initial_state: S| {
            let mut result = Vec::with_capacity(capacity);
            let mut current_state = initial_state;
            for state_computation in &states {
                let (value, new_state) = state_computation.run(current_state);
                result.push(value);
                current_state = new_state;
            }
            (result, current_state)
        })
    }

    #[cfg(feature = "effect")]
    fn traverse_io<B, F>(self, mut function: F) -> IO<Vec<B>>
    where
        F: FnMut(T) -> IO<B>,
        B: 'static,
        Vec<B>: 'static,
    {
        let capacity = self.len();
        let ios: Vec<IO<B>> = self.into_iter().map(&mut function).collect();

        IO::new(move || {
            let mut result = Vec::with_capacity(capacity);
            for io in ios {
                result.push(io.run_unsafe());
            }
            result
        })
    }

    #[cfg(all(feature = "effect", feature = "async"))]
    fn traverse_async_io<B, F>(self, mut function: F) -> AsyncIO<Vec<B>>
    where
        F: FnMut(T) -> AsyncIO<B>,
        B: Send + 'static,
        Vec<B>: Send + 'static,
    {
        let capacity = self.len();
        let async_ios: Vec<AsyncIO<B>> = self.into_iter().map(&mut function).collect();

        AsyncIO::new(move || async move {
            let mut result = Vec::with_capacity(capacity);
            for async_io in async_ios {
                result.push(async_io.run_async().await);
            }
            result
        })
    }

    #[cfg(all(feature = "effect", feature = "async"))]
    fn traverse_async_io_parallel<B, F>(self, mut function: F) -> AsyncIO<Vec<B>>
    where
        F: FnMut(T) -> AsyncIO<B> + 'static,
        B: Send + 'static,
        T: Send + 'static,
        Vec<B>: Send + 'static,
    {
        if self.is_empty() {
            return AsyncIO::pure(Vec::new());
        }

        // F doesn't need Send: apply function before spawning
        let async_ios: Vec<AsyncIO<B>> = self.into_iter().map(&mut function).collect();
        let capacity = async_ios.len();

        AsyncIO::new(move || async move {
            let handles: Vec<_> = async_ios
                .into_iter()
                .map(|async_io| tokio::spawn(async_io.run_async()))
                .collect();

            let mut results = Vec::with_capacity(capacity);
            let mut first_panic: Option<Box<dyn std::any::Any + Send>> = None;
            let mut first_cancellation: Option<tokio::task::JoinError> = None;

            for handle in handles {
                match handle.await {
                    Ok(value) => results.push(value),
                    Err(join_error) => {
                        if join_error.is_panic() {
                            if first_panic.is_none() {
                                first_panic = Some(join_error.into_panic());
                            }
                        } else if first_cancellation.is_none() {
                            first_cancellation = Some(join_error);
                        }
                    }
                }
            }

            // Panic takes priority over cancellation
            if let Some(panic_payload) = first_panic {
                std::panic::resume_unwind(panic_payload);
            }
            if let Some(join_error) = first_cancellation {
                panic!("Task was cancelled: {join_error}");
            }

            results
        })
    }
}

// =============================================================================
// Box<T> Implementation
// =============================================================================

impl<T> Traversable for Box<T> {
    fn traverse_option<B, F>(self, mut function: F) -> Option<Box<B>>
    where
        F: FnMut(T) -> Option<B>,
    {
        function(*self).map(Box::new)
    }

    fn traverse_result<B, E, F>(self, mut function: F) -> Result<Box<B>, E>
    where
        F: FnMut(T) -> Result<B, E>,
    {
        function(*self).map(Box::new)
    }

    #[cfg(feature = "effect")]
    fn traverse_reader<R, B, F>(self, mut function: F) -> Reader<R, Box<B>>
    where
        F: FnMut(T) -> Reader<R, B>,
        R: Clone + 'static,
        B: 'static,
        Box<B>: 'static,
    {
        function(*self).fmap(Box::new)
    }

    #[cfg(feature = "effect")]
    fn traverse_state<S, B, F>(self, mut function: F) -> State<S, Box<B>>
    where
        F: FnMut(T) -> State<S, B>,
        S: Clone + 'static,
        B: 'static,
        Box<B>: 'static,
    {
        function(*self).fmap(Box::new)
    }

    #[cfg(feature = "effect")]
    fn traverse_io<B, F>(self, mut function: F) -> IO<Box<B>>
    where
        F: FnMut(T) -> IO<B>,
        B: 'static,
        Box<B>: 'static,
    {
        function(*self).fmap(Box::new)
    }

    #[cfg(all(feature = "effect", feature = "async"))]
    fn traverse_async_io<B, F>(self, mut function: F) -> AsyncIO<Box<B>>
    where
        F: FnMut(T) -> AsyncIO<B>,
        B: Send + 'static,
        Box<B>: Send + 'static,
    {
        function(*self).fmap(Box::new)
    }

    #[cfg(all(feature = "effect", feature = "async"))]
    fn traverse_async_io_parallel<B, F>(self, function: F) -> AsyncIO<Box<B>>
    where
        F: FnMut(T) -> AsyncIO<B> + 'static,
        B: Send + 'static,
        T: Send + 'static,
        Box<B>: Send + 'static,
    {
        self.traverse_async_io(function)
    }
}

// =============================================================================
// Identity<A> Implementation
// =============================================================================

impl<A> Traversable for Identity<A> {
    fn traverse_option<B, F>(self, mut function: F) -> Option<Identity<B>>
    where
        F: FnMut(A) -> Option<B>,
    {
        function(self.0).map(Identity)
    }

    fn traverse_result<B, E, F>(self, mut function: F) -> Result<Identity<B>, E>
    where
        F: FnMut(A) -> Result<B, E>,
    {
        function(self.0).map(Identity)
    }

    #[cfg(feature = "effect")]
    fn traverse_reader<R, B, F>(self, mut function: F) -> Reader<R, Identity<B>>
    where
        F: FnMut(A) -> Reader<R, B>,
        R: Clone + 'static,
        B: 'static,
        Identity<B>: 'static,
    {
        function(self.0).fmap(Identity)
    }

    #[cfg(feature = "effect")]
    fn traverse_state<S, B, F>(self, mut function: F) -> State<S, Identity<B>>
    where
        F: FnMut(A) -> State<S, B>,
        S: Clone + 'static,
        B: 'static,
        Identity<B>: 'static,
    {
        function(self.0).fmap(Identity)
    }

    #[cfg(feature = "effect")]
    fn traverse_io<B, F>(self, mut function: F) -> IO<Identity<B>>
    where
        F: FnMut(A) -> IO<B>,
        B: 'static,
        Identity<B>: 'static,
    {
        function(self.0).fmap(Identity)
    }

    #[cfg(all(feature = "effect", feature = "async"))]
    fn traverse_async_io<B, F>(self, mut function: F) -> AsyncIO<Identity<B>>
    where
        F: FnMut(A) -> AsyncIO<B>,
        B: Send + 'static,
        Identity<B>: Send + 'static,
    {
        function(self.0).fmap(Identity)
    }

    #[cfg(all(feature = "effect", feature = "async"))]
    fn traverse_async_io_parallel<B, F>(self, function: F) -> AsyncIO<Identity<B>>
    where
        F: FnMut(A) -> AsyncIO<B> + 'static,
        B: Send + 'static,
        A: Send + 'static,
        Identity<B>: Send + 'static,
    {
        self.traverse_async_io(function)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // Helper Functions for Tests
    // =========================================================================

    fn parse_int(string: &str) -> Option<i32> {
        string.parse().ok()
    }

    fn parse_int_result(string: &str) -> Result<i32, &'static str> {
        string.parse().map_err(|_| "parse error")
    }

    fn validate_positive(number: i32) -> Result<i32, &'static str> {
        if number > 0 {
            Ok(number)
        } else {
            Err("must be positive")
        }
    }

    // =========================================================================
    // Option<A> Tests - traverse_option
    // =========================================================================

    #[rstest]
    fn option_traverse_option_some_to_some() {
        let value = Some("42");
        let result: Option<Option<i32>> = value.traverse_option(parse_int);
        assert_eq!(result, Some(Some(42)));
    }

    #[rstest]
    fn option_traverse_option_some_to_none() {
        let value = Some("not a number");
        let result: Option<Option<i32>> = value.traverse_option(parse_int);
        assert_eq!(result, None);
    }

    #[rstest]
    fn option_traverse_option_none() {
        let value: Option<&str> = None;
        let result: Option<Option<i32>> = value.traverse_option(parse_int);
        assert_eq!(result, Some(None));
    }

    // =========================================================================
    // Option<A> Tests - traverse_result
    // =========================================================================

    #[rstest]
    fn option_traverse_result_some_to_ok() {
        let value = Some("42");
        let result: Result<Option<i32>, _> = value.traverse_result(parse_int_result);
        assert_eq!(result, Ok(Some(42)));
    }

    #[rstest]
    fn option_traverse_result_some_to_err() {
        let value = Some("not a number");
        let result: Result<Option<i32>, _> = value.traverse_result(parse_int_result);
        assert_eq!(result, Err("parse error"));
    }

    #[rstest]
    fn option_traverse_result_none() {
        let value: Option<&str> = None;
        let result: Result<Option<i32>, &str> = value.traverse_result(parse_int_result);
        assert_eq!(result, Ok(None));
    }

    // =========================================================================
    // Result<T, E> Tests - traverse_option
    // =========================================================================

    #[rstest]
    fn result_traverse_option_ok_to_some() {
        let value: Result<&str, &str> = Ok("42");
        let result: Option<Result<i32, &str>> = value.traverse_option(parse_int);
        assert_eq!(result, Some(Ok(42)));
    }

    #[rstest]
    fn result_traverse_option_ok_to_none() {
        let value: Result<&str, &str> = Ok("not a number");
        let result: Option<Result<i32, &str>> = value.traverse_option(parse_int);
        assert_eq!(result, None);
    }

    #[rstest]
    fn result_traverse_option_err() {
        let value: Result<&str, &str> = Err("original error");
        let result: Option<Result<i32, &str>> = value.traverse_option(parse_int);
        assert_eq!(result, Some(Err("original error")));
    }

    // =========================================================================
    // Result<T, E> Tests - traverse_result
    // =========================================================================

    #[rstest]
    fn result_traverse_result_ok_to_ok() {
        let value: Result<i32, &str> = Ok(42);
        let result: Result<Result<i32, &str>, &str> = value.traverse_result(validate_positive);
        assert_eq!(result, Ok(Ok(42)));
    }

    #[rstest]
    fn result_traverse_result_ok_to_err() {
        let value: Result<i32, &str> = Ok(-5);
        let result: Result<Result<i32, &str>, &str> = value.traverse_result(validate_positive);
        assert_eq!(result, Err("must be positive"));
    }

    #[rstest]
    fn result_traverse_result_err() {
        let value: Result<i32, &str> = Err("original error");
        let result: Result<Result<i32, &str>, &str> = value.traverse_result(validate_positive);
        assert_eq!(result, Ok(Err("original error")));
    }

    // =========================================================================
    // Vec<A> Tests - traverse_option
    // =========================================================================

    #[rstest]
    fn vec_traverse_option_all_some() {
        let values = vec!["1", "2", "3"];
        let result: Option<Vec<i32>> = values.traverse_option(parse_int);
        assert_eq!(result, Some(vec![1, 2, 3]));
    }

    #[rstest]
    fn vec_traverse_option_with_none() {
        let values = vec!["1", "not a number", "3"];
        let result: Option<Vec<i32>> = values.traverse_option(parse_int);
        assert_eq!(result, None);
    }

    #[rstest]
    fn vec_traverse_option_first_fails() {
        let values = vec!["not a number", "2", "3"];
        let result: Option<Vec<i32>> = values.traverse_option(parse_int);
        assert_eq!(result, None);
    }

    #[rstest]
    fn vec_traverse_option_last_fails() {
        let values = vec!["1", "2", "not a number"];
        let result: Option<Vec<i32>> = values.traverse_option(parse_int);
        assert_eq!(result, None);
    }

    #[rstest]
    fn vec_traverse_option_empty() {
        let values: Vec<&str> = vec![];
        let result: Option<Vec<i32>> = values.traverse_option(parse_int);
        assert_eq!(result, Some(vec![]));
    }

    #[rstest]
    fn vec_traverse_option_single_some() {
        let values = vec!["42"];
        let result: Option<Vec<i32>> = values.traverse_option(parse_int);
        assert_eq!(result, Some(vec![42]));
    }

    #[rstest]
    fn vec_traverse_option_single_none() {
        let values = vec!["not a number"];
        let result: Option<Vec<i32>> = values.traverse_option(parse_int);
        assert_eq!(result, None);
    }

    // =========================================================================
    // Vec<A> Tests - traverse_result
    // =========================================================================

    #[rstest]
    fn vec_traverse_result_all_ok() {
        let values = vec![1, 2, 3];
        let result: Result<Vec<i32>, _> = values.traverse_result(validate_positive);
        assert_eq!(result, Ok(vec![1, 2, 3]));
    }

    #[rstest]
    fn vec_traverse_result_with_err() {
        let values = vec![1, -2, 3];
        let result: Result<Vec<i32>, _> = values.traverse_result(validate_positive);
        assert_eq!(result, Err("must be positive"));
    }

    #[rstest]
    fn vec_traverse_result_first_fails() {
        let values = vec![-1, 2, 3];
        let result: Result<Vec<i32>, _> = values.traverse_result(validate_positive);
        assert_eq!(result, Err("must be positive"));
    }

    #[rstest]
    fn vec_traverse_result_last_fails() {
        let values = vec![1, 2, -3];
        let result: Result<Vec<i32>, _> = values.traverse_result(validate_positive);
        assert_eq!(result, Err("must be positive"));
    }

    #[rstest]
    fn vec_traverse_result_empty() {
        let values: Vec<i32> = vec![];
        let result: Result<Vec<i32>, &str> = values.traverse_result(validate_positive);
        assert_eq!(result, Ok(vec![]));
    }

    // =========================================================================
    // Box<A> Tests
    // =========================================================================

    #[rstest]
    fn box_traverse_option_some() {
        let boxed = Box::new("42");
        let result: Option<Box<i32>> = boxed.traverse_option(parse_int);
        assert_eq!(result, Some(Box::new(42)));
    }

    #[rstest]
    fn box_traverse_option_none() {
        let boxed = Box::new("not a number");
        let result: Option<Box<i32>> = boxed.traverse_option(parse_int);
        assert_eq!(result, None);
    }

    #[rstest]
    fn box_traverse_result_ok() {
        let boxed = Box::new("42");
        let result: Result<Box<i32>, _> = boxed.traverse_result(parse_int_result);
        assert_eq!(result, Ok(Box::new(42)));
    }

    #[rstest]
    fn box_traverse_result_err() {
        let boxed = Box::new("not a number");
        let result: Result<Box<i32>, _> = boxed.traverse_result(parse_int_result);
        assert_eq!(result, Err("parse error"));
    }

    // =========================================================================
    // Identity<A> Tests
    // =========================================================================

    #[rstest]
    fn identity_traverse_option_some() {
        let wrapped = Identity::new("42");
        let result: Option<Identity<i32>> = wrapped.traverse_option(parse_int);
        assert_eq!(result, Some(Identity::new(42)));
    }

    #[rstest]
    fn identity_traverse_option_none() {
        let wrapped = Identity::new("not a number");
        let result: Option<Identity<i32>> = wrapped.traverse_option(parse_int);
        assert_eq!(result, None);
    }

    #[rstest]
    fn identity_traverse_result_ok() {
        let wrapped = Identity::new("42");
        let result: Result<Identity<i32>, _> = wrapped.traverse_result(parse_int_result);
        assert_eq!(result, Ok(Identity::new(42)));
    }

    #[rstest]
    fn identity_traverse_result_err() {
        let wrapped = Identity::new("not a number");
        let result: Result<Identity<i32>, _> = wrapped.traverse_result(parse_int_result);
        assert_eq!(result, Err("parse error"));
    }

    // =========================================================================
    // sequence_option Tests
    // =========================================================================

    #[rstest]
    fn vec_sequence_option_all_some() {
        let values: Vec<Option<i32>> = vec![Some(1), Some(2), Some(3)];
        let result: Option<Vec<i32>> = values.sequence_option();
        assert_eq!(result, Some(vec![1, 2, 3]));
    }

    #[rstest]
    fn vec_sequence_option_with_none() {
        let values: Vec<Option<i32>> = vec![Some(1), None, Some(3)];
        let result: Option<Vec<i32>> = values.sequence_option();
        assert_eq!(result, None);
    }

    #[rstest]
    fn vec_sequence_option_empty() {
        let values: Vec<Option<i32>> = vec![];
        let result: Option<Vec<i32>> = values.sequence_option();
        assert_eq!(result, Some(vec![]));
    }

    #[rstest]
    fn vec_sequence_option_all_none() {
        let values: Vec<Option<i32>> = vec![None, None, None];
        let result: Option<Vec<i32>> = values.sequence_option();
        assert_eq!(result, None);
    }

    #[rstest]
    fn option_sequence_option_some_some() {
        let value: Option<Option<i32>> = Some(Some(42));
        let result: Option<Option<i32>> = value.sequence_option();
        assert_eq!(result, Some(Some(42)));
    }

    #[rstest]
    fn option_sequence_option_some_none() {
        let value: Option<Option<i32>> = Some(None);
        let result: Option<Option<i32>> = value.sequence_option();
        assert_eq!(result, None);
    }

    #[rstest]
    fn option_sequence_option_none() {
        let value: Option<Option<i32>> = None;
        let result: Option<Option<i32>> = value.sequence_option();
        assert_eq!(result, Some(None));
    }

    // =========================================================================
    // sequence_result Tests
    // =========================================================================

    #[rstest]
    fn vec_sequence_result_all_ok() {
        let values: Vec<Result<i32, &str>> = vec![Ok(1), Ok(2), Ok(3)];
        let result: Result<Vec<i32>, _> = values.sequence_result();
        assert_eq!(result, Ok(vec![1, 2, 3]));
    }

    #[rstest]
    fn vec_sequence_result_with_err() {
        let values: Vec<Result<i32, &str>> = vec![Ok(1), Err("error"), Ok(3)];
        let result: Result<Vec<i32>, _> = values.sequence_result();
        assert_eq!(result, Err("error"));
    }

    #[rstest]
    fn vec_sequence_result_empty() {
        let values: Vec<Result<i32, &str>> = vec![];
        let result: Result<Vec<i32>, _> = values.sequence_result();
        assert_eq!(result, Ok(vec![]));
    }

    #[rstest]
    fn vec_sequence_result_first_error() {
        let values: Vec<Result<i32, &str>> = vec![Err("first error"), Ok(2), Err("second error")];
        let result: Result<Vec<i32>, _> = values.sequence_result();
        assert_eq!(result, Err("first error"));
    }

    // =========================================================================
    // traverse_option_ / for_each_option Tests
    // =========================================================================

    #[rstest]
    fn vec_traverse_option_underscore_all_some() {
        use std::cell::RefCell;

        let log = RefCell::new(Vec::new());
        let values = vec![1, 2, 3];

        let result = values.traverse_option_(|element| {
            log.borrow_mut().push(element);
            Some(())
        });

        assert_eq!(result, Some(()));
        assert_eq!(*log.borrow(), vec![1, 2, 3]);
    }

    #[rstest]
    fn vec_traverse_option_underscore_with_none() {
        use std::cell::RefCell;

        let log = RefCell::new(Vec::new());
        let values = vec![1, 2, 3];

        let result = values.traverse_option_(|element| {
            log.borrow_mut().push(element);
            if element == 2 { None } else { Some(()) }
        });

        assert_eq!(result, None);
        // Should stop at the first None, so only 1 and 2 are logged
        assert_eq!(*log.borrow(), vec![1, 2]);
    }

    #[rstest]
    fn vec_for_each_option_same_as_traverse_option_underscore() {
        use std::cell::RefCell;

        let log1 = RefCell::new(Vec::new());
        let log2 = RefCell::new(Vec::new());
        let values = vec![1, 2, 3];

        let result1 = values.clone().traverse_option_(|element| {
            log1.borrow_mut().push(element);
            Some(())
        });

        let result2 = values.for_each_option(|element| {
            log2.borrow_mut().push(element);
            Some(())
        });

        assert_eq!(result1, result2);
        assert_eq!(*log1.borrow(), *log2.borrow());
    }

    // =========================================================================
    // traverse_result_ / for_each_result Tests
    // =========================================================================

    #[rstest]
    fn vec_traverse_result_underscore_all_ok() {
        use std::cell::RefCell;

        let log = RefCell::new(Vec::new());
        let values = vec![1, 2, 3];

        let result: Result<(), &str> = values.traverse_result_(|element| {
            log.borrow_mut().push(element);
            Ok(())
        });

        assert_eq!(result, Ok(()));
        assert_eq!(*log.borrow(), vec![1, 2, 3]);
    }

    #[rstest]
    fn vec_traverse_result_underscore_with_err() {
        use std::cell::RefCell;

        let log = RefCell::new(Vec::new());
        let values = vec![1, 2, 3];

        let result: Result<(), &str> = values.traverse_result_(|element| {
            log.borrow_mut().push(element);
            if element == 2 {
                Err("error at 2")
            } else {
                Ok(())
            }
        });

        assert_eq!(result, Err("error at 2"));
        // Should stop at the first Err
        assert_eq!(*log.borrow(), vec![1, 2]);
    }

    #[rstest]
    fn vec_for_each_result_same_as_traverse_result_underscore() {
        use std::cell::RefCell;

        let log1 = RefCell::new(Vec::new());
        let log2 = RefCell::new(Vec::new());
        let values = vec![1, 2, 3];

        let result1: Result<(), &str> = values.clone().traverse_result_(|element| {
            log1.borrow_mut().push(element);
            Ok(())
        });

        let result2: Result<(), &str> = values.for_each_result(|element| {
            log2.borrow_mut().push(element);
            Ok(())
        });

        assert_eq!(result1, result2);
        assert_eq!(*log1.borrow(), *log2.borrow());
    }

    // =========================================================================
    // Complex/Integration Tests
    // =========================================================================

    #[rstest]
    fn complex_nested_traverse() {
        // Demonstrate traversing nested structures
        let values: Vec<Option<&str>> = vec![Some("1"), Some("2"), Some("3")];

        // First sequence the Options
        let sequenced: Option<Vec<&str>> = values.sequence_option();
        assert_eq!(sequenced, Some(vec!["1", "2", "3"]));

        // Then traverse to parse
        let parsed: Option<Option<Vec<i32>>> =
            sequenced.traverse_option(|strings| strings.traverse_option(|s| s.parse::<i32>().ok()));
        assert_eq!(parsed, Some(Some(vec![1, 2, 3])));
    }

    #[rstest]
    fn traverse_with_index() {
        use std::cell::RefCell;

        let index = RefCell::new(0);
        let values = vec!["a", "b", "c"];

        let result: Option<Vec<(usize, &str)>> = values.traverse_option(|element| {
            let current_index = *index.borrow();
            *index.borrow_mut() += 1;
            Some((current_index, element))
        });

        assert_eq!(result, Some(vec![(0, "a"), (1, "b"), (2, "c")]));
    }

    #[rstest]
    fn traverse_short_circuits_on_failure() {
        use std::cell::RefCell;

        let call_count = RefCell::new(0);
        let values = vec![1, 2, 3, 4, 5];

        let result: Option<Vec<i32>> = values.traverse_option(|element| {
            *call_count.borrow_mut() += 1;
            if element == 3 {
                None
            } else {
                Some(element * 2)
            }
        });

        assert_eq!(result, None);
        // Should have been called exactly 3 times (for 1, 2, and 3)
        assert_eq!(*call_count.borrow(), 3);
    }
}

// =============================================================================
// Property-Based Tests
// =============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_vec_traverse_option_identity(values in prop::collection::vec(any::<i32>(), 0..20)) {
            // traverse with Some should be equivalent to fmap with Identity
            let traversed: Option<Vec<i32>> = values.clone().traverse_option(Some);
            prop_assert_eq!(traversed, Some(values));
        }

        #[test]
        fn prop_vec_traverse_result_identity(values in prop::collection::vec(any::<i32>(), 0..20)) {
            // traverse with Ok should be equivalent to fmap with Identity
            let traversed: Result<Vec<i32>, ()> = values.clone().traverse_result(Ok::<_, ()>);
            prop_assert_eq!(traversed, Ok(values));
        }

        #[test]
        fn prop_option_traverse_option_identity(value in prop::option::of(any::<i32>())) {
            let traversed: Option<Option<i32>> = value.traverse_option(Some);
            prop_assert_eq!(traversed, Some(value));
        }

        #[test]
        fn prop_vec_sequence_option_all_some_equals_collect(values in prop::collection::vec(any::<i32>(), 0..20)) {
            let options: Vec<Option<i32>> = values.iter().copied().map(Some).collect();
            let sequenced: Option<Vec<i32>> = options.sequence_option();
            prop_assert_eq!(sequenced, Some(values));
        }

        #[test]
        fn prop_vec_traverse_preserves_length_on_success(values in prop::collection::vec(1i32..100, 0..20)) {
            // For positive numbers, validate_positive succeeds
            fn validate(number: i32) -> Option<i32> {
                if number > 0 { Some(number) } else { None }
            }

            let traversed: Option<Vec<i32>> = values.clone().traverse_option(validate);
            if let Some(result) = traversed {
                prop_assert_eq!(result.len(), values.len());
            }
        }

        #[test]
        fn prop_vec_traverse_option_none_means_at_least_one_none(
            values in prop::collection::vec(-10i32..10, 1..10)
        ) {
            fn to_option(number: i32) -> Option<i32> {
                if number >= 0 { Some(number) } else { None }
            }

            let traversed: Option<Vec<i32>> = values.clone().traverse_option(to_option);
            let has_negative = values.iter().any(|&element| element < 0);

            // If traversed is None, there must be at least one negative number
            if traversed.is_none() {
                prop_assert!(has_negative);
            }
            // If there's no negative number, traversed must be Some
            if !has_negative {
                prop_assert!(traversed.is_some());
            }
        }

        #[test]
        fn prop_empty_vec_traverse_always_succeeds(function_fails: bool) {
            let empty: Vec<i32> = vec![];

            let option_result: Option<Vec<i32>> = empty.clone().traverse_option(|_| {
                if function_fails { None } else { Some(0) }
            });
            prop_assert!(option_result.is_some());

            let result_result: Result<Vec<i32>, ()> = empty.traverse_result(|_| {
                if function_fails { Err(()) } else { Ok(0) }
            });
            prop_assert!(result_result.is_ok());
        }

        #[test]
        fn prop_identity_traverse_same_as_function(value: i32) {
            let wrapped = Identity::new(value);
            let traversed: Option<Identity<String>> =
                wrapped.traverse_option(|number| Some(number.to_string()));

            prop_assert_eq!(traversed, Some(Identity::new(value.to_string())));
        }
    }

    // =========================================================================
    // State Effect Tests
    // =========================================================================

    #[cfg(feature = "effect")]
    mod state_tests {
        use super::*;
        use crate::effect::State;
        use rstest::rstest;

        #[rstest]
        fn vec_traverse_state_threads_state_left_to_right() {
            let values = vec![1, 2, 3];
            let state = values.traverse_state(|value| {
                State::new(move |current_state: i32| (value + current_state, current_state + 1))
            });

            let (result, final_state) = state.run(0);
            // [1+0, 2+1, 3+2] = [1, 3, 5]
            assert_eq!(result, vec![1, 3, 5]);
            assert_eq!(final_state, 3);
        }

        #[rstest]
        fn vec_traverse_state_empty() {
            let values: Vec<i32> = vec![];
            let state = values.traverse_state(|value| {
                State::new(move |current_state: i32| (value, current_state + 1))
            });

            let (result, final_state) = state.run(0);
            assert_eq!(result, Vec::<i32>::new());
            assert_eq!(final_state, 0);
        }

        #[rstest]
        fn vec_traverse_state_indexing() {
            let items = vec!["a", "b", "c"];
            let state = items
                .traverse_state(|item| State::new(move |index: usize| ((index, item), index + 1)));

            let (result, final_index) = state.run(0);
            assert_eq!(result, vec![(0, "a"), (1, "b"), (2, "c")]);
            assert_eq!(final_index, 3);
        }

        #[rstest]
        fn option_traverse_state_some() {
            let value = Some(5);
            let state = value.traverse_state(|number| {
                State::new(move |current_state: i32| (number * 2, current_state + number))
            });

            let (result, final_state) = state.run(10);
            assert_eq!(result, Some(10));
            assert_eq!(final_state, 15);
        }

        #[rstest]
        fn option_traverse_state_none() {
            let value: Option<i32> = None;
            let state = value.traverse_state(|number| {
                State::new(move |current_state: i32| (number * 2, current_state + number))
            });

            let (result, final_state) = state.run(10);
            assert_eq!(result, None);
            assert_eq!(final_state, 10);
        }

        #[rstest]
        fn result_traverse_state_ok() {
            let value: Result<i32, &'static str> = Ok(5);
            let state = value.traverse_state(|number| {
                State::new(move |current_state: i32| (number * 2, current_state + number))
            });

            let (result, final_state) = state.run(10);
            assert_eq!(result, Ok(10));
            assert_eq!(final_state, 15);
        }

        #[rstest]
        fn result_traverse_state_err() {
            let value: Result<i32, &'static str> = Err("error");
            let state = value.traverse_state(|number| {
                State::new(move |current_state: i32| (number * 2, current_state + number))
            });

            let (result, final_state) = state.run(10);
            assert_eq!(result, Err("error"));
            assert_eq!(final_state, 10);
        }

        #[rstest]
        fn box_traverse_state() {
            let value = Box::new(42);
            let state = value.traverse_state(|number| {
                State::new(move |current_state: i32| (number * 2, current_state + 1))
            });

            let (result, final_state) = state.run(0);
            assert_eq!(*result, 84);
            assert_eq!(final_state, 1);
        }

        #[rstest]
        fn identity_traverse_state() {
            let value = Identity::new(42);
            let state = value.traverse_state(|number| {
                State::new(move |current_state: i32| (number * 2, current_state + 1))
            });

            let (result, final_state) = state.run(0);
            assert_eq!(result.0, 84);
            assert_eq!(final_state, 1);
        }

        #[rstest]
        fn vec_traverse_state_accumulator() {
            let values = vec![1, 2, 3, 4, 5];
            let state =
                values.traverse_state(|value| State::new(move |sum: i32| (value, sum + value)));

            let (result, total) = state.run(0);
            assert_eq!(result, vec![1, 2, 3, 4, 5]);
            assert_eq!(total, 15);
        }

        #[rstest]
        fn vec_sequence_state() {
            let states: Vec<State<i32, i32>> = vec![
                State::new(|state: i32| (state, state + 1)),
                State::new(|state: i32| (state * 2, state + 1)),
            ];
            let combined = states.sequence_state();
            let (result, final_state) = combined.run(1);
            // First state: result=1, new_state=2
            // Second state: result=4, new_state=3
            assert_eq!(result, vec![1, 4]);
            assert_eq!(final_state, 3);
        }

        #[rstest]
        fn vec_traverse_state_discard() {
            let values = vec![1, 2, 3];
            let state = values.traverse_state_(|value| State::modify(move |sum: i32| sum + value));

            let (result, total) = state.run(0);
            assert_eq!(result, ());
            assert_eq!(total, 6);
        }

        #[rstest]
        fn vec_for_each_state() {
            let values = vec![1, 2, 3];
            let state = values.for_each_state(|value| State::modify(move |sum: i32| sum + value));

            let (result, total) = state.run(0);
            assert_eq!(result, ());
            assert_eq!(total, 6);
        }
    }

    // =========================================================================
    // IO Effect Tests
    // =========================================================================

    #[cfg(feature = "effect")]
    mod io_tests {
        use super::*;
        use crate::effect::IO;
        use rstest::rstest;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        #[rstest]
        fn vec_traverse_io_sequential_execution() {
            let counter = Arc::new(AtomicUsize::new(0));
            let values = vec![1, 2, 3];

            let io = values.traverse_io(|value| {
                let counter_inner = counter.clone();
                IO::new(move || {
                    let previous = counter_inner.fetch_add(1, Ordering::SeqCst);
                    (previous, value)
                })
            });

            let result = io.run_unsafe();
            assert_eq!(result, vec![(0, 1), (1, 2), (2, 3)]);
        }

        #[rstest]
        fn vec_traverse_io_empty() {
            let values: Vec<i32> = vec![];
            let io = values.traverse_io(|value| IO::pure(value * 2));

            let result = io.run_unsafe();
            assert_eq!(result, Vec::<i32>::new());
        }

        #[rstest]
        fn option_traverse_io_some() {
            let value = Some(42);
            let io = value.traverse_io(|number| IO::pure(number * 2));

            let result = io.run_unsafe();
            assert_eq!(result, Some(84));
        }

        #[rstest]
        fn option_traverse_io_none() {
            let value: Option<i32> = None;
            let io = value.traverse_io(|number| IO::pure(number * 2));

            let result = io.run_unsafe();
            assert_eq!(result, None);
        }

        #[rstest]
        fn result_traverse_io_ok() {
            let value: Result<i32, &'static str> = Ok(42);
            let io = value.traverse_io(|number| IO::pure(number * 2));

            let result = io.run_unsafe();
            assert_eq!(result, Ok(84));
        }

        #[rstest]
        fn result_traverse_io_err() {
            let value: Result<i32, &'static str> = Err("error");
            let io = value.traverse_io(|number| IO::pure(number * 2));

            let result = io.run_unsafe();
            assert_eq!(result, Err("error"));
        }

        #[rstest]
        fn box_traverse_io() {
            let value = Box::new(42);
            let io = value.traverse_io(|number| IO::pure(number * 2));

            let result = io.run_unsafe();
            assert_eq!(*result, 84);
        }

        #[rstest]
        fn identity_traverse_io() {
            let value = Identity::new(42);
            let io = value.traverse_io(|number| IO::pure(number * 2));

            let result = io.run_unsafe();
            assert_eq!(result.0, 84);
        }

        #[rstest]
        fn vec_sequence_io() {
            let ios: Vec<IO<i32>> = vec![IO::pure(1), IO::pure(2), IO::pure(3)];
            let combined = ios.sequence_io();

            let result = combined.run_unsafe();
            assert_eq!(result, vec![1, 2, 3]);
        }

        #[rstest]
        fn vec_traverse_io_discard() {
            let counter = Arc::new(AtomicUsize::new(0));
            let values = vec![1, 2, 3];

            let counter_clone = counter.clone();
            let io = values.traverse_io_(move |_| {
                let counter_inner = counter_clone.clone();
                IO::new(move || {
                    counter_inner.fetch_add(1, Ordering::SeqCst);
                })
            });

            let () = io.run_unsafe();
            assert_eq!(counter.load(Ordering::SeqCst), 3);
        }

        #[rstest]
        fn vec_for_each_io() {
            let counter = Arc::new(AtomicUsize::new(0));
            let values = vec![1, 2, 3];

            let counter_clone = counter.clone();
            let io = values.for_each_io(move |_| {
                let counter_inner = counter_clone.clone();
                IO::new(move || {
                    counter_inner.fetch_add(1, Ordering::SeqCst);
                })
            });

            let () = io.run_unsafe();
            assert_eq!(counter.load(Ordering::SeqCst), 3);
        }
    }

    // =========================================================================
    // AsyncIO Effect Tests
    // =========================================================================

    #[cfg(all(feature = "effect", feature = "async"))]
    mod async_io_tests {
        use super::*;
        use crate::effect::AsyncIO;
        use rstest::rstest;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        #[rstest]
        #[tokio::test]
        async fn vec_traverse_async_io_sequential_execution() {
            let counter = Arc::new(AtomicUsize::new(0));
            let values = vec![1, 2, 3];

            let counter_clone = counter.clone();
            let async_io = values.traverse_async_io(move |value| {
                let counter_inner = counter_clone.clone();
                AsyncIO::new(move || async move {
                    let previous = counter_inner.fetch_add(1, Ordering::SeqCst);
                    (previous, value)
                })
            });

            let result = async_io.run_async().await;
            assert_eq!(result, vec![(0, 1), (1, 2), (2, 3)]);
        }

        #[rstest]
        #[tokio::test]
        async fn vec_traverse_async_io_empty() {
            let values: Vec<i32> = vec![];
            let async_io = values.traverse_async_io(|value| AsyncIO::pure(value * 2));

            let result = async_io.run_async().await;
            assert_eq!(result, Vec::<i32>::new());
        }

        #[rstest]
        #[tokio::test]
        async fn option_traverse_async_io_some() {
            let value = Some(42);
            let async_io = value.traverse_async_io(|number| AsyncIO::pure(number * 2));

            let result = async_io.run_async().await;
            assert_eq!(result, Some(84));
        }

        #[rstest]
        #[tokio::test]
        async fn option_traverse_async_io_none() {
            let value: Option<i32> = None;
            let async_io = value.traverse_async_io(|number| AsyncIO::pure(number * 2));

            let result = async_io.run_async().await;
            assert_eq!(result, None);
        }

        #[rstest]
        #[tokio::test]
        async fn result_traverse_async_io_ok() {
            let value: Result<i32, &'static str> = Ok(42);
            let async_io = value.traverse_async_io(|number| AsyncIO::pure(number * 2));

            let result = async_io.run_async().await;
            assert_eq!(result, Ok(84));
        }

        #[rstest]
        #[tokio::test]
        async fn result_traverse_async_io_err() {
            let value: Result<i32, &'static str> = Err("error");
            let async_io = value.traverse_async_io(|number| AsyncIO::pure(number * 2));

            let result = async_io.run_async().await;
            assert_eq!(result, Err("error"));
        }

        #[rstest]
        #[tokio::test]
        async fn box_traverse_async_io() {
            let value = Box::new(42);
            let async_io = value.traverse_async_io(|number| AsyncIO::pure(number * 2));

            let result = async_io.run_async().await;
            assert_eq!(*result, 84);
        }

        #[rstest]
        #[tokio::test]
        async fn identity_traverse_async_io() {
            let value = Identity::new(42);
            let async_io = value.traverse_async_io(|number| AsyncIO::pure(number * 2));

            let result = async_io.run_async().await;
            assert_eq!(result.0, 84);
        }

        #[rstest]
        #[tokio::test]
        async fn vec_sequence_async_io() {
            let async_ios: Vec<AsyncIO<i32>> =
                vec![AsyncIO::pure(1), AsyncIO::pure(2), AsyncIO::pure(3)];
            let combined = async_ios.sequence_async_io();

            let result = combined.run_async().await;
            assert_eq!(result, vec![1, 2, 3]);
        }

        #[rstest]
        #[tokio::test]
        async fn vec_traverse_async_io_discard() {
            let counter = Arc::new(AtomicUsize::new(0));
            let values = vec![1, 2, 3];

            let counter_clone = counter.clone();
            let async_io = values.traverse_async_io_(move |_| {
                let counter_inner = counter_clone.clone();
                AsyncIO::new(move || async move {
                    counter_inner.fetch_add(1, Ordering::SeqCst);
                })
            });

            async_io.run_async().await;
            assert_eq!(counter.load(Ordering::SeqCst), 3);
        }

        #[rstest]
        #[tokio::test]
        async fn vec_for_each_async_io() {
            let counter = Arc::new(AtomicUsize::new(0));
            let values = vec![1, 2, 3];

            let counter_clone = counter.clone();
            let async_io = values.for_each_async_io(move |_| {
                let counter_inner = counter_clone.clone();
                AsyncIO::new(move || async move {
                    counter_inner.fetch_add(1, Ordering::SeqCst);
                })
            });

            async_io.run_async().await;
            assert_eq!(counter.load(Ordering::SeqCst), 3);
        }

        // =========================================================================
        // traverse_async_io_parallel Tests
        // =========================================================================

        #[rstest]
        #[tokio::test]
        async fn vec_traverse_async_io_parallel_basic() {
            let values = vec![1, 2, 3];
            let async_io = values.traverse_async_io_parallel(|value| AsyncIO::pure(value * 2));

            let result = async_io.run_async().await;
            assert_eq!(result, vec![2, 4, 6]);
        }

        #[rstest]
        #[tokio::test]
        async fn vec_traverse_async_io_parallel_empty() {
            let values: Vec<i32> = vec![];
            let async_io = values.traverse_async_io_parallel(|value| AsyncIO::pure(value * 2));

            let result = async_io.run_async().await;
            assert_eq!(result, Vec::<i32>::new());
        }

        #[rstest]
        #[tokio::test]
        async fn vec_traverse_async_io_parallel_single_element() {
            let values = vec![42];
            let async_io = values.traverse_async_io_parallel(|value| AsyncIO::pure(value * 2));

            let result = async_io.run_async().await;
            assert_eq!(result, vec![84]);
        }

        #[rstest]
        #[tokio::test]
        async fn vec_traverse_async_io_parallel_identity() {
            let values = vec![1, 2, 3];
            let async_io = values.clone().traverse_async_io_parallel(AsyncIO::pure);

            let result = async_io.run_async().await;
            assert_eq!(result, values);
        }

        #[rstest]
        #[tokio::test(start_paused = true)]
        async fn vec_traverse_async_io_parallel_order_preservation() {
            use std::time::Duration;

            // Delays configured so later elements complete faster
            let delays = vec![100u64, 50, 10];
            let async_io = delays.traverse_async_io_parallel(|delay| {
                AsyncIO::new(move || async move {
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    delay
                })
            });

            let result = async_io.run_async().await;
            // Results maintain input order, not completion order
            assert_eq!(result, vec![100, 50, 10]);
        }

        #[rstest]
        #[tokio::test(start_paused = true)]
        async fn vec_traverse_async_io_parallel_is_parallel() {
            use std::time::Duration;

            let values = vec![1, 2, 3];

            let async_io = values.traverse_async_io_parallel(|value| {
                AsyncIO::new(move || async move {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    value
                })
            });

            let start = tokio::time::Instant::now();
            let _ = async_io.run_async().await;
            let elapsed = start.elapsed();

            // Parallel execution: ~100ms (not 300ms for sequential)
            assert!(elapsed < Duration::from_millis(150));
        }

        #[rstest]
        #[tokio::test]
        async fn vec_traverse_async_io_parallel_with_result_all_ok() {
            let inputs = vec!["1", "2", "3"];
            let async_io = inputs.traverse_async_io_parallel(|string| {
                let string = string.to_string();
                AsyncIO::new(move || async move {
                    string
                        .parse::<i32>()
                        .map_err(|_| format!("Failed to parse: {string}"))
                })
            });

            let results: Vec<Result<i32, String>> = async_io.run_async().await;
            assert!(results.iter().all(|result| result.is_ok()));
            assert_eq!(
                results
                    .into_iter()
                    .map(|result| result.unwrap())
                    .collect::<Vec<_>>(),
                vec![1, 2, 3]
            );
        }

        #[rstest]
        #[tokio::test]
        async fn vec_traverse_async_io_parallel_with_result_some_err() {
            let inputs = vec!["1", "invalid", "3"];
            let async_io = inputs.traverse_async_io_parallel(|string| {
                let string = string.to_string();
                AsyncIO::new(move || async move {
                    string
                        .parse::<i32>()
                        .map_err(|_| format!("Failed to parse: {string}"))
                })
            });

            let results: Vec<Result<i32, String>> = async_io.run_async().await;
            // All results are collected including errors
            assert!(results[0].is_ok());
            assert!(results[1].is_err());
            assert!(results[2].is_ok());
        }

        #[rstest]
        #[tokio::test]
        async fn option_traverse_async_io_parallel_some() {
            let value = Some(42);
            let async_io = value.traverse_async_io_parallel(|number| AsyncIO::pure(number * 2));

            let result = async_io.run_async().await;
            assert_eq!(result, Some(84));
        }

        #[rstest]
        #[tokio::test]
        async fn option_traverse_async_io_parallel_none() {
            let value: Option<i32> = None;
            let async_io = value.traverse_async_io_parallel(|number| AsyncIO::pure(number * 2));

            let result = async_io.run_async().await;
            assert_eq!(result, None);
        }

        #[rstest]
        #[tokio::test]
        async fn result_traverse_async_io_parallel_ok() {
            let value: Result<i32, &'static str> = Ok(42);
            let async_io = value.traverse_async_io_parallel(|number| AsyncIO::pure(number * 2));

            let result = async_io.run_async().await;
            assert_eq!(result, Ok(84));
        }

        #[rstest]
        #[tokio::test]
        async fn result_traverse_async_io_parallel_err() {
            let value: Result<i32, &'static str> = Err("error");
            let async_io = value.traverse_async_io_parallel(|number| AsyncIO::pure(number * 2));

            let result = async_io.run_async().await;
            assert_eq!(result, Err("error"));
        }

        #[rstest]
        #[tokio::test]
        async fn box_traverse_async_io_parallel() {
            let value = Box::new(42);
            let async_io = value.traverse_async_io_parallel(|number| AsyncIO::pure(number * 2));

            let result = async_io.run_async().await;
            assert_eq!(*result, 84);
        }

        #[rstest]
        #[tokio::test]
        async fn identity_traverse_async_io_parallel() {
            let value = Identity::new(42);
            let async_io = value.traverse_async_io_parallel(|number| AsyncIO::pure(number * 2));

            let result = async_io.run_async().await;
            assert_eq!(result.0, 84);
        }

        // =========================================================================
        // sequence_async_io_parallel Tests
        // =========================================================================

        #[rstest]
        #[tokio::test]
        async fn vec_sequence_async_io_parallel_basic() {
            let async_ios: Vec<AsyncIO<i32>> =
                vec![AsyncIO::pure(1), AsyncIO::pure(2), AsyncIO::pure(3)];
            let combined = async_ios.sequence_async_io_parallel();

            let result = combined.run_async().await;
            assert_eq!(result, vec![1, 2, 3]);
        }

        #[rstest]
        #[tokio::test]
        async fn vec_sequence_async_io_parallel_empty() {
            let async_ios: Vec<AsyncIO<i32>> = vec![];
            let combined = async_ios.sequence_async_io_parallel();

            let result = combined.run_async().await;
            assert_eq!(result, Vec::<i32>::new());
        }

        #[rstest]
        #[tokio::test(start_paused = true)]
        async fn vec_sequence_async_io_parallel_is_parallel() {
            use std::time::Duration;

            let async_ios: Vec<AsyncIO<i32>> = vec![
                AsyncIO::new(|| async {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    1
                }),
                AsyncIO::new(|| async {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    2
                }),
                AsyncIO::new(|| async {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    3
                }),
            ];

            let start = tokio::time::Instant::now();
            let result = async_ios.sequence_async_io_parallel().run_async().await;
            let elapsed = start.elapsed();

            assert_eq!(result, vec![1, 2, 3]);
            assert!(elapsed < Duration::from_millis(150));
        }

        // =========================================================================
        // traverse_async_io_parallel_ / for_each_async_io_parallel Tests
        // =========================================================================

        #[rstest]
        #[tokio::test]
        async fn vec_traverse_async_io_parallel_discard() {
            let counter = Arc::new(AtomicUsize::new(0));
            let values = vec![1, 2, 3];

            let counter_clone = counter.clone();
            let async_io = values.traverse_async_io_parallel_(move |_| {
                let counter_inner = counter_clone.clone();
                AsyncIO::new(move || async move {
                    counter_inner.fetch_add(1, Ordering::SeqCst);
                })
            });

            async_io.run_async().await;
            assert_eq!(counter.load(Ordering::SeqCst), 3);
        }

        #[rstest]
        #[tokio::test]
        async fn vec_for_each_async_io_parallel() {
            let counter = Arc::new(AtomicUsize::new(0));
            let values = vec![1, 2, 3];

            let counter_clone = counter.clone();
            let async_io = values.for_each_async_io_parallel(move |_| {
                let counter_inner = counter_clone.clone();
                AsyncIO::new(move || async move {
                    counter_inner.fetch_add(1, Ordering::SeqCst);
                })
            });

            async_io.run_async().await;
            assert_eq!(counter.load(Ordering::SeqCst), 3);
        }

        #[rstest]
        #[tokio::test(start_paused = true)]
        async fn vec_traverse_async_io_parallel_discard_is_parallel() {
            use std::time::Duration;

            let values = vec![1, 2, 3];

            let async_io = values.traverse_async_io_parallel_(|_| {
                AsyncIO::new(|| async {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                })
            });

            let start = tokio::time::Instant::now();
            async_io.run_async().await;
            let elapsed = start.elapsed();

            assert!(elapsed < Duration::from_millis(150));
        }

        // =========================================================================
        // Panic Handling Tests
        // =========================================================================

        #[rstest]
        #[tokio::test]
        #[should_panic(expected = "intentional panic")]
        async fn vec_traverse_async_io_parallel_panic_rethrow() {
            let values = vec![1, 2, 3];

            let async_io = values.traverse_async_io_parallel(|value| {
                AsyncIO::new(move || async move {
                    assert!(value != 2, "intentional panic");
                    value
                })
            });

            async_io.run_async().await;
        }

        #[rstest]
        fn vec_traverse_async_io_parallel_all_tasks_complete_before_panic() {
            use std::sync::atomic::AtomicBool;

            let completed_1 = Arc::new(AtomicBool::new(false));
            let completed_3 = Arc::new(AtomicBool::new(false));

            let values = vec![1, 2, 3];

            let completed_1_clone = completed_1.clone();
            let completed_3_clone = completed_3.clone();

            let async_io = values.traverse_async_io_parallel(move |value| {
                let completed_1_inner = completed_1_clone.clone();
                let completed_3_inner = completed_3_clone.clone();
                AsyncIO::new(move || async move {
                    match value {
                        1 => {
                            completed_1_inner.store(true, Ordering::SeqCst);
                            1
                        }
                        2 => {
                            panic!("intentional panic");
                        }
                        3 => {
                            completed_3_inner.store(true, Ordering::SeqCst);
                            3
                        }
                        _ => value,
                    }
                })
            });

            // Create a new runtime outside of any async context
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(async_io.run_async())
            }));

            assert!(result.is_err()); // Should have panicked
            // Both non-panicking tasks should have completed
            assert!(completed_1.load(Ordering::SeqCst));
            assert!(completed_3.load(Ordering::SeqCst));
        }
    }

    // =========================================================================
    // Reader Effect Tests (additional)
    // =========================================================================

    #[cfg(feature = "effect")]
    mod reader_additional_tests {
        use super::*;
        use crate::effect::Reader;
        use rstest::rstest;

        #[derive(Clone)]
        struct TestEnvironment {
            multiplier: i32,
        }

        #[rstest]
        fn vec_traverse_reader_all_elements() {
            let values = vec![1, 2, 3];
            let reader = values.traverse_reader(|number| {
                Reader::asks(move |environment: TestEnvironment| number * environment.multiplier)
            });

            let environment = TestEnvironment { multiplier: 10 };
            let result = reader.run(environment);
            assert_eq!(result, vec![10, 20, 30]);
        }

        #[rstest]
        fn option_traverse_reader_some() {
            let value = Some(5);
            let reader = value.traverse_reader(|number| {
                Reader::asks(move |environment: TestEnvironment| number * environment.multiplier)
            });

            let environment = TestEnvironment { multiplier: 10 };
            let result = reader.run(environment);
            assert_eq!(result, Some(50));
        }

        #[rstest]
        fn option_traverse_reader_none() {
            let value: Option<i32> = None;
            let reader = value.traverse_reader(|number| {
                Reader::asks(move |environment: TestEnvironment| number * environment.multiplier)
            });

            let environment = TestEnvironment { multiplier: 10 };
            let result = reader.run(environment);
            assert_eq!(result, None);
        }

        #[rstest]
        fn result_traverse_reader_ok() {
            let value: Result<i32, &'static str> = Ok(5);
            let reader = value.traverse_reader(|number| {
                Reader::asks(move |environment: TestEnvironment| number * environment.multiplier)
            });

            let environment = TestEnvironment { multiplier: 10 };
            let result = reader.run(environment);
            assert_eq!(result, Ok(50));
        }

        #[rstest]
        fn result_traverse_reader_err() {
            let value: Result<i32, &'static str> = Err("error");
            let reader = value.traverse_reader(|number| {
                Reader::asks(move |environment: TestEnvironment| number * environment.multiplier)
            });

            let environment = TestEnvironment { multiplier: 10 };
            let result = reader.run(environment);
            assert_eq!(result, Err("error"));
        }

        #[rstest]
        fn vec_sequence_reader() {
            let readers: Vec<Reader<i32, i32>> = vec![
                Reader::asks(|environment: i32| environment),
                Reader::asks(|environment: i32| environment * 2),
            ];
            let combined = readers.sequence_reader();
            let result = combined.run(5);
            assert_eq!(result, vec![5, 10]);
        }

        #[rstest]
        fn vec_traverse_reader_empty() {
            let values: Vec<i32> = vec![];
            let reader = values.traverse_reader(|number| {
                Reader::asks(move |environment: TestEnvironment| number * environment.multiplier)
            });

            let environment = TestEnvironment { multiplier: 10 };
            let result = reader.run(environment);
            assert_eq!(result, Vec::<i32>::new());
        }

        #[rstest]
        fn vec_traverse_reader_discard() {
            let values = vec![1, 2, 3];
            let reader = values.traverse_reader_(|_| Reader::pure(()));

            assert_eq!(reader.run(0), ());
        }

        #[rstest]
        fn vec_for_each_reader() {
            let values = vec![1, 2, 3];
            let reader = values.for_each_reader(|_| Reader::pure(()));

            assert_eq!(reader.run(0), ());
        }
    }
}
