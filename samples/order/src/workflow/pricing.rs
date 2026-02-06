//! Pricing module
//!
//! Provides pricing logic to convert `ValidatedOrder` to `PricedOrder`.
//!
//! # Function List
//!
//! - [`get_line_price`] - Gets the price from an order line
//! - [`to_priced_order_line`] - Attaches price to an order line
//! - [`add_comment_line`] - Adds a comment line when a promotion is applied
//! - [`get_pricing_function`] - Factory for pricing functions
//! - [`price_order`] - Main pricing function

use crate::simple_types::{BillingAmount, Price, ProductCode, PromotionCode};
use crate::workflow::{
    PlaceOrderError, PricedOrder, PricedOrderLine, PricedOrderProductLine, PricingError,
    PricingMethod, ValidatedOrder, ValidatedOrderLine,
};
use lambars::control::Lazy;
use rust_decimal::Decimal;
use std::rc::Rc;

// =============================================================================
// get_line_price (REQ-062)
// =============================================================================

/// Helper function to retrieve a price from a `PricedOrderLine`
///
/// For `ProductLine`, returns the line price;
/// for `CommentLine`, returns a price of 0.
///
/// # Arguments
///
/// * `line` - Priced order line
///
/// # Returns
///
/// Line price
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::pricing::get_line_price;
/// use order_taking_sample::workflow::{PricedOrderLine, PricedOrderProductLine};
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
/// let price = get_line_price(&line);
/// assert_eq!(price.value(), Decimal::from(500));
///
/// // CommentLine when
/// let comment_line = PricedOrderLine::CommentLine("Applied promotion".to_string());
/// let price = get_line_price(&comment_line);
/// assert_eq!(price.value(), Decimal::ZERO);
/// ```
#[must_use]
pub fn get_line_price(line: &PricedOrderLine) -> Price {
    match line {
        PricedOrderLine::ProductLine(product_line) => *product_line.line_price(),
        PricedOrderLine::CommentLine(_) => Price::unsafe_create(Decimal::ZERO),
    }
}

// =============================================================================
// to_priced_order_line (REQ-060)
// =============================================================================

/// Assigns a price to a `ValidatedOrderLine` to create a `PricedOrderLine`
///
/// Calculates the total line price as unit price times quantity.
/// Returns `PricingError` if the price is out of range.
///
/// # Arguments
///
/// * `get_product_price` - Function to retrieve a price from a product code
/// * `validated_order_line` - Validated order line
///
/// # Returns
///
/// * `Ok(PricedOrderLine)` - Priced order line
/// * `Err(PricingError)` - Pricing error (overflow, etc.)
///
/// # Errors
///
/// Returns `PricingError` when the unit price times quantity calculation result exceeds the `Price` range.
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::pricing::to_priced_order_line;
/// use order_taking_sample::workflow::{ValidatedOrderLine, PricedOrderLine};
/// use order_taking_sample::simple_types::{OrderLineId, ProductCode, OrderQuantity, Price};
/// use rust_decimal::Decimal;
///
/// let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
/// let validated_line = ValidatedOrderLine::new(
///     OrderLineId::create("OrderLineId", "line-001").unwrap(),
///     product_code.clone(),
///     OrderQuantity::create("Quantity", &product_code, Decimal::from(5)).unwrap(),
/// );
///
/// let get_price = |_: &ProductCode| Price::create(Decimal::from(100)).unwrap();
/// let result = to_priced_order_line(&get_price, &validated_line);
///
/// assert!(result.is_ok());
/// let priced_line = result.unwrap();
/// assert!(priced_line.is_product_line());
/// ```
pub fn to_priced_order_line<GetProductPriceFn>(
    get_product_price: &GetProductPriceFn,
    validated_order_line: &ValidatedOrderLine,
) -> Result<PricedOrderLine, PricingError>
where
    GetProductPriceFn: Fn(&ProductCode) -> Price + ?Sized,
{
    let quantity = validated_order_line.quantity().value();
    let unit_price = get_product_price(validated_order_line.product_code());

    let line_price = unit_price.multiply(quantity).map_err(|validation_error| {
        PricingError::new(&format!(
            "Price multiplication overflow: {}",
            validation_error.message
        ))
    })?;

    let priced_product_line = PricedOrderProductLine::new(
        validated_order_line.order_line_id().clone(),
        validated_order_line.product_code().clone(),
        *validated_order_line.quantity(),
        line_price,
    );

    Ok(PricedOrderLine::ProductLine(priced_product_line))
}

