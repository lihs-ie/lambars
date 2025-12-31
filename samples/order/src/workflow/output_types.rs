//! 出力イベント型
//!
//! `PlaceOrder` ワークフローの出力イベントを表す型を定義する。
//!
//! # 型一覧
//!
//! - [`OrderAcknowledgmentSent`] - 注文確認メール送信イベント
//! - [`ShippableOrderLine`] - 配送対象の注文明細
//! - [`ShippableOrderPlaced`] - 配送可能注文確定イベント
//! - [`BillableOrderPlaced`] - 請求可能注文確定イベント
//! - [`PlaceOrderEvent`] - ワークフロー出力イベント

use crate::compound_types::Address;
use crate::simple_types::{
    BillingAmount, EmailAddress, OrderId, OrderQuantity, PdfAttachment, ProductCode,
};

// =============================================================================
// OrderAcknowledgmentSent
// =============================================================================

/// 注文確認メール送信イベント
///
/// 注文確認メールが送信されたことを表すイベント型。
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::OrderAcknowledgmentSent;
/// use order_taking_sample::simple_types::{OrderId, EmailAddress};
///
/// let order_id = OrderId::create("OrderId", "order-001").unwrap();
/// let email = EmailAddress::create("EmailAddress", "john@example.com").unwrap();
/// let event = OrderAcknowledgmentSent::new(order_id, email);
///
/// assert_eq!(event.order_id().value(), "order-001");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderAcknowledgmentSent {
    order_id: OrderId,
    email_address: EmailAddress,
}

impl OrderAcknowledgmentSent {
    /// 新しい `OrderAcknowledgmentSent` を生成する
    ///
    /// # Arguments
    ///
    /// * `order_id` - 確認メールが送信された注文のID
    /// * `email_address` - 確認メールの送信先アドレス
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::OrderAcknowledgmentSent;
    /// use order_taking_sample::simple_types::{OrderId, EmailAddress};
    ///
    /// let order_id = OrderId::create("OrderId", "order-002").unwrap();
    /// let email = EmailAddress::create("EmailAddress", "jane@example.com").unwrap();
    /// let event = OrderAcknowledgmentSent::new(order_id, email);
    /// ```
    #[must_use]
    pub const fn new(order_id: OrderId, email_address: EmailAddress) -> Self {
        Self {
            order_id,
            email_address,
        }
    }

    /// 注文IDへの参照を返す
    #[must_use]
    pub const fn order_id(&self) -> &OrderId {
        &self.order_id
    }

    /// メールアドレスへの参照を返す
    #[must_use]
    pub const fn email_address(&self) -> &EmailAddress {
        &self.email_address
    }
}

// =============================================================================
// ShippableOrderLine
// =============================================================================

/// 配送対象の注文明細
///
/// 製品コードと数量のみを保持し、配送システムへの入力として使用される。
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::ShippableOrderLine;
/// use order_taking_sample::simple_types::{ProductCode, OrderQuantity};
/// use rust_decimal::Decimal;
///
/// let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
/// let quantity = OrderQuantity::create("Quantity", &product_code, Decimal::from(10)).unwrap();
/// let line = ShippableOrderLine::new(product_code, quantity);
///
/// assert_eq!(line.product_code().value(), "W1234");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShippableOrderLine {
    product_code: ProductCode,
    quantity: OrderQuantity,
}

impl ShippableOrderLine {
    /// 新しい `ShippableOrderLine` を生成する
    ///
    /// # Arguments
    ///
    /// * `product_code` - 配送対象の製品コード
    /// * `quantity` - 配送数量
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::ShippableOrderLine;
    /// use order_taking_sample::simple_types::{ProductCode, OrderQuantity};
    /// use rust_decimal::Decimal;
    ///
    /// let product_code = ProductCode::create("ProductCode", "G123").unwrap();
    /// let quantity = OrderQuantity::create("Quantity", &product_code, Decimal::new(25, 1)).unwrap();
    /// let line = ShippableOrderLine::new(product_code, quantity);
    /// ```
    #[must_use]
    pub const fn new(product_code: ProductCode, quantity: OrderQuantity) -> Self {
        Self {
            product_code,
            quantity,
        }
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
}

// =============================================================================
// ShippableOrderPlaced
// =============================================================================

/// 配送可能注文確定イベント
///
/// 配送可能な注文が確定したことを表すイベント型。
/// 配送に必要な情報（注文ID、配送先住所、明細、PDF）を保持する。
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{ShippableOrderPlaced, ShippableOrderLine};
/// use order_taking_sample::simple_types::{OrderId, ProductCode, OrderQuantity, PdfAttachment};
/// use order_taking_sample::compound_types::Address;
/// use rust_decimal::Decimal;
///
/// let order_id = OrderId::create("OrderId", "order-001").unwrap();
/// let address = Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
/// let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
/// let lines = vec![
///     ShippableOrderLine::new(
///         product_code.clone(),
///         OrderQuantity::create("Quantity", &product_code, Decimal::from(5)).unwrap(),
///     ),
/// ];
/// let pdf = PdfAttachment::new("label.pdf".to_string(), vec![1, 2, 3]);
///
/// let event = ShippableOrderPlaced::new(order_id, address, lines, pdf);
/// assert_eq!(event.shipment_lines().len(), 1);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShippableOrderPlaced {
    order_id: OrderId,
    shipping_address: Address,
    shipment_lines: Vec<ShippableOrderLine>,
    pdf: PdfAttachment,
}

