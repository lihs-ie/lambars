//! Choice combinators for Either type.
//!
//! This module provides Prisms for the Left and Right variants of `Either<L, R>`,
//! as well as a `choice` function that combines two Prisms operating on
//! different branches of an Either.
//!
//! # Examples
//!
//! ## Using left and right Prisms
//!
//! ```
//! use lambars::control::Either;
//! use lambars::optics::{Prism, choice::left_prism, choice::right_prism};
//!
//! let left_value: Either<i32, String> = Either::Left(42);
//! let right_value: Either<i32, String> = Either::Right("hello".to_string());
//!
//! let left_p = left_prism::<i32, String>();
//! let right_p = right_prism::<i32, String>();
//!
//! assert_eq!(left_p.preview(&left_value), Some(&42));
//! assert_eq!(left_p.preview(&right_value), None);
//!
//! assert_eq!(right_p.preview(&left_value), None);
//! assert_eq!(right_p.preview(&right_value), Some(&"hello".to_string()));
//! ```
//!
//! ## Using the choice function
//!
//! ```
//! use lambars::control::Either;
//! use lambars::optics::{Prism, FunctionPrism, choice::choice};
//!
//! // Prism that focuses on even numbers
//! let even_prism = FunctionPrism::new(
//!     |n: &i32| if *n % 2 == 0 { Some(n) } else { None },
//!     |n: i32| n,
//!     |n: i32| if n % 2 == 0 { Some(n) } else { None },
//! );
//!
//! // Prism that focuses on non-empty strings
//! let non_empty_prism = FunctionPrism::new(
//!     |s: &String| if s.is_empty() { None } else { Some(s) },
//!     |s: String| s,
//!     |s: String| if s.is_empty() { None } else { Some(s) },
//! );
//!
//! // Combined prism that works on Either<i32, String>
//! let combined = choice(even_prism, non_empty_prism);
//!
//! // ChoicePrism uses preview_owned because it transforms the inner types
//! // Left even number: works
//! let left_even: Either<i32, String> = Either::Left(42);
//! assert_eq!(combined.preview_owned(left_even), Some(Either::Left(42)));
//!
//! // Left odd number: fails
//! let left_odd: Either<i32, String> = Either::Left(41);
//! assert_eq!(combined.preview_owned(left_odd), None);
//!
//! // Right non-empty string: works
//! let right_nonempty: Either<i32, String> = Either::Right("hello".to_string());
//! assert_eq!(combined.preview_owned(right_nonempty), Some(Either::Right("hello".to_string())));
//!
//! // Right empty string: fails
//! let right_empty: Either<i32, String> = Either::Right(String::new());
//! assert_eq!(combined.preview_owned(right_empty), None);
//! ```

use std::marker::PhantomData;

#[cfg(feature = "control")]
use crate::control::Either;
use crate::optics::Prism;

/// A Prism that focuses on the Left variant of an Either.
///
/// # Type Parameters
///
/// - `L`: The Left variant type
/// - `R`: The Right variant type
///
/// # Examples
///
/// ```
/// use lambars::control::Either;
/// use lambars::optics::{Prism, choice::LeftPrism};
///
/// let prism: LeftPrism<i32, String> = LeftPrism::new();
///
/// let left: Either<i32, String> = Either::Left(42);
/// assert_eq!(prism.preview(&left), Some(&42));
///
/// let right: Either<i32, String> = Either::Right("hello".to_string());
/// assert_eq!(prism.preview(&right), None);
/// ```
#[cfg(feature = "control")]
#[derive(Debug, Clone)]
pub struct LeftPrism<L, R> {
    _marker: PhantomData<(L, R)>,
}

