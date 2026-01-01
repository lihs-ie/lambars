//! Monad type class - sequencing computations within a context.
//!
//! This module provides the `Monad` trait, which extends `Applicative` with
//! the ability to sequence computations where each step can depend on the
//! result of the previous step.
//!
//! A `Monad` is one of the most powerful abstractions in functional programming,
//! often described as a "programmable semicolon" because it controls how
//! computations are sequenced.
//!
//! # Laws
//!
//! All `Monad` implementations must satisfy these laws:
//!
//! ## Left Identity Law
//!
//! Lifting a pure value and binding a function is the same as applying the function:
//!
//! ```text
//! Self::pure(a).flat_map(f) == f(a)
//! ```
//!
//! ## Right Identity Law
//!
//! Binding `pure` to a monad returns the original monad:
//!
//! ```text
//! m.flat_map(Self::pure) == m
//! ```
//!
//! ## Associativity Law
//!
//! The order of binding operations can be reassociated:
//!
//! ```text
//! m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
//! ```
//!
//! # Examples
//!
//! ```rust
//! use lambars::typeclass::{Monad, Applicative};
//!
//! // Using flat_map to chain Option computations
//! let x = Some(5);
//! let y = x.flat_map(|n| if n > 0 { Some(n * 2) } else { None });
//! assert_eq!(y, Some(10));
//!
//! // Chain of computations with potential failure
//! fn parse_positive(s: &str) -> Option<i32> {
//!     s.parse::<i32>().ok().filter(|&n| n > 0)
//! }
//!
//! let result = Some("42")
//!     .flat_map(parse_positive)
//!     .flat_map(|n| Some(n * 2));
//! assert_eq!(result, Some(84));
//! ```

use super::applicative::Applicative;
use super::identity::Identity;

/// A type class for types that support sequencing of computations.
///
/// `Monad` extends `Applicative` with `flat_map`, which allows the result
/// of one computation to determine what computation to perform next.
/// This enables powerful control flow patterns within the monad context.
///
/// # Laws
///
/// ## Left Identity Law
///
/// Applying `pure` then `flat_map` with a function equals applying the function directly:
///
/// ```text
/// Self::pure(a).flat_map(f) == f(a)
/// ```
///
/// ## Right Identity Law
///
/// Binding with `pure` returns the original monad:
///
/// ```text
/// m.flat_map(Self::pure) == m
/// ```
///
/// ## Associativity Law
///
/// Binding operations can be reassociated:
///
/// ```text
/// m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
/// ```
///
/// # Examples
///
/// ```rust
/// use lambars::typeclass::{Monad, Applicative};
///
/// let x = Some(5);
/// let y = x.flat_map(|n| Some(n * 2));
/// assert_eq!(y, Some(10));
///
/// // Chaining with potential failure
/// let z = Some(10).flat_map(|n| {
///     if n > 0 {
///         Some(n / 2)
///     } else {
///         None
///     }
/// });
/// assert_eq!(z, Some(5));
/// ```
pub trait Monad: Applicative {
    /// Applies a function to the value inside the monad and flattens the result.
    ///
    /// This is the fundamental operation of the Monad type class. It takes a
    /// function that returns a new monad and "flattens" the nested result.
    ///
    /// In Haskell, this is `>>=` (bind). In Rust's standard library, this is
    /// similar to `and_then` on `Option` and `Result`.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes the inner value and returns a new monad
    ///
    /// # Returns
    ///
    /// A new monad with the result of applying the function
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Monad;
    ///
    /// let x = Some(5);
    /// let y = x.flat_map(|n| Some(n * 2));
    /// assert_eq!(y, Some(10));
    ///
    /// let z = Some(5);
    /// let w = z.flat_map(|n| if n > 10 { Some(n) } else { None });
    /// assert_eq!(w, None);
    /// ```
    fn flat_map<B, F>(self, function: F) -> Self::WithType<B>
    where
        F: FnOnce(Self::Inner) -> Self::WithType<B>;

