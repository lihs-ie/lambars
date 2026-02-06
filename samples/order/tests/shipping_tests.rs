//! Tests for shipping cost calculation and acknowledgment email sending
//!
//! Tests for Phase 6 implementation.

use lambars::effect::IO;
use order_taking_sample::compound_types::{Address, CustomerInfo};
use order_taking_sample::simple_types::{BillingAmount, OrderId, Price};
use order_taking_sample::workflow::{
    HtmlString, OrderAcknowledgment, PricedOrder, PricedOrderWithShippingMethod, PricingMethod,
    SendResult, ShippingInfo, ShippingMethod, ShippingRegion, acknowledge_order,
    add_shipping_info_to_order, calculate_shipping_cost, classify_shipping_region,
    free_vip_shipping,
};
use rstest::rstest;
use rust_decimal::Decimal;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

// =============================================================================
// Test helper functions
// =============================================================================

/// Creates a mock function that returns a fixed shipping cost
fn fixed_shipping_cost(cost: Decimal) -> impl Fn(&PricedOrder) -> Price {
    move |_| Price::unsafe_create(cost)
}

/// Creates a mock function that always returns `IO::pure(Sent)`
fn always_send() -> impl Fn(&OrderAcknowledgment) -> IO<SendResult> {
    |_| IO::pure(SendResult::Sent)
}

/// Creates a mock function that always returns `IO::pure(NotSent)`
fn never_send() -> impl Fn(&OrderAcknowledgment) -> IO<SendResult> {
    |_| IO::pure(SendResult::NotSent)
}

/// Creates a simple mock function for acknowledgment email generation
fn simple_letter() -> impl Fn(&PricedOrderWithShippingMethod) -> HtmlString {
    |order| {
        let order_id = order.priced_order().order_id().value();
        HtmlString::new(format!("<p>Order {order_id} confirmed</p>"))
    }
}

/// Creates a [`PricedOrder`] for testing
fn create_test_priced_order(vip_status: &str, country: &str, state: &str) -> PricedOrder {
    let order_id = OrderId::create("OrderId", "order-001").unwrap();
    let customer_info =
        CustomerInfo::create("John", "Doe", "john@example.com", vip_status).unwrap();
    let address =
        Address::create("123 Main St", "", "", "", "City", "12345", state, country).unwrap();
    let amount_to_bill = BillingAmount::create(Decimal::from(100)).unwrap();

    PricedOrder::new(
        order_id,
        customer_info,
        address.clone(),
        address,
        amount_to_bill,
        vec![],
        PricingMethod::Standard,
    )
}

/// Creates a [`PricedOrderWithShippingMethod`] for testing
fn create_test_order_with_shipping(
    vip_status: &str,
    country: &str,
    state: &str,
    shipping_cost: Decimal,
    shipping_method: ShippingMethod,
) -> PricedOrderWithShippingMethod {
    let priced_order = create_test_priced_order(vip_status, country, state);
    let shipping_info = ShippingInfo::new(shipping_method, Price::create(shipping_cost).unwrap());
    PricedOrderWithShippingMethod::new(shipping_info, priced_order)
}

// =============================================================================
// Tests for ShippingRegion enum
// =============================================================================

mod shipping_region_tests {
    use super::*;

    #[rstest]
    fn test_is_us_local_state() {
        let region = ShippingRegion::UsLocalState;
        assert!(region.is_us_local_state());
        assert!(!region.is_us_remote_state());
        assert!(!region.is_international());
    }

    #[rstest]
    fn test_is_us_remote_state() {
        let region = ShippingRegion::UsRemoteState;
        assert!(!region.is_us_local_state());
        assert!(region.is_us_remote_state());
        assert!(!region.is_international());
    }

    #[rstest]
    fn test_is_international() {
        let region = ShippingRegion::International;
        assert!(!region.is_us_local_state());
        assert!(!region.is_us_remote_state());
        assert!(region.is_international());
    }

    #[rstest]
    fn test_copy_trait() {
        let region1 = ShippingRegion::UsLocalState;
        let region2 = region1; // Copy
        assert_eq!(region1, region2);
    }

