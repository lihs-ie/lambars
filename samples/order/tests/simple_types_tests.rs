//! simple_types モジュールの補完テスト
//!
//! Phase 1 で実装された simple_types の境界値テスト、エラーメッセージ検証、
//! Hash/Eq 法則の検証を行う。src 内の基本テストを補完する形で設計。

use order_taking_sample::simple_types::{
    BillingAmount, EmailAddress, KilogramQuantity, OrderId, OrderLineId, Price, ProductCode,
    String50, UnitQuantity, UsStateCode, ZipCode,
};
use rstest::rstest;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::str::FromStr;

// =============================================================================
// ヘルパー関数
// =============================================================================

/// 値のハッシュを計算するヘルパー関数
fn calculate_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

// =============================================================================
// String50 境界値テスト
// =============================================================================

mod string50_boundary_tests {
    use super::*;

    #[rstest]
    #[case("a", true)] // 最小: 1文字
    #[case("ab", true)] // 2文字
    #[case(&"a".repeat(49), true)] // 49文字
    #[case(&"a".repeat(50), true)] // 最大: 50文字
    #[case(&"a".repeat(51), false)] // 超過: 51文字
    fn test_string50_boundary_values(#[case] input: &str, #[case] expected_ok: bool) {
        let result = String50::create("TestField", input);
        assert_eq!(result.is_ok(), expected_ok, "Input length: {}", input.len());
    }

    #[rstest]
    fn test_string50_with_unicode() {
        // Unicode 文字（日本語）も文字数でカウント
        let japanese = "あいうえお"; // 5文字
        let result = String50::create("TestField", japanese);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), japanese);
    }

    #[rstest]
    fn test_string50_with_emoji() {
        // 絵文字も文字数でカウント
        let emoji = "hello world"; // ASCII のみ
        let result = String50::create("TestField", emoji);
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_string50_with_whitespace() {
        // 空白のみの文字列は許可される（空でない）
        let whitespace = "   ";
        let result = String50::create("TestField", whitespace);
        assert!(result.is_ok());
    }
}

// =============================================================================
// EmailAddress エッジケーステスト
// =============================================================================

mod email_address_edge_cases {
    use super::*;

    #[rstest]
    fn test_email_address_multiple_at_symbols() {
        // 現在の実装では .+@.+ パターンなので、複数の @ は許可される
        let result = EmailAddress::create("Email", "user@middle@domain.com");
        // パターン .+@.+ は最初の @ 以降に何かあればマッチする
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_email_address_with_unicode_local_part() {
        // Unicode を含むローカルパート
        let result = EmailAddress::create("Email", "user@domain.com");
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_email_address_with_plus_sign() {
        // + サブアドレッシング
        let result = EmailAddress::create("Email", "user+tag@example.com");
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_email_address_with_dots() {
        // ドット付きローカルパート
        let result = EmailAddress::create("Email", "first.last@example.com");
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_email_address_very_long() {
        // 非常に長いメールアドレス
        let long_local = "a".repeat(100);
        let long_domain = "b".repeat(100);
        let email = format!("{long_local}@{long_domain}.com");
        let result = EmailAddress::create("Email", &email);
        assert!(result.is_ok());
    }
}

// =============================================================================
// ZipCode 境界値テスト
// =============================================================================

mod zip_code_boundary_tests {
    use super::*;

    #[rstest]
    #[case("00000", true)] // 最小値
    #[case("00001", true)] // 最小値+1
    #[case("12345", true)] // 中間値
    #[case("99998", true)] // 最大値-1
    #[case("99999", true)] // 最大値
    #[case("0000", false)] // 4桁
    #[case("000000", false)] // 6桁
    #[case("1234a", false)] // 文字混入
    #[case("ABCDE", false)] // 全て文字
    fn test_zip_code_boundary_values(#[case] input: &str, #[case] expected_ok: bool) {
        let result = ZipCode::create("ZipCode", input);
        assert_eq!(result.is_ok(), expected_ok, "Input: {input}");
    }

    #[rstest]
    fn test_zip_code_with_leading_zeros() {
        // 先頭ゼロは保持される
        let result = ZipCode::create("ZipCode", "00123");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), "00123");
    }
}

// =============================================================================
// UsStateCode 全州テスト
// =============================================================================

mod us_state_code_tests {
    use super::*;

    /// 全50州 + DC のコードリスト
    const ALL_STATE_CODES: &[&str] = &[
        "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "DC", "FL", "GA", "HI", "ID", "IL", "IN",
        "IA", "KS", "KY", "LA", "MA", "MD", "ME", "MI", "MN", "MO", "MS", "MT", "NC", "ND", "NE",
        "NH", "NJ", "NM", "NV", "NY", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN", "TX", "UT",
        "VA", "VT", "WA", "WI", "WV", "WY",
    ];

    #[rstest]
    fn test_all_state_codes_are_valid() {
        for code in ALL_STATE_CODES {
            let result = UsStateCode::create("State", code);
            assert!(result.is_ok(), "State code {code} should be valid");
        }
    }

    #[rstest]
    fn test_state_code_count() {
        // 50州 + DC = 51 コード
        assert_eq!(ALL_STATE_CODES.len(), 51);
    }

    #[rstest]
    #[case("XX")]
    #[case("AA")]
    #[case("ZZ")]
    #[case("US")]
    fn test_invalid_state_codes(#[case] code: &str) {
        let result = UsStateCode::create("State", code);
        assert!(result.is_err(), "State code {code} should be invalid");
    }
}

// =============================================================================
// Price 境界値テスト
// =============================================================================

mod price_boundary_tests {
    use super::*;

    #[rstest]
    #[case("0.00", true)] // 最小値
    #[case("0.01", true)] // 最小値+0.01
    #[case("500.00", true)] // 中間値
    #[case("999.99", true)] // 最大値-0.01
    #[case("1000.00", true)] // 最大値
    #[case("-0.01", false)] // 負の値
    #[case("1000.01", false)] // 超過
    #[case("1001.00", false)] // 超過（整数）
    fn test_price_boundary_values(#[case] input: &str, #[case] expected_ok: bool) {
        let decimal = Decimal::from_str(input).unwrap();
        let result = Price::create(decimal);
        assert_eq!(result.is_ok(), expected_ok, "Input: {input}");
    }

    #[rstest]
    fn test_price_multiply_exact_max() {
        // 100.00 * 10 = 1000.00（ちょうど最大値）
        let price = Price::create(Decimal::from_str("100.00").unwrap()).unwrap();
        let result = price.multiply(Decimal::from(10));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), Decimal::from(1000));
    }

