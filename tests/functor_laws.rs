//! Property-based tests for Functor laws.
//!
//! This module verifies that all Functor implementations satisfy the required laws:
//!
//! - **Identity Law**: `fa.fmap(|x| x) == fa`
//! - **Composition Law**: `fa.fmap(f).fmap(g) == fa.fmap(|x| g(f(x)))`
//!
//! Using proptest, we generate random inputs to thoroughly verify these laws
//! across a wide range of values.

use lambars::typeclass::{Functor, FunctorMut, Identity};
use proptest::prelude::*;

// =============================================================================
// Option<A> Property Tests
// =============================================================================

proptest! {
    /// Identity Law for Option<i32>: fmap with identity function returns the original value
    #[test]
    fn prop_option_identity_law(value in any::<Option<i32>>()) {
        let result = value.clone().fmap(|x| x);
        prop_assert_eq!(result, value);
    }

    /// Composition Law for Option<i32>: mapping composed functions equals composing maps
    #[test]
    fn prop_option_composition_law(value in any::<Option<i32>>()) {
        let function1 = |n: i32| n.wrapping_add(1);
        let function2 = |n: i32| n.wrapping_mul(2);

        let left = value.clone().fmap(function1).fmap(function2);
        let right = value.fmap(|x| function2(function1(x)));

        prop_assert_eq!(left, right);
    }

    /// Identity Law for Option<String>
    #[test]
    fn prop_option_string_identity_law(value in any::<Option<String>>()) {
        let result = value.clone().fmap(|x| x);
        prop_assert_eq!(result, value);
    }

    /// Composition Law for Option<String>: mapping length then doubling
    #[test]
    fn prop_option_string_composition_law(value in any::<Option<String>>()) {
        let function1 = |s: String| s.len();
        let function2 = |n: usize| n.wrapping_mul(2);

        let left = value.clone().fmap(function1).fmap(function2);
        let right = value.fmap(|x| function2(function1(x)));

        prop_assert_eq!(left, right);
    }
}

// =============================================================================
// Result<T, E> Property Tests
// =============================================================================

proptest! {
    /// Identity Law for Result<i32, String>
    #[test]
    fn prop_result_identity_law(value in prop::result::maybe_ok(any::<i32>(), any::<String>())) {
        let result = value.clone().fmap(|x| x);
        prop_assert_eq!(result, value);
    }

    /// Composition Law for Result<i32, String>
    #[test]
    fn prop_result_composition_law(value in prop::result::maybe_ok(any::<i32>(), any::<String>())) {
        let function1 = |n: i32| n.wrapping_add(1);
        let function2 = |n: i32| n.wrapping_mul(2);

        let left = value.clone().fmap(function1).fmap(function2);
        let right = value.fmap(|x| function2(function1(x)));

        prop_assert_eq!(left, right);
    }
}

// =============================================================================
// Vec<A> Property Tests (using FunctorMut)
// =============================================================================

proptest! {
    /// Identity Law for Vec<i32> using fmap_mut
    #[test]
    fn prop_vec_identity_law(value in any::<Vec<i32>>()) {
        let result = value.clone().fmap_mut(|x| x);
        prop_assert_eq!(result, value);
    }

    /// Composition Law for Vec<i32> using fmap_mut
    #[test]
    fn prop_vec_composition_law(value in any::<Vec<i32>>()) {
        let function1 = |n: i32| n.wrapping_add(1);
        let function2 = |n: i32| n.wrapping_mul(2);

        let left: Vec<i32> = value.clone().fmap_mut(function1).fmap_mut(function2);
        let right: Vec<i32> = value.fmap_mut(|x| function2(function1(x)));

        prop_assert_eq!(left, right);
    }

    /// Test that fmap_mut transforms all elements correctly
    #[test]
    fn prop_vec_fmap_mut_transforms_all(values in prop::collection::vec(any::<i32>(), 0..100)) {
        let doubled: Vec<i32> = values.clone().fmap_mut(|x| x.wrapping_mul(2));

        prop_assert_eq!(doubled.len(), values.len());
        for (original, result) in values.iter().zip(doubled.iter()) {
            prop_assert_eq!(*result, original.wrapping_mul(2));
        }
    }
}

// =============================================================================
// Box<A> Property Tests
// =============================================================================

proptest! {
    /// Identity Law for Box<i32>
    #[test]
    fn prop_box_identity_law(value in any::<i32>()) {
        let boxed = Box::new(value);
        let result = boxed.fmap(|x| x);
        prop_assert_eq!(*result, value);
    }

    /// Composition Law for Box<i32>
    #[test]
    fn prop_box_composition_law(value in any::<i32>()) {
        let function1 = |n: i32| n.wrapping_add(1);
        let function2 = |n: i32| n.wrapping_mul(2);

        let left = Box::new(value).fmap(function1).fmap(function2);
        let right = Box::new(value).fmap(|x| function2(function1(x)));

        prop_assert_eq!(*left, *right);
    }

    /// Identity Law for Box<String>
    #[test]
    fn prop_box_string_identity_law(value in any::<String>()) {
        let boxed = Box::new(value.clone());
        let result = boxed.fmap(|x| x);
        prop_assert_eq!(*result, value);
    }
}