impl ShippableOrderPlaced {
    /// 新しい `ShippableOrderPlaced` を生成する
    ///
    /// # Arguments
    ///
    /// * `order_id` - 配送対象の注文ID
    /// * `shipping_address` - 配送先住所
    /// * `shipment_lines` - 配送対象の明細リスト
    /// * `pdf` - 配送ラベル等の PDF 添付ファイル
    #[must_use]
    pub const fn new(
        order_id: OrderId,
        shipping_address: Address,
        shipment_lines: Vec<ShippableOrderLine>,
        pdf: PdfAttachment,
    ) -> Self {
        Self {
            order_id,
            shipping_address,
            shipment_lines,
            pdf,
        }
    }

    /// 注文IDへの参照を返す
    #[must_use]
    pub const fn order_id(&self) -> &OrderId {
        &self.order_id
    }

    /// 配送先住所への参照を返す
    #[must_use]
    pub const fn shipping_address(&self) -> &Address {
        &self.shipping_address
    }

    /// 配送明細リストへの参照を返す
    #[must_use]
    pub fn shipment_lines(&self) -> &[ShippableOrderLine] {
        &self.shipment_lines
    }

    /// PDF 添付ファイルへの参照を返す
    #[must_use]
    pub const fn pdf(&self) -> &PdfAttachment {
        &self.pdf
    }
}

// =============================================================================
// BillableOrderPlaced
// =============================================================================

/// 請求可能注文確定イベント
///
/// 請求可能な注文が確定したことを表すイベント型。
/// 請求に必要な情報（注文ID、請求先住所、請求金額）を保持する。
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::BillableOrderPlaced;
/// use order_taking_sample::simple_types::{OrderId, BillingAmount};
/// use order_taking_sample::compound_types::Address;
/// use rust_decimal::Decimal;
///
/// let order_id = OrderId::create("OrderId", "order-001").unwrap();
/// let address = Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap();
/// let amount = BillingAmount::create(Decimal::from(1000)).unwrap();
///
/// let event = BillableOrderPlaced::new(order_id, address, amount);
/// assert_eq!(event.amount_to_bill().value(), Decimal::from(1000));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BillableOrderPlaced {
    order_id: OrderId,
    billing_address: Address,
    amount_to_bill: BillingAmount,
}

impl BillableOrderPlaced {
    /// 新しい `BillableOrderPlaced` を生成する
    ///
    /// # Arguments
    ///
    /// * `order_id` - 請求対象の注文ID
    /// * `billing_address` - 請求先住所
    /// * `amount_to_bill` - 請求金額
    #[must_use]
    pub const fn new(
        order_id: OrderId,
        billing_address: Address,
        amount_to_bill: BillingAmount,
    ) -> Self {
        Self {
            order_id,
            billing_address,
            amount_to_bill,
        }
    }

    /// 注文IDへの参照を返す
    #[must_use]
    pub const fn order_id(&self) -> &OrderId {
        &self.order_id
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
}

// =============================================================================
// PlaceOrderEvent
// =============================================================================

/// `PlaceOrder` ワークフローの出力イベント
///
/// 配送イベント、請求イベント、確認メール送信イベントのいずれかを表す直和型。
/// ワークフロー完了時に `Vec<PlaceOrderEvent>` として返される。
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{PlaceOrderEvent, OrderAcknowledgmentSent};
/// use order_taking_sample::simple_types::{OrderId, EmailAddress};
///
/// let order_id = OrderId::create("OrderId", "order-001").unwrap();
/// let email = EmailAddress::create("EmailAddress", "john@example.com").unwrap();
/// let ack_event = OrderAcknowledgmentSent::new(order_id, email);
///
/// let event = PlaceOrderEvent::AcknowledgmentSent(ack_event);
/// assert!(event.is_acknowledgment());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaceOrderEvent {
    /// 配送可能な注文が確定したイベント
    ShippableOrderPlaced(ShippableOrderPlaced),

