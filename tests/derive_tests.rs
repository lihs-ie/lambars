//! Tests for derive macros (Lenses and Prisms).
//!
//! This module tests the automatically generated optics from derive macros:
//!
//! - `#[derive(Lenses)]`: Generates lens methods for struct fields
//! - `#[derive(Prisms)]`: Generates prism methods for enum variants
//!
//! # Note on Multi-field Variants
//!
//! For enum variants with multiple fields (tuple variants with 2+ fields,
//! or struct variants), the `preview` method is not available because Rust's
//! enum layout doesn't allow returning a reference to a tuple that doesn't
//! exist in memory. Use `preview_owned` and `review` for these variants.

use lambars::optics::{Lens, Prism};
use lambars_derive::{Lenses, Prisms};
use rstest::rstest;

// =============================================================================
// Test Structures for Lenses derive
// =============================================================================

/// Simple struct with basic field types
#[derive(Clone, PartialEq, Debug, Lenses)]
struct Point {
    x: i32,
    y: i32,
}

/// Struct with String field
#[derive(Clone, PartialEq, Debug, Lenses)]
struct Person {
    name: String,
    age: u32,
}

/// Nested struct for composition testing
#[derive(Clone, PartialEq, Debug, Lenses)]
struct Address {
    street: String,
    city: String,
    zip_code: String,
}

/// Struct with nested field
#[derive(Clone, PartialEq, Debug, Lenses)]
struct PersonWithAddress {
    name: String,
    address: Address,
}

/// Struct with generic type parameter
#[derive(Clone, PartialEq, Debug, Lenses)]
struct Container<T> {
    value: T,
    label: String,
}

// =============================================================================
// Test Enums for Prisms derive
// =============================================================================

/// Simple enum with single-field tuple variants (preview works)
#[derive(Clone, PartialEq, Debug, Prisms)]
enum SimpleShape {
    Circle(f64),
    Square(f64),
    Point,
}

/// Enum with multi-field variants (preview_owned only)
#[derive(Clone, PartialEq, Debug, Prisms)]
enum MultiFieldShape {
    Rectangle(f64, f64),
    Triangle(f64, f64, f64),
}

/// Enum with struct variants (preview_owned only)
#[derive(Clone, PartialEq, Debug, Prisms)]
enum Event {
    Click { x: i32, y: i32 },
    KeyPress(char),
    Scroll { delta_x: f64, delta_y: f64 },
    Close,
}

/// Generic enum with single-field variant
#[derive(Clone, PartialEq, Debug, Prisms)]
enum MyOption<T> {
    Some(T),
    None,
}

/// Enum for composition testing
#[derive(Clone, PartialEq, Debug, Prisms)]
enum Outer {
    Inner(Inner),
    Empty,
}

/// Inner enum
#[derive(Clone, PartialEq, Debug, Prisms)]
enum Inner {
    Value(i32),
    Nothing,
}

// =============================================================================
// Lenses derive: Basic Tests
// =============================================================================

/// Test that derived lens can get field value
#[test]
fn test_derived_lens_get() {
    let point = Point { x: 10, y: 20 };
    let x_lens = Point::x_lens();

    assert_eq!(*x_lens.get(&point), 10);
}

/// Test that derived lens can get y field
#[test]
fn test_derived_lens_get_y() {
    let point = Point { x: 10, y: 20 };
    let y_lens = Point::y_lens();

    assert_eq!(*y_lens.get(&point), 20);
}

/// Test that derived lens can set field value
#[test]
fn test_derived_lens_set() {
    let point = Point { x: 10, y: 20 };
    let x_lens = Point::x_lens();

    let updated = x_lens.set(point, 100);
    assert_eq!(updated.x, 100);
    assert_eq!(updated.y, 20); // Other field unchanged
}

/// Test that derived lens supports modify
#[test]
fn test_derived_lens_modify() {
    let point = Point { x: 10, y: 20 };
    let x_lens = Point::x_lens();

    let doubled = x_lens.modify(point, |x| x * 2);
    assert_eq!(doubled.x, 20);
}

