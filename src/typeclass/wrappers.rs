//! Numeric wrapper types for different algebraic operations.
//!
//! This module provides newtype wrappers that allow the same underlying type
//! to have different `Semigroup` and `Monoid` implementations. For example,
//! integers can be combined using addition (`Sum`) or multiplication (`Product`).
//!
//! # Available Wrappers
//!
//! - [`Sum`]: Addition-based semigroup/monoid (identity: 0)
//! - [`Product`]: Multiplication-based semigroup/monoid (identity: 1)
//! - [`Max`]: Maximum-based semigroup (identity: type minimum)
//! - [`Min`]: Minimum-based semigroup (identity: type maximum)
//!
//! # The Bounded Trait
//!
//! The [`Bounded`] trait provides minimum and maximum values for types,
//! which is necessary for `Max` and `Min` to have monoid instances.

// =============================================================================
// Sum Wrapper
// =============================================================================

/// A newtype wrapper that represents the additive semigroup/monoid.
///
/// When used with `Semigroup`, `Sum(a).combine(Sum(b))` equals `Sum(a + b)`.
/// When used with `Monoid`, the identity element is `Sum(0)`.
///
/// # Examples
///
/// ```rust
/// use functional_rusty::typeclass::Sum;
///
/// let a = Sum::new(3);
/// let b = Sum::new(5);
///
/// // Direct access to inner value
/// assert_eq!(a.into_inner() + b.into_inner(), 8);
/// ```
///
/// # Usage with Semigroup and Monoid
///
/// Once `Semigroup` and `Monoid` are implemented, you can use:
///
/// ```ignore
/// use functional_rusty::typeclass::{Sum, Semigroup, Monoid};
///
/// let a = Sum(3);
/// let b = Sum(5);
/// assert_eq!(a.combine(b), Sum(8));
/// assert_eq!(Sum::<i32>::empty(), Sum(0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Sum<A>(pub A);

impl<A> Sum<A> {
    /// Creates a new `Sum` wrapping the given value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::typeclass::Sum;
    ///
    /// let sum = Sum::new(42);
    /// assert_eq!(sum.into_inner(), 42);
    /// ```
    #[inline]
    pub const fn new(value: A) -> Self {
        Self(value)
    }

    /// Consumes the `Sum` and returns the inner value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::typeclass::Sum;
    ///
    /// let sum = Sum::new(42);
    /// assert_eq!(sum.into_inner(), 42);
    /// ```
    #[inline]
    pub fn into_inner(self) -> A {
        self.0
    }

    /// Returns a reference to the inner value.
    #[inline]
    pub const fn as_inner(&self) -> &A {
        &self.0
    }

    /// Returns a mutable reference to the inner value.
    #[inline]
    pub const fn as_inner_mut(&mut self) -> &mut A {
        &mut self.0
    }
}

impl<A> From<A> for Sum<A> {
    fn from(value: A) -> Self {
        Self::new(value)
    }
}

// =============================================================================
// Product Wrapper
// =============================================================================

/// A newtype wrapper that represents the multiplicative semigroup/monoid.
///
/// When used with `Semigroup`, `Product(a).combine(Product(b))` equals `Product(a * b)`.
/// When used with `Monoid`, the identity element is `Product(1)`.
///
/// # Examples
///
/// ```rust
/// use functional_rusty::typeclass::Product;
///
/// let a = Product::new(3);
/// let b = Product::new(5);
///
/// // Direct access to inner value
/// assert_eq!(a.into_inner() * b.into_inner(), 15);
/// ```
///
/// # Usage with Semigroup and Monoid
///
/// Once `Semigroup` and `Monoid` are implemented, you can use:
///
/// ```ignore
/// use functional_rusty::typeclass::{Product, Semigroup, Monoid};
///
/// let a = Product(3);
/// let b = Product(5);
/// assert_eq!(a.combine(b), Product(15));
/// assert_eq!(Product::<i32>::empty(), Product(1));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Product<A>(pub A);

