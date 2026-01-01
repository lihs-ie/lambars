//! Property-based tests for Traversal laws.
//!
//! Traversal Laws:
//!
//! 1. **Modify Identity Law**: Applying the identity function via modify_all yields the original.
//!    ```text
//!    traversal.modify_all(source, |x| x) == source
//!    ```
//!
//! 2. **Modify Composition Law**: Consecutive modify_all calls are equivalent to a single composed call.
//!    ```text
//!    traversal.modify_all(traversal.modify_all(source, f), g) == traversal.modify_all(source, |x| g(f(x)))
//!    ```

#![forbid(unsafe_code)]

use lambars::lens;
use lambars::optics::{
    Lens, OptionTraversal, Prism, ResultTraversal, Traversal, VecTraversal,
};
use lambars::prism;
use proptest::prelude::*;

// =============================================================================
// VecTraversal Laws
// =============================================================================

proptest! {
    #[test]
    fn prop_vec_traversal_modify_identity_law(elements in prop::collection::vec(any::<i32>(), 0..20)) {
        let traversal: VecTraversal<i32> = VecTraversal::new();
        let modified = traversal.modify_all(elements.clone(), |x| x);
        prop_assert_eq!(modified, elements);
    }

    #[test]
    fn prop_vec_traversal_modify_composition_law(elements in prop::collection::vec(any::<i32>(), 0..20)) {
        let traversal: VecTraversal<i32> = VecTraversal::new();

        // Using saturating operations to avoid overflow
        let function1 = |x: i32| x.saturating_add(1);
        let function2 = |x: i32| x.saturating_mul(2);

        let left = traversal.modify_all(
            traversal.modify_all(elements.clone(), function1),
            function2
        );
        let right = traversal.modify_all(elements, |x| function2(function1(x)));

        prop_assert_eq!(left, right);
    }

    #[test]
    fn prop_vec_traversal_set_all_modify_all_equivalence(
        elements in prop::collection::vec(any::<i32>(), 0..20),
        value in any::<i32>()
    ) {
        let traversal: VecTraversal<i32> = VecTraversal::new();

        let set_result = traversal.set_all(elements.clone(), value);
        let modify_result = traversal.modify_all(elements.clone(), |_| value);

        prop_assert_eq!(set_result, modify_result);
    }

    #[test]
    fn prop_vec_traversal_length_equals_vec_len(elements in prop::collection::vec(any::<i32>(), 0..50)) {
        let traversal: VecTraversal<i32> = VecTraversal::new();
        prop_assert_eq!(traversal.length(&elements), elements.len());
    }

    #[test]
    fn prop_vec_traversal_get_all_preserves_elements(elements in prop::collection::vec(any::<i32>(), 0..20)) {
        let traversal: VecTraversal<i32> = VecTraversal::new();
        let collected: Vec<i32> = traversal.get_all(&elements).cloned().collect();
        prop_assert_eq!(collected, elements);
    }

    #[test]
    fn prop_vec_traversal_fold_sum_equals_iter_sum(elements in prop::collection::vec(any::<i64>(), 0..20)) {
        let traversal: VecTraversal<i64> = VecTraversal::new();
        let fold_sum = traversal.fold(&elements, 0i64, |accumulator, element| accumulator.saturating_add(*element));
        let iter_sum: i64 = elements.iter().fold(0i64, |accumulator, &element| accumulator.saturating_add(element));
        prop_assert_eq!(fold_sum, iter_sum);
    }

    #[test]
    fn prop_vec_traversal_for_all_consistency(
        elements in prop::collection::vec(0..100i32, 0..20),
        threshold in any::<i32>()
    ) {
        let traversal: VecTraversal<i32> = VecTraversal::new();
        let traversal_result = traversal.for_all(&elements, |x| *x < threshold);
        let iter_result = elements.iter().all(|x| *x < threshold);
        prop_assert_eq!(traversal_result, iter_result);
    }

    #[test]
    fn prop_vec_traversal_exists_consistency(
        elements in prop::collection::vec(0..100i32, 0..20),
        threshold in any::<i32>()
    ) {
        let traversal: VecTraversal<i32> = VecTraversal::new();
        let traversal_result = traversal.exists(&elements, |x| *x >= threshold);
        let iter_result = elements.iter().any(|x| *x >= threshold);
        prop_assert_eq!(traversal_result, iter_result);
    }
}

// =============================================================================
// OptionTraversal Laws
// =============================================================================

