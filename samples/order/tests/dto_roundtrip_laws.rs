//! DTO 往復変換の proptest による検証
//!
//! DTO とドメイン型間の往復変換が情報を失わないことを検証する。
//!
//! 検証対象:
//! 1. Address <-> AddressDto の往復変換
//! 2. CustomerInfo <-> CustomerInfoDto の往復変換（VipStatus 考慮）
//! 3. OrderFormDto の JSON シリアライズ/デシリアライズ往復
//! 4. 出力 DTO の JSON シリアライズ/デシリアライズ往復

use order_taking_sample::compound_types::Address;
use order_taking_sample::dto::{
    AddressDto, BillableOrderPlacedDto, CustomerInfoDto, OrderAcknowledgmentSentDto, OrderFormDto,
    OrderFormLineDto, ShippableOrderLineDto, ShippableOrderPlacedDto,
};
use order_taking_sample::workflow::CheckedAddress;
use order_taking_sample::workflow::validation::to_address;
use proptest::prelude::*;
use rust_decimal::Decimal;

// =============================================================================
// 戦略（Strategy）定義
// =============================================================================

/// 有効な String50 用の文字列戦略（1-50文字の英数字）
fn valid_string50_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-zA-Z0-9 ]{1,50}")
        .unwrap()
        .prop_filter("non-empty", |s| !s.is_empty())
}

/// 有効なメールアドレス戦略
fn valid_email_strategy() -> impl Strategy<Value = String> {
    (
        proptest::string::string_regex("[a-zA-Z0-9]{1,10}").unwrap(),
        proptest::string::string_regex("[a-zA-Z0-9]{1,10}").unwrap(),
        proptest::string::string_regex("[a-zA-Z]{2,4}").unwrap(),
    )
        .prop_map(|(local, domain, tld)| format!("{local}@{domain}.{tld}"))
}

/// 有効な郵便番号戦略（5桁）
fn valid_zip_code_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[0-9]{5}").unwrap()
}

/// 有効な州コード戦略
fn valid_state_code_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("AL".to_string()),
        Just("AK".to_string()),
        Just("AZ".to_string()),
        Just("CA".to_string()),
        Just("NY".to_string()),
        Just("TX".to_string()),
        Just("FL".to_string()),
        Just("DC".to_string()),
    ]
}

/// AddressDto 戦略
fn address_dto_strategy() -> impl Strategy<Value = AddressDto> {
    (
        valid_string50_strategy(), // address_line1
        prop_oneof![Just(String::new()), valid_string50_strategy()], // address_line2
        Just(String::new()),       // address_line3（簡略化）
        Just(String::new()),       // address_line4（簡略化）
        valid_string50_strategy(), // city
        valid_zip_code_strategy(), // zip_code
        valid_state_code_strategy(), // state
        Just("USA".to_string()),   // country
    )
        .prop_map(
            |(line1, line2, line3, line4, city, zip, state, country)| AddressDto {
                address_line1: line1,
                address_line2: line2,
                address_line3: line3,
                address_line4: line4,
                city,
                zip_code: zip,
                state,
                country,
            },
        )
}

/// CustomerInfoDto 戦略
fn customer_info_dto_strategy() -> impl Strategy<Value = CustomerInfoDto> {
    (
        valid_string50_strategy(), // first_name
        valid_string50_strategy(), // last_name
        valid_email_strategy(),    // email_address
        prop_oneof![Just("Normal".to_string()), Just("VIP".to_string())], // vip_status
    )
        .prop_map(|(first, last, email, vip)| CustomerInfoDto {
            first_name: first,
            last_name: last,
            email_address: email,
            vip_status: vip,
        })
}

/// Widget 製品コード戦略
fn widget_code_strategy() -> impl Strategy<Value = String> {
    (0u32..10000u32).prop_map(|v| format!("W{v:04}"))
}

/// Gizmo 製品コード戦略
fn gizmo_code_strategy() -> impl Strategy<Value = String> {
    (0u32..1000u32).prop_map(|v| format!("G{v:03}"))
}

/// OrderFormLineDto 戦略
fn order_form_line_dto_strategy() -> impl Strategy<Value = OrderFormLineDto> {
    (
        valid_string50_strategy(),
        prop_oneof![widget_code_strategy(), gizmo_code_strategy()],
        // Widget は整数数量、Gizmo は小数数量だが、ここでは簡略化
        (1u32..100u32).prop_map(Decimal::from),
    )
        .prop_map(|(line_id, product_code, quantity)| OrderFormLineDto {
            order_line_id: line_id,
            product_code,
            quantity,
        })
}

