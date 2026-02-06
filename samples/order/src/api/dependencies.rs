//! Dummy dependency functions
//!
//! Provides dummy dependency functions to be injected into the `PlaceOrder` workflow.
//! In a real application, these would be replaced with actual external service integrations.

use std::rc::Rc;

use lambars::effect::IO;
use rust_decimal::Decimal;

use crate::simple_types::{Price, ProductCode};
use crate::workflow::UnvalidatedAddress;
use crate::workflow::acknowledgment_types::{HtmlString, OrderAcknowledgment, SendResult};
use crate::workflow::priced_types::PricedOrder;
use crate::workflow::shipping_types::PricedOrderWithShippingMethod;
use crate::workflow::validated_types::{AddressValidationError, CheckedAddress, PricingMethod};

// =============================================================================
// check_product_exists (REQ-090)
// =============================================================================

/// Dummy function to check whether a product exists
///
/// Returns `true` if the product code format is valid, treating it as existing.
///
/// # Arguments
///
/// * `product_code` - Product code to check
///
/// # Returns
///
/// `true` if the product exists
///
/// # Examples
///
/// ```
/// use order_taking_sample::api::check_product_exists;
/// use order_taking_sample::simple_types::ProductCode;
///
/// let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
/// assert!(check_product_exists(&product_code));
/// ```
#[must_use]
pub const fn check_product_exists(_product_code: &ProductCode) -> bool {
    // Dummy implementation: treat all products as existing
    true
}

// =============================================================================
// check_address_exists (REQ-090)
// =============================================================================

/// Dummy function to check whether an address is valid
///
/// Returns `Ok(CheckedAddress)` if the address format is valid.
///
/// # Arguments
///
/// * `address` - Address to check
///
/// # Returns
///
/// * `Ok(CheckedAddress)` - When the address is valid
/// * `Err(AddressValidationError)` - When the address is invalid (does not occur in this dummy implementation)
///
/// # Errors
///
/// This dummy implementation does not return errors.
/// In a real implementation, `AddressValidationError` would be returned when the address is not found.
///
/// # Examples
///
/// ```
/// use order_taking_sample::api::check_address_exists;
/// use order_taking_sample::workflow::UnvalidatedAddress;
///
/// let address = UnvalidatedAddress::new(
///     "123 Main St".to_string(),
///     "".to_string(),
///     "".to_string(),
///     "".to_string(),
///     "New York".to_string(),
///     "10001".to_string(),
///     "NY".to_string(),
///     "USA".to_string(),
/// );
///
/// let result = check_address_exists(&address);
/// assert!(result.is_ok());
/// ```
pub fn check_address_exists(
    address: &UnvalidatedAddress,
) -> Result<CheckedAddress, AddressValidationError> {
    // Dummy implementation: treat all addresses as valid and wrap in CheckedAddress
    Ok(CheckedAddress::new(address.clone()))
}

// =============================================================================
// get_pricing_function (REQ-090)
// =============================================================================

/// Dummy function that returns a pricing function
///
/// Generates a function that returns a fixed price based on the product code.
///
/// # Arguments
///
/// * `_pricing_method` - Pricing method (not used in this dummy implementation)
///
/// # Returns
///
/// A function that takes a product code and returns a price
///
/// # Examples
///
/// ```
/// use order_taking_sample::api::get_pricing_function;
/// use order_taking_sample::simple_types::ProductCode;
/// use order_taking_sample::workflow::PricingMethod;
/// use rust_decimal::Decimal;
///
/// let pricing_fn = get_pricing_function(&PricingMethod::Standard);
/// let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
/// let price = pricing_fn(&product_code);
///
/// assert_eq!(price.value(), Decimal::from(100));
/// ```
#[must_use]
pub fn get_pricing_function(_pricing_method: &PricingMethod) -> Rc<dyn Fn(&ProductCode) -> Price> {
    // Dummy implementation: fixed price of 100 for Widget, 50 for Gizmo
    Rc::new(|product_code: &ProductCode| match product_code {
        ProductCode::Widget(_) => Price::unsafe_create(Decimal::from(100)),
        ProductCode::Gizmo(_) => Price::unsafe_create(Decimal::from(50)),
    })
}

// =============================================================================
// calculate_shipping_cost (REQ-090)
// =============================================================================

/// Dummy function to calculate shipping cost
///
/// Returns a fixed shipping cost.
///
/// # Arguments
///
/// * `_priced_order` - Priced order (not used in this dummy implementation)
///
/// # Returns
///
/// Shipping cost (fixed at 10)
///
/// # Examples
///
/// ```ignore
/// use order_taking_sample::api::calculate_shipping_cost;
///
/// // Create and pass a PricedOrder (omitted)
/// // let cost = calculate_shipping_cost(&priced_order);
/// // assert_eq!(cost.value(), Decimal::from(10));
/// ```
#[must_use]
pub fn calculate_shipping_cost(_priced_order: &PricedOrder) -> Price {
    // Dummy implementation: fixed shipping cost
    Price::unsafe_create(Decimal::from(10))
}

// =============================================================================
// create_acknowledgment_letter (REQ-090)
// =============================================================================

/// Dummy function to generate an acknowledgment letter
///
/// Generates HTML for an acknowledgment email from order information.
///
/// # Arguments
///
/// * `order` - Priced order with shipping method
///
/// # Returns
///
/// HTML for the acknowledgment email
///
/// # Examples
///
/// ```ignore
/// use order_taking_sample::api::create_acknowledgment_letter;
///
/// // Create and pass a PricedOrderWithShippingMethod (omitted)
/// // let html = create_acknowledgment_letter(&order);
/// // assert!(html.value().contains("<p>"));
/// ```
#[must_use]
pub fn create_acknowledgment_letter(order: &PricedOrderWithShippingMethod) -> HtmlString {
    // Dummy implementation: generate simple HTML
    let order_id = order.priced_order().order_id().value();
    HtmlString::new(format!(
        "<h1>Order Confirmation</h1><p>Your order {order_id} has been received.</p>"
    ))
}

// =============================================================================
// send_acknowledgment (REQ-090)
// =============================================================================

/// Dummy function to send an acknowledgment email
///
/// Returns the result of sending the acknowledgment email.
///
/// # Arguments
///
/// * `_acknowledgment` - Acknowledgment information to send
///
/// # Returns
///
/// `IO<SendResult>` returning the send result
///
/// # Examples
///
/// ```ignore
/// use order_taking_sample::api::send_acknowledgment;
///
/// // Create and pass an OrderAcknowledgment (omitted)
/// // let io_result = send_acknowledgment(&acknowledgment);
/// // let result = io_result.run_unsafe();
/// // assert!(matches!(result, SendResult::Sent));
/// ```
#[must_use]
pub fn send_acknowledgment(_acknowledgment: &OrderAcknowledgment) -> IO<SendResult> {
    // Dummy implementation: always return success
    IO::pure(SendResult::Sent)
}