/// Test derived lens with String field
#[test]
fn test_derived_lens_string_field() {
    let person = Person {
        name: "Alice".to_string(),
        age: 30,
    };
    let name_lens = Person::name_lens();

    assert_eq!(*name_lens.get(&person), "Alice");

    let updated = name_lens.set(person, "Bob".to_string());
    assert_eq!(updated.name, "Bob");
    assert_eq!(updated.age, 30);
}

/// Test derived lens with age field
#[test]
fn test_derived_lens_age_field() {
    let person = Person {
        name: "Alice".to_string(),
        age: 30,
    };
    let age_lens = Person::age_lens();

    assert_eq!(*age_lens.get(&person), 30);

    let updated = age_lens.set(person, 31);
    assert_eq!(updated.age, 31);
    assert_eq!(updated.name, "Alice");
}

// =============================================================================
// Lenses derive: Composition Tests
// =============================================================================

/// Test lens composition with nested structures
#[test]
fn test_derived_lens_composition() {
    let person = PersonWithAddress {
        name: "Alice".to_string(),
        address: Address {
            street: "Main St".to_string(),
            city: "Tokyo".to_string(),
            zip_code: "100-0001".to_string(),
        },
    };

    let address_lens = PersonWithAddress::address_lens();
    let street_lens = Address::street_lens();
    let person_street_lens = address_lens.compose(street_lens);

    assert_eq!(*person_street_lens.get(&person), "Main St");

    let updated = person_street_lens.set(person, "Oak Ave".to_string());
    assert_eq!(updated.address.street, "Oak Ave");
    assert_eq!(updated.address.city, "Tokyo"); // Other fields unchanged
}

/// Test lens composition with city field
#[test]
fn test_derived_lens_composition_city() {
    let person = PersonWithAddress {
        name: "Alice".to_string(),
        address: Address {
            street: "Main St".to_string(),
            city: "Tokyo".to_string(),
            zip_code: "100-0001".to_string(),
        },
    };

    let address_lens = PersonWithAddress::address_lens();
    let city_lens = Address::city_lens();
    let person_city_lens = address_lens.compose(city_lens);

    assert_eq!(*person_city_lens.get(&person), "Tokyo");

    let updated = person_city_lens.set(person, "Osaka".to_string());
    assert_eq!(updated.address.city, "Osaka");
    assert_eq!(updated.address.street, "Main St");
}

// =============================================================================
// Lenses derive: Generic Type Tests
// =============================================================================

/// Test derived lens with generic struct
#[test]
fn test_derived_lens_generic_struct() {
    let container = Container {
        value: 42,
        label: "answer".to_string(),
    };
    let value_lens = Container::<i32>::value_lens();
    let label_lens = Container::<i32>::label_lens();

    assert_eq!(*value_lens.get(&container), 42);
    assert_eq!(*label_lens.get(&container), "answer");

    let updated = value_lens.set(container, 100);
    assert_eq!(updated.value, 100);
    assert_eq!(updated.label, "answer");
}

// =============================================================================
// Prisms derive: Single-field Tuple Variant Tests (preview works)
// =============================================================================

/// Test derived prism preview for Circle variant
#[test]
fn test_derived_prism_preview_circle() {
    let circle = SimpleShape::Circle(5.0);
    let circle_prism = SimpleShape::circle_prism();

    assert_eq!(circle_prism.preview(&circle), Some(&5.0));
}

/// Test derived prism preview returns None for non-matching variant
#[test]
fn test_derived_prism_preview_no_match() {
    let square = SimpleShape::Square(4.0);
    let circle_prism = SimpleShape::circle_prism();

    assert_eq!(circle_prism.preview(&square), None);
}

/// Test derived prism review constructs variant
#[test]
fn test_derived_prism_review() {
    let circle_prism = SimpleShape::circle_prism();
    let constructed = circle_prism.review(10.0);

    assert!(matches!(constructed, SimpleShape::Circle(r) if (r - 10.0).abs() < 1e-10));
}

