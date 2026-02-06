//! `PlaceOrder` workflow
//!
//! Phase 7 implementation. Integrates the entire workflow.
//!
//! # Design Principles
//!
//! - Dependency injection: receives all external dependencies as function arguments
//! - Error handling: functional error handling via Result and IO monad
//! - Composability: sequentially composes functions from each phase
//!
//! # Feature List
//!
//! - [`place_order`] - Executes the `PlaceOrder` workflow
//!
//! # Usage Examples
//!
//! ```ignore
//! use order_taking_sample::workflow::place_order;
//! // let io_result = place_order(&check_product, &check_address, &get_price, ...);
//! // let result = io_result.run_unsafe();
//! ```

use lambars::compose;
use lambars::effect::IO;
use std::rc::Rc;

use crate::simple_types::{Price, ProductCode};
use crate::workflow::UnvalidatedAddress;
use crate::workflow::acknowledgment_types::{HtmlString, OrderAcknowledgment, SendResult};
use crate::workflow::error_types::PlaceOrderError;
use crate::workflow::events::create_events;
use crate::workflow::output_types::PlaceOrderEvent;
use crate::workflow::priced_types::PricedOrder;
use crate::workflow::pricing::price_order;
use crate::workflow::shipping::{acknowledge_order, add_shipping_info_to_order, free_vip_shipping};
use crate::workflow::shipping_types::PricedOrderWithShippingMethod;
use crate::workflow::unvalidated_types::UnvalidatedOrder;
use crate::workflow::validated_types::{AddressValidationError, CheckedAddress, PricingMethod};
use crate::workflow::validation::validate_order;

// =============================================================================
// place_order (REQ-074)
// =============================================================================

/// Function that integrates the entire `PlaceOrder` workflow
///
/// Receives an unvalidated order, sequentially executes all processing steps,
/// and returns either an event list or an error.
///
/// # Processing Flow
///
/// 1. `validate_order` - Validates the unvalidated order (error: Validation)
/// 2. `price_order` - Calculates prices (error: Pricing)
/// 3. `add_shipping_info_to_order` - Shipping informationaddition
/// 4. `free_vip_shipping` - VIP freeshippingapplication
/// 5. `acknowledge_order` - Sends acknowledgment email (IO monad)
/// 6. `create_events` - Event generation
///
/// # Type Parameters
///
/// * `CheckProduct` - Function type for checking product existence
/// * `CheckAddress` - addressverificationfunctiontype
/// * `GetPricingFn` - Function type that returns a price retrieval function
/// * `CalculateShipping` - Function type for calculating shipping cost
/// * `CreateLetter` - Function type for creating acknowledgment emails
/// * `SendAcknowledgment` - Function type for sending acknowledgment email (returns IO)
///
/// # Arguments
///
/// * `check_product_exists` - Function to check product existence
/// * `check_address_exists` - addressverificationfunction
/// * `get_pricing_function` - Function that returns a price retrieval function
/// * `calculate_shipping_cost` - Function to calculate shipping cost
/// * `create_acknowledgment_letter` - Function to create an acknowledgment email
/// * `send_acknowledgment` - verificationEmail sending function
/// * `unvalidated_order` - unvalidatedorder
///
/// # Returns
///
/// `IO<Result<Vec<PlaceOrderEvent>, PlaceOrderError>>`
/// - On success: `Ok(Vec<PlaceOrderEvent>)` (1-3 events)
/// - On failure: `Err(PlaceOrderError)` (Validation or Pricing)
///
/// # Examples
///
/// ```ignore
/// use order_taking_sample::workflow::place_order;
/// use lambars::effect::IO;
///
/// let io_result = place_order(
///     &check_product,
///     &check_address,
///     &get_pricing_fn,
///     &calculate_shipping,
///     &create_letter,
///     &send_ack,
///     unvalidated_order,
/// );
///
/// // Execute the IO monad
/// let result = io_result.run_unsafe();
/// match result {
///     Ok(events) => println!("Events: {:?}", events),
///     Err(error) => println!("Error: {:?}", error),
/// }
/// ```
pub fn place_order<
    CheckProduct,
    CheckAddress,
    GetPricingFn,
    CalculateShipping,
    CreateLetter,
    SendAcknowledgment,
