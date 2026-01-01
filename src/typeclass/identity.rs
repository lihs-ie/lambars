//! Identity wrapper type - the identity functor.
//!
//! This module provides the `Identity` type, which is the simplest possible
//! wrapper around a value. It serves as:
//!
//! - The base case for monad transformers
//! - A simple model for testing type class laws
//! - A way to express "no additional effect" in effect systems

use super::TypeConstructor;

/// The identity functor - wraps a value without adding any behavior.
///
/// `Identity` is the simplest possible type constructor. It wraps a single
/// value and provides no additional functionality. This makes it useful as:
///
/// - A base monad for monad transformer stacks
/// - A testing model for type class laws (since it's the simplest implementation)
/// - A way to represent "pure" computation in effect systems
///
/// # Examples
///
/// ```rust
/// use lambars::typeclass::Identity;
///
/// let wrapped = Identity::new(42);
/// assert_eq!(wrapped.into_inner(), 42);
///
/// // Using the tuple-struct syntax
/// let wrapped = Identity(42);
/// assert_eq!(wrapped.0, 42);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Identity<A>(pub A);

impl<A> Identity<A> {
    /// Creates a new `Identity` wrapping the given value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Identity;
    ///
    /// let x = Identity::new(42);
    /// assert_eq!(x.into_inner(), 42);
    /// ```
    #[inline]
    pub const fn new(value: A) -> Self {
        Self(value)
    }

    /// Consumes the `Identity` and returns the inner value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Identity;
    ///
    /// let x = Identity::new(String::from("hello"));
    /// let inner: String = x.into_inner();
    /// assert_eq!(inner, "hello");
    /// ```
    #[inline]
    pub fn into_inner(self) -> A {
        self.0
    }

    /// Returns a reference to the inner value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Identity;
    ///
    /// let x = Identity::new(String::from("hello"));
    /// assert_eq!(x.as_inner(), "hello");
    /// ```
    #[inline]
    pub const fn as_inner(&self) -> &A {
        &self.0
    }

    /// Returns a mutable reference to the inner value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lambars::typeclass::Identity;
    ///
    /// let mut x = Identity::new(42);
    /// *x.as_inner_mut() = 100;
    /// assert_eq!(x.into_inner(), 100);
    /// ```
    #[inline]
    pub const fn as_inner_mut(&mut self) -> &mut A {
        &mut self.0
    }
}

impl<A> TypeConstructor for Identity<A> {
    type Inner = A;
    type WithType<B> = Identity<B>;
}

impl<A> From<A> for Identity<A> {
    fn from(value: A) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // Basic functionality tests
    // =========================================================================

    #[rstest]
    fn identity_new_creates_wrapper() {
        let wrapped = Identity::new(42);
        assert_eq!(wrapped.0, 42);
    }

    #[rstest]
    fn identity_into_inner_unwraps() {
        let wrapped = Identity::new(String::from("hello"));
        let inner = wrapped.into_inner();
        assert_eq!(inner, "hello");
    }

    #[rstest]
    fn identity_as_inner_returns_reference() {
        let wrapped = Identity::new(vec![1, 2, 3]);
        let inner_reference = wrapped.as_inner();
        assert_eq!(inner_reference, &vec![1, 2, 3]);
    }

    #[rstest]
    fn identity_as_inner_mut_allows_modification() {
        let mut wrapped = Identity::new(42);
        *wrapped.as_inner_mut() = 100;
        assert_eq!(wrapped.into_inner(), 100);
    }

    #[rstest]
    fn identity_tuple_struct_access() {
        let wrapped = Identity(42);
        assert_eq!(wrapped.0, 42);
    }

    // =========================================================================
    // Derive trait tests
    // =========================================================================

    #[rstest]
    fn identity_clone_works() {
        let original = Identity::new(42);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[rstest]
    fn identity_copy_works() {
        let original = Identity::new(42);
        let copied = original;
        assert_eq!(original, copied);
    }

    #[rstest]
    fn identity_partial_eq_works() {
        let first = Identity::new(42);
        let second = Identity::new(42);
        let third = Identity::new(100);

        assert_eq!(first, second);
        assert_ne!(first, third);
    }

    #[rstest]
    fn identity_ord_works() {
        let smaller = Identity::new(1);
        let larger = Identity::new(2);

        assert!(smaller < larger);
        assert!(larger > smaller);
    }

    #[rstest]
    fn identity_default_works() {
        let default_int: Identity<i32> = Identity::default();
        assert_eq!(default_int.into_inner(), 0);

        let default_string: Identity<String> = Identity::default();
        assert_eq!(default_string.into_inner(), String::new());
    }

    #[rstest]
    fn identity_debug_works() {
        let wrapped = Identity::new(42);
        let debug_output = format!("{:?}", wrapped);
        assert!(debug_output.contains("Identity"));
        assert!(debug_output.contains("42"));
    }

    // =========================================================================
    // TypeConstructor implementation tests
    // =========================================================================

    #[test]
    fn identity_type_constructor_inner_type() {
        fn assert_inner<T: TypeConstructor<Inner = i32>>() {}
        assert_inner::<Identity<i32>>();
    }

    #[test]
    fn identity_type_constructor_with_type() {
        fn transform<T: TypeConstructor>(_value: T) -> T::WithType<String>
        where
            T::WithType<String>: Default,
        {
            Default::default()
        }

        let result: Identity<String> = transform(Identity::new(42));
        assert_eq!(result, Identity(String::new()));
    }

    // =========================================================================
    // From implementation tests
    // =========================================================================

    #[rstest]
    fn identity_from_value() {
        let wrapped: Identity<i32> = 42.into();
        assert_eq!(wrapped.into_inner(), 42);
    }

    #[rstest]
    fn identity_from_string() {
        let wrapped: Identity<String> = String::from("hello").into();
        assert_eq!(wrapped.into_inner(), "hello");
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
    fn identity_preserves_integer_values(#[case] value: i32) {
        let wrapped = Identity::new(value);
        assert_eq!(wrapped.into_inner(), value);
    }

    #[rstest]
    #[case("")]
    #[case("hello")]
    #[case("hello world")]
    fn identity_preserves_string_values(#[case] value: &str) {
        let wrapped = Identity::new(value.to_string());
        assert_eq!(wrapped.into_inner(), value);
    }
}
