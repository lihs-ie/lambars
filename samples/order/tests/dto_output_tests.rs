//! Tests for Output DTOs
//!
//! ShippableOrderLineDto, ShippableOrderPlacedDto, BillableOrderPlacedDto,
//! OrderAcknowledgmentSentDto, PricedOrderProductLineDto, PricedOrderLineDto,
//! Tests for PlaceOrderEventDto

use order_taking_sample::compound_types::Address;
use order_taking_sample::dto::{
    BillableOrderPlacedDto, OrderAcknowledgmentSentDto, PlaceOrderEventDto, PricedOrderLineDto,
    PricedOrderProductLineDto, ShippableOrderLineDto, ShippableOrderPlacedDto,
};
use order_taking_sample::simple_types::{
    BillingAmount, EmailAddress, OrderId, OrderLineId, OrderQuantity, PdfAttachment, Price,
    ProductCode,
};
use order_taking_sample::workflow::{
    BillableOrderPlaced, OrderAcknowledgmentSent, PlaceOrderEvent, PricedOrderLine,
    PricedOrderProductLine, ShippableOrderLine, ShippableOrderPlaced,
};
use rstest::rstest;
use rust_decimal::Decimal;
use std::str::FromStr;

// =============================================================================
// Tests for ShippableOrderLineDto
// =============================================================================

mod shippable_order_line_dto_tests {
    use super::*;

    fn create_shippable_order_line() -> ShippableOrderLine {
        let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let quantity = OrderQuantity::create("Quantity", &product_code, Decimal::from(10)).unwrap();
        ShippableOrderLine::new(product_code, quantity)
    }

    #[rstest]
    fn test_from_domain() {
        let line = create_shippable_order_line();
        let dto = ShippableOrderLineDto::from_domain(&line);

        assert_eq!(dto.product_code, "W1234");
        assert_eq!(dto.quantity, Decimal::from(10));
    }

    #[rstest]
    fn test_serialize() {
        let line = create_shippable_order_line();
        let dto = ShippableOrderLineDto::from_domain(&line);

        let json = serde_json::to_string(&dto).unwrap();

        assert!(json.contains("\"product_code\":\"W1234\""));
        assert!(json.contains("\"quantity\":\"10\""));
    }

    #[rstest]
    fn test_deserialize() {
        let json = r#"{
            "product_code": "G123",
            "quantity": "2.5"
        }"#;

        let dto: ShippableOrderLineDto = serde_json::from_str(json).unwrap();

        assert_eq!(dto.product_code, "G123");
        assert_eq!(dto.quantity, Decimal::from_str("2.5").unwrap());
    }

    #[rstest]
    fn test_clone() {
        let line = create_shippable_order_line();
        let dto1 = ShippableOrderLineDto::from_domain(&line);
        let dto2 = dto1.clone();

        assert_eq!(dto1, dto2);
    }
}

// =============================================================================
// Tests for ShippableOrderPlacedDto
// =============================================================================

mod shippable_order_placed_dto_tests {
    use super::*;

    fn create_shippable_order_placed() -> ShippableOrderPlaced {
        let order_id = OrderId::create("OrderId", "order-001").unwrap();
        let address =
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
        let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let quantity = OrderQuantity::create("Quantity", &product_code, Decimal::from(5)).unwrap();
        let lines = vec![ShippableOrderLine::new(product_code, quantity)];
        let pdf = PdfAttachment::new("label.pdf".to_string(), vec![1, 2, 3, 4, 5]);

        ShippableOrderPlaced::new(order_id, address, lines, pdf)
    }

    #[rstest]
    fn test_from_domain() {
        let event = create_shippable_order_placed();
        let dto = ShippableOrderPlacedDto::from_domain(&event);

        assert_eq!(dto.order_id, "order-001");
        assert_eq!(dto.shipping_address.city, "New York");
        assert_eq!(dto.shipment_lines.len(), 1);
        assert_eq!(dto.pdf_name, "label.pdf");
        // Base64-encoded data
        assert_eq!(dto.pdf_data, "AQIDBAU=");
    }

