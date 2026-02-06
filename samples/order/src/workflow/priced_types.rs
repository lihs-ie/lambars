//! Priced types
//!
//! Defines types representing priced data.
//! Expresses the state where pricing information has been added to validated data.
//!
//! # Type List
//!
//! - [`PricedOrderProductLine`] - Priced product order line
//! - [`PricedOrderLine`] - Priced order line (product or comment)
//! - [`PricedOrder`] - Priced order

use crate::compound_types::{Address, CustomerInfo};
use crate::simple_types::{BillingAmount, OrderId, OrderLineId, OrderQuantity, Price, ProductCode};
use crate::workflow::validated_types::PricingMethod;
use lambars_derive::Lenses;

// =============================================================================
// PricedOrderProductLine
// =============================================================================

/// Priced product order line
///
/// A type that adds price information to [`ValidatedOrderLine`](super::ValidatedOrderLine).
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::PricedOrderProductLine;
/// use order_taking_sample::simple_types::{OrderLineId, ProductCode, OrderQuantity, Price};
/// use rust_decimal::Decimal;
///
/// let order_line_id = OrderLineId::create("OrderLineId", "line-001").unwrap();
/// let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
/// let quantity = OrderQuantity::create("Quantity", &product_code, Decimal::from(5)).unwrap();
/// let line_price = Price::create(Decimal::from(500)).unwrap();
///
/// let priced_line = PricedOrderProductLine::new(order_line_id, product_code, quantity, line_price);
/// assert_eq!(priced_line.line_price().value(), Decimal::from(500));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PricedOrderProductLine {
    order_line_id: OrderLineId,
    product_code: ProductCode,
    quantity: OrderQuantity,
    line_price: Price,
}

impl PricedOrderProductLine {
    /// Creates a new `PricedOrderProductLine`
    ///
    /// # Arguments
    ///
    /// * `order_line_id` - Order line ID
    /// * `product_code` - Product code
    /// * `quantity` - quantity
    /// * `line_price` - Total line price (unit price x quantity)
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::PricedOrderProductLine;
    /// use order_taking_sample::simple_types::{OrderLineId, ProductCode, OrderQuantity, Price};
    /// use rust_decimal::Decimal;
    ///
    /// let order_line_id = OrderLineId::create("OrderLineId", "line-002").unwrap();
    /// let product_code = ProductCode::create("ProductCode", "G123").unwrap();
    /// let quantity = OrderQuantity::create("Quantity", &product_code, Decimal::new(25, 1)).unwrap();
    /// let line_price = Price::create(Decimal::from(250)).unwrap();
    ///
    /// let priced_line = PricedOrderProductLine::new(order_line_id, product_code, quantity, line_price);
    /// ```
    #[must_use]
    pub const fn new(
        order_line_id: OrderLineId,
        product_code: ProductCode,
        quantity: OrderQuantity,
        line_price: Price,
    ) -> Self {
        Self {
            order_line_id,
            product_code,
            quantity,
            line_price,
        }
    }

    /// Returns a reference to Order line ID
    #[must_use]
    pub const fn order_line_id(&self) -> &OrderLineId {
        &self.order_line_id
    }

    /// Returns a reference to Product code
    #[must_use]
    pub const fn product_code(&self) -> &ProductCode {
        &self.product_code
    }

    /// Returns a reference to quantity
    #[must_use]
    pub const fn quantity(&self) -> &OrderQuantity {
        &self.quantity
    }

    /// Returns a reference to lineprice
    #[must_use]
    pub const fn line_price(&self) -> &Price {
        &self.line_price
    }
}

// =============================================================================
// PricedOrderLine
// =============================================================================