    #[rstest]
    fn test_clone_trait() {
        let region1 = ShippingRegion::UsRemoteState;
        // Verify that ShippingRegion implements Copy, but Clone is also available
        #[allow(clippy::clone_on_copy)]
        let region2 = region1.clone();
        assert_eq!(region1, region2);
    }

    #[rstest]
    fn test_debug_trait() {
        let region = ShippingRegion::International;
        let debug_str = format!("{region:?}");
        assert!(debug_str.contains("International"));
    }
}

// =============================================================================
// Tests for classify_shipping_region function
// =============================================================================

mod classify_shipping_region_tests {
    use super::*;

    #[rstest]
    fn test_california_is_local() {
        let address = Address::create(
            "123 Main St",
            "",
            "",
            "",
            "Los Angeles",
            "90001",
            "CA",
            "US",
        )
        .unwrap();
        let region = classify_shipping_region(&address);
        assert_eq!(region, ShippingRegion::UsLocalState);
    }

    #[rstest]
    fn test_oregon_is_local() {
        let address =
            Address::create("456 Oak Ave", "", "", "", "Portland", "97201", "OR", "US").unwrap();
        let region = classify_shipping_region(&address);
        assert_eq!(region, ShippingRegion::UsLocalState);
    }

    #[rstest]
    fn test_arizona_is_local_with_usa() {
        let address =
            Address::create("789 Pine Rd", "", "", "", "Phoenix", "85001", "AZ", "USA").unwrap();
        let region = classify_shipping_region(&address);
        assert_eq!(region, ShippingRegion::UsLocalState);
    }

    #[rstest]
    fn test_nevada_is_local() {
        let address = Address::create(
            "101 Vegas Blvd",
            "",
            "",
            "",
            "Las Vegas",
            "89101",
            "NV",
            "US",
        )
        .unwrap();
        let region = classify_shipping_region(&address);
        assert_eq!(region, ShippingRegion::UsLocalState);
    }

    #[rstest]
    fn test_new_york_is_remote() {
        let address =
            Address::create("200 Broadway", "", "", "", "New York", "10001", "NY", "US").unwrap();
        let region = classify_shipping_region(&address);
        assert_eq!(region, ShippingRegion::UsRemoteState);
    }

    #[rstest]
    fn test_texas_is_remote_with_usa() {
        let address =
            Address::create("300 Main St", "", "", "", "Houston", "77001", "TX", "USA").unwrap();
        let region = classify_shipping_region(&address);
        assert_eq!(region, ShippingRegion::UsRemoteState);
    }

    #[rstest]
    fn test_florida_is_remote() {
        let address =
            Address::create("400 Beach Rd", "", "", "", "Miami", "33101", "FL", "US").unwrap();
        let region = classify_shipping_region(&address);
        assert_eq!(region, ShippingRegion::UsRemoteState);
    }

    #[rstest]
    fn test_canada_is_international() {
        // For international shipping tests, uses valid state code (NY) and changes the country
        let address = Address::create(
            "500 Maple St",
            "",
            "",
            "",
            "Toronto",
            "12345",
            "NY",
            "Canada",
        )
        .unwrap();
        let region = classify_shipping_region(&address);
        assert_eq!(region, ShippingRegion::International);
    }

    #[rstest]
    fn test_japan_is_international() {
        // For international shipping tests, uses valid state code (CA) and changes the country
        let address =
            Address::create("1-1-1 Shibuya", "", "", "", "Tokyo", "15000", "CA", "Japan").unwrap();
        let region = classify_shipping_region(&address);
        assert_eq!(region, ShippingRegion::International);
    }

    #[rstest]
    fn test_uk_is_international() {
        // For international shipping tests, uses valid state code (TX) and changes the country
        let address =
            Address::create("10 Downing St", "", "", "", "London", "12345", "TX", "UK").unwrap();
        let region = classify_shipping_region(&address);
        assert_eq!(region, ShippingRegion::International);
    }
}

// =============================================================================
// Tests for calculate_shipping_cost function
// =============================================================================

mod calculate_shipping_cost_tests {
    use super::*;

    #[rstest]
    fn test_local_state_cost_5() {
        let priced_order = create_test_priced_order("Normal", "US", "CA");
        let cost = calculate_shipping_cost(&priced_order);
        assert_eq!(cost.value(), Decimal::from(5));
    }