/// OrderFormDto 戦略
fn order_form_dto_strategy() -> impl Strategy<Value = OrderFormDto> {
    (
        valid_string50_strategy(),    // order_id
        customer_info_dto_strategy(), // customer_info
        address_dto_strategy(),       // shipping_address
        address_dto_strategy(),       // billing_address
        proptest::collection::vec(order_form_line_dto_strategy(), 1..=5), // lines
        prop_oneof![Just(String::new()), Just("PROMO2025".to_string())], // promotion_code
    )
        .prop_map(
            |(order_id, customer, shipping, billing, lines, promo)| OrderFormDto {
                order_id,
                customer_info: customer,
                shipping_address: shipping,
                billing_address: billing,
                lines,
                promotion_code: promo,
            },
        )
}

// =============================================================================
// Address 往復変換テスト
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// AddressDto -> UnvalidatedAddress -> CheckedAddress -> Address -> AddressDto
    /// の往復で元の DTO と等しいことを検証
    #[test]
    fn test_address_dto_roundtrip(dto in address_dto_strategy()) {
        // DTO -> UnvalidatedAddress
        let unvalidated = dto.to_unvalidated_address();

        // UnvalidatedAddress -> CheckedAddress -> Address
        let checked = CheckedAddress::new(unvalidated);
        let address_result = to_address(&checked);

        // Address への変換が成功した場合のみ往復を検証
        if let Ok(address) = address_result {
            // Address -> AddressDto
            let roundtrip_dto = AddressDto::from_address(&address);

            // 元の DTO と比較
            prop_assert_eq!(dto.address_line1, roundtrip_dto.address_line1);
            prop_assert_eq!(dto.city, roundtrip_dto.city);
            prop_assert_eq!(dto.zip_code, roundtrip_dto.zip_code);
            prop_assert_eq!(dto.state, roundtrip_dto.state);
            prop_assert_eq!(dto.country, roundtrip_dto.country);

            // オプション行は空文字列として往復するので同値性を検証
            prop_assert_eq!(dto.address_line2, roundtrip_dto.address_line2);
            prop_assert_eq!(dto.address_line3, roundtrip_dto.address_line3);
            prop_assert_eq!(dto.address_line4, roundtrip_dto.address_line4);
        }
    }

    /// Address 生成後の DTO 往復が完全一致することを検証
    /// (Address::create で生成した有効な Address から開始)
    #[test]
    fn test_address_domain_to_dto_roundtrip(dto in address_dto_strategy()) {
        // まず Address を生成
        let address_result = Address::create(
            &dto.address_line1,
            &dto.address_line2,
            &dto.address_line3,
            &dto.address_line4,
            &dto.city,
            &dto.zip_code,
            &dto.state,
            &dto.country,
        );

        if let Ok(address) = address_result {
            // Address -> AddressDto -> UnvalidatedAddress -> Address
            let dto_from_address = AddressDto::from_address(&address);
            let unvalidated = dto_from_address.to_unvalidated_address();
            let checked = CheckedAddress::new(unvalidated);
            let roundtrip_address = to_address(&checked);

            prop_assert!(roundtrip_address.is_ok(), "Roundtrip should succeed for valid address");
            let roundtrip = roundtrip_address.unwrap();

            // ドメイン値の比較
            prop_assert_eq!(address.address_line1().value(), roundtrip.address_line1().value());
            prop_assert_eq!(address.city().value(), roundtrip.city().value());
            prop_assert_eq!(address.zip_code().value(), roundtrip.zip_code().value());
            prop_assert_eq!(address.state().value(), roundtrip.state().value());
        }
    }
}

// =============================================================================
// CustomerInfo 往復変換テスト（注意: VipStatus の正規化を考慮）
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// CustomerInfoDto の往復変換テスト
    /// VipStatus は "Normal" または "VIP" として正規化される
    #[test]
    fn test_customer_info_dto_roundtrip(dto in customer_info_dto_strategy()) {
        // DTO -> UnvalidatedCustomerInfo
        let unvalidated = dto.to_unvalidated_customer_info();

        // フィールドの往復確認
        prop_assert_eq!(dto.first_name.as_str(), unvalidated.first_name());
        prop_assert_eq!(dto.last_name.as_str(), unvalidated.last_name());
        prop_assert_eq!(dto.email_address.as_str(), unvalidated.email_address());
        prop_assert_eq!(dto.vip_status.as_str(), unvalidated.vip_status());
    }
}

