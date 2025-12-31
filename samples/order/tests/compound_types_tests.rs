//! compound_types モジュールの補完テスト
//!
//! Phase 2 で実装された PersonalName, CustomerInfo, Address の
//! Lens 操作テスト、Clone/Eq テスト、およびネスト構造へのアクセステストを行う。
//! src 内の基本テストを補完する形で設計。

use functional_rusty::optics::Lens;
use order_taking_sample::compound_types::{Address, CustomerInfo, PersonalName};
use order_taking_sample::simple_types::{EmailAddress, String50, UsStateCode, VipStatus, ZipCode};
use rstest::rstest;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

// =============================================================================
// ヘルパー関数
// =============================================================================

/// テスト用の PersonalName を生成する
fn create_test_personal_name(first: &str, last: &str) -> PersonalName {
    PersonalName::create(first, last).unwrap()
}

/// テスト用の CustomerInfo を生成する
fn create_test_customer_info(
    first: &str,
    last: &str,
    email: &str,
    vip_status: &str,
) -> CustomerInfo {
    CustomerInfo::create(first, last, email, vip_status).unwrap()
}

/// テスト用の Address を生成する
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

/// 値のハッシュを計算するヘルパー関数
fn calculate_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

// =============================================================================
// PersonalName Lens 合成テスト
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
        assert_eq!(updated.last_name().value(), "Doe"); // 変更されていない
    }

    #[rstest]
    fn test_personal_name_lens_set_last_name() {
        let name = create_test_personal_name("John", "Doe");
        let lens = PersonalName::last_name_lens();
        let new_last = String50::create("LastName", "Smith").unwrap();

        let updated = lens.set(name, new_last);

        assert_eq!(updated.first_name().value(), "John"); // 変更されていない
        assert_eq!(updated.last_name().value(), "Smith");
    }

    #[rstest]
    fn test_personal_name_lens_modify() {
        let name = create_test_personal_name("john", "doe");
        let lens = PersonalName::first_name_lens();

        let updated = lens.modify(name, |old| {
            // 最初の文字を大文字に
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

        // 元のオブジェクトは変更されていない
        assert_eq!(original.first_name().value(), "John");
        assert_eq!(updated.first_name().value(), "Jane");
    }
}

// =============================================================================
// CustomerInfo ネスト Lens 合成テスト
// =============================================================================

mod customer_info_nested_lens_tests {
    use super::*;

    #[rstest]
    fn test_customer_info_nested_lens_get_first_name() {
        let customer = create_test_customer_info("John", "Doe", "john@example.com", "Normal");

        // Lens 合成: CustomerInfo -> PersonalName -> String50 (first_name)
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

        // first_name が更新されている
        assert_eq!(updated.name().first_name().value(), "Jonathan");
        // 他のフィールドは変更されていない
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
        assert_eq!(updated.name().first_name().value(), "John"); // 変更なし
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

        // 複数の Lens 更新を連続して適用
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
// Address Lens 操作テスト（Option フィールド含む）
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
        assert_eq!(updated.city().value(), "New York"); // 変更なし
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
// Clone/Eq テスト
// =============================================================================

mod clone_eq_tests {
    use super::*;

    #[rstest]
    fn test_personal_name_clone_preserves_equality() {
        let original = create_test_personal_name("John", "Doe");
        let cloned = original.clone();

        assert_eq!(original, cloned);
        // 独立したインスタンスである（同じ値を持つ）
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

        // 異なる first_name
        assert_ne!(name1, name2);
        // 異なる last_name
        assert_ne!(name1, name3);
    }

    #[rstest]
    fn test_customer_info_ne_for_different_values() {
        let customer1 = create_test_customer_info("John", "Doe", "john@example.com", "Normal");
        let customer2 = create_test_customer_info("Jane", "Doe", "john@example.com", "Normal");
        let customer3 = create_test_customer_info("John", "Doe", "jane@example.com", "Normal");
        let customer4 = create_test_customer_info("John", "Doe", "john@example.com", "VIP");

        assert_ne!(customer1, customer2); // 異なる名前
        assert_ne!(customer1, customer3); // 異なるメール
        assert_ne!(customer1, customer4); // 異なる VIP ステータス
    }

    #[rstest]
    fn test_address_ne_for_different_values() {
        let address1 = create_test_address("123 Main St", "", "New York", "10001", "NY", "USA");
        let address2 = create_test_address("456 Oak Ave", "", "New York", "10001", "NY", "USA");
        let address3 = create_test_address("123 Main St", "", "Los Angeles", "90001", "CA", "USA");

        assert_ne!(address1, address2); // 異なる address_line1
        assert_ne!(address1, address3); // 異なる city, zip, state
    }
}

// =============================================================================
// Hash 一貫性テスト
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

        // 同じ値の別インスタンスでキーとして機能する
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
// 境界値テスト
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
// エラーハンドリングテスト
// =============================================================================

mod error_handling_tests {
    use super::*;

    #[rstest]
    fn test_personal_name_error_propagation() {
        // first_name が空
        let result = PersonalName::create("", "Doe");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field_name, "FirstName");

        // last_name が空
        let result = PersonalName::create("John", "");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field_name, "LastName");
    }

    #[rstest]
    fn test_customer_info_error_propagation() {
        // 各フィールドのエラーが適切に伝播することを確認
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
        // address_line1 が空
        let result = Address::create("", "", "", "", "New York", "10001", "NY", "USA");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field_name, "AddressLine1");

        // city が空
        let result = Address::create("123 Main St", "", "", "", "", "10001", "NY", "USA");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field_name, "City");

        // 無効な zip_code
        let result = Address::create("123 Main St", "", "", "", "New York", "1234", "NY", "USA");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field_name, "ZipCode");

        // 無効な state
        let result = Address::create("123 Main St", "", "", "", "New York", "10001", "XX", "USA");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field_name, "State");

        // country が空
        let result = Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field_name, "Country");
    }
}
