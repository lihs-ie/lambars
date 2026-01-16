#![cfg(feature = "typeclass")]
//! Property-based tests for Bifunctor laws.
//!
//! This module verifies that all Bifunctor implementations satisfy the required laws:
//!
//! - **Identity Law**: `bf.bimap(|x| x, |y| y) == bf`
//! - **Composition Law**: `bf.bimap(|x| f2(f1(x)), |y| g2(g1(y))) == bf.bimap(f1, g1).bimap(f2, g2)`
//! - **first/second Consistency Law**: `bf.bimap(f, g) == bf.first(f).second(g)`

use lambars::control::Either;
use lambars::typeclass::Bifunctor;
use proptest::prelude::*;

fn either_strategy() -> impl Strategy<Value = Either<i32, String>> {
    prop_oneof![
        any::<i32>().prop_map(Either::Left),
        any::<String>().prop_map(Either::Right),
    ]
}

proptest! {
    #[test]
    fn prop_either_identity_law(value in either_strategy()) {
        let result = value.clone().bimap(|x| x, |y| y);
        prop_assert_eq!(result, value);
    }

    #[test]
    fn prop_either_composition_law(value in either_strategy()) {
        let f1 = |x: i32| x.wrapping_add(1);
        let f2 = |x: i32| x.wrapping_mul(2);
        let g1 = |s: String| s.len();
        let g2 = |n: usize| n.wrapping_add(10);

        let left = value.clone().bimap(|x| f2(f1(x)), |s| g2(g1(s)));
        let right = value.bimap(f1, g1).bimap(f2, g2);

        prop_assert_eq!(left, right);
    }

    #[test]
    fn prop_either_first_second_consistency(value in either_strategy()) {
        let f = |x: i32| x.wrapping_mul(2);
        let g = |s: String| s.len();

        let by_bimap = value.clone().bimap(f, g);
        let by_first_second = value.clone().first(f).second(g);
        let by_second_first = value.second(g).first(f);

        prop_assert_eq!(by_bimap, by_first_second);
        prop_assert_eq!(by_first_second, by_second_first);
    }

    #[test]
    fn prop_result_identity_law(value in prop::result::maybe_ok(any::<i32>(), any::<String>())) {
        let result = value.clone().bimap(|e| e, |x| x);
        prop_assert_eq!(result, value);
    }

    #[test]
    fn prop_result_composition_law(value in prop::result::maybe_ok(any::<i32>(), any::<String>())) {
        let f1 = |e: String| e.len();
        let f2 = |n: usize| n.wrapping_add(100);
        let g1 = |x: i32| x.wrapping_add(1);
        let g2 = |x: i32| x.wrapping_mul(2);

        let left = value.clone().bimap(|e| f2(f1(e)), |x| g2(g1(x)));
        let right = value.bimap(f1, g1).bimap(f2, g2);

        prop_assert_eq!(left, right);
    }

    #[test]
    fn prop_result_first_second_consistency(value in prop::result::maybe_ok(any::<i32>(), any::<String>())) {
        let f = |e: String| e.len();
        let g = |x: i32| x.wrapping_mul(2);

        let by_bimap = value.clone().bimap(f, g);
        let by_first_second = value.clone().first(f).second(g);
        let by_second_first = value.second(g).first(f);

        prop_assert_eq!(by_bimap, by_first_second);
        prop_assert_eq!(by_first_second, by_second_first);
    }

    #[test]
    fn prop_tuple_identity_law(value in (any::<i32>(), any::<String>())) {
        let result = value.clone().bimap(|x| x, |y| y);
        prop_assert_eq!(result, value);
    }

    #[test]
    fn prop_tuple_composition_law(value in (any::<i32>(), any::<String>())) {
        let f1 = |x: i32| x.wrapping_add(1);
        let f2 = |x: i32| x.wrapping_mul(2);
        let g1 = |s: String| s.len();
        let g2 = |n: usize| n.wrapping_add(10);

        let left = value.clone().bimap(|x| f2(f1(x)), |s| g2(g1(s)));
        let right = value.bimap(f1, g1).bimap(f2, g2);

        prop_assert_eq!(left, right);
    }

    #[test]
    fn prop_tuple_first_second_consistency(value in (any::<i32>(), any::<String>())) {
        let f = |x: i32| x.wrapping_mul(2);
        let g = |s: String| s.len();

        let by_bimap = value.clone().bimap(f, g);
        let by_first_second = value.clone().first(f).second(g);
        let by_second_first = value.second(g).first(f);

        prop_assert_eq!(by_bimap, by_first_second);
        prop_assert_eq!(by_first_second, by_second_first);
    }
}
