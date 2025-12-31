//! イベント生成関数のテスト
//!
//! Phase 7 の events モジュールに対するユニットテスト。

use order_taking_sample::compound_types::{Address, CustomerInfo};
use order_taking_sample::simple_types::{
    BillingAmount, EmailAddress, OrderId, OrderLineId, OrderQuantity, Price, ProductCode,
};
use order_taking_sample::workflow::{
    OrderAcknowledgmentSent, PricedOrder, PricedOrderLine, PricedOrderProductLine, PricingMethod,
};
use rstest::rstest;
use rust_decimal::Decimal;

// =============================================================================
// テストヘルパー関数
// =============================================================================

fn create_test_product_line(
    line_id: &str,
    product_code_str: &str,
    quantity: i32,
    price: Decimal,
) -> PricedOrderLine {
    let order_line_id = OrderLineId::create("OrderLineId", line_id).unwrap();
    let product_code = ProductCode::create("ProductCode", product_code_str).unwrap();
    let qty = OrderQuantity::create("Quantity", &product_code, Decimal::from(quantity)).unwrap();
    let line_price = Price::create(price).unwrap();

    PricedOrderLine::ProductLine(PricedOrderProductLine::new(
        order_line_id,
        product_code,
        qty,
        line_price,
    ))
}

fn create_test_gizmo_product_line(
    line_id: &str,
    product_code_str: &str,
    quantity: Decimal,
    price: Decimal,
) -> PricedOrderLine {
    let order_line_id = OrderLineId::create("OrderLineId", line_id).unwrap();
    let product_code = ProductCode::create("ProductCode", product_code_str).unwrap();
    let qty = OrderQuantity::create("Quantity", &product_code, quantity).unwrap();
    let line_price = Price::create(price).unwrap();

    PricedOrderLine::ProductLine(PricedOrderProductLine::new(
        order_line_id,
        product_code,
        qty,
        line_price,
    ))
}

fn create_test_priced_order(
    order_id: &str,
    amount_to_bill: Decimal,
    lines: Vec<PricedOrderLine>,
) -> PricedOrder {
    let order_id = OrderId::create("OrderId", order_id).unwrap();
    let customer_info = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
    let address =
        Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "US").unwrap();
    let amount = BillingAmount::create(amount_to_bill).unwrap();

    PricedOrder::new(
        order_id,
        customer_info,
        address.clone(),
        address,
        amount,
        lines,
        PricingMethod::Standard,
    )
}

fn create_test_acknowledgment_event(order_id: &str) -> OrderAcknowledgmentSent {
    let order_id = OrderId::create("OrderId", order_id).unwrap();
    let email = EmailAddress::create("EmailAddress", "john@example.com").unwrap();
    OrderAcknowledgmentSent::new(order_id, email)
}

// =============================================================================
// make_shipment_line テスト (REQ-070)
// =============================================================================

mod make_shipment_line_tests {
    use super::*;
    use order_taking_sample::workflow::make_shipment_line;

    #[rstest]
    fn test_product_line_to_shipment_line() {
        // Arrange
        let product_line = create_test_product_line("line-001", "W1234", 5, Decimal::from(500));

        // Act
        let result = make_shipment_line(&product_line);

        // Assert
        assert!(result.is_some());
        let shipment_line = result.unwrap();
        assert_eq!(shipment_line.product_code().value(), "W1234");
    }

    #[rstest]
    fn test_comment_line_returns_none() {
        // Arrange
        let comment_line = PricedOrderLine::CommentLine("Gift message".to_string());

        // Act
        let result = make_shipment_line(&comment_line);

        // Assert
        assert!(result.is_none());
    }

    #[rstest]
    fn test_gizmo_product_line() {
        // Arrange
        let product_line = create_test_gizmo_product_line(
            "line-001",
            "G123",
            Decimal::new(25, 1),
            Decimal::from(250),
        );

        // Act
        let result = make_shipment_line(&product_line);

        // Assert
        assert!(result.is_some());
        let shipment_line = result.unwrap();
        assert_eq!(shipment_line.product_code().value(), "G123");
    }
}

// =============================================================================
// create_shipping_event テスト (REQ-071)
// =============================================================================

mod create_shipping_event_tests {
    use super::*;
    use order_taking_sample::workflow::create_shipping_event;

    #[rstest]
    fn test_single_product_line() {
        // Arrange
        let lines = vec![create_test_product_line(
            "line-001",
            "W1234",
            5,
            Decimal::from(500),
        )];
        let priced_order = create_test_priced_order("order-001", Decimal::from(500), lines);

        // Act
        let event = create_shipping_event(&priced_order);

        // Assert
        assert_eq!(event.order_id().value(), "order-001");
        assert_eq!(event.shipment_lines().len(), 1);
        assert_eq!(event.pdf().name(), "Orderorder-001.pdf");
    }

    #[rstest]
    fn test_multiple_product_lines() {
        // Arrange
        let lines = vec![
            create_test_product_line("line-001", "W1234", 5, Decimal::from(500)),
            create_test_gizmo_product_line(
                "line-002",
                "G123",
                Decimal::new(25, 1),
                Decimal::from(250),
            ),
        ];
        let priced_order = create_test_priced_order("order-002", Decimal::from(750), lines);

        // Act
        let event = create_shipping_event(&priced_order);

        // Assert
        assert_eq!(event.shipment_lines().len(), 2);
    }

    #[rstest]
    fn test_mixed_lines() {
        // Arrange
        let lines = vec![
            create_test_product_line("line-001", "W1234", 5, Decimal::from(500)),
            PricedOrderLine::CommentLine("Gift message".to_string()),
            create_test_gizmo_product_line(
                "line-003",
                "G123",
                Decimal::new(25, 1),
                Decimal::from(250),
            ),
        ];
        let priced_order = create_test_priced_order("order-003", Decimal::from(750), lines);

        // Act
        let event = create_shipping_event(&priced_order);

        // Assert
        // CommentLine は除外される
        assert_eq!(event.shipment_lines().len(), 2);
    }

