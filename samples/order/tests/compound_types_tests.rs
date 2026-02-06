//! Supplementary tests for the compound_types module
//!
//! Tests for PersonalName, CustomerInfo, and Address implemented in Phase 2:
//! Lens operation tests, Clone/Eq tests, and nested structure access tests.
//! Designed to complement the basic tests in src.

use lambars::optics::Lens;
use order_taking_sample::compound_types::{Address, CustomerInfo, PersonalName};
use order_taking_sample::simple_types::{EmailAddress, String50, UsStateCode, VipStatus, ZipCode};
use rstest::rstest;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

// =============================================================================
// Helper functions
// =============================================================================

/// Creates a PersonalName for testing
fn create_test_personal_name(first: &str, last: &str) -> PersonalName {
    PersonalName::create(first, last).unwrap()
}

/// Creates a CustomerInfo for testing
fn create_test_customer_info(
    first: &str,
    last: &str,
    email: &str,
    vip_status: &str,
) -> CustomerInfo {
    CustomerInfo::create(first, last, email, vip_status).unwrap()
}

/// Creates an Address for testing
fn create_test_address(
    line1: &str,
    line2: &str,
    city: &str,
    zip: &str,
    state: &str,
    country: &str,
) -> Address {
    Address::create(line1, line2, "", "", city, zip, state, country).unwrap()
}

/// Helper function to compute a value's hash
fn calculate_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

// =============================================================================
// PersonalName Lens composition tests
// =============================================================================

mod personal_name_lens_tests {
    use super::*;

    #[rstest]
    fn test_personal_name_lens_get_first_name() {
        let name = create_test_personal_name("John", "Doe");
        let lens = PersonalName::first_name_lens();

        assert_eq!(lens.get(&name).value(), "John");
    }

    #[rstest]
    fn test_personal_name_lens_get_last_name() {
        let name = create_test_personal_name("John", "Doe");
        let lens = PersonalName::last_name_lens();

        assert_eq!(lens.get(&name).value(), "Doe");
    }

    #[rstest]
    fn test_personal_name_lens_set_first_name() {
        let name = create_test_personal_name("John", "Doe");
        let lens = PersonalName::first_name_lens();
        let new_first = String50::create("FirstName", "Jane").unwrap();

        let updated = lens.set(name, new_first);

        assert_eq!(updated.first_name().value(), "Jane");
        assert_eq!(updated.last_name().value(), "Doe"); // Unchanged
    }

    #[rstest]
    fn test_personal_name_lens_set_last_name() {
        let name = create_test_personal_name("John", "Doe");
        let lens = PersonalName::last_name_lens();
        let new_last = String50::create("LastName", "Smith").unwrap();

        let updated = lens.set(name, new_last);

        assert_eq!(updated.first_name().value(), "John"); // Unchanged
        assert_eq!(updated.last_name().value(), "Smith");
    }

    #[rstest]
    fn test_personal_name_lens_modify() {
        let name = create_test_personal_name("john", "doe");
        let lens = PersonalName::first_name_lens();

        let updated = lens.modify(name, |old| {
            // Capitalize the first character
            let capitalized = old
                .value()
                .chars()
                .next()
                .unwrap()
                .to_uppercase()
                .to_string()
                + &old.value()[1..];
            String50::create("FirstName", &capitalized).unwrap()
        });

        assert_eq!(updated.first_name().value(), "John");
    }

    #[rstest]
    fn test_personal_name_lens_immutability() {
        let original = create_test_personal_name("John", "Doe");
        let lens = PersonalName::first_name_lens();
        let new_first = String50::create("FirstName", "Jane").unwrap();

        let updated = lens.set(original.clone(), new_first);

        // The original object has not been modified
        assert_eq!(original.first_name().value(), "John");
        assert_eq!(updated.first_name().value(), "Jane");
    }
}

// =============================================================================
// CustomerInfo nested Lens composition tests
// =============================================================================

mod customer_info_nested_lens_tests {
    use super::*;

    #[rstest]
    fn test_customer_info_nested_lens_get_first_name() {
        let customer = create_test_customer_info("John", "Doe", "john@example.com", "Normal");

        // Lens composition: CustomerInfo -> PersonalName -> String50 (first_name)
        let name_lens = CustomerInfo::name_lens();
        let first_name_lens = PersonalName::first_name_lens();
        let composed = name_lens.compose(first_name_lens);

        assert_eq!(composed.get(&customer).value(), "John");
    }

