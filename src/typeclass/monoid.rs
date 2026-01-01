//! Monoid type class - semigroups with an identity element.
//!
//! A monoid is a semigroup with an identity element. In other words, a type `T`
//! is a monoid if it has:
//!
//! 1. An associative binary operation `combine: (T, T) -> T` (from Semigroup)
//! 2. An identity element `empty: T` such that for all `a`:
//!    - `empty.combine(a) == a` (left identity)
//!    - `a.combine(empty) == a` (right identity)
//!
//! # Laws
//!
//! For all `a`, `b`, `c` of type `T`:
//!
//! ## Left Identity
//!
//! ```text
//! T::empty().combine(a) == a
//! ```
//!
//! ## Right Identity
//!
//! ```text
//! a.combine(T::empty()) == a
//! ```
//!
//! ## Associativity (inherited from Semigroup)
//!
//! ```text
//! (a.combine(b)).combine(c) == a.combine(b.combine(c))
//! ```
//!
//! # Examples
//!
//! ```rust
//! use lambars::typeclass::{Semigroup, Monoid};
//!
//! // String monoid with empty string as identity
//! assert_eq!(String::empty(), "");
//! assert_eq!(String::empty().combine(String::from("hello")), "hello");
//! assert_eq!(String::from("hello").combine(String::empty()), "hello");
//!
//! // Vec monoid with empty vec as identity
//! let vec: Vec<i32> = Vec::empty();
//! assert!(vec.is_empty());
//! ```

use std::ops::Add;

use super::Identity;
use super::semigroup::Semigroup;
use super::wrappers::{Bounded, Max, Min, Product, Sum};

/// A type class for semigroups with an identity element.
///
/// # Laws
///
/// All implementations must satisfy (in addition to Semigroup laws):
///
/// ## Left Identity
///
/// For all `a`:
/// ```text
/// Self::empty().combine(a) == a
/// ```
///
/// ## Right Identity
///
/// For all `a`:
/// ```text
/// a.combine(Self::empty()) == a
/// ```
///
/// # Examples
///
/// ```rust
/// use lambars::typeclass::{Semigroup, Monoid};
///
/// // Combining with empty yields the original value
/// let s = String::from("hello");
/// assert_eq!(String::empty().combine(s.clone()), s);
/// assert_eq!(s.clone().combine(String::empty()), s);
/// ```
pub trait Monoid: Semigroup {
    /// Returns the identity element for this monoid.
    ///
    /// The identity element satisfies:
    /// - `Self::empty().combine(a) == a` for all `a`
    /// - `a.combine(Self::empty()) == a` for all `a`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Monoid;
    ///
    /// assert_eq!(String::empty(), "");
    /// assert!(Vec::<i32>::empty().is_empty());
    /// ```
    fn empty() -> Self;

    /// Combines all elements in an iterator, starting from the identity element.
    ///
    /// Unlike [`Semigroup::reduce_all`], this method always returns a value
    /// (the identity element for empty iterators).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::{Semigroup, Monoid};
    ///
    /// let strings = vec![
    ///     String::from("a"),
    ///     String::from("b"),
    ///     String::from("c"),
    /// ];
    /// assert_eq!(String::combine_all(strings), "abc");
    ///
    /// // Empty iterator returns the identity element
    /// let empty: Vec<String> = vec![];
    /// assert_eq!(String::combine_all(empty), String::empty());
    /// ```
    fn combine_all<I>(iterator: I) -> Self
    where
        I: IntoIterator<Item = Self>,
        Self: Sized,
    {
        iterator
            .into_iter()
            .fold(Self::empty(), |accumulator, element| {
                accumulator.combine(element)
            })
    }

    /// Returns whether this value is the identity element.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Monoid;
    ///
    /// assert!(String::empty().is_empty_value());
    /// assert!(!String::from("hello").is_empty_value());
    /// ```
    fn is_empty_value(&self) -> bool
    where
        Self: PartialEq + Sized,
    {
        *self == Self::empty()
    }
}

// =============================================================================
// String Implementation
// =============================================================================

impl Monoid for String {
    fn empty() -> Self {
        Self::new()
    }
}

// =============================================================================
// Vec Implementation
// =============================================================================

impl<T: Clone> Monoid for Vec<T> {
    fn empty() -> Self {
        Self::new()
    }
}