    #[rstest]
    fn test_serialize() {
        let event = create_shippable_order_placed();
        let dto = ShippableOrderPlacedDto::from_domain(&event);

        let json = serde_json::to_string(&dto).unwrap();

        assert!(json.contains("\"order_id\":\"order-001\""));
        assert!(json.contains("\"pdf_name\":\"label.pdf\""));
        assert!(json.contains("\"pdf_data\":\"AQIDBAU=\""));
    }

    #[rstest]
    fn test_deserialize() {
        let json = r#"{
            "order_id": "order-002",
            "shipping_address": {
                "address_line1": "456 Oak Ave",
                "address_line2": "",
                "address_line3": "",
                "address_line4": "",
                "city": "Los Angeles",
                "zip_code": "90001",
                "state": "CA",
                "country": "USA"
            },
            "shipment_lines": [],
            "pdf_name": "test.pdf",
            "pdf_data": "dGVzdA=="
        }"#;

        let dto: ShippableOrderPlacedDto = serde_json::from_str(json).unwrap();

        assert_eq!(dto.order_id, "order-002");
        assert_eq!(dto.shipping_address.city, "Los Angeles");
        assert_eq!(dto.pdf_name, "test.pdf");
        assert_eq!(dto.pdf_data, "dGVzdA==");
    }

    #[rstest]
    fn test_clone() {
        let event = create_shippable_order_placed();
        let dto1 = ShippableOrderPlacedDto::from_domain(&event);
        let dto2 = dto1.clone();

        assert_eq!(dto1, dto2);
    }
}

// =============================================================================
// Tests for BillableOrderPlacedDto
// =============================================================================

mod billable_order_placed_dto_tests {
    use super::*;

    fn create_billable_order_placed() -> BillableOrderPlaced {
        let order_id = OrderId::create("OrderId", "order-001").unwrap();
        let address =
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
        let amount = BillingAmount::create(Decimal::from(1000)).unwrap();

        BillableOrderPlaced::new(order_id, address, amount)
    }

    #[rstest]
    fn test_from_domain() {
        let event = create_billable_order_placed();
        let dto = BillableOrderPlacedDto::from_domain(&event);

        assert_eq!(dto.order_id, "order-001");
        assert_eq!(dto.billing_address.city, "New York");
        assert_eq!(dto.amount_to_bill, Decimal::from(1000));
    }

    #[rstest]
    fn test_serialize() {
        let event = create_billable_order_placed();
        let dto = BillableOrderPlacedDto::from_domain(&event);

        let json = serde_json::to_string(&dto).unwrap();

        assert!(json.contains("\"order_id\":\"order-001\""));
        assert!(json.contains("\"amount_to_bill\":\"1000\""));
    }

    #[rstest]
    fn test_deserialize() {
        let json = r#"{
            "order_id": "order-002",
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
            "amount_to_bill": "500.50"
        }"#;

        let dto: BillableOrderPlacedDto = serde_json::from_str(json).unwrap();

        assert_eq!(dto.order_id, "order-002");
        assert_eq!(dto.amount_to_bill, Decimal::from_str("500.50").unwrap());
    }

    #[rstest]
    fn test_clone() {
        let event = create_billable_order_placed();
        let dto1 = BillableOrderPlacedDto::from_domain(&event);
        let dto2 = dto1.clone();

        assert_eq!(dto1, dto2);
    }
}

// =============================================================================
// Tests for OrderAcknowledgmentSentDto
// =============================================================================

mod order_acknowledgment_sent_dto_tests {
    use super::*;

    fn create_order_acknowledgment_sent() -> OrderAcknowledgmentSent {
        let order_id = OrderId::create("OrderId", "order-001").unwrap();
        let email = EmailAddress::create("EmailAddress", "john@example.com").unwrap();

        OrderAcknowledgmentSent::new(order_id, email)
    }