impl<A> Product<A> {
    /// Creates a new `Product` wrapping the given value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::typeclass::Product;
    ///
    /// let product = Product::new(42);
    /// assert_eq!(product.into_inner(), 42);
    /// ```
    #[inline]
    pub const fn new(value: A) -> Self {
        Self(value)
    }

    /// Consumes the `Product` and returns the inner value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::typeclass::Product;
    ///
    /// let product = Product::new(42);
    /// assert_eq!(product.into_inner(), 42);
    /// ```
    #[inline]
    pub fn into_inner(self) -> A {
        self.0
    }

    /// Returns a reference to the inner value.
    #[inline]
    pub const fn as_inner(&self) -> &A {
        &self.0
    }

    /// Returns a mutable reference to the inner value.
    #[inline]
    pub const fn as_inner_mut(&mut self) -> &mut A {
        &mut self.0
    }
}

impl<A> From<A> for Product<A> {
    fn from(value: A) -> Self {
        Self::new(value)
    }
}

// Note: Default is not derived for Product because the default value should be 1
// (the multiplicative identity), not 0 (the default for most numeric types).
// A custom Default implementation will be added with the Monoid trait.

// =============================================================================
// Max Wrapper
// =============================================================================

/// A newtype wrapper that represents the maximum semigroup.
///
/// When used with `Semigroup`, `Max(a).combine(Max(b))` equals `Max(max(a, b))`.
/// When used with `Monoid` (requires `Bounded`), the identity element is
/// `Max(A::MIN_VALUE)` (the minimum value of the type).
///
/// # Examples
///
/// ```rust
/// use functional_rusty::typeclass::Max;
///
/// let a = Max::new(3);
/// let b = Max::new(5);
///
/// // Direct comparison
/// let max_value = if a.into_inner() > b.into_inner() { a } else { b };
/// assert_eq!(max_value.into_inner(), 5);
/// ```
///
/// # Usage with Semigroup and Monoid
///
/// Once `Semigroup` and `Monoid` are implemented, you can use:
///
/// ```ignore
/// use functional_rusty::typeclass::{Max, Semigroup, Monoid};
///
/// let a = Max(3);
/// let b = Max(5);
/// assert_eq!(a.combine(b), Max(5));
/// assert_eq!(Max::<i32>::empty(), Max(i32::MIN));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Max<A>(pub A);

impl<A> Max<A> {
    /// Creates a new `Max` wrapping the given value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::typeclass::Max;
    ///
    /// let max = Max::new(42);
    /// assert_eq!(max.into_inner(), 42);
    /// ```
    #[inline]
    pub const fn new(value: A) -> Self {
        Self(value)
    }

    /// Consumes the `Max` and returns the inner value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::typeclass::Max;
    ///
    /// let max = Max::new(42);
    /// assert_eq!(max.into_inner(), 42);
    /// ```
    #[inline]
    pub fn into_inner(self) -> A {
        self.0
    }

    /// Returns a reference to the inner value.
    #[inline]
    pub const fn as_inner(&self) -> &A {
        &self.0
    }

    /// Returns a mutable reference to the inner value.
    #[inline]
    pub const fn as_inner_mut(&mut self) -> &mut A {
        &mut self.0
    }
}

impl<A> From<A> for Max<A> {
    fn from(value: A) -> Self {
        Self::new(value)
    }
}

// =============================================================================
// Min Wrapper
// =============================================================================

/// A newtype wrapper that represents the minimum semigroup.
///
/// When used with `Semigroup`, `Min(a).combine(Min(b))` equals `Min(min(a, b))`.
/// When used with `Monoid` (requires `Bounded`), the identity element is
/// `Min(A::MAX_VALUE)` (the maximum value of the type).
///
/// # Examples
///
/// ```rust
/// use functional_rusty::typeclass::Min;
///
/// let a = Min::new(3);
/// let b = Min::new(5);
///
/// // Direct comparison
/// let min_value = if a.into_inner() < b.into_inner() { a } else { b };
/// assert_eq!(min_value.into_inner(), 3);
/// ```
///
/// # Usage with Semigroup and Monoid
///
/// Once `Semigroup` and `Monoid` are implemented, you can use:
///
/// ```ignore
/// use functional_rusty::typeclass::{Min, Semigroup, Monoid};
///
/// let a = Min(3);
/// let b = Min(5);
/// assert_eq!(a.combine(b), Min(3));
/// assert_eq!(Min::<i32>::empty(), Min(i32::MAX));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Min<A>(pub A);

