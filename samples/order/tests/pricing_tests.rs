//! Supplementary tests for the pricing module
//!
//! Lazy type caching behavior from lambars,
//! decimal precision tests, and edge case tests.

use order_taking_sample::compound_types::{Address, CustomerInfo};
use order_taking_sample::simple_types::{
    BillingAmount, OrderId, OrderLineId, OrderQuantity, Price, ProductCode, PromotionCode,
};
use order_taking_sample::workflow::pricing::{
    add_comment_line, get_line_price, get_pricing_function, price_order, to_priced_order_line,
};
use order_taking_sample::workflow::{
    PricedOrderLine, PricedOrderProductLine, PricingMethod, ValidatedOrder, ValidatedOrderLine,
};
use rstest::rstest;
use rust_decimal::Decimal;
use std::cell::Cell;
use std::rc::Rc;
use std::str::FromStr;

// =============================================================================
// Test data factory
// =============================================================================

fn create_product_code(code: &str) -> ProductCode {
    ProductCode::create("ProductCode", code).unwrap()
}

fn create_order_line_id(id: &str) -> OrderLineId {
    OrderLineId::create("OrderLineId", id).unwrap()
}

fn create_price(value: i32) -> Price {
    Price::create(Decimal::from(value)).unwrap()
}

fn create_price_decimal(value: &str) -> Price {
    Price::create(Decimal::from_str(value).unwrap()).unwrap()
}

fn create_quantity(product_code: &ProductCode, value: i32) -> OrderQuantity {
    OrderQuantity::create("Quantity", product_code, Decimal::from(value)).unwrap()
}

