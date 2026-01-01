//! Semigroup type class - types with an associative binary operation.
//!
//! A semigroup is an algebraic structure consisting of a set together with
//! an associative binary operation. In programming terms, a type `T` is a
//! semigroup if there exists a function `combine: (T, T) -> T` that is
//! associative.
//!
//! # Laws
//!
//! For all `a`, `b`, `c` of type `T`:
//!
//! ## Associativity
//!
//! ```text
//! (a.combine(b)).combine(c) == a.combine(b.combine(c))
//! ```
//!
//! # Examples
//!
//! ```rust
//! use lambars::typeclass::Semigroup;
//!
//! // String concatenation
//! let hello = String::from("Hello, ");
//! let world = String::from("World!");
//! assert_eq!(hello.combine(world), "Hello, World!");
//!
//! // Vec concatenation
//! let vec1 = vec![1, 2];
//! let vec2 = vec![3, 4];
//! assert_eq!(vec1.combine(vec2), vec![1, 2, 3, 4]);
//! ```

use std::ops::{Add, Mul};

use super::Identity;
use super::wrappers::{Max, Min, Product, Sum};

/// A type class for types with an associative binary operation.
///
/// # Laws
///
/// All implementations must satisfy:
///
/// ## Associativity
///
/// For all `a`, `b`, `c`:
/// ```text
/// (a.combine(b)).combine(c) == a.combine(b.combine(c))
/// ```
///
/// # Examples
///
/// ```rust
/// use lambars::typeclass::Semigroup;
///
/// let a = String::from("foo");
/// let b = String::from("bar");
/// assert_eq!(a.combine(b), "foobar");
/// ```
pub trait Semigroup {
    /// Combines two values into one.
    ///
    /// This operation must be associative.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Semigroup;
    ///
    /// let result = String::from("Hello, ").combine(String::from("World!"));
    /// assert_eq!(result, "Hello, World!");
    /// ```
    #[must_use]
    fn combine(self, other: Self) -> Self;

    /// Combines two values by reference, returning a new value.
    ///
    /// The default implementation clones both values and calls `combine`.
    /// Types can override this for more efficient implementations.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Semigroup;
    ///
    /// let a = String::from("Hello, ");
    /// let b = String::from("World!");
    /// let result = a.combine_ref(&b);
    /// // Original values are still available
    /// assert_eq!(a, "Hello, ");
    /// assert_eq!(result, "Hello, World!");
    /// ```
    #[must_use]
    fn combine_ref(&self, other: &Self) -> Self
    where
        Self: Clone,
    {
        self.clone().combine(other.clone())
    }

    /// Combines a value with itself `n` times.
    ///
    /// `combine_n(x, 1)` returns `x`.
    /// `combine_n(x, 2)` returns `x.combine(x)`.
    /// `combine_n(x, 3)` returns `x.combine(x).combine(x)`.
    ///
    /// # Panics
    ///
    /// Panics if `count` is 0.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Semigroup;
    ///
    /// let s = String::from("ab");
    /// assert_eq!(s.combine_n(3), "ababab");
    /// ```
    #[must_use]
    fn combine_n(self, count: usize) -> Self
    where
        Self: Clone,
    {
        assert!(count > 0, "combine_n requires count > 0");

        if count == 1 {
            return self;
        }

        let mut result = self.clone();
        for _ in 1..count {
            result = result.combine(self.clone());
        }
        result
    }

    /// Reduces all elements in an iterator using the semigroup operation.
    ///
    /// Returns `None` if the iterator is empty.
    /// For a version that returns a default value for empty iterators, see
    /// [`Monoid::combine_all`](super::Monoid::combine_all).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Semigroup;
    ///
    /// let strings = vec![
    ///     String::from("a"),
    ///     String::from("b"),
    ///     String::from("c"),
    /// ];
    /// assert_eq!(String::reduce_all(strings), Some(String::from("abc")));
    ///
    /// let empty: Vec<String> = vec![];
    /// assert_eq!(String::reduce_all(empty), None);
    /// ```
    fn reduce_all<I>(iterator: I) -> Option<Self>
    where
        I: IntoIterator<Item = Self>,
        Self: Sized,
    {
        iterator
            .into_iter()
            .reduce(|accumulator, element| accumulator.combine(element))
    }
}

// =============================================================================
// String Implementation
// =============================================================================

impl Semigroup for String {
    fn combine(mut self, other: Self) -> Self {
        self.push_str(&other);
        self
    }

    fn combine_ref(&self, other: &Self) -> Self {
        let mut result = Self::with_capacity(self.len() + other.len());
        result.push_str(self);
        result.push_str(other);
        result
    }
}