impl<A> Min<A> {
    /// Creates a new `Min` wrapping the given value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::typeclass::Min;
    ///
    /// let min = Min::new(42);
    /// assert_eq!(min.into_inner(), 42);
    /// ```
    #[inline]
    pub const fn new(value: A) -> Self {
        Self(value)
    }

    /// Consumes the `Min` and returns the inner value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use functional_rusty::typeclass::Min;
    ///
    /// let min = Min::new(42);
    /// assert_eq!(min.into_inner(), 42);
    /// ```
    #[inline]
    pub fn into_inner(self) -> A {
        self.0
    }

    /// Returns a reference to the inner value.
    #[inline]
    pub const fn as_inner(&self) -> &A {
        &self.0
    }

    /// Returns a mutable reference to the inner value.
    #[inline]
    pub const fn as_inner_mut(&mut self) -> &mut A {
        &mut self.0
    }
}

impl<A> From<A> for Min<A> {
    fn from(value: A) -> Self {
        Self::new(value)
    }
}

// =============================================================================
// Bounded Trait
// =============================================================================

/// A trait for types that have minimum and maximum bounds.
///
/// This trait is used to provide identity elements for `Max` and `Min`
/// when used as monoids:
///
/// - `Max<A>` uses `A::MIN_VALUE` as its identity (any value combined with MIN gives that value)
/// - `Min<A>` uses `A::MAX_VALUE` as its identity (any value combined with MAX gives that value)
///
/// # Implementing Bounded
///
/// For custom types, implement `Bounded` by providing the extreme values:
///
/// ```rust
/// use functional_rusty::typeclass::Bounded;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// struct Score(u8);
///
/// impl Bounded for Score {
///     const MIN_VALUE: Self = Score(0);
///     const MAX_VALUE: Self = Score(100);
/// }
///
/// assert_eq!(Score::MIN_VALUE.0, 0);
/// assert_eq!(Score::MAX_VALUE.0, 100);
/// ```
pub trait Bounded {
    /// The minimum value of this type.
    const MIN_VALUE: Self;

    /// The maximum value of this type.
    const MAX_VALUE: Self;
}

// Implement Bounded for signed integer types
impl Bounded for i8 {
    const MIN_VALUE: Self = Self::MIN;
    const MAX_VALUE: Self = Self::MAX;
}

impl Bounded for i16 {
    const MIN_VALUE: Self = Self::MIN;
    const MAX_VALUE: Self = Self::MAX;
}

impl Bounded for i32 {
    const MIN_VALUE: Self = Self::MIN;
    const MAX_VALUE: Self = Self::MAX;
}

impl Bounded for i64 {
    const MIN_VALUE: Self = Self::MIN;
    const MAX_VALUE: Self = Self::MAX;
}

impl Bounded for i128 {
    const MIN_VALUE: Self = Self::MIN;
    const MAX_VALUE: Self = Self::MAX;
}

impl Bounded for isize {
    const MIN_VALUE: Self = Self::MIN;
    const MAX_VALUE: Self = Self::MAX;
}

// Implement Bounded for unsigned integer types
impl Bounded for u8 {
    const MIN_VALUE: Self = Self::MIN;
    const MAX_VALUE: Self = Self::MAX;
}

impl Bounded for u16 {
    const MIN_VALUE: Self = Self::MIN;
    const MAX_VALUE: Self = Self::MAX;
}

impl Bounded for u32 {
    const MIN_VALUE: Self = Self::MIN;
    const MAX_VALUE: Self = Self::MAX;
}

