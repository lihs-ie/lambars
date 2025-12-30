//! Applicative type class - applying functions within contexts.
//!
//! This module provides the `Applicative` trait, which extends `Functor` with
//! the ability to:
//!
//! - Lift pure values into the applicative context (`pure`)
//! - Combine multiple applicative values using a function (`map2`, `map3`)
//! - Create tuples of applicative values (`product`)
//!
//! `Applicative` is more powerful than `Functor` because it allows combining
//! multiple independent computations within the same context.
//!
//! # Laws
//!
//! All `Applicative` implementations must satisfy these laws:
//!
//! ## Identity Law
//!
//! Applying the identity function wrapped in `pure` should return the original value:
//!
//! ```text
//! pure(|x| x).apply(v) == v
//! ```
//!
//! ## Homomorphism Law
//!
//! Applying a pure function to a pure value equals pure of the function applied to the value:
//!
//! ```text
//! pure(f).apply(pure(x)) == pure(f(x))
//! ```
//!
//! ## Interchange Law
//!
//! The order of application can be swapped with appropriate wrapping:
//!
//! ```text
//! u.apply(pure(y)) == pure(|f| f(y)).apply(u)
//! ```
//!
//! ## Composition Law
//!
//! Function composition inside contexts works correctly:
//!
//! ```text
//! pure(compose).apply(u).apply(v).apply(w) == u.apply(v.apply(w))
//! ```
//!
//! # Examples
//!
//! ```rust
//! use functional_rusty::typeclass::Applicative;
//!
//! // Lifting a pure value into Option context
//! let x: Option<i32> = <Option<()>>::pure(42);
//! assert_eq!(x, Some(42));
//!
//! // Combining two Option values
//! let a = Some(1);
//! let b = Some(2);
//! let c = a.map2(b, |x, y| x + y);
//! assert_eq!(c, Some(3));
//!
//! // Creating a tuple of values
//! let x = Some(1);
//! let y = Some("hello");
//! assert_eq!(x.product(y), Some((1, "hello")));
//! ```

use super::functor::Functor;
use super::identity::Identity;

/// A type class for types that support lifting values and combining contexts.
///
/// `Applicative` extends `Functor` with the ability to:
///
/// - Lift any value into the context using `pure`
/// - Combine multiple values in the context using `map2`
///
/// # Laws
///
/// ## Identity Law
///
/// Applying identity through pure returns the original value:
///
/// ```text
/// pure(|x| x).apply(v) == v
/// ```
///
/// ## Homomorphism Law
///
/// Pure preserves function application:
///
/// ```text
/// pure(f).apply(pure(x)) == pure(f(x))
/// ```
///
/// ## Interchange Law
///
/// Application order can be swapped:
///
/// ```text
/// u.apply(pure(y)) == pure(|f| f(y)).apply(u)
/// ```
///
/// ## Composition Law
///
/// Composition is preserved:
///
/// ```text
/// pure(compose).apply(u).apply(v).apply(w) == u.apply(v.apply(w))
/// ```
///
/// # Examples
///
/// ```rust
/// use functional_rusty::typeclass::Applicative;
///
/// // Pure lifts a value into the context
/// let x: Option<i32> = <Option<()>>::pure(42);
/// assert_eq!(x, Some(42));
///
/// // map2 combines two values
/// let a = Some(3);
/// let b = Some(4);
/// let sum = a.map2(b, |x, y| x + y);
/// assert_eq!(sum, Some(7));
/// ```
pub trait Applicative: Functor {
    /// Lifts a pure value into the applicative context.
    ///
    /// This is the fundamental operation that allows creating an applicative
    /// value from any regular value.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to lift into the context
    ///
    /// # Returns
    ///
    /// The value wrapped in the applicative context
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::typeclass::Applicative;
    ///
    /// let x: Option<i32> = <Option<()>>::pure(42);
    /// assert_eq!(x, Some(42));
    ///
    /// let y: Result<String, ()> = <Result<(), ()>>::pure("hello".to_string());
    /// assert_eq!(y, Ok("hello".to_string()));
    /// ```
    fn pure<B>(value: B) -> Self::WithType<B>;

