//! Unit tests for Lens optics.
//!
//! This module contains comprehensive tests for the Lens trait and its implementations:
//!
//! - [`Lens`] trait: Basic lens operations (get, set, modify, modify_ref)
//! - [`FunctionLens`]: Lens implementation using getter and setter functions
//! - [`ComposedLens`]: Composition of two lenses
//! - [`lens!`] macro: Convenient lens creation for struct fields

use lambars::optics::{FunctionLens, Lens};
use rstest::rstest;

// =============================================================================
// Test Structures
// =============================================================================

#[derive(Clone, PartialEq, Debug)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Clone, PartialEq, Debug)]
struct Person {
    name: String,
    age: u32,
}

#[derive(Clone, PartialEq, Debug)]
struct Address {
    street: String,
    city: String,
}

#[derive(Clone, PartialEq, Debug)]
struct PersonWithAddress {
    name: String,
    address: Address,
}

#[derive(Clone, PartialEq, Debug)]
struct Outer {
    inner: Inner,
}

#[derive(Clone, PartialEq, Debug)]
struct Inner {
    value: i32,
}

// =============================================================================
// FunctionLens Basic Tests
// =============================================================================

/// Test that FunctionLens can get a field value
#[test]
fn test_function_lens_get() {
    let x_lens = FunctionLens::new(
        |point: &Point| &point.x,
        |point: Point, x: i32| Point { x, ..point },
    );

    let point = Point { x: 10, y: 20 };
    assert_eq!(*x_lens.get(&point), 10);
}

/// Test that FunctionLens can get another field value
#[test]
fn test_function_lens_get_y() {
    let y_lens = FunctionLens::new(
        |point: &Point| &point.y,
        |point: Point, y: i32| Point { y, ..point },
    );

    let point = Point { x: 10, y: 20 };
    assert_eq!(*y_lens.get(&point), 20);
}

/// Test that FunctionLens can set a field value
#[test]
fn test_function_lens_set() {
    let x_lens = FunctionLens::new(
        |point: &Point| &point.x,
        |point: Point, x: i32| Point { x, ..point },
    );

    let point = Point { x: 10, y: 20 };
    let updated = x_lens.set(point, 100);
    assert_eq!(updated.x, 100);
    assert_eq!(updated.y, 20); // Other fields remain unchanged
}

/// Test that set does not modify the original value (immutability)
#[test]
fn test_function_lens_does_not_modify_original() {
    let x_lens = FunctionLens::new(
        |point: &Point| &point.x,
        |point: Point, x: i32| Point { x, ..point },
    );

    let point = Point { x: 10, y: 20 };
    let _updated = x_lens.set(point.clone(), 100);
    // Original is not modified
    assert_eq!(point.x, 10);
}

/// Test FunctionLens with String field
#[test]
fn test_function_lens_string_field() {
    let name_lens = FunctionLens::new(
        |person: &Person| &person.name,
        |person: Person, name: String| Person { name, ..person },
    );

    let person = Person {
        name: "Alice".to_string(),
        age: 30,
    };
    assert_eq!(*name_lens.get(&person), "Alice");

    let updated = name_lens.set(person, "Bob".to_string());
    assert_eq!(updated.name, "Bob");
    assert_eq!(updated.age, 30);
}

// =============================================================================
// modify and modify_ref Tests
// =============================================================================

/// Test that modify applies a function to the focused value
#[test]
fn test_lens_modify() {
    let x_lens = FunctionLens::new(
        |point: &Point| &point.x,
        |point: Point, x: i32| Point { x, ..point },
    );

    let point = Point { x: 10, y: 20 };
    let doubled = x_lens.modify(point, |x| x * 2);
    assert_eq!(doubled.x, 20);
    assert_eq!(doubled.y, 20);
}

/// Test modify with more complex transformation
#[test]
fn test_lens_modify_complex() {
    let x_lens = FunctionLens::new(
        |point: &Point| &point.x,
        |point: Point, x: i32| Point { x, ..point },
    );

    let point = Point { x: 5, y: 20 };
    let transformed = x_lens.modify(point, |x| x * x + 1);
    assert_eq!(transformed.x, 26); // 5*5 + 1 = 26
}

/// Test modify_ref applies a function using reference to the focused value
#[test]
fn test_lens_modify_ref() {
    let name_lens = FunctionLens::new(
        |person: &Person| &person.name,
        |person: Person, name: String| Person { name, ..person },
    );

    let person = Person {
        name: "alice".to_string(),
        age: 30,
    };
    let upper = name_lens.modify_ref(person, |name| name.to_uppercase());
    assert_eq!(upper.name, "ALICE");
}

