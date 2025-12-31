//! 配送関連型
//!
//! 配送方法と配送情報を表す型を定義する。
//!
//! # 型一覧
//!
//! - [`ShippingMethod`] - 配送方法
//! - [`ShippingInfo`] - 配送情報
//! - [`PricedOrderWithShippingMethod`] - 配送情報付き価格計算済み注文

use crate::simple_types::Price;
use crate::workflow::priced_types::PricedOrder;
use functional_rusty_derive::Lenses;

// =============================================================================
// ShippingMethod
// =============================================================================

/// 配送方法
///
/// 郵便または各種宅配サービスのいずれかを表す。
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::ShippingMethod;
///
/// let postal = ShippingMethod::PostalService;
/// assert!(postal.is_postal_service());
///
/// let fedex24 = ShippingMethod::Fedex24;
/// assert!(fedex24.is_fedex24());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShippingMethod {
    /// 郵便サービス
    PostalService,

    /// `FedEx` 24時間配送
    Fedex24,

    /// `FedEx` 48時間配送
    Fedex48,

    /// UPS 48時間配送
    Ups48,
}

impl ShippingMethod {
    /// `PostalService` バリアントかどうかを返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::ShippingMethod;
    ///
    /// let method = ShippingMethod::PostalService;
    /// assert!(method.is_postal_service());
    /// ```
    #[must_use]
    pub const fn is_postal_service(&self) -> bool {
        matches!(self, Self::PostalService)
    }

    /// `Fedex24` バリアントかどうかを返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::ShippingMethod;
    ///
    /// let method = ShippingMethod::Fedex24;
    /// assert!(method.is_fedex24());
    /// ```
    #[must_use]
    pub const fn is_fedex24(&self) -> bool {
        matches!(self, Self::Fedex24)
    }

    /// `Fedex48` バリアントかどうかを返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::ShippingMethod;
    ///
    /// let method = ShippingMethod::Fedex48;
    /// assert!(method.is_fedex48());
    /// ```
    #[must_use]
    pub const fn is_fedex48(&self) -> bool {
        matches!(self, Self::Fedex48)
    }

    /// `Ups48` バリアントかどうかを返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::ShippingMethod;
    ///
    /// let method = ShippingMethod::Ups48;
    /// assert!(method.is_ups48());
    /// ```
    #[must_use]
    pub const fn is_ups48(&self) -> bool {
        matches!(self, Self::Ups48)
    }
}

// =============================================================================
// ShippingInfo
// =============================================================================

/// 配送情報
///
/// 配送方法と配送コストを保持する。
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{ShippingInfo, ShippingMethod};
/// use order_taking_sample::simple_types::Price;
/// use rust_decimal::Decimal;
///
/// let shipping_cost = Price::create(Decimal::from(15)).unwrap();
/// let shipping_info = ShippingInfo::new(ShippingMethod::Fedex24, shipping_cost.clone());
///
/// assert!(shipping_info.shipping_method().is_fedex24());
/// assert_eq!(shipping_info.shipping_cost(), &shipping_cost);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShippingInfo {
    shipping_method: ShippingMethod,
    shipping_cost: Price,
}

impl ShippingInfo {
    /// 新しい `ShippingInfo` を生成する
    ///
    /// # Arguments
    ///
    /// * `shipping_method` - 配送方法
    /// * `shipping_cost` - 配送コスト
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{ShippingInfo, ShippingMethod};
    /// use order_taking_sample::simple_types::Price;
    /// use rust_decimal::Decimal;
    ///
    /// let shipping_cost = Price::create(Decimal::from(10)).unwrap();
    /// let shipping_info = ShippingInfo::new(ShippingMethod::PostalService, shipping_cost);
    /// ```
    #[must_use]
    pub const fn new(shipping_method: ShippingMethod, shipping_cost: Price) -> Self {
        Self {
            shipping_method,
            shipping_cost,
        }
    }