    #[rstest]
    fn test_local_state_oregon_cost_5() {
        let priced_order = create_test_priced_order("Normal", "US", "OR");
        let cost = calculate_shipping_cost(&priced_order);
        assert_eq!(cost.value(), Decimal::from(5));
    }

    #[rstest]
    fn test_remote_state_cost_10() {
        let priced_order = create_test_priced_order("Normal", "US", "NY");
        let cost = calculate_shipping_cost(&priced_order);
        assert_eq!(cost.value(), Decimal::from(10));
    }

    #[rstest]
    fn test_remote_state_texas_cost_10() {
        let priced_order = create_test_priced_order("Normal", "USA", "TX");
        let cost = calculate_shipping_cost(&priced_order);
        assert_eq!(cost.value(), Decimal::from(10));
    }

    #[rstest]
    fn test_international_cost_20() {
        // For international shipping tests, uses valid state code (CA) and changes the country
        let priced_order = create_test_priced_order("Normal", "Japan", "CA");
        let cost = calculate_shipping_cost(&priced_order);
        assert_eq!(cost.value(), Decimal::from(20));
    }

    #[rstest]
    fn test_international_canada_cost_20() {
        // For international shipping tests, uses valid state code (NY) and changes the country
        let priced_order = create_test_priced_order("Normal", "Canada", "NY");
        let cost = calculate_shipping_cost(&priced_order);
        assert_eq!(cost.value(), Decimal::from(20));
    }
}

// =============================================================================
// Tests for add_shipping_info_to_order function
// =============================================================================

mod add_shipping_info_to_order_tests {
    use super::*;

    #[rstest]
    fn test_shipping_info_added_correctly() {
        let priced_order = create_test_priced_order("Normal", "US", "NY");
        let mock_cost = fixed_shipping_cost(Decimal::from(10));

        let order_with_shipping = add_shipping_info_to_order(&mock_cost, &priced_order);

        assert!(
            order_with_shipping
                .shipping_info()
                .shipping_method()
                .is_fedex24()
        );
        assert_eq!(
            order_with_shipping.shipping_info().shipping_cost().value(),
            Decimal::from(10)
        );
    }

    #[rstest]
    fn test_priced_order_preserved() {
        let priced_order = create_test_priced_order("Normal", "US", "CA");
        let mock_cost = fixed_shipping_cost(Decimal::from(5));

        let order_with_shipping = add_shipping_info_to_order(&mock_cost, &priced_order);

        assert_eq!(
            order_with_shipping.priced_order().order_id().value(),
            "order-001"
        );
        assert_eq!(
            order_with_shipping
                .priced_order()
                .customer_info()
                .email_address()
                .value(),
            "john@example.com"
        );
    }

    #[rstest]
    #[case(Decimal::from(5))]
    #[case(Decimal::from(10))]
    #[case(Decimal::from(20))]
    fn test_with_different_costs(#[case] cost: Decimal) {
        let priced_order = create_test_priced_order("Normal", "US", "NY");
        let mock_cost = fixed_shipping_cost(cost);

        let order_with_shipping = add_shipping_info_to_order(&mock_cost, &priced_order);

        assert_eq!(
            order_with_shipping.shipping_info().shipping_cost().value(),
            cost
        );
    }

    #[rstest]
    fn test_uses_calculate_shipping_cost_function() {
        let priced_order = create_test_priced_order("Normal", "US", "CA");

        let order_with_shipping =
            add_shipping_info_to_order(&calculate_shipping_cost, &priced_order);

        // CA is UsLocalState, so $5
        assert_eq!(
            order_with_shipping.shipping_info().shipping_cost().value(),
            Decimal::from(5)
        );
    }
}

// =============================================================================
// Tests for free_vip_shipping function
// =============================================================================

mod free_vip_shipping_tests {
    use super::*;

    #[rstest]
    fn test_normal_customer_unchanged() {
        let order = create_test_order_with_shipping(
            "Normal",
            "US",
            "NY",
            Decimal::from(10),
            ShippingMethod::Fedex24,
        );

        let updated = free_vip_shipping(order);

        assert_eq!(
            updated.shipping_info().shipping_cost().value(),
            Decimal::from(10)
        );
        assert!(updated.shipping_info().shipping_method().is_fedex24());
    }

