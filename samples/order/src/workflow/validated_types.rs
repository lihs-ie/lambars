//! 検証済み型とバリデーション関連型
//!
//! バリデーション済みのデータを表す型を定義する。
//! Phase 1 と Phase 2 の型を使用し、型安全な検証済みデータを保持する。
//!
//! # 型一覧
//!
//! - [`AddressValidationError`] - 住所検証エラー
//! - [`CheckedAddress`] - 外部サービスで検証済みの住所
//! - [`PricingMethod`] - 価格計算方法
//! - [`ValidatedOrderLine`] - 検証済み注文明細
//! - [`ValidatedOrder`] - 検証済み注文

use crate::compound_types::{Address, CustomerInfo};
use crate::simple_types::{OrderId, OrderLineId, OrderQuantity, ProductCode, PromotionCode};
use crate::workflow::unvalidated_types::UnvalidatedAddress;
use functional_rusty_derive::Lenses;
use thiserror::Error;

// =============================================================================
// AddressValidationError
// =============================================================================

/// 住所検証エラー
///
/// 住所検証サービスからのエラーを表す列挙型。
/// 形式エラーまたは住所が見つからないエラーのいずれか。
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::AddressValidationError;
///
/// let error = AddressValidationError::InvalidFormat;
/// assert!(error.is_invalid_format());
///
/// let error = AddressValidationError::AddressNotFound;
/// assert!(error.is_address_not_found());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
pub enum AddressValidationError {
    /// 住所の形式が無効
    #[error("Invalid address format")]
    InvalidFormat,

    /// 住所が見つからない
    #[error("Address not found")]
    AddressNotFound,
}

impl AddressValidationError {
    /// `InvalidFormat` バリアントかどうかを返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::AddressValidationError;
    ///
    /// let error = AddressValidationError::InvalidFormat;
    /// assert!(error.is_invalid_format());
    /// ```
    #[must_use]
    pub const fn is_invalid_format(&self) -> bool {
        matches!(self, Self::InvalidFormat)
    }

    /// `AddressNotFound` バリアントかどうかを返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::AddressValidationError;
    ///
    /// let error = AddressValidationError::AddressNotFound;
    /// assert!(error.is_address_not_found());
    /// ```
    #[must_use]
    pub const fn is_address_not_found(&self) -> bool {
        matches!(self, Self::AddressNotFound)
    }
}

// =============================================================================
// CheckedAddress
// =============================================================================

/// 外部サービスで検証済みの住所
///
/// [`UnvalidatedAddress`] をラップし、住所が検証済みであることを型レベルで保証する。
/// この型は外部サービスが住所を検証した後にのみ生成される。
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{CheckedAddress, UnvalidatedAddress};
///
/// let unvalidated = UnvalidatedAddress::new(
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
/// // 外部サービスで検証後にのみ CheckedAddress を生成
/// let checked = CheckedAddress::new(unvalidated.clone());
/// assert_eq!(checked.value().city(), "New York");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedAddress(UnvalidatedAddress);

impl CheckedAddress {
    /// 検証済み住所として `CheckedAddress` を生成する
    ///
    /// この関数は外部サービスが住所を検証した後にのみ呼び出すべき。
    ///
    /// # Arguments
    ///
    /// * `address` - 検証済みの住所データ
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{CheckedAddress, UnvalidatedAddress};
    ///
    /// let unvalidated = UnvalidatedAddress::new(
    ///     "456 Oak Ave".to_string(),
    ///     "".to_string(),
    ///     "".to_string(),
    ///     "".to_string(),
    ///     "Boston".to_string(),
    ///     "02101".to_string(),
    ///     "MA".to_string(),
    ///     "USA".to_string(),
    /// );
    /// let checked = CheckedAddress::new(unvalidated);
    /// ```
    #[must_use]
    pub const fn new(address: UnvalidatedAddress) -> Self {
        Self(address)
    }

    /// 内部の `UnvalidatedAddress` への参照を返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{CheckedAddress, UnvalidatedAddress};
    ///
    /// let unvalidated = UnvalidatedAddress::new(
    ///     "789 Pine Rd".to_string(),
    ///     "".to_string(),
    ///     "".to_string(),
    ///     "".to_string(),
    ///     "Chicago".to_string(),
    ///     "60601".to_string(),
    ///     "IL".to_string(),
    ///     "USA".to_string(),
    /// );
    /// let checked = CheckedAddress::new(unvalidated);
    /// assert_eq!(checked.value().state(), "IL");
    /// ```
    #[must_use]
    pub const fn value(&self) -> &UnvalidatedAddress {
        &self.0
    }

