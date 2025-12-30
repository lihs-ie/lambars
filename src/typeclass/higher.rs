//! Higher-Kinded Type emulation through Generic Associated Types.
//!
//! This module provides the foundation for emulating Higher-Kinded Types (HKT)
//! in Rust using Generic Associated Types (GAT). This is essential for defining
//! type class traits like Functor, Applicative, and Monad.
//!
//! # Background
//!
//! Rust does not natively support Higher-Kinded Types. For example, we cannot
//! write a trait that abstracts over `Option<_>` and `Vec<_>` as type constructors.
//! This module uses GAT to work around this limitation.
//!
//! # Example
//!
//! ```rust
//! use functional_rusty::typeclass::TypeConstructor;
//!
//! // Option implements TypeConstructor
//! fn transform_type<T: TypeConstructor>(value: T) -> T::WithType<String>
//! where
//!     T::WithType<String>: Default,
//! {
//!     Default::default()
//! }
//!
//! let some_int: Option<i32> = Some(42);
//! let none_string: Option<String> = transform_type(some_int);
//! assert_eq!(none_string, None);
//! ```

/// A trait representing a type constructor.
///
/// This trait emulates Higher-Kinded Types (HKT) using Generic Associated Types.
/// It allows abstracting over type constructors like `Option<_>`, `Result<_, E>`,
/// `Vec<_>`, etc.
///
/// # Type Parameters
///
/// The implementing type should be a type constructor applied to some type `A`,
/// for example `Option<A>` or `Vec<A>`.
///
/// # Associated Types
///
/// - `Inner`: The type parameter that this type constructor is currently applied to.
/// - `WithType<B>`: The same type constructor applied to a different type `B`.
///
/// # Laws
///
/// For any `F: TypeConstructor`:
///
/// 1. **Consistency**: `<F as TypeConstructor>::WithType<F::Inner>` should be
///    equivalent to `F` (up to type equality).
///
/// # Example
///
/// ```rust
/// use functional_rusty::typeclass::TypeConstructor;
///
/// // Option<i32> implements TypeConstructor
/// fn example<T: TypeConstructor<Inner = i32>>() {
///     // T::WithType<String> would be the same constructor with String
/// }
///
/// example::<Option<i32>>();
/// ```
pub trait TypeConstructor {
    /// The inner type that this type constructor is applied to.
    ///
    /// For example, for `Option<i32>`, this would be `i32`.
    type Inner;

    /// The same type constructor applied to a different type `B`.
    ///
    /// For example, for `Option<i32>`, `WithType<String>` would be `Option<String>`.
    ///
    /// The constraint `TypeConstructor<Inner = B>` ensures that the resulting
    /// type is also a valid type constructor, maintaining the ability to
    /// chain transformations.
    type WithType<B>: TypeConstructor<Inner = B>;
}

// =============================================================================
// Standard Library Type Implementations
// =============================================================================

impl<A> TypeConstructor for Option<A> {
    type Inner = A;
    type WithType<B> = Option<B>;
}

impl<T, E> TypeConstructor for Result<T, E> {
    type Inner = T;
    type WithType<B> = Result<B, E>;
}

impl<T> TypeConstructor for Vec<T> {
    type Inner = T;
    type WithType<B> = Vec<B>;
}

impl<T> TypeConstructor for Box<T> {
    type Inner = T;
    type WithType<B> = Box<B>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // Type-level tests (compile-time verification)
    // =========================================================================

    /// Verifies that Option<i32> has the correct Inner type.
    #[test]
    fn option_inner_type_is_correct() {
        fn assert_inner<T: TypeConstructor<Inner = i32>>() {}
        assert_inner::<Option<i32>>();
    }

    /// Verifies that Option<String> has the correct Inner type.
    #[test]
    fn option_inner_type_string_is_correct() {
        fn assert_inner<T: TypeConstructor<Inner = String>>() {}
        assert_inner::<Option<String>>();
    }