    #[rstest]
    fn test_from_domain() {
        let event = create_order_acknowledgment_sent();
        let dto = OrderAcknowledgmentSentDto::from_domain(&event);

        assert_eq!(dto.order_id, "order-001");
        assert_eq!(dto.email_address, "john@example.com");
    }

    #[rstest]
    fn test_serialize() {
        let event = create_order_acknowledgment_sent();
        let dto = OrderAcknowledgmentSentDto::from_domain(&event);

        let json = serde_json::to_string(&dto).unwrap();

        assert!(json.contains("\"order_id\":\"order-001\""));
        assert!(json.contains("\"email_address\":\"john@example.com\""));
    }

    #[rstest]
    fn test_deserialize() {
        let json = r#"{
            "order_id": "order-002",
            "email_address": "jane@example.com"
        }"#;

        let dto: OrderAcknowledgmentSentDto = serde_json::from_str(json).unwrap();

        assert_eq!(dto.order_id, "order-002");
        assert_eq!(dto.email_address, "jane@example.com");
    }

    #[rstest]
    fn test_clone() {
        let event = create_order_acknowledgment_sent();
        let dto1 = OrderAcknowledgmentSentDto::from_domain(&event);
        let dto2 = dto1.clone();

        assert_eq!(dto1, dto2);
    }
}

// =============================================================================
// Tests for PricedOrderProductLineDto
// =============================================================================

mod priced_order_product_line_dto_tests {
    use super::*;

    fn create_priced_order_product_line() -> PricedOrderProductLine {
        let order_line_id = OrderLineId::create("OrderLineId", "line-001").unwrap();
        let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let quantity = OrderQuantity::create("Quantity", &product_code, Decimal::from(5)).unwrap();
        let line_price = Price::create(Decimal::from(500)).unwrap();

        PricedOrderProductLine::new(order_line_id, product_code, quantity, line_price)
    }

    #[rstest]
    fn test_from_domain() {
        let line = create_priced_order_product_line();
        let dto = PricedOrderProductLineDto::from_domain(&line);

        assert_eq!(dto.order_line_id, "line-001");
        assert_eq!(dto.product_code, "W1234");
        assert_eq!(dto.quantity, Decimal::from(5));
        assert_eq!(dto.line_price, Decimal::from(500));
    }

    #[rstest]
    fn test_serialize() {
        let line = create_priced_order_product_line();
        let dto = PricedOrderProductLineDto::from_domain(&line);

        let json = serde_json::to_string(&dto).unwrap();

        assert!(json.contains("\"order_line_id\":\"line-001\""));
        assert!(json.contains("\"product_code\":\"W1234\""));
        assert!(json.contains("\"quantity\":\"5\""));
        assert!(json.contains("\"line_price\":\"500\""));
    }

    #[rstest]
    fn test_deserialize() {
        let json = r#"{
            "order_line_id": "line-002",
            "product_code": "G123",
            "quantity": "2.5",
            "line_price": "250.00"
        }"#;

        let dto: PricedOrderProductLineDto = serde_json::from_str(json).unwrap();

        assert_eq!(dto.order_line_id, "line-002");
        assert_eq!(dto.product_code, "G123");
        assert_eq!(dto.quantity, Decimal::from_str("2.5").unwrap());
        assert_eq!(dto.line_price, Decimal::from_str("250.00").unwrap());
    }

    #[rstest]
    fn test_clone() {
        let line = create_priced_order_product_line();
        let dto1 = PricedOrderProductLineDto::from_domain(&line);
        let dto2 = dto1.clone();

        assert_eq!(dto1, dto2);
    }
}

// =============================================================================
// Tests for PricedOrderLineDto
// =============================================================================

mod priced_order_line_dto_tests {
    use super::*;