    /// 請求可能な注文が確定したイベント
    BillableOrderPlaced(BillableOrderPlaced),

    /// 確認メールが送信されたイベント
    AcknowledgmentSent(OrderAcknowledgmentSent),
}

impl PlaceOrderEvent {
    /// `ShippableOrderPlaced` バリアントかどうかを返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{PlaceOrderEvent, ShippableOrderPlaced};
    /// use order_taking_sample::simple_types::{OrderId, PdfAttachment};
    /// use order_taking_sample::compound_types::Address;
    ///
    /// let order_id = OrderId::create("OrderId", "order-001").unwrap();
    /// let address = Address::create("123 Main St", "", "", "", "NYC", "10001", "NY", "USA").unwrap();
    /// let pdf = PdfAttachment::new("label.pdf".to_string(), vec![]);
    /// let shippable = ShippableOrderPlaced::new(order_id, address, vec![], pdf);
    /// let event = PlaceOrderEvent::ShippableOrderPlaced(shippable);
    /// assert!(event.is_shippable());
    /// ```
    #[must_use]
    pub const fn is_shippable(&self) -> bool {
        matches!(self, Self::ShippableOrderPlaced(_))
    }

    /// `BillableOrderPlaced` バリアントかどうかを返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{PlaceOrderEvent, BillableOrderPlaced};
    /// use order_taking_sample::simple_types::{OrderId, BillingAmount};
    /// use order_taking_sample::compound_types::Address;
    /// use rust_decimal::Decimal;
    ///
    /// let order_id = OrderId::create("OrderId", "order-001").unwrap();
    /// let address = Address::create("123 Main St", "", "", "", "NYC", "10001", "NY", "USA").unwrap();
    /// let amount = BillingAmount::create(Decimal::from(100)).unwrap();
    /// let billable = BillableOrderPlaced::new(order_id, address, amount);
    /// let event = PlaceOrderEvent::BillableOrderPlaced(billable);
    /// assert!(event.is_billable());
    /// ```
    #[must_use]
    pub const fn is_billable(&self) -> bool {
        matches!(self, Self::BillableOrderPlaced(_))
    }