    /// 配送方法を返す
    ///
    /// `ShippingMethod` は `Copy` なのでコピーを返す。
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{ShippingInfo, ShippingMethod};
    /// use order_taking_sample::simple_types::Price;
    /// use rust_decimal::Decimal;
    ///
    /// let shipping_cost = Price::create(Decimal::from(20)).unwrap();
    /// let shipping_info = ShippingInfo::new(ShippingMethod::Ups48, shipping_cost);
    /// assert!(shipping_info.shipping_method().is_ups48());
    /// ```
    #[must_use]
    pub const fn shipping_method(&self) -> ShippingMethod {
        self.shipping_method
    }

    /// 配送コストへの参照を返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{ShippingInfo, ShippingMethod};
    /// use order_taking_sample::simple_types::Price;
    /// use rust_decimal::Decimal;
    ///
    /// let shipping_cost = Price::create(Decimal::from(25)).unwrap();
    /// let shipping_info = ShippingInfo::new(ShippingMethod::Fedex48, shipping_cost);
    /// assert_eq!(shipping_info.shipping_cost().value(), Decimal::from(25));
    /// ```
    #[must_use]
    pub const fn shipping_cost(&self) -> &Price {
        &self.shipping_cost
    }
}

// =============================================================================
// PricedOrderWithShippingMethod
// =============================================================================