    /// Combines two applicative values using a binary function.
    ///
    /// This is the primary way to combine multiple independent computations
    /// within an applicative context. If either computation fails (in the
    /// sense appropriate to the specific applicative), the result fails.
    ///
    /// # Arguments
    ///
    /// * `other` - The second applicative value
    /// * `function` - A function that takes both inner values and produces a result
    ///
    /// # Returns
    ///
    /// An applicative containing the result of applying the function
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::typeclass::Applicative;
    ///
    /// let a = Some(1);
    /// let b = Some(2);
    /// let sum = a.map2(b, |x, y| x + y);
    /// assert_eq!(sum, Some(3));
    ///
    /// let a = Some(1);
    /// let b: Option<i32> = None;
    /// let sum = a.map2(b, |x, y| x + y);
    /// assert_eq!(sum, None);
    /// ```
    fn map2<B, C, F>(self, other: Self::WithType<B>, function: F) -> Self::WithType<C>
    where
        F: FnOnce(Self::Inner, B) -> C;

    /// Combines three applicative values using a ternary function.
    ///
    /// This is a convenience method built on top of `map2`.
    ///
    /// # Arguments
    ///
    /// * `second` - The second applicative value
    /// * `third` - The third applicative value
    /// * `function` - A function that takes all three inner values
    ///
    /// # Returns
    ///
    /// An applicative containing the result
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::typeclass::Applicative;
    ///
    /// let a = Some(1);
    /// let b = Some(2);
    /// let c = Some(3);
    /// let sum = a.map3(b, c, |x, y, z| x + y + z);
    /// assert_eq!(sum, Some(6));
    /// ```
    fn map3<B, C, D, F>(
        self,
        second: Self::WithType<B>,
        third: Self::WithType<C>,
        function: F,
    ) -> Self::WithType<D>
    where
        F: FnOnce(Self::Inner, B, C) -> D;

    /// Combines two applicative values into a tuple.
    ///
    /// This is equivalent to `map2(other, |a, b| (a, b))`.
    ///
    /// # Arguments
    ///
    /// * `other` - The second applicative value
    ///
    /// # Returns
    ///
    /// An applicative containing a tuple of both values
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::typeclass::Applicative;
    ///
    /// let a = Some(1);
    /// let b = Some("hello");
    /// assert_eq!(a.product(b), Some((1, "hello")));
    /// ```
    #[inline]
    fn product<B>(self, other: Self::WithType<B>) -> Self::WithType<(Self::Inner, B)>
    where
        Self: Sized,
    {
        self.map2(other, |a, b| (a, b))
    }

    /// Evaluates two applicatives and returns the left value.
    ///
    /// Both applicatives are evaluated, but only the left value is returned.
    /// This is useful when the right computation has a side effect that must
    /// be performed, but its value is not needed.
    ///
    /// # Arguments
    ///
    /// * `other` - The second applicative (evaluated but its value is discarded)
    ///
    /// # Returns
    ///
    /// An applicative containing the left value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::typeclass::Applicative;
    ///
    /// let a = Some(1);
    /// let b = Some(2);
    /// assert_eq!(a.product_left(b), Some(1));
    ///
    /// let a = Some(1);
    /// let b: Option<i32> = None;
    /// assert_eq!(a.product_left(b), None);
    /// ```
    #[inline]
    fn product_left<B>(self, other: Self::WithType<B>) -> Self::WithType<Self::Inner>
    where
        Self: Sized,
    {
        self.map2(other, |a, _| a)
    }

    /// Evaluates two applicatives and returns the right value.
    ///
    /// Both applicatives are evaluated, but only the right value is returned.
    /// This is useful when the left computation has a side effect that must
    /// be performed, but its value is not needed.
    ///
    /// # Arguments
    ///
    /// * `other` - The second applicative (its value is returned)
    ///
    /// # Returns
    ///
    /// An applicative containing the right value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::typeclass::Applicative;
    ///
    /// let a = Some(1);
    /// let b = Some(2);
    /// assert_eq!(a.product_right(b), Some(2));
    ///
    /// let a: Option<i32> = None;
    /// let b = Some(2);
    /// assert_eq!(a.product_right(b), None);
    /// ```
    #[inline]
    fn product_right<B>(self, other: Self::WithType<B>) -> Self::WithType<B>
    where
        Self: Sized,
    {
        self.map2(other, |_, b| b)
    }

