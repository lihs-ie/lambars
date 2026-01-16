//! Alternative type class - monoid structure on Applicative functors.
//!
//! This module provides the `Alternative` trait, which extends `Applicative` with
//! the ability to:
//!
//! - Represent failure or empty computation (`empty`)
//! - Choose between alternatives (`alt`)
//! - Filter computations based on conditions (`guard`)
//! - Make computations optional (`optional`)
//! - Choose from multiple alternatives (`choice`)
//!
//! `Alternative` is essential for parser combinators and non-deterministic computations.
//!
//! # Laws
//!
//! All `Alternative` implementations must satisfy these laws:
//!
//! ## Left Identity Law
//!
//! The empty value is the left identity for alt:
//!
//! ```text
//! empty.alt(x) == x
//! ```
//!
//! ## Right Identity Law
//!
//! The empty value is the right identity for alt:
//!
//! ```text
//! x.alt(empty) == x
//! ```
//!
//! ## Associativity Law
//!
//! The alt operation is associative:
//!
//! ```text
//! (x.alt(y)).alt(z) == x.alt(y.alt(z))
//! ```
//!
//! ## Recommended Laws (for well-behaved instances)
//!
//! ### Left Absorption
//!
//! ```text
//! empty.apply(x) == empty
//! ```
//!
//! ### Right Absorption
//!
//! ```text
//! ff.apply(empty) == empty
//! ```
//!
//! ### Left Distributivity
//!
//! ```text
//! (fa.alt(fb)).fmap(f) == fa.fmap(f).alt(fb.fmap(f))
//! ```
//!
//! # Examples
//!
//! ```rust
//! use lambars::typeclass::{Alternative, Functor};
//!
//! // Using empty as a failure value
//! let empty: Option<i32> = <Option<()>>::empty();
//! assert_eq!(empty, None);
//!
//! // Using alt for fallback
//! let first: Option<i32> = None;
//! let second: Option<i32> = Some(42);
//! assert_eq!(first.alt(second), Some(42));
//!
//! // Using guard for conditional filtering
//! fn filter_positive(n: i32) -> Option<i32> {
//!     <Option<()>>::guard(n > 0).fmap(move |_| n)
//! }
//! assert_eq!(filter_positive(5), Some(5));
//! assert_eq!(filter_positive(-3), None);
//! ```

use super::applicative::Applicative;

/// A type class for applicative functors with a monoid structure.
///
/// `Alternative` extends `Applicative` with the ability to represent failure
/// and combine computations with choice semantics.
///
/// # Laws
///
/// ## Left Identity
///
/// Empty is the left identity:
///
/// ```text
/// empty.alt(x) == x
/// ```
///
/// ## Right Identity
///
/// Empty is the right identity:
///
/// ```text
/// x.alt(empty) == x
/// ```
///
/// ## Associativity
///
/// alt is associative:
///
/// ```text
/// (x.alt(y)).alt(z) == x.alt(y.alt(z))
/// ```
///
/// # Examples
///
/// ```rust
/// use lambars::typeclass::Alternative;
///
/// // Option as Alternative
/// let first: Option<i32> = None;
/// let second = Some(42);
/// assert_eq!(first.alt(second), Some(42));
///
/// let first = Some(1);
/// let second = Some(2);
/// assert_eq!(first.alt(second), Some(1));
/// ```
pub trait Alternative: Applicative {
    /// Returns the identity element for alt.
    ///
    /// This represents a failed or empty computation in the Alternative context.
    ///
    /// # Returns
    ///
    /// The empty/failure value for this Alternative type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Alternative;
    ///
    /// let empty: Option<i32> = <Option<()>>::empty();
    /// assert_eq!(empty, None);
    /// ```
    fn empty<A>() -> Self::WithType<A>
    where
        A: 'static;

    /// Combines two alternatives, returning the first success.
    ///
    /// For `Option`, this returns `self` if it is `Some`, otherwise `alternative`.
    ///
    /// Note: `Vec` provides Alternative-like operations through the separate
    /// `AlternativeVec` extension trait due to Rust's orphan rules.
    ///
    /// # Arguments
    ///
    /// * `alternative` - The fallback value to use if `self` represents failure
    ///
    /// # Returns
    ///
    /// The result of combining the two alternatives.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Alternative;
    ///
    /// // First success wins
    /// let first: Option<i32> = None;
    /// let second: Option<i32> = Some(42);
    /// assert_eq!(first.alt(second), Some(42));
    ///
    /// // Already successful, alternative is ignored
    /// let first = Some(1);
    /// let second = Some(2);
    /// assert_eq!(first.alt(second), Some(1));
    /// ```
    #[must_use]
    fn alt(self, alternative: Self) -> Self;

