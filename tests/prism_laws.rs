//! Property-based tests for Prism laws.
//!
//! This module tests that Prism implementations satisfy the fundamental laws:
//!
//! 1. **PreviewReview Law**: `prism.preview(&prism.review(value)) == Some(&value)`
//! 2. **ReviewPreview Law**: If preview succeeds, then `prism.review(prism.preview(&source).unwrap().clone()) == source`

use lambars::optics::Prism;
use lambars::prism;
use proptest::prelude::*;

// =============================================================================
// Test data types
// =============================================================================

#[derive(Clone, PartialEq, Debug)]
enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
}

#[derive(Clone, PartialEq, Debug)]
enum MyOption<T> {
    Some(T),
    None,
}

#[derive(Clone, PartialEq, Debug)]
enum MyResult<T, E> {
    Ok(T),
    Err(E),
}

// =============================================================================
// PreviewReview Law tests
// =============================================================================

proptest! {
    /// PreviewReview Law: prism.preview(&prism.review(value)) == Some(&value)
    /// This ensures that review creates a value that can be successfully previewed.
    #[test]
    fn prop_preview_review_law_circle(radius in -1000.0f64..1000.0) {
        let circle_prism = prism!(Shape, Circle);
        let source = circle_prism.review(radius);
        let previewed = circle_prism.preview(&source);
        prop_assert!(previewed.is_some());
        prop_assert!((previewed.unwrap() - radius).abs() < 1e-10);
    }

    #[test]
    fn prop_preview_review_law_my_option_some(value in any::<i32>()) {
        let some_prism = prism!(MyOption<i32>, Some);
        let source = some_prism.review(value);
        let previewed = some_prism.preview(&source);
        prop_assert_eq!(previewed, Some(&value));
    }

    #[test]
    fn prop_preview_review_law_my_result_ok(value in any::<i32>()) {
        let ok_prism = prism!(MyResult<i32, String>, Ok);
        let source = ok_prism.review(value);
        let previewed = ok_prism.preview(&source);
        prop_assert_eq!(previewed, Some(&value));
    }

    #[test]
    fn prop_preview_review_law_my_result_err(error in "[a-z]{1,10}") {
        let err_prism = prism!(MyResult<i32, String>, Err);
        let source = err_prism.review(error.clone());
        let previewed = err_prism.preview(&source);
        prop_assert_eq!(previewed, Some(&error));
    }
}

// =============================================================================
// ReviewPreview Law tests
// =============================================================================

proptest! {
    /// ReviewPreview Law: If preview succeeds, then review(preview(&source).unwrap().clone()) == source
    /// This ensures that review can reconstruct the original source from a successful preview.
    #[test]
    fn prop_review_preview_law_circle(radius in -1000.0f64..1000.0) {
        let circle_prism = prism!(Shape, Circle);
        let source = Shape::Circle(radius);

        if let Some(value) = circle_prism.preview(&source) {
            let reconstructed = circle_prism.review(*value);
            // Compare Shape variants and values
            match (&reconstructed, &source) {
                (Shape::Circle(r1), Shape::Circle(r2)) => {
                    prop_assert!((r1 - r2).abs() < 1e-10);
                }
                _ => prop_assert!(false, "Expected Circle variant"),
            }
        }
    }

    #[test]
    fn prop_review_preview_law_my_option_some(value in any::<i32>()) {
        let some_prism = prism!(MyOption<i32>, Some);
        let source = MyOption::Some(value);

        if let Some(previewed) = some_prism.preview(&source) {
            let reconstructed = some_prism.review(*previewed);
            prop_assert_eq!(reconstructed, source);
        }
    }

    #[test]
    fn prop_review_preview_law_my_result_ok(value in any::<i32>()) {
        let ok_prism = prism!(MyResult<i32, String>, Ok);
        let source: MyResult<i32, String> = MyResult::Ok(value);

        if let Some(previewed) = ok_prism.preview(&source) {
            let reconstructed = ok_prism.review(*previewed);
            prop_assert_eq!(reconstructed, source);
        }
    }

    #[test]
    fn prop_review_preview_law_my_result_err(error in "[a-z]{1,10}") {
        let err_prism = prism!(MyResult<i32, String>, Err);
        let source: MyResult<i32, String> = MyResult::Err(error.clone());

        if let Some(previewed) = err_prism.preview(&source) {
            let reconstructed = err_prism.review(previewed.clone());
            prop_assert_eq!(reconstructed, source);
        }
    }
}

