//! Phase 11: pipe! macro integration tests
//!
//! Verifies the behavior of functions using the pipe! macro.
//! Since existing tests are in events_tests.rs,
//! This file verifies pipe! macro-specific patterns.

use order_taking_sample::compound_types::{Address, CustomerInfo};
use order_taking_sample::simple_types::{
    BillingAmount, EmailAddress, OrderId, OrderLineId, OrderQuantity, Price, ProductCode,
};
use order_taking_sample::workflow::{
    OrderAcknowledgmentSent, PricedOrder, PricedOrderLine, PricedOrderProductLine, PricingMethod,
    create_billing_event, create_events, create_shipping_event,
};
use rstest::rstest;
use rust_decimal::Decimal;

// =============================================================================
// Test helpers
// =============================================================================

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

fn create_test_product_line(line_id: &str, product_code: &str, price: Decimal) -> PricedOrderLine {
    let order_line_id = OrderLineId::create("OrderLineId", line_id).unwrap();
    let product_code = ProductCode::create("ProductCode", product_code).unwrap();
    let quantity = OrderQuantity::create("Quantity", &product_code, Decimal::from(5)).unwrap();
    let line_price = Price::create(price).unwrap();

    PricedOrderLine::ProductLine(PricedOrderProductLine::new(
        order_line_id,
        product_code,
        quantity,
        line_price,
    ))
}

// =============================================================================
// Tests for create_events (REQ-111)
// =============================================================================

#[rstest]
fn test_create_events_with_pipe_macro_generates_correct_order() {
    // Verify the pipe! macro implementation generates events in correct order
    let priced_order = create_test_priced_order("order-001", Decimal::from(1000), vec![]);
    let order_id = OrderId::create("OrderId", "order-001").unwrap();
    let email = EmailAddress::create("EmailAddress", "test@example.com").unwrap();
    let acknowledgment = OrderAcknowledgmentSent::new(order_id, email);

    let events = create_events(&priced_order, Some(acknowledgment));

    // Verify event order: Acknowledgment -> Shippable -> Billable
    assert_eq!(events.len(), 3);
    assert!(events[0].is_acknowledgment());
    assert!(events[1].is_shippable());
    assert!(events[2].is_billable());
}

#[rstest]
fn test_create_events_with_pipe_macro_handles_none_acknowledgment() {
    // Test when acknowledgment is None
    let priced_order = create_test_priced_order("order-002", Decimal::from(500), vec![]);

    let events = create_events(&priced_order, None);

    // No Acknowledgment: Shippable -> Billable
    assert_eq!(events.len(), 2);
    assert!(events[0].is_shippable());
    assert!(events[1].is_billable());
}

#[rstest]
fn test_create_events_with_pipe_macro_handles_zero_billing() {
    // Test when billing amount is 0
    let priced_order = create_test_priced_order("order-003", Decimal::ZERO, vec![]);
    let order_id = OrderId::create("OrderId", "order-003").unwrap();
    let email = EmailAddress::create("EmailAddress", "test@example.com").unwrap();
    let acknowledgment = OrderAcknowledgmentSent::new(order_id, email);

    let events = create_events(&priced_order, Some(acknowledgment));

    // No Billable: Acknowledgment -> Shippable
    assert_eq!(events.len(), 2);
    assert!(events[0].is_acknowledgment());
    assert!(events[1].is_shippable());
}

// =============================================================================
// Tests for create_shipping_event (REQ-112)
// =============================================================================

#[rstest]
fn test_create_shipping_event_with_pipe_macro_generates_pdf_name() {
    // Verify PDF name generation using pipe! macro
    let priced_order = create_test_priced_order("order-001", Decimal::from(100), vec![]);

    let event = create_shipping_event(&priced_order);

    // Verify the PDF name is generated correctly
    assert_eq!(event.pdf().name(), "Orderorder-001.pdf");
}

#[rstest]
fn test_create_shipping_event_with_pipe_macro_includes_order_id() {
    // Verify order_id is set correctly in event generation using pipe! macro
    let priced_order = create_test_priced_order("order-xyz", Decimal::from(100), vec![]);

    let event = create_shipping_event(&priced_order);

    assert_eq!(event.order_id().value(), "order-xyz");
}