impl Bounded for u64 {
    const MIN_VALUE: Self = Self::MIN;
    const MAX_VALUE: Self = Self::MAX;
}

impl Bounded for u128 {
    const MIN_VALUE: Self = Self::MIN;
    const MAX_VALUE: Self = Self::MAX;
}

impl Bounded for usize {
    const MIN_VALUE: Self = Self::MIN;
    const MAX_VALUE: Self = Self::MAX;
}

// Implement Bounded for floating point types
impl Bounded for f32 {
    const MIN_VALUE: Self = Self::NEG_INFINITY;
    const MAX_VALUE: Self = Self::INFINITY;
}

impl Bounded for f64 {
    const MIN_VALUE: Self = Self::NEG_INFINITY;
    const MAX_VALUE: Self = Self::INFINITY;
}

// Implement Bounded for char
impl Bounded for char {
    const MIN_VALUE: Self = '\0';
    const MAX_VALUE: Self = Self::MAX;
}

// Implement Bounded for bool
impl Bounded for bool {
    const MIN_VALUE: Self = false;
    const MAX_VALUE: Self = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // Sum wrapper tests
    // =========================================================================

    #[rstest]
    fn sum_new_creates_wrapper() {
        let sum = Sum::new(42);
        assert_eq!(sum.0, 42);
    }

    #[rstest]
    fn sum_into_inner_unwraps() {
        let sum = Sum::new(42);
        assert_eq!(sum.into_inner(), 42);
    }

    #[rstest]
    fn sum_as_inner_returns_reference() {
        let sum = Sum::new(42);
        assert_eq!(sum.as_inner(), &42);
    }

    #[rstest]
    fn sum_as_inner_mut_allows_modification() {
        let mut sum = Sum::new(42);
        *sum.as_inner_mut() = 100;
        assert_eq!(sum.into_inner(), 100);
    }