/// 配送情報付き価格計算済み注文
///
/// [`PricedOrder`] に配送情報を追加した型。
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{
///     PricedOrderWithShippingMethod, PricedOrder, ShippingInfo, ShippingMethod, PricingMethod,
/// };
/// use order_taking_sample::simple_types::{OrderId, Price, BillingAmount};
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
///
/// let shipping_cost = Price::create(Decimal::from(15)).unwrap();
/// let shipping_info = ShippingInfo::new(ShippingMethod::Fedex24, shipping_cost);
///
/// let order_with_shipping = PricedOrderWithShippingMethod::new(shipping_info.clone(), priced_order.clone());
/// assert_eq!(order_with_shipping.shipping_info(), &shipping_info);
/// assert_eq!(order_with_shipping.priced_order(), &priced_order);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Lenses)]
pub struct PricedOrderWithShippingMethod {
    shipping_info: ShippingInfo,
    priced_order: PricedOrder,
}

impl PricedOrderWithShippingMethod {
    /// 新しい `PricedOrderWithShippingMethod` を生成する
    ///
    /// # Arguments
    ///
    /// * `shipping_info` - 配送情報
    /// * `priced_order` - 価格計算済み注文
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{
    ///     PricedOrderWithShippingMethod, PricedOrder, ShippingInfo, ShippingMethod, PricingMethod,
    /// };
    /// use order_taking_sample::simple_types::{OrderId, Price, BillingAmount};
    /// use order_taking_sample::compound_types::{CustomerInfo, Address};
    /// use rust_decimal::Decimal;
    ///
    /// let priced_order = PricedOrder::new(
    ///     OrderId::create("OrderId", "order-002").unwrap(),
    ///     CustomerInfo::create("Jane", "Smith", "jane@example.com", "VIP").unwrap(),
    ///     Address::create("456 Oak Ave", "", "", "", "Boston", "02101", "MA", "USA").unwrap(),
    ///     Address::create("456 Oak Ave", "", "", "", "Boston", "02101", "MA", "USA").unwrap(),
    ///     BillingAmount::create(Decimal::from(500)).unwrap(),
    ///     vec![],
    ///     PricingMethod::Standard,
    /// );
    ///
    /// let shipping_info = ShippingInfo::new(
    ///     ShippingMethod::PostalService,
    ///     Price::create(Decimal::from(5)).unwrap(),
    /// );
    ///
    /// let order_with_shipping = PricedOrderWithShippingMethod::new(shipping_info, priced_order);
    /// ```
    #[must_use]
    pub const fn new(shipping_info: ShippingInfo, priced_order: PricedOrder) -> Self {
        Self {
            shipping_info,
            priced_order,
        }
    }

    /// 配送情報への参照を返す
    #[must_use]
    pub const fn shipping_info(&self) -> &ShippingInfo {
        &self.shipping_info
    }

    /// 価格計算済み注文への参照を返す
    #[must_use]
    pub const fn priced_order(&self) -> &PricedOrder {
        &self.priced_order
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compound_types::{Address, CustomerInfo};
    use crate::simple_types::{BillingAmount, OrderId};
    use crate::workflow::validated_types::PricingMethod;
    use rust_decimal::Decimal;

    mod shipping_method_tests {
        use super::*;

        #[test]
        fn test_postal_service() {
            let method = ShippingMethod::PostalService;
            assert!(method.is_postal_service());
            assert!(!method.is_fedex24());
            assert!(!method.is_fedex48());
            assert!(!method.is_ups48());
        }

        #[test]
        fn test_fedex24() {
            let method = ShippingMethod::Fedex24;
            assert!(!method.is_postal_service());
            assert!(method.is_fedex24());
            assert!(!method.is_fedex48());
            assert!(!method.is_ups48());
        }

        #[test]
        fn test_fedex48() {
            let method = ShippingMethod::Fedex48;
            assert!(!method.is_postal_service());
            assert!(!method.is_fedex24());
            assert!(method.is_fedex48());
            assert!(!method.is_ups48());
        }

        #[test]
        fn test_ups48() {
            let method = ShippingMethod::Ups48;
            assert!(!method.is_postal_service());
            assert!(!method.is_fedex24());
            assert!(!method.is_fedex48());
            assert!(method.is_ups48());
        }

        #[test]
        fn test_copy() {
            let method1 = ShippingMethod::Fedex24;
            let method2 = method1; // Copy
            assert_eq!(method1, method2);
        }
    }

    mod shipping_info_tests {
        use super::*;

        #[test]
        fn test_new_and_getters() {
            let shipping_cost = Price::create(Decimal::from(15)).unwrap();
            let shipping_info = ShippingInfo::new(ShippingMethod::Fedex24, shipping_cost.clone());

            assert!(shipping_info.shipping_method().is_fedex24());
            assert_eq!(shipping_info.shipping_cost(), &shipping_cost);
        }

        #[test]
        fn test_clone() {
            let shipping_cost = Price::create(Decimal::from(20)).unwrap();
            let shipping_info1 = ShippingInfo::new(ShippingMethod::Ups48, shipping_cost);
            let shipping_info2 = shipping_info1.clone();
            assert_eq!(shipping_info1, shipping_info2);
        }
    }

    mod priced_order_with_shipping_method_tests {
        use super::*;

        fn create_priced_order() -> PricedOrder {
            let order_id = OrderId::create("OrderId", "order-001").unwrap();
            let customer_info =
                CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
            let address =
                Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA")
                    .unwrap();
            let amount_to_bill = BillingAmount::create(Decimal::from(1000)).unwrap();

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

        fn create_shipping_info() -> ShippingInfo {
            let shipping_cost = Price::create(Decimal::from(15)).unwrap();
            ShippingInfo::new(ShippingMethod::Fedex24, shipping_cost)
        }

        #[test]
        fn test_new_and_getters() {
            let priced_order = create_priced_order();
            let shipping_info = create_shipping_info();

            let order_with_shipping =
                PricedOrderWithShippingMethod::new(shipping_info.clone(), priced_order.clone());

            assert_eq!(order_with_shipping.shipping_info(), &shipping_info);
            assert_eq!(order_with_shipping.priced_order(), &priced_order);
        }

        #[test]
        fn test_clone() {
            let order_with_shipping1 =
                PricedOrderWithShippingMethod::new(create_shipping_info(), create_priced_order());
            let order_with_shipping2 = order_with_shipping1.clone();
            assert_eq!(order_with_shipping1, order_with_shipping2);
        }
    }
}
