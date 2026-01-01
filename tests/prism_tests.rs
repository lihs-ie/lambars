//! Unit tests for Prism optics.
//!
//! Tests cover:
//! - Basic preview and review operations
//! - modify_option and modify_or_identity
//! - Prism composition
//! - prism! macro

use lambars::optics::{FunctionPrism, Prism};
use lambars::prism;
use rstest::rstest;

// =============================================================================
// Test data types
// =============================================================================

#[derive(Clone, PartialEq, Debug)]
enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
    Triangle(f64, f64, f64),
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

#[derive(Clone, PartialEq, Debug)]
enum Outer {
    Inner(Inner),
    Empty,
}

#[derive(Clone, PartialEq, Debug)]
enum Inner {
    Value(i32),
    Nothing,
}

// =============================================================================
// Prism trait existence tests
// =============================================================================

#[test]
fn test_prism_trait_exists() {
    fn assert_prism<P: Prism<Shape, f64>>(_prism: P) {}

    let circle_prism = FunctionPrism::new(
        |shape: &Shape| match shape {
            Shape::Circle(radius) => Some(radius),
            _ => None,
        },
        |radius: f64| Shape::Circle(radius),
        |shape: Shape| match shape {
            Shape::Circle(radius) => Some(radius),
            _ => None,
        },
    );

    assert_prism(circle_prism);
}

// =============================================================================
// FunctionPrism basic operations
// =============================================================================

#[test]
fn test_function_prism_preview_match() {
    let circle_prism = FunctionPrism::new(
        |shape: &Shape| match shape {
            Shape::Circle(radius) => Some(radius),
            _ => None,
        },
        |radius: f64| Shape::Circle(radius),
        |shape: Shape| match shape {
            Shape::Circle(radius) => Some(radius),
            _ => None,
        },
    );

    let circle = Shape::Circle(5.0);
    assert_eq!(circle_prism.preview(&circle), Some(&5.0));
}

#[test]
fn test_function_prism_preview_no_match() {
    let circle_prism = FunctionPrism::new(
        |shape: &Shape| match shape {
            Shape::Circle(radius) => Some(radius),
            _ => None,
        },
        |radius: f64| Shape::Circle(radius),
        |shape: Shape| match shape {
            Shape::Circle(radius) => Some(radius),
            _ => None,
        },
    );

    let rect = Shape::Rectangle(3.0, 4.0);
    assert_eq!(circle_prism.preview(&rect), None);
}

#[test]
fn test_function_prism_review() {
    let circle_prism = FunctionPrism::new(
        |shape: &Shape| match shape {
            Shape::Circle(radius) => Some(radius),
            _ => None,
        },
        |radius: f64| Shape::Circle(radius),
        |shape: Shape| match shape {
            Shape::Circle(radius) => Some(radius),
            _ => None,
        },
    );

    let constructed = circle_prism.review(10.0);
    assert!(matches!(constructed, Shape::Circle(r) if (r - 10.0).abs() < 1e-10));
}

#[test]
fn test_function_prism_preview_owned() {
    let circle_prism = FunctionPrism::new(
        |shape: &Shape| match shape {
            Shape::Circle(radius) => Some(radius),
            _ => None,
        },
        |radius: f64| Shape::Circle(radius),
        |shape: Shape| match shape {
            Shape::Circle(radius) => Some(radius),
            _ => None,
        },
    );

    let circle = Shape::Circle(5.0);
    assert_eq!(circle_prism.preview_owned(circle), Some(5.0));

    let rect = Shape::Rectangle(3.0, 4.0);
    assert_eq!(circle_prism.preview_owned(rect), None);
}

// =============================================================================
// modify_option and modify_or_identity tests
// =============================================================================

#[test]
fn test_prism_modify_option_match() {
    let circle_prism = FunctionPrism::new(
        |shape: &Shape| match shape {
            Shape::Circle(radius) => Some(radius),
            _ => None,
        },
        |radius: f64| Shape::Circle(radius),
        |shape: Shape| match shape {
            Shape::Circle(radius) => Some(radius),
            _ => None,
        },
    );

    let circle = Shape::Circle(5.0);
    let doubled = circle_prism.modify_option(circle, |radius| radius * 2.0);
    assert!(matches!(doubled, Some(Shape::Circle(r)) if (r - 10.0).abs() < 1e-10));
}

#[test]
fn test_prism_modify_option_no_match() {
    let circle_prism = FunctionPrism::new(
        |shape: &Shape| match shape {
            Shape::Circle(radius) => Some(radius),
            _ => None,
        },
        |radius: f64| Shape::Circle(radius),
        |shape: Shape| match shape {
            Shape::Circle(radius) => Some(radius),
            _ => None,
        },
    );

    let rect = Shape::Rectangle(3.0, 4.0);
    let result = circle_prism.modify_option(rect, |radius| radius * 2.0);
    assert!(result.is_none());
}