// =============================================================================
// Option Implementation
// =============================================================================

/// Option forms a monoid when its inner type is a semigroup.
/// The identity element is `None`.
impl<T: Semigroup> Monoid for Option<T> {
    fn empty() -> Self {
        None
    }
}

// =============================================================================
// Unit Type Implementation
// =============================================================================

/// The unit type forms a trivial monoid with `()` as the identity.
impl Monoid for () {
    fn empty() -> Self {}
}

// =============================================================================
// Identity Implementation
// =============================================================================

/// Identity forms a monoid when its inner type is a monoid.
impl<T: Monoid> Monoid for Identity<T> {
    fn empty() -> Self {
        Self(T::empty())
    }
}

// =============================================================================
// Numeric Wrapper Implementations
// =============================================================================

/// Sum forms a monoid under addition with 0 as the identity.
impl<A: Add<Output = A> + Default> Monoid for Sum<A> {
    fn empty() -> Self {
        Self(A::default())
    }
}

/// Product forms a monoid under multiplication with 1 as the identity.
///
/// We use a custom `One` trait requirement since `Default` returns 0 for numbers.
impl Monoid for Product<i8> {
    fn empty() -> Self {
        Self(1)
    }
}

impl Monoid for Product<i16> {
    fn empty() -> Self {
        Self(1)
    }
}

impl Monoid for Product<i32> {
    fn empty() -> Self {
        Self(1)
    }
}

impl Monoid for Product<i64> {
    fn empty() -> Self {
        Self(1)
    }
}

impl Monoid for Product<i128> {
    fn empty() -> Self {
        Self(1)
    }
}

impl Monoid for Product<isize> {
    fn empty() -> Self {
        Self(1)
    }
}

impl Monoid for Product<u8> {
    fn empty() -> Self {
        Self(1)
    }
}

impl Monoid for Product<u16> {
    fn empty() -> Self {
        Self(1)
    }
}

impl Monoid for Product<u32> {
    fn empty() -> Self {
        Self(1)
    }
}

impl Monoid for Product<u64> {
    fn empty() -> Self {
        Self(1)
    }
}

impl Monoid for Product<u128> {
    fn empty() -> Self {
        Self(1)
    }
}

impl Monoid for Product<usize> {
    fn empty() -> Self {
        Self(1)
    }
}

impl Monoid for Product<f32> {
    fn empty() -> Self {
        Self(1.0)
    }
}

impl Monoid for Product<f64> {
    fn empty() -> Self {
        Self(1.0)
    }
}

/// Max forms a monoid with the minimum bound as the identity.
impl<A: Ord + Bounded + Clone> Monoid for Max<A> {
    fn empty() -> Self {
        Self(A::MIN_VALUE)
    }
}

/// Min forms a monoid with the maximum bound as the identity.
impl<A: Ord + Bounded + Clone> Monoid for Min<A> {
    fn empty() -> Self {
        Self(A::MAX_VALUE)
    }
}

// =============================================================================
// Tuple Implementations
// =============================================================================

/// Tuples form a monoid when all their elements are monoids.
impl<A: Monoid, B: Monoid> Monoid for (A, B) {
    fn empty() -> Self {
        (A::empty(), B::empty())
    }
}

