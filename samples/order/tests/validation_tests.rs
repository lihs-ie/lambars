//! validation モジュールの補完テスト
//!
//! バリデーションロジックのエッジケース、エラー伝播、
//! バリデーション失敗時の早期リターン動作を検証する。

use order_taking_sample::simple_types::ProductCode;
use order_taking_sample::workflow::validation::{
    create_pricing_method, to_address, to_checked_address, to_customer_info, to_order_id,
    to_order_line_id, to_order_quantity, to_product_code, to_validated_order_line, validate_order,
};
use order_taking_sample::workflow::{
    AddressValidationError, CheckedAddress, PlaceOrderError, UnvalidatedAddress,
    UnvalidatedCustomerInfo, UnvalidatedOrder, UnvalidatedOrderLine,
};
use rstest::rstest;
use rust_decimal::Decimal;
use std::str::FromStr;

// =============================================================================
// テストデータファクトリ
// =============================================================================

fn valid_customer_info() -> UnvalidatedCustomerInfo {
    UnvalidatedCustomerInfo::new(
        "John".to_string(),
        "Doe".to_string(),
        "john@example.com".to_string(),
        "Normal".to_string(),
    )
}

fn valid_address() -> UnvalidatedAddress {
    UnvalidatedAddress::new(
        "123 Main St".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "New York".to_string(),
        "10001".to_string(),
        "NY".to_string(),
        "USA".to_string(),
    )
}

fn valid_order_line() -> UnvalidatedOrderLine {
    UnvalidatedOrderLine::new(
        "line-001".to_string(),
        "W1234".to_string(),
        Decimal::from(10),
    )
}

fn mock_product_exists() -> impl Fn(&ProductCode) -> bool {
    |_: &ProductCode| true
}

fn mock_address_valid()
-> impl Fn(&UnvalidatedAddress) -> Result<CheckedAddress, AddressValidationError> {
    |addr: &UnvalidatedAddress| Ok(CheckedAddress::new(addr.clone()))
}

fn mock_address_not_found()
-> impl Fn(&UnvalidatedAddress) -> Result<CheckedAddress, AddressValidationError> {
    |_: &UnvalidatedAddress| Err(AddressValidationError::AddressNotFound)
}

fn mock_address_invalid_format()
-> impl Fn(&UnvalidatedAddress) -> Result<CheckedAddress, AddressValidationError> {
    |_: &UnvalidatedAddress| Err(AddressValidationError::InvalidFormat)
}

// =============================================================================
// to_order_id 境界値テスト
// =============================================================================

mod to_order_id_edge_cases {
    use super::*;

    #[rstest]
    fn test_order_id_single_char() {
        let result = to_order_id("A");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), "A");
    }

    #[rstest]
    fn test_order_id_exact_max_length() {
        let id = "a".repeat(50);
        let result = to_order_id(&id);
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_order_id_one_over_max_length() {
        let id = "a".repeat(51);
        let result = to_order_id(&id);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_order_id_with_special_chars() {
        let result = to_order_id("ORD-2024-001_ABC");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), "ORD-2024-001_ABC");
    }

    #[rstest]
    fn test_order_id_with_unicode() {
        let result = to_order_id("order-001");
        assert!(result.is_ok());
    }
}

// =============================================================================
// to_order_line_id 境界値テスト
// =============================================================================

mod to_order_line_id_edge_cases {
    use super::*;

    #[rstest]
    fn test_order_line_id_single_char() {
        let result = to_order_line_id("1");
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_order_line_id_numeric_only() {
        let result = to_order_line_id("12345");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), "12345");
    }

    #[rstest]
    fn test_order_line_id_with_dashes() {
        let result = to_order_line_id("line-001-a");
        assert!(result.is_ok());
    }
}

// =============================================================================
// to_customer_info エッジケーステスト
// =============================================================================

mod to_customer_info_edge_cases {
    use super::*;