    /// Verifies that Option's WithType produces the correct type.
    #[test]
    fn option_with_type_produces_correct_type() {
        fn transform<T: TypeConstructor>(_value: T) -> T::WithType<String>
        where
            T::WithType<String>: Default,
        {
            Default::default()
        }

        let result: Option<String> = transform(Some(42));
        assert_eq!(result, None);
    }

    /// Verifies that Result<T, E> has the correct Inner type.
    #[test]
    fn result_inner_type_is_correct() {
        fn assert_inner<T: TypeConstructor<Inner = i32>>() {}
        assert_inner::<Result<i32, String>>();
    }

    /// Verifies that Result's WithType preserves the error type.
    #[test]
    fn result_with_type_preserves_error_type() {
        // This test verifies the type-level transformation works correctly
        // by checking that Result<T, E>::WithType<B> is Result<B, E>
        fn assert_result_with_type<T, E, B>()
        where
            Result<T, E>: TypeConstructor<Inner = T, WithType<B> = Result<B, E>>,
        {
        }

        assert_result_with_type::<i32, String, bool>();
        assert_result_with_type::<String, (), i32>();
        assert_result_with_type::<Vec<u8>, std::io::Error, String>();
    }

    /// Verifies that Vec<A> has the correct Inner type.
    #[test]
    fn vec_inner_type_is_correct() {
        fn assert_inner<T: TypeConstructor<Inner = i32>>() {}
        assert_inner::<Vec<i32>>();
    }

    /// Verifies that Vec's WithType produces the correct type.
    #[test]
    fn vec_with_type_produces_correct_type() {
        fn transform<T: TypeConstructor>(_value: T) -> T::WithType<char>
        where
            T::WithType<char>: Default,
        {
            Default::default()
        }

        let result: Vec<char> = transform(vec![1, 2, 3]);
        assert!(result.is_empty());
    }

    /// Verifies that Box<A> has the correct Inner type.
    #[test]
    fn box_inner_type_is_correct() {
        fn assert_inner<T: TypeConstructor<Inner = f64>>() {}
        assert_inner::<Box<f64>>();
    }

    /// Verifies that Box's WithType produces the correct type.
    #[test]
    fn box_with_type_produces_correct_type() {
        fn transform<T: TypeConstructor, B: Default>(_value: T) -> T::WithType<B>
        where
            T::WithType<B>: From<B>,
        {
            B::default().into()
        }

        let result: Box<String> = transform::<Box<i32>, String>(Box::new(42));
        assert_eq!(*result, String::new());
    }

    // =========================================================================
    // Property-based tests using rstest
    // =========================================================================

    /// Tests that WithType<Inner> is equivalent to the original type for Option.
    #[rstest]
    #[case(Some(42))]
    #[case(None)]
    fn option_with_type_inner_roundtrip(#[case] original: Option<i32>) {
        fn roundtrip<T: TypeConstructor>(value: T) -> T::WithType<T::Inner>
        where
            T::Inner: Clone,
            T: Into<T::WithType<T::Inner>>,
        {
            value.into()
        }

        // For Option, WithType<i32> is Option<i32>, so this should work
        let result: Option<i32> = roundtrip(original);
        assert_eq!(result, original);
    }

    /// Tests that nested type constructors work correctly.
    #[test]
    fn nested_type_constructor_works() {
        // Option<Vec<i32>> should be a TypeConstructor
        fn assert_type_constructor<T: TypeConstructor>() {}
        assert_type_constructor::<Option<Vec<i32>>>();

        // The Inner type should be Vec<i32>
        fn assert_inner<T: TypeConstructor<Inner = Vec<i32>>>() {}
        assert_inner::<Option<Vec<i32>>>();
    }

    /// Tests chaining WithType transformations.
    #[test]
    fn chained_with_type_transformations() {
        type Step1 = <Option<i32> as TypeConstructor>::WithType<String>;
        type Step2 = <Step1 as TypeConstructor>::WithType<bool>;

        fn assert_is_option_bool<T: TypeConstructor<Inner = bool>>() {}
        assert_is_option_bool::<Step2>();
    }
}
