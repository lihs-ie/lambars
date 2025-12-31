//! 入力 DTO のテスト
//!
//! CustomerInfoDto, AddressDto, OrderFormLineDto, OrderFormDto のテスト

use order_taking_sample::compound_types::Address;
use order_taking_sample::dto::{AddressDto, CustomerInfoDto, OrderFormDto, OrderFormLineDto};
use rstest::rstest;
use rust_decimal::Decimal;
use std::str::FromStr;

// =============================================================================
// CustomerInfoDto のテスト
// =============================================================================

mod customer_info_dto_tests {
    use super::*;

    #[rstest]
    fn test_deserialize_customer_info_dto() {
        let json = r#"{
            "first_name": "John",
            "last_name": "Doe",
            "email_address": "john@example.com",
            "vip_status": "Normal"
        }"#;

        let dto: CustomerInfoDto = serde_json::from_str(json).unwrap();

        assert_eq!(dto.first_name, "John");
        assert_eq!(dto.last_name, "Doe");
        assert_eq!(dto.email_address, "john@example.com");
        assert_eq!(dto.vip_status, "Normal");
    }

    #[rstest]
    fn test_serialize_customer_info_dto() {
        let dto = CustomerInfoDto {
            first_name: "Jane".to_string(),
            last_name: "Smith".to_string(),
            email_address: "jane@example.com".to_string(),
            vip_status: "VIP".to_string(),
        };

        let json = serde_json::to_string(&dto).unwrap();

        assert!(json.contains("\"first_name\":\"Jane\""));
        assert!(json.contains("\"last_name\":\"Smith\""));
        assert!(json.contains("\"email_address\":\"jane@example.com\""));
        assert!(json.contains("\"vip_status\":\"VIP\""));
    }

    #[rstest]
    fn test_to_unvalidated_customer_info() {
        let dto = CustomerInfoDto {
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            email_address: "john@example.com".to_string(),
            vip_status: "Normal".to_string(),
        };

        let unvalidated = dto.to_unvalidated_customer_info();

        assert_eq!(unvalidated.first_name(), "John");
        assert_eq!(unvalidated.last_name(), "Doe");
        assert_eq!(unvalidated.email_address(), "john@example.com");
        assert_eq!(unvalidated.vip_status(), "Normal");
    }

    #[rstest]
    fn test_customer_info_dto_clone() {
        let dto1 = CustomerInfoDto {
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            email_address: "john@example.com".to_string(),
            vip_status: "Normal".to_string(),
        };
        let dto2 = dto1.clone();

        assert_eq!(dto1, dto2);
    }
}

// =============================================================================
// AddressDto のテスト
// =============================================================================

mod address_dto_tests {
    use super::*;

    #[rstest]
    fn test_deserialize_address_dto() {
        let json = r#"{
            "address_line1": "123 Main St",
            "address_line2": "Apt 4B",
            "address_line3": "",
            "address_line4": "",
            "city": "New York",
            "zip_code": "10001",
            "state": "NY",
            "country": "USA"
        }"#;

        let dto: AddressDto = serde_json::from_str(json).unwrap();