impl<A: Monoid, B: Monoid, C: Monoid> Monoid for (A, B, C) {
    fn empty() -> Self {
        (A::empty(), B::empty(), C::empty())
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
    // String Monoid Tests
    // =========================================================================

    #[rstest]
    fn string_empty() {
        assert_eq!(String::empty(), "");
    }

    #[rstest]
    fn string_left_identity() {
        let value = String::from("hello");
        assert_eq!(String::empty().combine(value.clone()), value);
    }

    #[rstest]
    fn string_right_identity() {
        let value = String::from("hello");
        assert_eq!(value.clone().combine(String::empty()), value);
    }

    #[rstest]
    fn string_is_empty_value() {
        assert!(String::empty().is_empty_value());
        assert!(!String::from("hello").is_empty_value());
    }

    // =========================================================================
    // Vec Monoid Tests
    // =========================================================================

    #[rstest]
    fn vec_empty() {
        let empty: Vec<i32> = Vec::empty();
        assert!(empty.is_empty());
    }

    #[rstest]
    fn vec_left_identity() {
        let value = vec![1, 2, 3];
        assert_eq!(Vec::<i32>::empty().combine(value.clone()), value);
    }

    #[rstest]
    fn vec_right_identity() {
        let value = vec![1, 2, 3];
        assert_eq!(value.clone().combine(Vec::empty()), value);
    }

    #[rstest]
    fn vec_is_empty_value() {
        assert!(Vec::<i32>::empty().is_empty_value());
        assert!(!vec![1, 2, 3].is_empty_value());
    }

    // =========================================================================
    // Option Monoid Tests
    // =========================================================================

    #[rstest]
    fn option_empty() {
        let empty: Option<String> = Option::empty();
        assert_eq!(empty, None);
    }

    #[rstest]
    fn option_left_identity() {
        let value: Option<String> = Some(String::from("hello"));
        assert_eq!(Option::<String>::empty().combine(value.clone()), value);
    }

    #[rstest]
    fn option_right_identity() {
        let value: Option<String> = Some(String::from("hello"));
        assert_eq!(value.clone().combine(Option::empty()), value);
    }

    #[rstest]
    fn option_is_empty_value() {
        assert!(Option::<String>::empty().is_empty_value());
        assert!(!Some(String::from("hello")).is_empty_value());
    }

    // =========================================================================
    // Unit Type Monoid Tests
    // =========================================================================

    #[rstest]
    fn unit_empty() {
        assert_eq!(<()>::empty(), ());
    }

    #[rstest]
    fn unit_left_identity() {
        let empty: () = <()>::empty();
        assert_eq!(empty.combine(()), ());
    }

    #[rstest]
    fn unit_right_identity() {
        let empty: () = <()>::empty();
        assert_eq!(().combine(empty), ());
    }

    // =========================================================================
    // Identity Monoid Tests
    // =========================================================================

    #[rstest]
    fn identity_empty() {
        let empty: Identity<String> = Identity::empty();
        assert_eq!(empty, Identity::new(String::new()));
    }

    #[rstest]
    fn identity_left_identity() {
        let value = Identity::new(String::from("hello"));
        assert_eq!(Identity::<String>::empty().combine(value.clone()), value);
    }

    #[rstest]
    fn identity_right_identity() {
        let value = Identity::new(String::from("hello"));
        assert_eq!(value.clone().combine(Identity::empty()), value);
    }

    // =========================================================================
    // Sum Monoid Tests
    // =========================================================================

    #[rstest]
    fn sum_empty() {
        assert_eq!(Sum::<i32>::empty(), Sum(0));
    }

    #[rstest]
    fn sum_empty_f64() {
        assert_eq!(Sum::<f64>::empty(), Sum(0.0));
    }

    #[rstest]
    fn sum_left_identity() {
        let value = Sum::new(42);
        assert_eq!(Sum::<i32>::empty().combine(value), value);
    }

    #[rstest]
    fn sum_right_identity() {
        let value = Sum::new(42);
        assert_eq!(value.combine(Sum::empty()), value);
    }

    // =========================================================================
    // Product Monoid Tests
    // =========================================================================

    #[rstest]
    fn product_empty_i32() {
        assert_eq!(Product::<i32>::empty(), Product(1));
    }

    #[rstest]
    fn product_empty_f64() {
        assert_eq!(Product::<f64>::empty(), Product(1.0));
    }

    #[rstest]
    fn product_left_identity() {
        let value = Product::new(42);
        assert_eq!(Product::<i32>::empty().combine(value), value);
    }

    #[rstest]
    fn product_right_identity() {
        let value = Product::new(42);
        assert_eq!(value.combine(Product::empty()), value);
    }

    // Additional Product empty tests for coverage
    #[rstest]
    fn product_empty_i8() {
        assert_eq!(Product::<i8>::empty(), Product(1i8));
    }

    #[rstest]
    fn product_empty_i16() {
        assert_eq!(Product::<i16>::empty(), Product(1i16));
    }

    #[rstest]
    fn product_empty_i64() {
        assert_eq!(Product::<i64>::empty(), Product(1i64));
    }

    #[rstest]
    fn product_empty_i128() {
        assert_eq!(Product::<i128>::empty(), Product(1i128));
    }

    #[rstest]
    fn product_empty_isize() {
        assert_eq!(Product::<isize>::empty(), Product(1isize));
    }

    #[rstest]
    fn product_empty_u8() {
        assert_eq!(Product::<u8>::empty(), Product(1u8));
    }

    #[rstest]
    fn product_empty_u16() {
        assert_eq!(Product::<u16>::empty(), Product(1u16));
    }

    #[rstest]
    fn product_empty_u32() {
        assert_eq!(Product::<u32>::empty(), Product(1u32));
    }

    #[rstest]
    fn product_empty_u64() {
        assert_eq!(Product::<u64>::empty(), Product(1u64));
    }

    #[rstest]
    fn product_empty_u128() {
        assert_eq!(Product::<u128>::empty(), Product(1u128));
    }

    #[rstest]
    fn product_empty_usize() {
        assert_eq!(Product::<usize>::empty(), Product(1usize));
    }

    #[rstest]
    fn product_empty_f32() {
        assert_eq!(Product::<f32>::empty(), Product(1.0f32));
    }

    // =========================================================================
    // Max Monoid Tests
    // =========================================================================

    #[rstest]
    fn max_empty_i32() {
        assert_eq!(Max::<i32>::empty(), Max(i32::MIN));
    }

    #[rstest]
    fn max_empty_u8() {
        assert_eq!(Max::<u8>::empty(), Max(u8::MIN));
    }

    #[rstest]
    fn max_left_identity() {
        let value = Max::new(42i32);
        assert_eq!(Max::<i32>::empty().combine(value), value);
    }

    #[rstest]
    fn max_right_identity() {
        let value = Max::new(42i32);
        assert_eq!(value.combine(Max::empty()), value);
    }

    // =========================================================================
    // Min Monoid Tests
    // =========================================================================

    #[rstest]
    fn min_empty_i32() {
        assert_eq!(Min::<i32>::empty(), Min(i32::MAX));
    }

    #[rstest]
    fn min_empty_u8() {
        assert_eq!(Min::<u8>::empty(), Min(u8::MAX));
    }

    #[rstest]
    fn min_left_identity() {
        let value = Min::new(42i32);
        assert_eq!(Min::<i32>::empty().combine(value), value);
    }

    #[rstest]
    fn min_right_identity() {
        let value = Min::new(42i32);
        assert_eq!(value.combine(Min::empty()), value);
    }

    // =========================================================================
    // Tuple Monoid Tests
    // =========================================================================

    #[rstest]
    fn tuple2_empty() {
        let empty: (Sum<i32>, Product<i32>) = <(Sum<i32>, Product<i32>)>::empty();
        assert_eq!(empty, (Sum(0), Product(1)));
    }

    #[rstest]
    fn tuple2_left_identity() {
        let value = (Sum::new(3), Product::new(4));
        let empty: (Sum<i32>, Product<i32>) = <(Sum<i32>, Product<i32>)>::empty();
        assert_eq!(empty.combine(value), value);
    }

    #[rstest]
    fn tuple2_right_identity() {
        let value = (Sum::new(3), Product::new(4));
        assert_eq!(value.combine(<(Sum<i32>, Product<i32>)>::empty()), value);
    }

    #[rstest]
    fn tuple3_empty() {
        let empty: (Sum<i32>, Product<i32>, String) = <(Sum<i32>, Product<i32>, String)>::empty();
        assert_eq!(empty, (Sum(0), Product(1), String::new()));
    }

    // =========================================================================
    // combine_all Tests
    // =========================================================================

    #[rstest]
    fn combine_all_empty_returns_identity() {
        let empty: Vec<String> = vec![];
        assert_eq!(String::combine_all(empty), String::empty());
    }

    #[rstest]
    fn combine_all_single_element() {
        let single = vec![String::from("hello")];
        assert_eq!(String::combine_all(single), String::from("hello"));
    }

    #[rstest]
    fn combine_all_multiple_elements() {
        let multiple = vec![String::from("a"), String::from("b"), String::from("c")];
        assert_eq!(String::combine_all(multiple), String::from("abc"));
    }

    #[rstest]
    fn combine_all_sum() {
        let values = vec![Sum::new(1), Sum::new(2), Sum::new(3)];
        assert_eq!(Sum::combine_all(values), Sum::new(6));
    }

    #[rstest]
    fn combine_all_sum_empty() {
        let empty: Vec<Sum<i32>> = vec![];
        assert_eq!(Sum::combine_all(empty), Sum::empty());
    }

    #[rstest]
    fn combine_all_product() {
        let values = vec![Product::new(2), Product::new(3), Product::new(4)];
        assert_eq!(Product::combine_all(values), Product::new(24));
    }

    #[rstest]
    fn combine_all_product_empty() {
        let empty: Vec<Product<i32>> = vec![];
        assert_eq!(Product::combine_all(empty), Product::empty());
    }

    #[rstest]
    fn combine_all_max() {
        let values = vec![Max::new(1i32), Max::new(5i32), Max::new(3i32)];
        assert_eq!(Max::combine_all(values), Max::new(5i32));
    }

    #[rstest]
    fn combine_all_min() {
        let values = vec![Min::new(5i32), Min::new(1i32), Min::new(3i32)];
        assert_eq!(Min::combine_all(values), Min::new(1i32));
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
        fn prop_string_left_identity(value in "\\PC*") {
            prop_assert_eq!(String::empty().combine(value.clone()), value);
        }

        #[test]
        fn prop_string_right_identity(value in "\\PC*") {
            prop_assert_eq!(value.clone().combine(String::empty()), value);
        }

        #[test]
        fn prop_vec_i32_left_identity(value in prop::collection::vec(any::<i32>(), 0..10)) {
            prop_assert_eq!(Vec::<i32>::empty().combine(value.clone()), value);
        }

        #[test]
        fn prop_vec_i32_right_identity(value in prop::collection::vec(any::<i32>(), 0..10)) {
            prop_assert_eq!(value.clone().combine(Vec::empty()), value);
        }

        #[test]
        fn prop_sum_i32_left_identity(value: i32) {
            let sum_value = Sum::new(value);
            prop_assert_eq!(Sum::<i32>::empty().combine(sum_value), sum_value);
        }

        #[test]
        fn prop_sum_i32_right_identity(value: i32) {
            let sum_value = Sum::new(value);
            prop_assert_eq!(sum_value.combine(Sum::empty()), sum_value);
        }

        #[test]
        fn prop_product_i32_left_identity(value: i32) {
            let product_value = Product::new(value);
            prop_assert_eq!(Product::<i32>::empty().combine(product_value), product_value);
        }

        #[test]
        fn prop_product_i32_right_identity(value: i32) {
            let product_value = Product::new(value);
            prop_assert_eq!(product_value.combine(Product::empty()), product_value);
        }

        #[test]
        fn prop_max_i32_left_identity(value: i32) {
            let max_value = Max::new(value);
            prop_assert_eq!(Max::<i32>::empty().combine(max_value), max_value);
        }

        #[test]
        fn prop_max_i32_right_identity(value: i32) {
            let max_value = Max::new(value);
            prop_assert_eq!(max_value.combine(Max::empty()), max_value);
        }

        #[test]
        fn prop_min_i32_left_identity(value: i32) {
            let min_value = Min::new(value);
            prop_assert_eq!(Min::<i32>::empty().combine(min_value), min_value);
        }

        #[test]
        fn prop_min_i32_right_identity(value: i32) {
            let min_value = Min::new(value);
            prop_assert_eq!(min_value.combine(Min::empty()), min_value);
        }

        #[test]
        fn prop_option_left_identity(value in prop::option::of(any::<i32>())) {
            let opt_value = value.map(Sum::new);
            prop_assert_eq!(Option::<Sum<i32>>::empty().combine(opt_value), opt_value);
        }

        #[test]
        fn prop_option_right_identity(value in prop::option::of(any::<i32>())) {
            let opt_value = value.map(Sum::new);
            prop_assert_eq!(opt_value.combine(Option::empty()), opt_value);
        }

        #[test]
        fn prop_combine_all_equivalent_to_fold(
            values in prop::collection::vec(-1000i32..1000i32, 0..20)
        ) {
            // Use small values to avoid overflow
            let sum_values: Vec<Sum<i32>> = values.iter().copied().map(Sum::new).collect();

            let combined = Sum::combine_all(sum_values.clone());
            let folded = sum_values.into_iter().fold(Sum::empty(), |acc, x| acc.combine(x));

            prop_assert_eq!(combined, folded);
        }
    }
}