>(
    check_product_exists: &CheckProduct,
    check_address_exists: &CheckAddress,
    get_pricing_function: &GetPricingFn,
    calculate_shipping_cost: &CalculateShipping,
    create_acknowledgment_letter: &CreateLetter,
    send_acknowledgment: &SendAcknowledgment,
    unvalidated_order: &UnvalidatedOrder,
) -> IO<Result<Vec<PlaceOrderEvent>, PlaceOrderError>>
where
    CheckProduct: Fn(&ProductCode) -> bool,
    CheckAddress: Fn(&UnvalidatedAddress) -> Result<CheckedAddress, AddressValidationError>,
    GetPricingFn: Fn(&PricingMethod) -> Rc<dyn Fn(&ProductCode) -> Price>,
    CalculateShipping: Fn(&PricedOrder) -> Price,
    CreateLetter: Fn(&PricedOrderWithShippingMethod) -> HtmlString,
    SendAcknowledgment: Fn(&OrderAcknowledgment) -> IO<SendResult>,
{
    // Step 1: Validation
    let validated_order = match validate_order(
        check_product_exists,
        check_address_exists,
        unvalidated_order,
    ) {
        Ok(order) => order,
        Err(error) => return IO::pure(Err(error)),
    };

    // Step 2: Calculate prices
    let priced_order = match price_order(get_pricing_function, &validated_order) {
        Ok(order) => order,
        Err(error) => return IO::pure(Err(error)),
    };

    // Step 3-4: Define the shipping processing pipeline as a composition function
    // Partially apply add_shipping_info_to_order
    let add_shipping =
        |order: &PricedOrder| add_shipping_info_to_order(calculate_shipping_cost, order);

    // Compose free_vip_shipping and add_shipping
    // compose! composes right to left: free_vip_shipping(add_shipping(order))
    let process_shipping = compose!(free_vip_shipping, add_shipping);

    // Apply the composed function
    let priced_order_with_shipping = process_shipping(&priced_order);

    // Step 5: Send acknowledgment email (IO monad)
    let acknowledgment_io = acknowledge_order(
        create_acknowledgment_letter,
        send_acknowledgment,
        &priced_order_with_shipping,
    );

    // Step 6: Event generation (executed within IO)
    acknowledgment_io
        .fmap(move |acknowledgment_option| Ok(create_events(&priced_order, acknowledgment_option)))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::{UnvalidatedAddress, UnvalidatedCustomerInfo, UnvalidatedOrderLine};
    use rstest::rstest;
    use rust_decimal::Decimal;

    // =========================================================================
    // Test helpers
    // =========================================================================

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

    fn always_valid_address()
    -> impl Fn(&UnvalidatedAddress) -> Result<CheckedAddress, AddressValidationError> {
        |addr: &UnvalidatedAddress| Ok(CheckedAddress::new(addr.clone()))
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

    // =========================================================================
    // Tests for place_order
    // =========================================================================

    #[rstest]
    fn test_place_order_success() {
        let order = create_valid_order();
        let check_product = always_exists_product();
        let check_address = always_valid_address();
        let get_pricing_fn = fixed_price_function(Decimal::from(100));
        let calculate_shipping = calculate_shipping_cost_mock();
        let create_letter = create_letter_mock();
        let send_acknowledgment = always_send();

        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &calculate_shipping,
            &create_letter,
            &send_acknowledgment,
            &order,
        );

        let result = io_result.run_unsafe();
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 3);
    }

    #[rstest]
    fn test_place_order_validation_error() {
        let order = UnvalidatedOrder::new(
            "".to_string(), // Invalid order ID
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

        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &calculate_shipping,
            &create_letter,
            &send_acknowledgment,
            &order,
        );

        let result = io_result.run_unsafe();
        assert!(result.is_err());
        assert!(result.unwrap_err().is_validation());
    }

    #[rstest]
    fn test_place_order_mail_not_sent() {
        let order = create_valid_order();
        let check_product = always_exists_product();
        let check_address = always_valid_address();
        let get_pricing_fn = fixed_price_function(Decimal::from(100));
        let calculate_shipping = calculate_shipping_cost_mock();
        let create_letter = create_letter_mock();
        let send_acknowledgment = never_send();

        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &calculate_shipping,
            &create_letter,
            &send_acknowledgment,
            &order,
        );

        let result = io_result.run_unsafe();
        assert!(result.is_ok());
        let events = result.unwrap();
        // Succeeds even on email send failure, no AcknowledgmentSent
        assert_eq!(events.len(), 2);
    }
}
