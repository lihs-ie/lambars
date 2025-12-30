//! Property-based tests for Lens laws.
//!
//! This module verifies that all Lens implementations satisfy the required laws:
//!
//! - **GetPut Law**: `lens.set(source, lens.get(&source).clone()) == source`
//! - **PutGet Law**: `lens.get(&lens.set(source, value)) == &value`
//! - **PutPut Law**: `lens.set(lens.set(source, v1), v2) == lens.set(source, v2)`
//!
//! Using proptest, we generate random inputs to thoroughly verify these laws
//! across a wide range of values.

use functional_rusty::optics::{FunctionLens, Lens};
use proptest::prelude::*;

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

// =============================================================================
// Lens Laws for Point
// =============================================================================

proptest! {
    /// GetPut Law for Point.x: Getting and setting back yields the original
    #[test]
    fn prop_point_x_get_put_law(x in any::<i32>(), y in any::<i32>()) {
        let x_lens = functional_rusty::lens!(Point, x);
        let point = Point { x, y };
        let value = x_lens.get(&point).clone();
        let result = x_lens.set(point.clone(), value);
        prop_assert_eq!(result, point);
    }

    /// PutGet Law for Point.x: Setting then getting yields the set value
    #[test]
    fn prop_point_x_put_get_law(x in any::<i32>(), y in any::<i32>(), new_value in any::<i32>()) {
        let x_lens = functional_rusty::lens!(Point, x);
        let point = Point { x, y };
        let updated = x_lens.set(point, new_value);
        prop_assert_eq!(*x_lens.get(&updated), new_value);
    }

    /// PutPut Law for Point.x: Two consecutive sets is equivalent to the last set
    #[test]
    fn prop_point_x_put_put_law(
        x in any::<i32>(),
        y in any::<i32>(),
        value1 in any::<i32>(),
        value2 in any::<i32>()
    ) {
        let x_lens = functional_rusty::lens!(Point, x);
        let point = Point { x, y };
        let left = x_lens.set(x_lens.set(point.clone(), value1), value2);
        let right = x_lens.set(point, value2);
        prop_assert_eq!(left, right);
    }

    /// GetPut Law for Point.y
    #[test]
    fn prop_point_y_get_put_law(x in any::<i32>(), y in any::<i32>()) {
        let y_lens = functional_rusty::lens!(Point, y);
        let point = Point { x, y };
        let value = y_lens.get(&point).clone();
        let result = y_lens.set(point.clone(), value);
        prop_assert_eq!(result, point);
    }

    /// PutGet Law for Point.y
    #[test]
    fn prop_point_y_put_get_law(x in any::<i32>(), y in any::<i32>(), new_value in any::<i32>()) {
        let y_lens = functional_rusty::lens!(Point, y);
        let point = Point { x, y };
        let updated = y_lens.set(point, new_value);
        prop_assert_eq!(*y_lens.get(&updated), new_value);
    }

    /// PutPut Law for Point.y
    #[test]
    fn prop_point_y_put_put_law(
        x in any::<i32>(),
        y in any::<i32>(),
        value1 in any::<i32>(),
        value2 in any::<i32>()
    ) {
        let y_lens = functional_rusty::lens!(Point, y);
        let point = Point { x, y };
        let left = y_lens.set(y_lens.set(point.clone(), value1), value2);
        let right = y_lens.set(point, value2);
        prop_assert_eq!(left, right);
    }
}

// =============================================================================
// Lens Laws for Person with String field
// =============================================================================