// =============================================================================
// add_comment_line (REQ-061)
// =============================================================================

/// Adds a comment line when a promotion is applied
///
/// When `PricingMethod` is `Promotion`, appends a comment line to the end of the priced order line list
/// indicating that a promotion was applied.
/// For `Standard`, returns the list unchanged.
///
/// # Arguments
///
/// * `pricing_method` - Pricing method
/// * `lines` - Priced order linelist
///
/// # Returns
///
/// The line list with a comment line added (or unchanged)
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::pricing::add_comment_line;
/// use order_taking_sample::workflow::{PricedOrderLine, PricingMethod};
/// use order_taking_sample::simple_types::PromotionCode;
///
/// // No change for Standard
/// let lines: Vec<PricedOrderLine> = vec![];
/// let result = add_comment_line(&PricingMethod::Standard, lines);
/// assert!(result.is_empty());
///
/// // A comment line is added for Promotion
/// let lines: Vec<PricedOrderLine> = vec![];
/// let promo_code = PromotionCode::new("SUMMER2024".to_string());
/// let result = add_comment_line(&PricingMethod::Promotion(promo_code), lines);
/// assert_eq!(result.len(), 1);
/// assert!(result[0].is_comment_line());
/// ```
#[must_use]
pub fn add_comment_line(
    pricing_method: &PricingMethod,
    mut lines: Vec<PricedOrderLine>,
) -> Vec<PricedOrderLine> {
    match pricing_method {
        PricingMethod::Standard => lines,
        PricingMethod::Promotion(promotion_code) => {
            let comment = format!("Applied promotion {}", promotion_code.value());
            lines.push(PricedOrderLine::CommentLine(comment));
            lines
        }
    }
}

// =============================================================================
// get_pricing_function (REQ-059)
// =============================================================================

/// Factory for price retrieval functions
///
/// Takes a standard pricing retrieval function and a promotion pricing retrieval function,
/// and returns a factory function that returns a price retrieval function based on `PricingMethod`.
///
/// - For `Standard`: returns a function that returns the standard price
/// - For `Promotion`: prioritizes the promotion price, falling back to the standard price for non-targeted products
///
/// # Type Parameters
///
/// * `GetStandardPricesFn` - Function that returns a standard price retrieval function
/// * `GetPromotionPricesFn` - Function that returns a promotion price retrieval function
///
/// # Arguments
///
/// * `get_standard_prices` - Function that generates a standard price retrieval function
/// * `get_promotion_prices` - Function that generates a price retrieval function for a given promotion code
///
/// # Returns
///
/// A function that takes a `PricingMethod` and returns the corresponding price retrieval function
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::pricing::get_pricing_function;
/// use order_taking_sample::workflow::PricingMethod;
/// use order_taking_sample::simple_types::{ProductCode, Price, PromotionCode};
/// use rust_decimal::Decimal;
///
/// // Standard pricingretrievalfunction
/// let get_standard_prices = || {
///     Box::new(|_: &ProductCode| Price::create(Decimal::from(100)).unwrap())
///         as Box<dyn Fn(&ProductCode) -> Price>
/// };
///
/// // Promotional pricing retrieval function (always returns None)
/// let get_promotion_prices = |_: &PromotionCode| {
///     Box::new(|_: &ProductCode| None)
///         as Box<dyn Fn(&ProductCode) -> Option<Price>>
/// };
///
/// let pricing_fn = get_pricing_function(get_standard_prices, get_promotion_prices);
///
/// // Standard when
/// let get_price = pricing_fn(&PricingMethod::Standard);
/// let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
/// assert_eq!(get_price(&product_code).value(), Decimal::from(100));
/// ```
#[allow(clippy::type_complexity)]
pub fn get_pricing_function<GetStandardPricesFn, GetPromotionPricesFn>(
    get_standard_prices: GetStandardPricesFn,
    get_promotion_prices: GetPromotionPricesFn,
) -> impl Fn(&PricingMethod) -> Rc<dyn Fn(&ProductCode) -> Price>
where
    GetStandardPricesFn: Fn() -> Box<dyn Fn(&ProductCode) -> Price> + 'static,
    GetPromotionPricesFn:
        Fn(&PromotionCode) -> Box<dyn Fn(&ProductCode) -> Option<Price>> + 'static,
{
    // Use lambars' Lazy type to cache the standard price retrieval function
    let cached_standard_prices: Rc<Lazy<Box<dyn Fn(&ProductCode) -> Price>, GetStandardPricesFn>> =
        Rc::new(Lazy::new(get_standard_prices));

    let get_promotion_prices = Rc::new(get_promotion_prices);

    move |pricing_method: &PricingMethod| {
        // Deferred initialization of the standard price retrieval function (evaluated on first access via Lazy::force)
        let cached_standard_prices_clone = Rc::clone(&cached_standard_prices);

        let get_standard_price: Rc<dyn Fn(&ProductCode) -> Price> =
            Rc::new(move |product_code: &ProductCode| {
                cached_standard_prices_clone.force()(product_code)
            });

        match pricing_method {
            PricingMethod::Standard => get_standard_price,
            PricingMethod::Promotion(promotion_code) => {
                let promotion_price_function = get_promotion_prices(promotion_code);
                let standard_fallback = Rc::clone(&get_standard_price);

                Rc::new(move |product_code: &ProductCode| {
                    promotion_price_function(product_code)
                        .unwrap_or_else(|| standard_fallback(product_code))
                })
            }
        }
    }
}