    #[rstest]
    fn test_vip_customer_free_shipping() {
        let order = create_test_order_with_shipping(
            "VIP",
            "US",
            "NY",
            Decimal::from(10),
            ShippingMethod::PostalService,
        );

        let updated = free_vip_shipping(order);

        assert_eq!(
            updated.shipping_info().shipping_cost().value(),
            Decimal::ZERO
        );
        assert!(updated.shipping_info().shipping_method().is_fedex24());
    }

    #[rstest]
    fn test_vip_with_20_dollar_shipping() {
        // For international shipping tests, uses valid state code (CA) and changes the country
        let order = create_test_order_with_shipping(
            "VIP",
            "Japan",
            "CA",
            Decimal::from(20),
            ShippingMethod::Ups48,
        );

        let updated = free_vip_shipping(order);

        assert_eq!(
            updated.shipping_info().shipping_cost().value(),
            Decimal::ZERO
        );
        assert!(updated.shipping_info().shipping_method().is_fedex24());
    }

    #[rstest]
    fn test_priced_order_preserved_for_vip() {
        let order = create_test_order_with_shipping(
            "VIP",
            "US",
            "CA",
            Decimal::from(5),
            ShippingMethod::Fedex24,
        );

        let updated = free_vip_shipping(order);

        assert_eq!(updated.priced_order().order_id().value(), "order-001");
        assert_eq!(
            updated
                .priced_order()
                .customer_info()
                .email_address()
                .value(),
            "john@example.com"
        );
    }

    #[rstest]
    fn test_priced_order_preserved_for_normal() {
        let order = create_test_order_with_shipping(
            "Normal",
            "US",
            "CA",
            Decimal::from(5),
            ShippingMethod::Fedex24,
        );

        let updated = free_vip_shipping(order);

        assert_eq!(updated.priced_order().order_id().value(), "order-001");
    }
}

// =============================================================================
// Tests for acknowledge_order function
// =============================================================================

mod acknowledge_order_tests {
    use super::*;

    #[rstest]
    fn test_sent_returns_some_event() {
        let order = create_test_order_with_shipping(
            "Normal",
            "US",
            "NY",
            Decimal::from(10),
            ShippingMethod::Fedex24,
        );
        let create_letter = simple_letter();
        let send_acknowledgment = always_send();

        let io_result = acknowledge_order(&create_letter, &send_acknowledgment, &order);

        let result = io_result.run_unsafe();
        assert!(result.is_some());

        let event = result.unwrap();
        assert_eq!(event.order_id().value(), "order-001");
        assert_eq!(event.email_address().value(), "john@example.com");
    }

    #[rstest]
    fn test_not_sent_returns_none() {
        let order = create_test_order_with_shipping(
            "Normal",
            "US",
            "NY",
            Decimal::from(10),
            ShippingMethod::Fedex24,
        );
        let create_letter = simple_letter();
        let send_acknowledgment = never_send();

        let io_result = acknowledge_order(&create_letter, &send_acknowledgment, &order);

        let result = io_result.run_unsafe();
        assert!(result.is_none());
    }

    #[rstest]
    fn test_letter_generated_correctly() {
        let order = create_test_order_with_shipping(
            "Normal",
            "US",
            "CA",
            Decimal::from(5),
            ShippingMethod::Fedex24,
        );

        let create_letter = |o: &PricedOrderWithShippingMethod| {
            let html = format!(
                "<p>Order {} confirmed</p>",
                o.priced_order().order_id().value()
            );
            HtmlString::new(html)
        };
        let send_acknowledgment = |ack: &OrderAcknowledgment| {
            // Capture the letter content for verification
            let _ = ack.letter().value();
            IO::pure(SendResult::Sent)
        };

        let io_result = acknowledge_order(&create_letter, &send_acknowledgment, &order);
        let result = io_result.run_unsafe();
        assert!(result.is_some());
    }