#[cfg(feature = "control")]
impl<L, R> LeftPrism<L, R> {
    /// Creates a new `LeftPrism`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

#[cfg(feature = "control")]
impl<L, R> Default for LeftPrism<L, R> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "control")]
impl<L: Clone, R: Clone> Prism<Either<L, R>, L> for LeftPrism<L, R> {
    fn preview<'a>(&self, source: &'a Either<L, R>) -> Option<&'a L> {
        source.left_ref()
    }

    fn review(&self, value: L) -> Either<L, R> {
        Either::Left(value)
    }

    fn preview_owned(&self, source: Either<L, R>) -> Option<L> {
        source.left()
    }
}

/// Creates a `LeftPrism` for the given types.
///
/// This is a convenience function equivalent to `LeftPrism::new()`.
///
/// # Examples
///
/// ```
/// use lambars::control::Either;
/// use lambars::optics::{Prism, choice::left_prism};
///
/// let prism = left_prism::<i32, String>();
/// let left: Either<i32, String> = Either::Left(42);
/// assert_eq!(prism.preview(&left), Some(&42));
/// ```
#[cfg(feature = "control")]
#[must_use]
pub const fn left_prism<L, R>() -> LeftPrism<L, R> {
    LeftPrism::new()
}

/// A Prism that focuses on the Right variant of an Either.
///
/// # Type Parameters
///
/// - `L`: The Left variant type
/// - `R`: The Right variant type
///
/// # Examples
///
/// ```
/// use lambars::control::Either;
/// use lambars::optics::{Prism, choice::RightPrism};
///
/// let prism: RightPrism<i32, String> = RightPrism::new();
///
/// let right: Either<i32, String> = Either::Right("hello".to_string());
/// assert_eq!(prism.preview(&right), Some(&"hello".to_string()));
///
/// let left: Either<i32, String> = Either::Left(42);
/// assert_eq!(prism.preview(&left), None);
/// ```
#[cfg(feature = "control")]
#[derive(Debug, Clone)]
pub struct RightPrism<L, R> {
    _marker: PhantomData<(L, R)>,
}

#[cfg(feature = "control")]
impl<L, R> RightPrism<L, R> {
    /// Creates a new `RightPrism`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

#[cfg(feature = "control")]
impl<L, R> Default for RightPrism<L, R> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "control")]
impl<L: Clone, R: Clone> Prism<Either<L, R>, R> for RightPrism<L, R> {
    fn preview<'a>(&self, source: &'a Either<L, R>) -> Option<&'a R> {
        source.right_ref()
    }

    fn review(&self, value: R) -> Either<L, R> {
        Either::Right(value)
    }

    fn preview_owned(&self, source: Either<L, R>) -> Option<R> {
        source.right()
    }
}

/// Creates a `RightPrism` for the given types.
///
/// This is a convenience function equivalent to `RightPrism::new()`.
///
/// # Examples
///
/// ```
/// use lambars::control::Either;
/// use lambars::optics::{Prism, choice::right_prism};
///
/// let prism = right_prism::<i32, String>();
/// let right: Either<i32, String> = Either::Right("hello".to_string());
/// assert_eq!(prism.preview(&right), Some(&"hello".to_string()));
/// ```
#[cfg(feature = "control")]
#[must_use]
pub const fn right_prism<L, R>() -> RightPrism<L, R> {
    RightPrism::new()
}

/// A Prism that combines two Prisms, one for each branch of an Either.
///
/// This allows focusing on a value within either the Left or Right branch
/// of an Either, applying the appropriate Prism based on which branch is present.
///
/// # Type Parameters
///
/// - `P1`: The Prism type for the Left branch
/// - `P2`: The Prism type for the Right branch
/// - `L`: The Left variant type of the source Either
/// - `R`: The Right variant type of the source Either
/// - `A`: The target type when focusing through the Left branch
/// - `B`: The target type when focusing through the Right branch
///
/// # Note
///
/// The `preview` method returns `None` because `ChoicePrism` transforms the type.
/// Use `preview_owned` instead for owned access.
#[cfg(feature = "control")]
pub struct ChoicePrism<P1, P2, L, R, A, B> {
    left_prism: P1,
    right_prism: P2,
    _marker: PhantomData<(L, R, A, B)>,
}