/// Priced order line
///
/// A sum type representing either a product line or a comment line.
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{PricedOrderLine, PricedOrderProductLine};
/// use order_taking_sample::simple_types::{OrderLineId, ProductCode, OrderQuantity, Price};
/// use rust_decimal::Decimal;
///
/// // Product line
/// let order_line_id = OrderLineId::create("OrderLineId", "line-001").unwrap();
/// let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
/// let quantity = OrderQuantity::create("Quantity", &product_code, Decimal::from(5)).unwrap();
/// let line_price = Price::create(Decimal::from(500)).unwrap();
/// let product_line = PricedOrderProductLine::new(order_line_id, product_code, quantity, line_price);
///
/// let priced_line = PricedOrderLine::ProductLine(product_line);
/// assert!(priced_line.is_product_line());
/// assert!(priced_line.line_price().is_some());
///
/// // Comment line
/// let comment_line = PricedOrderLine::CommentLine("Gift message: Happy Birthday!".to_string());
/// assert!(comment_line.is_comment_line());
/// assert!(comment_line.line_price().is_none());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PricedOrderLine {
    /// Product line (with price)
    ProductLine(PricedOrderProductLine),

    /// Comment line (gift message, etc.)
    CommentLine(String),
}

impl PricedOrderLine {
    /// Returns whether this is the `ProductLine` variant
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{PricedOrderLine, PricedOrderProductLine};
    /// use order_taking_sample::simple_types::{OrderLineId, ProductCode, OrderQuantity, Price};
    /// use rust_decimal::Decimal;
    ///
    /// let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
    /// let product_line = PricedOrderProductLine::new(
    ///     OrderLineId::create("OrderLineId", "line-001").unwrap(),
    ///     product_code.clone(),
    ///     OrderQuantity::create("Quantity", &product_code, Decimal::from(5)).unwrap(),
    ///     Price::create(Decimal::from(500)).unwrap(),
    /// );
    /// let priced_line = PricedOrderLine::ProductLine(product_line);
    /// assert!(priced_line.is_product_line());
    /// ```
    #[must_use]
    pub const fn is_product_line(&self) -> bool {
        matches!(self, Self::ProductLine(_))
    }

    /// Returns whether this is the `CommentLine` variant
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::PricedOrderLine;
    ///
    /// let comment_line = PricedOrderLine::CommentLine("Thank you!".to_string());
    /// assert!(comment_line.is_comment_line());
    /// ```
    #[must_use]
    pub const fn is_comment_line(&self) -> bool {
        matches!(self, Self::CommentLine(_))
    }

    /// Returns the line price
    ///
    /// For `ProductLine`, returns `Some(&Price)`;
    /// for `CommentLine`, returns `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{PricedOrderLine, PricedOrderProductLine};
    /// use order_taking_sample::simple_types::{OrderLineId, ProductCode, OrderQuantity, Price};
    /// use rust_decimal::Decimal;
    ///
    /// // ProductLine has a price
    /// let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
    /// let product_line = PricedOrderProductLine::new(
    ///     OrderLineId::create("OrderLineId", "line-001").unwrap(),
    ///     product_code.clone(),
    ///     OrderQuantity::create("Quantity", &product_code, Decimal::from(5)).unwrap(),
    ///     Price::create(Decimal::from(500)).unwrap(),
    /// );
    /// let priced_line = PricedOrderLine::ProductLine(product_line);
    /// assert!(priced_line.line_price().is_some());
    ///
    /// // CommentLine does not have a price
    /// let comment_line = PricedOrderLine::CommentLine("Note".to_string());
    /// assert!(comment_line.line_price().is_none());
    /// ```
    #[must_use]
    pub const fn line_price(&self) -> Option<&Price> {
        match self {
            Self::ProductLine(line) => Some(line.line_price()),
            Self::CommentLine(_) => None,
        }
    }
}

// =============================================================================
// PricedOrder
// =============================================================================

