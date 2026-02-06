//! Proptest verification of Smart Constructor laws
//!
//! Verifies that types built with the Smart Constructor pattern satisfy the following properties:
//! 1. Equality law: the value obtained by value() equals the value at creation
//! 2. Invariant: values produced by Ok always satisfy constraints
//! 3. Idempotency: the same input produces the same result

use order_taking_sample::simple_types::{
    BillingAmount, EmailAddress, KilogramQuantity, OrderId, OrderLineId, Price, ProductCode,
    String50, UnitQuantity, UsStateCode, ZipCode,
};
use proptest::prelude::*;
use rust_decimal::Decimal;
use std::str::FromStr;

// =============================================================================
// Strategy definitions
// =============================================================================

/// String strategy for valid String50
fn valid_string50_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-zA-Z0-9 ]{1,50}")
        .unwrap()
        .prop_filter("non-empty", |s| !s.is_empty())
}

/// String strategy for invalid String50 (empty or 51+ characters)
fn invalid_string50_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::new()),
        proptest::string::string_regex("[a-zA-Z0-9]{51,100}").unwrap()
    ]
}

/// String strategy for valid EmailAddress
fn valid_email_strategy() -> impl Strategy<Value = String> {
    (
        proptest::string::string_regex("[a-zA-Z0-9._%+-]{1,20}").unwrap(),
        proptest::string::string_regex("[a-zA-Z0-9.-]{1,20}").unwrap(),
        proptest::string::string_regex("[a-zA-Z]{2,5}").unwrap(),
    )
        .prop_map(|(local, domain, tld)| format!("{local}@{domain}.{tld}"))
}

/// String strategy for invalid EmailAddress (no @)
fn invalid_email_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-zA-Z0-9]{1,30}").unwrap()
}

/// String strategy for valid ZipCode (5-digit number)
fn valid_zip_code_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[0-9]{5}").unwrap()
}

/// String strategy for invalid ZipCode
fn invalid_zip_code_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        proptest::string::string_regex("[0-9]{1,4}").unwrap(),
        proptest::string::string_regex("[0-9]{6,10}").unwrap(),
        proptest::string::string_regex("[a-zA-Z]{5}").unwrap()
    ]
}

/// Strategy for valid UsStateCode
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

/// Strategy for invalid UsStateCode
fn invalid_state_code_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("XX".to_string()),
        Just("ZZ".to_string()),
        Just("US".to_string())
    ]
}

/// Decimal strategy for valid Price (0-1000)
fn valid_price_strategy() -> impl Strategy<Value = Decimal> {
    (0u32..=1000u32).prop_map(Decimal::from)
}

/// Decimal strategy for invalid Price (negative or >1000)
fn invalid_price_strategy() -> impl Strategy<Value = Decimal> {
    prop_oneof![
        (1i32..100i32).prop_map(|v| Decimal::from(-v)),
        (1001u32..10000u32).prop_map(Decimal::from)
    ]
}

/// Decimal strategy for valid BillingAmount (0-10000)
fn valid_billing_amount_strategy() -> impl Strategy<Value = Decimal> {
    (0u32..=10000u32).prop_map(Decimal::from)
}

/// Decimal strategy for invalid BillingAmount
fn invalid_billing_amount_strategy() -> impl Strategy<Value = Decimal> {
    prop_oneof![
        (1i32..100i32).prop_map(|v| Decimal::from(-v)),
        (10001u32..100000u32).prop_map(Decimal::from)
    ]
}

/// u32 strategy for valid UnitQuantity (1-1000)
fn valid_unit_quantity_strategy() -> impl Strategy<Value = u32> {
    1u32..=1000u32
}

/// u32 strategy for invalid UnitQuantity
fn invalid_unit_quantity_strategy() -> impl Strategy<Value = u32> {
    prop_oneof![Just(0u32), (1001u32..10000u32)]
}