    /// Conditionally succeeds with `()` or fails.
    ///
    /// Returns `pure(())` if the condition is true, otherwise `empty`.
    /// This is useful for conditional filtering in applicative/monadic computations.
    ///
    /// # Arguments
    ///
    /// * `condition` - The condition to check
    ///
    /// # Returns
    ///
    /// `pure(())` if condition is true, `empty` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::{Alternative, Functor};
    ///
    /// fn filter_positive(n: i32) -> Option<i32> {
    ///     <Option<()>>::guard(n > 0).fmap(move |_| n)
    /// }
    ///
    /// assert_eq!(filter_positive(5), Some(5));
    /// assert_eq!(filter_positive(-3), None);
    /// ```
    #[inline]
    #[must_use]
    fn guard(condition: bool) -> Self::WithType<()>
    where
        Self: Sized,
    {
        if condition {
            Self::pure(())
        } else {
            Self::empty()
        }
    }

    /// Makes a computation optional, converting failure to `None`.
    ///
    /// Returns `self.fmap(Some).alt(pure(None))`, which always succeeds
    /// but wraps the result in `Option` to indicate whether the original
    /// computation succeeded.
    ///
    /// # Returns
    ///
    /// A computation that always succeeds with `Some(value)` if the original
    /// succeeded, or `None` if it failed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Alternative;
    ///
    /// let success: Option<i32> = Some(42);
    /// assert_eq!(success.optional(), Some(Some(42)));
    ///
    /// let failure: Option<i32> = None;
    /// assert_eq!(failure.optional(), Some(None));
    /// ```
    fn optional(self) -> Self::WithType<Option<Self::Inner>>
    where
        Self: Sized,
        Self::Inner: 'static;

    /// Chooses from multiple alternatives, returning the first success.
    ///
    /// Folds over the alternatives using `alt`, starting from `empty`.
    /// For `Option`, this returns the first `Some` value.
    ///
    /// Note: `Vec` provides `choice` through the separate `AlternativeVec`
    /// extension trait.
    ///
    /// # Arguments
    ///
    /// * `alternatives` - An iterator of alternative values
    ///
    /// # Returns
    ///
    /// The combined result of all alternatives.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Alternative;
    ///
    /// let alternatives = vec![None, Some(1), Some(2)];
    /// let result: Option<i32> = Option::choice(alternatives);
    /// assert_eq!(result, Some(1));
    ///
    /// let all_none: Vec<Option<i32>> = vec![None, None, None];
    /// let result: Option<i32> = Option::choice(all_none);
    /// assert_eq!(result, None);
    /// ```
    fn choice<I>(alternatives: I) -> Self
    where
        I: IntoIterator<Item = Self>,
        Self: Sized,
        Self::Inner: 'static;
}

impl<A> Alternative for Option<A> {
    #[inline]
    fn empty<B>() -> Option<B>
    where
        B: 'static,
    {
        None
    }

    #[inline]
    fn alt(self, alternative: Self) -> Self {
        self.or(alternative)
    }

    #[inline]
    fn optional(self) -> Option<Self>
    where
        A: 'static,
    {
        Some(self)
    }

    #[inline]
    fn choice<I>(alternatives: I) -> Self
    where
        I: IntoIterator<Item = Self>,
        Self::Inner: 'static,
    {
        alternatives.into_iter().find(Self::is_some).flatten()
    }
}

/// Extension trait for Vec to provide Alternative-like operations.
///
/// Vec's Alternative instance represents non-deterministic computation:
/// `alt` concatenates the two vectors (combining all possibilities).
///
/// This is a separate trait from `Alternative` because `Vec` does not
/// implement `Applicative` in the standard `Alternative` trait hierarchy.
pub trait AlternativeVec: Sized {
    /// The inner type of the Vec.
    type VecInner;

    /// Returns the identity element for alt (empty vector).
    ///
    /// # Returns
    ///
    /// An empty Vec.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::AlternativeVec;
    ///
    /// let empty: Vec<i32> = Vec::<()>::empty();
    /// assert!(empty.is_empty());
    /// ```
    #[must_use]
    fn empty<B>() -> Vec<B> {
        Vec::new()
    }

    /// Combines two vectors by concatenation.
    ///
    /// This represents non-deterministic choice: combining all possibilities
    /// from both vectors.
    ///
    /// # Arguments
    ///
    /// * `alternative` - The second vector to append
    ///
    /// # Returns
    ///
    /// A new vector containing elements from both vectors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::AlternativeVec;
    ///
    /// let first = vec![1, 2];
    /// let second = vec![3, 4];
    /// assert_eq!(first.alt(second), vec![1, 2, 3, 4]);
    /// ```
    #[must_use]
    fn alt(self, alternative: Self) -> Self;

