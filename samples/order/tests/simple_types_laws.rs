//! Smart Constructor 法則の proptest による検証
//!
//! Smart Constructor パターンで構築された型が以下の性質を満たすことを検証する:
//! 1. 等価性法則: value() で取得した値は生成時の値と等しい
//! 2. 不変条件: Ok で生成された値は常に制約を満たす
//! 3. べき等性: 同じ入力からは同じ結果が得られる

use order_taking_sample::simple_types::{
    BillingAmount, EmailAddress, KilogramQuantity, OrderId, OrderLineId, Price, ProductCode,
    String50, UnitQuantity, UsStateCode, ZipCode,
};
use proptest::prelude::*;
use rust_decimal::Decimal;
use std::str::FromStr;

// =============================================================================
// 戦略（Strategy）定義
// =============================================================================

/// 有効な String50 用の文字列戦略
fn valid_string50_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-zA-Z0-9 ]{1,50}")
        .unwrap()
        .prop_filter("non-empty", |s| !s.is_empty())
}

/// 無効な String50 用の文字列戦略（空または51文字以上）
fn invalid_string50_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::new()),
        proptest::string::string_regex("[a-zA-Z0-9]{51,100}").unwrap()
    ]
}

/// 有効な EmailAddress 用の文字列戦略
fn valid_email_strategy() -> impl Strategy<Value = String> {
    (
        proptest::string::string_regex("[a-zA-Z0-9._%+-]{1,20}").unwrap(),
        proptest::string::string_regex("[a-zA-Z0-9.-]{1,20}").unwrap(),
        proptest::string::string_regex("[a-zA-Z]{2,5}").unwrap(),
    )
        .prop_map(|(local, domain, tld)| format!("{local}@{domain}.{tld}"))
}

/// 無効な EmailAddress 用の文字列戦略（@ なし）
fn invalid_email_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-zA-Z0-9]{1,30}").unwrap()
}

/// 有効な ZipCode 用の文字列戦略（5桁の数字）
fn valid_zip_code_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[0-9]{5}").unwrap()
}

/// 無効な ZipCode 用の文字列戦略
fn invalid_zip_code_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        proptest::string::string_regex("[0-9]{1,4}").unwrap(),
        proptest::string::string_regex("[0-9]{6,10}").unwrap(),
        proptest::string::string_regex("[a-zA-Z]{5}").unwrap()
    ]
}

/// 有効な UsStateCode 用の戦略
fn valid_state_code_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("AL".to_string()),
        Just("AK".to_string()),
        Just("AZ".to_string()),
        Just("CA".to_string()),
        Just("NY".to_string()),
        Just("TX".to_string()),
        Just("FL".to_string()),
        Just("DC".to_string())
    ]
}

/// 無効な UsStateCode 用の戦略
fn invalid_state_code_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("XX".to_string()),
        Just("ZZ".to_string()),
        Just("US".to_string())
    ]
}

/// 有効な Price 用の Decimal 戦略（0-1000）
fn valid_price_strategy() -> impl Strategy<Value = Decimal> {
    (0u32..=1000u32).prop_map(Decimal::from)
}

/// 無効な Price 用の Decimal 戦略（負または1000超）
fn invalid_price_strategy() -> impl Strategy<Value = Decimal> {
    prop_oneof![
        (1i32..100i32).prop_map(|v| Decimal::from(-v)),
        (1001u32..10000u32).prop_map(Decimal::from)
    ]
}

/// 有効な BillingAmount 用の Decimal 戦略（0-10000）
fn valid_billing_amount_strategy() -> impl Strategy<Value = Decimal> {
    (0u32..=10000u32).prop_map(Decimal::from)
}

/// 無効な BillingAmount 用の Decimal 戦略
fn invalid_billing_amount_strategy() -> impl Strategy<Value = Decimal> {
    prop_oneof![
        (1i32..100i32).prop_map(|v| Decimal::from(-v)),
        (10001u32..100000u32).prop_map(Decimal::from)
    ]
}

/// 有効な UnitQuantity 用の u32 戦略（1-1000）
fn valid_unit_quantity_strategy() -> impl Strategy<Value = u32> {
    1u32..=1000u32
}

/// 無効な UnitQuantity 用の u32 戦略
fn invalid_unit_quantity_strategy() -> impl Strategy<Value = u32> {
    prop_oneof![Just(0u32), (1001u32..10000u32)]
}

/// 有効な KilogramQuantity 用の Decimal 戦略（0.05-100）
fn valid_kilogram_quantity_strategy() -> impl Strategy<Value = Decimal> {
    (5u32..=10000u32).prop_map(|v| Decimal::from(v) / Decimal::from(100))
}