/// Test derived prism for Square variant
#[test]
fn test_derived_prism_square() {
    let square = SimpleShape::Square(4.0);
    let square_prism = SimpleShape::square_prism();

    assert_eq!(square_prism.preview(&square), Some(&4.0));

    let constructed = square_prism.review(5.0);
    assert!(matches!(constructed, SimpleShape::Square(s) if (s - 5.0).abs() < 1e-10));
}

/// Test derived prism for unit variant
#[test]
fn test_derived_prism_unit_variant() {
    let point = SimpleShape::Point;
    let point_prism = SimpleShape::point_prism();

    assert_eq!(point_prism.preview(&point), Some(&()));

    let constructed = point_prism.review(());
    assert!(matches!(constructed, SimpleShape::Point));
}

// =============================================================================
// Prisms derive: Multi-field Tuple Variant Tests (preview_owned only)
// =============================================================================

/// Test derived prism for Rectangle variant with preview_owned
#[test]
fn test_derived_prism_rectangle_preview_owned() {
    let rect = MultiFieldShape::Rectangle(3.0, 4.0);
    let rectangle_prism = MultiFieldShape::rectangle_prism();

    let extracted = rectangle_prism.preview_owned(rect);
    assert_eq!(extracted, Some((3.0, 4.0)));
}

/// Test derived prism for Rectangle variant with review
#[test]
fn test_derived_prism_rectangle_review() {
    let rectangle_prism = MultiFieldShape::rectangle_prism();

    let constructed = rectangle_prism.review((5.0, 6.0));
    assert!(
        matches!(constructed, MultiFieldShape::Rectangle(w, h) if (w - 5.0).abs() < 1e-10 && (h - 6.0).abs() < 1e-10)
    );
}

/// Test derived prism for Triangle variant with preview_owned
#[test]
fn test_derived_prism_triangle_preview_owned() {
    let triangle = MultiFieldShape::Triangle(3.0, 4.0, 5.0);
    let triangle_prism = MultiFieldShape::triangle_prism();

    let extracted = triangle_prism.preview_owned(triangle);
    assert_eq!(extracted, Some((3.0, 4.0, 5.0)));
}

/// Test derived prism for Triangle variant with review
#[test]
fn test_derived_prism_triangle_review() {
    let triangle_prism = MultiFieldShape::triangle_prism();

    let constructed = triangle_prism.review((3.0, 4.0, 5.0));
    assert!(matches!(constructed, MultiFieldShape::Triangle(a, b, c)
        if (a - 3.0).abs() < 1e-10 && (b - 4.0).abs() < 1e-10 && (c - 5.0).abs() < 1e-10));
}

// =============================================================================
// Prisms derive: Struct Variant Tests (preview_owned only)
// =============================================================================

/// Test derived prism for struct variant Click with preview_owned
#[test]
fn test_derived_prism_struct_variant_click_preview_owned() {
    let click = Event::Click { x: 10, y: 20 };
    let click_prism = Event::click_prism();

    let extracted = click_prism.preview_owned(click);
    assert_eq!(extracted, Some((10, 20)));
}

/// Test derived prism for struct variant Click with review
#[test]
fn test_derived_prism_struct_variant_click_review() {
    let click_prism = Event::click_prism();

    let constructed = click_prism.review((30, 40));
    assert!(matches!(constructed, Event::Click { x: 30, y: 40 }));
}

/// Test derived prism for struct variant Scroll with preview_owned
#[test]
fn test_derived_prism_struct_variant_scroll_preview_owned() {
    let scroll = Event::Scroll {
        delta_x: 1.5,
        delta_y: 2.5,
    };
    let scroll_prism = Event::scroll_prism();

    let extracted = scroll_prism.preview_owned(scroll);
    assert_eq!(extracted, Some((1.5, 2.5)));
}

/// Test derived prism for KeyPress variant (single-field, preview works)
#[test]
fn test_derived_prism_keypress() {
    let key = Event::KeyPress('a');
    let key_prism = Event::key_press_prism();

    assert_eq!(key_prism.preview(&key), Some(&'a'));
}

/// Test derived prism for unit struct variant Close
#[test]
fn test_derived_prism_close() {
    let close = Event::Close;
    let close_prism = Event::close_prism();

    assert_eq!(close_prism.preview(&close), Some(&()));
}