    /// 内部の `UnvalidatedAddress` を消費して返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::{CheckedAddress, UnvalidatedAddress};
    ///
    /// let unvalidated = UnvalidatedAddress::new(
    ///     "321 Elm St".to_string(),
    ///     "".to_string(),
    ///     "".to_string(),
    ///     "".to_string(),
    ///     "Seattle".to_string(),
    ///     "98101".to_string(),
    ///     "WA".to_string(),
    ///     "USA".to_string(),
    /// );
    /// let checked = CheckedAddress::new(unvalidated);
    /// let inner = checked.into_inner();
    /// assert_eq!(inner.city(), "Seattle");
    /// ```
    #[must_use]
    pub fn into_inner(self) -> UnvalidatedAddress {
        self.0
    }
}

// =============================================================================
// PricingMethod
// =============================================================================

/// 価格計算方法
///
/// 標準価格またはプロモーション価格のいずれかを表す。
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::PricingMethod;
/// use order_taking_sample::simple_types::PromotionCode;
///
/// // 標準価格
/// let standard = PricingMethod::Standard;
/// assert!(standard.is_standard());
/// assert!(standard.promotion_code().is_none());
///
/// // プロモーション価格
/// let promo_code = PromotionCode::new("SUMMER2024".to_string());
/// let promotion = PricingMethod::Promotion(promo_code.clone());
/// assert!(promotion.is_promotion());
/// assert_eq!(promotion.promotion_code(), Some(&promo_code));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PricingMethod {
    /// 標準価格を適用
    Standard,

    /// プロモーション価格を適用
    Promotion(PromotionCode),
}

impl PricingMethod {
    /// `Standard` バリアントかどうかを返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::PricingMethod;
    ///
    /// let method = PricingMethod::Standard;
    /// assert!(method.is_standard());
    /// ```
    #[must_use]
    pub const fn is_standard(&self) -> bool {
        matches!(self, Self::Standard)
    }

    /// `Promotion` バリアントかどうかを返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::PricingMethod;
    /// use order_taking_sample::simple_types::PromotionCode;
    ///
    /// let promo_code = PromotionCode::new("SALE".to_string());
    /// let method = PricingMethod::Promotion(promo_code);
    /// assert!(method.is_promotion());
    /// ```
    #[must_use]
    pub const fn is_promotion(&self) -> bool {
        matches!(self, Self::Promotion(_))
    }

    /// プロモーションコードを返す
    ///
    /// `Standard` の場合は `None` を返す。
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::PricingMethod;
    /// use order_taking_sample::simple_types::PromotionCode;
    ///
    /// let standard = PricingMethod::Standard;
    /// assert!(standard.promotion_code().is_none());
    ///
    /// let promo_code = PromotionCode::new("WINTER".to_string());
    /// let promotion = PricingMethod::Promotion(promo_code);
    /// assert!(promotion.promotion_code().is_some());
    /// ```
    #[must_use]
    pub const fn promotion_code(&self) -> Option<&PromotionCode> {
        match self {
            Self::Standard => None,
            Self::Promotion(code) => Some(code),
        }
    }
}

// =============================================================================
// ValidatedOrderLine
// =============================================================================

/// 検証済み注文明細
///
/// バリデーション済みの注文明細を表す型。
/// 全てのフィールドが検証済みの型（Phase 1 で定義）を使用する。
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::ValidatedOrderLine;
/// use order_taking_sample::simple_types::{OrderLineId, ProductCode, OrderQuantity};
///
/// use rust_decimal::Decimal;
///
/// let order_line_id = OrderLineId::create("OrderLineId", "line-001").unwrap();
/// let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
/// let quantity = OrderQuantity::create("Quantity", &product_code, Decimal::from(10)).unwrap();
///
/// let validated_line = ValidatedOrderLine::new(order_line_id, product_code, quantity);
/// assert_eq!(validated_line.order_line_id().value(), "line-001");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Lenses)]
pub struct ValidatedOrderLine {
    order_line_id: OrderLineId,
    product_code: ProductCode,
    quantity: OrderQuantity,
}