/// Decimal strategy for valid KilogramQuantity (0.05-100)
fn valid_kilogram_quantity_strategy() -> impl Strategy<Value = Decimal> {
    (5u32..=10000u32).prop_map(|v| Decimal::from(v) / Decimal::from(100))
}

/// Decimal strategy for invalid KilogramQuantity
fn invalid_kilogram_quantity_strategy() -> impl Strategy<Value = Decimal> {
    prop_oneof![
        (0u32..4u32).prop_map(|v| Decimal::from(v) / Decimal::from(100)), // 0.00-0.04
        (10001u32..20000u32).prop_map(|v| Decimal::from(v) / Decimal::from(100))  // 100.01+
    ]
}

/// Strategy for valid Widget ProductCode (W + 4 digits)
fn valid_widget_code_strategy() -> impl Strategy<Value = String> {
    (0u32..10000u32).prop_map(|v| format!("W{v:04}"))
}

/// Strategy for valid Gizmo ProductCode (G + 3 digits)
fn valid_gizmo_code_strategy() -> impl Strategy<Value = String> {
    (0u32..1000u32).prop_map(|v| format!("G{v:03}"))
}

/// Strategy for invalid ProductCode
fn invalid_product_code_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Uppercase letter other than W/G + 4 digits ([A-FH-VX-Z] excludes W, G)
        proptest::string::string_regex("[A-FH-VX-Z][0-9]{4}").unwrap(),
        Just("W123".to_string()),  // Widget code but 3 digits (4 digits required)
        Just("G1234".to_string()), // Gizmo code but 4 digits (3 digits required)
        Just("W".to_string()),     // No digits
        Just("G".to_string()),     // No digits
        Just("12345".to_string()), // No prefix
        Just("w0001".to_string()), // Lowercase w (uppercase required)
        Just("g001".to_string())   // Lowercase g (uppercase required)
    ]
}

// =============================================================================
// String50 lawTest
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// String50: Ok is returned for valid input, and value() equals input
    #[test]
    fn test_string50_valid_roundtrip(input in valid_string50_strategy()) {
        let result = String50::create("TestField", &input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input.as_str());
    }

    /// String50: Err is returned for invalid input
    #[test]
    fn test_string50_invalid_fails(input in invalid_string50_strategy()) {
        let result = String50::create("TestField", &input);
        prop_assert!(result.is_err());
    }

    /// String50: Idempotency - same input produces same result
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
// EmailAddress lawTest
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// EmailAddress: Ok is returned for valid input, and value() equals input
    #[test]
    fn test_email_valid_roundtrip(input in valid_email_strategy()) {
        let result = EmailAddress::create("Email", &input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input.as_str());
    }

    /// EmailAddress: Err is returned for invalid input
    #[test]
    fn test_email_invalid_fails(input in invalid_email_strategy()) {
        let result = EmailAddress::create("Email", &input);
        prop_assert!(result.is_err());
    }
}

// =============================================================================
// ZipCode lawTest
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// ZipCode: Ok is returned for valid input, and value() equals input
    #[test]
    fn test_zip_code_valid_roundtrip(input in valid_zip_code_strategy()) {
        let result = ZipCode::create("ZipCode", &input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input.as_str());
    }

    /// ZipCode: Err is returned for invalid input
    #[test]
    fn test_zip_code_invalid_fails(input in invalid_zip_code_strategy()) {
        let result = ZipCode::create("ZipCode", &input);
        prop_assert!(result.is_err());
    }
}

// =============================================================================
// UsStateCode lawTest
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// UsStateCode: Ok is returned for valid input, and value() equals input
    #[test]
    fn test_state_code_valid_roundtrip(input in valid_state_code_strategy()) {
        let result = UsStateCode::create("State", &input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input.as_str());
    }

    /// UsStateCode: Err is returned for invalid input
    #[test]
    fn test_state_code_invalid_fails(input in invalid_state_code_strategy()) {
        let result = UsStateCode::create("State", &input);
        prop_assert!(result.is_err());
    }
}