// =============================================================================
// Prisms derive: Generic Type Tests
// =============================================================================

/// Test derived prism for generic enum Some variant
#[test]
fn test_derived_prism_generic_some() {
    let some_value = MyOption::Some(42);
    let some_prism = MyOption::<i32>::some_prism();

    assert_eq!(some_prism.preview(&some_value), Some(&42));

    let constructed = some_prism.review(100);
    assert_eq!(constructed, MyOption::Some(100));
}

/// Test derived prism for generic enum None variant
#[test]
fn test_derived_prism_generic_none() {
    let none_value: MyOption<i32> = MyOption::None;
    let none_prism = MyOption::<i32>::none_prism();

    assert_eq!(none_prism.preview(&none_value), Some(&()));
}

// =============================================================================
// Prisms derive: Composition Tests
// =============================================================================

/// Test prism composition with nested enums
#[test]
fn test_derived_prism_composition() {
    let data = Outer::Inner(Inner::Value(42));
    let outer_inner_prism = Outer::inner_prism();
    let inner_value_prism = Inner::value_prism();
    let composed = outer_inner_prism.compose(inner_value_prism);

    assert_eq!(composed.preview(&data), Some(&42));
}

/// Test prism composition with nested enums - review
#[test]
fn test_derived_prism_composition_review() {
    let outer_inner_prism = Outer::inner_prism();
    let inner_value_prism = Inner::value_prism();
    let composed = outer_inner_prism.compose(inner_value_prism);

    let constructed = composed.review(100);
    assert!(matches!(constructed, Outer::Inner(Inner::Value(100))));
}

// =============================================================================
// Prisms derive: modify Tests
// =============================================================================

/// Test prism modify_option for matching variant
#[test]
fn test_derived_prism_modify_option_match() {
    let circle = SimpleShape::Circle(5.0);
    let circle_prism = SimpleShape::circle_prism();

    let doubled = circle_prism.modify_option(circle, |r| r * 2.0);
    assert!(matches!(doubled, Some(SimpleShape::Circle(r)) if (r - 10.0).abs() < 1e-10));
}

/// Test prism modify_option for non-matching variant
#[test]
fn test_derived_prism_modify_option_no_match() {
    let square = SimpleShape::Square(4.0);
    let circle_prism = SimpleShape::circle_prism();

    let result = circle_prism.modify_option(square, |r| r * 2.0);
    assert!(result.is_none());
}

/// Test prism modify_or_identity
#[test]
fn test_derived_prism_modify_or_identity() {
    let circle = SimpleShape::Circle(5.0);
    let circle_prism = SimpleShape::circle_prism();

    let doubled = circle_prism.modify_or_identity(circle, |r| r * 2.0);
    assert!(matches!(doubled, SimpleShape::Circle(r) if (r - 10.0).abs() < 1e-10));

    let square = SimpleShape::Square(4.0);
    let unchanged = circle_prism.modify_or_identity(square.clone(), |r| r * 2.0);
    assert_eq!(unchanged, square);
}

// =============================================================================
// Parameterized Tests
// =============================================================================

/// Parameterized test for Point lens get
#[rstest]
#[case(Point { x: 0, y: 0 }, 0, 0)]
#[case(Point { x: 10, y: 20 }, 10, 20)]
#[case(Point { x: -5, y: 100 }, -5, 100)]
#[case(Point { x: i32::MAX, y: i32::MIN }, i32::MAX, i32::MIN)]
fn test_derived_lens_get_parameterized(
    #[case] point: Point,
    #[case] expected_x: i32,
    #[case] expected_y: i32,
) {
    let x_lens = Point::x_lens();
    let y_lens = Point::y_lens();

    assert_eq!(*x_lens.get(&point), expected_x);
    assert_eq!(*y_lens.get(&point), expected_y);
}