    /// Alias for `flat_map` to match Rust's naming conventions.
    ///
    /// This method is provided for familiarity with Rust's `Option::and_then`
    /// and `Result::and_then` methods.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that takes the inner value and returns a new monad
    ///
    /// # Returns
    ///
    /// A new monad with the result of applying the function
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Monad;
    ///
    /// let x = Some(5);
    /// let y = x.and_then(|n| Some(n * 2));
    /// assert_eq!(y, Some(10));
    /// ```
    #[inline]
    fn and_then<B, F>(self, function: F) -> Self::WithType<B>
    where
        Self: Sized,
        F: FnOnce(Self::Inner) -> Self::WithType<B>,
    {
        self.flat_map(function)
    }

    /// Sequences two monadic computations, discarding the first result.
    ///
    /// This evaluates `self`, ignores its value, and returns `next`.
    /// In Haskell, this is the `>>` operator.
    ///
    /// Note: If `self` represents a failure (e.g., `None` or `Err`),
    /// the failure propagates and `next` is not returned.
    ///
    /// # Arguments
    ///
    /// * `next` - The monad to return after evaluating `self`
    ///
    /// # Returns
    ///
    /// The `next` monad if `self` succeeds, otherwise propagates failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Monad;
    ///
    /// let x = Some(5);
    /// let y = x.then(Some("hello"));
    /// assert_eq!(y, Some("hello"));
    ///
    /// let z: Option<i32> = None;
    /// let w = z.then(Some("hello"));
    /// assert_eq!(w, None);
    /// ```
    #[inline]
    fn then<B>(self, next: Self::WithType<B>) -> Self::WithType<B>
    where
        Self: Sized,
    {
        self.flat_map(|_| next)
    }
}

// =============================================================================
// Option<A> Implementation
// =============================================================================

impl<A> Monad for Option<A> {
    #[inline]
    fn flat_map<B, F>(self, function: F) -> Option<B>
    where
        F: FnOnce(A) -> Option<B>,
    {
        // Delegate to Option's built-in and_then
        Self::and_then(self, function)
    }
}

// =============================================================================
// Result<T, E> Implementation
// =============================================================================

impl<T, E: Clone> Monad for Result<T, E> {
    #[inline]
    fn flat_map<B, F>(self, function: F) -> Result<B, E>
    where
        F: FnOnce(T) -> Result<B, E>,
    {
        // Delegate to Result's built-in and_then
        Self::and_then(self, function)
    }
}

// =============================================================================
// Vec<A> Implementation
//
// Note: Vec requires FnMut for flat_map because the function needs to be
// called for each element. This is expressed through a separate trait to
// maintain the Monad interface clean with FnOnce.
// =============================================================================

/// Extension trait for Vec to provide Monad-like operations.
///
/// Vec's Monad instance represents non-deterministic computation:
/// `flat_map` applies a function to each element and concatenates all results.
///
/// This trait requires `FnMut` because the function needs to be called
/// for each element in the Vec.
///
/// # Examples
///
/// ```rust
/// use lambars::typeclass::MonadVec;
///
/// let numbers = vec![1, 2, 3];
/// let result = numbers.flat_map(|n| vec![n, n * 10]);
/// assert_eq!(result, vec![1, 10, 2, 20, 3, 30]);
/// ```
pub trait MonadVec: Sized {
    /// The inner type of the Vec.
    type VecInner;

    /// Applies a function to each element and flattens the results.
    ///
    /// This is the list monad's bind operation, representing non-deterministic
    /// computation where each element can produce multiple results.
    ///
    /// # Arguments
    ///
    /// * `function` - A function that returns a Vec for each element
    ///
    /// # Returns
    ///
    /// A Vec containing all results concatenated
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::MonadVec;
    ///
    /// let v = vec![1, 2, 3];
    /// let result = v.flat_map(|n| vec![n, n * 10]);
    /// assert_eq!(result, vec![1, 10, 2, 20, 3, 30]);
    /// ```
    fn flat_map<B, F>(self, function: F) -> Vec<B>
    where
        F: FnMut(Self::VecInner) -> Vec<B>;

    /// Alias for `flat_map` to match Rust's naming conventions.
    #[inline]
    fn and_then<B, F>(self, function: F) -> Vec<B>
    where
        F: FnMut(Self::VecInner) -> Vec<B>,
    {
        self.flat_map(function)
    }