#[cfg(feature = "control")]
impl<P1, P2, L, R, A, B> ChoicePrism<P1, P2, L, R, A, B> {
    /// Creates a new `ChoicePrism` from two Prisms.
    #[must_use]
    pub const fn new(left_prism: P1, right_prism: P2) -> Self {
        Self {
            left_prism,
            right_prism,
            _marker: PhantomData,
        }
    }
}

#[cfg(feature = "control")]
impl<P1: Clone, P2: Clone, L, R, A, B> Clone for ChoicePrism<P1, P2, L, R, A, B> {
    fn clone(&self) -> Self {
        Self {
            left_prism: self.left_prism.clone(),
            right_prism: self.right_prism.clone(),
            _marker: PhantomData,
        }
    }
}

#[cfg(feature = "control")]
impl<P1: std::fmt::Debug, P2: std::fmt::Debug, L, R, A, B> std::fmt::Debug
    for ChoicePrism<P1, P2, L, R, A, B>
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ChoicePrism")
            .field("left_prism", &self.left_prism)
            .field("right_prism", &self.right_prism)
            .finish()
    }
}

#[cfg(feature = "control")]
impl<P1, P2, L, R, A, B> Prism<Either<L, R>, Either<A, B>> for ChoicePrism<P1, P2, L, R, A, B>
where
    P1: Prism<L, A>,
    P2: Prism<R, B>,
    L: Clone,
    R: Clone,
    A: Clone,
    B: Clone,
{
    fn preview<'a>(&self, source: &'a Either<L, R>) -> Option<&'a Either<A, B>> {
        // Cannot return a reference because we would need to allocate a new Either.
        // Use preview_owned instead.
        let _ = source;
        None
    }

    fn review(&self, value: Either<A, B>) -> Either<L, R> {
        match value {
            Either::Left(a) => Either::Left(self.left_prism.review(a)),
            Either::Right(b) => Either::Right(self.right_prism.review(b)),
        }
    }

    fn preview_owned(&self, source: Either<L, R>) -> Option<Either<A, B>> {
        match source {
            Either::Left(left) => self.left_prism.preview_owned(left).map(Either::Left),
            Either::Right(right) => self.right_prism.preview_owned(right).map(Either::Right),
        }
    }
}

/// Creates a `ChoicePrism` that combines two Prisms for the Left and Right branches.
///
/// The resulting Prism focuses on `Either<A, B>` within an `Either<L, R>`,
/// where P1 focuses on `A` within `L`, and P2 focuses on `B` within `R`.
///
/// # Examples
///
/// ```
/// use lambars::control::Either;
/// use lambars::optics::{Prism, FunctionPrism, choice::choice};
///
/// // Identity prisms for demonstration
/// let int_prism = FunctionPrism::new(
///     |n: &i32| Some(n),
///     |n: i32| n,
///     |n: i32| Some(n),
/// );
/// let str_prism = FunctionPrism::new(
///     |s: &String| Some(s),
///     |s: String| s,
///     |s: String| Some(s),
/// );
///
/// let combined = choice(int_prism, str_prism);
///
/// let left: Either<i32, String> = Either::Left(42);
/// assert_eq!(combined.preview_owned(left), Some(Either::Left(42)));
/// ```
#[cfg(feature = "control")]
#[must_use]
pub const fn choice<L, R, A, B, P1, P2>(
    left_prism: P1,
    right_prism: P2,
) -> ChoicePrism<P1, P2, L, R, A, B>
where
    P1: Prism<L, A>,
    P2: Prism<R, B>,
{
    ChoicePrism::new(left_prism, right_prism)
}

#[cfg(all(test, feature = "control"))]
mod tests {
    use super::*;
    use crate::optics::FunctionPrism;

    // =========================================================================
    // LeftPrism Tests
    // =========================================================================

