//! Shipping cost calculation and acknowledgment email sending
//!
//! Phase 6 implementation. Classifies shipping regions, calculates shipping costs,
//! provides free shipping for VIP customers, and sends order acknowledgment emails.
//!
//! # Feature List
//!
//! - [`ShippingRegion`] - Shipping region classification (US local, US remote, international)
//! - [`classify_shipping_region`] - Classifies shipping region from address
//! - [`calculate_shipping_cost`] - Calculates cost based on shipping region
//! - [`add_shipping_info_to_order`] - Adds shipping info to an order
//! - [`free_vip_shipping`] - Makes shipping free for VIP customers
//! - [`acknowledge_order`] - Sends order acknowledgment email (IO monad)
//!
//! # Usage Examples
//!
//! ```
//! use order_taking_sample::workflow::{
//!     ShippingRegion, classify_shipping_region, calculate_shipping_cost,
//!     add_shipping_info_to_order, free_vip_shipping,
//! };
//! use order_taking_sample::compound_types::Address;
//!
//! // Classify shipping region from address
//! let address = Address::create(
//!     "123 Main St", "", "", "", "Los Angeles", "90001", "CA", "US"
//! ).unwrap();
//! let region = classify_shipping_region(&address);
//! assert!(region.is_us_local_state());
//! ```

use lambars::effect::IO;
use lambars::optics::Lens;

use crate::compound_types::Address;
use crate::simple_types::{Price, VipStatus};
use crate::workflow::acknowledgment_types::{HtmlString, OrderAcknowledgment, SendResult};
use crate::workflow::output_types::OrderAcknowledgmentSent;
use crate::workflow::priced_types::PricedOrder;
use crate::workflow::shipping_types::{
    PricedOrderWithShippingMethod, ShippingInfo, ShippingMethod,
};

// =============================================================================
// Constant definitions
// =============================================================================

/// List of US local state codes
///
/// West coast states near California.
const US_LOCAL_STATES: [&str; 4] = ["CA", "OR", "AZ", "NV"];

/// Country name variations representing the United States
const US_COUNTRY_NAMES: [&str; 3] = ["US", "USA", "United States"];

/// Shipping cost for US local states (in dollars)
const LOCAL_STATE_SHIPPING_COST: u32 = 5;

/// Shipping cost for US remote states (in dollars)
const REMOTE_STATE_SHIPPING_COST: u32 = 10;

/// International shipping cost (in dollars)
const INTERNATIONAL_SHIPPING_COST: u32 = 20;

// =============================================================================
// ShippingRegion enum
// =============================================================================

/// Shipping region classification
///
/// Classifies a shipping address into one of three categories.
/// Shipping cost is determined based on the category.
///
/// # Categories
///
/// - [`UsLocalState`](ShippingRegion::UsLocalState) - US local states (CA, OR, AZ, NV)
/// - [`UsRemoteState`](ShippingRegion::UsRemoteState) - US remote state (other US states)
/// - [`International`](ShippingRegion::International) - International (non-US)
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::ShippingRegion;
///
/// let region = ShippingRegion::UsLocalState;
/// assert!(region.is_us_local_state());
/// assert!(!region.is_us_remote_state());
/// assert!(!region.is_international());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShippingRegion {
    /// US local states (CA, OR, AZ, NV)
    ///
    /// The lowest shipping rate is applied.
    UsLocalState,

    /// US remote states (US states other than local ones)
    ///
    /// A moderate shipping rate is applied.
    UsRemoteState,

    /// International shipping (outside the US)
    ///
    /// The highest shipping rate is applied.
    International,
}

impl ShippingRegion {
    /// Returns whether this is the `UsLocalState` variant
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::ShippingRegion;
    ///
    /// let region = ShippingRegion::UsLocalState;
    /// assert!(region.is_us_local_state());
    /// ```
    #[must_use]
    pub const fn is_us_local_state(&self) -> bool {
        matches!(self, Self::UsLocalState)
    }