// =============================================================================
// Vec Implementation
// =============================================================================

impl<T: Clone> Semigroup for Vec<T> {
    fn combine(mut self, mut other: Self) -> Self {
        self.append(&mut other);
        self
    }

    fn combine_ref(&self, other: &Self) -> Self {
        let mut result = Self::with_capacity(self.len() + other.len());
        result.extend(self.iter().cloned());
        result.extend(other.iter().cloned());
        result
    }
}

// =============================================================================
// Option Implementation
// =============================================================================

/// Option forms a semigroup when its inner type is a semigroup.
///
/// The combination follows these rules:
/// - `Some(a).combine(Some(b))` = `Some(a.combine(b))`
/// - `Some(a).combine(None)` = `Some(a)`
/// - `None.combine(Some(b))` = `Some(b)`
/// - `None.combine(None)` = `None`
impl<T: Semigroup> Semigroup for Option<T> {
    fn combine(self, other: Self) -> Self {
        match (self, other) {
            (Some(left), Some(right)) => Some(left.combine(right)),
            (Some(value), None) | (None, Some(value)) => Some(value),
            (None, None) => None,
        }
    }
}

// =============================================================================
// Result Implementation
// =============================================================================

/// Result forms a semigroup when its success type is a semigroup.
///
/// The combination follows these rules:
/// - `Ok(a).combine(Ok(b))` = `Ok(a.combine(b))`
/// - `Err(e).combine(_)` = `Err(e)` (first error wins)
/// - `Ok(_).combine(Err(e))` = `Err(e)`
impl<T: Semigroup, E> Semigroup for Result<T, E> {
    fn combine(self, other: Self) -> Self {
        match (self, other) {
            (Ok(left), Ok(right)) => Ok(left.combine(right)),
            (Err(error), _) | (_, Err(error)) => Err(error),
        }
    }
}

// =============================================================================
// Unit Type Implementation
// =============================================================================

/// The unit type forms a trivial semigroup.
impl Semigroup for () {
    fn combine(self, _other: Self) -> Self {}
}

// =============================================================================
// Identity Implementation
// =============================================================================

/// Identity forms a semigroup when its inner type is a semigroup.
impl<T: Semigroup> Semigroup for Identity<T> {
    fn combine(self, other: Self) -> Self {
        Self(self.0.combine(other.0))
    }
}

// =============================================================================
// Numeric Wrapper Implementations
// =============================================================================