    #[rstest]
    fn test_io_deferred_execution() {
        let order = create_test_order_with_shipping(
            "Normal",
            "US",
            "NY",
            Decimal::from(10),
            ShippingMethod::Fedex24,
        );
        let create_letter = simple_letter();

        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let send_acknowledgment = move |_: &OrderAcknowledgment| {
            let flag = executed_clone.clone();
            IO::new(move || {
                flag.store(true, Ordering::SeqCst);
                SendResult::Sent
            })
        };

        let io_result = acknowledge_order(&create_letter, &send_acknowledgment, &order);

        // Creating an IO does not execute it
        assert!(!executed.load(Ordering::SeqCst));

        // Executed by run_unsafe()
        let result = io_result.run_unsafe();
        assert!(executed.load(Ordering::SeqCst));
        assert!(result.is_some());
    }

    #[rstest]
    fn test_vip_customer_acknowledgment() {
        let order = create_test_order_with_shipping(
            "VIP",
            "US",
            "NY",
            Decimal::from(0), // VIP has free shipping
            ShippingMethod::Fedex24,
        );
        let create_letter = simple_letter();
        let send_acknowledgment = always_send();

        let io_result = acknowledge_order(&create_letter, &send_acknowledgment, &order);

        let result = io_result.run_unsafe();
        assert!(result.is_some());

        let event = result.unwrap();
        assert_eq!(event.order_id().value(), "order-001");
    }
}

// =============================================================================
// Integration tests
// =============================================================================

mod integration_tests {
    use super::*;

    #[rstest]
    fn test_full_shipping_workflow_normal_customer() {
        // 1. Create a priced order
        let priced_order = create_test_priced_order("Normal", "US", "CA");

        // 2. Add shipping info using the actual calculate_shipping_cost function
        let order_with_shipping =
            add_shipping_info_to_order(&calculate_shipping_cost, &priced_order);

        // CA is UsLocalState, so $5
        assert_eq!(
            order_with_shipping.shipping_info().shipping_cost().value(),
            Decimal::from(5)
        );
        assert!(
            order_with_shipping
                .shipping_info()
                .shipping_method()
                .is_fedex24()
        );

        // 3. Apply VIP discount (should have no effect for Normal customer)
        let final_order = free_vip_shipping(order_with_shipping);

        // Still $5 for Normal customer
        assert_eq!(
            final_order.shipping_info().shipping_cost().value(),
            Decimal::from(5)
        );

        // 4. Send acknowledgment
        let create_letter = simple_letter();
        let send_acknowledgment = always_send();
        let io_result = acknowledge_order(&create_letter, &send_acknowledgment, &final_order);

        let result = io_result.run_unsafe();
        assert!(result.is_some());
        assert_eq!(result.unwrap().order_id().value(), "order-001");
    }

    #[rstest]
    fn test_full_shipping_workflow_vip_customer() {
        // 1. Create a priced order for VIP customer
        // For international shipping tests, uses valid state code (NY) and changes the country
        let priced_order = create_test_priced_order("VIP", "Japan", "NY");

        // 2. Add shipping info using the actual calculate_shipping_cost function
        let order_with_shipping =
            add_shipping_info_to_order(&calculate_shipping_cost, &priced_order);

        // Japan is International, so $20
        assert_eq!(
            order_with_shipping.shipping_info().shipping_cost().value(),
            Decimal::from(20)
        );

        // 3. Apply VIP discount (should make shipping free)
        let final_order = free_vip_shipping(order_with_shipping);

        // Free shipping for VIP
        assert_eq!(
            final_order.shipping_info().shipping_cost().value(),
            Decimal::ZERO
        );
        assert!(final_order.shipping_info().shipping_method().is_fedex24());

        // 4. Send acknowledgment
        let create_letter = simple_letter();
        let send_acknowledgment = always_send();
        let io_result = acknowledge_order(&create_letter, &send_acknowledgment, &final_order);

        let result = io_result.run_unsafe();
        assert!(result.is_some());
    }

    #[rstest]
    fn test_full_workflow_acknowledgment_failure() {
        let priced_order = create_test_priced_order("Normal", "US", "NY");
        let order_with_shipping =
            add_shipping_info_to_order(&calculate_shipping_cost, &priced_order);
        let final_order = free_vip_shipping(order_with_shipping);

        let create_letter = simple_letter();
        let send_acknowledgment = never_send();
        let io_result = acknowledge_order(&create_letter, &send_acknowledgment, &final_order);

        let result = io_result.run_unsafe();
        assert!(result.is_none());
    }
}
