//! PlaceOrder ワークフローのテスト
//!
//! Phase 7 の place_order 関数に対するユニットテストと統合テスト。

use lambars::effect::IO;
use order_taking_sample::simple_types::{Price, ProductCode};
use order_taking_sample::workflow::{
    AddressValidationError, CheckedAddress, HtmlString, OrderAcknowledgment, PricedOrder,
    PricedOrderWithShippingMethod, PricingMethod, SendResult, UnvalidatedAddress,
    UnvalidatedCustomerInfo, UnvalidatedOrder, UnvalidatedOrderLine,
};
use rstest::rstest;
use rust_decimal::Decimal;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

// =============================================================================
// テストヘルパー関数
// =============================================================================

fn create_valid_customer_info() -> UnvalidatedCustomerInfo {
    UnvalidatedCustomerInfo::new(
        "John".to_string(),
        "Doe".to_string(),
        "john@example.com".to_string(),
        "Normal".to_string(),
    )
}

fn create_valid_address() -> UnvalidatedAddress {
    UnvalidatedAddress::new(
        "123 Main St".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "New York".to_string(),
        "10001".to_string(),
        "NY".to_string(),
        "US".to_string(),
    )
}

fn create_valid_order_line() -> UnvalidatedOrderLine {
    UnvalidatedOrderLine::new(
        "line-001".to_string(),
        "W1234".to_string(),
        Decimal::from(10),
    )
}

fn create_valid_order() -> UnvalidatedOrder {
    UnvalidatedOrder::new(
        "order-001".to_string(),
        create_valid_customer_info(),
        create_valid_address(),
        create_valid_address(),
        vec![create_valid_order_line()],
        "".to_string(),
    )
}

fn always_exists_product() -> impl Fn(&ProductCode) -> bool {
    |_: &ProductCode| true
}

fn never_exists_product() -> impl Fn(&ProductCode) -> bool {
    |_: &ProductCode| false
}

fn always_valid_address()
-> impl Fn(&UnvalidatedAddress) -> Result<CheckedAddress, AddressValidationError> {
    |addr: &UnvalidatedAddress| Ok(CheckedAddress::new(addr.clone()))
}

fn address_not_found()
-> impl Fn(&UnvalidatedAddress) -> Result<CheckedAddress, AddressValidationError> {
    |_: &UnvalidatedAddress| Err(AddressValidationError::AddressNotFound)
}

fn fixed_price_function(
    price: Decimal,
) -> impl Fn(&PricingMethod) -> Rc<dyn Fn(&ProductCode) -> Price> {
    move |_: &PricingMethod| {
        let price_clone = price;
        Rc::new(move |_: &ProductCode| Price::unsafe_create(price_clone))
    }
}

fn calculate_shipping_cost_mock() -> impl Fn(&PricedOrder) -> Price {
    |_: &PricedOrder| Price::unsafe_create(Decimal::from(10))
}

fn create_letter_mock() -> impl Fn(&PricedOrderWithShippingMethod) -> HtmlString {
    |_: &PricedOrderWithShippingMethod| HtmlString::new("<p>Order confirmed</p>".to_string())
}

fn always_send() -> impl Fn(&OrderAcknowledgment) -> IO<SendResult> {
    |_: &OrderAcknowledgment| IO::pure(SendResult::Sent)
}

fn never_send() -> impl Fn(&OrderAcknowledgment) -> IO<SendResult> {
    |_: &OrderAcknowledgment| IO::pure(SendResult::NotSent)
}

// =============================================================================
// place_order テスト (REQ-074)
// =============================================================================

mod place_order_tests {
    use super::*;
    use order_taking_sample::workflow::place_order;

    #[rstest]
    fn test_success_full_flow() {
        // Arrange
        let order = create_valid_order();
        let check_product = always_exists_product();
        let check_address = always_valid_address();
        let get_pricing_fn = fixed_price_function(Decimal::from(100));
        let calculate_shipping = calculate_shipping_cost_mock();
        let create_letter = create_letter_mock();
        let send_acknowledgment = always_send();

        // Act
        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &calculate_shipping,
            &create_letter,
            &send_acknowledgment,
            &order,
        );