/// Sum forms a semigroup under addition.
impl<A: Add<Output = A>> Semigroup for Sum<A> {
    fn combine(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

/// Product forms a semigroup under multiplication.
impl<A: Mul<Output = A>> Semigroup for Product<A> {
    fn combine(self, other: Self) -> Self {
        Self(self.0 * other.0)
    }
}

/// Max forms a semigroup by taking the maximum value.
impl<A: Ord> Semigroup for Max<A> {
    fn combine(self, other: Self) -> Self {
        if self.0 >= other.0 { self } else { other }
    }
}

/// Min forms a semigroup by taking the minimum value.
impl<A: Ord> Semigroup for Min<A> {
    fn combine(self, other: Self) -> Self {
        if self.0 <= other.0 { self } else { other }
    }
}

// =============================================================================
// Tuple Implementations
// =============================================================================

/// Tuples form a semigroup when all their elements are semigroups.
impl<A: Semigroup, B: Semigroup> Semigroup for (A, B) {
    fn combine(self, other: Self) -> Self {
        (self.0.combine(other.0), self.1.combine(other.1))
    }
}

impl<A: Semigroup, B: Semigroup, C: Semigroup> Semigroup for (A, B, C) {
    fn combine(self, other: Self) -> Self {
        (
            self.0.combine(other.0),
            self.1.combine(other.1),
            self.2.combine(other.2),
        )
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
    // String Semigroup Tests
    // =========================================================================

    #[rstest]
    fn string_combine_concatenates() {
        let left = String::from("Hello, ");
        let right = String::from("World!");
        assert_eq!(left.combine(right), "Hello, World!");
    }

    #[rstest]
    fn string_combine_with_empty() {
        let left = String::from("Hello");
        let right = String::new();
        assert_eq!(left.combine(right), "Hello");
    }

    #[rstest]
    fn string_combine_ref_preserves_originals() {
        let left = String::from("Hello, ");
        let right = String::from("World!");
        let result = left.combine_ref(&right);
        assert_eq!(result, "Hello, World!");
        assert_eq!(left, "Hello, ");
        assert_eq!(right, "World!");
    }

    #[rstest]
    fn string_combine_n_single() {
        let value = String::from("ab");
        assert_eq!(value.combine_n(1), "ab");
    }

    #[rstest]
    fn string_combine_n_multiple() {
        let value = String::from("ab");
        assert_eq!(value.combine_n(3), "ababab");
    }

    #[rstest]
    #[should_panic(expected = "combine_n requires count > 0")]
    fn string_combine_n_zero_panics() {
        let value = String::from("ab");
        let _ = value.combine_n(0);
    }

    // =========================================================================
    // Vec Semigroup Tests
    // =========================================================================

    #[rstest]
    fn vec_combine_concatenates() {
        let left = vec![1, 2];
        let right = vec![3, 4];
        assert_eq!(left.combine(right), vec![1, 2, 3, 4]);
    }

    #[rstest]
    fn vec_combine_with_empty() {
        let left: Vec<i32> = vec![1, 2];
        let right: Vec<i32> = vec![];
        assert_eq!(left.combine(right), vec![1, 2]);
    }

    #[rstest]
    fn vec_combine_ref_preserves_originals() {
        let left = vec![1, 2];
        let right = vec![3, 4];
        let result = left.combine_ref(&right);
        assert_eq!(result, vec![1, 2, 3, 4]);
        assert_eq!(left, vec![1, 2]);
        assert_eq!(right, vec![3, 4]);
    }

    // =========================================================================
    // Option Semigroup Tests
    // =========================================================================

    #[rstest]
    fn option_combine_some_some() {
        let left: Option<String> = Some(String::from("Hello, "));
        let right: Option<String> = Some(String::from("World!"));
        assert_eq!(left.combine(right), Some(String::from("Hello, World!")));
    }

    #[rstest]
    fn option_combine_some_none() {
        let left: Option<String> = Some(String::from("Hello"));
        let right: Option<String> = None;
        assert_eq!(left.combine(right), Some(String::from("Hello")));
    }

    #[rstest]
    fn option_combine_none_some() {
        let left: Option<String> = None;
        let right: Option<String> = Some(String::from("World"));
        assert_eq!(left.combine(right), Some(String::from("World")));
    }

    #[rstest]
    fn option_combine_none_none() {
        let left: Option<String> = None;
        let right: Option<String> = None;
        assert_eq!(left.combine(right), None);
    }

    // =========================================================================
    // Result Semigroup Tests
    // =========================================================================

    #[rstest]
    fn result_combine_ok_ok() {
        let left: Result<String, &str> = Ok(String::from("Hello, "));
        let right: Result<String, &str> = Ok(String::from("World!"));
        assert_eq!(left.combine(right), Ok(String::from("Hello, World!")));
    }

    #[rstest]
    fn result_combine_err_ok() {
        let left: Result<String, &str> = Err("error1");
        let right: Result<String, &str> = Ok(String::from("World"));
        assert_eq!(left.combine(right), Err("error1"));
    }

    #[rstest]
    fn result_combine_ok_err() {
        let left: Result<String, &str> = Ok(String::from("Hello"));
        let right: Result<String, &str> = Err("error2");
        assert_eq!(left.combine(right), Err("error2"));
    }

    #[rstest]
    fn result_combine_err_err() {
        let left: Result<String, &str> = Err("error1");
        let right: Result<String, &str> = Err("error2");
        // First error wins
        assert_eq!(left.combine(right), Err("error1"));
    }

    // =========================================================================
    // Unit Type Semigroup Tests
    // =========================================================================

    #[rstest]
    fn unit_combine() {
        let left = ();
        let right = ();
        assert_eq!(left.combine(right), ());
    }

    // =========================================================================
    // Identity Semigroup Tests
    // =========================================================================

    #[rstest]
    fn identity_combine() {
        let left = Identity::new(String::from("Hello, "));
        let right = Identity::new(String::from("World!"));
        assert_eq!(
            left.combine(right),
            Identity::new(String::from("Hello, World!"))
        );
    }

    // =========================================================================
    // Sum Semigroup Tests
    // =========================================================================

    #[rstest]
    fn sum_combine_adds() {
        let left = Sum::new(3);
        let right = Sum::new(5);
        assert_eq!(left.combine(right), Sum::new(8));
    }

    #[rstest]
    fn sum_combine_with_zero() {
        let left = Sum::new(42);
        let right = Sum::new(0);
        assert_eq!(left.combine(right), Sum::new(42));
    }

    #[rstest]
    fn sum_combine_negative() {
        let left = Sum::new(10);
        let right = Sum::new(-3);
        assert_eq!(left.combine(right), Sum::new(7));
    }

    // =========================================================================
    // Product Semigroup Tests
    // =========================================================================

    #[rstest]
    fn product_combine_multiplies() {
        let left = Product::new(3);
        let right = Product::new(5);
        assert_eq!(left.combine(right), Product::new(15));
    }

    #[rstest]
    fn product_combine_with_one() {
        let left = Product::new(42);
        let right = Product::new(1);
        assert_eq!(left.combine(right), Product::new(42));
    }

    #[rstest]
    fn product_combine_with_zero() {
        let left = Product::new(42);
        let right = Product::new(0);
        assert_eq!(left.combine(right), Product::new(0));
    }

    // =========================================================================
    // Max Semigroup Tests
    // =========================================================================

    #[rstest]
    fn max_combine_takes_maximum() {
        let left = Max::new(3);
        let right = Max::new(5);
        assert_eq!(left.combine(right), Max::new(5));
    }

    #[rstest]
    fn max_combine_equal_values() {
        let left = Max::new(5);
        let right = Max::new(5);
        assert_eq!(left.combine(right), Max::new(5));
    }

    #[rstest]
    fn max_combine_negative() {
        let left = Max::new(-10);
        let right = Max::new(-3);
        assert_eq!(left.combine(right), Max::new(-3));
    }

    // =========================================================================
    // Min Semigroup Tests
    // =========================================================================

    #[rstest]
    fn min_combine_takes_minimum() {
        let left = Min::new(3);
        let right = Min::new(5);
        assert_eq!(left.combine(right), Min::new(3));
    }

    #[rstest]
    fn min_combine_equal_values() {
        let left = Min::new(5);
        let right = Min::new(5);
        assert_eq!(left.combine(right), Min::new(5));
    }

    #[rstest]
    fn min_combine_negative() {
        let left = Min::new(-10);
        let right = Min::new(-3);
        assert_eq!(left.combine(right), Min::new(-10));
    }

    // =========================================================================
    // Tuple Semigroup Tests
    // =========================================================================

    #[rstest]
    fn tuple2_combine() {
        let left = (Sum::new(1), Product::new(2));
        let right = (Sum::new(3), Product::new(4));
        assert_eq!(left.combine(right), (Sum::new(4), Product::new(8)));
    }

    #[rstest]
    fn tuple3_combine() {
        let left = (Sum::new(1), Product::new(2), String::from("a"));
        let right = (Sum::new(3), Product::new(4), String::from("b"));
        assert_eq!(
            left.combine(right),
            (Sum::new(4), Product::new(8), String::from("ab"))
        );
    }

    // =========================================================================
    // reduce_all Tests
    // =========================================================================

    #[rstest]
    fn reduce_all_empty_returns_none() {
        let empty: Vec<String> = vec![];
        assert_eq!(String::reduce_all(empty), None);
    }

    #[rstest]
    fn reduce_all_single_element() {
        let single = vec![String::from("hello")];
        assert_eq!(String::reduce_all(single), Some(String::from("hello")));
    }

    #[rstest]
    fn reduce_all_multiple_elements() {
        let multiple = vec![String::from("a"), String::from("b"), String::from("c")];
        assert_eq!(String::reduce_all(multiple), Some(String::from("abc")));
    }

    #[rstest]
    fn reduce_all_sum() {
        let values = vec![Sum::new(1), Sum::new(2), Sum::new(3)];
        assert_eq!(Sum::reduce_all(values), Some(Sum::new(6)));
    }

    // =========================================================================
    // Associativity Law Tests
    // =========================================================================

    #[rstest]
    fn string_associativity() {
        let first = String::from("a");
        let second = String::from("b");
        let third = String::from("c");

        let left_associated = first.clone().combine(second.clone()).combine(third.clone());
        let right_associated = first.combine(second.combine(third));

        assert_eq!(left_associated, right_associated);
    }

    #[rstest]
    fn vec_associativity() {
        let first = vec![1];
        let second = vec![2];
        let third = vec![3];

        let left_associated = first.clone().combine(second.clone()).combine(third.clone());
        let right_associated = first.combine(second.combine(third));

        assert_eq!(left_associated, right_associated);
    }

    #[rstest]
    fn sum_associativity() {
        let first = Sum::new(1);
        let second = Sum::new(2);
        let third = Sum::new(3);

        let left_associated = first.combine(second).combine(third);
        let right_associated = Sum::new(1).combine(Sum::new(2).combine(Sum::new(3)));

        assert_eq!(left_associated, right_associated);
    }

    #[rstest]
    fn product_associativity() {
        let first = Product::new(2);
        let second = Product::new(3);
        let third = Product::new(4);

        let left_associated = first.combine(second).combine(third);
        let right_associated = Product::new(2).combine(Product::new(3).combine(Product::new(4)));

        assert_eq!(left_associated, right_associated);
    }

    #[rstest]
    fn max_associativity() {
        let first = Max::new(1);
        let second = Max::new(5);
        let third = Max::new(3);

        let left_associated = first.combine(second).combine(third);
        let right_associated = Max::new(1).combine(Max::new(5).combine(Max::new(3)));

        assert_eq!(left_associated, right_associated);
    }

    #[rstest]
    fn min_associativity() {
        let first = Min::new(5);
        let second = Min::new(1);
        let third = Min::new(3);

        let left_associated = first.combine(second).combine(third);
        let right_associated = Min::new(5).combine(Min::new(1).combine(Min::new(3)));

        assert_eq!(left_associated, right_associated);
    }

    #[rstest]
    fn option_associativity() {
        let first: Option<Sum<i32>> = Some(Sum::new(1));
        let second: Option<Sum<i32>> = Some(Sum::new(2));
        let third: Option<Sum<i32>> = Some(Sum::new(3));

        let left_associated = first.combine(second).combine(third);
        let right_associated =
            Some(Sum::new(1)).combine(Some(Sum::new(2)).combine(Some(Sum::new(3))));

        assert_eq!(left_associated, right_associated);
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
        fn prop_string_associativity(
            first in "\\PC*",
            second in "\\PC*",
            third in "\\PC*"
        ) {
            let left = first.clone().combine(second.clone()).combine(third.clone());
            let right = first.combine(second.combine(third));
            prop_assert_eq!(left, right);
        }

        #[test]
        fn prop_vec_i32_associativity(
            first in prop::collection::vec(any::<i32>(), 0..10),
            second in prop::collection::vec(any::<i32>(), 0..10),
            third in prop::collection::vec(any::<i32>(), 0..10)
        ) {
            let left = first.clone().combine(second.clone()).combine(third.clone());
            let right = first.combine(second.combine(third));
            prop_assert_eq!(left, right);
        }

        #[test]
        fn prop_sum_i32_associativity(
            first in -10000i32..10000i32,
            second in -10000i32..10000i32,
            third in -10000i32..10000i32
        ) {
            // Use small values to avoid overflow
            let left = Sum::new(first).combine(Sum::new(second)).combine(Sum::new(third));
            let right = Sum::new(first).combine(Sum::new(second).combine(Sum::new(third)));
            prop_assert_eq!(left, right);
        }

        #[test]
        fn prop_product_i32_associativity(
            first in -100i32..100i32,
            second in -100i32..100i32,
            third in -100i32..100i32
        ) {
            // Use small values to avoid overflow
            let left = Product::new(first).combine(Product::new(second)).combine(Product::new(third));
            let right = Product::new(first).combine(Product::new(second).combine(Product::new(third)));
            prop_assert_eq!(left, right);
        }

        #[test]
        fn prop_max_i32_associativity(first: i32, second: i32, third: i32) {
            let left = Max::new(first).combine(Max::new(second)).combine(Max::new(third));
            let right = Max::new(first).combine(Max::new(second).combine(Max::new(third)));
            prop_assert_eq!(left, right);
        }

        #[test]
        fn prop_min_i32_associativity(first: i32, second: i32, third: i32) {
            let left = Min::new(first).combine(Min::new(second)).combine(Min::new(third));
            let right = Min::new(first).combine(Min::new(second).combine(Min::new(third)));
            prop_assert_eq!(left, right);
        }

        #[test]
        fn prop_option_sum_associativity(
            first in prop::option::of(-10000i32..10000i32),
            second in prop::option::of(-10000i32..10000i32),
            third in prop::option::of(-10000i32..10000i32)
        ) {
            // Use small values to avoid overflow
            let first_opt = first.map(Sum::new);
            let second_opt = second.map(Sum::new);
            let third_opt = third.map(Sum::new);

            let left = first_opt.combine(second_opt).combine(third_opt);
            let right = first_opt.combine(second_opt.combine(third_opt));
            prop_assert_eq!(left, right);
        }

        #[test]
        fn prop_combine_n_equals_repeated_combine(value in "\\PC{1,5}", count in 1usize..5) {
            let combined_n = value.clone().combine_n(count);

            let mut repeated = value.clone();
            for _ in 1..count {
                repeated = repeated.combine(value.clone());
            }

            prop_assert_eq!(combined_n, repeated);
        }
    }
}
