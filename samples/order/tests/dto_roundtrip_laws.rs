//! Proptest verification of DTO round-trip conversions
//!
//! Verifies that round-trip conversion between DTOs and domain types does not lose information.
//!
//! verificationtarget:
//! 1. Address <-> AddressDto round-trip conversion
//! 2. CustomerInfo <-> CustomerInfoDto round-trip conversion (considering VipStatus)
//! 3. OrderFormDto JSON serialize/deserialize round-trip
//! 4. Output DTO JSON serialize/deserialize round-trip

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
// Strategy definitions
// =============================================================================

/// String strategy for valid String50 (1-50 alphanumeric characters)
fn valid_string50_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-zA-Z0-9 ]{1,50}")
        .unwrap()
        .prop_filter("non-empty", |s| !s.is_empty())
}

/// Strategy for valid email addresses
fn valid_email_strategy() -> impl Strategy<Value = String> {
    (
        proptest::string::string_regex("[a-zA-Z0-9]{1,10}").unwrap(),
        proptest::string::string_regex("[a-zA-Z0-9]{1,10}").unwrap(),
        proptest::string::string_regex("[a-zA-Z]{2,4}").unwrap(),
    )
        .prop_map(|(local, domain, tld)| format!("{local}@{domain}.{tld}"))
}

/// Strategy for valid zip codes (5 digits)
fn valid_zip_code_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[0-9]{5}").unwrap()
}

/// Strategy for valid state codes
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

/// Strategy for AddressDto
fn address_dto_strategy() -> impl Strategy<Value = AddressDto> {
    (
        valid_string50_strategy(), // address_line1
        prop_oneof![Just(String::new()), valid_string50_strategy()], // address_line2
        Just(String::new()),       // address_line3 (simplified)
        Just(String::new()),       // address_line4 (simplified)
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

/// Strategy for CustomerInfoDto
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

/// Strategy for Widget product codes
fn widget_code_strategy() -> impl Strategy<Value = String> {
    (0u32..10000u32).prop_map(|v| format!("W{v:04}"))
}

/// Strategy for Gizmo product codes
fn gizmo_code_strategy() -> impl Strategy<Value = String> {
    (0u32..1000u32).prop_map(|v| format!("G{v:03}"))
}

/// Strategy for OrderFormLineDto
fn order_form_line_dto_strategy() -> impl Strategy<Value = OrderFormLineDto> {
    (
        valid_string50_strategy(),
        prop_oneof![widget_code_strategy(), gizmo_code_strategy()],
        // Widget uses integer quantity, Gizmo uses decimal, but simplified here
        (1u32..100u32).prop_map(Decimal::from),
    )
        .prop_map(|(line_id, product_code, quantity)| OrderFormLineDto {
            order_line_id: line_id,
            product_code,
            quantity,
        })
}

/// Strategy for OrderFormDto
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
// Address round-trip conversion tests
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// AddressDto -> UnvalidatedAddress -> CheckedAddress -> Address -> AddressDto
    /// Verifies the round-trip equals the original DTO
    #[test]
    fn test_address_dto_roundtrip(dto in address_dto_strategy()) {
        // DTO -> UnvalidatedAddress
        let unvalidated = dto.to_unvalidated_address();

        // UnvalidatedAddress -> CheckedAddress -> Address
        let checked = CheckedAddress::new(unvalidated);
        let address_result = to_address(&checked);

        // Only verify round-trip when conversion to Address succeeds
        if let Ok(address) = address_result {
            // Address -> AddressDto
            let roundtrip_dto = AddressDto::from_address(&address);

            // Compare with original DTO
            prop_assert_eq!(dto.address_line1, roundtrip_dto.address_line1);
            prop_assert_eq!(dto.city, roundtrip_dto.city);
            prop_assert_eq!(dto.zip_code, roundtrip_dto.zip_code);
            prop_assert_eq!(dto.state, roundtrip_dto.state);
            prop_assert_eq!(dto.country, roundtrip_dto.country);

            // Optional lines round-trip as empty strings, so verify value equality
            prop_assert_eq!(dto.address_line2, roundtrip_dto.address_line2);
            prop_assert_eq!(dto.address_line3, roundtrip_dto.address_line3);
            prop_assert_eq!(dto.address_line4, roundtrip_dto.address_line4);
        }
    }

    /// Verifies the DTO round-trip matches exactly after Address creation
    /// (Starting from a valid Address created with Address::create)
    #[test]
    fn test_address_domain_to_dto_roundtrip(dto in address_dto_strategy()) {
        // First create an Address
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

            // Compare domain values
            prop_assert_eq!(address.address_line1().value(), roundtrip.address_line1().value());
            prop_assert_eq!(address.city().value(), roundtrip.city().value());
            prop_assert_eq!(address.zip_code().value(), roundtrip.zip_code().value());
            prop_assert_eq!(address.state().value(), roundtrip.state().value());
        }
    }
}