        // Assert
        let result = io_result.run_unsafe();
        assert!(result.is_ok());
        let events = result.unwrap();
        // AcknowledgmentSent + ShippableOrderPlaced + BillableOrderPlaced = 3
        assert_eq!(events.len(), 3);
        assert!(events[0].is_acknowledgment());
        assert!(events[1].is_shippable());
        assert!(events[2].is_billable());
    }

    #[rstest]
    fn test_validation_error_invalid_order_id() {
        // Arrange
        let order = UnvalidatedOrder::new(
            "".to_string(), // 空の注文ID
            create_valid_customer_info(),
            create_valid_address(),
            create_valid_address(),
            vec![create_valid_order_line()],
            "".to_string(),
        );
        let check_product = always_exists_product();
        let check_address = always_valid_address();
        let get_pricing_fn = fixed_price_function(Decimal::from(100));
        let calculate_shipping = calculate_shipping_cost_mock();
        let create_letter = create_letter_mock();
        let send_acknowledgment = always_send();

        // Act
        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &calculate_shipping,
            &create_letter,
            &send_acknowledgment,
            &order,
        );

        // Assert
        let result = io_result.run_unsafe();
        assert!(result.is_err());
        assert!(result.unwrap_err().is_validation());
    }

    #[rstest]
    fn test_validation_error_product_not_exists() {
        // Arrange
        let order = create_valid_order();
        let check_product = never_exists_product();
        let check_address = always_valid_address();
        let get_pricing_fn = fixed_price_function(Decimal::from(100));
        let calculate_shipping = calculate_shipping_cost_mock();
        let create_letter = create_letter_mock();
        let send_acknowledgment = always_send();

        // Act
        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &calculate_shipping,
            &create_letter,
            &send_acknowledgment,
            &order,
        );

        // Assert
        let result = io_result.run_unsafe();
        assert!(result.is_err());
        assert!(result.unwrap_err().is_validation());
    }

    #[rstest]
    fn test_validation_error_address_not_found() {
        // Arrange
        let order = create_valid_order();
        let check_product = always_exists_product();
        let check_address = address_not_found();
        let get_pricing_fn = fixed_price_function(Decimal::from(100));
        let calculate_shipping = calculate_shipping_cost_mock();
        let create_letter = create_letter_mock();
        let send_acknowledgment = always_send();

        // Act
        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &calculate_shipping,
            &create_letter,
            &send_acknowledgment,
            &order,
        );

        // Assert
        let result = io_result.run_unsafe();
        assert!(result.is_err());
        assert!(result.unwrap_err().is_validation());
    }

    #[rstest]
    fn test_mail_not_sent_still_success() {
        // Arrange
        let order = create_valid_order();
        let check_product = always_exists_product();
        let check_address = always_valid_address();
        let get_pricing_fn = fixed_price_function(Decimal::from(100));
        let calculate_shipping = calculate_shipping_cost_mock();
        let create_letter = create_letter_mock();
        let send_acknowledgment = never_send();

        // Act
        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &calculate_shipping,
            &create_letter,
            &send_acknowledgment,
            &order,
        );

        // Assert
        let result = io_result.run_unsafe();
        assert!(result.is_ok());
        let events = result.unwrap();
        // メール送信失敗でも成功、AcknowledgmentSent なし
        // ShippableOrderPlaced + BillableOrderPlaced = 2
        assert_eq!(events.len(), 2);
        assert!(events[0].is_shippable());
        assert!(events[1].is_billable());
    }

    #[rstest]
    fn test_vip_free_shipping() {
        // Arrange
        let vip_customer = UnvalidatedCustomerInfo::new(
            "Jane".to_string(),
            "VIP".to_string(),
            "jane@example.com".to_string(),
            "VIP".to_string(),
        );
        let order = UnvalidatedOrder::new(
            "order-vip-001".to_string(),
            vip_customer,
            create_valid_address(),
            create_valid_address(),
            vec![create_valid_order_line()],
            "".to_string(),
        );
        let check_product = always_exists_product();
        let check_address = always_valid_address();
        let get_pricing_fn = fixed_price_function(Decimal::from(100));
        let calculate_shipping = calculate_shipping_cost_mock();
        let create_letter = create_letter_mock();
        let send_acknowledgment = always_send();

        // Act
        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &calculate_shipping,
            &create_letter,
            &send_acknowledgment,
            &order,
        );

        // Assert
        let result = io_result.run_unsafe();
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 3);
    }

    #[rstest]
    fn test_io_deferred_execution() {
        // Arrange
        let order = create_valid_order();
        let check_product = always_exists_product();
        let check_address = always_valid_address();
        let get_pricing_fn = fixed_price_function(Decimal::from(100));
        let calculate_shipping = calculate_shipping_cost_mock();
        let create_letter = create_letter_mock();

        // 副作用が実行されたかどうかを追跡
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let mock_send = move |_: &OrderAcknowledgment| {
            let flag = executed_clone.clone();
            IO::new(move || {
                flag.store(true, Ordering::SeqCst);
                SendResult::Sent
            })
        };

        // Act
        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &calculate_shipping,
            &create_letter,
            &mock_send,
            &order,
        );

        // IO が生成されただけでは実行されない
        assert!(!executed.load(Ordering::SeqCst));

        // run_unsafe() で実行される
        let result = io_result.run_unsafe();
        assert!(result.is_ok());
        assert!(executed.load(Ordering::SeqCst));
    }

    #[rstest]
    fn test_with_promotion_code() {
        // Arrange
        let order = UnvalidatedOrder::new(
            "order-promo-001".to_string(),
            create_valid_customer_info(),
            create_valid_address(),
            create_valid_address(),
            vec![create_valid_order_line()],
            "SUMMER2024".to_string(),
        );
        let check_product = always_exists_product();
        let check_address = always_valid_address();
        let get_pricing_fn = fixed_price_function(Decimal::from(80)); // 割引価格
        let calculate_shipping = calculate_shipping_cost_mock();
        let create_letter = create_letter_mock();
        let send_acknowledgment = always_send();

        // Act
        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &calculate_shipping,
            &create_letter,
            &send_acknowledgment,
            &order,
        );

        // Assert
        let result = io_result.run_unsafe();
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 3);
    }

    #[rstest]
    fn test_multiple_order_lines() {
        // Arrange
        let lines = vec![
            UnvalidatedOrderLine::new(
                "line-001".to_string(),
                "W1234".to_string(),
                Decimal::from(5),
            ),
            UnvalidatedOrderLine::new(
                "line-002".to_string(),
                "G123".to_string(),
                Decimal::new(25, 1), // 2.5
            ),
        ];
        let order = UnvalidatedOrder::new(
            "order-multi-001".to_string(),
            create_valid_customer_info(),
            create_valid_address(),
            create_valid_address(),
            lines,
            "".to_string(),
        );
        let check_product = always_exists_product();
        let check_address = always_valid_address();
        let get_pricing_fn = fixed_price_function(Decimal::from(100));
        let calculate_shipping = calculate_shipping_cost_mock();
        let create_letter = create_letter_mock();
        let send_acknowledgment = always_send();

        // Act
        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &calculate_shipping,
            &create_letter,
            &send_acknowledgment,
            &order,
        );

        // Assert
        let result = io_result.run_unsafe();
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 3);
    }
}