    #[test]
    fn test_left_prism_preview_left() {
        let prism: LeftPrism<i32, String> = LeftPrism::new();
        let left: Either<i32, String> = Either::Left(42);
        assert_eq!(prism.preview(&left), Some(&42));
    }

    #[test]
    fn test_left_prism_preview_right() {
        let prism: LeftPrism<i32, String> = LeftPrism::new();
        let right: Either<i32, String> = Either::Right("hello".to_string());
        assert_eq!(prism.preview(&right), None);
    }

    #[test]
    fn test_left_prism_review() {
        let prism: LeftPrism<i32, String> = LeftPrism::new();
        let result = prism.review(42);
        assert_eq!(result, Either::Left(42));
    }

    #[test]
    fn test_left_prism_preview_owned_left() {
        let prism: LeftPrism<i32, String> = LeftPrism::new();
        let left: Either<i32, String> = Either::Left(42);
        assert_eq!(prism.preview_owned(left), Some(42));
    }

    #[test]
    fn test_left_prism_preview_owned_right() {
        let prism: LeftPrism<i32, String> = LeftPrism::new();
        let right: Either<i32, String> = Either::Right("hello".to_string());
        assert_eq!(prism.preview_owned(right), None);
    }

    #[test]
    fn test_left_prism_default() {
        let prism: LeftPrism<i32, String> = LeftPrism::default();
        let left: Either<i32, String> = Either::Left(42);
        assert_eq!(prism.preview(&left), Some(&42));
    }

    #[test]
    fn test_left_prism_clone() {
        let prism: LeftPrism<i32, String> = LeftPrism::new();
        let cloned = prism;
        let left: Either<i32, String> = Either::Left(42);
        assert_eq!(cloned.preview(&left), Some(&42));
    }

    #[test]
    fn test_left_prism_debug() {
        let prism: LeftPrism<i32, String> = LeftPrism::new();
        let debug_string = format!("{prism:?}");
        assert!(debug_string.contains("LeftPrism"));
    }

    #[test]
    fn test_left_prism_convenience_function() {
        let prism = left_prism::<i32, String>();
        let left: Either<i32, String> = Either::Left(42);
        assert_eq!(prism.preview(&left), Some(&42));
    }

    // =========================================================================
    // RightPrism Tests
    // =========================================================================

    #[test]
    fn test_right_prism_preview_right() {
        let prism: RightPrism<i32, String> = RightPrism::new();
        let right: Either<i32, String> = Either::Right("hello".to_string());
        assert_eq!(prism.preview(&right), Some(&"hello".to_string()));
    }

    #[test]
    fn test_right_prism_preview_left() {
        let prism: RightPrism<i32, String> = RightPrism::new();
        let left: Either<i32, String> = Either::Left(42);
        assert_eq!(prism.preview(&left), None);
    }

    #[test]
    fn test_right_prism_review() {
        let prism: RightPrism<i32, String> = RightPrism::new();
        let result = prism.review("hello".to_string());
        assert_eq!(result, Either::Right("hello".to_string()));
    }

    #[test]
    fn test_right_prism_preview_owned_right() {
        let prism: RightPrism<i32, String> = RightPrism::new();
        let right: Either<i32, String> = Either::Right("hello".to_string());
        assert_eq!(prism.preview_owned(right), Some("hello".to_string()));
    }

    #[test]
    fn test_right_prism_preview_owned_left() {
        let prism: RightPrism<i32, String> = RightPrism::new();
        let left: Either<i32, String> = Either::Left(42);
        assert_eq!(prism.preview_owned(left), None);
    }

    #[test]
    fn test_right_prism_default() {
        let prism: RightPrism<i32, String> = RightPrism::default();
        let right: Either<i32, String> = Either::Right("hello".to_string());
        assert_eq!(prism.preview(&right), Some(&"hello".to_string()));
    }

    #[test]
    fn test_right_prism_clone() {
        let prism: RightPrism<i32, String> = RightPrism::new();
        let cloned = prism;
        let right: Either<i32, String> = Either::Right("hello".to_string());
        assert_eq!(cloned.preview(&right), Some(&"hello".to_string()));
    }