// =============================================================================
// price_order (REQ-063)
// =============================================================================

/// Main function that converts a `ValidatedOrder` to a `PricedOrder`
///
/// Integrates all pricing logic and injects the price retrieval function.
///
/// # Type Parameters
///
/// * `GetPricingFn` - Function that returns a price retrieval function from a `PricingMethod`
///
/// # Arguments
///
/// * `get_pricing_function` - Function that returns a price retrieval function from a pricing method
/// * `validated_order` - Validated order
///
/// # Returns
///
/// * `Ok(PricedOrder)` - Priced order
/// * `Err(PlaceOrderError)` - Pricing error
///
/// # Errors
///
/// If an error such as overflow occurs during price calculation,
/// or if an error occurs during billing amount calculation, returns `PlaceOrderError::Pricing`.
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::pricing::price_order;
/// use order_taking_sample::workflow::{ValidatedOrder, ValidatedOrderLine, PricingMethod};
/// use order_taking_sample::simple_types::{OrderId, OrderLineId, ProductCode, OrderQuantity, Price};
/// use order_taking_sample::compound_types::{CustomerInfo, Address};
/// use rust_decimal::Decimal;
/// use std::rc::Rc;
///
/// let order_id = OrderId::create("OrderId", "order-001").unwrap();
/// let customer_info = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
/// let address = Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
///
/// let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
/// let validated_line = ValidatedOrderLine::new(
///     OrderLineId::create("OrderLineId", "line-001").unwrap(),
///     product_code.clone(),
///     OrderQuantity::create("Quantity", &product_code, Decimal::from(5)).unwrap(),
/// );
///
/// let validated_order = ValidatedOrder::new(
///     order_id,
///     customer_info,
///     address.clone(),
///     address,
///     vec![validated_line],
///     PricingMethod::Standard,
/// );
///
/// let get_pricing_fn = |_: &PricingMethod| {
///     Rc::new(|_: &ProductCode| Price::create(Decimal::from(100)).unwrap())
///         as Rc<dyn Fn(&ProductCode) -> Price>
/// };
///
/// let result = price_order(&get_pricing_fn, &validated_order);
/// assert!(result.is_ok());
/// ```
pub fn price_order<GetPricingFn>(
    get_pricing_function: &GetPricingFn,
    validated_order: &ValidatedOrder,
) -> Result<PricedOrder, PlaceOrderError>
where
    GetPricingFn: Fn(&PricingMethod) -> Rc<dyn Fn(&ProductCode) -> Price>,
{
    let get_product_price = get_pricing_function(validated_order.pricing_method());

    // Assign prices to each order line
    let priced_lines_result: Result<Vec<PricedOrderLine>, PricingError> = validated_order
        .lines()
        .iter()
        .map(|line| to_priced_order_line(&*get_product_price, line))
        .collect();

    let priced_lines = priced_lines_result.map_err(PlaceOrderError::Pricing)?;

    // Add comment lines
    let lines_with_comment = add_comment_line(validated_order.pricing_method(), priced_lines);

    // Calculate billing amount
    let line_prices: Vec<Price> = lines_with_comment.iter().map(get_line_price).collect();

    let amount_to_bill = BillingAmount::sum_prices(&line_prices).map_err(|validation_error| {
        PlaceOrderError::Pricing(PricingError::new(&format!(
            "Billing amount calculation error: {}",
            validation_error.message
        )))
    })?;

    Ok(PricedOrder::new(
        validated_order.order_id().clone(),
        validated_order.customer_info().clone(),
        validated_order.shipping_address().clone(),
        validated_order.billing_address().clone(),
        amount_to_bill,
        lines_with_comment,
        validated_order.pricing_method().clone(),
    ))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compound_types::{Address, CustomerInfo};
    use crate::simple_types::{OrderId, OrderLineId, OrderQuantity};
    use std::cell::Cell;

    // =========================================================================
    // Mock helper functions
    // =========================================================================

    fn create_product_code(code: &str) -> ProductCode {
        ProductCode::create("ProductCode", code).unwrap()
    }

    fn create_order_line_id(id: &str) -> OrderLineId {
        OrderLineId::create("OrderLineId", id).unwrap()
    }

    fn create_quantity(product_code: &ProductCode, value: i32) -> OrderQuantity {
        OrderQuantity::create("Quantity", product_code, Decimal::from(value)).unwrap()
    }

    fn create_price(value: i32) -> Price {
        Price::create(Decimal::from(value)).unwrap()
    }

    fn create_validated_order_line(
        line_id: &str,
        product_code_str: &str,
        quantity: i32,
    ) -> ValidatedOrderLine {
        let product_code = create_product_code(product_code_str);
        ValidatedOrderLine::new(
            create_order_line_id(line_id),
            product_code.clone(),
            create_quantity(&product_code, quantity),
        )
    }

    fn create_customer_info() -> CustomerInfo {
        CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap()
    }

    fn create_address() -> Address {
        Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap()
    }

    fn create_order_id() -> OrderId {
        OrderId::create("OrderId", "order-001").unwrap()
    }

    fn create_validated_order(
        lines: Vec<ValidatedOrderLine>,
        pricing_method: PricingMethod,
    ) -> ValidatedOrder {
        ValidatedOrder::new(
            create_order_id(),
            create_customer_info(),
            create_address(),
            create_address(),
            lines,
            pricing_method,
        )
    }

    // =========================================================================
    // get_line_price tests
    // =========================================================================

    mod get_line_price_tests {
        use super::*;

        #[test]
        fn test_product_line_returns_price() {
            let product_code = create_product_code("W1234");
            let product_line = PricedOrderProductLine::new(
                create_order_line_id("line-001"),
                product_code.clone(),
                create_quantity(&product_code, 5),
                create_price(500),
            );
            let line = PricedOrderLine::ProductLine(product_line);

            let price = get_line_price(&line);
            assert_eq!(price.value(), Decimal::from(500));
        }

        #[test]
        fn test_comment_line_returns_zero() {
            let line = PricedOrderLine::CommentLine("Applied promotion".to_string());

            let price = get_line_price(&line);
            assert_eq!(price.value(), Decimal::ZERO);
        }
    }

    // =========================================================================
    // to_priced_order_line tests
    // =========================================================================

    mod to_priced_order_line_tests {
        use super::*;

        #[test]
        fn test_widget_price_calculation() {
            let validated_line = create_validated_order_line("line-001", "W1234", 10);
            let get_price = |_: &ProductCode| create_price(50);

            let result = to_priced_order_line(&get_price, &validated_line);

            assert!(result.is_ok());
            let priced_line = result.unwrap();
            assert!(priced_line.is_product_line());
            assert_eq!(
                priced_line.line_price().unwrap().value(),
                Decimal::from(500)
            );
        }

        #[test]
        fn test_gizmo_price_calculation() {
            let product_code = create_product_code("G123");
            let validated_line = ValidatedOrderLine::new(
                create_order_line_id("line-002"),
                product_code.clone(),
                OrderQuantity::create("Quantity", &product_code, Decimal::new(55, 1)).unwrap(), // 5.5
            );
            let get_price = |_: &ProductCode| create_price(20);

            let result = to_priced_order_line(&get_price, &validated_line);

            assert!(result.is_ok());
            let priced_line = result.unwrap();
            // 5.5 * 20 = 110
            assert_eq!(
                priced_line.line_price().unwrap().value(),
                Decimal::from(110)
            );
        }

        #[test]
        fn test_price_overflow_boundary() {
            // 11 * 100 = 1100 > 1000 (Price upper limit)
            let validated_line = create_validated_order_line("line-001", "W1234", 11);
            let get_price = |_: &ProductCode| create_price(100);

            let result = to_priced_order_line(&get_price, &validated_line);

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert!(error.message().contains("overflow"));
        }

        #[test]
        fn test_price_boundary_max() {
            // 10 * 100 = 1000 (exactly at Price upper limit)
            let validated_line = create_validated_order_line("line-001", "W1234", 10);
            let get_price = |_: &ProductCode| create_price(100);

            let result = to_priced_order_line(&get_price, &validated_line);

            assert!(result.is_ok());
            let priced_line = result.unwrap();
            assert_eq!(
                priced_line.line_price().unwrap().value(),
                Decimal::from(1000)
            );
        }
    }

    // =========================================================================
    // add_comment_line tests
    // =========================================================================

    mod add_comment_line_tests {
        use super::*;

        #[test]
        fn test_standard_no_comment() {
            let lines: Vec<PricedOrderLine> = vec![];
            let result = add_comment_line(&PricingMethod::Standard, lines);
            assert!(result.is_empty());
        }

        #[test]
        fn test_promotion_adds_comment() {
            let lines: Vec<PricedOrderLine> = vec![];
            let promo_code = PromotionCode::new("SUMMER2024".to_string());
            let result = add_comment_line(&PricingMethod::Promotion(promo_code), lines);

            assert_eq!(result.len(), 1);
            assert!(result[0].is_comment_line());
            if let PricedOrderLine::CommentLine(comment) = &result[0] {
                assert!(comment.contains("Applied promotion SUMMER2024"));
            }
        }

        #[test]
        fn test_promotion_appends_to_existing_lines() {
            let product_code = create_product_code("W1234");
            let product_line = PricedOrderProductLine::new(
                create_order_line_id("line-001"),
                product_code.clone(),
                create_quantity(&product_code, 5),
                create_price(500),
            );
            let lines = vec![PricedOrderLine::ProductLine(product_line)];
            let promo_code = PromotionCode::new("WINTER".to_string());

            let result = add_comment_line(&PricingMethod::Promotion(promo_code), lines);

            assert_eq!(result.len(), 2);
            assert!(result[0].is_product_line());
            assert!(result[1].is_comment_line());
        }
    }

    // =========================================================================
    // get_pricing_function tests
    // =========================================================================

    mod get_pricing_function_tests {
        use super::*;

        #[test]
        fn test_standard_pricing() {
            let get_standard_prices = || {
                Box::new(|_: &ProductCode| create_price(100)) as Box<dyn Fn(&ProductCode) -> Price>
            };
            let get_promotion_prices = |_: &PromotionCode| {
                Box::new(|_: &ProductCode| None) as Box<dyn Fn(&ProductCode) -> Option<Price>>
            };

            let pricing_fn = get_pricing_function(get_standard_prices, get_promotion_prices);
            let get_price = pricing_fn(&PricingMethod::Standard);
            let product_code = create_product_code("W1234");

            assert_eq!(get_price(&product_code).value(), Decimal::from(100));
        }

        #[test]
        fn test_promotion_pricing_with_promo_price() {
            let get_standard_prices = || {
                Box::new(|_: &ProductCode| create_price(100)) as Box<dyn Fn(&ProductCode) -> Price>
            };
            let get_promotion_prices = |_: &PromotionCode| {
                Box::new(|_: &ProductCode| Some(create_price(80)))
                    as Box<dyn Fn(&ProductCode) -> Option<Price>>
            };

            let pricing_fn = get_pricing_function(get_standard_prices, get_promotion_prices);
            let promo_code = PromotionCode::new("SUMMER2024".to_string());
            let get_price = pricing_fn(&PricingMethod::Promotion(promo_code));
            let product_code = create_product_code("W1234");

            assert_eq!(get_price(&product_code).value(), Decimal::from(80));
        }

        #[test]
        fn test_promotion_pricing_fallback_to_standard() {
            let get_standard_prices = || {
                Box::new(|_: &ProductCode| create_price(150)) as Box<dyn Fn(&ProductCode) -> Price>
            };
            let get_promotion_prices = |_: &PromotionCode| {
                Box::new(|_: &ProductCode| None) as Box<dyn Fn(&ProductCode) -> Option<Price>>
            };

            let pricing_fn = get_pricing_function(get_standard_prices, get_promotion_prices);
            let promo_code = PromotionCode::new("SUMMER2024".to_string());
            let get_price = pricing_fn(&PricingMethod::Promotion(promo_code));
            let product_code = create_product_code("G123");

            // Not a promotion target, so falls back to the standard price
            assert_eq!(get_price(&product_code).value(), Decimal::from(150));
        }

        #[test]
        fn test_caches_standard_prices() {
            let call_count = Rc::new(Cell::new(0));
            let call_count_clone = Rc::clone(&call_count);

            let get_standard_prices = move || {
                call_count_clone.set(call_count_clone.get() + 1);
                Box::new(|_: &ProductCode| create_price(100)) as Box<dyn Fn(&ProductCode) -> Price>
            };
            let get_promotion_prices = |_: &PromotionCode| {
                Box::new(|_: &ProductCode| None) as Box<dyn Fn(&ProductCode) -> Option<Price>>
            };

            let pricing_fn = get_pricing_function(get_standard_prices, get_promotion_prices);
            let product_code = create_product_code("W1234");

            // Multiple calls
            let get_price1 = pricing_fn(&PricingMethod::Standard);
            let _ = get_price1(&product_code);
            let get_price2 = pricing_fn(&PricingMethod::Standard);
            let _ = get_price2(&product_code);
            let get_price3 = pricing_fn(&PricingMethod::Standard);
            let _ = get_price3(&product_code);

            // Initialization happens only once
            assert_eq!(call_count.get(), 1);
        }
    }

    // =========================================================================
    // price_order tests
    // =========================================================================

    mod price_order_tests {
        use super::*;

        #[test]
        fn test_single_line_order() {
            let validated_line = create_validated_order_line("line-001", "W1234", 5);
            let validated_order =
                create_validated_order(vec![validated_line], PricingMethod::Standard);

            let get_pricing_fn = |_: &PricingMethod| {
                Rc::new(|_: &ProductCode| create_price(100)) as Rc<dyn Fn(&ProductCode) -> Price>
            };

            let result = price_order(&get_pricing_fn, &validated_order);

            assert!(result.is_ok());
            let priced_order = result.unwrap();
            // 5 * 100 = 500
            assert_eq!(priced_order.amount_to_bill().value(), Decimal::from(500));
            assert_eq!(priced_order.lines().len(), 1);
        }

        #[test]
        fn test_multiple_lines_order() {
            let lines = vec![
                create_validated_order_line("line-001", "W1234", 5), // 5 * 100 = 500
                create_validated_order_line("line-002", "W5678", 3), // 3 * 100 = 300
            ];
            let validated_order = create_validated_order(lines, PricingMethod::Standard);

            let get_pricing_fn = |_: &PricingMethod| {
                Rc::new(|_: &ProductCode| create_price(100)) as Rc<dyn Fn(&ProductCode) -> Price>
            };

            let result = price_order(&get_pricing_fn, &validated_order);

            assert!(result.is_ok());
            let priced_order = result.unwrap();
            // 500 + 300 = 800
            assert_eq!(priced_order.amount_to_bill().value(), Decimal::from(800));
            assert_eq!(priced_order.lines().len(), 2);
        }

        #[test]
        fn test_with_promotion() {
            let validated_line = create_validated_order_line("line-001", "W1234", 5);
            let promo_code = PromotionCode::new("SUMMER2024".to_string());
            let validated_order =
                create_validated_order(vec![validated_line], PricingMethod::Promotion(promo_code));

            let get_pricing_fn = |_: &PricingMethod| {
                Rc::new(|_: &ProductCode| create_price(80)) as Rc<dyn Fn(&ProductCode) -> Price>
            };

            let result = price_order(&get_pricing_fn, &validated_order);

            assert!(result.is_ok());
            let priced_order = result.unwrap();
            // 5 * 80 = 400
            assert_eq!(priced_order.amount_to_bill().value(), Decimal::from(400));
            // 1 product line + 1 comment line
            assert_eq!(priced_order.lines().len(), 2);
            assert!(priced_order.lines()[1].is_comment_line());
        }

        #[test]
        fn test_empty_order() {
            let validated_order = create_validated_order(vec![], PricingMethod::Standard);

            let get_pricing_fn = |_: &PricingMethod| {
                Rc::new(|_: &ProductCode| create_price(100)) as Rc<dyn Fn(&ProductCode) -> Price>
            };

            let result = price_order(&get_pricing_fn, &validated_order);

            assert!(result.is_ok());
            let priced_order = result.unwrap();
            assert_eq!(priced_order.amount_to_bill().value(), Decimal::ZERO);
            assert!(priced_order.lines().is_empty());
        }

        #[test]
        fn test_pricing_error_propagation() {
            // 11 * 100 = 1100 > 1000 (Price upper limit)
            let validated_line = create_validated_order_line("line-001", "W1234", 11);
            let validated_order =
                create_validated_order(vec![validated_line], PricingMethod::Standard);

            let get_pricing_fn = |_: &PricingMethod| {
                Rc::new(|_: &ProductCode| create_price(100)) as Rc<dyn Fn(&ProductCode) -> Price>
            };

            let result = price_order(&get_pricing_fn, &validated_order);

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert!(error.is_pricing());
        }

        #[test]
        fn test_preserves_order_fields() {
            let validated_line = create_validated_order_line("line-001", "W1234", 5);
            let validated_order =
                create_validated_order(vec![validated_line], PricingMethod::Standard);

            let get_pricing_fn = |_: &PricingMethod| {
                Rc::new(|_: &ProductCode| create_price(100)) as Rc<dyn Fn(&ProductCode) -> Price>
            };

            let result = price_order(&get_pricing_fn, &validated_order);

            assert!(result.is_ok());
            let priced_order = result.unwrap();
            assert_eq!(priced_order.order_id(), validated_order.order_id());
            assert_eq!(
                priced_order.customer_info(),
                validated_order.customer_info()
            );
            assert_eq!(
                priced_order.shipping_address(),
                validated_order.shipping_address()
            );
            assert_eq!(
                priced_order.billing_address(),
                validated_order.billing_address()
            );
            assert_eq!(
                priced_order.pricing_method(),
                validated_order.pricing_method()
            );
        }

        #[test]
        fn test_billing_amount_overflow() {
            // BillingAmount upper limit is 10000
            // 11 lines x 1000 yen = 11000 > 10000, overflow
            // Price upper limit is 1000, so each line is 10 units x 100 yen = 1000 yen
            let mut lines = Vec::new();
            for index in 0..11 {
                lines.push(create_validated_order_line(
                    &format!("line-{:03}", index),
                    "W1234",
                    10,
                ));
            }
            let validated_order = create_validated_order(lines, PricingMethod::Standard);

            let get_pricing_fn = |_: &PricingMethod| {
                Rc::new(|_: &ProductCode| create_price(100)) as Rc<dyn Fn(&ProductCode) -> Price>
            };

            let result = price_order(&get_pricing_fn, &validated_order);

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert!(error.is_pricing());
            if let PlaceOrderError::Pricing(pricing_error) = error {
                assert!(pricing_error.message().contains("Billing amount"));
            }
        }
    }
}