proptest! {
    /// GetPut Law for Person.name
    #[test]
    fn prop_person_name_get_put_law(name in ".*", age in any::<u32>()) {
        let name_lens = functional_rusty::lens!(Person, name);
        let person = Person { name: name.clone(), age };
        let value = name_lens.get(&person).clone();
        let result = name_lens.set(person.clone(), value);
        prop_assert_eq!(result, person);
    }

    /// PutGet Law for Person.name
    #[test]
    fn prop_person_name_put_get_law(name in ".*", age in any::<u32>(), new_name in ".*") {
        let name_lens = functional_rusty::lens!(Person, name);
        let person = Person { name, age };
        let updated = name_lens.set(person, new_name.clone());
        prop_assert_eq!(name_lens.get(&updated), &new_name);
    }

    /// PutPut Law for Person.name
    #[test]
    fn prop_person_name_put_put_law(
        name in ".*",
        age in any::<u32>(),
        name1 in ".*",
        name2 in ".*"
    ) {
        let name_lens = functional_rusty::lens!(Person, name);
        let person = Person { name, age };
        let left = name_lens.set(name_lens.set(person.clone(), name1), name2.clone());
        let right = name_lens.set(person, name2);
        prop_assert_eq!(left, right);
    }

    /// GetPut Law for Person.age
    #[test]
    fn prop_person_age_get_put_law(name in ".*", age in any::<u32>()) {
        let age_lens = functional_rusty::lens!(Person, age);
        let person = Person { name, age };
        let value = age_lens.get(&person).clone();
        let result = age_lens.set(person.clone(), value);
        prop_assert_eq!(result, person);
    }

    /// PutGet Law for Person.age
    #[test]
    fn prop_person_age_put_get_law(name in ".*", age in any::<u32>(), new_age in any::<u32>()) {
        let age_lens = functional_rusty::lens!(Person, age);
        let person = Person { name, age };
        let updated = age_lens.set(person, new_age);
        prop_assert_eq!(*age_lens.get(&updated), new_age);
    }

    /// PutPut Law for Person.age
    #[test]
    fn prop_person_age_put_put_law(
        name in ".*",
        age in any::<u32>(),
        age1 in any::<u32>(),
        age2 in any::<u32>()
    ) {
        let age_lens = functional_rusty::lens!(Person, age);
        let person = Person { name, age };
        let left = age_lens.set(age_lens.set(person.clone(), age1), age2);
        let right = age_lens.set(person, age2);
        prop_assert_eq!(left, right);
    }
}

// =============================================================================
// Composed Lens Laws
// =============================================================================

proptest! {
    /// GetPut Law for composed lens (PersonWithAddress.address.street)
    #[test]
    fn prop_composed_lens_get_put_law(
        name in "[a-z]{1,10}",
        street in "[a-z]{1,10}",
        city in "[a-z]{1,10}"
    ) {
        let address_lens = functional_rusty::lens!(PersonWithAddress, address);
        let street_lens = functional_rusty::lens!(Address, street);
        let person_street = address_lens.compose(street_lens);

        let person = PersonWithAddress {
            name: name.clone(),
            address: Address {
                street: street.clone(),
                city: city.clone(),
            },
        };

        let value = person_street.get(&person).clone();
        let result = person_street.set(person.clone(), value);
        prop_assert_eq!(result, person);
    }

    /// PutGet Law for composed lens (PersonWithAddress.address.street)
    #[test]
    fn prop_composed_lens_put_get_law(
        name in "[a-z]{1,10}",
        street in "[a-z]{1,10}",
        city in "[a-z]{1,10}",
        new_street in "[a-z]{1,10}"
    ) {
        let address_lens = functional_rusty::lens!(PersonWithAddress, address);
        let street_lens = functional_rusty::lens!(Address, street);
        let person_street = address_lens.compose(street_lens);

        let person = PersonWithAddress {
            name,
            address: Address {
                street,
                city,
            },
        };

        let updated = person_street.set(person, new_street.clone());
        prop_assert_eq!(person_street.get(&updated), &new_street);
    }

    /// PutPut Law for composed lens (PersonWithAddress.address.street)
    #[test]
    fn prop_composed_lens_put_put_law(
        name in "[a-z]{1,10}",
        street in "[a-z]{1,10}",
        city in "[a-z]{1,10}",
        street1 in "[a-z]{1,10}",
        street2 in "[a-z]{1,10}"
    ) {
        let address_lens = functional_rusty::lens!(PersonWithAddress, address);
        let street_lens = functional_rusty::lens!(Address, street);
        let person_street = address_lens.compose(street_lens);

        let person = PersonWithAddress {
            name,
            address: Address {
                street,
                city,
            },
        };

        let left = person_street.set(person_street.set(person.clone(), street1), street2.clone());
        let right = person_street.set(person, street2);
        prop_assert_eq!(left, right);
    }
}

// =============================================================================
// modify preserves laws Tests
// =============================================================================