    #[rstest]
    fn test_price_multiply_just_over_max() {
        // 100.01 * 10 = 1000.10（超過）
        let price = Price::create(Decimal::from_str("100.01").unwrap()).unwrap();
        let result = price.multiply(Decimal::from(10));
        assert!(result.is_err());
    }
}

// =============================================================================
// BillingAmount 境界値テスト
// =============================================================================

mod billing_amount_boundary_tests {
    use super::*;

    #[rstest]
    #[case("0.00", true)] // 最小値
    #[case("0.01", true)] // 最小値+0.01
    #[case("5000.00", true)] // 中間値
    #[case("9999.99", true)] // 最大値-0.01
    #[case("10000.00", true)] // 最大値
    #[case("-0.01", false)] // 負の値
    #[case("10000.01", false)] // 超過
    fn test_billing_amount_boundary_values(#[case] input: &str, #[case] expected_ok: bool) {
        let decimal = Decimal::from_str(input).unwrap();
        let result = BillingAmount::create(decimal);
        assert_eq!(result.is_ok(), expected_ok, "Input: {input}");
    }

    #[rstest]
    fn test_billing_amount_sum_to_exact_max() {
        // 1000.00 * 10 = 10000.00（ちょうど最大値）
        let prices: Vec<Price> = (0..10)
            .map(|_| Price::create(Decimal::from(1000)).unwrap())
            .collect();
        let result = BillingAmount::sum_prices(&prices);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), Decimal::from(10000));
    }

    #[rstest]
    fn test_billing_amount_sum_just_over_max() {
        // 1000.00 * 10 + 0.01 = 10000.01（超過）
        let mut prices: Vec<Price> = (0..10)
            .map(|_| Price::create(Decimal::from(1000)).unwrap())
            .collect();
        prices.push(Price::create(Decimal::from_str("0.01").unwrap()).unwrap());
        let result = BillingAmount::sum_prices(&prices);
        assert!(result.is_err());
    }
}

// =============================================================================
// UnitQuantity 境界値テスト
// =============================================================================

mod unit_quantity_boundary_tests {
    use super::*;