/// Priced order
///
/// A type that adds price information to [`ValidatedOrder`](super::ValidatedOrder).
/// Has a billing amount and a list of priced order lines.
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{PricedOrder, PricedOrderLine, PricedOrderProductLine, PricingMethod};
/// use order_taking_sample::simple_types::{OrderId, OrderLineId, ProductCode, OrderQuantity, Price, BillingAmount};
/// use order_taking_sample::compound_types::{CustomerInfo, Address};
/// use rust_decimal::Decimal;
///
/// let order_id = OrderId::create("OrderId", "order-001").unwrap();
/// let customer_info = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
/// let address = Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
/// let amount_to_bill = BillingAmount::create(Decimal::from(1000)).unwrap();
///
/// let priced_order = PricedOrder::new(
///     order_id,
///     customer_info,
///     address.clone(),
///     address,
///     amount_to_bill,
///     vec![],
///     PricingMethod::Standard,
/// );
/// assert_eq!(priced_order.amount_to_bill().value(), Decimal::from(1000));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Lenses)]
pub struct PricedOrder {
    order_id: OrderId,
    customer_info: CustomerInfo,
    shipping_address: Address,
    billing_address: Address,
    amount_to_bill: BillingAmount,
    lines: Vec<PricedOrderLine>,
    pricing_method: PricingMethod,
}