// =============================================================================
// Price lawTest
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Price: Ok is returned for valid input, and value() equals input
    #[test]
    fn test_price_valid_roundtrip(input in valid_price_strategy()) {
        let result = Price::create(input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input);
    }

    /// Price: Err is returned for invalid input
    #[test]
    fn test_price_invalid_fails(input in invalid_price_strategy()) {
        let result = Price::create(input);
        prop_assert!(result.is_err());
    }

    /// Price: Invariant - Ok value is always in range [0, 1000]
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
// BillingAmount lawTest
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// BillingAmount: Ok is returned for valid input, and value() equals input
    #[test]
    fn test_billing_amount_valid_roundtrip(input in valid_billing_amount_strategy()) {
        let result = BillingAmount::create(input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input);
    }

    /// BillingAmount: Err is returned for invalid input
    #[test]
    fn test_billing_amount_invalid_fails(input in invalid_billing_amount_strategy()) {
        let result = BillingAmount::create(input);
        prop_assert!(result.is_err());
    }

    /// BillingAmount: Invariant - Ok value is always in range [0, 10000]
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
// UnitQuantity lawTest
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// UnitQuantity: Ok is returned for valid input, and value() equals input
    #[test]
    fn test_unit_quantity_valid_roundtrip(input in valid_unit_quantity_strategy()) {
        let result = UnitQuantity::create("Quantity", input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input);
    }

    /// UnitQuantity: Err is returned for invalid input
    #[test]
    fn test_unit_quantity_invalid_fails(input in invalid_unit_quantity_strategy()) {
        let result = UnitQuantity::create("Quantity", input);
        prop_assert!(result.is_err());
    }

    /// UnitQuantity: Invariant - Ok value is always in range [1, 1000]
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
// KilogramQuantity lawTest
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// KilogramQuantity: Ok is returned for valid input, and value() equals input
    #[test]
    fn test_kilogram_quantity_valid_roundtrip(input in valid_kilogram_quantity_strategy()) {
        let result = KilogramQuantity::create("Weight", input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input);
    }

    /// KilogramQuantity: Err is returned for invalid input
    #[test]
    fn test_kilogram_quantity_invalid_fails(input in invalid_kilogram_quantity_strategy()) {
        let result = KilogramQuantity::create("Weight", input);
        prop_assert!(result.is_err());
    }

    /// KilogramQuantity: Invariant - Ok value is always in range [0.05, 100]
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
// ProductCode lawTest
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// ProductCode (Widget): Ok is returned for valid input, and value() equals input
    #[test]
    fn test_widget_code_valid_roundtrip(input in valid_widget_code_strategy()) {
        let result = ProductCode::create("ProductCode", &input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input.as_str());
        prop_assert!(matches!(value, ProductCode::Widget(_)));
    }

    /// ProductCode (Gizmo): Ok is returned for valid input, and value() equals input
    #[test]
    fn test_gizmo_code_valid_roundtrip(input in valid_gizmo_code_strategy()) {
        let result = ProductCode::create("ProductCode", &input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input.as_str());
        prop_assert!(matches!(value, ProductCode::Gizmo(_)));
    }

    /// ProductCode: Err is returned for invalid input
    #[test]
    fn test_product_code_invalid_fails(input in invalid_product_code_strategy()) {
        let result = ProductCode::create("ProductCode", &input);
        prop_assert!(result.is_err());
    }
}

// =============================================================================
// OrderId/OrderLineId lawTest
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// OrderId: Ok is returned for valid input, and value() equals input
    #[test]
    fn test_order_id_valid_roundtrip(input in valid_string50_strategy()) {
        let result = OrderId::create("OrderId", &input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input.as_str());
    }

    /// OrderLineId: Ok is returned for valid input, and value() equals input
    #[test]
    fn test_order_line_id_valid_roundtrip(input in valid_string50_strategy()) {
        let result = OrderLineId::create("OrderLineId", &input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input.as_str());
    }
}