#[test]
fn test_prism_modify_or_identity_match() {
    let circle_prism = FunctionPrism::new(
        |shape: &Shape| match shape {
            Shape::Circle(radius) => Some(radius),
            _ => None,
        },
        |radius: f64| Shape::Circle(radius),
        |shape: Shape| match shape {
            Shape::Circle(radius) => Some(radius),
            _ => None,
        },
    );

    let circle = Shape::Circle(5.0);
    let doubled = circle_prism.modify_or_identity(circle, |radius| radius * 2.0);
    assert!(matches!(doubled, Shape::Circle(r) if (r - 10.0).abs() < 1e-10));
}

#[test]
fn test_prism_modify_or_identity_no_match() {
    let circle_prism = FunctionPrism::new(
        |shape: &Shape| match shape {
            Shape::Circle(radius) => Some(radius),
            _ => None,
        },
        |radius: f64| Shape::Circle(radius),
        |shape: Shape| match shape {
            Shape::Circle(radius) => Some(radius),
            _ => None,
        },
    );

    let rect = Shape::Rectangle(3.0, 4.0);
    let result = circle_prism.modify_or_identity(rect, |radius| radius * 2.0);
    // Rectangle should be unchanged
    assert!(
        matches!(result, Shape::Rectangle(w, h) if (w - 3.0).abs() < 1e-10 && (h - 4.0).abs() < 1e-10)
    );
}

// =============================================================================
// Prism composition tests
// =============================================================================

#[test]
fn test_prism_compose_preview() {
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
            Inner::Value(value) => Some(value),
            _ => None,
        },
        |value: i32| Inner::Value(value),
        |inner: Inner| match inner {
            Inner::Value(value) => Some(value),
            _ => None,
        },
    );

    let outer_value = outer_inner_prism.compose(inner_value_prism);

    let data = Outer::Inner(Inner::Value(42));
    assert_eq!(outer_value.preview(&data), Some(&42));

    let empty = Outer::Empty;
    assert_eq!(outer_value.preview(&empty), None);

    let inner_nothing = Outer::Inner(Inner::Nothing);
    assert_eq!(outer_value.preview(&inner_nothing), None);
}

#[test]
fn test_prism_compose_review() {
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
            Inner::Value(value) => Some(value),
            _ => None,
        },
        |value: i32| Inner::Value(value),
        |inner: Inner| match inner {
            Inner::Value(value) => Some(value),
            _ => None,
        },
    );

    let outer_value = outer_inner_prism.compose(inner_value_prism);

    let constructed = outer_value.review(100);
    assert!(matches!(constructed, Outer::Inner(Inner::Value(100))));
}

#[test]
fn test_prism_compose_modify() {
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
            Inner::Value(value) => Some(value),
            _ => None,
        },
        |value: i32| Inner::Value(value),
        |inner: Inner| match inner {
            Inner::Value(value) => Some(value),
            _ => None,
        },
    );

    let outer_value = outer_inner_prism.compose(inner_value_prism);

    let data = Outer::Inner(Inner::Value(42));
    let doubled = outer_value.modify_option(data, |value| value * 2);
    assert!(matches!(doubled, Some(Outer::Inner(Inner::Value(84)))));
}

// =============================================================================
// prism! macro tests
// =============================================================================

#[test]
fn test_prism_macro_single_value_variant() {
    let some_prism = prism!(MyOption<i32>, Some);

    let some_value = MyOption::Some(42);
    assert_eq!(some_prism.preview(&some_value), Some(&42));

    let none_value: MyOption<i32> = MyOption::None;
    assert_eq!(some_prism.preview(&none_value), None);

    let constructed = some_prism.review(100);
    assert_eq!(constructed, MyOption::Some(100));
}

#[test]
fn test_prism_macro_with_result_ok() {
    let ok_prism = prism!(MyResult<i32, String>, Ok);

    let ok_value: MyResult<i32, String> = MyResult::Ok(42);
    assert_eq!(ok_prism.preview(&ok_value), Some(&42));

    let err_value: MyResult<i32, String> = MyResult::Err("error".to_string());
    assert_eq!(ok_prism.preview(&err_value), None);

    let constructed = ok_prism.review(100);
    assert!(matches!(constructed, MyResult::Ok(100)));
}