    #[rstest]
    fn sum_clone_works() {
        let original = Sum::new(42);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[rstest]
    fn sum_copy_works() {
        let original = Sum::new(42);
        let copied = original;
        assert_eq!(original, copied);
    }

    #[rstest]
    fn sum_default_is_zero() {
        let default: Sum<i32> = Sum::default();
        assert_eq!(default.into_inner(), 0);
    }

    #[rstest]
    fn sum_from_value() {
        let sum: Sum<i32> = 42.into();
        assert_eq!(sum.into_inner(), 42);
    }

    #[rstest]
    fn sum_ord_works() {
        let smaller = Sum::new(1);
        let larger = Sum::new(2);
        assert!(smaller < larger);
    }

    // =========================================================================
    // Product wrapper tests
    // =========================================================================

    #[rstest]
    fn product_new_creates_wrapper() {
        let product = Product::new(42);
        assert_eq!(product.0, 42);
    }

    #[rstest]
    fn product_into_inner_unwraps() {
        let product = Product::new(42);
        assert_eq!(product.into_inner(), 42);
    }

    #[rstest]
    fn product_as_inner_returns_reference() {
        let product = Product::new(42);
        assert_eq!(product.as_inner(), &42);
    }

    #[rstest]
    fn product_as_inner_mut_allows_modification() {
        let mut product = Product::new(42);
        *product.as_inner_mut() = 100;
        assert_eq!(product.into_inner(), 100);
    }

    #[rstest]
    fn product_clone_works() {
        let original = Product::new(42);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[rstest]
    fn product_copy_works() {
        let original = Product::new(42);
        let copied = original;
        assert_eq!(original, copied);
    }

    #[rstest]
    fn product_from_value() {
        let product: Product<i32> = 42.into();
        assert_eq!(product.into_inner(), 42);
    }

    #[rstest]
    fn product_ord_works() {
        let smaller = Product::new(1);
        let larger = Product::new(2);
        assert!(smaller < larger);
    }

    // =========================================================================
    // Max wrapper tests
    // =========================================================================

    #[rstest]
    fn max_new_creates_wrapper() {
        let max = Max::new(42);
        assert_eq!(max.0, 42);
    }

    #[rstest]
    fn max_into_inner_unwraps() {
        let max = Max::new(42);
        assert_eq!(max.into_inner(), 42);
    }

    #[rstest]
    fn max_as_inner_returns_reference() {
        let max = Max::new(42);
        assert_eq!(max.as_inner(), &42);
    }

    #[rstest]
    fn max_as_inner_mut_allows_modification() {
        let mut max = Max::new(42);
        *max.as_inner_mut() = 100;
        assert_eq!(max.into_inner(), 100);
    }

    #[rstest]
    fn max_clone_works() {
        let original = Max::new(42);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[rstest]
    fn max_copy_works() {
        let original = Max::new(42);
        let copied = original;
        assert_eq!(original, copied);
    }

    #[rstest]
    fn max_from_value() {
        let max: Max<i32> = 42.into();
        assert_eq!(max.into_inner(), 42);
    }

    #[rstest]
    fn max_ord_works() {
        let smaller = Max::new(1);
        let larger = Max::new(2);
        assert!(smaller < larger);
    }

    // =========================================================================
    // Min wrapper tests
    // =========================================================================

    #[rstest]
    fn min_new_creates_wrapper() {
        let min = Min::new(42);
        assert_eq!(min.0, 42);
    }

    #[rstest]
    fn min_into_inner_unwraps() {
        let min = Min::new(42);
        assert_eq!(min.into_inner(), 42);
    }

    #[rstest]
    fn min_as_inner_returns_reference() {
        let min = Min::new(42);
        assert_eq!(min.as_inner(), &42);
    }

    #[rstest]
    fn min_as_inner_mut_allows_modification() {
        let mut min = Min::new(42);
        *min.as_inner_mut() = 100;
        assert_eq!(min.into_inner(), 100);
    }

    #[rstest]
    fn min_clone_works() {
        let original = Min::new(42);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[rstest]
    fn min_copy_works() {
        let original = Min::new(42);
        let copied = original;
        assert_eq!(original, copied);
    }

    #[rstest]
    fn min_from_value() {
        let min: Min<i32> = 42.into();
        assert_eq!(min.into_inner(), 42);
    }

    #[rstest]
    fn min_ord_works() {
        let smaller = Min::new(1);
        let larger = Min::new(2);
        assert!(smaller < larger);
    }

    // =========================================================================
    // Bounded trait tests
    // =========================================================================

    #[rstest]
    fn bounded_i8_values() {
        assert_eq!(i8::MIN_VALUE, i8::MIN);
        assert_eq!(i8::MAX_VALUE, i8::MAX);
    }

    #[rstest]
    fn bounded_i16_values() {
        assert_eq!(i16::MIN_VALUE, i16::MIN);
        assert_eq!(i16::MAX_VALUE, i16::MAX);
    }

    #[rstest]
    fn bounded_i32_values() {
        assert_eq!(i32::MIN_VALUE, i32::MIN);
        assert_eq!(i32::MAX_VALUE, i32::MAX);
    }

    #[rstest]
    fn bounded_i64_values() {
        assert_eq!(i64::MIN_VALUE, i64::MIN);
        assert_eq!(i64::MAX_VALUE, i64::MAX);
    }

    #[rstest]
    fn bounded_i128_values() {
        assert_eq!(i128::MIN_VALUE, i128::MIN);
        assert_eq!(i128::MAX_VALUE, i128::MAX);
    }

    #[rstest]
    fn bounded_isize_values() {
        assert_eq!(isize::MIN_VALUE, isize::MIN);
        assert_eq!(isize::MAX_VALUE, isize::MAX);
    }

    #[rstest]
    fn bounded_u8_values() {
        assert_eq!(u8::MIN_VALUE, u8::MIN);
        assert_eq!(u8::MAX_VALUE, u8::MAX);
    }

    #[rstest]
    fn bounded_u16_values() {
        assert_eq!(u16::MIN_VALUE, u16::MIN);
        assert_eq!(u16::MAX_VALUE, u16::MAX);
    }

    #[rstest]
    fn bounded_u32_values() {
        assert_eq!(u32::MIN_VALUE, u32::MIN);
        assert_eq!(u32::MAX_VALUE, u32::MAX);
    }

    #[rstest]
    fn bounded_u64_values() {
        assert_eq!(u64::MIN_VALUE, u64::MIN);
        assert_eq!(u64::MAX_VALUE, u64::MAX);
    }

    #[rstest]
    fn bounded_u128_values() {
        assert_eq!(u128::MIN_VALUE, u128::MIN);
        assert_eq!(u128::MAX_VALUE, u128::MAX);
    }

    #[rstest]
    fn bounded_usize_values() {
        assert_eq!(usize::MIN_VALUE, usize::MIN);
        assert_eq!(usize::MAX_VALUE, usize::MAX);
    }

    #[rstest]
    fn bounded_f32_values() {
        assert!(f32::MIN_VALUE.is_infinite() && f32::MIN_VALUE.is_sign_negative());
        assert!(f32::MAX_VALUE.is_infinite() && f32::MAX_VALUE.is_sign_positive());
    }

    #[rstest]
    fn bounded_f64_values() {
        assert!(f64::MIN_VALUE.is_infinite() && f64::MIN_VALUE.is_sign_negative());
        assert!(f64::MAX_VALUE.is_infinite() && f64::MAX_VALUE.is_sign_positive());
    }

    #[rstest]
    fn bounded_char_values() {
        assert_eq!(char::MIN_VALUE, '\0');
        assert_eq!(char::MAX_VALUE, char::MAX);
    }

    #[rstest]
    fn bounded_bool_values() {
        assert_eq!(bool::MIN_VALUE, false);
        assert_eq!(bool::MAX_VALUE, true);
    }

    // =========================================================================
    // Parameterized tests
    // =========================================================================

    #[rstest]
    #[case(0)]
    #[case(1)]
    #[case(-1)]
    #[case(i32::MIN)]
    #[case(i32::MAX)]
    fn sum_preserves_integer_values(#[case] value: i32) {
        let sum = Sum::new(value);
        assert_eq!(sum.into_inner(), value);
    }

    #[rstest]
    #[case(0)]
    #[case(1)]
    #[case(-1)]
    #[case(i32::MIN)]
    #[case(i32::MAX)]
    fn product_preserves_integer_values(#[case] value: i32) {
        let product = Product::new(value);
        assert_eq!(product.into_inner(), value);
    }

    #[rstest]
    #[case(0)]
    #[case(1)]
    #[case(-1)]
    #[case(i32::MIN)]
    #[case(i32::MAX)]
    fn max_preserves_integer_values(#[case] value: i32) {
        let max = Max::new(value);
        assert_eq!(max.into_inner(), value);
    }

    #[rstest]
    #[case(0)]
    #[case(1)]
    #[case(-1)]
    #[case(i32::MIN)]
    #[case(i32::MAX)]
    fn min_preserves_integer_values(#[case] value: i32) {
        let min = Min::new(value);
        assert_eq!(min.into_inner(), value);
    }

    // =========================================================================
    // Debug output tests
    // =========================================================================

    #[rstest]
    fn sum_debug_output() {
        let sum = Sum::new(42);
        let debug = format!("{:?}", sum);
        assert!(debug.contains("Sum"));
        assert!(debug.contains("42"));
    }

    #[rstest]
    fn product_debug_output() {
        let product = Product::new(42);
        let debug = format!("{:?}", product);
        assert!(debug.contains("Product"));
        assert!(debug.contains("42"));
    }

    #[rstest]
    fn max_debug_output() {
        let max = Max::new(42);
        let debug = format!("{:?}", max);
        assert!(debug.contains("Max"));
        assert!(debug.contains("42"));
    }

    #[rstest]
    fn min_debug_output() {
        let min = Min::new(42);
        let debug = format!("{:?}", min);
        assert!(debug.contains("Min"));
        assert!(debug.contains("42"));
    }
}