proptest! {
    /// modify with identity function preserves the value (derived from GetPut)
    #[test]
    fn prop_modify_identity_preserves_value(x in any::<i32>(), y in any::<i32>()) {
        let x_lens = functional_rusty::lens!(Point, x);
        let point = Point { x, y };
        let result = x_lens.modify(point.clone(), |v| v);
        prop_assert_eq!(result, point);
    }

    /// modify composes correctly: modify(f) then modify(g) equals modify(g . f)
    #[test]
    fn prop_modify_composition(x in any::<i32>(), y in any::<i32>()) {
        let x_lens = functional_rusty::lens!(Point, x);
        let point = Point { x, y };

        let function1 = |n: i32| n.wrapping_add(1);
        let function2 = |n: i32| n.wrapping_mul(2);

        let left = x_lens.modify(x_lens.modify(point.clone(), function1), function2);
        let right = x_lens.modify(point, |v| function2(function1(v)));

        prop_assert_eq!(left, right);
    }
}

// =============================================================================
// FunctionLens specific laws Tests
// =============================================================================

proptest! {
    /// FunctionLens satisfies GetPut law
    #[test]
    fn prop_function_lens_get_put_law(x in any::<i32>(), y in any::<i32>()) {
        let x_lens = FunctionLens::new(
            |point: &Point| &point.x,
            |point: Point, x: i32| Point { x, ..point },
        );

        let point = Point { x, y };
        let value = x_lens.get(&point).clone();
        let result = x_lens.set(point.clone(), value);
        prop_assert_eq!(result, point);
    }

    /// FunctionLens satisfies PutGet law
    #[test]
    fn prop_function_lens_put_get_law(x in any::<i32>(), y in any::<i32>(), new_value in any::<i32>()) {
        let x_lens = FunctionLens::new(
            |point: &Point| &point.x,
            |point: Point, x: i32| Point { x, ..point },
        );

        let point = Point { x, y };
        let updated = x_lens.set(point, new_value);
        prop_assert_eq!(*x_lens.get(&updated), new_value);
    }

    /// FunctionLens satisfies PutPut law
    #[test]
    fn prop_function_lens_put_put_law(
        x in any::<i32>(),
        y in any::<i32>(),
        value1 in any::<i32>(),
        value2 in any::<i32>()
    ) {
        let x_lens = FunctionLens::new(
            |point: &Point| &point.x,
            |point: Point, x: i32| Point { x, ..point },
        );

        let point = Point { x, y };
        let left = x_lens.set(x_lens.set(point.clone(), value1), value2);
        let right = x_lens.set(point, value2);
        prop_assert_eq!(left, right);
    }
}

// =============================================================================
// Three-level composition laws Tests
// =============================================================================

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

proptest! {
    /// GetPut Law for three-level composition
    #[test]
    fn prop_three_level_get_put_law(value in any::<i32>()) {
        let lens_1_2 = functional_rusty::lens!(Level1, level2);
        let lens_2_3 = functional_rusty::lens!(Level2, level3);
        let lens_3_value = functional_rusty::lens!(Level3, value);

        let deep_lens = lens_1_2.compose(lens_2_3).compose(lens_3_value);

        let data = Level1 {
            level2: Level2 {
                level3: Level3 { value },
            },
        };

        let got_value = deep_lens.get(&data).clone();
        let result = deep_lens.set(data.clone(), got_value);
        prop_assert_eq!(result, data);
    }

    /// PutGet Law for three-level composition
    #[test]
    fn prop_three_level_put_get_law(original_value in any::<i32>(), new_value in any::<i32>()) {
        let lens_1_2 = functional_rusty::lens!(Level1, level2);
        let lens_2_3 = functional_rusty::lens!(Level2, level3);
        let lens_3_value = functional_rusty::lens!(Level3, value);

        let deep_lens = lens_1_2.compose(lens_2_3).compose(lens_3_value);

        let data = Level1 {
            level2: Level2 {
                level3: Level3 { value: original_value },
            },
        };

        let updated = deep_lens.set(data, new_value);
        prop_assert_eq!(*deep_lens.get(&updated), new_value);
    }

    /// PutPut Law for three-level composition
    #[test]
    fn prop_three_level_put_put_law(
        original_value in any::<i32>(),
        value1 in any::<i32>(),
        value2 in any::<i32>()
    ) {
        let lens_1_2 = functional_rusty::lens!(Level1, level2);
        let lens_2_3 = functional_rusty::lens!(Level2, level3);
        let lens_3_value = functional_rusty::lens!(Level3, value);

        let deep_lens = lens_1_2.compose(lens_2_3).compose(lens_3_value);

        let data = Level1 {
            level2: Level2 {
                level3: Level3 { value: original_value },
            },
        };

        let left = deep_lens.set(deep_lens.set(data.clone(), value1), value2);
        let right = deep_lens.set(data, value2);
        prop_assert_eq!(left, right);
    }
}