#[test]
fn test_prism_macro_with_result_err() {
    let err_prism = prism!(MyResult<i32, String>, Err);

    let ok_value: MyResult<i32, String> = MyResult::Ok(42);
    assert_eq!(err_prism.preview(&ok_value), None);

    let err_value: MyResult<i32, String> = MyResult::Err("error".to_string());
    assert_eq!(err_prism.preview(&err_value), Some(&"error".to_string()));

    let constructed = err_prism.review("new error".to_string());
    assert!(matches!(constructed, MyResult::Err(e) if e == "new error"));
}

#[test]
fn test_prism_macro_shape_circle() {
    let circle_prism = prism!(Shape, Circle);

    let circle = Shape::Circle(5.0);
    assert_eq!(circle_prism.preview(&circle), Some(&5.0));

    let rect = Shape::Rectangle(3.0, 4.0);
    assert_eq!(circle_prism.preview(&rect), None);

    let constructed = circle_prism.review(10.0);
    assert!(matches!(constructed, Shape::Circle(r) if (r - 10.0).abs() < 1e-10));
}

// =============================================================================
// to_traversal tests
// =============================================================================

#[test]
fn test_prism_to_traversal_some() {
    let some_prism = prism!(MyOption<i32>, Some);
    let traversal = some_prism.to_traversal();

    let some_value = MyOption::Some(42);
    let all: Vec<&i32> = traversal.get_all(&some_value).collect();
    assert_eq!(all, vec![&42]);
}

#[test]
fn test_prism_to_traversal_none() {
    let some_prism = prism!(MyOption<i32>, Some);
    let traversal = some_prism.to_traversal();

    let none_value: MyOption<i32> = MyOption::None;
    let all: Vec<&i32> = traversal.get_all(&none_value).collect();
    assert!(all.is_empty());
}

// =============================================================================
// Clone and Debug tests
// =============================================================================

#[test]
fn test_function_prism_clone() {
    let some_prism = prism!(MyOption<i32>, Some);
    let cloned = some_prism.clone();

    let some_value = MyOption::Some(42);
    assert_eq!(some_prism.preview(&some_value), cloned.preview(&some_value));
}

#[test]
fn test_function_prism_debug() {
    let some_prism = prism!(MyOption<i32>, Some);
    let debug_str = format!("{:?}", some_prism);
    assert!(debug_str.contains("FunctionPrism"));
}

#[test]
fn test_composed_prism_clone() {
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
            Inner::Value(value) => Some(value),
            _ => None,
        },
        |value: i32| Inner::Value(value),
        |inner: Inner| match inner {
            Inner::Value(value) => Some(value),
            _ => None,
        },
    );

    let outer_value = outer_inner_prism.compose(inner_value_prism);
    let cloned = outer_value.clone();

    let data = Outer::Inner(Inner::Value(42));
    assert_eq!(outer_value.preview(&data), cloned.preview(&data));
}

#[test]
fn test_composed_prism_debug() {
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
            Inner::Value(value) => Some(value),
            _ => None,
        },
        |value: i32| Inner::Value(value),
        |inner: Inner| match inner {
            Inner::Value(value) => Some(value),
            _ => None,
        },
    );

    let outer_value = outer_inner_prism.compose(inner_value_prism);
    let debug_str = format!("{:?}", outer_value);
    assert!(debug_str.contains("ComposedPrism"));
}

// =============================================================================
// rstest parameterized tests
// =============================================================================

#[rstest]
#[case(Shape::Circle(5.0), Some(&5.0))]
#[case(Shape::Circle(0.0), Some(&0.0))]
#[case(Shape::Circle(100.5), Some(&100.5))]
#[case(Shape::Rectangle(3.0, 4.0), None)]
#[case(Shape::Triangle(1.0, 2.0, 3.0), None)]
fn test_circle_prism_preview_parameterized(#[case] input: Shape, #[case] expected: Option<&f64>) {
    let circle_prism = prism!(Shape, Circle);
    assert_eq!(circle_prism.preview(&input), expected);
}

#[rstest]
#[case(5.0)]
#[case(0.0)]
#[case(100.5)]
#[case(-10.0)]
fn test_circle_prism_review_parameterized(#[case] radius: f64) {
    let circle_prism = prism!(Shape, Circle);
    let constructed = circle_prism.review(radius);
    assert!(matches!(constructed, Shape::Circle(r) if (r - radius).abs() < 1e-10));
}

// =============================================================================
// PrismAsTraversal additional tests
// =============================================================================

mod prism_as_traversal_additional_tests {
    use super::*;
    use lambars::optics::Traversal;

    #[test]
    fn test_prism_as_traversal_length_matching() {
        let some_prism = prism!(MyOption<i32>, Some);
        let traversal = some_prism.to_traversal();

        let some_value = MyOption::Some(42);
        assert_eq!(traversal.length(&some_value), 1);
    }

