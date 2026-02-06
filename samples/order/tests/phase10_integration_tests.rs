//! Phase 10 integration tests
//!
//! Integration tests using PricingCatalog and acknowledge_order_with_logging.

use lambars::effect::IO;
use order_taking_sample::compound_types::{Address, CustomerInfo};
use order_taking_sample::simple_types::{BillingAmount, OrderId, Price, ProductCode};
use order_taking_sample::workflow::{
    HtmlString, OrderAcknowledgment, PricedOrder, PricedOrderWithShippingMethod, PricingCatalog,
    PricingMethod, SendResult, acknowledge_order_with_logging, add_shipping_info_to_order,
    calculate_shipping_cost, create_catalog_pricing_function,
};
use rstest::rstest;
use rust_decimal::Decimal;
use std::cell::RefCell;
use std::rc::Rc;

// =============================================================================
// Test helpers
// =============================================================================

/// Creates a ProductCode for testing
fn create_test_product_code(code: &str) -> ProductCode {
    ProductCode::create("field", code).unwrap()
}

/// Creates a Price for testing
fn create_test_price(value: u32) -> Price {
    Price::create(Decimal::from(value)).unwrap()
}

/// Creates a PricedOrder for testing
fn create_test_priced_order(amount: Decimal) -> PricedOrder {
    let order_id = OrderId::create("OrderId", "order-001").unwrap();
    let customer_info = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
    let address = Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "US")
        .expect("Valid address");
    let billing_amount = BillingAmount::create(amount).unwrap();

    PricedOrder::new(
        order_id,
        customer_info,
        address.clone(),
        address,
        billing_amount,
        vec![],
        PricingMethod::Standard,
    )
}

// =============================================================================
// PricingCatalog Integration tests
// =============================================================================

#[rstest]
fn test_catalog_pricing_function_found() {
    let widget_code = create_test_product_code("W1234");
    let gizmo_code = create_test_product_code("G123");
    let widget_price = create_test_price(100);
    let gizmo_price = create_test_price(50);
    let default_price = create_test_price(10);

    let catalog = PricingCatalog::new()
        .set_price(&widget_code, widget_price)
        .set_price(&gizmo_code, gizmo_price);

    let get_price = create_catalog_pricing_function(catalog, default_price);

    // Products in the catalog return the set price
    assert_eq!(get_price(&widget_code).value(), Decimal::from(100));
    assert_eq!(get_price(&gizmo_code).value(), Decimal::from(50));
}

#[rstest]
fn test_catalog_pricing_function_not_found() {
    let widget_code = create_test_product_code("W1234");
    let unknown_code = create_test_product_code("W9999");
    let default_price = create_test_price(25);

    let catalog = PricingCatalog::singleton(&widget_code, create_test_price(100));
    let get_price = create_catalog_pricing_function(catalog, default_price);

    // Products not in the catalog return the default price
    assert_eq!(get_price(&unknown_code).value(), Decimal::from(25));
}

#[rstest]
fn test_catalog_merge_workflow() {
    // Merge base catalog and extension catalog
    let base_catalog = PricingCatalog::new()
        .set_price(&create_test_product_code("W1234"), create_test_price(100))
        .set_price(&create_test_product_code("G123"), create_test_price(50));

    let extension_catalog = PricingCatalog::new()
        .set_price(&create_test_product_code("W5678"), create_test_price(150))
        .set_price(&create_test_product_code("W1234"), create_test_price(120)); // Override

    let merged_catalog = base_catalog.merge(&extension_catalog);

    // verification
    assert_eq!(merged_catalog.len(), 3);

    // W1234 is overridden by extension value (120)
    assert_eq!(
        merged_catalog
            .get_price(&create_test_product_code("W1234"))
            .unwrap()
            .value(),
        Decimal::from(120)
    );

    // G123 remains original
    assert_eq!(
        merged_catalog
            .get_price(&create_test_product_code("G123"))
            .unwrap()
            .value(),
        Decimal::from(50)
    );

    // W5678 is added from extension
    assert_eq!(
        merged_catalog
            .get_price(&create_test_product_code("W5678"))
            .unwrap()
            .value(),
        Decimal::from(150)
    );
}

#[rstest]
fn test_catalog_immutability_in_workflow() {
    // Original catalog
    let original_catalog =
        PricingCatalog::singleton(&create_test_product_code("W1234"), create_test_price(100));

    // Create price retrieval function
    let get_price1 =
        create_catalog_pricing_function(original_catalog.clone(), create_test_price(10));

    // "Update" the catalog (actually creates a new catalog)
    let updated_catalog =
        original_catalog.set_price(&create_test_product_code("W1234"), create_test_price(200));

    let get_price2 = create_catalog_pricing_function(updated_catalog, create_test_price(10));

    // Price retrieval function using the original catalog returns original prices
    assert_eq!(
        get_price1(&create_test_product_code("W1234")).value(),
        Decimal::from(100)
    );

    // Price retrieval function using updated catalog returns new prices
    assert_eq!(
        get_price2(&create_test_product_code("W1234")).value(),
        Decimal::from(200)
    );
}