    #[rstest]
    fn test_customer_info_max_length_names() {
        let max_name = "a".repeat(50);
        let unvalidated = UnvalidatedCustomerInfo::new(
            max_name.clone(),
            max_name,
            "test@example.com".to_string(),
            "Normal".to_string(),
        );
        let result = to_customer_info(&unvalidated);
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_customer_info_vip_lowercase_accepted() {
        // 小文字の "vip" も受け入れられる
        let unvalidated = UnvalidatedCustomerInfo::new(
            "John".to_string(),
            "Doe".to_string(),
            "john@example.com".to_string(),
            "vip".to_string(),
        );
        let result = to_customer_info(&unvalidated);
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_customer_info_invalid_vip_status_rejected() {
        // "Vip" や "NORMAL" などの混在ケースは無効
        let unvalidated = UnvalidatedCustomerInfo::new(
            "John".to_string(),
            "Doe".to_string(),
            "john@example.com".to_string(),
            "Vip".to_string(), // "Vip" は無効（"vip" か "VIP" のみ）
        );
        let result = to_customer_info(&unvalidated);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field_name, "VipStatus");
    }

    #[rstest]
    fn test_customer_info_email_with_subdomain() {
        let unvalidated = UnvalidatedCustomerInfo::new(
            "John".to_string(),
            "Doe".to_string(),
            "john@mail.example.com".to_string(),
            "Normal".to_string(),
        );
        let result = to_customer_info(&unvalidated);
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_customer_info_error_field_priority() {
        // first_name が空の場合、first_name のエラーが最初に返される
        let unvalidated = UnvalidatedCustomerInfo::new(
            "".to_string(),
            "".to_string(),
            "invalid".to_string(),
            "InvalidStatus".to_string(),
        );
        let result = to_customer_info(&unvalidated);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field_name, "FirstName");
    }
}

// =============================================================================
// to_address エッジケーステスト
// =============================================================================

mod to_address_edge_cases {
    use super::*;

    #[rstest]
    fn test_address_all_optional_fields() {
        let unvalidated = UnvalidatedAddress::new(
            "123 Main St".to_string(),
            "Apt 4B".to_string(),
            "Building A".to_string(),
            "Floor 5".to_string(),
            "New York".to_string(),
            "10001".to_string(),
            "NY".to_string(),
            "USA".to_string(),
        );
        let checked = CheckedAddress::new(unvalidated);
        let result = to_address(&checked);
        assert!(result.is_ok());
        let address = result.unwrap();
        assert!(address.address_line2().is_some());
        assert!(address.address_line3().is_some());
        assert!(address.address_line4().is_some());
    }

    #[rstest]
    fn test_address_max_length_fields() {
        let max_str = "a".repeat(50);
        let unvalidated = UnvalidatedAddress::new(
            max_str.clone(),
            max_str.clone(),
            max_str.clone(),
            max_str.clone(),
            max_str.clone(),
            "12345".to_string(),
            "NY".to_string(),
            max_str,
        );
        let checked = CheckedAddress::new(unvalidated);
        let result = to_address(&checked);
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_address_dc_state_code() {
        let unvalidated = UnvalidatedAddress::new(
            "1600 Pennsylvania Ave".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "Washington".to_string(),
            "20500".to_string(),
            "DC".to_string(),
            "USA".to_string(),
        );
        let checked = CheckedAddress::new(unvalidated);
        let result = to_address(&checked);
        assert!(result.is_ok());
    }
}

// =============================================================================
// to_checked_address エラーマッピングテスト
// =============================================================================

mod to_checked_address_error_mapping {
    use super::*;

    #[rstest]
    fn test_checked_address_error_address_not_found_message() {
        let address = valid_address();
        let check_address = mock_address_not_found();
        let result = to_checked_address(&check_address, &address);
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Address");
        assert_eq!(error.message, "Address not found");
    }

    #[rstest]
    fn test_checked_address_error_invalid_format_message() {
        let address = valid_address();
        let check_address = mock_address_invalid_format();
        let result = to_checked_address(&check_address, &address);
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Address");
        assert_eq!(error.message, "Address has bad format");
    }
}

// =============================================================================
// to_product_code 境界値テスト
// =============================================================================

mod to_product_code_edge_cases {
    use super::*;

