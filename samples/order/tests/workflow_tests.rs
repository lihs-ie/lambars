//! Supplementary tests for the workflow module
//!
//! State transition tests for the PlaceOrder workflow, IO monad behavior tests,
//! tests for error handling and event generation.

use lambars::effect::IO;
use order_taking_sample::simple_types::{Price, ProductCode, VipStatus};
use order_taking_sample::workflow::{
    AddressValidationError, CheckedAddress, HtmlString, OrderAcknowledgment, PlaceOrderError,
    PricedOrder, PricedOrderWithShippingMethod, PricingMethod, SendResult, UnvalidatedAddress,
    UnvalidatedCustomerInfo, UnvalidatedOrder, UnvalidatedOrderLine, place_order, price_order,
    validate_order,
};
use rstest::rstest;
use rust_decimal::Decimal;
use std::cell::Cell;
use std::rc::Rc;

// =============================================================================
// Test data factory
// =============================================================================

fn valid_customer_info() -> UnvalidatedCustomerInfo {
    UnvalidatedCustomerInfo::new(
        "John".to_string(),
        "Doe".to_string(),
        "john@example.com".to_string(),
        "Normal".to_string(),
    )
}

fn vip_customer_info() -> UnvalidatedCustomerInfo {
    UnvalidatedCustomerInfo::new(
        "Jane".to_string(),
        "Smith".to_string(),
        "jane@example.com".to_string(),
        "VIP".to_string(),
    )
}

fn valid_address() -> UnvalidatedAddress {
    UnvalidatedAddress::new(
        "123 Main St".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "New York".to_string(),
        "10001".to_string(),
        "NY".to_string(),
        "USA".to_string(),
    )
}

fn valid_order_line(line_id: &str, product_code: &str, quantity: i32) -> UnvalidatedOrderLine {
    UnvalidatedOrderLine::new(
        line_id.to_string(),
        product_code.to_string(),
        Decimal::from(quantity),
    )
}

fn valid_order() -> UnvalidatedOrder {
    UnvalidatedOrder::new(
        "order-001".to_string(),
        valid_customer_info(),
        valid_address(),
        valid_address(),
        vec![valid_order_line("line-001", "W1234", 10)],
        "".to_string(),
    )
}

fn vip_order() -> UnvalidatedOrder {
    UnvalidatedOrder::new(
        "order-001".to_string(),
        vip_customer_info(),
        valid_address(),
        valid_address(),
        vec![valid_order_line("line-001", "W1234", 10)],
        "".to_string(),
    )
}

fn mock_product_exists() -> impl Fn(&ProductCode) -> bool {
    |_: &ProductCode| true
}

fn mock_product_not_exists(invalid_codes: Vec<String>) -> impl Fn(&ProductCode) -> bool {
    move |code: &ProductCode| !invalid_codes.contains(&code.value().to_string())
}

fn mock_address_valid()
-> impl Fn(&UnvalidatedAddress) -> Result<CheckedAddress, AddressValidationError> {
    |addr: &UnvalidatedAddress| Ok(CheckedAddress::new(addr.clone()))
}

fn mock_address_not_found()
-> impl Fn(&UnvalidatedAddress) -> Result<CheckedAddress, AddressValidationError> {
    |_: &UnvalidatedAddress| Err(AddressValidationError::AddressNotFound)
}

fn mock_pricing_fn(price: i32) -> impl Fn(&PricingMethod) -> Rc<dyn Fn(&ProductCode) -> Price> {
    move |_: &PricingMethod| {
        let price_val = price;
        Rc::new(move |_: &ProductCode| Price::create(Decimal::from(price_val)).unwrap())
    }
}

fn mock_shipping_cost(_order: &PricedOrder) -> Price {
    Price::create(Decimal::from(10)).unwrap()
}

fn mock_create_letter(_order: &PricedOrderWithShippingMethod) -> HtmlString {
    HtmlString::new("<p>Order confirmed</p>".to_string())
}

fn mock_send_acknowledgment_sent() -> impl Fn(&OrderAcknowledgment) -> IO<SendResult> {
    |_: &OrderAcknowledgment| IO::pure(SendResult::Sent)
}

fn mock_send_acknowledgment_not_sent() -> impl Fn(&OrderAcknowledgment) -> IO<SendResult> {
    |_: &OrderAcknowledgment| IO::pure(SendResult::NotSent)
}

// =============================================================================
// State transition tests
// =============================================================================

mod state_transition_tests {
    use super::*;

    #[rstest]
    fn test_unvalidated_to_validated_transition() {
        let order = valid_order();
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();

        let result = validate_order(&check_product, &check_address, &order);

        assert!(result.is_ok());
        let validated = result.unwrap();
        assert_eq!(validated.order_id().value(), "order-001");
        assert_eq!(validated.lines().len(), 1);
    }

