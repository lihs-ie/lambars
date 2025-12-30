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
//! use functional_rusty::typeclass::Traversable;
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
/// use functional_rusty::typeclass::Traversable;
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
    /// use functional_rusty::typeclass::Traversable;
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
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::typeclass::Traversable;
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
    /// use functional_rusty::typeclass::Traversable;
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
    /// use functional_rusty::typeclass::Traversable;
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
    /// use functional_rusty::typeclass::Traversable;
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
    /// use functional_rusty::typeclass::Traversable;
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
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::typeclass::Traversable;
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
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::typeclass::Traversable;
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
}

// =============================================================================
// Option<A> Implementation
// =============================================================================

impl<A> Traversable for Option<A> {
    fn traverse_option<B, F>(self, mut function: F) -> Option<Option<B>>
    where
        F: FnMut(A) -> Option<B>,
    {
        match self {
            Some(element) => function(element).map(Some),
            None => Some(None),
        }
    }

    fn traverse_result<B, E, F>(self, mut function: F) -> Result<Option<B>, E>
    where
        F: FnMut(A) -> Result<B, E>,
    {
        match self {
            Some(element) => function(element).map(Some),
            None => Ok(None),
        }
    }
}

// =============================================================================
// Result<T, E> Implementation
// =============================================================================

impl<T, E: Clone> Traversable for Result<T, E> {
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
}

// =============================================================================
// Vec<A> Implementation
// =============================================================================

impl<A> Traversable for Vec<A> {
    fn traverse_option<B, F>(self, mut function: F) -> Option<Vec<B>>
    where
        F: FnMut(A) -> Option<B>,
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
        F: FnMut(A) -> Result<B, E>,
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
}

// =============================================================================
// Box<A> Implementation
// =============================================================================

impl<A> Traversable for Box<A> {
    fn traverse_option<B, F>(self, mut function: F) -> Option<Box<B>>
    where
        F: FnMut(A) -> Option<B>,
    {
        function(*self).map(Box::new)
    }

    fn traverse_result<B, E, F>(self, mut function: F) -> Result<Box<B>, E>
    where
        F: FnMut(A) -> Result<B, E>,
    {
        function(*self).map(Box::new)
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
            let traversed: Result<Vec<i32>, ()> = values.clone().traverse_result(|element| Ok::<_, ()>(element));
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
            fn transform(number: i32) -> Option<String> {
                Some(number.to_string())
            }

            let wrapped = Identity::new(value);
            let traversed: Option<Identity<String>> = wrapped.traverse_option(transform);

            prop_assert_eq!(traversed, Some(Identity::new(value.to_string())));
        }
    }
}