    /// Sequences two Vec computations, discarding the first results.
    ///
    /// For each element in `self`, the entire `next` Vec is included.
    /// This produces `self.len() * next.len()` elements.
    fn then<B: Clone>(self, next: Vec<B>) -> Vec<B>;

    /// Flattens a nested Vec one level.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::MonadVec;
    ///
    /// let nested = vec![vec![1, 2], vec![3, 4]];
    /// let flat: Vec<i32> = nested.flatten();
    /// assert_eq!(flat, vec![1, 2, 3, 4]);
    /// ```
    fn flatten<B>(self) -> Vec<B>
    where
        Self::VecInner: IntoIterator<Item = B>;
}

impl<A> MonadVec for Vec<A> {
    type VecInner = A;

    #[inline]
    fn flat_map<B, F>(self, function: F) -> Vec<B>
    where
        F: FnMut(A) -> Vec<B>,
    {
        self.into_iter().flat_map(function).collect()
    }

    #[inline]
    fn then<B: Clone>(self, next: Vec<B>) -> Vec<B> {
        let length = self.len();
        let next_length = next.len();
        let capacity = length.saturating_mul(next_length);
        let mut result = Vec::with_capacity(capacity);
        for _ in self {
            result.extend(next.iter().cloned());
        }
        result
    }

    fn flatten<B>(self) -> Vec<B>
    where
        A: IntoIterator<Item = B>,
    {
        self.into_iter().flat_map(IntoIterator::into_iter).collect()
    }
}

// =============================================================================
// Box<A> Implementation
// =============================================================================

impl<A> Monad for Box<A> {
    #[inline]
    fn flat_map<B, F>(self, function: F) -> Box<B>
    where
        F: FnOnce(A) -> Box<B>,
    {
        function(*self)
    }
}

// =============================================================================
// Identity<A> Implementation
// =============================================================================