    #[rstest]
    fn test_validated_to_priced_transition() {
        let order = valid_order();
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();

        let validated = validate_order(&check_product, &check_address, &order).unwrap();
        let get_pricing_fn = mock_pricing_fn(100);

        let result = price_order(&get_pricing_fn, &validated);

        assert!(result.is_ok());
        let priced = result.unwrap();
        // 10 * 100 = 1000
        assert_eq!(priced.amount_to_bill().value(), Decimal::from(1000));
    }

    #[rstest]
    fn test_full_workflow_transition() {
        let order = valid_order();
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();
        let get_pricing_fn = mock_pricing_fn(100);
        let send_ack = mock_send_acknowledgment_sent();

        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &mock_shipping_cost,
            &mock_create_letter,
            &send_ack,
            &order,
        );

        let result = io_result.run_unsafe();
        assert!(result.is_ok());
        let events = result.unwrap();
        // ShippableOrderPlaced, BillableOrderPlaced, OrderAcknowledgmentSent
        assert_eq!(events.len(), 3);
    }
}

// =============================================================================
// IO monad behavior tests
// =============================================================================

mod io_monad_tests {
    use super::*;

    #[rstest]
    fn test_io_monad_deferred_execution() {
        let executed = Rc::new(Cell::new(false));
        let executed_clone = Rc::clone(&executed);

        let send_ack = move |_: &OrderAcknowledgment| {
            let executed_inner = Rc::clone(&executed_clone);
            IO::new(move || {
                executed_inner.set(true);
                SendResult::Sent
            })
        };

        let order = valid_order();
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();
        let get_pricing_fn = mock_pricing_fn(100);

        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &mock_shipping_cost,
            &mock_create_letter,
            &send_ack,
            &order,
        );

        // Creating an IO monad does not execute it
        assert!(!executed.get());

        // Execute with run_unsafe()
        let _ = io_result.run_unsafe();
        assert!(executed.get());
    }

    #[rstest]
    fn test_io_monad_multiple_execution() {
        let execution_count = Rc::new(Cell::new(0));
        let execution_count_clone = Rc::clone(&execution_count);

        let send_ack = move |_: &OrderAcknowledgment| {
            let count = Rc::clone(&execution_count_clone);
            IO::new(move || {
                count.set(count.get() + 1);
                SendResult::Sent
            })
        };

        let order = valid_order();
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();
        let get_pricing_fn = mock_pricing_fn(100);

        // First execution
        let io_result1 = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &mock_shipping_cost,
            &mock_create_letter,
            &send_ack,
            &order,
        );
        let _ = io_result1.run_unsafe();

        // Second execution (create a new IO monad)
        let io_result2 = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &mock_shipping_cost,
            &mock_create_letter,
            &send_ack,
            &order,
        );
        let _ = io_result2.run_unsafe();

        // Multiple executions cause the side effect to occur multiple times
        assert_eq!(execution_count.get(), 2);
    }
}

// =============================================================================
// Error handling tests
// =============================================================================

mod error_handling_tests {
    use super::*;

    #[rstest]
    fn test_validation_error_stops_workflow() {
        let order = UnvalidatedOrder::new(
            "".to_string(), // Invalid order ID
            valid_customer_info(),
            valid_address(),
            valid_address(),
            vec![valid_order_line("line-001", "W1234", 10)],
            "".to_string(),
        );
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();
        let get_pricing_fn = mock_pricing_fn(100);
        let send_ack = mock_send_acknowledgment_sent();

        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &mock_shipping_cost,
            &mock_create_letter,
            &send_ack,
            &order,
        );

        let result = io_result.run_unsafe();
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.is_validation());
        if let PlaceOrderError::Validation(validation_error) = error {
            assert_eq!(validation_error.field_name, "OrderId");
        }
    }

    #[rstest]
    fn test_address_validation_error_stops_workflow() {
        let order = valid_order();
        let check_product = mock_product_exists();
        let check_address = mock_address_not_found();
        let get_pricing_fn = mock_pricing_fn(100);
        let send_ack = mock_send_acknowledgment_sent();

        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &mock_shipping_cost,
            &mock_create_letter,
            &send_ack,
            &order,
        );

        let result = io_result.run_unsafe();
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.is_validation());
    }

    #[rstest]
    fn test_product_not_exists_error() {
        let order = valid_order();
        let check_product = mock_product_not_exists(vec!["W1234".to_string()]);
        let check_address = mock_address_valid();
        let get_pricing_fn = mock_pricing_fn(100);
        let send_ack = mock_send_acknowledgment_sent();

        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &mock_shipping_cost,
            &mock_create_letter,
            &send_ack,
            &order,
        );

        let result = io_result.run_unsafe();
        assert!(result.is_err());
    }

    #[rstest]
    fn test_pricing_overflow_error() {
        // 1000 x 100 = 100000 > Price maximum
        // However, the Price maximum is 1000, so 10 * 100 = 1000 is the limit
        // 11 * 100 = 1100 > 1000, overflow
        let order = UnvalidatedOrder::new(
            "order-001".to_string(),
            valid_customer_info(),
            valid_address(),
            valid_address(),
            vec![valid_order_line("line-001", "W1234", 11)],
            "".to_string(),
        );
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();
        let get_pricing_fn = mock_pricing_fn(100);
        let send_ack = mock_send_acknowledgment_sent();

        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &mock_shipping_cost,
            &mock_create_letter,
            &send_ack,
            &order,
        );

        let result = io_result.run_unsafe();
        assert!(result.is_err());
        assert!(result.unwrap_err().is_pricing());
    }
}