    #[rstest]
    fn test_customer_info_nested_lens_set_first_name() {
        let customer = create_test_customer_info("John", "Doe", "john@example.com", "Normal");

        let name_lens = CustomerInfo::name_lens();
        let first_name_lens = PersonalName::first_name_lens();
        let composed = name_lens.compose(first_name_lens);

        let new_first = String50::create("FirstName", "Jonathan").unwrap();
        let updated = composed.set(customer, new_first);

        // first_name has been updated
        assert_eq!(updated.name().first_name().value(), "Jonathan");
        // Other fields are unchanged
        assert_eq!(updated.name().last_name().value(), "Doe");
        assert_eq!(updated.email_address().value(), "john@example.com");
    }

    #[rstest]
    fn test_customer_info_nested_lens_get_last_name() {
        let customer = create_test_customer_info("John", "Doe", "john@example.com", "VIP");

        let name_lens = CustomerInfo::name_lens();
        let last_name_lens = PersonalName::last_name_lens();
        let composed = name_lens.compose(last_name_lens);

        assert_eq!(composed.get(&customer).value(), "Doe");
    }

    #[rstest]
    fn test_customer_info_nested_lens_modify_last_name() {
        let customer = create_test_customer_info("John", "Doe", "john@example.com", "Normal");

        let name_lens = CustomerInfo::name_lens();
        let last_name_lens = PersonalName::last_name_lens();
        let composed = name_lens.compose(last_name_lens);

        let updated = composed.modify(customer, |old| {
            String50::create("LastName", &old.value().to_uppercase()).unwrap()
        });

        assert_eq!(updated.name().last_name().value(), "DOE");
        assert_eq!(updated.name().first_name().value(), "John"); // Not modified
    }

    #[rstest]
    fn test_customer_info_direct_lens_operations() {
        let customer = create_test_customer_info("John", "Doe", "john@example.com", "Normal");

        // email_address Lens
        let email_lens = CustomerInfo::email_address_lens();
        assert_eq!(email_lens.get(&customer).value(), "john@example.com");

        let new_email = EmailAddress::create("EmailAddress", "jane@test.org").unwrap();
        let updated = email_lens.set(customer.clone(), new_email);
        assert_eq!(updated.email_address().value(), "jane@test.org");

        // vip_status Lens
        let vip_lens = CustomerInfo::vip_status_lens();
        assert!(matches!(*vip_lens.get(&customer), VipStatus::Normal));

        let updated_vip = vip_lens.set(customer, VipStatus::Vip);
        assert!(matches!(updated_vip.vip_status(), VipStatus::Vip));
    }

    #[rstest]
    fn test_multiple_lens_updates_in_sequence() {
        let customer = create_test_customer_info("John", "Doe", "john@example.com", "Normal");

        // Apply multiple Lens updates consecutively
        let name_lens = CustomerInfo::name_lens();
        let first_name_lens = PersonalName::first_name_lens();
        let last_name_lens = PersonalName::last_name_lens();

        let first_composed = name_lens.clone().compose(first_name_lens);
        let last_composed = name_lens.compose(last_name_lens);

        let new_first = String50::create("FirstName", "Jane").unwrap();
        let updated1 = first_composed.set(customer, new_first);

        let new_last = String50::create("LastName", "Smith").unwrap();
        let updated2 = last_composed.set(updated1, new_last);

        assert_eq!(updated2.name().first_name().value(), "Jane");
        assert_eq!(updated2.name().last_name().value(), "Smith");
    }
}

// =============================================================================
// Address Lens operation tests (including Option fields)
// =============================================================================

mod address_lens_tests {
    use super::*;

    #[rstest]
    fn test_address_lens_optional_field_some() {
        let address =
            create_test_address("123 Main St", "Apt 4B", "New York", "10001", "NY", "USA");

        let lens = Address::address_line2_lens();
        let value = lens.get(&address);

        assert!(value.is_some());
        assert_eq!(value.as_ref().unwrap().value(), "Apt 4B");
    }

    #[rstest]
    fn test_address_lens_optional_field_none() {
        let address = create_test_address("123 Main St", "", "New York", "10001", "NY", "USA");

        let lens = Address::address_line2_lens();
        let value = lens.get(&address);

        assert!(value.is_none());
    }

    #[rstest]
    fn test_address_lens_set_optional_field_some_to_some() {
        let address =
            create_test_address("123 Main St", "Apt 4B", "New York", "10001", "NY", "USA");

        let lens = Address::address_line2_lens();
        let new_value = Some(String50::create("AddressLine2", "Suite 100").unwrap());

        let updated = lens.set(address, new_value);

        assert_eq!(
            updated.address_line2().map(|s| s.value()),
            Some("Suite 100")
        );
    }

    #[rstest]
    fn test_address_lens_set_optional_field_none_to_some() {
        let address = create_test_address("123 Main St", "", "New York", "10001", "NY", "USA");

        let lens = Address::address_line2_lens();
        let new_value = Some(String50::create("AddressLine2", "Apt 5C").unwrap());

        let updated = lens.set(address, new_value);

        assert_eq!(updated.address_line2().map(|s| s.value()), Some("Apt 5C"));
    }