        assert_eq!(dto.address_line1, "123 Main St");
        assert_eq!(dto.address_line2, "Apt 4B");
        assert_eq!(dto.address_line3, "");
        assert_eq!(dto.address_line4, "");
        assert_eq!(dto.city, "New York");
        assert_eq!(dto.zip_code, "10001");
        assert_eq!(dto.state, "NY");
        assert_eq!(dto.country, "USA");
    }

    #[rstest]
    fn test_serialize_address_dto() {
        let dto = AddressDto {
            address_line1: "456 Oak Ave".to_string(),
            address_line2: "".to_string(),
            address_line3: "".to_string(),
            address_line4: "".to_string(),
            city: "Los Angeles".to_string(),
            zip_code: "90001".to_string(),
            state: "CA".to_string(),
            country: "USA".to_string(),
        };

        let json = serde_json::to_string(&dto).unwrap();

        assert!(json.contains("\"address_line1\":\"456 Oak Ave\""));
        assert!(json.contains("\"city\":\"Los Angeles\""));
    }

    #[rstest]
    fn test_to_unvalidated_address() {
        let dto = AddressDto {
            address_line1: "123 Main St".to_string(),
            address_line2: "Apt 4B".to_string(),
            address_line3: "".to_string(),
            address_line4: "".to_string(),
            city: "New York".to_string(),
            zip_code: "10001".to_string(),
            state: "NY".to_string(),
            country: "USA".to_string(),
        };

        let unvalidated = dto.to_unvalidated_address();

        assert_eq!(unvalidated.address_line1(), "123 Main St");
        assert_eq!(unvalidated.address_line2(), "Apt 4B");
        assert_eq!(unvalidated.city(), "New York");
        assert_eq!(unvalidated.zip_code(), "10001");
        assert_eq!(unvalidated.state(), "NY");
        assert_eq!(unvalidated.country(), "USA");
    }

    #[rstest]
    fn test_from_address() {
        let address = Address::create(
            "123 Main St",
            "Apt 4B",
            "",
            "",
            "New York",
            "10001",
            "NY",
            "USA",
        )
        .unwrap();

        let dto = AddressDto::from_address(&address);

        assert_eq!(dto.address_line1, "123 Main St");
        assert_eq!(dto.address_line2, "Apt 4B");
        assert_eq!(dto.address_line3, "");
        assert_eq!(dto.address_line4, "");
        assert_eq!(dto.city, "New York");
        assert_eq!(dto.zip_code, "10001");
        assert_eq!(dto.state, "NY");
        assert_eq!(dto.country, "USA");
    }

    #[rstest]
    fn test_from_address_with_optional_lines() {
        // 全てのオプショナル行が None の場合
        let address = Address::create(
            "456 Oak Ave",
            "",
            "",
            "",
            "Los Angeles",
            "90001",
            "CA",
            "USA",
        )
        .unwrap();

        let dto = AddressDto::from_address(&address);

        assert_eq!(dto.address_line1, "456 Oak Ave");
        assert_eq!(dto.address_line2, "");
        assert_eq!(dto.address_line3, "");
        assert_eq!(dto.address_line4, "");
    }

    #[rstest]
    fn test_address_dto_clone() {
        let dto1 = AddressDto {
            address_line1: "123 Main St".to_string(),
            address_line2: "".to_string(),
            address_line3: "".to_string(),
            address_line4: "".to_string(),
            city: "New York".to_string(),
            zip_code: "10001".to_string(),
            state: "NY".to_string(),
            country: "USA".to_string(),
        };
        let dto2 = dto1.clone();

        assert_eq!(dto1, dto2);
    }
}

// =============================================================================
// OrderFormLineDto のテスト
// =============================================================================

mod order_form_line_dto_tests {
    use super::*;

    #[rstest]
    fn test_deserialize_order_form_line_dto() {
        let json = r#"{
            "order_line_id": "line-001",
            "product_code": "W1234",
            "quantity": "10"
        }"#;

        let dto: OrderFormLineDto = serde_json::from_str(json).unwrap();

        assert_eq!(dto.order_line_id, "line-001");
        assert_eq!(dto.product_code, "W1234");
        assert_eq!(dto.quantity, Decimal::from(10));
    }

    #[rstest]
    fn test_serialize_order_form_line_dto() {
        let dto = OrderFormLineDto {
            order_line_id: "line-002".to_string(),
            product_code: "G123".to_string(),
            quantity: Decimal::from_str("2.5").unwrap(),
        };

        let json = serde_json::to_string(&dto).unwrap();

        assert!(json.contains("\"order_line_id\":\"line-002\""));
        assert!(json.contains("\"product_code\":\"G123\""));
        assert!(json.contains("\"quantity\":\"2.5\""));
    }

    #[rstest]
    fn test_to_unvalidated_order_line() {
        let dto = OrderFormLineDto {
            order_line_id: "line-001".to_string(),
            product_code: "W1234".to_string(),
            quantity: Decimal::from(10),
        };

        let unvalidated = dto.to_unvalidated_order_line();

        assert_eq!(unvalidated.order_line_id(), "line-001");
        assert_eq!(unvalidated.product_code(), "W1234");
        assert_eq!(unvalidated.quantity(), Decimal::from(10));
    }

    #[rstest]
    fn test_quantity_decimal_handling() {
        // 小数点を含む数量
        let json = r#"{
            "order_line_id": "line-003",
            "product_code": "G456",
            "quantity": "10.5"
        }"#;

        let dto: OrderFormLineDto = serde_json::from_str(json).unwrap();

        assert_eq!(dto.quantity, Decimal::from_str("10.5").unwrap());
    }

    #[rstest]
    fn test_order_form_line_dto_clone() {
        let dto1 = OrderFormLineDto {
            order_line_id: "line-001".to_string(),
            product_code: "W1234".to_string(),
            quantity: Decimal::from(10),
        };
        let dto2 = dto1.clone();

        assert_eq!(dto1, dto2);
    }
}