    #[test]
    fn test_right_prism_debug() {
        let prism: RightPrism<i32, String> = RightPrism::new();
        let debug_string = format!("{prism:?}");
        assert!(debug_string.contains("RightPrism"));
    }

    #[test]
    fn test_right_prism_convenience_function() {
        let prism = right_prism::<i32, String>();
        let right: Either<i32, String> = Either::Right("hello".to_string());
        assert_eq!(prism.preview(&right), Some(&"hello".to_string()));
    }

    // =========================================================================
    // Prism Laws Tests for LeftPrism
    // =========================================================================

    #[test]
    fn test_left_prism_preview_review_law() {
        // Law: prism.preview(&prism.review(value)) == Some(&value)
        let prism = left_prism::<i32, String>();
        let value = 42;
        let reviewed = prism.review(value);
        assert_eq!(prism.preview(&reviewed), Some(&42));
    }

    #[test]
    fn test_left_prism_review_preview_law() {
        // Law: if prism.preview(source).is_some() then
        //      prism.review(prism.preview(source).unwrap().clone()) == source
        let prism = left_prism::<i32, String>();
        let source: Either<i32, String> = Either::Left(42);

        if let Some(value) = prism.preview(&source) {
            let reconstructed = prism.review(*value);
            assert_eq!(reconstructed, source);
        }
    }

    // =========================================================================
    // Prism Laws Tests for RightPrism
    // =========================================================================

    #[test]
    fn test_right_prism_preview_review_law() {
        // Law: prism.preview(&prism.review(value)) == Some(&value)
        let prism = right_prism::<i32, String>();
        let value = "hello".to_string();
        let reviewed = prism.review(value.clone());
        assert_eq!(prism.preview(&reviewed), Some(&value));
    }

    #[test]
    fn test_right_prism_review_preview_law() {
        // Law: if prism.preview(source).is_some() then
        //      prism.review(prism.preview(source).unwrap().clone()) == source
        let prism = right_prism::<i32, String>();
        let source: Either<i32, String> = Either::Right("hello".to_string());

        if let Some(value) = prism.preview(&source) {
            let reconstructed = prism.review(value.clone());
            assert_eq!(reconstructed, source);
        }
    }

    // =========================================================================
    // ChoicePrism Tests
    // =========================================================================

    #[allow(clippy::type_complexity)]
    fn identity_prism_int() -> FunctionPrism<
        i32,
        i32,
        impl Fn(&i32) -> Option<&i32>,
        impl Fn(i32) -> i32,
        impl Fn(i32) -> Option<i32>,
    > {
        FunctionPrism::new(|n: &i32| Some(n), |n: i32| n, |n: i32| Some(n))
    }

    #[allow(clippy::type_complexity)]
    fn identity_prism_string() -> FunctionPrism<
        String,
        String,
        impl Fn(&String) -> Option<&String>,
        impl Fn(String) -> String,
        impl Fn(String) -> Option<String>,
    > {
        FunctionPrism::new(|s: &String| Some(s), |s: String| s, |s: String| Some(s))
    }

    #[allow(clippy::type_complexity)]
    fn even_prism() -> FunctionPrism<
        i32,
        i32,
        impl Fn(&i32) -> Option<&i32>,
        impl Fn(i32) -> i32,
        impl Fn(i32) -> Option<i32>,
    > {
        FunctionPrism::new(
            |n: &i32| if *n % 2 == 0 { Some(n) } else { None },
            |n: i32| n,
            |n: i32| if n % 2 == 0 { Some(n) } else { None },
        )
    }

    #[allow(clippy::type_complexity)]
    fn non_empty_prism() -> FunctionPrism<
        String,
        String,
        impl Fn(&String) -> Option<&String>,
        impl Fn(String) -> String,
        impl Fn(String) -> Option<String>,
    > {
        FunctionPrism::new(
            |s: &String| if s.is_empty() { None } else { Some(s) },
            |s: String| s,
            |s: String| if s.is_empty() { None } else { Some(s) },
        )
    }