    #[rstest]
    fn test_address_lens_set_optional_field_some_to_none() {
        let address =
            create_test_address("123 Main St", "Apt 4B", "New York", "10001", "NY", "USA");

        let lens = Address::address_line2_lens();

        let updated = lens.set(address, None);

        assert!(updated.address_line2().is_none());
    }

    #[rstest]
    fn test_address_lens_all_required_fields() {
        let address = create_test_address("123 Main St", "", "New York", "10001", "NY", "USA");

        // address_line1
        let line1_lens = Address::address_line1_lens();
        assert_eq!(line1_lens.get(&address).value(), "123 Main St");

        // city
        let city_lens = Address::city_lens();
        assert_eq!(city_lens.get(&address).value(), "New York");

        // zip_code
        let zip_lens = Address::zip_code_lens();
        assert_eq!(zip_lens.get(&address).value(), "10001");

        // state
        let state_lens = Address::state_lens();
        assert_eq!(state_lens.get(&address).value(), "NY");

        // country
        let country_lens = Address::country_lens();
        assert_eq!(country_lens.get(&address).value(), "USA");
    }

    #[rstest]
    fn test_address_lens_update_zip_code() {
        let address = create_test_address("123 Main St", "", "New York", "10001", "NY", "USA");

        let lens = Address::zip_code_lens();
        let new_zip = ZipCode::create("ZipCode", "90210").unwrap();

        let updated = lens.set(address, new_zip);

        assert_eq!(updated.zip_code().value(), "90210");
        assert_eq!(updated.city().value(), "New York"); // Not modified
    }

    #[rstest]
    fn test_address_lens_update_state() {
        let address = create_test_address("123 Main St", "", "New York", "10001", "NY", "USA");

        let lens = Address::state_lens();
        let new_state = UsStateCode::create("State", "CA").unwrap();

        let updated = lens.set(address, new_state);

        assert_eq!(updated.state().value(), "CA");
    }
}

// =============================================================================
// Clone/Eq tests
// =============================================================================

mod clone_eq_tests {
    use super::*;

    #[rstest]
    fn test_personal_name_clone_preserves_equality() {
        let original = create_test_personal_name("John", "Doe");
        let cloned = original.clone();

        assert_eq!(original, cloned);
        // Independent instances (with the same values)
        assert_eq!(original.first_name().value(), cloned.first_name().value());
    }

    #[rstest]
    fn test_customer_info_clone_preserves_equality() {
        let original = create_test_customer_info("John", "Doe", "john@example.com", "VIP");
        let cloned = original.clone();

        assert_eq!(original, cloned);
        assert_eq!(
            original.email_address().value(),
            cloned.email_address().value()
        );
    }

    #[rstest]
    fn test_address_clone_preserves_equality() {
        let original =
            create_test_address("123 Main St", "Apt 4B", "New York", "10001", "NY", "USA");
        let cloned = original.clone();

        assert_eq!(original, cloned);
        assert_eq!(original.city().value(), cloned.city().value());
    }

    #[rstest]
    fn test_personal_name_ne_for_different_values() {
        let name1 = create_test_personal_name("John", "Doe");
        let name2 = create_test_personal_name("Jane", "Doe");
        let name3 = create_test_personal_name("John", "Smith");

        // Different first_name
        assert_ne!(name1, name2);
        // Different last_name
        assert_ne!(name1, name3);
    }

    #[rstest]
    fn test_customer_info_ne_for_different_values() {
        let customer1 = create_test_customer_info("John", "Doe", "john@example.com", "Normal");
        let customer2 = create_test_customer_info("Jane", "Doe", "john@example.com", "Normal");
        let customer3 = create_test_customer_info("John", "Doe", "jane@example.com", "Normal");
        let customer4 = create_test_customer_info("John", "Doe", "john@example.com", "VIP");

        assert_ne!(customer1, customer2); // Different names
        assert_ne!(customer1, customer3); // Different email
        assert_ne!(customer1, customer4); // Different VIP status
    }

    #[rstest]
    fn test_address_ne_for_different_values() {
        let address1 = create_test_address("123 Main St", "", "New York", "10001", "NY", "USA");
        let address2 = create_test_address("456 Oak Ave", "", "New York", "10001", "NY", "USA");
        let address3 = create_test_address("123 Main St", "", "Los Angeles", "90001", "CA", "USA");

        assert_ne!(address1, address2); // Different address_line1
        assert_ne!(address1, address3); // Different city, zip, state
    }
}

// =============================================================================
// Hash consistency tests
// =============================================================================

mod hash_consistency_tests {
    use super::*;