#[rstest]
fn test_create_shipping_event_with_pipe_macro_filters_comment_lines() {
    // Verify the pipe! macro implementation correctly filters CommentLine
    let lines = vec![
        create_test_product_line("line-001", "W1234", Decimal::from(100)),
        PricedOrderLine::CommentLine("Gift wrapping".to_string()),
        create_test_product_line("line-002", "G567", Decimal::from(200)),
    ];
    let priced_order = create_test_priced_order("order-001", Decimal::from(300), lines);

    let event = create_shipping_event(&priced_order);

    // CommentLine is filtered out, only ProductLine is included
    assert_eq!(event.shipment_lines().len(), 2);
}

// =============================================================================
// Tests for create_billing_event (REQ-113)
// =============================================================================

#[rstest]
fn test_create_billing_event_with_pipe_macro_returns_some_for_positive_amount() {
    // Verify the pipe! macro implementation returns Some for positive amounts
    let priced_order = create_test_priced_order("order-001", Decimal::from(1000), vec![]);

    let result = create_billing_event(&priced_order);

    assert!(result.is_some());
    let event = result.unwrap();
    assert_eq!(event.amount_to_bill().value(), Decimal::from(1000));
}

#[rstest]
fn test_create_billing_event_with_pipe_macro_returns_none_for_zero_amount() {
    // Verify the pipe! macro implementation returns None for 0 amount
    let priced_order = create_test_priced_order("order-002", Decimal::ZERO, vec![]);

    let result = create_billing_event(&priced_order);

    assert!(result.is_none());
}

#[rstest]
fn test_create_billing_event_with_pipe_macro_includes_correct_order_id() {
    // Verify the pipe! macro implementation includes correct order_id
    let priced_order = create_test_priced_order("order-billing", Decimal::from(500), vec![]);

    let result = create_billing_event(&priced_order);

    assert!(result.is_some());
    let event = result.unwrap();
    assert_eq!(event.order_id().value(), "order-billing");
}

// =============================================================================
// pipe! macro composition tests (REQ-117)
// =============================================================================

#[rstest]
fn test_pipe_macro_composition_in_event_pipeline() {
    // Verify function composition via pipe! macro works correctly
    // shipping_event -> PlaceOrderEvent::ShippableOrderPlaced conversion
    let lines = vec![create_test_product_line(
        "line-001",
        "W1234",
        Decimal::from(100),
    )];
    let priced_order = create_test_priced_order("order-composite", Decimal::from(100), lines);

    let events = create_events(&priced_order, None);

    // Verify the Shippable event is generated correctly
    assert!(events.len() >= 1);
    let shippable_event = &events[0];
    assert!(shippable_event.is_shippable());

    // Verify ShippableOrderPlaced contents
    if let order_taking_sample::workflow::PlaceOrderEvent::ShippableOrderPlaced(event) =
        shippable_event
    {
        assert_eq!(event.order_id().value(), "order-composite");
        assert_eq!(event.shipment_lines().len(), 1);
    } else {
        panic!("Expected ShippableOrderPlaced event");
    }
}

#[rstest]
fn test_pipe_macro_preserves_data_integrity() {
    // Verify data integrity is maintained even with pipe! macro
    let order_id_str = "order-integrity-test";
    let billing_amount = Decimal::new(12345, 2); // 123.45
    let lines = vec![create_test_product_line(
        "line-001",
        "W1234",
        Decimal::from(100),
    )];
    let priced_order = create_test_priced_order(order_id_str, billing_amount, lines);
    let email = EmailAddress::create("EmailAddress", "integrity@example.com").unwrap();
    let acknowledgment =
        OrderAcknowledgmentSent::new(OrderId::create("OrderId", order_id_str).unwrap(), email);

    let events = create_events(&priced_order, Some(acknowledgment));

    // Verify data integrity of all events
    assert_eq!(events.len(), 3);

    // Acknowledgment event
    if let order_taking_sample::workflow::PlaceOrderEvent::AcknowledgmentSent(ack) = &events[0] {
        assert_eq!(ack.order_id().value(), order_id_str);
    }

    // Shippable event
    if let order_taking_sample::workflow::PlaceOrderEvent::ShippableOrderPlaced(ship) = &events[1] {
        assert_eq!(ship.order_id().value(), order_id_str);
        assert_eq!(ship.pdf().name(), format!("Order{}.pdf", order_id_str));
    }

    // Billable event
    if let order_taking_sample::workflow::PlaceOrderEvent::BillableOrderPlaced(bill) = &events[2] {
        assert_eq!(bill.order_id().value(), order_id_str);
        assert_eq!(bill.amount_to_bill().value(), billing_amount);
    }
}