// =============================================================================
// OrderFormDto JSON 往復テスト
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// OrderFormDto の JSON シリアライズ/デシリアライズ往復
    #[test]
    fn test_order_form_dto_json_roundtrip(dto in order_form_dto_strategy()) {
        // DTO -> JSON
        let json_result = serde_json::to_string(&dto);
        prop_assert!(json_result.is_ok(), "Serialization should succeed");
        let json = json_result.unwrap();

        // JSON -> DTO
        let deserialized_result: Result<OrderFormDto, _> = serde_json::from_str(&json);
        prop_assert!(deserialized_result.is_ok(), "Deserialization should succeed");
        let deserialized = deserialized_result.unwrap();

        // 元の DTO と比較
        prop_assert_eq!(dto, deserialized, "Roundtrip should preserve all data");
    }

    /// AddressDto の JSON 往復
    #[test]
    fn test_address_dto_json_roundtrip(dto in address_dto_strategy()) {
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: AddressDto = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(dto, deserialized);
    }

    /// CustomerInfoDto の JSON 往復
    #[test]
    fn test_customer_info_dto_json_roundtrip(dto in customer_info_dto_strategy()) {
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: CustomerInfoDto = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(dto, deserialized);
    }

    /// OrderFormLineDto の JSON 往復
    #[test]
    fn test_order_form_line_dto_json_roundtrip(dto in order_form_line_dto_strategy()) {
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: OrderFormLineDto = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(dto, deserialized);
    }
}

// =============================================================================
// 出力 DTO JSON 往復テスト
// =============================================================================

/// ShippableOrderLineDto 戦略
fn shippable_order_line_dto_strategy() -> impl Strategy<Value = ShippableOrderLineDto> {
    (
        prop_oneof![widget_code_strategy(), gizmo_code_strategy()],
        (1u32..1000u32).prop_map(Decimal::from),
    )
        .prop_map(|(code, qty)| ShippableOrderLineDto {
            product_code: code,
            quantity: qty,
        })
}

/// ShippableOrderPlacedDto 戦略
fn shippable_order_placed_dto_strategy() -> impl Strategy<Value = ShippableOrderPlacedDto> {
    (
        valid_string50_strategy(),
        address_dto_strategy(),
        proptest::collection::vec(shippable_order_line_dto_strategy(), 1..=3),
        Just("order.pdf".to_string()),
        // Base64 encoded dummy data
        Just("SGVsbG8gV29ybGQh".to_string()),
    )
        .prop_map(|(id, addr, lines, name, data)| ShippableOrderPlacedDto {
            order_id: id,
            shipping_address: addr,
            shipment_lines: lines,
            pdf_name: name,
            pdf_data: data,
        })
}

/// BillableOrderPlacedDto 戦略
fn billable_order_placed_dto_strategy() -> impl Strategy<Value = BillableOrderPlacedDto> {
    (
        valid_string50_strategy(),
        address_dto_strategy(),
        (0u32..10000u32).prop_map(Decimal::from),
    )
        .prop_map(|(id, addr, amount)| BillableOrderPlacedDto {
            order_id: id,
            billing_address: addr,
            amount_to_bill: amount,
        })
}