/// 無効な KilogramQuantity 用の Decimal 戦略
fn invalid_kilogram_quantity_strategy() -> impl Strategy<Value = Decimal> {
    prop_oneof![
        (0u32..4u32).prop_map(|v| Decimal::from(v) / Decimal::from(100)), // 0.00-0.04
        (10001u32..20000u32).prop_map(|v| Decimal::from(v) / Decimal::from(100))  // 100.01+
    ]
}

/// 有効な Widget ProductCode 用の戦略（W + 4桁）
fn valid_widget_code_strategy() -> impl Strategy<Value = String> {
    (0u32..10000u32).prop_map(|v| format!("W{v:04}"))
}

/// 有効な Gizmo ProductCode 用の戦略（G + 3桁）
fn valid_gizmo_code_strategy() -> impl Strategy<Value = String> {
    (0u32..1000u32).prop_map(|v| format!("G{v:03}"))
}

/// 無効な ProductCode 用の戦略
fn invalid_product_code_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // W/G 以外の大文字 + 4桁（[A-FH-VX-Z] は W, G を除外）
        proptest::string::string_regex("[A-FH-VX-Z][0-9]{4}").unwrap(),
        Just("W123".to_string()),  // Widget コードだが3桁（4桁が必要）
        Just("G1234".to_string()), // Gizmo コードだが4桁（3桁が必要）
        Just("W".to_string()),     // 数字がない
        Just("G".to_string()),     // 数字がない
        Just("12345".to_string()), // プレフィックスがない
        Just("w0001".to_string()), // 小文字の w（大文字が必要）
        Just("g001".to_string())   // 小文字の g（大文字が必要）
    ]
}

// =============================================================================
// String50 法則テスト
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// String50: 有効な入力で Ok が返り、value() は入力と等しい
    #[test]
    fn test_string50_valid_roundtrip(input in valid_string50_strategy()) {
        let result = String50::create("TestField", &input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input.as_str());
    }

    /// String50: 無効な入力で Err が返る
    #[test]
    fn test_string50_invalid_fails(input in invalid_string50_strategy()) {
        let result = String50::create("TestField", &input);
        prop_assert!(result.is_err());
    }

    /// String50: べき等性 - 同じ入力から同じ結果
    #[test]
    fn test_string50_idempotent(input in valid_string50_strategy()) {
        let result1 = String50::create("TestField", &input);
        let result2 = String50::create("TestField", &input);
        prop_assert_eq!(result1.is_ok(), result2.is_ok());
        if let (Ok(v1), Ok(v2)) = (result1, result2) {
            prop_assert_eq!(v1, v2);
        }
    }
}

// =============================================================================
// EmailAddress 法則テスト
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// EmailAddress: 有効な入力で Ok が返り、value() は入力と等しい
    #[test]
    fn test_email_valid_roundtrip(input in valid_email_strategy()) {
        let result = EmailAddress::create("Email", &input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input.as_str());
    }

    /// EmailAddress: 無効な入力で Err が返る
    #[test]
    fn test_email_invalid_fails(input in invalid_email_strategy()) {
        let result = EmailAddress::create("Email", &input);
        prop_assert!(result.is_err());
    }
}

// =============================================================================
// ZipCode 法則テスト
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// ZipCode: 有効な入力で Ok が返り、value() は入力と等しい
    #[test]
    fn test_zip_code_valid_roundtrip(input in valid_zip_code_strategy()) {
        let result = ZipCode::create("ZipCode", &input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input.as_str());
    }

    /// ZipCode: 無効な入力で Err が返る
    #[test]
    fn test_zip_code_invalid_fails(input in invalid_zip_code_strategy()) {
        let result = ZipCode::create("ZipCode", &input);
        prop_assert!(result.is_err());
    }
}

// =============================================================================
// UsStateCode 法則テスト
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// UsStateCode: 有効な入力で Ok が返り、value() は入力と等しい
    #[test]
    fn test_state_code_valid_roundtrip(input in valid_state_code_strategy()) {
        let result = UsStateCode::create("State", &input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input.as_str());
    }

    /// UsStateCode: 無効な入力で Err が返る
    #[test]
    fn test_state_code_invalid_fails(input in invalid_state_code_strategy()) {
        let result = UsStateCode::create("State", &input);
        prop_assert!(result.is_err());
    }
}

// =============================================================================
// Price 法則テスト
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Price: 有効な入力で Ok が返り、value() は入力と等しい
    #[test]
    fn test_price_valid_roundtrip(input in valid_price_strategy()) {
        let result = Price::create(input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input);
    }

    /// Price: 無効な入力で Err が返る
    #[test]
    fn test_price_invalid_fails(input in invalid_price_strategy()) {
        let result = Price::create(input);
        prop_assert!(result.is_err());
    }

    /// Price: 不変条件 - Ok で返された値は常に [0, 1000] の範囲内
    #[test]
    fn test_price_invariant(input in valid_price_strategy()) {
        let result = Price::create(input);
        if let Ok(price) = result {
            prop_assert!(price.value() >= Decimal::ZERO);
            prop_assert!(price.value() <= Decimal::from(1000));
        }
    }
}