    /// Conditionally succeeds with `()` or returns empty.
    ///
    /// Returns `vec![()]` if the condition is true, otherwise `vec![]`.
    ///
    /// # Arguments
    ///
    /// * `condition` - The condition to check
    ///
    /// # Returns
    ///
    /// `vec![()]` if condition is true, `vec![]` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::AlternativeVec;
    ///
    /// let result: Vec<()> = Vec::<()>::guard(true);
    /// assert_eq!(result, vec![()]);
    ///
    /// let result: Vec<()> = Vec::<()>::guard(false);
    /// assert!(result.is_empty());
    /// ```
    #[must_use]
    fn guard(condition: bool) -> Vec<()> {
        if condition { vec![()] } else { Vec::new() }
    }

    /// Makes a computation optional.
    ///
    /// Returns a vector where each element is wrapped in `Some`,
    /// plus `None` to represent the empty case.
    ///
    /// # Returns
    ///
    /// A vector containing `Some(x)` for each element `x`, plus `None`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::AlternativeVec;
    ///
    /// let values = vec![1, 2];
    /// let result: Vec<Option<i32>> = values.optional();
    /// assert!(result.contains(&Some(1)));
    /// assert!(result.contains(&Some(2)));
    /// assert!(result.contains(&None));
    /// ```
    fn optional(self) -> Vec<Option<Self::VecInner>>;

    /// Chooses from multiple alternatives by concatenating all.
    ///
    /// For Vec, this concatenates all vectors in the iterator.
    ///
    /// # Arguments
    ///
    /// * `alternatives` - An iterator of vectors
    ///
    /// # Returns
    ///
    /// A vector containing all elements from all input vectors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::AlternativeVec;
    ///
    /// let alternatives = vec![vec![1, 2], vec![3], vec![4, 5]];
    /// let result: Vec<i32> = Vec::choice(alternatives);
    /// assert_eq!(result, vec![1, 2, 3, 4, 5]);
    /// ```
    fn choice<I>(alternatives: I) -> Self
    where
        I: IntoIterator<Item = Self>;
}

impl<T> AlternativeVec for Vec<T> {
    type VecInner = T;

    #[inline]
    fn alt(self, alternative: Self) -> Self {
        self.into_iter().chain(alternative).collect()
    }

    #[inline]
    fn optional(self) -> Vec<Option<T>> {
        self.into_iter()
            .map(Some)
            .chain(std::iter::once(None))
            .collect()
    }

