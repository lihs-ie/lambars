//! Event generation
//!
//! Phase 7 implementation. Generates [`PlaceOrderEvent`] from [`PricedOrder`].
//!
//! # Design Principles
//!
//! - Pure functions: all event generation functions are referentially transparent
//! - Immutability: never modifies input data, always generates new events
//! - Composability: builds larger functions from smaller ones
//! - Pattern matching: `ProductLine`/`CommentLine` branches of [`PricedOrderLine`]
//!
//! # Feature List
//!
//! - [`make_shipment_line`] - Converts [`PricedOrderLine`] to [`ShippableOrderLine`]
//! - [`create_shipping_event`] - Generates shipping events
//! - [`create_billing_event`] - Generates billing events (conditional)
//! - [`create_events`] - Integrates all events
//!
//! # Usage Examples
//!
//! ```
//! use order_taking_sample::workflow::{
//!     create_events, PricedOrder, OrderAcknowledgmentSent, PricingMethod,
//! };
//! use order_taking_sample::simple_types::{OrderId, BillingAmount, EmailAddress};
//! use order_taking_sample::compound_types::{CustomerInfo, Address};
//! use rust_decimal::Decimal;
//!
//! let order_id = OrderId::create("OrderId", "order-001").unwrap();
//! let customer_info = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
//! let address = Address::create(
//!     "123 Main St", "", "", "", "New York", "10001", "NY", "US"
//! ).unwrap();
//! let amount_to_bill = BillingAmount::create(Decimal::from(100)).unwrap();
//!
//! let priced_order = PricedOrder::new(
//!     order_id.clone(), customer_info, address.clone(), address, amount_to_bill, vec![], PricingMethod::Standard,
//! );
//!
//! let email = EmailAddress::create("EmailAddress", "john@example.com").unwrap();
//! let acknowledgment_event = OrderAcknowledgmentSent::new(order_id.clone(), email);
//! let events = create_events(&priced_order, Some(acknowledgment_event));
//! // AcknowledgmentSent + ShippableOrderPlaced + BillableOrderPlaced = 3
//! assert_eq!(events.len(), 3);
//! ```

use lambars::{compose, pipe};
use rust_decimal::Decimal;

use crate::simple_types::PdfAttachment;
use crate::workflow::output_types::{
    BillableOrderPlaced, OrderAcknowledgmentSent, PlaceOrderEvent, ShippableOrderLine,
    ShippableOrderPlaced,
};
use crate::workflow::priced_types::{PricedOrder, PricedOrderLine};

// =============================================================================
// make_shipment_line (REQ-070)
// =============================================================================