/// Test modify_ref with String length calculation
#[test]
fn test_lens_modify_ref_string_len() {
    #[derive(Clone, PartialEq, Debug)]
    struct Container {
        text: String,
        length: usize,
    }

    let length_lens = FunctionLens::new(
        |container: &Container| &container.length,
        |container: Container, length: usize| Container {
            length,
            ..container
        },
    );

    let _text_lens = FunctionLens::new(
        |container: &Container| &container.text,
        |container: Container, text: String| Container { text, ..container },
    );

    let container = Container {
        text: "hello".to_string(),
        length: 0,
    };

    // Update length based on text
    let updated = length_lens.modify_ref(container.clone(), |_| container.text.len());
    assert_eq!(updated.length, 5);
}

// =============================================================================
// ComposedLens Tests
// =============================================================================

/// Test lens composition for getting nested fields
#[test]
fn test_lens_compose_get() {
    let address_lens = FunctionLens::new(
        |person: &PersonWithAddress| &person.address,
        |person: PersonWithAddress, address: Address| PersonWithAddress { address, ..person },
    );

    let street_lens = FunctionLens::new(
        |address: &Address| &address.street,
        |address: Address, street: String| Address { street, ..address },
    );

    let person_street = address_lens.compose(street_lens);

    let person = PersonWithAddress {
        name: "Alice".to_string(),
        address: Address {
            street: "Main St".to_string(),
            city: "Tokyo".to_string(),
        },
    };

    assert_eq!(*person_street.get(&person), "Main St");
}

/// Test lens composition for setting nested fields
#[test]
fn test_lens_compose_set() {
    let address_lens = FunctionLens::new(
        |person: &PersonWithAddress| &person.address,
        |person: PersonWithAddress, address: Address| PersonWithAddress { address, ..person },
    );

    let street_lens = FunctionLens::new(
        |address: &Address| &address.street,
        |address: Address, street: String| Address { street, ..address },
    );

    let person_street = address_lens.compose(street_lens);

    let person = PersonWithAddress {
        name: "Alice".to_string(),
        address: Address {
            street: "Main St".to_string(),
            city: "Tokyo".to_string(),
        },
    };

    let updated = person_street.set(person, "Oak Ave".to_string());
    assert_eq!(updated.address.street, "Oak Ave");
    assert_eq!(updated.address.city, "Tokyo"); // Other fields unchanged
    assert_eq!(updated.name, "Alice");
}

/// Test lens composition for modifying nested fields
#[test]
fn test_lens_compose_modify() {
    let address_lens = FunctionLens::new(
        |person: &PersonWithAddress| &person.address,
        |person: PersonWithAddress, address: Address| PersonWithAddress { address, ..person },
    );

    let street_lens = FunctionLens::new(
        |address: &Address| &address.street,
        |address: Address, street: String| Address { street, ..address },
    );

    let person_street = address_lens.compose(street_lens);

    let person = PersonWithAddress {
        name: "Alice".to_string(),
        address: Address {
            street: "main st".to_string(),
            city: "Tokyo".to_string(),
        },
    };

    let upper = person_street.modify(person, |street| street.to_uppercase());
    assert_eq!(upper.address.street, "MAIN ST");
}

/// Test three-level lens composition
#[test]
fn test_lens_compose_three_levels() {
    #[derive(Clone, PartialEq, Debug)]
    struct Level1 {
        level2: Level2,
    }

    #[derive(Clone, PartialEq, Debug)]
    struct Level2 {
        level3: Level3,
    }

    #[derive(Clone, PartialEq, Debug)]
    struct Level3 {
        value: i32,
    }

    let lens_1_2 = FunctionLens::new(
        |l1: &Level1| &l1.level2,
        |_l1: Level1, level2: Level2| Level1 { level2 },
    );

    let lens_2_3 = FunctionLens::new(
        |l2: &Level2| &l2.level3,
        |_l2: Level2, level3: Level3| Level2 { level3 },
    );

    let lens_3_value = FunctionLens::new(
        |l3: &Level3| &l3.value,
        |_l3: Level3, value: i32| Level3 { value },
    );

    let deep_lens = lens_1_2.compose(lens_2_3).compose(lens_3_value);

    let data = Level1 {
        level2: Level2 {
            level3: Level3 { value: 42 },
        },
    };

    assert_eq!(*deep_lens.get(&data), 42);

    let updated = deep_lens.set(data, 100);
    assert_eq!(updated.level2.level3.value, 100);
}

