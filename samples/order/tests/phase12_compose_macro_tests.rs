//! Phase 12: compose! macro integration tests
//!
//! Verifies the behavior of functions using the compose! macro.
//! - Law verification (associativity, identity)
//! - Domain function composition tests
//! - Tests combining with partial application

use lambars::compose;
use lambars::compose::identity;
use order_taking_sample::compound_types::{Address, CustomerInfo};
use order_taking_sample::simple_types::{
    BillingAmount, EmailAddress, OrderId, OrderLineId, OrderQuantity, Price, ProductCode,
};
use order_taking_sample::workflow::{
    OrderAcknowledgmentSent, PlaceOrderEvent, PricedOrder, PricedOrderLine, PricedOrderProductLine,
    PricingMethod, add_shipping_info_to_order, calculate_shipping_cost, create_events,
    create_shipping_event, free_vip_shipping,
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
    vip_status: &str,
) -> PricedOrder {
    let order_id = OrderId::create("OrderId", order_id).unwrap();
    let customer_info =
        CustomerInfo::create("John", "Doe", "john@example.com", vip_status).unwrap();
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
// compose! macro law verification tests (REQ-124)
// =============================================================================

#[rstest]
fn test_compose_associativity() {
    // Associativity law: compose!(f, compose!(g, h)) == compose!(compose!(f, g), h)
    fn f(x: i32) -> i32 {
        x + 1
    }
    fn g(x: i32) -> i32 {
        x * 2
    }
    fn h(x: i32) -> i32 {
        x - 3
    }

    let left = compose!(f, compose!(g, h));
    let right = compose!(compose!(f, g), h);

    // Test with multiple input values
    for x in [0, 1, 5, 10, 100] {
        assert_eq!(left(x), right(x), "Associativity failed for x = {}", x);
    }
}

#[rstest]
fn test_compose_left_identity() {
    // Left identity: compose!(identity, f) == f
    fn f(x: i32) -> i32 {
        x * 2
    }

    let composed = compose!(identity, f);

    for x in [0, 1, 5, 10, 100] {
        assert_eq!(composed(x), f(x), "Left identity failed for x = {}", x);
    }
}

#[rstest]
fn test_compose_right_identity() {
    // Right identity: compose!(f, identity) == f
    fn f(x: i32) -> i32 {
        x * 2
    }

    let composed = compose!(f, identity);

    for x in [0, 1, 5, 10, 100] {
        assert_eq!(composed(x), f(x), "Right identity failed for x = {}", x);
    }
}

// =============================================================================
// Domain function composition tests (REQ-124, REQ-125)
// =============================================================================

#[rstest]
fn test_compose_shipping_event_creation() {
    // Composition of shipping event creation function using compose!
    let to_shipping_event = compose!(PlaceOrderEvent::ShippableOrderPlaced, create_shipping_event);

    let priced_order =
        create_test_priced_order("order-compose-001", Decimal::from(100), vec![], "Normal");

    let event = to_shipping_event(&priced_order);

    // Verify it is a ShippableOrderPlaced event
    assert!(event.is_shippable());

    // Verify event contents
    if let PlaceOrderEvent::ShippableOrderPlaced(shipping_event) = event {
        assert_eq!(shipping_event.order_id().value(), "order-compose-001");
        assert_eq!(shipping_event.pdf().name(), "Orderorder-compose-001.pdf");
    } else {
        panic!("Expected ShippableOrderPlaced event");
    }
}

#[rstest]
fn test_compose_pipeline_with_partial_application_normal_customer() {
    // Test combining partial application and compose! (regular customer)
    let priced_order =
        create_test_priced_order("order-normal", Decimal::from(100), vec![], "Normal");

    // Partially apply add_shipping_info_to_order
    let add_shipping =
        |order: &PricedOrder| add_shipping_info_to_order(&calculate_shipping_cost, order);

    // Compose free_vip_shipping and add_shipping
    let process_shipping = compose!(free_vip_shipping, add_shipping);

    let result = process_shipping(&priced_order);

    // Regular customers incur shipping charges (NY is a remote state, so $10)
    assert_eq!(
        result.shipping_info().shipping_cost().value(),
        Decimal::from(10)
    );
}

#[rstest]
fn test_compose_pipeline_with_partial_application_vip_customer() {
    // Test combining partial application and compose! (VIP customer)
    let priced_order = create_test_priced_order("order-vip", Decimal::from(100), vec![], "VIP");

    // Partially apply add_shipping_info_to_order
    let add_shipping =
        |order: &PricedOrder| add_shipping_info_to_order(&calculate_shipping_cost, order);

    // Compose free_vip_shipping and add_shipping
    let process_shipping = compose!(free_vip_shipping, add_shipping);

    let result = process_shipping(&priced_order);

    // VIP customer has free shipping
    assert_eq!(
        result.shipping_info().shipping_cost().value(),
        Decimal::ZERO
    );
    assert!(result.shipping_info().shipping_method().is_fedex24());
}

// =============================================================================
// create_events integration tests (REQ-125)
// =============================================================================

#[rstest]
fn test_create_events_with_compose_macro_generates_correct_order() {
    // Verify compose! macro implementation generates correct event order
    let priced_order = create_test_priced_order("order-001", Decimal::from(1000), vec![], "Normal");
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
fn test_create_events_shipping_event_content_with_compose() {
    // Verify ShippableOrderPlaced content generated by compose! macro
    let lines = vec![
        create_test_product_line("line-001", "W1234", Decimal::from(100)),
        create_test_product_line("line-002", "G567", Decimal::from(200)),
    ];
    let priced_order =
        create_test_priced_order("order-compose-test", Decimal::from(300), lines, "Normal");

    let events = create_events(&priced_order, None);

    assert_eq!(events.len(), 2);

    // Verify ShippableOrderPlaced event contents
    if let PlaceOrderEvent::ShippableOrderPlaced(shipping_event) = &events[0] {
        assert_eq!(shipping_event.order_id().value(), "order-compose-test");
        assert_eq!(shipping_event.shipment_lines().len(), 2);
        assert_eq!(shipping_event.pdf().name(), "Orderorder-compose-test.pdf");
    } else {
        panic!("Expected ShippableOrderPlaced event at index 0");
    }
}

// =============================================================================
// Shipping processing pipeline integration tests (REQ-125)
// =============================================================================

#[rstest]
fn test_shipping_pipeline_preserves_order_data() {
    // Verify shipping processing pipeline composed with compose! preserves order data
    let lines = vec![create_test_product_line(
        "line-001",
        "W1234",
        Decimal::from(500),
    )];
    let priced_order =
        create_test_priced_order("order-preserve-data", Decimal::from(500), lines, "Normal");

    let add_shipping =
        |order: &PricedOrder| add_shipping_info_to_order(&calculate_shipping_cost, order);
    let process_shipping = compose!(free_vip_shipping, add_shipping);

    let result = process_shipping(&priced_order);

    // Verify original order data is preserved
    assert_eq!(
        result.priced_order().order_id().value(),
        "order-preserve-data"
    );
    assert_eq!(
        result.priced_order().amount_to_bill().value(),
        Decimal::from(500)
    );
    assert_eq!(result.priced_order().lines().len(), 1);
}

#[rstest]
fn test_shipping_pipeline_with_international_address() {
    // Test for international shipping
    let order_id = OrderId::create("OrderId", "order-intl").unwrap();
    let customer_info = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
    // Uses valid state code for international shipping tests and changes the country
    let address =
        Address::create("123 Main St", "", "", "", "Tokyo", "10000", "NY", "Japan").unwrap();
    let amount = BillingAmount::create(Decimal::from(100)).unwrap();

    let priced_order = PricedOrder::new(
        order_id,
        customer_info,
        address.clone(),
        address,
        amount,
        vec![],
        PricingMethod::Standard,
    );

    let add_shipping =
        |order: &PricedOrder| add_shipping_info_to_order(&calculate_shipping_cost, order);
    let process_shipping = compose!(free_vip_shipping, add_shipping);

    let result = process_shipping(&priced_order);

    // International shipping is $20
    assert_eq!(
        result.shipping_info().shipping_cost().value(),
        Decimal::from(20)
    );
}

// =============================================================================
// compose! vs pipe! comparison tests
// =============================================================================

#[rstest]
fn test_compose_produces_reusable_function() {
    // Verify function generated by compose! is reusable
    let to_shipping_event = compose!(PlaceOrderEvent::ShippableOrderPlaced, create_shipping_event);

    // Apply same composition function to multiple orders
    let order1 = create_test_priced_order("order-reuse-1", Decimal::from(100), vec![], "Normal");
    let order2 = create_test_priced_order("order-reuse-2", Decimal::from(200), vec![], "Normal");

    let event1 = to_shipping_event(&order1);
    let event2 = to_shipping_event(&order2);

    // Both are ShippableOrderPlaced events
    assert!(event1.is_shippable());
    assert!(event2.is_shippable());

    // Each has the correct order_id
    if let PlaceOrderEvent::ShippableOrderPlaced(e1) = &event1 {
        assert_eq!(e1.order_id().value(), "order-reuse-1");
    }
    if let PlaceOrderEvent::ShippableOrderPlaced(e2) = &event2 {
        assert_eq!(e2.order_id().value(), "order-reuse-2");
    }
}