    fn create_product_line() -> PricedOrderLine {
        let order_line_id = OrderLineId::create("OrderLineId", "line-001").unwrap();
        let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let quantity = OrderQuantity::create("Quantity", &product_code, Decimal::from(5)).unwrap();
        let line_price = Price::create(Decimal::from(500)).unwrap();
        let product_line =
            PricedOrderProductLine::new(order_line_id, product_code, quantity, line_price);

        PricedOrderLine::ProductLine(product_line)
    }

    #[rstest]
    fn test_from_domain_product_line() {
        let line = create_product_line();
        let dto = PricedOrderLineDto::from_domain(&line);

        match dto {
            PricedOrderLineDto::ProductLine(product_dto) => {
                assert_eq!(product_dto.order_line_id, "line-001");
                assert_eq!(product_dto.product_code, "W1234");
            }
            PricedOrderLineDto::CommentLine(_) => panic!("Expected ProductLine"),
        }
    }

    #[rstest]
    fn test_from_domain_comment_line() {
        let line = PricedOrderLine::CommentLine("Gift message".to_string());
        let dto = PricedOrderLineDto::from_domain(&line);

        match dto {
            PricedOrderLineDto::ProductLine(_) => panic!("Expected CommentLine"),
            PricedOrderLineDto::CommentLine(comment) => {
                assert_eq!(comment, "Gift message");
            }
        }
    }

    #[rstest]
    fn test_serialize_product_line() {
        let line = create_product_line();
        let dto = PricedOrderLineDto::from_domain(&line);

        let json = serde_json::to_string(&dto).unwrap();

        assert!(json.contains("\"type\":\"ProductLine\""));
        assert!(json.contains("\"data\":{"));
    }

    #[rstest]
    fn test_serialize_comment_line() {
        let line = PricedOrderLine::CommentLine("Thank you!".to_string());
        let dto = PricedOrderLineDto::from_domain(&line);

        let json = serde_json::to_string(&dto).unwrap();

        assert!(json.contains("\"type\":\"CommentLine\""));
        assert!(json.contains("\"data\":\"Thank you!\""));
    }

    #[rstest]
    fn test_deserialize_product_line() {
        let json = r#"{
            "type": "ProductLine",
            "data": {
                "order_line_id": "line-002",
                "product_code": "G123",
                "quantity": "2.5",
                "line_price": "250"
            }
        }"#;

        let dto: PricedOrderLineDto = serde_json::from_str(json).unwrap();

        match dto {
            PricedOrderLineDto::ProductLine(product_dto) => {
                assert_eq!(product_dto.order_line_id, "line-002");
            }
            PricedOrderLineDto::CommentLine(_) => panic!("Expected ProductLine"),
        }
    }

    #[rstest]
    fn test_deserialize_comment_line() {
        let json = r#"{
            "type": "CommentLine",
            "data": "Special instructions"
        }"#;

        let dto: PricedOrderLineDto = serde_json::from_str(json).unwrap();

        match dto {
            PricedOrderLineDto::ProductLine(_) => panic!("Expected CommentLine"),
            PricedOrderLineDto::CommentLine(comment) => {
                assert_eq!(comment, "Special instructions");
            }
        }
    }

    #[rstest]
    fn test_clone() {
        let line = create_product_line();
        let dto1 = PricedOrderLineDto::from_domain(&line);
        let dto2 = dto1.clone();

        assert_eq!(dto1, dto2);
    }
}

// =============================================================================
// Tests for PlaceOrderEventDto
// =============================================================================

mod place_order_event_dto_tests {
    use super::*;

    fn create_shippable_event() -> PlaceOrderEvent {
        let order_id = OrderId::create("OrderId", "order-001").unwrap();
        let address =
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
        let pdf = PdfAttachment::new("label.pdf".to_string(), vec![]);

        PlaceOrderEvent::ShippableOrderPlaced(ShippableOrderPlaced::new(
            order_id,
            address,
            vec![],
            pdf,
        ))
    }