/// Parameterized test for Circle prism preview
#[rstest]
#[case(SimpleShape::Circle(5.0), Some(&5.0))]
#[case(SimpleShape::Circle(0.0), Some(&0.0))]
#[case(SimpleShape::Circle(100.5), Some(&100.5))]
#[case(SimpleShape::Square(4.0), None)]
#[case(SimpleShape::Point, None)]
fn test_derived_prism_preview_parameterized(
    #[case] shape: SimpleShape,
    #[case] expected: Option<&f64>,
) {
    let circle_prism = SimpleShape::circle_prism();
    assert_eq!(circle_prism.preview(&shape), expected);
}

// =============================================================================
// Clone and Debug Tests
// =============================================================================

/// Test that derived lenses implement Clone
#[test]
fn test_derived_lens_clone() {
    let x_lens = Point::x_lens();
    let cloned = x_lens.clone();

    let point = Point { x: 10, y: 20 };
    assert_eq!(*x_lens.get(&point), *cloned.get(&point));
}

/// Test that derived prisms implement Clone
#[test]
fn test_derived_prism_clone() {
    let circle_prism = SimpleShape::circle_prism();
    let cloned = circle_prism.clone();

    let circle = SimpleShape::Circle(5.0);
    assert_eq!(circle_prism.preview(&circle), cloned.preview(&circle));
}

// =============================================================================
// Lens Laws Tests (derived lenses should satisfy Lens laws)
// =============================================================================

/// GetPut Law: lens.set(source, lens.get(&source).clone()) == source
#[test]
fn test_derived_lens_getput_law() {
    let point = Point { x: 10, y: 20 };
    let x_lens = Point::x_lens();

    let value = x_lens.get(&point).clone();
    let result = x_lens.set(point.clone(), value);

    assert_eq!(result, point);
}

/// PutGet Law: lens.get(&lens.set(source, value)) == &value
#[test]
fn test_derived_lens_putget_law() {
    let point = Point { x: 10, y: 20 };
    let x_lens = Point::x_lens();
    let new_value = 100;

    let updated = x_lens.set(point, new_value);
    assert_eq!(*x_lens.get(&updated), new_value);
}

/// PutPut Law: lens.set(lens.set(source, v1), v2) == lens.set(source, v2)
#[test]
fn test_derived_lens_putput_law() {
    let point = Point { x: 10, y: 20 };
    let x_lens = Point::x_lens();

    let set_twice = x_lens.set(x_lens.set(point.clone(), 50), 100);
    let set_once = x_lens.set(point, 100);

    assert_eq!(set_twice, set_once);
}

// =============================================================================
// Prism Laws Tests (derived prisms should satisfy Prism laws)
// =============================================================================

/// PreviewReview Law: prism.preview(&prism.review(value)) == Some(&value)
#[test]
fn test_derived_prism_preview_review_law() {
    let circle_prism = SimpleShape::circle_prism();
    let value = 5.0;

    let constructed = circle_prism.review(value);
    let previewed = circle_prism.preview(&constructed);

    assert_eq!(previewed, Some(&value));
}

/// ReviewPreview Law: If preview succeeds, review(preview(source).clone()) == source
#[test]
fn test_derived_prism_review_preview_law() {
    let circle_prism = SimpleShape::circle_prism();
    let circle = SimpleShape::Circle(5.0);

    if let Some(value) = circle_prism.preview(&circle) {
        let reconstructed = circle_prism.review(*value);
        assert_eq!(reconstructed, circle);
    }
}

// =============================================================================
// Multi-field Prism Laws Tests (using preview_owned)
// =============================================================================

/// PreviewReview Law for multi-field: prism.preview_owned(prism.review(value)) == Some(value)
#[test]
fn test_derived_prism_multifield_preview_review_law() {
    let rectangle_prism = MultiFieldShape::rectangle_prism();
    let value = (3.0, 4.0);

    let constructed = rectangle_prism.review(value);
    let previewed = rectangle_prism.preview_owned(constructed);

    assert_eq!(previewed, Some(value));
}

/// Struct variant PreviewReview Law
#[test]
fn test_derived_prism_struct_variant_preview_review_law() {
    let click_prism = Event::click_prism();
    let value = (10, 20);

    let constructed = click_prism.review(value);
    let previewed = click_prism.preview_owned(constructed);

    assert_eq!(previewed, Some(value));
}