    #[rstest]
    fn test_comment_only() {
        // Arrange
        let lines = vec![PricedOrderLine::CommentLine(
            "Special instructions".to_string(),
        )];
        let priced_order = create_test_priced_order("order-004", Decimal::ZERO, lines);

        // Act
        let event = create_shipping_event(&priced_order);

        // Assert
        // 空の shipment_lines でも ShippableOrderPlaced は生成される
        assert_eq!(event.shipment_lines().len(), 0);
        assert_eq!(event.order_id().value(), "order-004");
    }

    #[rstest]
    fn test_pdf_name_format() {
        // Arrange
        let priced_order = create_test_priced_order("test-order-123", Decimal::from(100), vec![]);

        // Act
        let event = create_shipping_event(&priced_order);

        // Assert
        assert_eq!(event.pdf().name(), "Ordertest-order-123.pdf");
    }
}

// =============================================================================
// create_billing_event テスト (REQ-072)
// =============================================================================

mod create_billing_event_tests {
    use super::*;
    use order_taking_sample::workflow::create_billing_event;

    #[rstest]
    fn test_positive_amount() {
        // Arrange
        let priced_order = create_test_priced_order("order-001", Decimal::from(1000), vec![]);

        // Act
        let result = create_billing_event(&priced_order);

        // Assert
        assert!(result.is_some());
        let event = result.unwrap();
        assert_eq!(event.order_id().value(), "order-001");
        assert_eq!(event.amount_to_bill().value(), Decimal::from(1000));
    }

    #[rstest]
    fn test_zero_amount() {
        // Arrange
        let priced_order = create_test_priced_order("order-002", Decimal::ZERO, vec![]);

        // Act
        let result = create_billing_event(&priced_order);

        // Assert
        // プロモーションで全額割引の場合など
        assert!(result.is_none());
    }

    #[rstest]
    fn test_small_positive_amount() {
        // Arrange
        let priced_order = create_test_priced_order("order-003", Decimal::new(1, 2), vec![]);

        // Act
        let result = create_billing_event(&priced_order);

        // Assert
        assert!(result.is_some());
        let event = result.unwrap();
        assert_eq!(event.amount_to_bill().value(), Decimal::new(1, 2));
    }
}

// =============================================================================
// create_events テスト (REQ-073)
// =============================================================================

mod create_events_tests {
    use super::*;
    use order_taking_sample::workflow::{PlaceOrderEvent, create_events};

    #[rstest]
    fn test_all_events() {
        // Arrange: 確認メール + 配送 + 請求
        let priced_order = create_test_priced_order("order-001", Decimal::from(1000), vec![]);
        let acknowledgment_event = Some(create_test_acknowledgment_event("order-001"));

        // Act
        let events = create_events(&priced_order, acknowledgment_event);

        // Assert
        assert_eq!(events.len(), 3);
        assert!(events[0].is_acknowledgment());
        assert!(events[1].is_shippable());
        assert!(events[2].is_billable());
    }

    #[rstest]
    fn test_no_acknowledgment() {
        // Arrange: 確認メールなし、請求あり
        let priced_order = create_test_priced_order("order-002", Decimal::from(500), vec![]);
        let acknowledgment_event = None;

        // Act
        let events = create_events(&priced_order, acknowledgment_event);

        // Assert
        assert_eq!(events.len(), 2);
        assert!(events[0].is_shippable());
        assert!(events[1].is_billable());
    }

    #[rstest]
    fn test_no_billing() {
        // Arrange: 確認メールあり、請求なし
        let priced_order = create_test_priced_order("order-003", Decimal::ZERO, vec![]);
        let acknowledgment_event = Some(create_test_acknowledgment_event("order-003"));

        // Act
        let events = create_events(&priced_order, acknowledgment_event);

        // Assert
        assert_eq!(events.len(), 2);
        assert!(events[0].is_acknowledgment());
        assert!(events[1].is_shippable());
    }

    #[rstest]
    fn test_minimal_events() {
        // Arrange: 確認メールなし、請求なし（最小ケース）
        let priced_order = create_test_priced_order("order-004", Decimal::ZERO, vec![]);
        let acknowledgment_event = None;

        // Act
        let events = create_events(&priced_order, acknowledgment_event);

        // Assert
        // 最小ケース - ShippableOrderPlaced のみ
        assert_eq!(events.len(), 1);
        assert!(events[0].is_shippable());
    }

    #[rstest]
    fn test_event_order() {
        // Arrange
        let priced_order = create_test_priced_order("order-005", Decimal::from(100), vec![]);
        let acknowledgment_event = Some(create_test_acknowledgment_event("order-005"));

        // Act
        let events = create_events(&priced_order, acknowledgment_event);

        // Assert: イベントの順序確認
        // 1. AcknowledgmentSent
        // 2. ShippableOrderPlaced
        // 3. BillableOrderPlaced
        assert_eq!(events.len(), 3);

        match &events[0] {
            PlaceOrderEvent::AcknowledgmentSent(_) => {}
            _ => panic!("Expected AcknowledgmentSent at index 0"),
        }

        match &events[1] {
            PlaceOrderEvent::ShippableOrderPlaced(_) => {}
            _ => panic!("Expected ShippableOrderPlaced at index 1"),
        }

        match &events[2] {
            PlaceOrderEvent::BillableOrderPlaced(_) => {}
            _ => panic!("Expected BillableOrderPlaced at index 2"),
        }
    }
}
