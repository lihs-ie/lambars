//! Unit tests for Optional optics.
//!
//! Optional is a Lens + Prism composition result.
//! It represents an optic that may or may not be able to access a value.

use functional_rusty::optics::{FunctionPrism, Lens, LensComposeExtension, Optional};
use functional_rusty::{lens, prism};
use rstest::rstest;

// =============================================================================
// Test data types
// =============================================================================

#[derive(Clone, PartialEq, Debug)]
enum MyOption<T> {
    Some(T),
    None,
}

#[derive(Clone, PartialEq, Debug)]
struct Container {
    maybe_value: MyOption<i32>,
}

#[derive(Clone, PartialEq, Debug)]
struct NestedContainer {
    container: Container,
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

#[derive(Clone, PartialEq, Debug)]
struct OuterWrapper {
    outer: Outer,
}

// =============================================================================
// Optional trait existence tests
// =============================================================================

#[test]
fn test_optional_trait_exists() {
    fn assert_optional<O: Optional<Container, i32>>(_optional: O) {}

    let container_lens = lens!(Container, maybe_value);
    let some_prism = prism!(MyOption<i32>, Some);
    let optional = container_lens.compose_prism(some_prism);

    assert_optional(optional);
}

// =============================================================================
// LensPrismComposition basic operations
// =============================================================================

#[test]
fn test_lens_prism_composition_get_option_some() {
    let container_lens = lens!(Container, maybe_value);
    let some_prism = prism!(MyOption<i32>, Some);
    let optional = container_lens.compose_prism(some_prism);

    let container = Container {
        maybe_value: MyOption::Some(42),
    };
    assert_eq!(optional.get_option(&container), Some(&42));
}

#[test]
fn test_lens_prism_composition_get_option_none() {
    let container_lens = lens!(Container, maybe_value);
    let some_prism = prism!(MyOption<i32>, Some);
    let optional = container_lens.compose_prism(some_prism);

    let container = Container {
        maybe_value: MyOption::None,
    };
    assert_eq!(optional.get_option(&container), None);
}

#[test]
fn test_lens_prism_composition_set_some() {
    let container_lens = lens!(Container, maybe_value);
    let some_prism = prism!(MyOption<i32>, Some);
    let optional = container_lens.compose_prism(some_prism);

    let container = Container {
        maybe_value: MyOption::Some(42),
    };
    let updated = optional.set(container, 100);
    assert_eq!(updated.maybe_value, MyOption::Some(100));
}

#[test]
fn test_lens_prism_composition_set_none() {
    let container_lens = lens!(Container, maybe_value);
    let some_prism = prism!(MyOption<i32>, Some);
    let optional = container_lens.compose_prism(some_prism);

    let container = Container {
        maybe_value: MyOption::None,
    };
    // Setting on None should still create Some
    let updated = optional.set(container, 100);
    assert_eq!(updated.maybe_value, MyOption::Some(100));
}

// =============================================================================
// modify_option tests
// =============================================================================

#[test]
fn test_optional_modify_option_some() {
    let container_lens = lens!(Container, maybe_value);
    let some_prism = prism!(MyOption<i32>, Some);
    let optional = container_lens.compose_prism(some_prism);

    let container = Container {
        maybe_value: MyOption::Some(42),
    };
    let result = optional.modify_option(container, |value| value * 2);
    assert!(result.is_some());
    assert_eq!(result.unwrap().maybe_value, MyOption::Some(84));
}

#[test]
fn test_optional_modify_option_none() {
    let container_lens = lens!(Container, maybe_value);
    let some_prism = prism!(MyOption<i32>, Some);
    let optional = container_lens.compose_prism(some_prism);

    let container = Container {
        maybe_value: MyOption::None,
    };
    let result = optional.modify_option(container, |value| value * 2);
    assert!(result.is_none());
}

// =============================================================================
// is_present tests
// =============================================================================

#[test]
fn test_optional_is_present_some() {
    let container_lens = lens!(Container, maybe_value);
    let some_prism = prism!(MyOption<i32>, Some);
    let optional = container_lens.compose_prism(some_prism);

    let container = Container {
        maybe_value: MyOption::Some(42),
    };
    assert!(optional.is_present(&container));
}

#[test]
fn test_optional_is_present_none() {
    let container_lens = lens!(Container, maybe_value);
    let some_prism = prism!(MyOption<i32>, Some);
    let optional = container_lens.compose_prism(some_prism);

    let container = Container {
        maybe_value: MyOption::None,
    };
    assert!(!optional.is_present(&container));
}

// =============================================================================
// ComposedOptional tests
// =============================================================================

#[test]
fn test_composed_optional_get_option() {
    // Create a second optional (for testing composition, we'll simulate one)
    let nested_lens = lens!(NestedContainer, container);
    let nested_some_prism = prism!(MyOption<i32>, Some);
    let container_inner_lens = lens!(Container, maybe_value);

    // Compose nested_lens with the container optional
    let nested_optional = nested_lens.compose(container_inner_lens);
    // Now compose with prism
    let full_optional = nested_optional.compose_prism(nested_some_prism);

    let nested = NestedContainer {
        container: Container {
            maybe_value: MyOption::Some(42),
        },
    };

    assert_eq!(full_optional.get_option(&nested), Some(&42));
}

#[test]
fn test_optional_compose_optional() {
    // Create first Optional: OuterWrapper.outer -> Outer.Inner
    let outer_lens = lens!(OuterWrapper, outer);
    let inner_prism = FunctionPrism::new(
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
    let optional1 = outer_lens.compose_prism(inner_prism);

    // Create second Optional: Inner.Value
    let value_prism = FunctionPrism::new(
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

    // Compose the two optionals
    let composed = optional1.compose(value_prism);

    // Test with full path present
    let wrapper = OuterWrapper {
        outer: Outer::Inner(Inner::Value(42)),
    };
    assert_eq!(composed.get_option(&wrapper), Some(&42));

    // Test with inner missing
    let wrapper_nothing = OuterWrapper {
        outer: Outer::Inner(Inner::Nothing),
    };
    assert_eq!(composed.get_option(&wrapper_nothing), None);

    // Test with outer missing
    let wrapper_empty = OuterWrapper {
        outer: Outer::Empty,
    };
    assert_eq!(composed.get_option(&wrapper_empty), None);
}

// =============================================================================
// Clone and Debug tests
// =============================================================================

#[test]
fn test_lens_prism_composition_clone() {
    let container_lens = lens!(Container, maybe_value);
    let some_prism = prism!(MyOption<i32>, Some);
    let optional = container_lens.compose_prism(some_prism);
    let cloned = optional.clone();

    let container = Container {
        maybe_value: MyOption::Some(42),
    };
    assert_eq!(
        optional.get_option(&container),
        cloned.get_option(&container)
    );
}

#[test]
fn test_lens_prism_composition_debug() {
    let container_lens = lens!(Container, maybe_value);
    let some_prism = prism!(MyOption<i32>, Some);
    let optional = container_lens.compose_prism(some_prism);
    let debug_str = format!("{:?}", optional);
    assert!(debug_str.contains("LensPrismComposition"));
}

// =============================================================================
// rstest parameterized tests
// =============================================================================

#[rstest]
#[case(MyOption::Some(10), Some(&10))]
#[case(MyOption::Some(0), Some(&0))]
#[case(MyOption::Some(-42), Some(&-42))]
#[case(MyOption::None, None)]
fn test_optional_get_option_parameterized(
    #[case] maybe: MyOption<i32>,
    #[case] expected: Option<&i32>,
) {
    let container_lens = lens!(Container, maybe_value);
    let some_prism = prism!(MyOption<i32>, Some);
    let optional = container_lens.compose_prism(some_prism);

    let container = Container { maybe_value: maybe };
    assert_eq!(optional.get_option(&container), expected);
}

#[rstest]
#[case(MyOption::Some(42), true)]
#[case(MyOption::None, false)]
fn test_optional_is_present_parameterized(#[case] maybe: MyOption<i32>, #[case] expected: bool) {
    let container_lens = lens!(Container, maybe_value);
    let some_prism = prism!(MyOption<i32>, Some);
    let optional = container_lens.compose_prism(some_prism);

    let container = Container { maybe_value: maybe };
    assert_eq!(optional.is_present(&container), expected);
}

// =============================================================================
// ComposedOptional additional tests
// =============================================================================

mod composed_optional_additional_tests {
    use super::*;

    #[test]
    fn test_composed_optional_debug() {
        let outer_lens = lens!(OuterWrapper, outer);
        let inner_prism = FunctionPrism::new(
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
        let optional1 = outer_lens.compose_prism(inner_prism);

        let value_prism = FunctionPrism::new(
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

        let composed = optional1.compose(value_prism);
        let debug_str = format!("{:?}", composed);
        assert!(debug_str.contains("ComposedOptional"));
    }

    #[test]
    fn test_composed_optional_clone() {
        let outer_lens = lens!(OuterWrapper, outer);
        let inner_prism = FunctionPrism::new(
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
        let optional1 = outer_lens.compose_prism(inner_prism);

        let value_prism = FunctionPrism::new(
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

        let composed = optional1.compose(value_prism);
        let cloned = composed.clone();

        let wrapper = OuterWrapper {
            outer: Outer::Inner(Inner::Value(42)),
        };
        assert_eq!(composed.get_option(&wrapper), cloned.get_option(&wrapper));
    }

    #[test]
    fn test_composed_optional_set() {
        let outer_lens = lens!(OuterWrapper, outer);
        let inner_prism = FunctionPrism::new(
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
        let optional1 = outer_lens.compose_prism(inner_prism);

        let value_prism = FunctionPrism::new(
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

        let composed = optional1.compose(value_prism);

        let wrapper = OuterWrapper {
            outer: Outer::Inner(Inner::Value(42)),
        };
        let updated = composed.set(wrapper, 100);
        assert_eq!(updated.outer, Outer::Inner(Inner::Value(100)));
    }

    #[test]
    fn test_composed_optional_modify_option() {
        let outer_lens = lens!(OuterWrapper, outer);
        let inner_prism = FunctionPrism::new(
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
        let optional1 = outer_lens.compose_prism(inner_prism);

        let value_prism = FunctionPrism::new(
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

        let composed = optional1.compose(value_prism);

        // Test with value present
        let wrapper = OuterWrapper {
            outer: Outer::Inner(Inner::Value(10)),
        };
        let result = composed.modify_option(wrapper, |v| v * 2);
        assert!(result.is_some());
        assert_eq!(result.unwrap().outer, Outer::Inner(Inner::Value(20)));

        // Test with value absent
        let wrapper_empty = OuterWrapper {
            outer: Outer::Empty,
        };
        let result = composed.modify_option(wrapper_empty, |v| v * 2);
        assert!(result.is_none());
    }

    #[test]
    fn test_composed_optional_is_present() {
        let outer_lens = lens!(OuterWrapper, outer);
        let inner_prism = FunctionPrism::new(
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
        let optional1 = outer_lens.compose_prism(inner_prism);

        let value_prism = FunctionPrism::new(
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

        let composed = optional1.compose(value_prism);

        let wrapper = OuterWrapper {
            outer: Outer::Inner(Inner::Value(42)),
        };
        assert!(composed.is_present(&wrapper));

        let wrapper_nothing = OuterWrapper {
            outer: Outer::Inner(Inner::Nothing),
        };
        assert!(!composed.is_present(&wrapper_nothing));

        let wrapper_empty = OuterWrapper {
            outer: Outer::Empty,
        };
        assert!(!composed.is_present(&wrapper_empty));
    }
}