    #[inline]
    fn choice<I>(alternatives: I) -> Self
    where
        I: IntoIterator<Item = Self>,
    {
        alternatives.into_iter().flatten().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn option_empty_is_none() {
        let empty: Option<i32> = <Option<()>>::empty();
        assert_eq!(empty, None);
    }

    #[rstest]
    fn option_empty_with_string() {
        let empty: Option<String> = <Option<()>>::empty();
        assert_eq!(empty, None);
    }

    #[rstest]
    fn option_alt_none_some() {
        let first: Option<i32> = None;
        let second: Option<i32> = Some(42);
        assert_eq!(first.alt(second), Some(42));
    }

    #[rstest]
    fn option_alt_some_none() {
        let first: Option<i32> = Some(1);
        let second: Option<i32> = None;
        assert_eq!(first.alt(second), Some(1));
    }

    #[rstest]
    fn option_alt_some_some() {
        let first: Option<i32> = Some(1);
        let second: Option<i32> = Some(2);
        assert_eq!(first.alt(second), Some(1));
    }

    #[rstest]
    fn option_alt_none_none() {
        let first: Option<i32> = None;
        let second: Option<i32> = None;
        assert_eq!(first.alt(second), None);
    }

    #[rstest]
    fn option_guard_true() {
        let result: Option<()> = <Option<()>>::guard(true);
        assert_eq!(result, Some(()));
    }

    #[rstest]
    fn option_guard_false() {
        let result: Option<()> = <Option<()>>::guard(false);
        assert_eq!(result, None);
    }

    #[rstest]
    fn option_optional_some() {
        let value: Option<i32> = Some(42);
        assert_eq!(value.optional(), Some(Some(42)));
    }

    #[rstest]
    fn option_optional_none() {
        let value: Option<i32> = None;
        assert_eq!(value.optional(), Some(None));
    }

    #[rstest]
    fn option_choice_finds_first_some() {
        let alternatives = vec![None, Some(1), Some(2)];
        let result: Option<i32> = Option::choice(alternatives);
        assert_eq!(result, Some(1));
    }

    #[rstest]
    fn option_choice_all_none() {
        let alternatives: Vec<Option<i32>> = vec![None, None, None];
        let result: Option<i32> = Option::choice(alternatives);
        assert_eq!(result, None);
    }

    #[rstest]
    fn option_choice_empty_iterator() {
        let alternatives: Vec<Option<i32>> = vec![];
        let result: Option<i32> = Option::choice(alternatives);
        assert_eq!(result, None);
    }

    #[rstest]
    fn option_choice_first_is_some() {
        let alternatives = vec![Some(1), Some(2), Some(3)];
        let result: Option<i32> = Option::choice(alternatives);
        assert_eq!(result, Some(1));
    }

    #[rstest]
    fn vec_empty_is_empty_vec() {
        let empty: Vec<i32> = Vec::<()>::empty();
        assert!(empty.is_empty());
    }

    #[rstest]
    fn vec_empty_with_string() {
        let empty: Vec<String> = Vec::<()>::empty();
        assert!(empty.is_empty());
    }

    #[rstest]
    fn vec_alt_concatenates() {
        let first = vec![1, 2];
        let second = vec![3, 4];
        assert_eq!(first.alt(second), vec![1, 2, 3, 4]);
    }

    #[rstest]
    fn vec_alt_empty_first() {
        let first: Vec<i32> = vec![];
        let second = vec![3, 4];
        assert_eq!(first.alt(second), vec![3, 4]);
    }

    #[rstest]
    fn vec_alt_empty_second() {
        let first = vec![1, 2];
        let second: Vec<i32> = vec![];
        assert_eq!(first.alt(second), vec![1, 2]);
    }

    #[rstest]
    fn vec_alt_both_empty() {
        let first: Vec<i32> = vec![];
        let second: Vec<i32> = vec![];
        assert!(first.alt(second).is_empty());
    }

    #[rstest]
    fn vec_guard_true() {
        let result: Vec<()> = Vec::<()>::guard(true);
        assert_eq!(result, vec![()]);
    }

    #[rstest]
    fn vec_guard_false() {
        let result: Vec<()> = Vec::<()>::guard(false);
        assert!(result.is_empty());
    }

    #[rstest]
    fn vec_optional_non_empty() {
        let value = vec![1, 2, 3];
        let result: Vec<Option<i32>> = value.optional();
        assert_eq!(result, vec![Some(1), Some(2), Some(3), None]);
    }

    #[rstest]
    fn vec_optional_empty() {
        let value: Vec<i32> = vec![];
        let result: Vec<Option<i32>> = value.optional();
        assert_eq!(result, vec![None]);
    }

    #[rstest]
    fn vec_choice_concatenates_all() {
        let alternatives = vec![vec![1, 2], vec![3], vec![4, 5, 6]];
        let result: Vec<i32> = Vec::choice(alternatives);
        assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
    }

    #[rstest]
    fn vec_choice_with_empty() {
        let alternatives = vec![vec![], vec![1], vec![]];
        let result: Vec<i32> = Vec::choice(alternatives);
        assert_eq!(result, vec![1]);
    }

    #[rstest]
    fn vec_choice_empty_iterator() {
        let alternatives: Vec<Vec<i32>> = vec![];
        let result: Vec<i32> = Vec::choice(alternatives);
        assert!(result.is_empty());
    }

    #[rstest]
    fn option_filter_positive_with_guard() {
        use crate::typeclass::Functor;

        fn filter_positive(n: i32) -> Option<i32> {
            <Option<()>>::guard(n > 0).fmap(move |()| n)
        }

        assert_eq!(filter_positive(5), Some(5));
        assert_eq!(filter_positive(-3), None);
        assert_eq!(filter_positive(0), None);
    }

    #[rstest]
    fn option_fallback_chain() {
        fn try_parse_int(s: &str) -> Option<i32> {
            s.parse().ok()
        }

        let input = "not a number";
        let result = try_parse_int(input).alt(Some(0));
        assert_eq!(result, Some(0));

        let input = "42";
        let result = try_parse_int(input).alt(Some(0));
        assert_eq!(result, Some(42));
    }

    #[rstest]
    fn vec_nondeterministic_computation() {
        // Simulate non-deterministic choice
        let path_a = vec![1, 2];
        let path_b = vec![3, 4];
        let all_paths = path_a.alt(path_b);
        assert_eq!(all_paths, vec![1, 2, 3, 4]);
    }

    #[rstest]
    fn option_choice_parser_combinator_style() {
        // Simulating parser combinators: try multiple parsers
        fn parse_keyword<'a>(input: &'a str, keyword: &'a str) -> Option<&'a str> {
            if input.starts_with(keyword) {
                Some(keyword)
            } else {
                None
            }
        }

        let input = "if x then y";
        let keywords = vec!["while", "for", "if", "else"];
        let parsers: Vec<Option<&str>> = keywords
            .into_iter()
            .map(|kw| parse_keyword(input, kw))
            .collect();

        let result = Option::choice(parsers);
        assert_eq!(result, Some("if"));
    }
}