// =============================================================================
// OrderFormDto のテスト
// =============================================================================

mod order_form_dto_tests {
    use super::*;

    fn create_valid_order_form_json() -> &'static str {
        r#"{
            "order_id": "order-001",
            "customer_info": {
                "first_name": "John",
                "last_name": "Doe",
                "email_address": "john@example.com",
                "vip_status": "Normal"
            },
            "shipping_address": {
                "address_line1": "123 Main St",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "New York",
                "zip_code": "10001",
                "state": "NY",
                "country": "USA"
            },
            "billing_address": {
                "address_line1": "456 Oak Ave",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Los Angeles",
                "zip_code": "90001",
                "state": "CA",
                "country": "USA"
            },
            "lines": [
                {
                    "order_line_id": "line-001",
                    "product_code": "W1234",
                    "quantity": "10"
                }
            ],
            "promotion_code": ""
        }"#
    }

    #[rstest]
    fn test_deserialize_order_form_dto() {
        let json = create_valid_order_form_json();

        let dto: OrderFormDto = serde_json::from_str(json).unwrap();

        assert_eq!(dto.order_id, "order-001");
        assert_eq!(dto.customer_info.first_name, "John");
        assert_eq!(dto.shipping_address.city, "New York");
        assert_eq!(dto.billing_address.city, "Los Angeles");
        assert_eq!(dto.lines.len(), 1);
        assert_eq!(dto.promotion_code, "");
    }

    #[rstest]
    fn test_serialize_order_form_dto() {
        let dto = OrderFormDto {
            order_id: "order-001".to_string(),
            customer_info: CustomerInfoDto {
                first_name: "John".to_string(),
                last_name: "Doe".to_string(),
                email_address: "john@example.com".to_string(),
                vip_status: "Normal".to_string(),
            },
            shipping_address: AddressDto {
                address_line1: "123 Main St".to_string(),
                address_line2: "".to_string(),
                address_line3: "".to_string(),
                address_line4: "".to_string(),
                city: "New York".to_string(),
                zip_code: "10001".to_string(),
                state: "NY".to_string(),
                country: "USA".to_string(),
            },
            billing_address: AddressDto {
                address_line1: "456 Oak Ave".to_string(),
                address_line2: "".to_string(),
                address_line3: "".to_string(),
                address_line4: "".to_string(),
                city: "Los Angeles".to_string(),
                zip_code: "90001".to_string(),
                state: "CA".to_string(),
                country: "USA".to_string(),
            },
            lines: vec![OrderFormLineDto {
                order_line_id: "line-001".to_string(),
                product_code: "W1234".to_string(),
                quantity: Decimal::from(10),
            }],
            promotion_code: "PROMO2024".to_string(),
        };

        let json = serde_json::to_string(&dto).unwrap();

        assert!(json.contains("\"order_id\":\"order-001\""));
        assert!(json.contains("\"promotion_code\":\"PROMO2024\""));
    }

    #[rstest]
    fn test_to_unvalidated_order() {
        let dto = OrderFormDto {
            order_id: "order-001".to_string(),
            customer_info: CustomerInfoDto {
                first_name: "John".to_string(),
                last_name: "Doe".to_string(),
                email_address: "john@example.com".to_string(),
                vip_status: "Normal".to_string(),
            },
            shipping_address: AddressDto {
                address_line1: "123 Main St".to_string(),
                address_line2: "".to_string(),
                address_line3: "".to_string(),
                address_line4: "".to_string(),
                city: "New York".to_string(),
                zip_code: "10001".to_string(),
                state: "NY".to_string(),
                country: "USA".to_string(),
            },
            billing_address: AddressDto {
                address_line1: "456 Oak Ave".to_string(),
                address_line2: "".to_string(),
                address_line3: "".to_string(),
                address_line4: "".to_string(),
                city: "Los Angeles".to_string(),
                zip_code: "90001".to_string(),
                state: "CA".to_string(),
                country: "USA".to_string(),
            },
            lines: vec![OrderFormLineDto {
                order_line_id: "line-001".to_string(),
                product_code: "W1234".to_string(),
                quantity: Decimal::from(10),
            }],
            promotion_code: "PROMO2024".to_string(),
        };

        let unvalidated = dto.to_unvalidated_order();

        assert_eq!(unvalidated.order_id(), "order-001");
        assert_eq!(unvalidated.customer_info().first_name(), "John");
        assert_eq!(unvalidated.shipping_address().city(), "New York");
        assert_eq!(unvalidated.billing_address().city(), "Los Angeles");
        assert_eq!(unvalidated.lines().len(), 1);
        assert_eq!(unvalidated.promotion_code(), "PROMO2024");
    }

    #[rstest]
    fn test_nested_dto_conversion() {
        let dto = OrderFormDto {
            order_id: "order-002".to_string(),
            customer_info: CustomerInfoDto {
                first_name: "Jane".to_string(),
                last_name: "Smith".to_string(),
                email_address: "jane@example.com".to_string(),
                vip_status: "VIP".to_string(),
            },
            shipping_address: AddressDto {
                address_line1: "789 Pine Rd".to_string(),
                address_line2: "Suite 100".to_string(),
                address_line3: "".to_string(),
                address_line4: "".to_string(),
                city: "Chicago".to_string(),
                zip_code: "60601".to_string(),
                state: "IL".to_string(),
                country: "USA".to_string(),
            },
            billing_address: AddressDto {
                address_line1: "789 Pine Rd".to_string(),
                address_line2: "Suite 100".to_string(),
                address_line3: "".to_string(),
                address_line4: "".to_string(),
                city: "Chicago".to_string(),
                zip_code: "60601".to_string(),
                state: "IL".to_string(),
                country: "USA".to_string(),
            },
            lines: vec![
                OrderFormLineDto {
                    order_line_id: "line-001".to_string(),
                    product_code: "W1234".to_string(),
                    quantity: Decimal::from(5),
                },
                OrderFormLineDto {
                    order_line_id: "line-002".to_string(),
                    product_code: "G123".to_string(),
                    quantity: Decimal::from_str("2.5").unwrap(),
                },
            ],
            promotion_code: "".to_string(),
        };

        let unvalidated = dto.to_unvalidated_order();

        // 複数の明細が正しく変換されていることを確認
        assert_eq!(unvalidated.lines().len(), 2);
        assert_eq!(unvalidated.lines()[0].product_code(), "W1234");
        assert_eq!(unvalidated.lines()[1].product_code(), "G123");
        assert_eq!(
            unvalidated.lines()[1].quantity(),
            Decimal::from_str("2.5").unwrap()
        );
    }

    #[rstest]
    fn test_order_form_dto_clone() {
        let dto1 = OrderFormDto {
            order_id: "order-001".to_string(),
            customer_info: CustomerInfoDto {
                first_name: "John".to_string(),
                last_name: "Doe".to_string(),
                email_address: "john@example.com".to_string(),
                vip_status: "Normal".to_string(),
            },
            shipping_address: AddressDto {
                address_line1: "123 Main St".to_string(),
                address_line2: "".to_string(),
                address_line3: "".to_string(),
                address_line4: "".to_string(),
                city: "New York".to_string(),
                zip_code: "10001".to_string(),
                state: "NY".to_string(),
                country: "USA".to_string(),
            },
            billing_address: AddressDto {
                address_line1: "123 Main St".to_string(),
                address_line2: "".to_string(),
                address_line3: "".to_string(),
                address_line4: "".to_string(),
                city: "New York".to_string(),
                zip_code: "10001".to_string(),
                state: "NY".to_string(),
                country: "USA".to_string(),
            },
            lines: vec![],
            promotion_code: "".to_string(),
        };
        let dto2 = dto1.clone();

        assert_eq!(dto1, dto2);
    }
}