    fn create_billable_event() -> PlaceOrderEvent {
        let order_id = OrderId::create("OrderId", "order-001").unwrap();
        let address =
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
        let amount = BillingAmount::create(Decimal::from(1000)).unwrap();

        PlaceOrderEvent::BillableOrderPlaced(BillableOrderPlaced::new(order_id, address, amount))
    }

    fn create_acknowledgment_event() -> PlaceOrderEvent {
        let order_id = OrderId::create("OrderId", "order-001").unwrap();
        let email = EmailAddress::create("EmailAddress", "john@example.com").unwrap();

        PlaceOrderEvent::AcknowledgmentSent(OrderAcknowledgmentSent::new(order_id, email))
    }

    #[rstest]
    fn test_from_domain_shippable() {
        let event = create_shippable_event();
        let dto = PlaceOrderEventDto::from_domain(&event);

        match dto {
            PlaceOrderEventDto::ShippableOrderPlaced(shippable) => {
                assert_eq!(shippable.order_id, "order-001");
            }
            _ => panic!("Expected ShippableOrderPlaced"),
        }
    }

    #[rstest]
    fn test_from_domain_billable() {
        let event = create_billable_event();
        let dto = PlaceOrderEventDto::from_domain(&event);

        match dto {
            PlaceOrderEventDto::BillableOrderPlaced(billable) => {
                assert_eq!(billable.order_id, "order-001");
                assert_eq!(billable.amount_to_bill, Decimal::from(1000));
            }
            _ => panic!("Expected BillableOrderPlaced"),
        }
    }

    #[rstest]
    fn test_from_domain_acknowledgment() {
        let event = create_acknowledgment_event();
        let dto = PlaceOrderEventDto::from_domain(&event);

        match dto {
            PlaceOrderEventDto::AcknowledgmentSent(ack) => {
                assert_eq!(ack.order_id, "order-001");
                assert_eq!(ack.email_address, "john@example.com");
            }
            _ => panic!("Expected AcknowledgmentSent"),
        }
    }

    #[rstest]
    fn test_from_domain_list() {
        let events = vec![
            create_shippable_event(),
            create_billable_event(),
            create_acknowledgment_event(),
        ];

        let dtos = PlaceOrderEventDto::from_domain_list(&events);

        assert_eq!(dtos.len(), 3);
        assert!(matches!(
            dtos[0],
            PlaceOrderEventDto::ShippableOrderPlaced(_)
        ));
        assert!(matches!(
            dtos[1],
            PlaceOrderEventDto::BillableOrderPlaced(_)
        ));
        assert!(matches!(dtos[2], PlaceOrderEventDto::AcknowledgmentSent(_)));
    }

    #[rstest]
    fn test_serialize_shippable() {
        let event = create_shippable_event();
        let dto = PlaceOrderEventDto::from_domain(&event);

        let json = serde_json::to_string(&dto).unwrap();

        assert!(json.contains("\"type\":\"ShippableOrderPlaced\""));
        assert!(json.contains("\"data\":{"));
    }

    #[rstest]
    fn test_serialize_billable() {
        let event = create_billable_event();
        let dto = PlaceOrderEventDto::from_domain(&event);

        let json = serde_json::to_string(&dto).unwrap();

        assert!(json.contains("\"type\":\"BillableOrderPlaced\""));
    }

    #[rstest]
    fn test_serialize_acknowledgment() {
        let event = create_acknowledgment_event();
        let dto = PlaceOrderEventDto::from_domain(&event);

        let json = serde_json::to_string(&dto).unwrap();

        assert!(json.contains("\"type\":\"AcknowledgmentSent\""));
    }

    #[rstest]
    fn test_clone() {
        let event = create_shippable_event();
        let dto1 = PlaceOrderEventDto::from_domain(&event);
        let dto2 = dto1.clone();

        assert_eq!(dto1, dto2);
    }
}