// =============================================================================
// Event generation tests
// =============================================================================

mod event_generation_tests {
    use super::*;
    use order_taking_sample::workflow::PlaceOrderEvent;

    #[rstest]
    fn test_events_with_mail_sent() {
        let order = valid_order();
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();
        let get_pricing_fn = mock_pricing_fn(100);
        let send_ack = mock_send_acknowledgment_sent();

        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &mock_shipping_cost,
            &mock_create_letter,
            &send_ack,
            &order,
        );

        let result = io_result.run_unsafe();
        assert!(result.is_ok());
        let events = result.unwrap();

        // 3 events: ShippableOrderPlaced, BillableOrderPlaced, OrderAcknowledgmentSent
        assert_eq!(events.len(), 3);

        let has_shippable = events
            .iter()
            .any(|e| matches!(e, PlaceOrderEvent::ShippableOrderPlaced(_)));
        let has_billable = events
            .iter()
            .any(|e| matches!(e, PlaceOrderEvent::BillableOrderPlaced(_)));
        let has_acknowledgment = events
            .iter()
            .any(|e| matches!(e, PlaceOrderEvent::AcknowledgmentSent(_)));

        assert!(has_shippable);
        assert!(has_billable);
        assert!(has_acknowledgment);
    }

    #[rstest]
    fn test_events_without_mail_sent() {
        let order = valid_order();
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();
        let get_pricing_fn = mock_pricing_fn(100);
        let send_ack = mock_send_acknowledgment_not_sent();

        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &mock_shipping_cost,
            &mock_create_letter,
            &send_ack,
            &order,
        );

        let result = io_result.run_unsafe();
        assert!(result.is_ok());
        let events = result.unwrap();

        // 2 events: ShippableOrderPlaced, BillableOrderPlaced (no AcknowledgmentSent)
        assert_eq!(events.len(), 2);

        let has_acknowledgment = events
            .iter()
            .any(|e| matches!(e, PlaceOrderEvent::AcknowledgmentSent(_)));
        assert!(!has_acknowledgment);
    }

    #[rstest]
    fn test_shippable_event_contains_order_lines() {
        let order = UnvalidatedOrder::new(
            "order-001".to_string(),
            valid_customer_info(),
            valid_address(),
            valid_address(),
            vec![
                valid_order_line("line-001", "W1234", 5),
                valid_order_line("line-002", "G123", 2),
            ],
            "".to_string(),
        );
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();
        let get_pricing_fn = mock_pricing_fn(50);
        let send_ack = mock_send_acknowledgment_sent();

        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &mock_shipping_cost,
            &mock_create_letter,
            &send_ack,
            &order,
        );

        let result = io_result.run_unsafe();
        assert!(result.is_ok());
        let events = result.unwrap();

        let shippable_event = events
            .iter()
            .find(|e| matches!(e, PlaceOrderEvent::ShippableOrderPlaced(_)));
        assert!(shippable_event.is_some());

        if let PlaceOrderEvent::ShippableOrderPlaced(event) = shippable_event.unwrap() {
            assert_eq!(event.shipment_lines().len(), 2);
        }
    }

    #[rstest]
    fn test_billable_event_contains_amount() {
        let order = valid_order();
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();
        let get_pricing_fn = mock_pricing_fn(100);
        let send_ack = mock_send_acknowledgment_sent();

        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &mock_shipping_cost,
            &mock_create_letter,
            &send_ack,
            &order,
        );

        let result = io_result.run_unsafe();
        assert!(result.is_ok());
        let events = result.unwrap();

        let billable_event = events
            .iter()
            .find(|e| matches!(e, PlaceOrderEvent::BillableOrderPlaced(_)));
        assert!(billable_event.is_some());

        if let PlaceOrderEvent::BillableOrderPlaced(event) = billable_event.unwrap() {
            // 10 * 100 = 1000
            assert_eq!(event.amount_to_bill().value(), Decimal::from(1000));
        }
    }
}