// =============================================================================
// lens! Macro Tests
// =============================================================================

/// Test lens! macro for simple field access
#[test]
fn test_lens_macro_simple() {
    let x_lens = lambars::lens!(Point, x);
    let y_lens = lambars::lens!(Point, y);

    let point = Point { x: 10, y: 20 };
    assert_eq!(*x_lens.get(&point), 10);
    assert_eq!(*y_lens.get(&point), 20);
}

/// Test lens! macro for set operation
#[test]
fn test_lens_macro_set() {
    let x_lens = lambars::lens!(Point, x);
    let point = Point { x: 10, y: 20 };
    let updated = x_lens.set(point, 100);
    assert_eq!(updated, Point { x: 100, y: 20 });
}

/// Test lens! macro for modify operation
#[test]
fn test_lens_macro_modify() {
    let x_lens = lambars::lens!(Point, x);
    let point = Point { x: 10, y: 20 };
    let doubled = x_lens.modify(point, |x| x * 2);
    assert_eq!(doubled.x, 20);
}

/// Test lens! macro with String field
#[test]
fn test_lens_macro_string_field() {
    let name_lens = lambars::lens!(Person, name);

    let person = Person {
        name: "Alice".to_string(),
        age: 30,
    };

    assert_eq!(*name_lens.get(&person), "Alice");

    let updated = name_lens.set(person, "Bob".to_string());
    assert_eq!(updated.name, "Bob");
    assert_eq!(updated.age, 30);
}

/// Test lens! macro composition for nested structures
#[test]
fn test_lens_macro_nested_composition() {
    let inner_lens = lambars::lens!(Outer, inner);
    let value_lens = lambars::lens!(Inner, value);
    let outer_value = inner_lens.compose(value_lens);

    let data = Outer {
        inner: Inner { value: 42 },
    };
    assert_eq!(*outer_value.get(&data), 42);

    let updated = outer_value.set(data, 100);
    assert_eq!(updated.inner.value, 100);
}

/// Test lens! macro with address composition
#[test]
fn test_lens_macro_address_composition() {
    let address_lens = lambars::lens!(PersonWithAddress, address);
    let street_lens = lambars::lens!(Address, street);
    let city_lens = lambars::lens!(Address, city);

    let person_street = address_lens.clone().compose(street_lens);
    let person_city = address_lens.compose(city_lens);

    let person = PersonWithAddress {
        name: "Alice".to_string(),
        address: Address {
            street: "Main St".to_string(),
            city: "Tokyo".to_string(),
        },
    };

    assert_eq!(*person_street.get(&person), "Main St");
    assert_eq!(*person_city.get(&person), "Tokyo");
}

// =============================================================================
// LensAsTraversal Tests
// =============================================================================

/// Test conversion of Lens to Traversal
#[test]
fn test_lens_to_traversal() {
    let x_lens = lambars::lens!(Point, x);
    let traversal = x_lens.to_traversal();

    let point = Point { x: 10, y: 20 };

    // get_all should return iterator with single element
    let all: Vec<&i32> = traversal.get_all(&point).collect();
    assert_eq!(all, vec![&10]);
}

// =============================================================================
// Edge Cases and Error Handling
// =============================================================================

/// Test lens with zero value
#[test]
fn test_lens_with_zero() {
    let x_lens = lambars::lens!(Point, x);
    let point = Point { x: 0, y: 0 };
    assert_eq!(*x_lens.get(&point), 0);

    let updated = x_lens.set(point, 0);
    assert_eq!(updated.x, 0);
}

/// Test lens with negative values
#[test]
fn test_lens_with_negative() {
    let x_lens = lambars::lens!(Point, x);
    let point = Point { x: -10, y: -20 };
    assert_eq!(*x_lens.get(&point), -10);

    let updated = x_lens.set(point, -100);
    assert_eq!(updated.x, -100);
}

/// Test lens with empty string
#[test]
fn test_lens_with_empty_string() {
    let name_lens = lambars::lens!(Person, name);
    let person = Person {
        name: String::new(),
        age: 30,
    };
    assert_eq!(*name_lens.get(&person), "");

    let updated = name_lens.set(person, String::new());
    assert_eq!(updated.name, "");
}