/// OrderAcknowledgmentSentDto 戦略
fn order_acknowledgment_sent_dto_strategy() -> impl Strategy<Value = OrderAcknowledgmentSentDto> {
    (valid_string50_strategy(), valid_email_strategy()).prop_map(|(id, email)| {
        OrderAcknowledgmentSentDto {
            order_id: id,
            email_address: email,
        }
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// ShippableOrderLineDto の JSON 往復
    #[test]
    fn test_shippable_order_line_dto_json_roundtrip(dto in shippable_order_line_dto_strategy()) {
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: ShippableOrderLineDto = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(dto, deserialized);
    }

    /// ShippableOrderPlacedDto の JSON 往復
    #[test]
    fn test_shippable_order_placed_dto_json_roundtrip(dto in shippable_order_placed_dto_strategy()) {
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: ShippableOrderPlacedDto = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(dto, deserialized);
    }

    /// BillableOrderPlacedDto の JSON 往復
    #[test]
    fn test_billable_order_placed_dto_json_roundtrip(dto in billable_order_placed_dto_strategy()) {
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: BillableOrderPlacedDto = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(dto, deserialized);
    }

    /// OrderAcknowledgmentSentDto の JSON 往復
    #[test]
    fn test_order_acknowledgment_sent_dto_json_roundtrip(dto in order_acknowledgment_sent_dto_strategy()) {
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: OrderAcknowledgmentSentDto = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(dto, deserialized);
    }
}

// =============================================================================
// 変換の純粋性テスト
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// 同じ入力に対して同じ出力が返される（参照透過性）
    #[test]
    fn test_address_dto_conversion_is_pure(dto in address_dto_strategy()) {
        let unvalidated1 = dto.to_unvalidated_address();
        let unvalidated2 = dto.to_unvalidated_address();

        // 同じ入力から同じ出力
        prop_assert_eq!(unvalidated1.address_line1(), unvalidated2.address_line1());
        prop_assert_eq!(unvalidated1.city(), unvalidated2.city());
        prop_assert_eq!(unvalidated1.zip_code(), unvalidated2.zip_code());
    }

    /// CustomerInfoDto 変換の純粋性
    #[test]
    fn test_customer_info_dto_conversion_is_pure(dto in customer_info_dto_strategy()) {
        let unvalidated1 = dto.to_unvalidated_customer_info();
        let unvalidated2 = dto.to_unvalidated_customer_info();

        prop_assert_eq!(unvalidated1.first_name(), unvalidated2.first_name());
        prop_assert_eq!(unvalidated1.last_name(), unvalidated2.last_name());
        prop_assert_eq!(unvalidated1.email_address(), unvalidated2.email_address());
        prop_assert_eq!(unvalidated1.vip_status(), unvalidated2.vip_status());
    }

    /// OrderFormDto 変換の純粋性
    #[test]
    fn test_order_form_dto_conversion_is_pure(dto in order_form_dto_strategy()) {
        let unvalidated1 = dto.to_unvalidated_order();
        let unvalidated2 = dto.to_unvalidated_order();

        prop_assert_eq!(unvalidated1.order_id(), unvalidated2.order_id());
        prop_assert_eq!(unvalidated1.lines().len(), unvalidated2.lines().len());
        prop_assert_eq!(unvalidated1.promotion_code(), unvalidated2.promotion_code());
    }
}

// =============================================================================
// 境界値での往復テスト
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// 最大長の文字列を含む DTO の往復
    #[test]
    fn test_max_length_string_roundtrip(
        _dummy in any::<u8>()
    ) {
        // 50文字の最大長文字列
        let max_string = "A".repeat(50);

        let dto = AddressDto {
            address_line1: max_string.clone(),
            address_line2: String::new(),
            address_line3: String::new(),
            address_line4: String::new(),
            city: max_string.clone(),
            zip_code: "12345".to_string(),
            state: "NY".to_string(),
            country: max_string,
        };

        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: AddressDto = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(dto, deserialized);
    }

    /// 空のオプションフィールドの往復
    #[test]
    fn test_empty_optional_fields_roundtrip(_dummy in any::<u8>()) {
        let dto = AddressDto {
            address_line1: "123 Main St".to_string(),
            address_line2: String::new(),
            address_line3: String::new(),
            address_line4: String::new(),
            city: "City".to_string(),
            zip_code: "12345".to_string(),
            state: "NY".to_string(),
            country: "USA".to_string(),
        };

        // Address への変換
        let address_result = Address::create(
            &dto.address_line1,
            &dto.address_line2,
            &dto.address_line3,
            &dto.address_line4,
            &dto.city,
            &dto.zip_code,
            &dto.state,
            &dto.country,
        );

        prop_assert!(address_result.is_ok());
        let address = address_result.unwrap();

        // 往復
        let roundtrip_dto = AddressDto::from_address(&address);

        // オプションフィールドは空文字列として保持される
        prop_assert_eq!(dto.address_line2, roundtrip_dto.address_line2);
        prop_assert_eq!(dto.address_line3, roundtrip_dto.address_line3);
        prop_assert_eq!(dto.address_line4, roundtrip_dto.address_line4);
    }
}

// =============================================================================
// Decimal 精度保持テスト
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Decimal 値が JSON 往復で精度を保持することを検証
    #[test]
    fn test_decimal_precision_preserved_in_json(
        integer_part in 0u32..1000u32,
        decimal_part in 0u32..100u32
    ) {
        // 小数点以下2桁の Decimal を作成
        let decimal = Decimal::from(integer_part) + Decimal::from(decimal_part) / Decimal::from(100);

        let dto = OrderFormLineDto {
            order_line_id: "line-001".to_string(),
            product_code: "W1234".to_string(),
            quantity: decimal,
        };

        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: OrderFormLineDto = serde_json::from_str(&json).unwrap();

        prop_assert_eq!(dto.quantity, deserialized.quantity, "Decimal precision should be preserved");
    }

    /// 請求金額の精度保持
    #[test]
    fn test_billing_amount_precision_preserved(
        integer_part in 0u32..10000u32,
        decimal_part in 0u32..100u32
    ) {
        let amount = Decimal::from(integer_part) + Decimal::from(decimal_part) / Decimal::from(100);

        let dto = BillableOrderPlacedDto {
            order_id: "order-001".to_string(),
            billing_address: AddressDto {
                address_line1: "123 Main St".to_string(),
                address_line2: String::new(),
                address_line3: String::new(),
                address_line4: String::new(),
                city: "City".to_string(),
                zip_code: "12345".to_string(),
                state: "NY".to_string(),
                country: "USA".to_string(),
            },
            amount_to_bill: amount,
        };

        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: BillableOrderPlacedDto = serde_json::from_str(&json).unwrap();

        prop_assert_eq!(dto.amount_to_bill, deserialized.amount_to_bill);
    }
}