proptest! {
    #[test]
    fn prop_option_traversal_modify_identity_law_some(value in any::<i32>()) {
        let traversal: OptionTraversal<i32> = OptionTraversal::new();
        let source = Some(value);
        let modified = traversal.modify_all(source.clone(), |x| x);
        prop_assert_eq!(modified, source);
    }

    #[test]
    fn prop_option_traversal_modify_composition_law_some(value in any::<i32>()) {
        let traversal: OptionTraversal<i32> = OptionTraversal::new();
        let source = Some(value);

        let function1 = |x: i32| x.saturating_add(5);
        let function2 = |x: i32| x.saturating_mul(3);

        let left = traversal.modify_all(
            traversal.modify_all(source.clone(), function1),
            function2
        );
        let right = traversal.modify_all(source, |x| function2(function1(x)));

        prop_assert_eq!(left, right);
    }

    #[test]
    fn prop_option_traversal_length(maybe_value in proptest::option::of(any::<i32>())) {
        let traversal: OptionTraversal<i32> = OptionTraversal::new();
        let expected_length = if maybe_value.is_some() { 1 } else { 0 };
        prop_assert_eq!(traversal.length(&maybe_value), expected_length);
    }
}

// Non-proptest tests for None cases
#[test]
fn test_option_traversal_modify_identity_law_none() {
    let traversal: OptionTraversal<i32> = OptionTraversal::new();
    let source: Option<i32> = None;
    let modified = traversal.modify_all(source.clone(), |x| x);
    assert_eq!(modified, source);
}

#[test]
fn test_option_traversal_modify_composition_law_none() {
    let traversal: OptionTraversal<i32> = OptionTraversal::new();
    let source: Option<i32> = None;

    let function1 = |x: i32| x.saturating_add(5);
    let function2 = |x: i32| x.saturating_mul(3);

    let left = traversal.modify_all(traversal.modify_all(source.clone(), function1), function2);
    let right = traversal.modify_all(source, |x| function2(function1(x)));

    assert_eq!(left, right);
}

// =============================================================================
// ResultTraversal Laws
// =============================================================================

proptest! {
    #[test]
    fn prop_result_traversal_modify_identity_law_ok(value in any::<i32>()) {
        let traversal: ResultTraversal<i32, String> = ResultTraversal::new();
        let source: Result<i32, String> = Ok(value);
        let modified = traversal.modify_all(source.clone(), |x| x);
        prop_assert_eq!(modified, source);
    }

    #[test]
    fn prop_result_traversal_modify_identity_law_err(error in "[a-z]{1,10}") {
        let traversal: ResultTraversal<i32, String> = ResultTraversal::new();
        let source: Result<i32, String> = Err(error.clone());
        let modified = traversal.modify_all(source.clone(), |x| x);
        prop_assert_eq!(modified, source);
    }

    #[test]
    fn prop_result_traversal_modify_composition_law_ok(value in any::<i32>()) {
        let traversal: ResultTraversal<i32, String> = ResultTraversal::new();
        let source: Result<i32, String> = Ok(value);

        let function1 = |x: i32| x.saturating_add(10);
        let function2 = |x: i32| x.saturating_sub(3);

        let left = traversal.modify_all(
            traversal.modify_all(source.clone(), function1),
            function2
        );
        let right = traversal.modify_all(source, |x| function2(function1(x)));

        prop_assert_eq!(left, right);
    }

    #[test]
    fn prop_result_traversal_modify_composition_law_err(error in "[a-z]{1,10}") {
        let traversal: ResultTraversal<i32, String> = ResultTraversal::new();
        let source: Result<i32, String> = Err(error.clone());

        let function1 = |x: i32| x.saturating_add(10);
        let function2 = |x: i32| x.saturating_sub(3);

        let left = traversal.modify_all(
            traversal.modify_all(source.clone(), function1),
            function2
        );
        let right = traversal.modify_all(source, |x| function2(function1(x)));

        prop_assert_eq!(left, right);
    }

    #[test]
    fn prop_result_traversal_err_preserved(error in "[a-z]{1,10}") {
        let traversal: ResultTraversal<i32, String> = ResultTraversal::new();
        let source: Result<i32, String> = Err(error.clone());
        let modified = traversal.modify_all(source, |x| x * 100);
        prop_assert_eq!(modified, Err(error));
    }
}

// =============================================================================
// ComposedTraversal Laws
// =============================================================================