impl ValidatedOrderLine {
    /// 新しい `ValidatedOrderLine` を生成する
    ///
    /// # Arguments
    ///
    /// * `order_line_id` - 検証済み注文明細ID
    /// * `product_code` - 検証済み製品コード
    /// * `quantity` - 検証済み数量
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::ValidatedOrderLine;
    /// use order_taking_sample::simple_types::{OrderLineId, ProductCode, OrderQuantity};
    /// use rust_decimal::Decimal;
    ///
    /// let order_line_id = OrderLineId::create("OrderLineId", "line-002").unwrap();
    /// let product_code = ProductCode::create("ProductCode", "G123").unwrap();
    /// let quantity = OrderQuantity::create("Quantity", &product_code, Decimal::new(25, 1)).unwrap();
    ///
    /// let validated_line = ValidatedOrderLine::new(order_line_id, product_code, quantity);
    /// ```
    #[must_use]
    pub const fn new(
        order_line_id: OrderLineId,
        product_code: ProductCode,
        quantity: OrderQuantity,
    ) -> Self {
        Self {
            order_line_id,
            product_code,
            quantity,
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
}

// =============================================================================
// ValidatedOrder
// =============================================================================

/// 検証済み注文
///
/// バリデーション済みの注文を表す型。
/// 全てのフィールドが検証済みの型を使用する。
/// `UnvalidatedOrder` からの変換後の状態を表す。
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{ValidatedOrder, ValidatedOrderLine, PricingMethod};
/// use order_taking_sample::simple_types::{OrderId, OrderLineId, ProductCode, OrderQuantity};
/// use order_taking_sample::compound_types::{CustomerInfo, Address};
///
/// let order_id = OrderId::create("OrderId", "order-001").unwrap();
/// let customer_info = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
/// let address = Address::create(
///     "123 Main St", "", "", "", "New York", "10001", "NY", "USA"
/// ).unwrap();
///
/// let validated_order = ValidatedOrder::new(
///     order_id,
///     customer_info,
///     address.clone(),
///     address,
///     vec![],
///     PricingMethod::Standard,
/// );
/// assert_eq!(validated_order.order_id().value(), "order-001");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Lenses)]
pub struct ValidatedOrder {
    order_id: OrderId,
    customer_info: CustomerInfo,
    shipping_address: Address,
    billing_address: Address,
    lines: Vec<ValidatedOrderLine>,
    pricing_method: PricingMethod,
}

impl ValidatedOrder {
    /// 新しい `ValidatedOrder` を生成する
    ///
    /// # Arguments
    ///
    /// * `order_id` - 検証済み注文ID
    /// * `customer_info` - 検証済み顧客情報
    /// * `shipping_address` - 検証済み配送先住所
    /// * `billing_address` - 検証済み請求先住所
    /// * `lines` - 検証済み注文明細リスト
    /// * `pricing_method` - 価格計算方法
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        order_id: OrderId,
        customer_info: CustomerInfo,
        shipping_address: Address,
        billing_address: Address,
        lines: Vec<ValidatedOrderLine>,
        pricing_method: PricingMethod,
    ) -> Self {
        Self {
            order_id,
            customer_info,
            shipping_address,
            billing_address,
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

    /// 注文明細リストへの参照を返す
    #[must_use]
    pub fn lines(&self) -> &[ValidatedOrderLine] {
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

    mod address_validation_error_tests {
        use super::*;

        #[test]
        fn test_invalid_format() {
            let error = AddressValidationError::InvalidFormat;
            assert!(error.is_invalid_format());
            assert!(!error.is_address_not_found());
        }

        #[test]
        fn test_address_not_found() {
            let error = AddressValidationError::AddressNotFound;
            assert!(!error.is_invalid_format());
            assert!(error.is_address_not_found());
        }

        #[test]
        fn test_display() {
            let error = AddressValidationError::InvalidFormat;
            assert!(error.to_string().contains("Invalid"));

            let error = AddressValidationError::AddressNotFound;
            assert!(error.to_string().contains("not found"));
        }

        #[test]
        fn test_copy() {
            let error1 = AddressValidationError::InvalidFormat;
            let error2 = error1; // Copy
            assert_eq!(error1, error2);
        }
    }

    mod checked_address_tests {
        use super::*;

        fn create_unvalidated_address() -> UnvalidatedAddress {
            UnvalidatedAddress::new(
                "123 Main St".to_string(),
                "Apt 4".to_string(),
                "".to_string(),
                "".to_string(),
                "New York".to_string(),
                "10001".to_string(),
                "NY".to_string(),
                "USA".to_string(),
            )
        }

        #[test]
        fn test_new_and_value() {
            let unvalidated = create_unvalidated_address();
            let checked = CheckedAddress::new(unvalidated.clone());
            assert_eq!(checked.value(), &unvalidated);
        }

        #[test]
        fn test_into_inner() {
            let unvalidated = create_unvalidated_address();
            let checked = CheckedAddress::new(unvalidated.clone());
            let inner = checked.into_inner();
            assert_eq!(inner, unvalidated);
        }

        #[test]
        fn test_clone() {
            let checked1 = CheckedAddress::new(create_unvalidated_address());
            let checked2 = checked1.clone();
            assert_eq!(checked1, checked2);
        }
    }

    mod pricing_method_tests {
        use super::*;

        #[test]
        fn test_standard() {
            let method = PricingMethod::Standard;
            assert!(method.is_standard());
            assert!(!method.is_promotion());
            assert!(method.promotion_code().is_none());
        }

        #[test]
        fn test_promotion() {
            let promo_code = PromotionCode::new("SUMMER2024".to_string());
            let method = PricingMethod::Promotion(promo_code.clone());
            assert!(!method.is_standard());
            assert!(method.is_promotion());
            assert_eq!(method.promotion_code(), Some(&promo_code));
        }

        #[test]
        fn test_clone() {
            let promo_code = PromotionCode::new("WINTER".to_string());
            let method1 = PricingMethod::Promotion(promo_code);
            let method2 = method1.clone();
            assert_eq!(method1, method2);
        }
    }

    mod validated_order_line_tests {
        use super::*;
        use rust_decimal::Decimal;

        #[test]
        fn test_new_and_getters() {
            let order_line_id = OrderLineId::create("OrderLineId", "line-001").unwrap();
            let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
            let quantity =
                OrderQuantity::create("Quantity", &product_code, Decimal::from(10)).unwrap();

            let validated_line = ValidatedOrderLine::new(
                order_line_id.clone(),
                product_code.clone(),
                quantity.clone(),
            );

            assert_eq!(validated_line.order_line_id(), &order_line_id);
            assert_eq!(validated_line.product_code(), &product_code);
            assert_eq!(validated_line.quantity(), &quantity);
        }

        #[test]
        fn test_clone() {
            let order_line_id = OrderLineId::create("OrderLineId", "line-002").unwrap();
            let product_code = ProductCode::create("ProductCode", "G123").unwrap();
            let quantity =
                OrderQuantity::create("Quantity", &product_code, Decimal::new(25, 1)).unwrap();

            let validated_line1 = ValidatedOrderLine::new(order_line_id, product_code, quantity);
            let validated_line2 = validated_line1.clone();
            assert_eq!(validated_line1, validated_line2);
        }
    }

    mod validated_order_tests {
        use super::*;
        use rust_decimal::Decimal;

        fn create_order_id() -> OrderId {
            OrderId::create("OrderId", "order-001").unwrap()
        }

        fn create_customer_info() -> CustomerInfo {
            CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap()
        }

        fn create_address() -> Address {
            Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "USA").unwrap()
        }

        fn create_validated_order_line() -> ValidatedOrderLine {
            let order_line_id = OrderLineId::create("OrderLineId", "line-001").unwrap();
            let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
            let quantity =
                OrderQuantity::create("Quantity", &product_code, Decimal::from(10)).unwrap();
            ValidatedOrderLine::new(order_line_id, product_code, quantity)
        }

        #[test]
        fn test_new_and_getters() {
            let order_id = create_order_id();
            let customer_info = create_customer_info();
            let shipping_address = create_address();
            let billing_address = create_address();
            let lines = vec![create_validated_order_line()];
            let pricing_method = PricingMethod::Standard;

            let validated_order = ValidatedOrder::new(
                order_id.clone(),
                customer_info.clone(),
                shipping_address.clone(),
                billing_address.clone(),
                lines.clone(),
                pricing_method.clone(),
            );

            assert_eq!(validated_order.order_id(), &order_id);
            assert_eq!(validated_order.customer_info(), &customer_info);
            assert_eq!(validated_order.shipping_address(), &shipping_address);
            assert_eq!(validated_order.billing_address(), &billing_address);
            assert_eq!(validated_order.lines().len(), 1);
            assert_eq!(validated_order.pricing_method(), &pricing_method);
        }

        #[test]
        fn test_with_promotion() {
            let promo_code = PromotionCode::new("PROMO2024".to_string());
            let pricing_method = PricingMethod::Promotion(promo_code);

            let validated_order = ValidatedOrder::new(
                create_order_id(),
                create_customer_info(),
                create_address(),
                create_address(),
                vec![],
                pricing_method.clone(),
            );

            assert!(validated_order.pricing_method().is_promotion());
        }

        #[test]
        fn test_clone() {
            let validated_order1 = ValidatedOrder::new(
                create_order_id(),
                create_customer_info(),
                create_address(),
                create_address(),
                vec![create_validated_order_line()],
                PricingMethod::Standard,
            );
            let validated_order2 = validated_order1.clone();
            assert_eq!(validated_order1, validated_order2);
        }
    }
}