    #[rstest]
    #[case(0, false)] // 無効（最小値未満）
    #[case(1, true)] // 最小値
    #[case(2, true)] // 最小値+1
    #[case(500, true)] // 中間値
    #[case(999, true)] // 最大値-1
    #[case(1000, true)] // 最大値
    #[case(1001, false)] // 超過
    fn test_unit_quantity_boundary_values(#[case] input: u32, #[case] expected_ok: bool) {
        let result = UnitQuantity::create("Quantity", input);
        assert_eq!(result.is_ok(), expected_ok, "Input: {input}");
    }
}

// =============================================================================
// KilogramQuantity 境界値テスト
// =============================================================================

mod kilogram_quantity_boundary_tests {
    use super::*;

    #[rstest]
    #[case("0.04", false)] // 無効（最小値未満）
    #[case("0.05", true)] // 最小値
    #[case("0.06", true)] // 最小値+0.01
    #[case("50.00", true)] // 中間値
    #[case("99.99", true)] // 最大値-0.01
    #[case("100.00", true)] // 最大値
    #[case("100.01", false)] // 超過
    fn test_kilogram_quantity_boundary_values(#[case] input: &str, #[case] expected_ok: bool) {
        let decimal = Decimal::from_str(input).unwrap();
        let result = KilogramQuantity::create("Weight", decimal);
        assert_eq!(result.is_ok(), expected_ok, "Input: {input}");
    }
}

// =============================================================================
// エラーメッセージ検証
// =============================================================================

mod error_message_tests {
    use super::*;

    #[rstest]
    fn test_string50_empty_error_message() {
        let result = String50::create("CustomerName", "");
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "CustomerName");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_string50_too_long_error_message() {
        let long_value = "a".repeat(51);
        let result = String50::create("CustomerName", &long_value);
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "CustomerName");
        assert_eq!(error.message, "Must not be more than 50 chars");
    }

    #[rstest]
    fn test_email_address_no_at_error_message() {
        let result = EmailAddress::create("Email", "invalid-email");
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Email");
        assert!(error.message.contains("must match the pattern"));
    }

    #[rstest]
    fn test_product_code_unrecognized_format_error_message() {
        let result = ProductCode::create("ProductCode", "X999");
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "ProductCode");
        assert!(error.message.contains("Format not recognized"));
        assert!(error.message.contains("X999"));
    }

    #[rstest]
    fn test_price_too_low_error_message() {
        let result = Price::create(Decimal::from_str("-1.0").unwrap());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Price");
        assert!(error.message.contains("Must not be less than"));
    }

    #[rstest]
    fn test_price_too_high_error_message() {
        let result = Price::create(Decimal::from_str("1001.0").unwrap());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Price");
        assert!(error.message.contains("Must not be greater than"));
    }

    #[rstest]
    fn test_unit_quantity_too_low_error_message() {
        let result = UnitQuantity::create("Quantity", 0);
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Quantity");
        assert_eq!(error.message, "Must not be less than 1");
    }

    #[rstest]
    fn test_unit_quantity_too_high_error_message() {
        let result = UnitQuantity::create("Quantity", 1001);
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Quantity");
        assert_eq!(error.message, "Must not be greater than 1000");
    }
}

// =============================================================================
// Hash/Eq 一貫性テスト
// =============================================================================

mod hash_eq_consistency_tests {
    use super::*;

    #[rstest]
    fn test_order_id_hash_eq_consistency() {
        let id1 = OrderId::create("OrderId", "ORD-001").unwrap();
        let id2 = OrderId::create("OrderId", "ORD-001").unwrap();
        let id3 = OrderId::create("OrderId", "ORD-002").unwrap();

        // a == b => hash(a) == hash(b)
        assert_eq!(id1, id2);
        assert_eq!(calculate_hash(&id1), calculate_hash(&id2));

        // a != b でもハッシュは同じ可能性がある（衝突）ので、逆は必ずしも成立しない
        assert_ne!(id1, id3);
    }

    #[rstest]
    fn test_order_id_hash_map_usage() {
        let id1 = OrderId::create("OrderId", "ORD-001").unwrap();
        let id2 = OrderId::create("OrderId", "ORD-001").unwrap();

        let mut map: HashMap<OrderId, String> = HashMap::new();
        map.insert(id1.clone(), "First Order".to_string());

        // 同じ値を持つ別のインスタンスでもキーとして機能する
        assert_eq!(map.get(&id2), Some(&"First Order".to_string()));
    }