    /// Returns whether this is the `UsRemoteState` variant
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::ShippingRegion;
    ///
    /// let region = ShippingRegion::UsRemoteState;
    /// assert!(region.is_us_remote_state());
    /// ```
    #[must_use]
    pub const fn is_us_remote_state(&self) -> bool {
        matches!(self, Self::UsRemoteState)
    }

    /// Returns whether this is the `International` variant
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::ShippingRegion;
    ///
    /// let region = ShippingRegion::International;
    /// assert!(region.is_international());
    /// ```
    #[must_use]
    pub const fn is_international(&self) -> bool {
        matches!(self, Self::International)
    }
}

// =============================================================================
// classify_shipping_region function
// =============================================================================

/// Classifies the shipping region from an address
///
/// Determines the [`ShippingRegion`] based on the country and state code of the address.
///
/// # Classification Rules
///
/// 1. If country is not "US", "USA", or "United States" -> [`International`](ShippingRegion::International)
/// 2. If state code is "CA", "OR", "AZ", or "NV" -> [`UsLocalState`](ShippingRegion::UsLocalState)
/// 3. Otherwise within US -> [`UsRemoteState`](ShippingRegion::UsRemoteState)
///
/// # Arguments
///
/// * `address` - Address to classify
///
/// # Returns
///
/// A [`ShippingRegion`] representing the shipping region
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::classify_shipping_region;
/// use order_taking_sample::compound_types::Address;
///
/// // California is a local state
/// let ca_address = Address::create(
///     "123 Main St", "", "", "", "Los Angeles", "90001", "CA", "US"
/// ).unwrap();
/// let region = classify_shipping_region(&ca_address);
/// assert!(region.is_us_local_state());
///
/// // New York is a remote state
/// let ny_address = Address::create(
///     "456 Broadway", "", "", "", "New York", "10001", "NY", "US"
/// ).unwrap();
/// let region = classify_shipping_region(&ny_address);
/// assert!(region.is_us_remote_state());
///
/// // Japan is international (UsStateCode requires a valid US state code, but determination is by country)
/// let jp_address = Address::create(
///     "1-1-1 Shibuya", "", "", "", "Tokyo", "15000", "CA", "Japan"
/// ).unwrap();
/// let region = classify_shipping_region(&jp_address);
/// assert!(region.is_international());
/// ```
#[must_use]
pub fn classify_shipping_region(address: &Address) -> ShippingRegion {
    let country = address.country().value();
    let state = address.state().value();

    // International shipping if the country is not the US
    let is_us = US_COUNTRY_NAMES.contains(&country);
    if !is_us {
        return ShippingRegion::International;
    }

    // Determine if it is a local state
    let is_local = US_LOCAL_STATES.contains(&state);
    if is_local {
        ShippingRegion::UsLocalState
    } else {
        ShippingRegion::UsRemoteState
    }
}

// =============================================================================
// calculate_shipping_cost function
// =============================================================================

/// Calculates shipping cost from a priced order
///
/// Determines shipping cost based on the shipping region classification of the address.
///
/// # Shipping Cost
///
/// | Region | Cost |
/// |------|--------|
/// | US local states (CA, OR, AZ, NV) | $5 |
/// | US remote states | $10 |
/// | International | $20 |
///
/// # Arguments
///
/// * `priced_order` - Priced order
///
/// # Returns
///
/// A [`Price`] representing the shipping cost
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::calculate_shipping_cost;
/// use order_taking_sample::workflow::{PricedOrder, PricingMethod};
/// use order_taking_sample::compound_types::{CustomerInfo, Address};
/// use order_taking_sample::simple_types::{OrderId, BillingAmount};
/// use rust_decimal::Decimal;
///
/// let order_id = OrderId::create("OrderId", "order-001").unwrap();
/// let customer = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
/// let address = Address::create(
///     "123 Main St", "", "", "", "Los Angeles", "90001", "CA", "US"
/// ).unwrap();
/// let amount = BillingAmount::create(Decimal::from(100)).unwrap();
///
/// let priced_order = PricedOrder::new(
///     order_id, customer, address.clone(), address, amount, vec![], PricingMethod::Standard
/// );
///
/// let cost = calculate_shipping_cost(&priced_order);
/// assert_eq!(cost.value(), Decimal::from(5)); // CA is UsLocalState
/// ```
#[must_use]
pub fn calculate_shipping_cost(priced_order: &PricedOrder) -> Price {
    let region = classify_shipping_region(priced_order.shipping_address());

    let cost = match region {
        ShippingRegion::UsLocalState => LOCAL_STATE_SHIPPING_COST,
        ShippingRegion::UsRemoteState => REMOTE_STATE_SHIPPING_COST,
        ShippingRegion::International => INTERNATIONAL_SHIPPING_COST,
    };

    Price::unsafe_create(rust_decimal::Decimal::from(cost))
}