    #[test]
    fn test_prism_as_traversal_length_non_matching() {
        let some_prism = prism!(MyOption<i32>, Some);
        let traversal = some_prism.to_traversal();

        let none_value: MyOption<i32> = MyOption::None;
        assert_eq!(traversal.length(&none_value), 0);
    }

    #[test]
    fn test_prism_as_traversal_set_all_matching() {
        let some_prism = prism!(MyOption<i32>, Some);
        let traversal = some_prism.to_traversal();

        let some_value = MyOption::Some(42);
        let result = traversal.set_all(some_value, 100);
        assert_eq!(result, MyOption::Some(100));
    }

    #[test]
    fn test_prism_as_traversal_set_all_non_matching() {
        let some_prism = prism!(MyOption<i32>, Some);
        let traversal = some_prism.to_traversal();

        let none_value: MyOption<i32> = MyOption::None;
        let result = traversal.set_all(none_value, 100);
        assert_eq!(result, MyOption::None);
    }

    #[test]
    fn test_prism_as_traversal_fold_matching() {
        let some_prism = prism!(MyOption<i32>, Some);
        let traversal = some_prism.to_traversal();

        let some_value = MyOption::Some(10);
        let result = traversal.fold(&some_value, 0, |accumulator, element| accumulator + element);
        assert_eq!(result, 10);
    }

    #[test]
    fn test_prism_as_traversal_fold_non_matching() {
        let some_prism = prism!(MyOption<i32>, Some);
        let traversal = some_prism.to_traversal();

        let none_value: MyOption<i32> = MyOption::None;
        let result = traversal.fold(&none_value, 0, |accumulator, element| accumulator + element);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_prism_as_traversal_for_all() {
        let some_prism = prism!(MyOption<i32>, Some);
        let traversal = some_prism.to_traversal();

        let some_value = MyOption::Some(10);
        assert!(traversal.for_all(&some_value, |x| *x > 5));
        assert!(!traversal.for_all(&some_value, |x| *x > 15));

        let none_value: MyOption<i32> = MyOption::None;
        // Vacuously true for empty traversal
        assert!(traversal.for_all(&none_value, |x| *x > 100));
    }

    #[test]
    fn test_prism_as_traversal_exists() {
        let some_prism = prism!(MyOption<i32>, Some);
        let traversal = some_prism.to_traversal();

        let some_value = MyOption::Some(10);
        assert!(traversal.exists(&some_value, |x| *x > 5));
        assert!(!traversal.exists(&some_value, |x| *x > 15));

        let none_value: MyOption<i32> = MyOption::None;
        assert!(!traversal.exists(&none_value, |x| *x > 0));
    }

    #[test]
    fn test_prism_as_traversal_head_option() {
        let some_prism = prism!(MyOption<i32>, Some);
        let traversal = some_prism.to_traversal();

        let some_value = MyOption::Some(42);
        assert_eq!(traversal.head_option(&some_value), Some(&42));

        let none_value: MyOption<i32> = MyOption::None;
        assert_eq!(traversal.head_option(&none_value), None);
    }
}

// =============================================================================
// ComposedPrism additional tests
// =============================================================================

mod composed_prism_additional_tests {
    use super::*;

    #[test]
    fn test_composed_prism_preview_owned() {
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
                Inner::Value(value) => Some(value),
                _ => None,
            },
            |value: i32| Inner::Value(value),
            |inner: Inner| match inner {
                Inner::Value(value) => Some(value),
                _ => None,
            },
        );

        let outer_value = outer_inner_prism.compose(inner_value_prism);

        let data = Outer::Inner(Inner::Value(42));
        assert_eq!(outer_value.preview_owned(data), Some(42));

        let empty = Outer::Empty;
        assert_eq!(outer_value.preview_owned(empty), None);
    }

    #[test]
    fn test_composed_prism_modify_or_identity() {
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
                Inner::Value(value) => Some(value),
                _ => None,
            },
            |value: i32| Inner::Value(value),
            |inner: Inner| match inner {
                Inner::Value(value) => Some(value),
                _ => None,
            },
        );

        let outer_value = outer_inner_prism.compose(inner_value_prism);

        // Matching case
        let data = Outer::Inner(Inner::Value(42));
        let result = outer_value.modify_or_identity(data, |x| x * 2);
        assert_eq!(result, Outer::Inner(Inner::Value(84)));

        // Non-matching case (outer doesn't match)
        let empty = Outer::Empty;
        let result = outer_value.modify_or_identity(empty.clone(), |x| x * 2);
        assert_eq!(result, Outer::Empty);
    }
}