/// Test lens with i32::MAX and i32::MIN
#[rstest]
#[case(i32::MAX)]
#[case(i32::MIN)]
fn test_lens_with_extreme_values(#[case] value: i32) {
    let x_lens = lambars::lens!(Point, x);
    let point = Point { x: value, y: 0 };
    assert_eq!(*x_lens.get(&point), value);

    let updated = x_lens.set(point, value);
    assert_eq!(updated.x, value);
}

// =============================================================================
// Clone and Copy semantics Tests
// =============================================================================

/// Test that lens can be cloned and both work independently
#[test]
fn test_lens_clone() {
    let x_lens = lambars::lens!(Point, x);
    let x_lens_clone = x_lens.clone();

    let point = Point { x: 10, y: 20 };
    assert_eq!(*x_lens.get(&point), *x_lens_clone.get(&point));
}

/// Test that composed lens can be cloned
#[test]
fn test_composed_lens_clone() {
    let inner_lens = lambars::lens!(Outer, inner);
    let value_lens = lambars::lens!(Inner, value);
    let composed = inner_lens.compose(value_lens);
    let composed_clone = composed.clone();

    let data = Outer {
        inner: Inner { value: 42 },
    };

    assert_eq!(*composed.get(&data), *composed_clone.get(&data));
}

// =============================================================================
// Debug Trait Tests
// =============================================================================

#[test]
fn test_function_lens_debug() {
    let x_lens = lambars::lens!(Point, x);
    let debug_str = format!("{:?}", x_lens);
    assert!(debug_str.contains("FunctionLens"));
}

#[test]
fn test_composed_lens_debug() {
    let inner_lens = lambars::lens!(Outer, inner);
    let value_lens = lambars::lens!(Inner, value);
    let composed = inner_lens.compose(value_lens);

    let debug_str = format!("{:?}", composed);
    assert!(debug_str.contains("ComposedLens"));
}

// =============================================================================
// LensAsTraversal Additional Tests
// =============================================================================

mod lens_as_traversal_additional_tests {
    use super::*;
    use lambars::optics::Traversal;

    #[test]
    fn test_lens_as_traversal_set_all() {
        let x_lens = lambars::lens!(Point, x);
        let traversal = x_lens.to_traversal();

        let point = Point { x: 10, y: 20 };
        let updated = traversal.set_all(point, 100);
        assert_eq!(updated.x, 100);
        assert_eq!(updated.y, 20);
    }

    #[test]
    fn test_lens_as_traversal_fold() {
        let x_lens = lambars::lens!(Point, x);
        let traversal = x_lens.to_traversal();

        let point = Point { x: 10, y: 20 };
        let sum = traversal.fold(&point, 0, |accumulator, element| accumulator + element);
        assert_eq!(sum, 10);
    }

    #[test]
    fn test_lens_as_traversal_for_all() {
        let x_lens = lambars::lens!(Point, x);
        let traversal = x_lens.to_traversal();

        let point = Point { x: 10, y: 20 };
        assert!(traversal.for_all(&point, |x| *x > 5));
        assert!(!traversal.for_all(&point, |x| *x > 15));
    }

    #[test]
    fn test_lens_as_traversal_exists() {
        let x_lens = lambars::lens!(Point, x);
        let traversal = x_lens.to_traversal();

        let point = Point { x: 10, y: 20 };
        assert!(traversal.exists(&point, |x| *x > 5));
        assert!(!traversal.exists(&point, |x| *x > 15));
    }

    #[test]
    fn test_lens_as_traversal_head_option() {
        let x_lens = lambars::lens!(Point, x);
        let traversal = x_lens.to_traversal();

        let point = Point { x: 10, y: 20 };
        assert_eq!(traversal.head_option(&point), Some(&10));
    }
}

// =============================================================================
// ComposedLens Additional Tests
// =============================================================================

mod composed_lens_additional_tests {
    use super::*;

    #[test]
    fn test_composed_lens_modify_ref() {
        let address_lens = FunctionLens::new(
            |person: &PersonWithAddress| &person.address,
            |person: PersonWithAddress, address: Address| PersonWithAddress { address, ..person },
        );

        let street_lens = FunctionLens::new(
            |address: &Address| &address.street,
            |address: Address, street: String| Address { street, ..address },
        );

        let person_street = address_lens.compose(street_lens);

        let person = PersonWithAddress {
            name: "Alice".to_string(),
            address: Address {
                street: "main st".to_string(),
                city: "Tokyo".to_string(),
            },
        };

        let upper = person_street.modify_ref(person, |street| street.to_uppercase());
        assert_eq!(upper.address.street, "MAIN ST");
    }
}