    #[rstest]
    fn test_product_code_widget_min_valid() {
        let check_product = mock_product_exists();
        let result = to_product_code(&check_product, "W0000");
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_product_code_widget_max_valid() {
        let check_product = mock_product_exists();
        let result = to_product_code(&check_product, "W9999");
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_product_code_gizmo_min_valid() {
        let check_product = mock_product_exists();
        let result = to_product_code(&check_product, "G000");
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_product_code_gizmo_max_valid() {
        let check_product = mock_product_exists();
        let result = to_product_code(&check_product, "G999");
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_product_code_widget_too_few_digits() {
        let check_product = mock_product_exists();
        let result = to_product_code(&check_product, "W123");
        assert!(result.is_err());
    }

    #[rstest]
    fn test_product_code_gizmo_too_many_digits() {
        let check_product = mock_product_exists();
        let result = to_product_code(&check_product, "G1234");
        assert!(result.is_err());
    }

    #[rstest]
    fn test_product_code_lowercase_prefix() {
        let check_product = mock_product_exists();
        let result = to_product_code(&check_product, "w1234");
        assert!(result.is_err());
    }
}

// =============================================================================
// to_order_quantity 境界値テスト
// =============================================================================

mod to_order_quantity_boundary_tests {
    use super::*;

    #[rstest]
    fn test_unit_quantity_min_valid() {
        let widget_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let result = to_order_quantity(&widget_code, Decimal::from(1));
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_unit_quantity_max_valid() {
        let widget_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let result = to_order_quantity(&widget_code, Decimal::from(1000));
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_unit_quantity_below_min() {
        let widget_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let result = to_order_quantity(&widget_code, Decimal::from(0));
        assert!(result.is_err());
    }

    #[rstest]
    fn test_unit_quantity_above_max() {
        let widget_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let result = to_order_quantity(&widget_code, Decimal::from(1001));
        assert!(result.is_err());
    }

    #[rstest]
    fn test_unit_quantity_decimal_truncated() {
        let widget_code = ProductCode::create("ProductCode", "W1234").unwrap();
        // Widget の場合、小数は切り捨てられる（u32 に変換）
        let result = to_order_quantity(&widget_code, Decimal::from_str("10.5").unwrap());
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_kilogram_quantity_min_valid() {
        let gizmo_code = ProductCode::create("ProductCode", "G123").unwrap();
        let result = to_order_quantity(&gizmo_code, Decimal::from_str("0.05").unwrap());
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_kilogram_quantity_max_valid() {
        let gizmo_code = ProductCode::create("ProductCode", "G123").unwrap();
        let result = to_order_quantity(&gizmo_code, Decimal::from_str("100.00").unwrap());
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_kilogram_quantity_below_min() {
        let gizmo_code = ProductCode::create("ProductCode", "G123").unwrap();
        let result = to_order_quantity(&gizmo_code, Decimal::from_str("0.04").unwrap());
        assert!(result.is_err());
    }

    #[rstest]
    fn test_kilogram_quantity_above_max() {
        let gizmo_code = ProductCode::create("ProductCode", "G123").unwrap();
        let result = to_order_quantity(&gizmo_code, Decimal::from_str("100.01").unwrap());
        assert!(result.is_err());
    }
}

// =============================================================================
// create_pricing_method テスト
// =============================================================================

mod create_pricing_method_tests {
    use super::*;

    #[rstest]
    fn test_empty_string_returns_standard() {
        let result = create_pricing_method("");
        assert!(result.is_standard());
        assert!(result.promotion_code().is_none());
    }

    #[rstest]
    fn test_whitespace_is_promotion() {
        // 空白のみでも Promotion として扱われる
        let result = create_pricing_method("   ");
        assert!(result.is_promotion());
    }

    #[rstest]
    fn test_any_string_is_promotion() {
        let result = create_pricing_method("ANY_CODE_HERE");
        assert!(result.is_promotion());
        assert_eq!(result.promotion_code().unwrap().value(), "ANY_CODE_HERE");
    }

    #[rstest]
    fn test_long_promotion_code() {
        let long_code = "A".repeat(100);
        let result = create_pricing_method(&long_code);
        assert!(result.is_promotion());
    }
}

// =============================================================================
// to_validated_order_line エラー順序テスト
// =============================================================================

mod to_validated_order_line_error_order {
    use super::*;

    #[rstest]
    fn test_error_order_line_id_first() {
        // order_line_id が空で、product_code も無効な場合
        let unvalidated = UnvalidatedOrderLine::new(
            "".to_string(),
            "X999".to_string(), // 無効な形式
            Decimal::from(0),   // 無効な数量
        );
        let check_product = mock_product_exists();
        let result = to_validated_order_line(&check_product, &unvalidated);
        assert!(result.is_err());
        // OrderLineId のエラーが最初に返される
        assert_eq!(result.unwrap_err().field_name, "OrderLineId");
    }

    #[rstest]
    fn test_error_product_code_second() {
        // order_line_id は有効、product_code が無効
        let unvalidated = UnvalidatedOrderLine::new(
            "line-001".to_string(),
            "X999".to_string(), // 無効な形式
            Decimal::from(10),
        );
        let check_product = mock_product_exists();
        let result = to_validated_order_line(&check_product, &unvalidated);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field_name, "ProductCode");
    }

    #[rstest]
    fn test_error_quantity_third() {
        // order_line_id, product_code は有効、quantity が無効
        let unvalidated = UnvalidatedOrderLine::new(
            "line-001".to_string(),
            "W1234".to_string(),
            Decimal::from(0), // 無効な数量
        );
        let check_product = mock_product_exists();
        let result = to_validated_order_line(&check_product, &unvalidated);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field_name, "Quantity");
    }
}

// =============================================================================
// validate_order 複合エラーテスト
// =============================================================================

mod validate_order_complex_scenarios {
    use super::*;

    #[rstest]
    fn test_validate_order_different_shipping_billing_addresses() {
        let shipping = UnvalidatedAddress::new(
            "123 Main St".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "New York".to_string(),
            "10001".to_string(),
            "NY".to_string(),
            "USA".to_string(),
        );
        let billing = UnvalidatedAddress::new(
            "456 Oak Ave".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "Los Angeles".to_string(),
            "90001".to_string(),
            "CA".to_string(),
            "USA".to_string(),
        );
        let order = UnvalidatedOrder::new(
            "order-001".to_string(),
            valid_customer_info(),
            shipping,
            billing,
            vec![valid_order_line()],
            "".to_string(),
        );
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();
        let result = validate_order(&check_product, &check_address, &order);
        assert!(result.is_ok());
        let validated = result.unwrap();
        assert_eq!(validated.shipping_address().city().value(), "New York");
        assert_eq!(validated.billing_address().city().value(), "Los Angeles");
    }

    #[rstest]
    fn test_validate_order_many_order_lines() {
        let lines: Vec<UnvalidatedOrderLine> = (0..100)
            .map(|i| {
                UnvalidatedOrderLine::new(
                    format!("line-{i:03}"),
                    "W1234".to_string(),
                    Decimal::from(i % 1000 + 1),
                )
            })
            .collect();
        let order = UnvalidatedOrder::new(
            "order-001".to_string(),
            valid_customer_info(),
            valid_address(),
            valid_address(),
            lines,
            "".to_string(),
        );
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();
        let result = validate_order(&check_product, &check_address, &order);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().lines().len(), 100);
    }

    #[rstest]
    fn test_validate_order_mixed_product_types() {
        let lines = vec![
            UnvalidatedOrderLine::new(
                "line-001".to_string(),
                "W1234".to_string(),
                Decimal::from(10),
            ),
            UnvalidatedOrderLine::new(
                "line-002".to_string(),
                "G123".to_string(),
                Decimal::from_str("5.5").unwrap(),
            ),
            UnvalidatedOrderLine::new(
                "line-003".to_string(),
                "W5678".to_string(),
                Decimal::from(20),
            ),
            UnvalidatedOrderLine::new(
                "line-004".to_string(),
                "G456".to_string(),
                Decimal::from_str("10.25").unwrap(),
            ),
        ];
        let order = UnvalidatedOrder::new(
            "order-001".to_string(),
            valid_customer_info(),
            valid_address(),
            valid_address(),
            lines,
            "SUMMER2024".to_string(),
        );
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();
        let result = validate_order(&check_product, &check_address, &order);
        assert!(result.is_ok());
        let validated = result.unwrap();
        assert_eq!(validated.lines().len(), 4);
        assert!(validated.pricing_method().is_promotion());
    }

    #[rstest]
    fn test_validate_order_fails_on_second_line() {
        let lines = vec![
            UnvalidatedOrderLine::new(
                "line-001".to_string(),
                "W1234".to_string(),
                Decimal::from(10),
            ),
            UnvalidatedOrderLine::new("".to_string(), "W5678".to_string(), Decimal::from(20)), // 無効
            UnvalidatedOrderLine::new("line-003".to_string(), "G123".to_string(), Decimal::from(5)),
        ];
        let order = UnvalidatedOrder::new(
            "order-001".to_string(),
            valid_customer_info(),
            valid_address(),
            valid_address(),
            lines,
            "".to_string(),
        );
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();
        let result = validate_order(&check_product, &check_address, &order);
        assert!(result.is_err());
    }
}

// =============================================================================
// PlaceOrderError 変換テスト
// =============================================================================

mod place_order_error_conversion {
    use super::*;

    #[rstest]
    fn test_validation_error_to_place_order_error() {
        let order = UnvalidatedOrder::new(
            "".to_string(), // 無効
            valid_customer_info(),
            valid_address(),
            valid_address(),
            vec![valid_order_line()],
            "".to_string(),
        );
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();
        let result = validate_order(&check_product, &check_address, &order);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.is_validation());
        // PlaceOrderError::Validation(_) のパターンマッチ
        if let PlaceOrderError::Validation(validation_error) = error {
            assert_eq!(validation_error.field_name, "OrderId");
        } else {
            panic!("Expected PlaceOrderError::Validation");
        }
    }
}