    /// Applies a function inside the context to a value inside the context.
    ///
    /// This method is available when `Self` contains a function type. It applies
    /// the contained function to the value in `other`.
    ///
    /// # Arguments
    ///
    /// * `other` - An applicative containing the value to apply the function to
    ///
    /// # Returns
    ///
    /// An applicative containing the result of applying the function
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::typeclass::Applicative;
    ///
    /// let function: Option<fn(i32) -> i32> = Some(|x| x + 1);
    /// let value = Some(5);
    /// let result = function.apply(value);
    /// assert_eq!(result, Some(6));
    /// ```
    fn apply<B, Output>(self, other: Self::WithType<B>) -> Self::WithType<Output>
    where
        Self: Sized,
        Self::Inner: FnOnce(B) -> Output;
}

// =============================================================================
// Option<A> Implementation
// =============================================================================

impl<A> Applicative for Option<A> {
    #[inline]
    fn pure<B>(value: B) -> Option<B> {
        Some(value)
    }

    #[inline]
    fn map2<B, C, F>(self, other: Option<B>, function: F) -> Option<C>
    where
        F: FnOnce(A, B) -> C,
    {
        match (self, other) {
            (Some(a), Some(b)) => Some(function(a, b)),
            _ => None,
        }
    }

    #[inline]
    fn map3<B, C, D, F>(self, second: Option<B>, third: Option<C>, function: F) -> Option<D>
    where
        F: FnOnce(A, B, C) -> D,
    {
        match (self, second, third) {
            (Some(a), Some(b), Some(c)) => Some(function(a, b, c)),
            _ => None,
        }
    }

    #[inline]
    fn apply<B, Output>(self, other: Option<B>) -> Option<Output>
    where
        A: FnOnce(B) -> Output,
    {
        match (self, other) {
            (Some(function), Some(b)) => Some(function(b)),
            _ => None,
        }
    }
}

// =============================================================================
// Result<T, E> Implementation
// =============================================================================

impl<T, E: Clone> Applicative for Result<T, E> {
    #[inline]
    fn pure<B>(value: B) -> Result<B, E> {
        Ok(value)
    }

    #[inline]
    fn map2<B, C, F>(self, other: Result<B, E>, function: F) -> Result<C, E>
    where
        F: FnOnce(T, B) -> C,
    {
        match (self, other) {
            (Ok(a), Ok(b)) => Ok(function(a, b)),
            (Err(error), _) => Err(error),
            (_, Err(error)) => Err(error),
        }
    }

    #[inline]
    fn map3<B, C, D, F>(
        self,
        second: Result<B, E>,
        third: Result<C, E>,
        function: F,
    ) -> Result<D, E>
    where
        F: FnOnce(T, B, C) -> D,
    {
        match (self, second, third) {
            (Ok(a), Ok(b), Ok(c)) => Ok(function(a, b, c)),
            (Err(error), _, _) => Err(error),
            (_, Err(error), _) => Err(error),
            (_, _, Err(error)) => Err(error),
        }
    }

    #[inline]
    fn apply<B, Output>(self, other: Result<B, E>) -> Result<Output, E>
    where
        T: FnOnce(B) -> Output,
    {
        match (self, other) {
            (Ok(function), Ok(b)) => Ok(function(b)),
            (Err(error), _) => Err(error),
            (_, Err(error)) => Err(error),
        }
    }
}

// =============================================================================
// Vec<A> Implementation
//
// Note: Vec requires Clone bounds for map2/map3/apply because we need to
// iterate over the cartesian product of all elements. This is expressed
// through a separate trait to maintain the Applicative interface clean.
// =============================================================================

/// Extension trait for Vec to provide Applicative-like operations.
///
/// Vec's Applicative instance represents non-deterministic computation:
/// combining two Vecs produces all possible combinations (cartesian product).
///
/// This trait requires Clone bounds because we need to create all combinations.
pub trait ApplicativeVec: Sized {
    /// The inner type of the Vec.
    type VecInner;