// =============================================================================
// Identity<A> Property Tests
// =============================================================================

proptest! {
    /// Identity Law for Identity<i32>
    #[test]
    fn prop_identity_wrapper_identity_law(value in any::<i32>()) {
        let wrapped = Identity::new(value);
        let result = wrapped.clone().fmap(|x| x);
        prop_assert_eq!(result, wrapped);
    }

    /// Composition Law for Identity<i32>
    #[test]
    fn prop_identity_wrapper_composition_law(value in any::<i32>()) {
        let function1 = |n: i32| n.wrapping_add(1);
        let function2 = |n: i32| n.wrapping_mul(2);

        let left = Identity::new(value).fmap(function1).fmap(function2);
        let right = Identity::new(value).fmap(|x| function2(function1(x)));

        prop_assert_eq!(left, right);
    }

    /// Identity Law for Identity<String>
    #[test]
    fn prop_identity_wrapper_string_identity_law(value in any::<String>()) {
        let wrapped = Identity::new(value.clone());
        let result = wrapped.clone().fmap(|x| x);
        prop_assert_eq!(result, wrapped);
    }

    /// Composition Law for Identity<String>
    #[test]
    fn prop_identity_wrapper_string_composition_law(value in any::<String>()) {
        let function1 = |s: String| s.len();
        let function2 = |n: usize| n.wrapping_mul(2);

        let left = Identity::new(value.clone()).fmap(function1).fmap(function2);
        let right = Identity::new(value).fmap(|x| function2(function1(x)));

        prop_assert_eq!(left, right);
    }
}

// =============================================================================
// Derived Operation Tests
// =============================================================================

proptest! {
    /// Test that replace is equivalent to fmap(|_| value)
    #[test]
    fn prop_option_replace_is_fmap_const(
        original in any::<Option<i32>>(),
        replacement in any::<String>()
    ) {
        let left = original.clone().replace(replacement.clone());
        let right = original.fmap(|_| replacement);
        prop_assert_eq!(left, right);
    }

    /// Test that void is equivalent to replace(())
    #[test]
    fn prop_option_void_is_replace_unit(value in any::<Option<i32>>()) {
        let left = value.clone().void();
        let right = value.replace(());
        prop_assert_eq!(left, right);
    }

    /// Test that replace preserves structure for Identity
    #[test]
    fn prop_identity_replace_preserves_structure(value in any::<i32>(), replacement in any::<String>()) {
        let wrapped = Identity::new(value);
        let result = wrapped.replace(replacement.clone());
        prop_assert_eq!(result, Identity::new(replacement));
    }

    /// Test that void preserves structure for Box
    #[test]
    fn prop_box_void_preserves_structure(value in any::<i32>()) {
        let boxed = Box::new(value);
        let result = boxed.void();
        prop_assert_eq!(*result, ());
    }
}

// =============================================================================
// fmap_ref Tests
// =============================================================================

proptest! {
    /// Test that fmap_ref does not consume the original Option
    #[test]
    fn prop_option_fmap_ref_preserves_original(value in any::<Option<String>>()) {
        let original = value.clone();
        let _ = original.fmap_ref(|s| s.len());
        // After fmap_ref, original should still be accessible
        prop_assert_eq!(original, value);
    }

    /// Test that fmap_ref produces the same result as fmap with cloned input
    #[test]
    fn prop_option_fmap_ref_consistent_with_fmap(value in any::<Option<i32>>()) {
        let result_ref = value.fmap_ref(|x| x.wrapping_add(1));
        let result_owned = value.fmap(|x| x.wrapping_add(1));
        prop_assert_eq!(result_ref, result_owned);
    }

    /// Test that fmap_ref does not consume the original Identity
    #[test]
    fn prop_identity_fmap_ref_preserves_original(value in any::<String>()) {
        let wrapped = Identity::new(value.clone());
        let _ = wrapped.fmap_ref(|s| s.len());
        // After fmap_ref, wrapped should still be accessible
        prop_assert_eq!(wrapped, Identity::new(value));
    }
}

// =============================================================================
// Cross-type Consistency Tests
// =============================================================================

proptest! {
    /// Test that Option::Some and Identity have consistent behavior
    #[test]
    fn prop_some_consistent_with_identity(value in any::<i32>()) {
        let function = |n: i32| n.wrapping_mul(3);

        let option_result = Some(value).fmap(function);
        let identity_result = Identity::new(value).fmap(function);

        prop_assert_eq!(option_result, Some(identity_result.into_inner()));
    }

    /// Test that Box and Identity have consistent behavior
    #[test]
    fn prop_box_consistent_with_identity(value in any::<i32>()) {
        let function = |n: i32| n.wrapping_mul(3);

        let box_result = Box::new(value).fmap(function);
        let identity_result = Identity::new(value).fmap(function);

        prop_assert_eq!(*box_result, identity_result.into_inner());
    }
}