// =============================================================================
// BillingAmount 法則テスト
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// BillingAmount: 有効な入力で Ok が返り、value() は入力と等しい
    #[test]
    fn test_billing_amount_valid_roundtrip(input in valid_billing_amount_strategy()) {
        let result = BillingAmount::create(input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input);
    }

    /// BillingAmount: 無効な入力で Err が返る
    #[test]
    fn test_billing_amount_invalid_fails(input in invalid_billing_amount_strategy()) {
        let result = BillingAmount::create(input);
        prop_assert!(result.is_err());
    }

    /// BillingAmount: 不変条件 - Ok で返された値は常に [0, 10000] の範囲内
    #[test]
    fn test_billing_amount_invariant(input in valid_billing_amount_strategy()) {
        let result = BillingAmount::create(input);
        if let Ok(amount) = result {
            prop_assert!(amount.value() >= Decimal::ZERO);
            prop_assert!(amount.value() <= Decimal::from(10000));
        }
    }
}

// =============================================================================
// UnitQuantity 法則テスト
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// UnitQuantity: 有効な入力で Ok が返り、value() は入力と等しい
    #[test]
    fn test_unit_quantity_valid_roundtrip(input in valid_unit_quantity_strategy()) {
        let result = UnitQuantity::create("Quantity", input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input);
    }

    /// UnitQuantity: 無効な入力で Err が返る
    #[test]
    fn test_unit_quantity_invalid_fails(input in invalid_unit_quantity_strategy()) {
        let result = UnitQuantity::create("Quantity", input);
        prop_assert!(result.is_err());
    }

    /// UnitQuantity: 不変条件 - Ok で返された値は常に [1, 1000] の範囲内
    #[test]
    fn test_unit_quantity_invariant(input in valid_unit_quantity_strategy()) {
        let result = UnitQuantity::create("Quantity", input);
        if let Ok(qty) = result {
            prop_assert!(qty.value() >= 1);
            prop_assert!(qty.value() <= 1000);
        }
    }
}

// =============================================================================
// KilogramQuantity 法則テスト
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// KilogramQuantity: 有効な入力で Ok が返り、value() は入力と等しい
    #[test]
    fn test_kilogram_quantity_valid_roundtrip(input in valid_kilogram_quantity_strategy()) {
        let result = KilogramQuantity::create("Weight", input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input);
    }

    /// KilogramQuantity: 無効な入力で Err が返る
    #[test]
    fn test_kilogram_quantity_invalid_fails(input in invalid_kilogram_quantity_strategy()) {
        let result = KilogramQuantity::create("Weight", input);
        prop_assert!(result.is_err());
    }

    /// KilogramQuantity: 不変条件 - Ok で返された値は常に [0.05, 100] の範囲内
    #[test]
    fn test_kilogram_quantity_invariant(input in valid_kilogram_quantity_strategy()) {
        let result = KilogramQuantity::create("Weight", input);
        if let Ok(qty) = result {
            prop_assert!(qty.value() >= Decimal::from_str("0.05").unwrap());
            prop_assert!(qty.value() <= Decimal::from(100));
        }
    }
}

// =============================================================================
// ProductCode 法則テスト
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// ProductCode (Widget): 有効な入力で Ok が返り、value() は入力と等しい
    #[test]
    fn test_widget_code_valid_roundtrip(input in valid_widget_code_strategy()) {
        let result = ProductCode::create("ProductCode", &input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input.as_str());
        prop_assert!(matches!(value, ProductCode::Widget(_)));
    }

    /// ProductCode (Gizmo): 有効な入力で Ok が返り、value() は入力と等しい
    #[test]
    fn test_gizmo_code_valid_roundtrip(input in valid_gizmo_code_strategy()) {
        let result = ProductCode::create("ProductCode", &input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input.as_str());
        prop_assert!(matches!(value, ProductCode::Gizmo(_)));
    }

    /// ProductCode: 無効な入力で Err が返る
    #[test]
    fn test_product_code_invalid_fails(input in invalid_product_code_strategy()) {
        let result = ProductCode::create("ProductCode", &input);
        prop_assert!(result.is_err());
    }
}

// =============================================================================
// OrderId/OrderLineId 法則テスト
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// OrderId: 有効な入力で Ok が返り、value() は入力と等しい
    #[test]
    fn test_order_id_valid_roundtrip(input in valid_string50_strategy()) {
        let result = OrderId::create("OrderId", &input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input.as_str());
    }

    /// OrderLineId: 有効な入力で Ok が返り、value() は入力と等しい
    #[test]
    fn test_order_line_id_valid_roundtrip(input in valid_string50_strategy()) {
        let result = OrderLineId::create("OrderLineId", &input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input.as_str());
    }
}