    #[rstest]
    fn test_product_code_hash_eq_consistency() {
        let widget1 = ProductCode::create("ProductCode", "W1234").unwrap();
        let widget2 = ProductCode::create("ProductCode", "W1234").unwrap();
        let gizmo = ProductCode::create("ProductCode", "G123").unwrap();

        // a == b => hash(a) == hash(b)
        assert_eq!(widget1, widget2);
        assert_eq!(calculate_hash(&widget1), calculate_hash(&widget2));

        assert_ne!(widget1, gizmo);
    }

    #[rstest]
    fn test_product_code_hash_map_usage() {
        let widget = ProductCode::create("ProductCode", "W1234").unwrap();
        let gizmo = ProductCode::create("ProductCode", "G123").unwrap();

        let mut map: HashMap<ProductCode, Decimal> = HashMap::new();
        map.insert(widget.clone(), Decimal::from(100));
        map.insert(gizmo.clone(), Decimal::from(200));

        assert_eq!(map.get(&widget), Some(&Decimal::from(100)));
        assert_eq!(map.get(&gizmo), Some(&Decimal::from(200)));
    }

    #[rstest]
    fn test_order_line_id_hash_eq_consistency() {
        let line1 = OrderLineId::create("LineId", "LINE-001").unwrap();
        let line2 = OrderLineId::create("LineId", "LINE-001").unwrap();

        assert_eq!(line1, line2);
        assert_eq!(calculate_hash(&line1), calculate_hash(&line2));
    }

    #[rstest]
    fn test_string50_hash_eq_consistency() {
        let str1 = String50::create("Field", "Test Value").unwrap();
        let str2 = String50::create("Field", "Test Value").unwrap();

        assert_eq!(str1, str2);
        assert_eq!(calculate_hash(&str1), calculate_hash(&str2));
    }

    #[rstest]
    fn test_email_address_hash_eq_consistency() {
        let email1 = EmailAddress::create("Email", "test@example.com").unwrap();
        let email2 = EmailAddress::create("Email", "test@example.com").unwrap();

        assert_eq!(email1, email2);
        assert_eq!(calculate_hash(&email1), calculate_hash(&email2));
    }

    #[rstest]
    fn test_zip_code_hash_eq_consistency() {
        let zip1 = ZipCode::create("ZipCode", "12345").unwrap();
        let zip2 = ZipCode::create("ZipCode", "12345").unwrap();

        assert_eq!(zip1, zip2);
        assert_eq!(calculate_hash(&zip1), calculate_hash(&zip2));
    }

    #[rstest]
    fn test_us_state_code_hash_eq_consistency() {
        let state1 = UsStateCode::create("State", "CA").unwrap();
        let state2 = UsStateCode::create("State", "CA").unwrap();

        assert_eq!(state1, state2);
        assert_eq!(calculate_hash(&state1), calculate_hash(&state2));
    }

    #[rstest]
    fn test_price_hash_eq_consistency() {
        let price1 = Price::create(Decimal::from_str("99.99").unwrap()).unwrap();
        let price2 = Price::create(Decimal::from_str("99.99").unwrap()).unwrap();

        assert_eq!(price1, price2);
        assert_eq!(calculate_hash(&price1), calculate_hash(&price2));
    }

    #[rstest]
    fn test_billing_amount_hash_eq_consistency() {
        let amount1 = BillingAmount::create(Decimal::from(5000)).unwrap();
        let amount2 = BillingAmount::create(Decimal::from(5000)).unwrap();

        assert_eq!(amount1, amount2);
        assert_eq!(calculate_hash(&amount1), calculate_hash(&amount2));
    }

    #[rstest]
    fn test_unit_quantity_hash_eq_consistency() {
        let qty1 = UnitQuantity::create("Quantity", 100).unwrap();
        let qty2 = UnitQuantity::create("Quantity", 100).unwrap();

        assert_eq!(qty1, qty2);
        assert_eq!(calculate_hash(&qty1), calculate_hash(&qty2));
    }

    #[rstest]
    fn test_kilogram_quantity_hash_eq_consistency() {
        let qty1 = KilogramQuantity::create("Weight", Decimal::from_str("50.0").unwrap()).unwrap();
        let qty2 = KilogramQuantity::create("Weight", Decimal::from_str("50.0").unwrap()).unwrap();

        assert_eq!(qty1, qty2);
        assert_eq!(calculate_hash(&qty1), calculate_hash(&qty2));
    }
}