    /// `AcknowledgmentSent` バリアントかどうかを返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{PlaceOrderEvent, OrderAcknowledgmentSent};
    /// use order_taking_sample::simple_types::{OrderId, EmailAddress};
    ///
    /// let order_id = OrderId::create("OrderId", "order-001").unwrap();
    /// let email = EmailAddress::create("EmailAddress", "test@example.com").unwrap();
    /// let ack = OrderAcknowledgmentSent::new(order_id, email);
    /// let event = PlaceOrderEvent::AcknowledgmentSent(ack);
    /// assert!(event.is_acknowledgment());
    /// ```
    #[must_use]
    pub const fn is_acknowledgment(&self) -> bool {
        matches!(self, Self::AcknowledgmentSent(_))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    mod order_acknowledgment_sent_tests {
        use super::*;

        fn create_order_id() -> OrderId {
            OrderId::create("OrderId", "order-001").unwrap()
        }

        fn create_email() -> EmailAddress {
            EmailAddress::create("EmailAddress", "test@example.com").unwrap()
        }

        #[test]
        fn test_new_and_getters() {
            let order_id = create_order_id();
            let email = create_email();
            let event = OrderAcknowledgmentSent::new(order_id.clone(), email.clone());

            assert_eq!(event.order_id(), &order_id);
            assert_eq!(event.email_address(), &email);
        }

        #[test]
        fn test_clone() {
            let event1 = OrderAcknowledgmentSent::new(create_order_id(), create_email());
            let event2 = event1.clone();
            assert_eq!(event1, event2);
        }
    }

    mod shippable_order_line_tests {
        use super::*;

        #[test]
        fn test_new_and_getters() {
            let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
            let quantity =
                OrderQuantity::create("Quantity", &product_code, Decimal::from(10)).unwrap();
            let line = ShippableOrderLine::new(product_code.clone(), quantity.clone());

            assert_eq!(line.product_code(), &product_code);
            assert_eq!(line.quantity(), &quantity);
        }

        #[test]
        fn test_clone() {
            let product_code = ProductCode::create("ProductCode", "G123").unwrap();
            let quantity =
                OrderQuantity::create("Quantity", &product_code, Decimal::new(25, 1)).unwrap();
            let line1 = ShippableOrderLine::new(product_code, quantity);
            let line2 = line1.clone();
            assert_eq!(line1, line2);
        }
    }

    mod shippable_order_placed_tests {
        use super::*;

        fn create_order_id() -> OrderId {
            OrderId::create("OrderId", "order-001").unwrap()
        }

        fn create_address() -> Address {
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap()
        }

        fn create_shippable_order_line() -> ShippableOrderLine {
            let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
            let quantity =
                OrderQuantity::create("Quantity", &product_code, Decimal::from(5)).unwrap();
            ShippableOrderLine::new(product_code, quantity)
        }

        fn create_pdf() -> PdfAttachment {
            PdfAttachment::new("label.pdf".to_string(), vec![1, 2, 3])
        }

        #[test]
        fn test_new_and_getters() {
            let order_id = create_order_id();
            let address = create_address();
            let lines = vec![create_shippable_order_line()];
            let pdf = create_pdf();

            let event = ShippableOrderPlaced::new(
                order_id.clone(),
                address.clone(),
                lines.clone(),
                pdf.clone(),
            );

            assert_eq!(event.order_id(), &order_id);
            assert_eq!(event.shipping_address(), &address);
            assert_eq!(event.shipment_lines().len(), 1);
            assert_eq!(event.pdf(), &pdf);
        }

        #[test]
        fn test_clone() {
            let event1 = ShippableOrderPlaced::new(
                create_order_id(),
                create_address(),
                vec![create_shippable_order_line()],
                create_pdf(),
            );
            let event2 = event1.clone();
            assert_eq!(event1, event2);
        }
    }

    mod billable_order_placed_tests {
        use super::*;

        fn create_order_id() -> OrderId {
            OrderId::create("OrderId", "order-001").unwrap()
        }

        fn create_address() -> Address {
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap()
        }

        fn create_amount() -> BillingAmount {
            BillingAmount::create(Decimal::from(1000)).unwrap()
        }

        #[test]
        fn test_new_and_getters() {
            let order_id = create_order_id();
            let address = create_address();
            let amount = create_amount();

            let event = BillableOrderPlaced::new(order_id.clone(), address.clone(), amount.clone());

            assert_eq!(event.order_id(), &order_id);
            assert_eq!(event.billing_address(), &address);
            assert_eq!(event.amount_to_bill(), &amount);
        }

        #[test]
        fn test_clone() {
            let event1 =
                BillableOrderPlaced::new(create_order_id(), create_address(), create_amount());
            let event2 = event1.clone();
            assert_eq!(event1, event2);
        }
    }

    mod place_order_event_tests {
        use super::*;

        fn create_order_id() -> OrderId {
            OrderId::create("OrderId", "order-001").unwrap()
        }

        fn create_address() -> Address {
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap()
        }

        fn create_email() -> EmailAddress {
            EmailAddress::create("EmailAddress", "test@example.com").unwrap()
        }

        fn create_shippable_event() -> ShippableOrderPlaced {
            let pdf = PdfAttachment::new("label.pdf".to_string(), vec![]);
            ShippableOrderPlaced::new(create_order_id(), create_address(), vec![], pdf)
        }

        fn create_billable_event() -> BillableOrderPlaced {
            let amount = BillingAmount::create(Decimal::from(1000)).unwrap();
            BillableOrderPlaced::new(create_order_id(), create_address(), amount)
        }

        fn create_acknowledgment_event() -> OrderAcknowledgmentSent {
            OrderAcknowledgmentSent::new(create_order_id(), create_email())
        }

        #[test]
        fn test_shippable_variant() {
            let event = PlaceOrderEvent::ShippableOrderPlaced(create_shippable_event());
            assert!(event.is_shippable());
            assert!(!event.is_billable());
            assert!(!event.is_acknowledgment());
        }

        #[test]
        fn test_billable_variant() {
            let event = PlaceOrderEvent::BillableOrderPlaced(create_billable_event());
            assert!(!event.is_shippable());
            assert!(event.is_billable());
            assert!(!event.is_acknowledgment());
        }

        #[test]
        fn test_acknowledgment_variant() {
            let event = PlaceOrderEvent::AcknowledgmentSent(create_acknowledgment_event());
            assert!(!event.is_shippable());
            assert!(!event.is_billable());
            assert!(event.is_acknowledgment());
        }

        #[test]
        fn test_clone() {
            let event1 = PlaceOrderEvent::AcknowledgmentSent(create_acknowledgment_event());
            let event2 = event1.clone();
            assert_eq!(event1, event2);
        }
    }
}