// =============================================================================
// add_shipping_info_to_order function
// =============================================================================

/// Adds shipping information to a priced order
///
/// Using the shipping cost calculation function passed as an argument,
/// generates a new order with shipping information.
///
/// # Shipping Method
///
/// Defaults to `Fedex24` (24-hour delivery).
///
/// # Arguments
///
/// * `calculate_shipping_cost_function` - Function to calculate shipping cost
/// * `priced_order` - Priced order
///
/// # Returns
///
/// A [`PricedOrderWithShippingMethod`] with shipping information
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{
///     add_shipping_info_to_order, calculate_shipping_cost,
///     PricedOrder, PricingMethod,
/// };
/// use order_taking_sample::compound_types::{CustomerInfo, Address};
/// use order_taking_sample::simple_types::{OrderId, BillingAmount};
/// use rust_decimal::Decimal;
///
/// let order_id = OrderId::create("OrderId", "order-001").unwrap();
/// let customer = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
/// let address = Address::create(
///     "123 Main St", "", "", "", "New York", "10001", "NY", "US"
/// ).unwrap();
/// let amount = BillingAmount::create(Decimal::from(100)).unwrap();
///
/// let priced_order = PricedOrder::new(
///     order_id, customer, address.clone(), address, amount, vec![], PricingMethod::Standard
/// );
///
/// let order_with_shipping = add_shipping_info_to_order(&calculate_shipping_cost, &priced_order);
/// assert!(order_with_shipping.shipping_info().shipping_method().is_fedex24());
/// assert_eq!(order_with_shipping.shipping_info().shipping_cost().value(), Decimal::from(10));
/// ```
#[must_use]
pub fn add_shipping_info_to_order<F>(
    calculate_shipping_cost_function: &F,
    priced_order: &PricedOrder,
) -> PricedOrderWithShippingMethod
where
    F: Fn(&PricedOrder) -> Price,
{
    let shipping_cost = calculate_shipping_cost_function(priced_order);
    let shipping_method = ShippingMethod::Fedex24;
    let shipping_info = ShippingInfo::new(shipping_method, shipping_cost);

    PricedOrderWithShippingMethod::new(shipping_info, priced_order.clone())
}

// =============================================================================
// free_vip_shipping function
// =============================================================================

/// Waives shipping charges for VIP customers
///
/// If the customer is a VIP, sets shipping cost to $0 and changes the shipping method to `Fedex24`.
/// For regular customers, the original shipping information is retained as-is.
///
/// # Using Lens
///
/// This function uses `PricedOrderWithShippingMethod::shipping_info_lens()` to
/// immutably update the shipping information.
///
/// # Arguments
///
/// * `order` - Order with shipping information
///
/// # Returns
///
/// A [`PricedOrderWithShippingMethod`] with VIP discount applied
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{
///     free_vip_shipping, PricedOrder, PricedOrderWithShippingMethod,
///     ShippingInfo, ShippingMethod, PricingMethod,
/// };
/// use order_taking_sample::compound_types::{CustomerInfo, Address};
/// use order_taking_sample::simple_types::{OrderId, BillingAmount, Price};
/// use rust_decimal::Decimal;
///
/// // Create a VIP customer order
/// let order_id = OrderId::create("OrderId", "order-001").unwrap();
/// let customer = CustomerInfo::create("John", "Doe", "john@example.com", "VIP").unwrap();
/// let address = Address::create(
///     "123 Main St", "", "", "", "New York", "10001", "NY", "US"
/// ).unwrap();
/// let amount = BillingAmount::create(Decimal::from(100)).unwrap();
/// let priced_order = PricedOrder::new(
///     order_id, customer, address.clone(), address, amount, vec![], PricingMethod::Standard
/// );
///
/// let shipping_info = ShippingInfo::new(
///     ShippingMethod::PostalService,
///     Price::create(Decimal::from(10)).unwrap()
/// );
/// let order = PricedOrderWithShippingMethod::new(shipping_info, priced_order);
///
/// let updated = free_vip_shipping(order);
/// assert_eq!(updated.shipping_info().shipping_cost().value(), Decimal::ZERO);
/// assert!(updated.shipping_info().shipping_method().is_fedex24());
/// ```
#[must_use]
pub fn free_vip_shipping(order: PricedOrderWithShippingMethod) -> PricedOrderWithShippingMethod {
    let vip_status = order.priced_order().customer_info().vip_status();

    match vip_status {
        VipStatus::Vip => {
            let free_shipping_info = ShippingInfo::new(
                ShippingMethod::Fedex24,
                Price::unsafe_create(rust_decimal::Decimal::ZERO),
            );
            PricedOrderWithShippingMethod::shipping_info_lens().set(order, free_shipping_info)
        }
        VipStatus::Normal => order,
    }
}

// =============================================================================
// acknowledge_order function
// =============================================================================

/// Sends an order acknowledgment email
///
/// This function wraps side effects in an IO monad and returns them.
/// Actual email sending is deferred until `run_unsafe()` is called.
///
/// # Processing Flow
///
/// 1. Generate the acknowledgment email body (`create_letter` function)
/// 2. Create `OrderAcknowledgment` from email address and body
/// 3. Execute email sending (`send_acknowledgment` function, IO monad)
/// 4. Generate `OrderAcknowledgmentSent` event based on send result
///
/// # Arguments
///
/// * `create_letter` - Function to generate email body (HTML) from order
/// * `send_acknowledgment` - Function to send acknowledgment email (returns IO monad)
/// * `order` - Order with shipping information
///
/// # Returns
///
/// Returns `IO<Option<OrderAcknowledgmentSent>>`.
/// - On success: IO containing `Some(OrderAcknowledgmentSent)`
/// - On failure: IO containing `None`
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{
///     acknowledge_order, PricedOrder, PricedOrderWithShippingMethod,
///     ShippingInfo, ShippingMethod, PricingMethod, HtmlString,
///     OrderAcknowledgment, SendResult,
/// };
/// use order_taking_sample::compound_types::{CustomerInfo, Address};
/// use order_taking_sample::simple_types::{OrderId, BillingAmount, Price};
/// use lambars::effect::IO;
/// use rust_decimal::Decimal;
///
/// // Create the order
/// let order_id = OrderId::create("OrderId", "order-001").unwrap();
/// let customer = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
/// let address = Address::create(
///     "123 Main St", "", "", "", "New York", "10001", "NY", "US"
/// ).unwrap();
/// let amount = BillingAmount::create(Decimal::from(100)).unwrap();
/// let priced_order = PricedOrder::new(
///     order_id, customer, address.clone(), address, amount, vec![], PricingMethod::Standard
/// );
/// let shipping_info = ShippingInfo::new(
///     ShippingMethod::Fedex24,
///     Price::create(Decimal::from(10)).unwrap()
/// );
/// let order = PricedOrderWithShippingMethod::new(shipping_info, priced_order);
///
/// // Define mock functions
/// let create_letter = |_: &PricedOrderWithShippingMethod| {
///     HtmlString::new("<p>Order confirmed</p>".to_string())
/// };
/// let send_acknowledgment = |_: &OrderAcknowledgment| {
///     IO::pure(SendResult::Sent)
/// };
///
/// // Get IO monad (not yet executed)
/// let io_result = acknowledge_order(&create_letter, &send_acknowledgment, &order);
///
/// // Execute and get the result
/// let result = io_result.run_unsafe();
/// assert!(result.is_some());
/// ```
pub fn acknowledge_order<CreateLetter, SendAcknowledgment>(
    create_letter: &CreateLetter,
    send_acknowledgment: &SendAcknowledgment,
    order: &PricedOrderWithShippingMethod,
) -> IO<Option<OrderAcknowledgmentSent>>
where
    CreateLetter: Fn(&PricedOrderWithShippingMethod) -> HtmlString,
    SendAcknowledgment: Fn(&OrderAcknowledgment) -> IO<SendResult>,
{
    // Generate acknowledgment email body
    let letter = create_letter(order);

    // Get email address
    let email_address = order.priced_order().customer_info().email_address().clone();

    // Create OrderAcknowledgment
    let acknowledgment = OrderAcknowledgment::new(email_address.clone(), letter);

    // Get order ID
    let order_id = order.priced_order().order_id().clone();

    // Wrap email sending in IO
    let send_result_io = send_acknowledgment(&acknowledgment);

    // Generate OrderAcknowledgmentSent event based on send result
    send_result_io.fmap(move |send_result| match send_result {
        SendResult::Sent => Some(OrderAcknowledgmentSent::new(order_id, email_address)),
        SendResult::NotSent => None,
    })
}

// =============================================================================
// acknowledge_order_with_logging function
// =============================================================================

/// Sends an order acknowledgment email (with logging)
///
/// Uses the eff! macro to chain multiple IO operations (logging, email sending)
/// in do-notation style.
///
/// # Processing Flow
///
/// 1. Log "Creating acknowledgment letter"
/// 2. Generate acknowledgment email body (pure)
/// 3. Log "Sending acknowledgment email"
/// 4. Execute email sending
/// 5. Log "Acknowledgment process completed"
/// 6. Generate `OrderAcknowledgmentSent` event based on send result
///
/// # eff! macro syntax
///
/// - `_ <= io_action;` - Execute IO and discard the result
/// - `pattern <= io_action;` - Extract value from IO and bind it
/// - `let pattern = expr;` - Bind a pure value
/// - `IO::pure(...)` - Return the final IO
///
/// # Type Parameters
///
/// * `CreateLetter` - Function type to generate acknowledgment email body
/// * `SendAcknowledgment` - Function type to send email (returns IO monad)
/// * `LogAction` - Function type for logging (returns IO monad)
///
/// # Arguments
///
/// * `create_letter` - Acknowledgment email body generation function
/// * `send_acknowledgment` - Email sending function
/// * `log_action` - Logging function
/// * `order` - Order with shipping information
///
/// # Returns
///
/// `IO<Option<OrderAcknowledgmentSent>>` - Lazily executed result
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{
///     acknowledge_order_with_logging, PricedOrder, PricedOrderWithShippingMethod,
///     PricingMethod, ShippingInfo, ShippingMethod, HtmlString,
///     OrderAcknowledgment, SendResult,
/// };
/// use order_taking_sample::compound_types::{CustomerInfo, Address};
/// use order_taking_sample::simple_types::{OrderId, BillingAmount, Price};
/// use lambars::effect::IO;
/// use rust_decimal::Decimal;
///
/// let log_action = |message: &str| {
///     let message = message.to_string();
///     IO::new(move || println!("[LOG] {}", message))
/// };
///
/// let create_letter = |_: &PricedOrderWithShippingMethod| {
///     HtmlString::new("<p>Order confirmed</p>".to_string())
/// };
///
/// let send_acknowledgment = |_: &OrderAcknowledgment| {
///     IO::pure(SendResult::Sent)
/// };
///
/// // Test data creation omitted
/// // let io_result = acknowledge_order_with_logging(
/// //     &create_letter, &send_acknowledgment, &log_action, &order
/// // );
/// // let result = io_result.run_unsafe();
/// ```
pub fn acknowledge_order_with_logging<CreateLetter, SendAcknowledgment, LogAction>(
    create_letter: &CreateLetter,
    send_acknowledgment: &SendAcknowledgment,
    log_action: &LogAction,
    order: &PricedOrderWithShippingMethod,
) -> IO<Option<OrderAcknowledgmentSent>>
where
    CreateLetter: Fn(&PricedOrderWithShippingMethod) -> HtmlString,
    SendAcknowledgment: Fn(&OrderAcknowledgment) -> IO<SendResult>,
    LogAction: Fn(&str) -> IO<()>,
{
    // Clone required values for closures in advance
    let email_address = order.priced_order().customer_info().email_address().clone();
    let order_id = order.priced_order().order_id().clone();

    // Pre-generate IO for logging
    let log_creating = log_action("Creating acknowledgment letter");
    let letter = create_letter(order);
    let acknowledgment = OrderAcknowledgment::new(email_address.clone(), letter);
    let log_sending = log_action("Sending acknowledgment email");
    let send_io = send_acknowledgment(&acknowledgment);
    let log_completed = log_action("Acknowledgment process completed");

    // Chain IO operations with the eff! macro
    lambars::eff! {
        _ <= log_creating;
        _ <= log_sending;
        result <= send_io;
        _ <= log_completed;
        IO::pure(match result {
            SendResult::Sent => Some(OrderAcknowledgmentSent::new(order_id, email_address)),
            SendResult::NotSent => None,
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compound_types::CustomerInfo;
    use crate::simple_types::{BillingAmount, OrderId};
    use crate::workflow::PricingMethod;
    use rstest::rstest;
    use rust_decimal::Decimal;

    // =========================================================================
    // Test helpers
    // =========================================================================

    fn create_test_address(country: &str, state: &str) -> Address {
        Address::create("123 Main St", "", "", "", "City", "12345", state, country).unwrap()
    }

    fn create_test_priced_order(country: &str, state: &str, vip_status: &str) -> PricedOrder {
        let order_id = OrderId::create("OrderId", "order-001").unwrap();
        let customer_info =
            CustomerInfo::create("John", "Doe", "john@example.com", vip_status).unwrap();
        let address = create_test_address(country, state);
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

    // =========================================================================
    // Tests for ShippingRegion
    // =========================================================================

    #[rstest]
    fn test_shipping_region_variants() {
        assert!(ShippingRegion::UsLocalState.is_us_local_state());
        assert!(!ShippingRegion::UsLocalState.is_us_remote_state());
        assert!(!ShippingRegion::UsLocalState.is_international());

        assert!(!ShippingRegion::UsRemoteState.is_us_local_state());
        assert!(ShippingRegion::UsRemoteState.is_us_remote_state());
        assert!(!ShippingRegion::UsRemoteState.is_international());

        assert!(!ShippingRegion::International.is_us_local_state());
        assert!(!ShippingRegion::International.is_us_remote_state());
        assert!(ShippingRegion::International.is_international());
    }

    // =========================================================================
    // Tests for classify_shipping_region
    // =========================================================================

    #[rstest]
    #[case("US", "CA", ShippingRegion::UsLocalState)]
    #[case("US", "OR", ShippingRegion::UsLocalState)]
    #[case("US", "AZ", ShippingRegion::UsLocalState)]
    #[case("US", "NV", ShippingRegion::UsLocalState)]
    #[case("USA", "CA", ShippingRegion::UsLocalState)]
    #[case("US", "NY", ShippingRegion::UsRemoteState)]
    #[case("US", "TX", ShippingRegion::UsRemoteState)]
    #[case("USA", "FL", ShippingRegion::UsRemoteState)]
    // For international shipping tests, uses valid state code (NY) and changes the country
    #[case("Japan", "NY", ShippingRegion::International)]
    #[case("Canada", "NY", ShippingRegion::International)]
    #[case("UK", "NY", ShippingRegion::International)]
    fn test_classify_shipping_region(
        #[case] country: &str,
        #[case] state: &str,
        #[case] expected: ShippingRegion,
    ) {
        let address = create_test_address(country, state);
        let region = classify_shipping_region(&address);
        assert_eq!(region, expected);
    }

    // =========================================================================
    // Tests for calculate_shipping_cost
    // =========================================================================

    #[rstest]
    #[case("US", "CA", 5)]
    #[case("US", "NY", 10)]
    // For international shipping tests, uses valid state code (NY) and changes the country
    #[case("Japan", "NY", 20)]
    fn test_calculate_shipping_cost(
        #[case] country: &str,
        #[case] state: &str,
        #[case] expected_cost: u32,
    ) {
        let priced_order = create_test_priced_order(country, state, "Normal");
        let cost = calculate_shipping_cost(&priced_order);
        assert_eq!(cost.value(), Decimal::from(expected_cost));
    }

    // =========================================================================
    // Tests for add_shipping_info_to_order
    // =========================================================================

    #[rstest]
    fn test_add_shipping_info_to_order() {
        let priced_order = create_test_priced_order("US", "NY", "Normal");
        let order_with_shipping =
            add_shipping_info_to_order(&calculate_shipping_cost, &priced_order);

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
        assert_eq!(
            order_with_shipping.priced_order().order_id().value(),
            "order-001"
        );
    }

    // =========================================================================
    // Tests for free_vip_shipping
    // =========================================================================

    #[rstest]
    fn test_free_vip_shipping_for_vip() {
        let priced_order = create_test_priced_order("US", "NY", "VIP");
        let shipping_info = ShippingInfo::new(
            ShippingMethod::PostalService,
            Price::unsafe_create(Decimal::from(10)),
        );
        let order = PricedOrderWithShippingMethod::new(shipping_info, priced_order);

        let updated = free_vip_shipping(order);

        assert_eq!(
            updated.shipping_info().shipping_cost().value(),
            Decimal::ZERO
        );
        assert!(updated.shipping_info().shipping_method().is_fedex24());
    }

    #[rstest]
    fn test_free_vip_shipping_for_normal() {
        let priced_order = create_test_priced_order("US", "NY", "Normal");
        let shipping_info = ShippingInfo::new(
            ShippingMethod::PostalService,
            Price::unsafe_create(Decimal::from(10)),
        );
        let order = PricedOrderWithShippingMethod::new(shipping_info, priced_order);

        let updated = free_vip_shipping(order);

        assert_eq!(
            updated.shipping_info().shipping_cost().value(),
            Decimal::from(10)
        );
        assert!(
            updated
                .shipping_info()
                .shipping_method()
                .is_postal_service()
        );
    }

    // =========================================================================
    // Tests for acknowledge_order
    // =========================================================================

    #[rstest]
    fn test_acknowledge_order_sent() {
        let priced_order = create_test_priced_order("US", "NY", "Normal");
        let shipping_info = ShippingInfo::new(
            ShippingMethod::Fedex24,
            Price::unsafe_create(Decimal::from(10)),
        );
        let order = PricedOrderWithShippingMethod::new(shipping_info, priced_order);

        let create_letter =
            |_: &PricedOrderWithShippingMethod| HtmlString::new("<p>Test</p>".to_string());
        let send_acknowledgment = |_: &OrderAcknowledgment| IO::pure(SendResult::Sent);

        let io_result = acknowledge_order(&create_letter, &send_acknowledgment, &order);
        let result = io_result.run_unsafe();

        assert!(result.is_some());
        let event = result.unwrap();
        assert_eq!(event.order_id().value(), "order-001");
        assert_eq!(event.email_address().value(), "john@example.com");
    }

    #[rstest]
    fn test_acknowledge_order_not_sent() {
        let priced_order = create_test_priced_order("US", "NY", "Normal");
        let shipping_info = ShippingInfo::new(
            ShippingMethod::Fedex24,
            Price::unsafe_create(Decimal::from(10)),
        );
        let order = PricedOrderWithShippingMethod::new(shipping_info, priced_order);

        let create_letter =
            |_: &PricedOrderWithShippingMethod| HtmlString::new("<p>Test</p>".to_string());
        let send_acknowledgment = |_: &OrderAcknowledgment| IO::pure(SendResult::NotSent);

        let io_result = acknowledge_order(&create_letter, &send_acknowledgment, &order);
        let result = io_result.run_unsafe();

        assert!(result.is_none());
    }
}
