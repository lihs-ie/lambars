#![cfg(feature = "typeclass")]
//! Property-based tests for Alternative type class laws.
//!
//! This module tests the fundamental laws that all Alternative implementations must satisfy:
//!
//! ## Monoid Laws (Required)
//!
//! 1. **Left Identity**: `empty.alt(x) == x`
//! 2. **Right Identity**: `x.alt(empty) == x`
//! 3. **Associativity**: `(x.alt(y)).alt(z) == x.alt(y.alt(z))`
//!
//! ## Interaction with Applicative (Recommended)
//!
//! 4. **Left Absorption**: `empty.apply(x) == empty`
//! 5. **Right Absorption**: `ff.apply(empty) == empty`
//! 6. **Left Distributivity**: `(fa.alt(fb)).fmap(f) == fa.fmap(f).alt(fb.fmap(f))`

use lambars::typeclass::{Alternative, AlternativeVec, Applicative, ApplicativeVec, Functor};
use proptest::prelude::*;
use rstest::rstest;

proptest! {
    #[test]
    fn prop_option_left_identity(value in any::<Option<i32>>()) {
        let empty: Option<i32> = <Option<()>>::empty();
        prop_assert_eq!(empty.alt(value), value);
    }

    #[test]
    fn prop_option_right_identity(value in any::<Option<i32>>()) {
        let empty: Option<i32> = <Option<()>>::empty();
        prop_assert_eq!(value.alt(empty), value);
    }

    #[test]
    fn prop_option_associativity(
        x in any::<Option<i32>>(),
        y in any::<Option<i32>>(),
        z in any::<Option<i32>>()
    ) {
        let left = x.alt(y).alt(z);
        let right = x.alt(y.alt(z));
        prop_assert_eq!(left, right);
    }

    #[test]
    fn prop_option_left_absorption(value in any::<Option<i32>>()) {
        let empty: Option<fn(i32) -> i32> = <Option<()>>::empty();
        let result: Option<i32> = empty.apply(value);
        let expected: Option<i32> = <Option<()>>::empty();
        prop_assert_eq!(result, expected);
    }

    #[test]
    fn prop_option_right_absorption(_value in any::<i32>()) {
        let function: Option<fn(i32) -> i32> = Some(|x| x.wrapping_mul(2));
        let empty: Option<i32> = <Option<()>>::empty();
        let result: Option<i32> = function.apply(empty);
        let expected: Option<i32> = <Option<()>>::empty();
        prop_assert_eq!(result, expected);
    }

    #[test]
    fn prop_option_left_distributivity(
        fa in any::<Option<i32>>(),
        fb in any::<Option<i32>>()
    ) {
        let function = |n: i32| n.wrapping_mul(2);
        let left = fa.alt(fb).fmap(function);
        let right = fa.fmap(function).alt(fb.fmap(function));
        prop_assert_eq!(left, right);
    }

    #[test]
    fn prop_vec_left_identity(value in prop::collection::vec(any::<i32>(), 0..10)) {
        let empty: Vec<i32> = Vec::<()>::empty();
        prop_assert_eq!(empty.alt(value.clone()), value);
    }

    #[test]
    fn prop_vec_right_identity(value in prop::collection::vec(any::<i32>(), 0..10)) {
        let empty: Vec<i32> = Vec::<()>::empty();
        prop_assert_eq!(value.clone().alt(empty), value);
    }

    #[test]
    fn prop_vec_associativity(
        x in prop::collection::vec(any::<i32>(), 0..5),
        y in prop::collection::vec(any::<i32>(), 0..5),
        z in prop::collection::vec(any::<i32>(), 0..5)
    ) {
        let left = x.clone().alt(y.clone()).alt(z.clone());
        let right = x.alt(y.alt(z));
        prop_assert_eq!(left, right);
    }

    #[test]
    fn prop_vec_left_absorption(value in prop::collection::vec(any::<i32>(), 0..5)) {
        let empty: Vec<fn(i32) -> i32> = Vec::<()>::empty();
        let result: Vec<i32> = empty.apply(value);
        let expected: Vec<i32> = Vec::<()>::empty();
        prop_assert_eq!(result, expected);
    }

    #[test]
    fn prop_vec_right_absorption(_value in any::<i32>()) {
        let function: Vec<fn(i32) -> i32> = vec![|x| x.wrapping_mul(2)];
        let empty: Vec<i32> = Vec::<()>::empty();
        let result: Vec<i32> = function.apply(empty);
        let expected: Vec<i32> = Vec::<()>::empty();
        prop_assert_eq!(result, expected);
    }

    #[test]
    fn prop_vec_left_distributivity(
        fa in prop::collection::vec(any::<i32>(), 0..5),
        fb in prop::collection::vec(any::<i32>(), 0..5)
    ) {
        use lambars::typeclass::FunctorMut;

        let function = |n: i32| n.wrapping_mul(2);
        let left: Vec<i32> = fa.clone().alt(fb.clone()).fmap_mut(function);
        let right: Vec<i32> = fa.fmap_mut(function).alt(fb.fmap_mut(function));
        prop_assert_eq!(left, right);
    }
}

