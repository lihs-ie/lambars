//! Property-based tests for pattern guard (if let) in for_! macro.

#![cfg(feature = "compose")]

use lambars::for_;
use proptest::prelude::*;

proptest! {
    /// Pattern guard with Option is equivalent to Iterator::flatten
    #[test]
    fn pattern_guard_option_equivalence(values in prop::collection::vec(
        prop::option::of(-100i32..100i32),
        0..20
    )) {
        let result1 = for_! {
            opt <= values.clone();
            if let Some(x) = opt;
            yield x
        };

        let result2: Vec<i32> = values.into_iter().flatten().collect();

        prop_assert_eq!(result1, result2);
    }

    /// Pattern guard is equivalent to filter_map
    #[test]
    fn pattern_guard_filter_map_equivalence(values in prop::collection::vec(-100i32..100i32, 0..20)) {
        let result1 = for_! {
            x <= values.clone();
            if let Some(doubled) = if x > 0 { Some(x * 2) } else { None };
            yield doubled
        };

        let result2: Vec<i32> = values.into_iter()
            .filter_map(|x| if x > 0 { Some(x * 2) } else { None })
            .collect();

        prop_assert_eq!(result1, result2);
    }

    /// Pattern guard preserves order of elements
    #[test]
    fn pattern_guard_order_preservation(values in prop::collection::vec(
        prop::option::of(0i32..1000),
        0..50
    )) {
        let result = for_! {
            opt <= values.clone();
            if let Some(x) = opt;
            yield x
        };

        // Verify order is preserved
        let expected: Vec<i32> = values.into_iter().flatten().collect();
        prop_assert_eq!(result, expected);
    }

    /// Multiple consecutive pattern guards act as logical AND
    #[test]
    fn multiple_pattern_guards_conjunction(values in prop::collection::vec(
        prop::option::of(prop::option::of(-50i32..50)),
        0..20
    )) {
        let result1 = for_! {
            outer <= values.clone();
            if let Some(inner) = outer;
            if let Some(x) = inner;
            yield x
        };

        let result2: Vec<i32> = values.into_iter()
            .flatten()
            .flatten()
            .collect();

        prop_assert_eq!(result1, result2);
    }

    /// Pattern guard with Result is equivalent to filter_map with Result::ok
    #[test]
    fn pattern_guard_result_ok_equivalence(values in prop::collection::vec(-50i32..50, 0..20)) {
        fn try_double(x: i32) -> Result<i32, &'static str> {
            if x > 0 {
                Ok(x * 2)
            } else {
                Err("negative or zero")
            }
        }

        let result1 = for_! {
            x <= values.clone();
            if let Ok(doubled) = try_double(x);
            yield doubled
        };

        let result2: Vec<i32> = values.into_iter()
            .filter_map(|x| try_double(x).ok())
            .collect();

        prop_assert_eq!(result1, result2);
    }

    /// Pattern guard preserves empty collection behavior
    #[test]
    fn pattern_guard_empty_collection(_dummy: u8) {
        let empty: Vec<Option<i32>> = vec![];
        let result = for_! {
            opt <= empty;
            if let Some(x) = opt;
            yield x
        };
        prop_assert!(result.is_empty());
    }

    /// Pattern guard with all None returns empty result
    #[test]
    fn pattern_guard_all_none(count in 0usize..20) {
        let all_none: Vec<Option<i32>> = vec![None; count];
        let result = for_! {
            opt <= all_none;
            if let Some(x) = opt;
            yield x
        };
        prop_assert!(result.is_empty());
    }

    /// Pattern guard combined with regular guard
    #[test]
    fn pattern_guard_with_regular_guard(values in prop::collection::vec(
        prop::option::of(0i32..100),
        0..20
    )) {
        let threshold = 50;

        let result1 = for_! {
            opt <= values.clone();
            if let Some(x) = opt;
            if x > threshold;
            yield x
        };

        let result2: Vec<i32> = values.into_iter()
            .flatten()
            .filter(|&x| x > threshold)
            .collect();

        prop_assert_eq!(result1, result2);
    }

    /// Pattern guard with let binding
    #[test]
    fn pattern_guard_with_let_binding(values in prop::collection::vec(
        prop::option::of(1i32..100),
        0..20
    )) {
        let result1 = for_! {
            opt <= values.clone();
            if let Some(x) = opt;
            let doubled = x * 2;
            yield doubled
        };

        let result2: Vec<i32> = values.into_iter()
            .flatten()
            .map(|x| x * 2)
            .collect();

        prop_assert_eq!(result1, result2);
    }
}