// =============================================================================
// CustomerInfo round-trip conversion tests (note: considering VipStatus normalization)
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Round-trip conversion test for CustomerInfoDto
    /// VipStatus is normalized to "Normal" or "VIP"
    #[test]
    fn test_customer_info_dto_roundtrip(dto in customer_info_dto_strategy()) {
        // DTO -> UnvalidatedCustomerInfo
        let unvalidated = dto.to_unvalidated_customer_info();

        // Verify field round-trip
        prop_assert_eq!(dto.first_name.as_str(), unvalidated.first_name());
        prop_assert_eq!(dto.last_name.as_str(), unvalidated.last_name());
        prop_assert_eq!(dto.email_address.as_str(), unvalidated.email_address());
        prop_assert_eq!(dto.vip_status.as_str(), unvalidated.vip_status());
    }
}

// =============================================================================
// OrderFormDto JSON round-trip tests
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// JSON serialization/deserialization round-trip for OrderFormDto
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

        // Compare with original DTO
        prop_assert_eq!(dto, deserialized, "Roundtrip should preserve all data");
    }

    /// JSON round-trip for AddressDto
    #[test]
    fn test_address_dto_json_roundtrip(dto in address_dto_strategy()) {
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: AddressDto = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(dto, deserialized);
    }

    /// JSON round-trip for CustomerInfoDto
    #[test]
    fn test_customer_info_dto_json_roundtrip(dto in customer_info_dto_strategy()) {
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: CustomerInfoDto = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(dto, deserialized);
    }

    /// JSON round-trip for OrderFormLineDto
    #[test]
    fn test_order_form_line_dto_json_roundtrip(dto in order_form_line_dto_strategy()) {
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: OrderFormLineDto = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(dto, deserialized);
    }
}

// =============================================================================
// Output DTOs JSON round-trip tests
// =============================================================================

/// Strategy for ShippableOrderLineDto
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

/// Strategy for ShippableOrderPlacedDto
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

/// Strategy for BillableOrderPlacedDto
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

/// Strategy for OrderAcknowledgmentSentDto
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

    /// JSON round-trip for ShippableOrderLineDto
    #[test]
    fn test_shippable_order_line_dto_json_roundtrip(dto in shippable_order_line_dto_strategy()) {
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: ShippableOrderLineDto = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(dto, deserialized);
    }

    /// JSON round-trip for ShippableOrderPlacedDto
    #[test]
    fn test_shippable_order_placed_dto_json_roundtrip(dto in shippable_order_placed_dto_strategy()) {
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: ShippableOrderPlacedDto = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(dto, deserialized);
    }

    /// JSON round-trip for BillableOrderPlacedDto
    #[test]
    fn test_billable_order_placed_dto_json_roundtrip(dto in billable_order_placed_dto_strategy()) {
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: BillableOrderPlacedDto = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(dto, deserialized);
    }

    /// JSON round-trip for OrderAcknowledgmentSentDto
    #[test]
    fn test_order_acknowledgment_sent_dto_json_roundtrip(dto in order_acknowledgment_sent_dto_strategy()) {
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: OrderAcknowledgmentSentDto = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(dto, deserialized);
    }
}

// =============================================================================
// Conversion purity tests
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Same input returns same output (referential transparency)
    #[test]
    fn test_address_dto_conversion_is_pure(dto in address_dto_strategy()) {
        let unvalidated1 = dto.to_unvalidated_address();
        let unvalidated2 = dto.to_unvalidated_address();

        // Same input produces same output
        prop_assert_eq!(unvalidated1.address_line1(), unvalidated2.address_line1());
        prop_assert_eq!(unvalidated1.city(), unvalidated2.city());
        prop_assert_eq!(unvalidated1.zip_code(), unvalidated2.zip_code());
    }

    /// Purity of CustomerInfoDto conversion
    #[test]
    fn test_customer_info_dto_conversion_is_pure(dto in customer_info_dto_strategy()) {
        let unvalidated1 = dto.to_unvalidated_customer_info();
        let unvalidated2 = dto.to_unvalidated_customer_info();

        prop_assert_eq!(unvalidated1.first_name(), unvalidated2.first_name());
        prop_assert_eq!(unvalidated1.last_name(), unvalidated2.last_name());
        prop_assert_eq!(unvalidated1.email_address(), unvalidated2.email_address());
        prop_assert_eq!(unvalidated1.vip_status(), unvalidated2.vip_status());
    }

    /// Purity of OrderFormDto conversion
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
// Round-trip tests at boundary values
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Round-trip of DTO containing maximum-length strings
    #[test]
    fn test_max_length_string_roundtrip(
        _dummy in any::<u8>()
    ) {
        // Maximum-length string of 50 characters
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

    /// Round-trip of empty optional fields
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

        // Conversion to Address
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

        // Round-trip
        let roundtrip_dto = AddressDto::from_address(&address);

        // Optional fields are preserved as empty strings
        prop_assert_eq!(dto.address_line2, roundtrip_dto.address_line2);
        prop_assert_eq!(dto.address_line3, roundtrip_dto.address_line3);
        prop_assert_eq!(dto.address_line4, roundtrip_dto.address_line4);
    }
}

// =============================================================================
// Decimal precision preservation tests
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Verifies Decimal values maintain precision through JSON round-trip
    #[test]
    fn test_decimal_precision_preserved_in_json(
        integer_part in 0u32..1000u32,
        decimal_part in 0u32..100u32
    ) {
        // Create a Decimal with up to 2 decimal places
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

    /// Billing amount precision preservation
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