    #[rstest]
    fn test_personal_name_hash_eq_consistency() {
        let name1 = create_test_personal_name("John", "Doe");
        let name2 = create_test_personal_name("John", "Doe");

        // a == b => hash(a) == hash(b)
        assert_eq!(name1, name2);
        assert_eq!(calculate_hash(&name1), calculate_hash(&name2));
    }

    #[rstest]
    fn test_personal_name_hash_map_usage() {
        let name1 = create_test_personal_name("John", "Doe");
        let name2 = create_test_personal_name("John", "Doe");

        let mut map: HashMap<PersonalName, String> = HashMap::new();
        map.insert(name1.clone(), "Employee".to_string());

        // A different instance with the same value works as a key
        assert_eq!(map.get(&name2), Some(&"Employee".to_string()));
    }

    #[rstest]
    fn test_address_hash_eq_consistency() {
        let addr1 = create_test_address("123 Main St", "Apt 4B", "New York", "10001", "NY", "USA");
        let addr2 = create_test_address("123 Main St", "Apt 4B", "New York", "10001", "NY", "USA");

        assert_eq!(addr1, addr2);
        assert_eq!(calculate_hash(&addr1), calculate_hash(&addr2));
    }

    #[rstest]
    fn test_address_hash_map_usage() {
        let addr1 = create_test_address("123 Main St", "", "New York", "10001", "NY", "USA");
        let addr2 = create_test_address("123 Main St", "", "New York", "10001", "NY", "USA");

        let mut map: HashMap<Address, i32> = HashMap::new();
        map.insert(addr1.clone(), 100);

        assert_eq!(map.get(&addr2), Some(&100));
    }
}

// =============================================================================
// Boundary value tests
// =============================================================================

mod boundary_tests {
    use super::*;

    #[rstest]
    fn test_personal_name_with_max_length_strings() {
        let max_name = "a".repeat(50);
        let result = PersonalName::create(&max_name, &max_name);

        assert!(result.is_ok());
        let name = result.unwrap();
        assert_eq!(name.first_name().value().len(), 50);
        assert_eq!(name.last_name().value().len(), 50);
    }

    #[rstest]
    fn test_customer_info_with_max_length_strings() {
        let max_name = "a".repeat(50);
        let max_email = format!("{}@{}.com", "a".repeat(20), "b".repeat(20));
        let result = CustomerInfo::create(&max_name, &max_name, &max_email, "VIP");

        assert!(result.is_ok());
    }

    #[rstest]
    fn test_address_with_all_optional_fields() {
        let address = Address::create(
            "123 Main St",
            "Line 2",
            "Line 3",
            "Line 4",
            "New York",
            "10001",
            "NY",
            "USA",
        )
        .unwrap();

        assert!(address.address_line2().is_some());
        assert!(address.address_line3().is_some());
        assert!(address.address_line4().is_some());
    }

    #[rstest]
    fn test_address_with_no_optional_fields() {
        let address =
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();

        assert!(address.address_line2().is_none());
        assert!(address.address_line3().is_none());
        assert!(address.address_line4().is_none());
    }
}

// =============================================================================
// Error handling tests
// =============================================================================

mod error_handling_tests {
    use super::*;

    #[rstest]
    fn test_personal_name_error_propagation() {
        // first_name is empty
        let result = PersonalName::create("", "Doe");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field_name, "FirstName");

        // last_name is empty
        let result = PersonalName::create("John", "");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field_name, "LastName");
    }

    #[rstest]
    fn test_customer_info_error_propagation() {
        // Verify errors for each field propagate correctly
        let test_cases = vec![
            (("", "Doe", "john@example.com", "Normal"), "FirstName"),
            (("John", "", "john@example.com", "Normal"), "LastName"),
            (("John", "Doe", "invalid", "Normal"), "EmailAddress"),
            (("John", "Doe", "john@example.com", "Premium"), "VipStatus"),
        ];

        for ((first, last, email, vip), expected_field) in test_cases {
            let result = CustomerInfo::create(first, last, email, vip);
            assert!(result.is_err());
            assert_eq!(
                result.unwrap_err().field_name,
                expected_field,
                "Expected error for field {expected_field}"
            );
        }
    }

    #[rstest]
    fn test_address_error_propagation() {
        // address_line1 is empty
        let result = Address::create("", "", "", "", "New York", "10001", "NY", "USA");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field_name, "AddressLine1");

        // city is empty
        let result = Address::create("123 Main St", "", "", "", "", "10001", "NY", "USA");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field_name, "City");

        // invalid zip_code
        let result = Address::create("123 Main St", "", "", "", "New York", "1234", "NY", "USA");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field_name, "ZipCode");

        // invalid state
        let result = Address::create("123 Main St", "", "", "", "New York", "10001", "XX", "USA");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field_name, "State");

        // country is empty
        let result = Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field_name, "Country");
    }
}
