//! 価格付き型
//!
//! 価格計算済みのデータを表す型を定義する。
//! 検証済みデータに価格情報を追加した状態を表現する。
//!
//! # 型一覧
//!
//! - [`PricedOrderProductLine`] - 価格付き製品注文明細
//! - [`PricedOrderLine`] - 価格付き注文明細（製品またはコメント）
//! - [`PricedOrder`] - 価格計算済み注文

use crate::compound_types::{Address, CustomerInfo};
use crate::simple_types::{BillingAmount, OrderId, OrderLineId, OrderQuantity, Price, ProductCode};
use crate::workflow::validated_types::PricingMethod;
use lambars_derive::Lenses;

// =============================================================================
// PricedOrderProductLine
// =============================================================================

/// 価格付き製品注文明細
///
/// [`ValidatedOrderLine`](super::ValidatedOrderLine) に価格情報を追加した型。
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
    /// 新しい `PricedOrderProductLine` を生成する
    ///
    /// # Arguments
    ///
    /// * `order_line_id` - 注文明細ID
    /// * `product_code` - 製品コード
    /// * `quantity` - 数量
    /// * `line_price` - 明細の合計価格（単価 x 数量）
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

    /// 注文明細IDへの参照を返す
    #[must_use]
    pub const fn order_line_id(&self) -> &OrderLineId {
        &self.order_line_id
    }

    /// 製品コードへの参照を返す
    #[must_use]
    pub const fn product_code(&self) -> &ProductCode {
        &self.product_code
    }

    /// 数量への参照を返す
    #[must_use]
    pub const fn quantity(&self) -> &OrderQuantity {
        &self.quantity
    }

    /// 明細価格への参照を返す
    #[must_use]
    pub const fn line_price(&self) -> &Price {
        &self.line_price
    }
}

// =============================================================================
// PricedOrderLine
// =============================================================================

/// 価格付き注文明細
///
/// 製品明細またはコメント明細のいずれかを表す直和型。
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{PricedOrderLine, PricedOrderProductLine};
/// use order_taking_sample::simple_types::{OrderLineId, ProductCode, OrderQuantity, Price};
/// use rust_decimal::Decimal;
///
/// // 製品明細
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
/// // コメント明細
/// let comment_line = PricedOrderLine::CommentLine("Gift message: Happy Birthday!".to_string());
/// assert!(comment_line.is_comment_line());
/// assert!(comment_line.line_price().is_none());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PricedOrderLine {
    /// 製品の明細（価格付き）
    ProductLine(PricedOrderProductLine),

    /// コメント行（ギフトメッセージなど）
    CommentLine(String),
}

impl PricedOrderLine {
    /// `ProductLine` バリアントかどうかを返す
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

    /// `CommentLine` バリアントかどうかを返す
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

    /// 明細の価格を返す
    ///
    /// `ProductLine` の場合は `Some(&Price)` を、
    /// `CommentLine` の場合は `None` を返す。
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{PricedOrderLine, PricedOrderProductLine};
    /// use order_taking_sample::simple_types::{OrderLineId, ProductCode, OrderQuantity, Price};
    /// use rust_decimal::Decimal;
    ///
    /// // ProductLine は価格を持つ
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
    /// // CommentLine は価格を持たない
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

/// 価格計算済み注文
///
/// [`ValidatedOrder`](super::ValidatedOrder) に価格情報を追加した型。
/// 請求金額と価格付き明細リストを持つ。
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
    /// 新しい `PricedOrder` を生成する
    ///
    /// # Arguments
    ///
    /// * `order_id` - 注文ID
    /// * `customer_info` - 顧客情報
    /// * `shipping_address` - 配送先住所
    /// * `billing_address` - 請求先住所
    /// * `amount_to_bill` - 請求合計金額
    /// * `lines` - 価格付き注文明細リスト
    /// * `pricing_method` - 使用された価格計算方法
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

    /// 注文IDへの参照を返す
    #[must_use]
    pub const fn order_id(&self) -> &OrderId {
        &self.order_id
    }

    /// 顧客情報への参照を返す
    #[must_use]
    pub const fn customer_info(&self) -> &CustomerInfo {
        &self.customer_info
    }

    /// 配送先住所への参照を返す
    #[must_use]
    pub const fn shipping_address(&self) -> &Address {
        &self.shipping_address
    }

    /// 請求先住所への参照を返す
    #[must_use]
    pub const fn billing_address(&self) -> &Address {
        &self.billing_address
    }

    /// 請求金額への参照を返す
    #[must_use]
    pub const fn amount_to_bill(&self) -> &BillingAmount {
        &self.amount_to_bill
    }

    /// 価格付き明細リストへの参照を返す
    #[must_use]
    pub fn lines(&self) -> &[PricedOrderLine] {
        &self.lines
    }

    /// 価格計算方法への参照を返す
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