#[rstest]
fn option_guard_true_returns_pure_unit() {
    let result: Option<()> = <Option<()>>::guard(true);
    assert_eq!(result, Some(()));
}

#[rstest]
fn option_guard_false_returns_empty() {
    let result: Option<()> = <Option<()>>::guard(false);
    assert_eq!(result, None);
}

#[rstest]
fn vec_guard_true_returns_pure_unit() {
    let result: Vec<()> = Vec::<()>::guard(true);
    assert_eq!(result, vec![()]);
}

#[rstest]
fn vec_guard_false_returns_empty() {
    let result: Vec<()> = Vec::<()>::guard(false);
    assert!(result.is_empty());
}

#[rstest]
fn option_optional_some_returns_some_some() {
    let value: Option<i32> = Some(42);
    let result: Option<Option<i32>> = value.optional();
    assert_eq!(result, Some(Some(42)));
}

#[rstest]
fn option_optional_none_returns_some_none() {
    let value: Option<i32> = None;
    let result: Option<Option<i32>> = value.optional();
    assert_eq!(result, Some(None));
}

#[rstest]
fn vec_optional_non_empty_returns_vec_with_some() {
    let value: Vec<i32> = vec![1, 2, 3];
    let result: Vec<Option<i32>> = value.optional();
    assert!(result.contains(&Some(1)));
    assert!(result.contains(&Some(2)));
    assert!(result.contains(&Some(3)));
    assert!(result.contains(&None));
}

#[rstest]
fn vec_optional_empty_returns_vec_with_none() {
    let value: Vec<i32> = vec![];
    let result: Vec<Option<i32>> = value.optional();
    assert_eq!(result, vec![None]);
}

#[rstest]
fn option_choice_first_some_wins() {
    let alternatives = vec![None, Some(1), Some(2)];
    let result: Option<i32> = Option::choice(alternatives);
    assert_eq!(result, Some(1));
}

#[rstest]
fn option_choice_all_none_returns_none() {
    let alternatives: Vec<Option<i32>> = vec![None, None, None];
    let result: Option<i32> = Option::choice(alternatives);
    assert_eq!(result, None);
}

#[rstest]
fn option_choice_empty_iterator_returns_empty() {
    let alternatives: Vec<Option<i32>> = vec![];
    let result: Option<i32> = Option::choice(alternatives);
    assert_eq!(result, None);
}

#[rstest]
fn vec_choice_concatenates_all() {
    let alternatives = vec![vec![1, 2], vec![3], vec![4, 5, 6]];
    let result: Vec<i32> = Vec::choice(alternatives);
    assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
}

#[rstest]
fn vec_choice_with_empty_vecs() {
    let alternatives: Vec<Vec<i32>> = vec![vec![], vec![1], vec![]];
    let result: Vec<i32> = Vec::choice(alternatives);
    assert_eq!(result, vec![1]);
}

#[rstest]
fn vec_choice_empty_iterator_returns_empty() {
    let alternatives: Vec<Vec<i32>> = vec![];
    let result: Vec<i32> = Vec::choice(alternatives);
    assert!(result.is_empty());
}

#[rstest]
fn option_filter_with_guard() {
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

    fn try_parse_default(_s: &str) -> Option<i32> {
        Some(0)
    }

    let input = "not a number";
    let result = try_parse_int(input).alt(try_parse_default(input));
    assert_eq!(result, Some(0));

    let input = "42";
    let result = try_parse_int(input).alt(try_parse_default(input));
    assert_eq!(result, Some(42));
}

#[rstest]
fn vec_nondeterministic_choice() {
    let path_a = vec![1, 2];
    let path_b = vec![3, 4];
    let all_paths = path_a.alt(path_b);
    assert_eq!(all_paths, vec![1, 2, 3, 4]);
}