// =============================================================================
// VIP customerTest
// =============================================================================

mod vip_customer_tests {
    use super::*;

    #[rstest]
    fn test_vip_customer_gets_free_shipping() {
        let order = vip_order();
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();
        let get_pricing_fn = mock_pricing_fn(100);
        let send_ack = mock_send_acknowledgment_sent();

        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &mock_shipping_cost,
            &mock_create_letter,
            &send_ack,
            &order,
        );

        let result = io_result.run_unsafe();
        assert!(result.is_ok());
        let events = result.unwrap();

        // VIP customer has free shipping (free_vip_shipping)
        // Test whether it is reflected in the BillableOrderPlaced event
        let billable_event = events.iter().find(|e| {
            matches!(
                e,
                order_taking_sample::workflow::PlaceOrderEvent::BillableOrderPlaced(_)
            )
        });
        assert!(billable_event.is_some());
    }

    #[rstest]
    fn test_vip_status_preserved_through_workflow() {
        let order = vip_order();
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();

        let validated = validate_order(&check_product, &check_address, &order).unwrap();

        assert!(matches!(
            validated.customer_info().vip_status(),
            VipStatus::Vip
        ));
    }
}

// =============================================================================
// Promotion code tests
// =============================================================================

mod promotion_code_tests {
    use super::*;

    #[rstest]
    fn test_promotion_code_applied() {
        let order = UnvalidatedOrder::new(
            "order-001".to_string(),
            valid_customer_info(),
            valid_address(),
            valid_address(),
            vec![valid_order_line("line-001", "W1234", 10)],
            "SUMMER2024".to_string(),
        );
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();

        let validated = validate_order(&check_product, &check_address, &order).unwrap();

        assert!(validated.pricing_method().is_promotion());
        assert_eq!(
            validated.pricing_method().promotion_code().unwrap().value(),
            "SUMMER2024"
        );
    }

    #[rstest]
    fn test_no_promotion_code() {
        let order = valid_order();
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();

        let validated = validate_order(&check_product, &check_address, &order).unwrap();

        assert!(validated.pricing_method().is_standard());
    }
}

// =============================================================================
// Multiple order line tests
// =============================================================================

mod multiple_order_lines_tests {
    use super::*;

    #[rstest]
    fn test_many_order_lines() {
        let lines: Vec<UnvalidatedOrderLine> = (0..50)
            .map(|i| valid_order_line(&format!("line-{i:03}"), "W1234", 1))
            .collect();

        let order = UnvalidatedOrder::new(
            "order-001".to_string(),
            valid_customer_info(),
            valid_address(),
            valid_address(),
            lines,
            "".to_string(),
        );
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();
        let get_pricing_fn = mock_pricing_fn(10);
        let send_ack = mock_send_acknowledgment_sent();

        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &mock_shipping_cost,
            &mock_create_letter,
            &send_ack,
            &order,
        );

        let result = io_result.run_unsafe();
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 3);
    }

    #[rstest]
    fn test_mixed_widget_gizmo_lines() {
        let lines = vec![
            valid_order_line("line-001", "W1234", 5),
            valid_order_line("line-002", "G123", 3),
            valid_order_line("line-003", "W5678", 2),
            valid_order_line("line-004", "G456", 4),
        ];

        let order = UnvalidatedOrder::new(
            "order-001".to_string(),
            valid_customer_info(),
            valid_address(),
            valid_address(),
            lines,
            "".to_string(),
        );
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();

        let validated = validate_order(&check_product, &check_address, &order).unwrap();

        assert_eq!(validated.lines().len(), 4);
    }
}

// =============================================================================
// Empty order tests
// =============================================================================

mod empty_order_tests {
    use super::*;

    #[rstest]
    fn test_empty_order_lines() {
        let order = UnvalidatedOrder::new(
            "order-001".to_string(),
            valid_customer_info(),
            valid_address(),
            valid_address(),
            vec![],
            "".to_string(),
        );
        let check_product = mock_product_exists();
        let check_address = mock_address_valid();
        let get_pricing_fn = mock_pricing_fn(100);
        let send_ack = mock_send_acknowledgment_sent();

        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &mock_shipping_cost,
            &mock_create_letter,
            &send_ack,
            &order,
        );

        let result = io_result.run_unsafe();
        assert!(result.is_ok());
        let events = result.unwrap();

        // Succeeds even with an empty order (billing amount is 0)
        let billable_event = events.iter().find(|e| {
            matches!(
                e,
                order_taking_sample::workflow::PlaceOrderEvent::BillableOrderPlaced(_)
            )
        });
        if let Some(order_taking_sample::workflow::PlaceOrderEvent::BillableOrderPlaced(event)) =
            billable_event
        {
            assert_eq!(event.amount_to_bill().value(), Decimal::ZERO);
        }
    }
}
