//! Tests for dummy dependency functions
//!
//! check_product_exists, check_address_exists, get_pricing_function,
//! Tests for calculate_shipping_cost, create_acknowledgment_letter, and send_acknowledgment

use order_taking_sample::api::{
    calculate_shipping_cost, check_address_exists, check_product_exists,
    create_acknowledgment_letter, get_pricing_function, send_acknowledgment,
};
use order_taking_sample::compound_types::{Address, CustomerInfo};
use order_taking_sample::simple_types::{BillingAmount, OrderId, Price, ProductCode};
use order_taking_sample::workflow::acknowledgment_types::{
    HtmlString, OrderAcknowledgment, SendResult,
};
use order_taking_sample::workflow::{
    PricedOrder, PricedOrderWithShippingMethod, PricingMethod, ShippingInfo, ShippingMethod,
    UnvalidatedAddress,
};
use rstest::rstest;
use rust_decimal::Decimal;

// =============================================================================
// Tests for check_product_exists
// =============================================================================

mod check_product_exists_tests {
    use super::*;

    #[rstest]
    fn test_widget_product_exists() {
        let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
        assert!(check_product_exists(&product_code));
    }

    #[rstest]
    fn test_gizmo_product_exists() {
        let product_code = ProductCode::create("ProductCode", "G123").unwrap();
        assert!(check_product_exists(&product_code));
    }
}

// =============================================================================
// Tests for check_address_exists
// =============================================================================

mod check_address_exists_tests {
    use super::*;

    #[rstest]
    fn test_valid_address() {
        let address = UnvalidatedAddress::new(
            "123 Main St".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "New York".to_string(),
            "10001".to_string(),
            "NY".to_string(),
            "USA".to_string(),
        );

        let result = check_address_exists(&address);

        assert!(result.is_ok());
    }

    #[rstest]
    fn test_returns_checked_address() {
        let address = UnvalidatedAddress::new(
            "456 Oak Ave".to_string(),
            "Suite 100".to_string(),
            "".to_string(),
            "".to_string(),
            "Los Angeles".to_string(),
            "90001".to_string(),
            "CA".to_string(),
            "USA".to_string(),
        );

        let result = check_address_exists(&address);

        assert!(result.is_ok());
        let checked = result.unwrap();
        assert_eq!(checked.value().address_line1(), address.address_line1());
    }
}

// =============================================================================
// Tests for get_pricing_function
// =============================================================================

mod get_pricing_function_tests {
    use super::*;

    #[rstest]
    fn test_widget_price() {
        let pricing_fn = get_pricing_function(&PricingMethod::Standard);
        let widget_code = ProductCode::create("ProductCode", "W1234").unwrap();

        let price = pricing_fn(&widget_code);

        assert_eq!(price.value(), Decimal::from(100));
    }

    #[rstest]
    fn test_gizmo_price() {
        let pricing_fn = get_pricing_function(&PricingMethod::Standard);
        let gizmo_code = ProductCode::create("ProductCode", "G123").unwrap();

        let price = pricing_fn(&gizmo_code);

        assert_eq!(price.value(), Decimal::from(50));
    }

    #[rstest]
    fn test_different_pricing_methods() {
        // pricing_method is unused in the dummy implementation, but we test it anyway
        let standard_fn = get_pricing_function(&PricingMethod::Standard);
        let promotion_fn = get_pricing_function(&PricingMethod::Promotion(
            order_taking_sample::simple_types::PromotionCode::new("PROMO".to_string()),
        ));

        let widget_code = ProductCode::create("ProductCode", "W1234").unwrap();

        // Both return the same price (dummy implementation)
        assert_eq!(standard_fn(&widget_code).value(), Decimal::from(100));
        assert_eq!(promotion_fn(&widget_code).value(), Decimal::from(100));
    }
}

// =============================================================================
// Tests for calculate_shipping_cost
// =============================================================================

mod calculate_shipping_cost_tests {
    use super::*;

    fn create_priced_order() -> PricedOrder {
        let order_id = OrderId::create("OrderId", "order-001").unwrap();
        let customer_info =
            CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
        let address =
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
        let amount_to_bill = BillingAmount::create(Decimal::from(1000)).unwrap();

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

    #[rstest]
    fn test_fixed_shipping_cost() {
        let priced_order = create_priced_order();

        let cost = calculate_shipping_cost(&priced_order);

        assert_eq!(cost.value(), Decimal::from(10));
    }
}

// =============================================================================
// Tests for create_acknowledgment_letter
// =============================================================================

mod create_acknowledgment_letter_tests {
    use super::*;

    fn create_priced_order_with_shipping() -> PricedOrderWithShippingMethod {
        let order_id = OrderId::create("OrderId", "order-001").unwrap();
        let customer_info =
            CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
        let address =
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
        let amount_to_bill = BillingAmount::create(Decimal::from(1000)).unwrap();

        let priced_order = PricedOrder::new(
            order_id,
            customer_info,
            address.clone(),
            address,
            amount_to_bill,
            vec![],
            PricingMethod::Standard,
        );

        let shipping_info = ShippingInfo::new(
            ShippingMethod::PostalService,
            Price::create(Decimal::from(10)).unwrap(),
        );

        PricedOrderWithShippingMethod::new(shipping_info, priced_order)
    }

    #[rstest]
    fn test_creates_html_string() {
        let order = create_priced_order_with_shipping();

        let html = create_acknowledgment_letter(&order);

        assert!(html.value().contains("<h1>Order Confirmation</h1>"));
    }

    #[rstest]
    fn test_includes_order_id() {
        let order = create_priced_order_with_shipping();

        let html = create_acknowledgment_letter(&order);

        assert!(html.value().contains("order-001"));
    }
}

// =============================================================================
// Tests for send_acknowledgment
// =============================================================================

mod send_acknowledgment_tests {
    use super::*;
    use order_taking_sample::simple_types::EmailAddress;

    fn create_acknowledgment() -> OrderAcknowledgment {
        let email = EmailAddress::create("EmailAddress", "john@example.com").unwrap();
        let letter = HtmlString::new("<p>Test</p>".to_string());
        OrderAcknowledgment::new(email, letter)
    }

    #[rstest]
    fn test_returns_io() {
        let ack = create_acknowledgment();

        let io_result = send_acknowledgment(&ack);

        // Execute the IO monad and verify the result
        let result = io_result.run_unsafe();
        assert!(matches!(result, SendResult::Sent));
    }

    #[rstest]
    fn test_always_returns_sent() {
        let ack = create_acknowledgment();

        let io_result = send_acknowledgment(&ack);
        let result = io_result.run_unsafe();

        assert!(matches!(result, SendResult::Sent));
    }
}