impl PricedOrder {
    /// Creates a new `PricedOrder`
    ///
    /// # Arguments
    ///
    /// * `order_id` - Order ID
    /// * `customer_info` - customer information
    /// * `shipping_address` - Shipping address
    /// * `billing_address` - Billing address
    /// * `amount_to_bill` - Total billing amount
    /// * `lines` - Priced order linelist
    /// * `pricing_method` - Pricing method used
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        order_id: OrderId,
        customer_info: CustomerInfo,
        shipping_address: Address,
        billing_address: Address,
        amount_to_bill: BillingAmount,
        lines: Vec<PricedOrderLine>,
        pricing_method: PricingMethod,
    ) -> Self {
        Self {
            order_id,
            customer_info,
            shipping_address,
            billing_address,
            amount_to_bill,
            lines,
            pricing_method,
        }
    }

    /// Returns a reference to Order ID
    #[must_use]
    pub const fn order_id(&self) -> &OrderId {
        &self.order_id
    }

    /// Returns a reference to customer information
    #[must_use]
    pub const fn customer_info(&self) -> &CustomerInfo {
        &self.customer_info
    }

    /// Returns a reference to Shipping address
    #[must_use]
    pub const fn shipping_address(&self) -> &Address {
        &self.shipping_address
    }

    /// Returns a reference to Billing address
    #[must_use]
    pub const fn billing_address(&self) -> &Address {
        &self.billing_address
    }

    /// Returns a reference to Billing amount
    #[must_use]
    pub const fn amount_to_bill(&self) -> &BillingAmount {
        &self.amount_to_bill
    }

    /// Returns a reference to Priced order lines
    #[must_use]
    pub fn lines(&self) -> &[PricedOrderLine] {
        &self.lines
    }

    /// Returns a reference to Pricing method
    #[must_use]
    pub const fn pricing_method(&self) -> &PricingMethod {
        &self.pricing_method
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    mod priced_order_product_line_tests {
        use super::*;

        fn create_product_line() -> PricedOrderProductLine {
            let order_line_id = OrderLineId::create("OrderLineId", "line-001").unwrap();
            let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
            let quantity =
                OrderQuantity::create("Quantity", &product_code, Decimal::from(5)).unwrap();
            let line_price = Price::create(Decimal::from(500)).unwrap();
            PricedOrderProductLine::new(order_line_id, product_code, quantity, line_price)
        }

        #[test]
        fn test_new_and_getters() {
            let order_line_id = OrderLineId::create("OrderLineId", "line-001").unwrap();
            let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
            let quantity =
                OrderQuantity::create("Quantity", &product_code, Decimal::from(5)).unwrap();
            let line_price = Price::create(Decimal::from(500)).unwrap();

            let priced_line = PricedOrderProductLine::new(
                order_line_id.clone(),
                product_code.clone(),
                quantity.clone(),
                line_price.clone(),
            );

            assert_eq!(priced_line.order_line_id(), &order_line_id);
            assert_eq!(priced_line.product_code(), &product_code);
            assert_eq!(priced_line.quantity(), &quantity);
            assert_eq!(priced_line.line_price(), &line_price);
        }

        #[test]
        fn test_clone() {
            let priced_line1 = create_product_line();
            let priced_line2 = priced_line1.clone();
            assert_eq!(priced_line1, priced_line2);
        }
    }

    mod priced_order_line_tests {
        use super::*;

        fn create_product_line() -> PricedOrderProductLine {
            let order_line_id = OrderLineId::create("OrderLineId", "line-001").unwrap();
            let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
            let quantity =
                OrderQuantity::create("Quantity", &product_code, Decimal::from(5)).unwrap();
            let line_price = Price::create(Decimal::from(500)).unwrap();
            PricedOrderProductLine::new(order_line_id, product_code, quantity, line_price)
        }

        #[test]
        fn test_product_line_variant() {
            let product_line = create_product_line();
            let priced_line = PricedOrderLine::ProductLine(product_line.clone());

            assert!(priced_line.is_product_line());
            assert!(!priced_line.is_comment_line());
            assert_eq!(priced_line.line_price(), Some(product_line.line_price()));
        }

        #[test]
        fn test_comment_line_variant() {
            let comment_line = PricedOrderLine::CommentLine("Gift message".to_string());

            assert!(!comment_line.is_product_line());
            assert!(comment_line.is_comment_line());
            assert!(comment_line.line_price().is_none());
        }

        #[test]
        fn test_clone() {
            let priced_line1 = PricedOrderLine::ProductLine(create_product_line());
            let priced_line2 = priced_line1.clone();
            assert_eq!(priced_line1, priced_line2);
        }
    }

    mod priced_order_tests {
        use super::*;

        fn create_order_id() -> OrderId {
            OrderId::create("OrderId", "order-001").unwrap()
        }

        fn create_customer_info() -> CustomerInfo {
            CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap()
        }

        fn create_address() -> Address {
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap()
        }

        fn create_priced_order_line() -> PricedOrderLine {
            let order_line_id = OrderLineId::create("OrderLineId", "line-001").unwrap();
            let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
            let quantity =
                OrderQuantity::create("Quantity", &product_code, Decimal::from(5)).unwrap();
            let line_price = Price::create(Decimal::from(500)).unwrap();
            let product_line =
                PricedOrderProductLine::new(order_line_id, product_code, quantity, line_price);
            PricedOrderLine::ProductLine(product_line)
        }

        #[test]
        fn test_new_and_getters() {
            let order_id = create_order_id();
            let customer_info = create_customer_info();
            let shipping_address = create_address();
            let billing_address = create_address();
            let amount_to_bill = BillingAmount::create(Decimal::from(1000)).unwrap();
            let lines = vec![create_priced_order_line()];
            let pricing_method = PricingMethod::Standard;

            let priced_order = PricedOrder::new(
                order_id.clone(),
                customer_info.clone(),
                shipping_address.clone(),
                billing_address.clone(),
                amount_to_bill.clone(),
                lines.clone(),
                pricing_method.clone(),
            );

            assert_eq!(priced_order.order_id(), &order_id);
            assert_eq!(priced_order.customer_info(), &customer_info);
            assert_eq!(priced_order.shipping_address(), &shipping_address);
            assert_eq!(priced_order.billing_address(), &billing_address);
            assert_eq!(priced_order.amount_to_bill(), &amount_to_bill);
            assert_eq!(priced_order.lines().len(), 1);
            assert_eq!(priced_order.pricing_method(), &pricing_method);
        }

        #[test]
        fn test_with_multiple_lines() {
            let lines = vec![
                create_priced_order_line(),
                PricedOrderLine::CommentLine("Gift message".to_string()),
            ];

            let priced_order = PricedOrder::new(
                create_order_id(),
                create_customer_info(),
                create_address(),
                create_address(),
                BillingAmount::create(Decimal::from(500)).unwrap(),
                lines,
                PricingMethod::Standard,
            );

            assert_eq!(priced_order.lines().len(), 2);
        }

        #[test]
        fn test_clone() {
            let priced_order1 = PricedOrder::new(
                create_order_id(),
                create_customer_info(),
                create_address(),
                create_address(),
                BillingAmount::create(Decimal::from(1000)).unwrap(),
                vec![create_priced_order_line()],
                PricingMethod::Standard,
            );
            let priced_order2 = priced_order1.clone();
            assert_eq!(priced_order1, priced_order2);
        }
    }
}