/// Converts a [`PricedOrderLine`] to a [`ShippableOrderLine`]
///
/// For `ProductLine`, extracts the product code and quantity to create a [`ShippableOrderLine`].
/// For `CommentLine`, returns `None` (not shippable).
///
/// # Arguments
///
/// * `line` - Priced order line
///
/// # Returns
///
/// * `Some(ShippableOrderLine)` - `ProductLine` when
/// * `None` - `CommentLine` when
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{
///     make_shipment_line, PricedOrderLine, PricedOrderProductLine,
/// };
/// use order_taking_sample::simple_types::{OrderLineId, ProductCode, OrderQuantity, Price};
/// use rust_decimal::Decimal;
///
/// // ProductLine when
/// let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
/// let product_line = PricedOrderProductLine::new(
///     OrderLineId::create("OrderLineId", "line-001").unwrap(),
///     product_code.clone(),
///     OrderQuantity::create("Quantity", &product_code, Decimal::from(5)).unwrap(),
///     Price::create(Decimal::from(500)).unwrap(),
/// );
/// let line = PricedOrderLine::ProductLine(product_line);
/// let result = make_shipment_line(&line);
/// assert!(result.is_some());
///
/// // CommentLine when
/// let comment_line = PricedOrderLine::CommentLine("Gift message".to_string());
/// let result = make_shipment_line(&comment_line);
/// assert!(result.is_none());
/// ```
#[must_use]
pub fn make_shipment_line(line: &PricedOrderLine) -> Option<ShippableOrderLine> {
    match line {
        PricedOrderLine::ProductLine(product_line) => Some(ShippableOrderLine::new(
            product_line.product_code().clone(),
            *product_line.quantity(),
        )),
        PricedOrderLine::CommentLine(_) => None,
    }
}

// =============================================================================
// create_shipping_event (REQ-071)
// =============================================================================

/// Generates a [`ShippableOrderPlaced`] event from a [`PricedOrder`]
///
/// Extracts shippable lines (`ProductLine` only),
/// and generates an event including Order ID, shipping address, line list, and PDF.
///
/// # Arguments
///
/// * `priced_order` - Priced order
///
/// # Returns
///
/// Shippable order placed event
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{
///     create_shipping_event, PricedOrder, PricingMethod,
/// };
/// use order_taking_sample::simple_types::{OrderId, BillingAmount};
/// use order_taking_sample::compound_types::{CustomerInfo, Address};
/// use rust_decimal::Decimal;
///
/// let order_id = OrderId::create("OrderId", "order-001").unwrap();
/// let customer_info = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
/// let address = Address::create(
///     "123 Main St", "", "", "", "New York", "10001", "NY", "US"
/// ).unwrap();
/// let amount_to_bill = BillingAmount::create(Decimal::from(100)).unwrap();
///
/// let priced_order = PricedOrder::new(
///     order_id, customer_info, address.clone(), address, amount_to_bill, vec![], PricingMethod::Standard,
/// );
///
/// let event = create_shipping_event(&priced_order);
/// assert_eq!(event.pdf().name(), "Orderorder-001.pdf");
/// ```
#[must_use]
pub fn create_shipping_event(priced_order: &PricedOrder) -> ShippableOrderPlaced {
    // Currently passing make_shipment_line directly (no compose! needed for a single function)
    // If additional conversion is needed:
    //   let transform = compose!(additional_transform, make_shipment_line);
    //   priced_order.lines().iter().filter_map(transform).collect()
    let shipment_lines: Vec<ShippableOrderLine> = priced_order
        .lines()
        .iter()
        .filter_map(make_shipment_line)
        .collect();

    let pdf_name = format!("Order{}.pdf", priced_order.order_id().value());
    let pdf = PdfAttachment::new(pdf_name, vec![]);

    ShippableOrderPlaced::new(
        priced_order.order_id().clone(),
        priced_order.shipping_address().clone(),
        shipment_lines,
        pdf,
    )
}

// =============================================================================
// create_billing_event (REQ-072)
// =============================================================================

/// Conditionally generates a [`BillableOrderPlaced`] event from a [`PricedOrder`]
///
/// Generates an event only when the billing amount is positive; returns `None` when 0 or less.
///
/// # Arguments
///
/// * `priced_order` - Priced order
///
/// # Returns
///
/// * `Some(BillableOrderPlaced)` - When the billing amount is positive
/// * `None` - When the billing amount is 0 or less
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{
///     create_billing_event, PricedOrder, PricingMethod,
/// };
/// use order_taking_sample::simple_types::{OrderId, BillingAmount};
/// use order_taking_sample::compound_types::{CustomerInfo, Address};
/// use rust_decimal::Decimal;
///
/// // When the billing amount is positive
/// let order_id = OrderId::create("OrderId", "order-001").unwrap();
/// let customer_info = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
/// let address = Address::create(
///     "123 Main St", "", "", "", "New York", "10001", "NY", "US"
/// ).unwrap();
/// let amount_to_bill = BillingAmount::create(Decimal::from(100)).unwrap();
///
/// let priced_order = PricedOrder::new(
///     order_id, customer_info, address.clone(), address, amount_to_bill, vec![], PricingMethod::Standard,
/// );
///
/// let event = create_billing_event(&priced_order);
/// assert!(event.is_some());
/// ```
#[must_use]
pub fn create_billing_event(priced_order: &PricedOrder) -> Option<BillableOrderPlaced> {
    let billing_amount = priced_order.amount_to_bill().value();
    if billing_amount > Decimal::ZERO {
        Some(BillableOrderPlaced::new(
            priced_order.order_id().clone(),
            priced_order.billing_address().clone(),
            *priced_order.amount_to_bill(),
        ))
    } else {
        None
    }
}

// =============================================================================
// create_events (REQ-073)
// =============================================================================

/// Integrates all events from a [`PricedOrder`] and an acknowledgment sent event
///
/// Event order:
/// 1. `AcknowledgmentSent` (if present)
/// 2. `ShippableOrderPlaced` (always generated)
/// 3. `BillableOrderPlaced` (when billing amount is positive)
///
/// # Arguments
///
/// * `priced_order` - Priced order
/// * `acknowledgment_event` - Acknowledgment sent event (Option)
///
/// # Returns
///
/// all [`PlaceOrderEvent`] includes Vec
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{
///     create_events, PricedOrder, PricingMethod, OrderAcknowledgmentSent,
/// };
/// use order_taking_sample::simple_types::{OrderId, BillingAmount, EmailAddress};
/// use order_taking_sample::compound_types::{CustomerInfo, Address};
/// use rust_decimal::Decimal;
///
/// let order_id = OrderId::create("OrderId", "order-001").unwrap();
/// let customer_info = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
/// let address = Address::create(
///     "123 Main St", "", "", "", "New York", "10001", "NY", "US"
/// ).unwrap();
/// let amount_to_bill = BillingAmount::create(Decimal::from(100)).unwrap();
///
/// let priced_order = PricedOrder::new(
///     order_id.clone(), customer_info, address.clone(), address, amount_to_bill, vec![], PricingMethod::Standard,
/// );
///
/// // With acknowledgment email and billing: 3 events
/// let email = EmailAddress::create("EmailAddress", "john@example.com").unwrap();
/// let acknowledgment_event = OrderAcknowledgmentSent::new(order_id.clone(), email);
/// let events = create_events(&priced_order, Some(acknowledgment_event));
/// assert_eq!(events.len(), 3);
/// ```
#[must_use]
pub fn create_events(
    priced_order: &PricedOrder,
    acknowledgment_event: Option<OrderAcknowledgmentSent>,
) -> Vec<PlaceOrderEvent> {
    // Composition function for creating shipping events
    // compose! composes right to left: ShippableOrderPlaced(create_shipping_event(order))
    let to_shipping_event = compose!(PlaceOrderEvent::ShippableOrderPlaced, create_shipping_event);

    // Acknowledgment sent event (if present)
    let acknowledgment_events: Vec<PlaceOrderEvent> = acknowledgment_event
        .map(PlaceOrderEvent::AcknowledgmentSent)
        .into_iter()
        .collect();

    // Shipping event (always generated): apply the composition function
    let shipping_events = vec![to_shipping_event(priced_order)];

    // Billing event (only when billing amount is positive)
    let billing_events: Vec<PlaceOrderEvent> = pipe!(priced_order, create_billing_event)
        .map(PlaceOrderEvent::BillableOrderPlaced)
        .into_iter()
        .collect();

    // Combine events
    [acknowledgment_events, shipping_events, billing_events].concat()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compound_types::{Address, CustomerInfo};
    use crate::simple_types::{
        BillingAmount, EmailAddress, OrderId, OrderLineId, OrderQuantity, Price, ProductCode,
    };
    use crate::workflow::PricingMethod;
    use rstest::rstest;

    // =========================================================================
    // Test helpers
    // =========================================================================

    fn create_test_priced_order(
        order_id: &str,
        amount_to_bill: Decimal,
        lines: Vec<PricedOrderLine>,
    ) -> PricedOrder {
        let order_id = OrderId::create("OrderId", order_id).unwrap();
        let customer_info =
            CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
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

    fn create_test_product_line(price: Decimal) -> PricedOrderLine {
        let order_line_id = OrderLineId::create("OrderLineId", "line-001").unwrap();
        let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let quantity = OrderQuantity::create("Quantity", &product_code, Decimal::from(5)).unwrap();
        let line_price = Price::create(price).unwrap();

        PricedOrderLine::ProductLine(crate::workflow::PricedOrderProductLine::new(
            order_line_id,
            product_code,
            quantity,
            line_price,
        ))
    }

    // =========================================================================
    // Tests for make_shipment_line
    // =========================================================================

    #[rstest]
    fn test_make_shipment_line_product_line() {
        let product_line = create_test_product_line(Decimal::from(500));
        let result = make_shipment_line(&product_line);

        assert!(result.is_some());
        let shipment_line = result.unwrap();
        assert_eq!(shipment_line.product_code().value(), "W1234");
    }

    #[rstest]
    fn test_make_shipment_line_comment_line() {
        let comment_line = PricedOrderLine::CommentLine("Test comment".to_string());
        let result = make_shipment_line(&comment_line);

        assert!(result.is_none());
    }

    // =========================================================================
    // Tests for create_shipping_event
    // =========================================================================

    #[rstest]
    fn test_create_shipping_event_with_lines() {
        let lines = vec![create_test_product_line(Decimal::from(500))];
        let priced_order = create_test_priced_order("order-001", Decimal::from(500), lines);

        let event = create_shipping_event(&priced_order);

        assert_eq!(event.order_id().value(), "order-001");
        assert_eq!(event.shipment_lines().len(), 1);
        assert_eq!(event.pdf().name(), "Orderorder-001.pdf");
    }

    #[rstest]
    fn test_create_shipping_event_empty_lines() {
        let priced_order = create_test_priced_order("order-002", Decimal::from(100), vec![]);

        let event = create_shipping_event(&priced_order);

        assert_eq!(event.shipment_lines().len(), 0);
    }

    // =========================================================================
    // Tests for create_billing_event
    // =========================================================================

    #[rstest]
    fn test_create_billing_event_positive() {
        let priced_order = create_test_priced_order("order-001", Decimal::from(1000), vec![]);

        let result = create_billing_event(&priced_order);

        assert!(result.is_some());
        let event = result.unwrap();
        assert_eq!(event.amount_to_bill().value(), Decimal::from(1000));
    }

    #[rstest]
    fn test_create_billing_event_zero() {
        let priced_order = create_test_priced_order("order-002", Decimal::ZERO, vec![]);

        let result = create_billing_event(&priced_order);

        assert!(result.is_none());
    }

    // =========================================================================
    // Tests for create_events
    // =========================================================================

    #[rstest]
    fn test_create_events_all() {
        let priced_order = create_test_priced_order("order-001", Decimal::from(1000), vec![]);
        let order_id = OrderId::create("OrderId", "order-001").unwrap();
        let email = EmailAddress::create("EmailAddress", "test@example.com").unwrap();
        let acknowledgment = OrderAcknowledgmentSent::new(order_id, email);

        let events = create_events(&priced_order, Some(acknowledgment));

        assert_eq!(events.len(), 3);
        assert!(events[0].is_acknowledgment());
        assert!(events[1].is_shippable());
        assert!(events[2].is_billable());
    }

    #[rstest]
    fn test_create_events_no_acknowledgment() {
        let priced_order = create_test_priced_order("order-002", Decimal::from(500), vec![]);

        let events = create_events(&priced_order, None);

        assert_eq!(events.len(), 2);
        assert!(events[0].is_shippable());
        assert!(events[1].is_billable());
    }

    #[rstest]
    fn test_create_events_no_billing() {
        let priced_order = create_test_priced_order("order-003", Decimal::ZERO, vec![]);
        let order_id = OrderId::create("OrderId", "order-003").unwrap();
        let email = EmailAddress::create("EmailAddress", "test@example.com").unwrap();
        let acknowledgment = OrderAcknowledgmentSent::new(order_id, email);

        let events = create_events(&priced_order, Some(acknowledgment));

        assert_eq!(events.len(), 2);
        assert!(events[0].is_acknowledgment());
        assert!(events[1].is_shippable());
    }

    #[rstest]
    fn test_create_events_minimal() {
        let priced_order = create_test_priced_order("order-004", Decimal::ZERO, vec![]);

        let events = create_events(&priced_order, None);

        assert_eq!(events.len(), 1);
        assert!(events[0].is_shippable());
    }
}