// =============================================================================
// Additional property tests
// =============================================================================

proptest! {
    /// Preview should return None for non-matching variants
    #[test]
    fn prop_preview_returns_none_for_non_matching(width in 0.0f64..100.0, height in 0.0f64..100.0) {
        let circle_prism = prism!(Shape, Circle);
        let rect = Shape::Rectangle(width, height);
        prop_assert!(circle_prism.preview(&rect).is_none());
    }

    /// modify_option should return None for non-matching variants
    #[test]
    fn prop_modify_option_returns_none_for_non_matching(width in 0.0f64..100.0, height in 0.0f64..100.0) {
        let circle_prism = prism!(Shape, Circle);
        let rect = Shape::Rectangle(width, height);
        let result = circle_prism.modify_option(rect, |r| r * 2.0);
        prop_assert!(result.is_none());
    }

    /// modify_or_identity should return original for non-matching variants
    #[test]
    fn prop_modify_or_identity_returns_original(width in 0.0f64..100.0, height in 0.0f64..100.0) {
        let circle_prism = prism!(Shape, Circle);
        let rect = Shape::Rectangle(width, height);
        let result = circle_prism.modify_or_identity(rect.clone(), |r| r * 2.0);
        prop_assert_eq!(result, rect);
    }

    /// Composed prism should satisfy PreviewReview law
    #[test]
    fn prop_composed_prism_preview_review_law(value in any::<i32>()) {
        #[allow(dead_code)]
        #[derive(Clone, PartialEq, Debug)]
        enum Outer { Inner(Inner), Empty }

        #[allow(dead_code)]
        #[derive(Clone, PartialEq, Debug)]
        enum Inner { Value(i32), Nothing }

        use lambars::optics::FunctionPrism;

        let outer_inner_prism = FunctionPrism::new(
            |outer: &Outer| match outer {
                Outer::Inner(inner) => Some(inner),
                _ => None,
            },
            |inner: Inner| Outer::Inner(inner),
            |outer: Outer| match outer {
                Outer::Inner(inner) => Some(inner),
                _ => None,
            },
        );

        let inner_value_prism = FunctionPrism::new(
            |inner: &Inner| match inner {
                Inner::Value(v) => Some(v),
                _ => None,
            },
            |v: i32| Inner::Value(v),
            |inner: Inner| match inner {
                Inner::Value(v) => Some(v),
                _ => None,
            },
        );

        let composed = outer_inner_prism.compose(inner_value_prism);

        // PreviewReview law for composed prism
        let source = composed.review(value);
        let previewed = composed.preview(&source);
        prop_assert_eq!(previewed, Some(&value));
    }
}

// =============================================================================
// Tests for owned preview
// =============================================================================

proptest! {
    /// preview_owned should extract the value by ownership
    #[test]
    fn prop_preview_owned_extracts_value(value in any::<i32>()) {
        let some_prism = prism!(MyOption<i32>, Some);
        let source = MyOption::Some(value);
        let owned = some_prism.preview_owned(source);
        prop_assert_eq!(owned, Some(value));
    }

    /// preview_owned should return None for non-matching variants
    #[test]
    fn prop_preview_owned_returns_none_for_non_matching(_value in any::<i32>()) {
        let some_prism = prism!(MyOption<i32>, Some);
        let source: MyOption<i32> = MyOption::None;
        let owned = some_prism.preview_owned(source);
        prop_assert!(owned.is_none());
    }
}