    /// Lifts a pure value into a singleton Vec.
    fn pure<B>(value: B) -> Vec<B> {
        vec![value]
    }

    /// Combines two Vecs using a binary function (cartesian product).
    fn map2<B: Clone, C, F>(self, other: Vec<B>, function: F) -> Vec<C>
    where
        Self::VecInner: Clone,
        F: FnMut(Self::VecInner, B) -> C;

    /// Combines three Vecs using a ternary function (cartesian product).
    fn map3<B: Clone, C: Clone, D, F>(self, second: Vec<B>, third: Vec<C>, function: F) -> Vec<D>
    where
        Self::VecInner: Clone,
        F: FnMut(Self::VecInner, B, C) -> D;

    /// Creates the cartesian product of two Vecs as tuples.
    fn product<B: Clone>(self, other: Vec<B>) -> Vec<(Self::VecInner, B)>
    where
        Self::VecInner: Clone;

    /// Applies functions in this Vec to values in another Vec.
    fn apply<B: Clone, Output>(self, other: Vec<B>) -> Vec<Output>
    where
        Self::VecInner: FnMut(B) -> Output + Clone;
}

impl<A> ApplicativeVec for Vec<A> {
    type VecInner = A;

    #[inline]
    fn map2<B: Clone, C, F>(self, other: Vec<B>, mut function: F) -> Vec<C>
    where
        A: Clone,
        F: FnMut(A, B) -> C,
    {
        let capacity = self.len().saturating_mul(other.len());
        let mut result = Vec::with_capacity(capacity);
        for a in &self {
            for b in &other {
                result.push(function(a.clone(), b.clone()));
            }
        }
        result
    }

    #[inline]
    fn map3<B: Clone, C: Clone, D, F>(
        self,
        second: Vec<B>,
        third: Vec<C>,
        mut function: F,
    ) -> Vec<D>
    where
        A: Clone,
        F: FnMut(A, B, C) -> D,
    {
        let capacity = self
            .len()
            .saturating_mul(second.len())
            .saturating_mul(third.len());
        let mut result = Vec::with_capacity(capacity);
        for a in &self {
            for b in &second {
                for c in &third {
                    result.push(function(a.clone(), b.clone(), c.clone()));
                }
            }
        }
        result
    }

    #[inline]
    fn product<B: Clone>(self, other: Vec<B>) -> Vec<(A, B)>
    where
        A: Clone,
    {
        self.map2(other, |a, b| (a, b))
    }

    #[inline]
    fn apply<B: Clone, Output>(self, other: Vec<B>) -> Vec<Output>
    where
        A: FnMut(B) -> Output + Clone,
    {
        let capacity = self.len().saturating_mul(other.len());
        let mut result = Vec::with_capacity(capacity);
        for mut function in self {
            for b in &other {
                result.push(function(b.clone()));
            }
        }
        result
    }
}

// =============================================================================
// Box<A> Implementation
// =============================================================================

impl<A> Applicative for Box<A> {
    #[inline]
    fn pure<B>(value: B) -> Box<B> {
        Box::new(value)
    }

    #[inline]
    fn map2<B, C, F>(self, other: Box<B>, function: F) -> Box<C>
    where
        F: FnOnce(A, B) -> C,
    {
        Box::new(function(*self, *other))
    }

    #[inline]
    fn map3<B, C, D, F>(self, second: Box<B>, third: Box<C>, function: F) -> Box<D>
    where
        F: FnOnce(A, B, C) -> D,
    {
        Box::new(function(*self, *second, *third))
    }

    #[inline]
    fn apply<B, Output>(self, other: Box<B>) -> Box<Output>
    where
        A: FnOnce(B) -> Output,
    {
        Box::new((*self)(*other))
    }
}

// =============================================================================
// Identity<A> Implementation
// =============================================================================

impl<A> Applicative for Identity<A> {
    #[inline]
    fn pure<B>(value: B) -> Identity<B> {
        Identity::new(value)
    }