impl<A> Monad for Identity<A> {
    #[inline]
    fn flat_map<B, F>(self, function: F) -> Identity<B>
    where
        F: FnOnce(A) -> Identity<B>,
    {
        function(self.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typeclass::Applicative;
    use rstest::rstest;

    // =========================================================================
    // Option<A> Tests
    // =========================================================================

    #[rstest]
    fn option_flat_map_some_to_some() {
        let x = Some(5);
        let y = x.flat_map(|n| Some(n * 2));
        assert_eq!(y, Some(10));
    }

    #[rstest]
    fn option_flat_map_some_to_none() {
        let x = Some(-5);
        let y = x.flat_map(|n| if n > 0 { Some(n * 2) } else { None });
        assert_eq!(y, None);
    }

    #[rstest]
    fn option_flat_map_none() {
        let x: Option<i32> = None;
        let y = x.flat_map(|n| Some(n * 2));
        assert_eq!(y, None);
    }

    #[rstest]
    fn option_and_then_alias() {
        let x = Some(5);
        let flat_map_result = x.clone().flat_map(|n| Some(n * 2));
        let and_then_result = x.and_then(|n| Some(n * 2));
        assert_eq!(flat_map_result, and_then_result);
    }

    #[rstest]
    fn option_then_some() {
        let x = Some(5);
        let y = x.then(Some("hello"));
        assert_eq!(y, Some("hello"));
    }

    #[rstest]
    fn option_then_none() {
        let x: Option<i32> = None;
        let y = x.then(Some("hello"));
        assert_eq!(y, None);
    }

    #[rstest]
    fn option_flatten_some_some() {
        let nested: Option<Option<i32>> = Some(Some(42));
        let flat: Option<i32> = nested.flatten();
        assert_eq!(flat, Some(42));
    }

    #[rstest]
    fn option_flatten_some_none() {
        let nested: Option<Option<i32>> = Some(None);
        let flat: Option<i32> = nested.flatten();
        assert_eq!(flat, None);
    }

    #[rstest]
    fn option_flatten_none() {
        let nested: Option<Option<i32>> = None;
        let flat: Option<i32> = nested.flatten();
        assert_eq!(flat, None);
    }

    // =========================================================================
    // Result<T, E> Tests
    // =========================================================================

    #[rstest]
    fn result_flat_map_ok_to_ok() {
        let x: Result<i32, &str> = Ok(5);
        let y = x.flat_map(|n| Ok(n * 2));
        assert_eq!(y, Ok(10));
    }

    #[rstest]
    fn result_flat_map_ok_to_err() {
        let x: Result<i32, &str> = Ok(-5);
        let y = x.flat_map(|n| {
            if n > 0 {
                Ok(n * 2)
            } else {
                Err("negative number")
            }
        });
        assert_eq!(y, Err("negative number"));
    }

    #[rstest]
    fn result_flat_map_err() {
        let x: Result<i32, &str> = Err("initial error");
        let y = x.flat_map(|n| Ok(n * 2));
        assert_eq!(y, Err("initial error"));
    }

    #[rstest]
    fn result_and_then_alias() {
        let x: Result<i32, &str> = Ok(5);
        let flat_map_result = x.clone().flat_map(|n| Ok(n * 2));
        let and_then_result = x.and_then(|n| Ok(n * 2));
        assert_eq!(flat_map_result, and_then_result);
    }

    #[rstest]
    fn result_then_ok() {
        let x: Result<i32, &str> = Ok(5);
        let y = x.then(Ok("hello"));
        assert_eq!(y, Ok("hello"));
    }

    #[rstest]
    fn result_then_err() {
        let x: Result<i32, &str> = Err("error");
        let y = x.then(Ok("hello"));
        assert_eq!(y, Err("error"));
    }

    #[rstest]
    fn result_flatten_ok_ok() {
        let nested: Result<Result<i32, &str>, &str> = Ok(Ok(42));
        let flat: Result<i32, &str> = nested.flatten();
        assert_eq!(flat, Ok(42));
    }

    #[rstest]
    fn result_flatten_ok_err() {
        let nested: Result<Result<i32, &str>, &str> = Ok(Err("inner error"));
        let flat: Result<i32, &str> = nested.flatten();
        assert_eq!(flat, Err("inner error"));
    }

    #[rstest]
    fn result_flatten_err() {
        let nested: Result<Result<i32, &str>, &str> = Err("outer error");
        let flat: Result<i32, &str> = nested.flatten();
        assert_eq!(flat, Err("outer error"));
    }

    // =========================================================================
    // Vec<A> Tests (using MonadVec trait)
    // =========================================================================

    #[rstest]
    fn vec_flat_map_expands_elements() {
        let numbers = vec![1, 2, 3];
        let result = numbers.flat_map(|n| vec![n, n * 10]);
        assert_eq!(result, vec![1, 10, 2, 20, 3, 30]);
    }

    #[rstest]
    fn vec_flat_map_empty_input() {
        let empty: Vec<i32> = vec![];
        let result = empty.flat_map(|n| vec![n, n * 10]);
        assert!(result.is_empty());
    }

    #[rstest]
    fn vec_flat_map_produces_empty() {
        let numbers = vec![1, 2, 3];
        let result: Vec<i32> = numbers.flat_map(|_| vec![]);
        assert!(result.is_empty());
    }

    #[rstest]
    fn vec_flat_map_single_element_output() {
        let numbers = vec![1, 2, 3];
        let result = numbers.flat_map(|n| vec![n * 10]);
        assert_eq!(result, vec![10, 20, 30]);
    }

    #[rstest]
    fn vec_and_then_alias() {
        let numbers = vec![1, 2];
        let flat_map_result = numbers.clone().flat_map(|n| vec![n, n * 10]);
        let and_then_result = numbers.and_then(|n| vec![n, n * 10]);
        assert_eq!(flat_map_result, and_then_result);
    }

    #[rstest]
    fn vec_then_multiplies() {
        let first = vec![1, 2];
        let second = vec!["a", "b"];
        let result = first.then(second);
        // Each element of first produces the entire second vec
        assert_eq!(result, vec!["a", "b", "a", "b"]);
    }

    #[rstest]
    fn vec_then_empty_first() {
        let first: Vec<i32> = vec![];
        let second = vec!["a", "b"];
        let result = first.then(second);
        assert!(result.is_empty());
    }

    #[rstest]
    fn vec_flatten_nested() {
        let nested = vec![vec![1, 2], vec![3, 4]];
        let flat: Vec<i32> = nested.flatten();
        assert_eq!(flat, vec![1, 2, 3, 4]);
    }

    #[rstest]
    fn vec_flatten_with_empty() {
        let nested = vec![vec![1, 2], vec![], vec![3]];
        let flat: Vec<i32> = nested.flatten();
        assert_eq!(flat, vec![1, 2, 3]);
    }

    #[rstest]
    fn vec_flatten_empty() {
        let nested: Vec<Vec<i32>> = vec![];
        let flat: Vec<i32> = nested.flatten();
        assert!(flat.is_empty());
    }

    // =========================================================================
    // Box<A> Tests
    // =========================================================================

    #[rstest]
    fn box_flat_map_transforms() {
        let boxed = Box::new(5);
        let result = boxed.flat_map(|n| Box::new(n * 2));
        assert_eq!(*result, 10);
    }

    #[rstest]
    fn box_and_then_alias() {
        let boxed = Box::new(5);
        let flat_map_result = Box::new(5).flat_map(|n| Box::new(n * 2));
        let and_then_result = boxed.and_then(|n| Box::new(n * 2));
        assert_eq!(flat_map_result, and_then_result);
    }

    #[rstest]
    fn box_then_replaces() {
        let first = Box::new(5);
        let second = Box::new("hello");
        let result = first.then(second);
        assert_eq!(*result, "hello");
    }

    #[rstest]
    fn box_flatten_nested() {
        // Box doesn't have a built-in flatten, so we use flat_map with identity
        let nested: Box<Box<i32>> = Box::new(Box::new(42));
        let flat: Box<i32> = nested.flat_map(|inner| inner);
        assert_eq!(*flat, 42);
    }

    // =========================================================================
    // Identity<A> Tests
    // =========================================================================

    #[rstest]
    fn identity_flat_map_transforms() {
        let wrapped = Identity::new(5);
        let result = wrapped.flat_map(|n| Identity::new(n * 2));
        assert_eq!(result, Identity::new(10));
    }

    #[rstest]
    fn identity_and_then_alias() {
        let wrapped = Identity::new(5);
        let flat_map_result = Identity::new(5).flat_map(|n| Identity::new(n * 2));
        let and_then_result = wrapped.and_then(|n| Identity::new(n * 2));
        assert_eq!(flat_map_result, and_then_result);
    }

    #[rstest]
    fn identity_then_replaces() {
        let first = Identity::new(5);
        let second = Identity::new("hello");
        let result = first.then(second);
        assert_eq!(result, Identity::new("hello"));
    }

    #[rstest]
    fn identity_flatten_nested() {
        // Identity uses flat_map with identity function for flatten
        let nested = Identity::new(Identity::new(42));
        let flat: Identity<i32> = nested.flat_map(|inner| inner);
        assert_eq!(flat, Identity::new(42));
    }

    // =========================================================================
    // Monad Law Tests (Unit Tests)
    // =========================================================================

    // Left Identity Law: pure(a).flat_map(f) == f(a)

    #[rstest]
    fn option_left_identity_law() {
        let value = 5;
        let function = |n: i32| Some(n * 2);

        let left: Option<i32> = <Option<()>>::pure(value).flat_map(function);
        let right: Option<i32> = function(value);

        assert_eq!(left, right);
        assert_eq!(left, Some(10));
    }

    #[rstest]
    fn result_left_identity_law() {
        let value = 5;
        let function = |n: i32| -> Result<i32, ()> { Ok(n * 2) };

        let left: Result<i32, ()> = <Result<(), ()>>::pure(value).flat_map(function);
        let right: Result<i32, ()> = function(value);

        assert_eq!(left, right);
        assert_eq!(left, Ok(10));
    }

    #[rstest]
    fn box_left_identity_law() {
        let value = 5;
        let function = |n: i32| Box::new(n * 2);

        let left: Box<i32> = <Box<()>>::pure(value).flat_map(function);
        let right: Box<i32> = function(value);

        assert_eq!(left, right);
        assert_eq!(*left, 10);
    }

    #[rstest]
    fn identity_left_identity_law() {
        let value = 5;
        let function = |n: i32| Identity::new(n * 2);

        let left: Identity<i32> = <Identity<()>>::pure(value).flat_map(function);
        let right: Identity<i32> = function(value);

        assert_eq!(left, right);
        assert_eq!(left, Identity::new(10));
    }

    // Right Identity Law: m.flat_map(pure) == m

    #[rstest]
    fn option_right_identity_law_some() {
        let monad = Some(42);
        let result = monad.clone().flat_map(|x| <Option<()>>::pure(x));
        assert_eq!(result, monad);
    }

    #[rstest]
    fn option_right_identity_law_none() {
        let monad: Option<i32> = None;
        let result = monad.clone().flat_map(|x| <Option<()>>::pure(x));
        assert_eq!(result, monad);
    }

    #[rstest]
    fn result_right_identity_law_ok() {
        let monad: Result<i32, &str> = Ok(42);
        let result = monad.clone().flat_map(|x| <Result<(), &str>>::pure(x));
        assert_eq!(result, monad);
    }

    #[rstest]
    fn result_right_identity_law_err() {
        let monad: Result<i32, &str> = Err("error");
        let result = monad.clone().flat_map(|x| <Result<(), &str>>::pure(x));
        assert_eq!(result, monad);
    }

    #[rstest]
    fn box_right_identity_law() {
        let monad = Box::new(42);
        let result = Box::new(42).flat_map(|x| <Box<()>>::pure(x));
        assert_eq!(result, monad);
    }

    #[rstest]
    fn identity_right_identity_law() {
        let monad = Identity::new(42);
        let result = monad.clone().flat_map(|x| <Identity<()>>::pure(x));
        assert_eq!(result, monad);
    }

    // Associativity Law: m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))

    #[rstest]
    fn option_associativity_law() {
        let monad = Some(5);
        let function1 = |n: i32| Some(n + 1);
        let function2 = |n: i32| Some(n * 2);

        let left = monad.clone().flat_map(function1).flat_map(function2);
        let right = monad.flat_map(|x| function1(x).flat_map(function2));

        assert_eq!(left, right);
        assert_eq!(left, Some(12)); // (5 + 1) * 2 = 12
    }

    #[rstest]
    fn option_associativity_law_with_failure() {
        let monad = Some(5);
        let function1 = |n: i32| if n > 0 { Some(n - 10) } else { None };
        let function2 = |n: i32| if n > 0 { Some(n * 2) } else { None };

        let left = monad.clone().flat_map(function1).flat_map(function2);
        let right = monad.flat_map(|x| function1(x).flat_map(function2));

        assert_eq!(left, right);
        assert_eq!(left, None); // 5 - 10 = -5, which fails function2
    }

    #[rstest]
    fn result_associativity_law() {
        let monad: Result<i32, &str> = Ok(5);
        let function1 = |n: i32| -> Result<i32, &str> { Ok(n + 1) };
        let function2 = |n: i32| -> Result<i32, &str> { Ok(n * 2) };

        let left = monad.clone().flat_map(function1).flat_map(function2);
        let right = monad.flat_map(|x| function1(x).flat_map(function2));

        assert_eq!(left, right);
        assert_eq!(left, Ok(12));
    }

    #[rstest]
    fn box_associativity_law() {
        let monad = Box::new(5);
        let function1 = |n: i32| Box::new(n + 1);
        let function2 = |n: i32| Box::new(n * 2);

        let left = Box::new(5).flat_map(function1).flat_map(function2);
        let right = monad.flat_map(|x| function1(x).flat_map(function2));

        assert_eq!(left, right);
        assert_eq!(*left, 12);
    }

    #[rstest]
    fn identity_associativity_law() {
        let monad = Identity::new(5);
        let function1 = |n: i32| Identity::new(n + 1);
        let function2 = |n: i32| Identity::new(n * 2);

        let left = monad.clone().flat_map(function1).flat_map(function2);
        let right = monad.flat_map(|x| function1(x).flat_map(function2));

        assert_eq!(left, right);
        assert_eq!(left, Identity::new(12));
    }

    // =========================================================================
    // Vec Law Tests (using MonadVec)
    // =========================================================================

    #[rstest]
    fn vec_left_identity_law() {
        use crate::typeclass::ApplicativeVec;

        let value = 5;
        let function = |n: i32| vec![n, n * 10];

        let left: Vec<i32> = Vec::<i32>::pure(value).flat_map(function);
        let right: Vec<i32> = function(value);

        assert_eq!(left, right);
        assert_eq!(left, vec![5, 50]);
    }

    #[rstest]
    fn vec_right_identity_law() {
        use crate::typeclass::ApplicativeVec;

        let monad = vec![1, 2, 3];
        let result = monad.clone().flat_map(|x| Vec::<i32>::pure(x));
        assert_eq!(result, monad);
    }

    #[rstest]
    fn vec_associativity_law() {
        let monad = vec![1, 2];
        let function1 = |n: i32| vec![n, n + 10];
        let function2 = |n: i32| vec![n, n * 100];

        let left: Vec<i32> = monad.clone().flat_map(function1).flat_map(function2);
        let right: Vec<i32> = monad.flat_map(|x| function1(x).flat_map(function2));

        assert_eq!(left, right);
        // [1, 11, 2, 12] -> each element goes through function2
        // 1 -> [1, 100], 11 -> [11, 1100], 2 -> [2, 200], 12 -> [12, 1200]
        assert_eq!(left, vec![1, 100, 11, 1100, 2, 200, 12, 1200]);
    }

    // =========================================================================
    // Use Case Tests
    // =========================================================================

    #[rstest]
    fn option_chained_parsing() {
        fn parse_int(input: &str) -> Option<i32> {
            input.parse().ok()
        }

        fn validate_positive(n: i32) -> Option<i32> {
            if n > 0 { Some(n) } else { None }
        }

        fn double(n: i32) -> Option<i32> {
            Some(n * 2)
        }

        // Successful chain
        let result = parse_int("42").flat_map(validate_positive).flat_map(double);
        assert_eq!(result, Some(84));

        // Failure in parsing
        let result = parse_int("not a number")
            .flat_map(validate_positive)
            .flat_map(double);
        assert_eq!(result, None);

        // Failure in validation
        let result = parse_int("-5").flat_map(validate_positive).flat_map(double);
        assert_eq!(result, None);
    }

    #[rstest]
    fn result_chained_operations() {
        fn divide(numerator: i32, denominator: i32) -> Result<i32, &'static str> {
            if denominator == 0 {
                Err("division by zero")
            } else {
                Ok(numerator / denominator)
            }
        }

        fn safe_sqrt(n: i32) -> Result<f64, &'static str> {
            if n < 0 {
                Err("negative number")
            } else {
                Ok((n as f64).sqrt())
            }
        }

        // Successful chain
        let result: Result<f64, &str> = divide(100, 4).flat_map(|n| safe_sqrt(n));
        assert_eq!(result, Ok(5.0));

        // Failure in division
        let result: Result<f64, &str> = divide(100, 0).flat_map(|n| safe_sqrt(n));
        assert_eq!(result, Err("division by zero"));
    }