// =============================================================================
// acknowledge_order_with_logging Integration tests
// =============================================================================

#[rstest]
fn test_acknowledge_order_with_logging_integration() {
    // Create an order for testing
    let priced_order = create_test_priced_order(Decimal::from(100));

    // Add shipping information
    let order_with_shipping = add_shipping_info_to_order(&calculate_shipping_cost, &priced_order);

    // For logging
    let log_messages: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let log_messages_clone = log_messages.clone();

    let log_action = move |message: &str| {
        let messages = log_messages_clone.clone();
        let message = message.to_string();
        IO::new(move || {
            messages.borrow_mut().push(message.clone());
        })
    };

    let create_letter = |order: &PricedOrderWithShippingMethod| {
        let content = format!(
            "<p>Order {} confirmed. Total: ${}</p>",
            order.priced_order().order_id().value(),
            order.priced_order().amount_to_bill().value()
        );
        HtmlString::new(content)
    };

    let send_acknowledgment = |_: &OrderAcknowledgment| IO::pure(SendResult::Sent);

    // verificationExecute email sending
    let io_result = acknowledge_order_with_logging(
        &create_letter,
        &send_acknowledgment,
        &log_action,
        &order_with_shipping,
    );

    let result = io_result.run_unsafe();

    // Verify send succeeded
    assert!(result.is_some());
    let event = result.unwrap();
    assert_eq!(event.order_id().value(), "order-001");
    assert_eq!(event.email_address().value(), "john@example.com");

    // Verify log output
    let messages = log_messages.borrow();
    assert_eq!(messages.len(), 3);
    assert!(messages[0].contains("Creating"));
    assert!(messages[1].contains("Sending"));
    assert!(messages[2].contains("completed"));
}

#[rstest]
fn test_acknowledge_order_with_logging_not_sent() {
    let priced_order = create_test_priced_order(Decimal::from(100));
    let order_with_shipping = add_shipping_info_to_order(&calculate_shipping_cost, &priced_order);

    let log_messages: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let log_messages_clone = log_messages.clone();

    let log_action = move |message: &str| {
        let messages = log_messages_clone.clone();
        let message = message.to_string();
        IO::new(move || {
            messages.borrow_mut().push(message.clone());
        })
    };

    let create_letter =
        |_: &PricedOrderWithShippingMethod| HtmlString::new("<p>Test</p>".to_string());

    // Simulate send failure
    let send_acknowledgment = |_: &OrderAcknowledgment| IO::pure(SendResult::NotSent);

    let io_result = acknowledge_order_with_logging(
        &create_letter,
        &send_acknowledgment,
        &log_action,
        &order_with_shipping,
    );

    let result = io_result.run_unsafe();

    // None on send failure
    assert!(result.is_none());

    // All logs are output
    let messages = log_messages.borrow();
    assert_eq!(messages.len(), 3);
}

// =============================================================================
// Complete workflow integration test
// =============================================================================

#[rstest]
fn test_complete_workflow_with_catalog_and_logging() {
    // 1. Create catalog
    let catalog = PricingCatalog::new()
        .set_price(&create_test_product_code("W1234"), create_test_price(100))
        .set_price(&create_test_product_code("G123"), create_test_price(50));

    // 2. Generate price retrieval function
    let get_price = create_catalog_pricing_function(catalog, create_test_price(10));

    // 3. Verify prices
    assert_eq!(
        get_price(&create_test_product_code("W1234")).value(),
        Decimal::from(100)
    );

    // 4. Temporary order data (already priced)
    let priced_order = create_test_priced_order(Decimal::from(250));

    // 5. Shipping informationaddition
    let order_with_shipping = add_shipping_info_to_order(&calculate_shipping_cost, &priced_order);

    // NY is a remote state so $10
    assert_eq!(
        order_with_shipping.shipping_info().shipping_cost().value(),
        Decimal::from(10)
    );

    // 6. Send acknowledgment email (using eff! macro)
    let log_messages: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let log_messages_clone = log_messages.clone();

    let log_action = move |message: &str| {
        let messages = log_messages_clone.clone();
        let message = message.to_string();
        IO::new(move || {
            messages.borrow_mut().push(message.clone());
        })
    };

    let create_letter =
        |_: &PricedOrderWithShippingMethod| HtmlString::new("<p>Confirmed</p>".to_string());

    let send_acknowledgment = |_: &OrderAcknowledgment| IO::pure(SendResult::Sent);

    let io_result = acknowledge_order_with_logging(
        &create_letter,
        &send_acknowledgment,
        &log_action,
        &order_with_shipping,
    );

    // 7. Event generation
    let result = io_result.run_unsafe();
    assert!(result.is_some());

    let event = result.unwrap();
    assert_eq!(event.order_id().value(), "order-001");
    assert_eq!(event.email_address().value(), "john@example.com");

    // Verify logs are output correctly
    let messages = log_messages.borrow();
    assert_eq!(messages.len(), 3);
}