    #[inline]
    fn map2<B, C, F>(self, other: Identity<B>, function: F) -> Identity<C>
    where
        F: FnOnce(A, B) -> C,
    {
        Identity::new(function(self.into_inner(), other.into_inner()))
    }

    #[inline]
    fn map3<B, C, D, F>(self, second: Identity<B>, third: Identity<C>, function: F) -> Identity<D>
    where
        F: FnOnce(A, B, C) -> D,
    {
        Identity::new(function(
            self.into_inner(),
            second.into_inner(),
            third.into_inner(),
        ))
    }

    #[inline]
    fn apply<B, Output>(self, other: Identity<B>) -> Identity<Output>
    where
        A: FnOnce(B) -> Output,
    {
        Identity::new((self.into_inner())(other.into_inner()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typeclass::Functor;
    use rstest::rstest;

    // =========================================================================
    // Option<A> Tests
    // =========================================================================

    #[rstest]
    fn option_pure_creates_some() {
        let result: Option<i32> = <Option<()>>::pure(42);
        assert_eq!(result, Some(42));
    }

    #[rstest]
    fn option_pure_with_string() {
        let result: Option<String> = <Option<()>>::pure("hello".to_string());
        assert_eq!(result, Some("hello".to_string()));
    }

    #[rstest]
    fn option_map2_some_some() {
        let a = Some(1);
        let b = Some(2);
        let result = a.map2(b, |x, y| x + y);
        assert_eq!(result, Some(3));
    }

    #[rstest]
    fn option_map2_some_none() {
        let a = Some(1);
        let b: Option<i32> = None;
        let result = a.map2(b, |x, y| x + y);
        assert_eq!(result, None);
    }

    #[rstest]
    fn option_map2_none_some() {
        let a: Option<i32> = None;
        let b = Some(2);
        let result = a.map2(b, |x, y| x + y);
        assert_eq!(result, None);
    }

    #[rstest]
    fn option_map2_none_none() {
        let a: Option<i32> = None;
        let b: Option<i32> = None;
        let result = a.map2(b, |x, y| x + y);
        assert_eq!(result, None);
    }

    #[rstest]
    fn option_map3_all_some() {
        let a = Some(1);
        let b = Some(2);
        let c = Some(3);
        let result = a.map3(b, c, |x, y, z| x + y + z);
        assert_eq!(result, Some(6));
    }

    #[rstest]
    fn option_map3_with_none() {
        let a = Some(1);
        let b: Option<i32> = None;
        let c = Some(3);
        let result = a.map3(b, c, |x, y, z| x + y + z);
        assert_eq!(result, None);
    }

    #[rstest]
    fn option_product_some_some() {
        let a = Some(1);
        let b = Some("hello");
        let result = a.product(b);
        assert_eq!(result, Some((1, "hello")));
    }

    #[rstest]
    fn option_product_with_none() {
        let a = Some(1);
        let b: Option<&str> = None;
        let result = a.product(b);
        assert_eq!(result, None);
    }

    #[rstest]
    fn option_product_left_returns_left() {
        let a = Some(1);
        let b = Some(2);
        let result = a.product_left(b);
        assert_eq!(result, Some(1));
    }

    #[rstest]
    fn option_product_left_with_none() {
        let a = Some(1);
        let b: Option<i32> = None;
        let result = a.product_left(b);
        assert_eq!(result, None);
    }

    #[rstest]
    fn option_product_right_returns_right() {
        let a = Some(1);
        let b = Some(2);
        let result = a.product_right(b);
        assert_eq!(result, Some(2));
    }

    #[rstest]
    fn option_product_right_with_none() {
        let a: Option<i32> = None;
        let b = Some(2);
        let result = a.product_right(b);
        assert_eq!(result, None);
    }

    #[rstest]
    fn option_apply_with_function() {
        let function: Option<fn(i32) -> i32> = Some(|x| x + 1);
        let value = Some(5);
        let result = function.apply(value);
        assert_eq!(result, Some(6));
    }

    #[rstest]
    fn option_apply_with_none_function() {
        let function: Option<fn(i32) -> i32> = None;
        let value = Some(5);
        let result = function.apply(value);
        assert_eq!(result, None);
    }

    #[rstest]
    fn option_apply_with_none_value() {
        let function: Option<fn(i32) -> i32> = Some(|x| x + 1);
        let value: Option<i32> = None;
        let result = function.apply(value);
        assert_eq!(result, None);
    }

    // =========================================================================
    // Result<T, E> Tests
    // =========================================================================

    #[rstest]
    fn result_pure_creates_ok() {
        let result: Result<i32, String> = <Result<(), String>>::pure(42);
        assert_eq!(result, Ok(42));
    }

    #[rstest]
    fn result_map2_ok_ok() {
        let a: Result<i32, &str> = Ok(1);
        let b: Result<i32, &str> = Ok(2);
        let result = a.map2(b, |x, y| x + y);
        assert_eq!(result, Ok(3));
    }

    #[rstest]
    fn result_map2_ok_err() {
        let a: Result<i32, &str> = Ok(1);
        let b: Result<i32, &str> = Err("error");
        let result = a.map2(b, |x, y| x + y);
        assert_eq!(result, Err("error"));
    }

    #[rstest]
    fn result_map2_err_ok() {
        let a: Result<i32, &str> = Err("error");
        let b: Result<i32, &str> = Ok(2);
        let result = a.map2(b, |x, y| x + y);
        assert_eq!(result, Err("error"));
    }

    #[rstest]
    fn result_map2_err_err_returns_first() {
        let a: Result<i32, &str> = Err("first");
        let b: Result<i32, &str> = Err("second");
        let result = a.map2(b, |x, y| x + y);
        assert_eq!(result, Err("first"));
    }

    #[rstest]
    fn result_map3_all_ok() {
        let a: Result<i32, &str> = Ok(1);
        let b: Result<i32, &str> = Ok(2);
        let c: Result<i32, &str> = Ok(3);
        let result = a.map3(b, c, |x, y, z| x + y + z);
        assert_eq!(result, Ok(6));
    }

    #[rstest]
    fn result_product_ok_ok() {
        let a: Result<i32, &str> = Ok(1);
        let b: Result<&str, &str> = Ok("hello");
        let result = a.product(b);
        assert_eq!(result, Ok((1, "hello")));
    }

    #[rstest]
    fn result_apply_with_function() {
        let function: Result<fn(i32) -> i32, &str> = Ok(|x| x + 1);
        let value: Result<i32, &str> = Ok(5);
        let result = function.apply(value);
        assert_eq!(result, Ok(6));
    }

    // =========================================================================
    // Vec<A> Tests (using ApplicativeVec trait)
    // =========================================================================

    #[rstest]
    fn vec_pure_creates_singleton() {
        let result: Vec<i32> = Vec::<i32>::pure(42);
        assert_eq!(result, vec![42]);
    }

    #[rstest]
    fn vec_map2_all_combinations() {
        let a = vec![1, 2];
        let b = vec![10, 20];
        let result = a.map2(b, |x, y| x + y);
        // All combinations: 1+10, 1+20, 2+10, 2+20
        assert_eq!(result, vec![11, 21, 12, 22]);
    }

    #[rstest]
    fn vec_map2_with_empty() {
        let a = vec![1, 2];
        let b: Vec<i32> = vec![];
        let result = a.map2(b, |x, y| x + y);
        assert!(result.is_empty());
    }

    #[rstest]
    fn vec_map3_all_combinations() {
        let a = vec![1, 2];
        let b = vec![10, 20];
        let c = vec![100];
        let result = a.map3(b, c, |x, y, z| x + y + z);
        // Combinations: 1+10+100, 1+20+100, 2+10+100, 2+20+100
        assert_eq!(result, vec![111, 121, 112, 122]);
    }

    #[rstest]
    fn vec_product_creates_tuples() {
        let a = vec![1, 2];
        let b = vec!["a", "b"];
        let result = a.product(b);
        assert_eq!(result, vec![(1, "a"), (1, "b"), (2, "a"), (2, "b")]);
    }

    #[rstest]
    fn vec_apply_all_combinations() {
        let functions: Vec<fn(i32) -> i32> = vec![|x| x + 1, |x| x * 2];
        let values = vec![5, 10];
        let result = functions.apply(values);
        // Combinations: (5+1), (10+1), (5*2), (10*2)
        assert_eq!(result, vec![6, 11, 10, 20]);
    }

    // =========================================================================
    // Box<A> Tests
    // =========================================================================

    #[rstest]
    fn box_pure_creates_box() {
        let result: Box<i32> = <Box<()>>::pure(42);
        assert_eq!(*result, 42);
    }

    #[rstest]
    fn box_map2_combines_values() {
        let a = Box::new(1);
        let b = Box::new(2);
        let result = a.map2(b, |x, y| x + y);
        assert_eq!(*result, 3);
    }

    #[rstest]
    fn box_map3_combines_values() {
        let a = Box::new(1);
        let b = Box::new(2);
        let c = Box::new(3);
        let result = a.map3(b, c, |x, y, z| x + y + z);
        assert_eq!(*result, 6);
    }

    #[rstest]
    fn box_product_creates_tuple() {
        let a = Box::new(1);
        let b = Box::new("hello");
        let result = a.product(b);
        assert_eq!(*result, (1, "hello"));
    }

    #[rstest]
    fn box_apply_with_function() {
        let function: Box<fn(i32) -> i32> = Box::new(|x| x + 1);
        let value = Box::new(5);
        let result = function.apply(value);
        assert_eq!(*result, 6);
    }

    // =========================================================================
    // Identity<A> Tests
    // =========================================================================

    #[rstest]
    fn identity_pure_creates_identity() {
        let result: Identity<i32> = <Identity<()>>::pure(42);
        assert_eq!(result, Identity::new(42));
    }

    #[rstest]
    fn identity_map2_combines_values() {
        let a = Identity::new(1);
        let b = Identity::new(2);
        let result = a.map2(b, |x, y| x + y);
        assert_eq!(result, Identity::new(3));
    }

    #[rstest]
    fn identity_map3_combines_values() {
        let a = Identity::new(1);
        let b = Identity::new(2);
        let c = Identity::new(3);
        let result = a.map3(b, c, |x, y, z| x + y + z);
        assert_eq!(result, Identity::new(6));
    }

    #[rstest]
    fn identity_product_creates_tuple() {
        let a = Identity::new(1);
        let b = Identity::new("hello");
        let result = a.product(b);
        assert_eq!(result, Identity::new((1, "hello")));
    }

    #[rstest]
    fn identity_apply_with_function() {
        let function: Identity<fn(i32) -> i32> = Identity::new(|x| x + 1);
        let value = Identity::new(5);
        let result = function.apply(value);
        assert_eq!(result, Identity::new(6));
    }

    // =========================================================================
    // Applicative Law Tests
    // =========================================================================

    // Identity Law: pure(id).apply(v) == v
    // We test this using map2: pure(x).map2(v, |_, y| y) == v for the structure
    // and pure(x).fmap(|x| x) == pure(x) for the value

    #[rstest]
    fn option_identity_law_with_fmap() {
        let value = 42;
        let pure_value: Option<i32> = <Option<()>>::pure(value);
        let result = pure_value.fmap(|x| x);
        assert_eq!(result, Some(value));
    }

    #[rstest]
    fn result_identity_law_with_fmap() {
        let value = 42;
        let pure_value: Result<i32, ()> = <Result<(), ()>>::pure(value);
        let result = pure_value.fmap(|x| x);
        assert_eq!(result, Ok(value));
    }

    #[rstest]
    fn identity_identity_law_with_fmap() {
        let value = 42;
        let pure_value: Identity<i32> = <Identity<()>>::pure(value);
        let result = pure_value.fmap(|x| x);
        assert_eq!(result, Identity::new(value));
    }

    // Homomorphism Law: pure(f).apply(pure(x)) == pure(f(x))

    #[rstest]
    fn option_homomorphism_law() {
        let function = |x: i32| x + 1;
        let value = 5;

        let left: Option<i32> = <Option<()>>::pure(function).apply(<Option<()>>::pure(value));
        let right: Option<i32> = <Option<()>>::pure(function(value));

        assert_eq!(left, right);
        assert_eq!(left, Some(6));
    }

    #[rstest]
    fn result_homomorphism_law() {
        let function = |x: i32| x + 1;
        let value = 5;

        let left: Result<i32, ()> =
            <Result<(), ()>>::pure(function).apply(<Result<(), ()>>::pure(value));
        let right: Result<i32, ()> = <Result<(), ()>>::pure(function(value));

        assert_eq!(left, right);
        assert_eq!(left, Ok(6));
    }

    #[rstest]
    fn identity_homomorphism_law() {
        let function = |x: i32| x + 1;
        let value = 5;

        let left: Identity<i32> = <Identity<()>>::pure(function).apply(<Identity<()>>::pure(value));
        let right: Identity<i32> = <Identity<()>>::pure(function(value));

        assert_eq!(left, right);
        assert_eq!(left, Identity::new(6));
    }

    // Composition through map2: verify that combining operations works correctly

    #[rstest]
    fn option_composition_via_map2() {
        let a = Some(1);
        let b = Some(2);
        let c = Some(3);

        // ((a, b), c) style
        let left = a
            .clone()
            .map2(b.clone(), |x, y| (x, y))
            .map2(c.clone(), |(x, y), z| x + y + z);

        // Direct map3
        let right = a.map3(b, c, |x, y, z| x + y + z);

        assert_eq!(left, right);
        assert_eq!(left, Some(6));
    }

    #[rstest]
    fn result_composition_via_map2() {
        let a: Result<i32, ()> = Ok(1);
        let b: Result<i32, ()> = Ok(2);
        let c: Result<i32, ()> = Ok(3);

        let left = a
            .clone()
            .map2(b.clone(), |x, y| (x, y))
            .map2(c.clone(), |(x, y), z| x + y + z);
        let right = a.map3(b, c, |x, y, z| x + y + z);

        assert_eq!(left, right);
        assert_eq!(left, Ok(6));
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[rstest]
    fn option_map2_with_different_types() {
        let a = Some(42);
        let b = Some("hello");
        let result = a.map2(b, |n, s| format!("{}: {}", n, s));
        assert_eq!(result, Some("42: hello".to_string()));
    }

    #[rstest]
    fn vec_map2_preserves_order() {
        let a = vec![1, 2, 3];
        let b = vec![10];
        let result = a.map2(b, |x, y| x + y);
        assert_eq!(result, vec![11, 12, 13]);
    }

    #[rstest]
    fn vec_product_empty_vec() {
        let a: Vec<i32> = vec![];
        let b = vec![1, 2];
        let result = a.product(b);
        assert!(result.is_empty());
    }

    // =========================================================================
    // Use Case Tests
    // =========================================================================

    #[rstest]
    fn option_validate_multiple_fields() {
        // Simulating form validation where all fields must be present
        fn parse_age(input: &str) -> Option<u32> {
            input.parse().ok()
        }

        fn parse_name(input: &str) -> Option<String> {
            if input.is_empty() {
                None
            } else {
                Some(input.to_string())
            }
        }

        let age_input = "25";
        let name_input = "Alice";

        let age = parse_age(age_input);
        let name = parse_name(name_input);

        let user = age.map2(name, |a, n| (a, n));
        assert_eq!(user, Some((25, "Alice".to_string())));
    }

    #[rstest]
    fn result_combine_validations() {
        fn validate_positive(value: i32) -> Result<i32, &'static str> {
            if value > 0 {
                Ok(value)
            } else {
                Err("Value must be positive")
            }
        }

        fn validate_even(value: i32) -> Result<i32, &'static str> {
            if value % 2 == 0 {
                Ok(value)
            } else {
                Err("Value must be even")
            }
        }

        let a = validate_positive(4);
        let b = validate_even(6);

        let result = a.map2(b, |x, y| x + y);
        assert_eq!(result, Ok(10));
    }

    #[rstest]
    fn vec_cartesian_product_use_case() {
        // Generate all possible coordinates in a 2x2 grid
        let x_coords = vec![0, 1];
        let y_coords = vec![0, 1];

        let coordinates = x_coords.product(y_coords);
        assert_eq!(coordinates, vec![(0, 0), (0, 1), (1, 0), (1, 1)]);
    }
}