proptest! {
    #[test]
    fn prop_composed_traversal_modify_identity_law(
        nested in prop::collection::vec(prop::collection::vec(any::<i32>(), 0..5), 0..5)
    ) {
        let outer: VecTraversal<Vec<i32>> = VecTraversal::new();
        let inner: VecTraversal<i32> = VecTraversal::new();
        let composed = outer.compose(inner);

        let modified = composed.modify_all(nested.clone(), |x| x);
        prop_assert_eq!(modified, nested);
    }

    #[test]
    fn prop_composed_traversal_modify_composition_law(
        nested in prop::collection::vec(prop::collection::vec(any::<i32>(), 0..5), 0..5)
    ) {
        let outer: VecTraversal<Vec<i32>> = VecTraversal::new();
        let inner: VecTraversal<i32> = VecTraversal::new();
        let composed = outer.compose(inner);

        let function1 = |x: i32| x.saturating_add(1);
        let function2 = |x: i32| x.saturating_mul(2);

        let left = composed.modify_all(
            composed.modify_all(nested.clone(), function1),
            function2
        );
        let right = composed.modify_all(nested, |x| function2(function1(x)));

        prop_assert_eq!(left, right);
    }

    #[test]
    fn prop_composed_traversal_length_sum(
        nested in prop::collection::vec(prop::collection::vec(any::<i32>(), 0..5), 0..5)
    ) {
        let outer: VecTraversal<Vec<i32>> = VecTraversal::new();
        let inner: VecTraversal<i32> = VecTraversal::new();
        let composed = outer.compose(inner);

        let traversal_length = composed.length(&nested);
        let expected_length: usize = nested.iter().map(|v| v.len()).sum();

        prop_assert_eq!(traversal_length, expected_length);
    }
}

// =============================================================================
// LensAsTraversal Laws
// =============================================================================

#[derive(Clone, PartialEq, Debug)]
struct TestContainer {
    value: i32,
}

proptest! {
    #[test]
    fn prop_lens_as_traversal_modify_identity_law(value in any::<i32>()) {
        let value_lens = lens!(TestContainer, value);
        let traversal = value_lens.to_traversal();

        let source = TestContainer { value };
        let modified = traversal.modify_all(source.clone(), |x| x);

        prop_assert_eq!(modified, source);
    }

    #[test]
    fn prop_lens_as_traversal_modify_composition_law(value in any::<i32>()) {
        let value_lens = lens!(TestContainer, value);
        let traversal = value_lens.to_traversal();

        let source = TestContainer { value };

        let function1 = |x: i32| x.saturating_add(7);
        let function2 = |x: i32| x.saturating_mul(11);

        let left = traversal.modify_all(
            traversal.modify_all(source.clone(), function1),
            function2
        );
        let right = traversal.modify_all(source, |x| function2(function1(x)));

        prop_assert_eq!(left, right);
    }

    #[test]
    fn prop_lens_as_traversal_length_is_one(value in any::<i32>()) {
        let value_lens = lens!(TestContainer, value);
        let traversal = value_lens.to_traversal();

        let source = TestContainer { value };
        prop_assert_eq!(traversal.length(&source), 1);
    }
}

// =============================================================================
// PrismAsTraversal Laws
// =============================================================================

#[derive(Clone, PartialEq, Debug)]
enum TestOption<T> {
    Some(T),
    None,
}

proptest! {
    #[test]
    fn prop_prism_as_traversal_modify_identity_law_some(value in any::<i32>()) {
        let some_prism = prism!(TestOption<i32>, Some);
        let traversal = some_prism.to_traversal();

        let source = TestOption::Some(value);
        let modified = traversal.modify_all(source.clone(), |x| x);

        prop_assert_eq!(modified, source);
    }

    #[test]
    fn prop_prism_as_traversal_modify_composition_law(value in any::<i32>()) {
        let some_prism = prism!(TestOption<i32>, Some);
        let traversal = some_prism.to_traversal();

        let source = TestOption::Some(value);

        let function1 = |x: i32| x.saturating_add(13);
        let function2 = |x: i32| x.saturating_mul(17);

        let left = traversal.modify_all(
            traversal.modify_all(source.clone(), function1),
            function2
        );
        let right = traversal.modify_all(source, |x| function2(function1(x)));

        prop_assert_eq!(left, right);
    }
}

// Non-proptest test for None case
#[test]
fn test_prism_as_traversal_modify_identity_law_none() {
    let some_prism = prism!(TestOption<i32>, Some);
    let traversal = some_prism.to_traversal();

    let source: TestOption<i32> = TestOption::None;
    let modified = traversal.modify_all(source.clone(), |x| x);

    assert_eq!(modified, source);
}