    #[test]
    fn test_choice_prism_left_success() {
        let combined = choice(identity_prism_int(), identity_prism_string());
        let left: Either<i32, String> = Either::Left(42);

        let result = combined.preview_owned(left);
        assert_eq!(result, Some(Either::Left(42)));
    }

    #[test]
    fn test_choice_prism_right_success() {
        let combined = choice(identity_prism_int(), identity_prism_string());
        let right: Either<i32, String> = Either::Right("hello".to_string());

        let result = combined.preview_owned(right);
        assert_eq!(result, Some(Either::Right("hello".to_string())));
    }

    #[test]
    fn test_choice_prism_left_fail() {
        let combined = choice(even_prism(), identity_prism_string());
        let left_odd: Either<i32, String> = Either::Left(41);

        let result = combined.preview_owned(left_odd);
        assert_eq!(result, None);
    }

    #[test]
    fn test_choice_prism_right_fail() {
        let combined = choice(identity_prism_int(), non_empty_prism());
        let right_empty: Either<i32, String> = Either::Right(String::new());

        let result = combined.preview_owned(right_empty);
        assert_eq!(result, None);
    }

    #[test]
    fn test_choice_prism_review_left() {
        let combined = choice(identity_prism_int(), identity_prism_string());
        let result = combined.review(Either::Left(42));
        assert_eq!(result, Either::Left(42));
    }

    #[test]
    fn test_choice_prism_review_right() {
        let combined = choice(identity_prism_int(), identity_prism_string());
        let result = combined.review(Either::Right("hello".to_string()));
        assert_eq!(result, Either::Right("hello".to_string()));
    }

    #[test]
    fn test_choice_prism_preview_returns_none() {
        // The reference-returning preview cannot work for ChoicePrism
        // because we would need to allocate a new Either
        let combined = choice(identity_prism_int(), identity_prism_string());
        let left: Either<i32, String> = Either::Left(42);

        let result = combined.preview(&left);
        assert!(result.is_none());
    }

    #[test]
    fn test_left_and_right_prism_clone() {
        // LeftPrism and RightPrism implement Clone
        let left_p = left_prism::<i32, String>();
        let right_p = right_prism::<i32, String>();

        let cloned_left = left_p;
        let cloned_right = right_p;

        let left: Either<i32, String> = Either::Left(42);
        let right: Either<i32, String> = Either::Right("hello".to_string());

        assert_eq!(cloned_left.preview(&left), Some(&42));
        assert_eq!(cloned_right.preview(&right), Some(&"hello".to_string()));
    }

    #[test]
    #[allow(clippy::type_complexity, clippy::redundant_clone)]
    fn test_choice_prism_clone() {
        let left_p = left_prism::<i32, String>();
        let right_p = right_prism::<String, i32>();

        let combined: ChoicePrism<
            LeftPrism<i32, String>,
            RightPrism<String, i32>,
            Either<i32, String>,
            Either<String, i32>,
            i32,
            i32,
        > = ChoicePrism::new(left_p, right_p);

        let cloned = combined.clone();

        let left: Either<Either<i32, String>, Either<String, i32>> = Either::Left(Either::Left(42));
        let result = cloned.preview_owned(left);
        assert_eq!(result, Some(Either::Left(42)));

        let right: Either<Either<i32, String>, Either<String, i32>> =
            Either::Right(Either::Right(100));
        let result = cloned.preview_owned(right);
        assert_eq!(result, Some(Either::Right(100)));
    }

    #[test]
    fn test_choice_prism_debug() {
        let combined = choice(identity_prism_int(), identity_prism_string());
        let debug_string = format!("{combined:?}");
        assert!(debug_string.contains("ChoicePrism"));
    }
}