    #[rstest]
    fn vec_non_deterministic_computation() {
        // Generate all possible combinations
        let dice1 = vec![1, 2, 3, 4, 5, 6];
        let dice2 = vec![1, 2, 3, 4, 5, 6];

        // Find all ways to roll 7
        let ways_to_seven: Vec<(i32, i32)> = dice1.flat_map(|a| {
            dice2
                .clone()
                .flat_map(move |b| if a + b == 7 { vec![(a, b)] } else { vec![] })
        });

        assert_eq!(
            ways_to_seven,
            vec![(1, 6), (2, 5), (3, 4), (4, 3), (5, 2), (6, 1)]
        );
    }

    #[rstest]
    fn identity_composition_example() {
        // Identity monad just wraps values - useful as a base case
        let result = Identity::new(5)
            .flat_map(|n| Identity::new(n + 1))
            .flat_map(|n| Identity::new(n * 2));
        assert_eq!(result, Identity::new(12));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use crate::typeclass::{Applicative, ApplicativeVec};
    use proptest::prelude::*;

    // =========================================================================
    // Property Tests for Monad Laws
    // =========================================================================

    proptest! {
        // Left Identity Law: pure(a).flat_map(f) == f(a)

        #[test]
        fn prop_option_left_identity(value in any::<i32>()) {
            let function = |n: i32| if n % 2 == 0 { Some(n.wrapping_mul(2)) } else { None };

            let left: Option<i32> = <Option<()>>::pure(value).flat_map(function);
            let right: Option<i32> = function(value);

            prop_assert_eq!(left, right);
        }

        #[test]
        fn prop_result_left_identity(value in any::<i32>()) {
            let function = |n: i32| -> Result<i32, &'static str> { Ok(n.wrapping_mul(2)) };

            let left: Result<i32, &str> = <Result<(), &str>>::pure(value).flat_map(function);
            let right: Result<i32, &str> = function(value);

            prop_assert_eq!(left, right);
        }

        #[test]
        fn prop_identity_left_identity(value in any::<i32>()) {
            let function = |n: i32| Identity::new(n.wrapping_mul(2));

            let left = <Identity<()>>::pure(value).flat_map(function);
            let right = function(value);

            prop_assert_eq!(left, right);
        }

        #[test]
        fn prop_vec_left_identity(value in any::<i32>()) {
            let function = |n: i32| vec![n, n.wrapping_add(1)];

            let left: Vec<i32> = Vec::<i32>::pure(value).flat_map(function);
            let right: Vec<i32> = function(value);

            prop_assert_eq!(left, right);
        }

        // Right Identity Law: m.flat_map(pure) == m

        #[test]
        fn prop_option_right_identity(monad in any::<Option<i32>>()) {
            let result = monad.clone().flat_map(|x| <Option<()>>::pure(x));
            prop_assert_eq!(result, monad);
        }

        #[test]
        fn prop_result_right_identity(
            monad in prop::result::maybe_ok(any::<i32>(), any::<String>())
        ) {
            let result = monad.clone().flat_map(|x| <Result<(), String>>::pure(x));
            prop_assert_eq!(result, monad);
        }

        #[test]
        fn prop_identity_right_identity(value in any::<i32>()) {
            let monad = Identity::new(value);
            let result = monad.clone().flat_map(|x| <Identity<()>>::pure(x));
            prop_assert_eq!(result, monad);
        }

        #[test]
        fn prop_vec_right_identity(monad in prop::collection::vec(any::<i32>(), 0..10)) {
            let result = monad.clone().flat_map(|x| Vec::<i32>::pure(x));
            prop_assert_eq!(result, monad);
        }

        // Associativity Law: m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))

        #[test]
        fn prop_option_associativity(value in any::<i32>()) {
            let monad = Some(value);
            let function1 = |n: i32| Some(n.wrapping_add(1));
            let function2 = |n: i32| Some(n.wrapping_mul(2));

            let left = monad.clone().flat_map(function1).flat_map(function2);
            let right = monad.flat_map(|x| function1(x).flat_map(function2));

            prop_assert_eq!(left, right);
        }

        #[test]
        fn prop_result_associativity(value in any::<i32>()) {
            let monad: Result<i32, ()> = Ok(value);
            let function1 = |n: i32| -> Result<i32, ()> { Ok(n.wrapping_add(1)) };
            let function2 = |n: i32| -> Result<i32, ()> { Ok(n.wrapping_mul(2)) };

            let left = monad.clone().flat_map(function1).flat_map(function2);
            let right = monad.flat_map(|x| function1(x).flat_map(function2));

            prop_assert_eq!(left, right);
        }

        #[test]
        fn prop_identity_associativity(value in any::<i32>()) {
            let monad = Identity::new(value);
            let function1 = |n: i32| Identity::new(n.wrapping_add(1));
            let function2 = |n: i32| Identity::new(n.wrapping_mul(2));

            let left = monad.clone().flat_map(function1).flat_map(function2);
            let right = monad.flat_map(|x| function1(x).flat_map(function2));

            prop_assert_eq!(left, right);
        }

        #[test]
        fn prop_vec_associativity(monad in prop::collection::vec(any::<i32>(), 0..5)) {
            let function1 = |n: i32| vec![n, n.wrapping_add(1)];
            let function2 = |n: i32| vec![n.wrapping_mul(10)];

            let left: Vec<i32> = monad.clone().flat_map(function1).flat_map(function2);
            let right: Vec<i32> = monad.flat_map(|x| function1(x).flat_map(function2));

            prop_assert_eq!(left, right);
        }
    }
}