fn create_quantity_decimal(product_code: &ProductCode, value: &str) -> OrderQuantity {
    OrderQuantity::create("Quantity", product_code, Decimal::from_str(value).unwrap()).unwrap()
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

// =============================================================================
// Lazy caching behavior tests
// =============================================================================

mod lazy_cache_tests {
    use super::*;

    #[rstest]
    fn test_standard_pricing_caches_initialization() {
        let initialization_count = Rc::new(Cell::new(0));
        let initialization_count_clone = Rc::clone(&initialization_count);

        let get_standard_prices = move || {
            initialization_count_clone.set(initialization_count_clone.get() + 1);
            Box::new(|_: &ProductCode| create_price(100)) as Box<dyn Fn(&ProductCode) -> Price>
        };
        let get_promotion_prices = |_: &PromotionCode| {
            Box::new(|_: &ProductCode| None) as Box<dyn Fn(&ProductCode) -> Option<Price>>
        };

        let pricing_fn = get_pricing_function(get_standard_prices, get_promotion_prices);
        let product_code = create_product_code("W1234");

        // Retrieve standard pricing multiple times
        for _ in 0..10 {
            let get_price = pricing_fn(&PricingMethod::Standard);
            let _ = get_price(&product_code);
        }

        // Initialization happens only once (deferred evaluation and caching via Lazy)
        assert_eq!(initialization_count.get(), 1);
    }

    #[rstest]
    fn test_promotion_pricing_also_uses_cached_standard() {
        let standard_init_count = Rc::new(Cell::new(0));
        let standard_init_count_clone = Rc::clone(&standard_init_count);

        let get_standard_prices = move || {
            standard_init_count_clone.set(standard_init_count_clone.get() + 1);
            Box::new(|_: &ProductCode| create_price(100)) as Box<dyn Fn(&ProductCode) -> Price>
        };
        let get_promotion_prices = |_: &PromotionCode| {
            // Not a promotion target (returns None) -> falls back to standard price
            Box::new(|_: &ProductCode| None) as Box<dyn Fn(&ProductCode) -> Option<Price>>
        };

        let pricing_fn = get_pricing_function(get_standard_prices, get_promotion_prices);
        let product_code = create_product_code("W1234");
        let promo_code = PromotionCode::new("SUMMER2024".to_string());

        // Called once with standard pricing
        let get_standard = pricing_fn(&PricingMethod::Standard);
        let _ = get_standard(&product_code);

        // Called multiple times with promotional pricing (fallback)
        for _ in 0..5 {
            let get_promo = pricing_fn(&PricingMethod::Promotion(promo_code.clone()));
            let _ = get_promo(&product_code);
        }

        // Standard pricing function initialization happens only once
        assert_eq!(standard_init_count.get(), 1);
    }

    #[rstest]
    fn test_lazy_deferred_until_first_access() {
        let initialized = Rc::new(Cell::new(false));
        let initialized_clone = Rc::clone(&initialized);

        let get_standard_prices = move || {
            initialized_clone.set(true);
            Box::new(|_: &ProductCode| create_price(100)) as Box<dyn Fn(&ProductCode) -> Price>
        };
        let get_promotion_prices = |_: &PromotionCode| {
            Box::new(|_: &ProductCode| None) as Box<dyn Fn(&ProductCode) -> Option<Price>>
        };

        let pricing_fn = get_pricing_function(get_standard_prices, get_promotion_prices);

        // Creating pricing_fn does not trigger initialization
        assert!(!initialized.get());

        // Initialization occurs when actually performing price retrieval
        let product_code = create_product_code("W1234");
        let get_price = pricing_fn(&PricingMethod::Standard);
        let _ = get_price(&product_code);

        assert!(initialized.get());
    }
}

// =============================================================================
// Decimal precision tests
// =============================================================================

mod decimal_precision_tests {
    use super::*;

    #[rstest]
    fn test_gizmo_quantity_precision() {
        let product_code = create_product_code("G123");
        let quantity = create_quantity_decimal(&product_code, "5.55"); // 5.55 kg
        let validated_line =
            ValidatedOrderLine::new(create_order_line_id("line-001"), product_code, quantity);

        let get_price = |_: &ProductCode| create_price_decimal("10.00"); // 10.00 per kg

        let result = to_priced_order_line(&get_price, &validated_line);

        assert!(result.is_ok());
        let priced_line = result.unwrap();
        // 5.55 * 10.00 = 55.50
        assert_eq!(
            priced_line.line_price().unwrap().value(),
            Decimal::from_str("55.50").unwrap()
        );
    }

    #[rstest]
    fn test_price_with_cents() {
        let validated_line = create_validated_order_line("line-001", "W1234", 3);
        let get_price = |_: &ProductCode| create_price_decimal("99.99"); // 99.99

        let result = to_priced_order_line(&get_price, &validated_line);

        assert!(result.is_ok());
        let priced_line = result.unwrap();
        // 3 * 99.99 = 299.97
        assert_eq!(
            priced_line.line_price().unwrap().value(),
            Decimal::from_str("299.97").unwrap()
        );
    }

    #[rstest]
    fn test_small_quantity_small_price() {
        let product_code = create_product_code("G123");
        let quantity = create_quantity_decimal(&product_code, "0.05"); // minimumquantity
        let validated_line =
            ValidatedOrderLine::new(create_order_line_id("line-001"), product_code, quantity);

        let get_price = |_: &ProductCode| create_price_decimal("0.01"); // minimumprice

        let result = to_priced_order_line(&get_price, &validated_line);

        assert!(result.is_ok());
        let priced_line = result.unwrap();
        // 0.05 * 0.01 = 0.0005 (extremely small)
        assert!(priced_line.line_price().unwrap().value() < Decimal::from_str("0.01").unwrap());
    }

    #[rstest]
    fn test_billing_amount_sum_precision() {
        let prices = vec![
            create_price_decimal("33.33"),
            create_price_decimal("33.33"),
            create_price_decimal("33.34"),
        ];
        let result = BillingAmount::sum_prices(&prices);

        assert!(result.is_ok());
        // 33.33 + 33.33 + 33.34 = 100.00
        assert_eq!(result.unwrap().value(), Decimal::from(100));
    }
}

// =============================================================================
// Edge case tests
// =============================================================================

mod edge_case_tests {
    use super::*;

    #[rstest]
    fn test_zero_price_product() {
        let validated_line = create_validated_order_line("line-001", "W1234", 10);
        let get_price = |_: &ProductCode| create_price(0); // freeproduct

        let result = to_priced_order_line(&get_price, &validated_line);

        assert!(result.is_ok());
        let priced_line = result.unwrap();
        // 10 * 0 = 0
        assert_eq!(priced_line.line_price().unwrap().value(), Decimal::ZERO);
    }

    #[rstest]
    fn test_max_valid_price() {
        let validated_line = create_validated_order_line("line-001", "W1234", 1);
        let get_price = |_: &ProductCode| create_price(1000); // Price upper limit

        let result = to_priced_order_line(&get_price, &validated_line);

        assert!(result.is_ok());
        let priced_line = result.unwrap();
        // 1 * 1000 = 1000
        assert_eq!(
            priced_line.line_price().unwrap().value(),
            Decimal::from(1000)
        );
    }

    #[rstest]
    fn test_price_just_below_overflow() {
        let validated_line = create_validated_order_line("line-001", "W1234", 10);
        let get_price = |_: &ProductCode| create_price(100);

        let result = to_priced_order_line(&get_price, &validated_line);

        assert!(result.is_ok());
        // 10 * 100 = 1000 (exactly at upper limit)
        assert_eq!(
            result.unwrap().line_price().unwrap().value(),
            Decimal::from(1000)
        );
    }

    #[rstest]
    fn test_price_just_over_overflow() {
        let validated_line = create_validated_order_line("line-001", "W1234", 11);
        let get_price = |_: &ProductCode| create_price(100);

        let result = to_priced_order_line(&get_price, &validated_line);

        // 11 * 100 = 1100 > 1000 (exceeds upper limit)
        assert!(result.is_err());
    }

    #[rstest]
    fn test_billing_amount_max_valid() {
        // BillingAmount upper limit is 10000
        // 10 lines x 1000 yen = 10000 (exactly at upper limit)
        let mut lines = Vec::new();
        for index in 0..10 {
            lines.push(create_validated_order_line(
                &format!("line-{:03}", index),
                "W1234",
                10, // 10 units x 100 yen = 1000 yen
            ));
        }
        let validated_order = create_validated_order(lines, PricingMethod::Standard);

        let get_pricing_fn = |_: &PricingMethod| {
            Rc::new(|_: &ProductCode| create_price(100)) as Rc<dyn Fn(&ProductCode) -> Price>
        };

        let result = price_order(&get_pricing_fn, &validated_order);

        assert!(result.is_ok());
        let priced_order = result.unwrap();
        assert_eq!(priced_order.amount_to_bill().value(), Decimal::from(10000));
    }

    #[rstest]
    fn test_billing_amount_just_over_max() {
        // 10000.01 > 10000 (exceeds upper limit)
        // 10 lines x 1000 yen + 1 line x 0.01 yen (not actually possible, but conceptually)
        // Instead test with 11 lines x 1000 yen = 11000
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
    }
}

// =============================================================================
// get_line_price Test
// =============================================================================

mod get_line_price_tests {
    use super::*;

    #[rstest]
    fn test_product_line_returns_exact_price() {
        let product_code = create_product_code("W1234");
        let product_line = PricedOrderProductLine::new(
            create_order_line_id("line-001"),
            product_code.clone(),
            create_quantity(&product_code, 5),
            create_price_decimal("123.45"),
        );
        let line = PricedOrderLine::ProductLine(product_line);

        let price = get_line_price(&line);
        assert_eq!(price.value(), Decimal::from_str("123.45").unwrap());
    }

    #[rstest]
    fn test_comment_line_always_zero() {
        let test_cases = vec![
            "Applied promotion SUMMER2024",
            "",
            "Special discount",
            "Very long comment that describes a complex promotion scenario",
        ];

        for comment in test_cases {
            let line = PricedOrderLine::CommentLine(comment.to_string());
            let price = get_line_price(&line);
            assert_eq!(
                price.value(),
                Decimal::ZERO,
                "Failed for comment: {comment}"
            );
        }
    }
}

// =============================================================================
// add_comment_line Test
// =============================================================================

mod add_comment_line_tests {
    use super::*;

    #[rstest]
    fn test_standard_preserves_original_lines() {
        let product_code = create_product_code("W1234");
        let product_line = PricedOrderProductLine::new(
            create_order_line_id("line-001"),
            product_code.clone(),
            create_quantity(&product_code, 5),
            create_price(500),
        );
        let original_lines = vec![PricedOrderLine::ProductLine(product_line)];
        let original_len = original_lines.len();

        let result = add_comment_line(&PricingMethod::Standard, original_lines);

        assert_eq!(result.len(), original_len);
    }

    #[rstest]
    fn test_promotion_includes_code_in_comment() {
        let promo_code = PromotionCode::new("WINTER50OFF".to_string());
        let lines: Vec<PricedOrderLine> = vec![];

        let result = add_comment_line(&PricingMethod::Promotion(promo_code), lines);

        assert_eq!(result.len(), 1);
        if let PricedOrderLine::CommentLine(comment) = &result[0] {
            assert!(comment.contains("WINTER50OFF"));
        } else {
            panic!("Expected CommentLine");
        }
    }

    #[rstest]
    fn test_empty_promotion_code() {
        let promo_code = PromotionCode::new(String::new());
        let lines: Vec<PricedOrderLine> = vec![];

        let result = add_comment_line(&PricingMethod::Promotion(promo_code), lines);

        assert_eq!(result.len(), 1);
        if let PricedOrderLine::CommentLine(comment) = &result[0] {
            assert!(comment.contains("Applied promotion"));
        } else {
            panic!("Expected CommentLine");
        }
    }
}

// =============================================================================
// Multiple product type tests
// =============================================================================

mod mixed_product_types_tests {
    use super::*;

    #[rstest]
    fn test_widget_and_gizmo_mixed_order() {
        let widget_code = create_product_code("W1234");
        let gizmo_code = create_product_code("G123");

        let widget_line = ValidatedOrderLine::new(
            create_order_line_id("line-001"),
            widget_code.clone(),
            create_quantity(&widget_code, 5), // 5 units
        );
        let gizmo_line = ValidatedOrderLine::new(
            create_order_line_id("line-002"),
            gizmo_code.clone(),
            create_quantity_decimal(&gizmo_code, "2.5"), // 2.5 kg
        );

        let validated_order =
            create_validated_order(vec![widget_line, gizmo_line], PricingMethod::Standard);

        // Widget: 100, Gizmo: 50
        let get_pricing_fn = |_: &PricingMethod| {
            Rc::new(|product_code: &ProductCode| match product_code {
                ProductCode::Widget(_) => create_price(100),
                ProductCode::Gizmo(_) => create_price(50),
            }) as Rc<dyn Fn(&ProductCode) -> Price>
        };

        let result = price_order(&get_pricing_fn, &validated_order);

        assert!(result.is_ok());
        let priced_order = result.unwrap();
        // Widget: 5 * 100 = 500
        // Gizmo: 2.5 * 50 = 125
        // Total: 625
        assert_eq!(priced_order.amount_to_bill().value(), Decimal::from(625));
    }
}
