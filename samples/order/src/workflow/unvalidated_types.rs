//! 未検証の入力型
//!
//! 外部から受け取った未検証のデータを表す型を定義する。
//! 全てのフィールドは String または数値型で、バリデーション前の生データを保持する。
//!
//! # 型一覧
//!
//! - [`UnvalidatedCustomerInfo`] - 未検証の顧客情報
//! - [`UnvalidatedAddress`] - 未検証の住所
//! - [`UnvalidatedOrderLine`] - 未検証の注文明細
//! - [`UnvalidatedOrder`] - 未検証の注文
//!
//! # 設計方針
//!
//! これらの型は意図的にバリデーションロジックを含まない。
//! バリデーションは別のステップで行い、検証済みの型に変換される。

use rust_decimal::Decimal;

// =============================================================================
// UnvalidatedCustomerInfo
// =============================================================================

/// 未検証の顧客情報
///
/// 外部から受け取った未検証の顧客情報を表す型。
/// 全てのフィールドは String 型で、バリデーション前の生データを保持する。
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::UnvalidatedCustomerInfo;
///
/// let customer_info = UnvalidatedCustomerInfo::new(
///     "John".to_string(),
///     "Doe".to_string(),
///     "john@example.com".to_string(),
///     "Normal".to_string(),
/// );
/// assert_eq!(customer_info.first_name(), "John");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnvalidatedCustomerInfo {
    first_name: String,
    last_name: String,
    email_address: String,
    vip_status: String,
}

impl UnvalidatedCustomerInfo {
    /// 新しい `UnvalidatedCustomerInfo` を生成する
    ///
    /// バリデーションは行わない。バリデーションは後続のステップで行う。
    ///
    /// # Arguments
    ///
    /// * `first_name` - 名
    /// * `last_name` - 姓
    /// * `email_address` - メールアドレス
    /// * `vip_status` - VIP ステータス文字列（"Normal", "VIP" 等）
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::UnvalidatedCustomerInfo;
    ///
    /// let customer_info = UnvalidatedCustomerInfo::new(
    ///     "Jane".to_string(),
    ///     "Smith".to_string(),
    ///     "jane@example.com".to_string(),
    ///     "VIP".to_string(),
    /// );
    /// ```
    #[must_use]
    pub const fn new(
        first_name: String,
        last_name: String,
        email_address: String,
        vip_status: String,
    ) -> Self {
        Self {
            first_name,
            last_name,
            email_address,
            vip_status,
        }
    }

    /// 名への参照を返す
    #[must_use]
    pub fn first_name(&self) -> &str {
        &self.first_name
    }

    /// 姓への参照を返す
    #[must_use]
    pub fn last_name(&self) -> &str {
        &self.last_name
    }

    /// メールアドレスへの参照を返す
    #[must_use]
    pub fn email_address(&self) -> &str {
        &self.email_address
    }

    /// VIP ステータスへの参照を返す
    #[must_use]
    pub fn vip_status(&self) -> &str {
        &self.vip_status
    }
}

// =============================================================================
// UnvalidatedAddress
// =============================================================================

/// 未検証の住所
///
/// 外部から受け取った未検証の住所を表す型。
/// 全てのフィールドは String 型で、バリデーション前の生データを保持する。
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::UnvalidatedAddress;
///
/// let address = UnvalidatedAddress::new(
///     "123 Main St".to_string(),
///     "Apt 4".to_string(),
///     "".to_string(),
///     "".to_string(),
///     "New York".to_string(),
///     "10001".to_string(),
///     "NY".to_string(),
///     "USA".to_string(),
/// );
/// assert_eq!(address.city(), "New York");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnvalidatedAddress {
    address_line1: String,
    address_line2: String,
    address_line3: String,
    address_line4: String,
    city: String,
    zip_code: String,
    state: String,
    country: String,
}

impl UnvalidatedAddress {
    /// 新しい `UnvalidatedAddress` を生成する
    ///
    /// バリデーションは行わない。バリデーションは後続のステップで行う。
    ///
    /// # Arguments
    ///
    /// * `address_line1` - 住所行1
    /// * `address_line2` - 住所行2（空文字列の場合もあり）
    /// * `address_line3` - 住所行3（空文字列の場合もあり）
    /// * `address_line4` - 住所行4（空文字列の場合もあり）
    /// * `city` - 市
    /// * `zip_code` - 郵便番号
    /// * `state` - 州コード
    /// * `country` - 国名
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        address_line1: String,
        address_line2: String,
        address_line3: String,
        address_line4: String,
        city: String,
        zip_code: String,
        state: String,
        country: String,
    ) -> Self {
        Self {
            address_line1,
            address_line2,
            address_line3,
            address_line4,
            city,
            zip_code,
            state,
            country,
        }
    }

    /// 住所行1への参照を返す
    #[must_use]
    pub fn address_line1(&self) -> &str {
        &self.address_line1
    }

    /// 住所行2への参照を返す
    #[must_use]
    pub fn address_line2(&self) -> &str {
        &self.address_line2
    }

    /// 住所行3への参照を返す
    #[must_use]
    pub fn address_line3(&self) -> &str {
        &self.address_line3
    }

    /// 住所行4への参照を返す
    #[must_use]
    pub fn address_line4(&self) -> &str {
        &self.address_line4
    }

    /// 市への参照を返す
    #[must_use]
    pub fn city(&self) -> &str {
        &self.city
    }

    /// 郵便番号への参照を返す
    #[must_use]
    pub fn zip_code(&self) -> &str {
        &self.zip_code
    }

    /// 州コードへの参照を返す
    #[must_use]
    pub fn state(&self) -> &str {
        &self.state
    }

    /// 国名への参照を返す
    #[must_use]
    pub fn country(&self) -> &str {
        &self.country
    }
}

// =============================================================================
// UnvalidatedOrderLine
// =============================================================================

/// 未検証の注文明細
///
/// 外部から受け取った未検証の注文明細を表す型。
/// 注文明細ID、製品コード、数量を保持する。
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::UnvalidatedOrderLine;
/// use rust_decimal::Decimal;
/// use std::str::FromStr;
///
/// let order_line = UnvalidatedOrderLine::new(
///     "line-001".to_string(),
///     "W1234".to_string(),
///     Decimal::from_str("10").unwrap(),
/// );
/// assert_eq!(order_line.product_code(), "W1234");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnvalidatedOrderLine {
    order_line_id: String,
    product_code: String,
    quantity: Decimal,
}

impl UnvalidatedOrderLine {
    /// 新しい `UnvalidatedOrderLine` を生成する
    ///
    /// バリデーションは行わない。バリデーションは後続のステップで行う。
    ///
    /// # Arguments
    ///
    /// * `order_line_id` - 注文明細ID
    /// * `product_code` - 製品コード（"W1234" や "G123" 形式）
    /// * `quantity` - 数量（個数または重量）
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::UnvalidatedOrderLine;
    /// use rust_decimal::Decimal;
    ///
    /// let order_line = UnvalidatedOrderLine::new(
    ///     "line-002".to_string(),
    ///     "G123".to_string(),
    ///     Decimal::new(250, 2),  // 2.50
    /// );
    /// ```
    #[must_use]
    pub const fn new(order_line_id: String, product_code: String, quantity: Decimal) -> Self {
        Self {
            order_line_id,
            product_code,
            quantity,
        }
    }

    /// 注文明細IDへの参照を返す
    #[must_use]
    pub fn order_line_id(&self) -> &str {
        &self.order_line_id
    }

    /// 製品コードへの参照を返す
    #[must_use]
    pub fn product_code(&self) -> &str {
        &self.product_code
    }

    /// 数量を返す
    #[must_use]
    pub const fn quantity(&self) -> Decimal {
        self.quantity
    }
}

// =============================================================================
// UnvalidatedOrder
// =============================================================================

/// 未検証の注文
///
/// 外部から受け取った未検証の注文を表す型。
/// `PlaceOrder` ワークフローの入力として使用される。
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{
///     UnvalidatedOrder, UnvalidatedCustomerInfo, UnvalidatedAddress, UnvalidatedOrderLine,
/// };
/// use rust_decimal::Decimal;
///
/// let customer_info = UnvalidatedCustomerInfo::new(
///     "John".to_string(),
///     "Doe".to_string(),
///     "john@example.com".to_string(),
///     "Normal".to_string(),
/// );
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
/// let lines = vec![
///     UnvalidatedOrderLine::new("line-001".to_string(), "W1234".to_string(), Decimal::from(10)),
/// ];
///
/// let order = UnvalidatedOrder::new(
///     "order-001".to_string(),
///     customer_info,
///     address.clone(),
///     address,
///     lines,
///     "".to_string(),
/// );
/// assert_eq!(order.order_id(), "order-001");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnvalidatedOrder {
    order_id: String,
    customer_info: UnvalidatedCustomerInfo,
    shipping_address: UnvalidatedAddress,
    billing_address: UnvalidatedAddress,
    lines: Vec<UnvalidatedOrderLine>,
    promotion_code: String,
}

impl UnvalidatedOrder {
    /// 新しい `UnvalidatedOrder` を生成する
    ///
    /// バリデーションは行わない。バリデーションは後続のステップで行う。
    ///
    /// # Arguments
    ///
    /// * `order_id` - 注文ID
    /// * `customer_info` - 顧客情報
    /// * `shipping_address` - 配送先住所
    /// * `billing_address` - 請求先住所
    /// * `lines` - 注文明細リスト
    /// * `promotion_code` - プロモーションコード（空文字列の場合もあり）
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        order_id: String,
        customer_info: UnvalidatedCustomerInfo,
        shipping_address: UnvalidatedAddress,
        billing_address: UnvalidatedAddress,
        lines: Vec<UnvalidatedOrderLine>,
        promotion_code: String,
    ) -> Self {
        Self {
            order_id,
            customer_info,
            shipping_address,
            billing_address,
            lines,
            promotion_code,
        }
    }

    /// 注文IDへの参照を返す
    #[must_use]
    pub fn order_id(&self) -> &str {
        &self.order_id
    }

    /// 顧客情報への参照を返す
    #[must_use]
    pub const fn customer_info(&self) -> &UnvalidatedCustomerInfo {
        &self.customer_info
    }

    /// 配送先住所への参照を返す
    #[must_use]
    pub const fn shipping_address(&self) -> &UnvalidatedAddress {
        &self.shipping_address
    }

    /// 請求先住所への参照を返す
    #[must_use]
    pub const fn billing_address(&self) -> &UnvalidatedAddress {
        &self.billing_address
    }

    /// 注文明細リストへの参照を返す
    #[must_use]
    pub fn lines(&self) -> &[UnvalidatedOrderLine] {
        &self.lines
    }

    /// プロモーションコードへの参照を返す
    #[must_use]
    pub fn promotion_code(&self) -> &str {
        &self.promotion_code
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    mod unvalidated_customer_info_tests {
        use super::*;

        #[test]
        fn test_new_and_getters() {
            let customer_info = UnvalidatedCustomerInfo::new(
                "John".to_string(),
                "Doe".to_string(),
                "john@example.com".to_string(),
                "VIP".to_string(),
            );

            assert_eq!(customer_info.first_name(), "John");
            assert_eq!(customer_info.last_name(), "Doe");
            assert_eq!(customer_info.email_address(), "john@example.com");
            assert_eq!(customer_info.vip_status(), "VIP");
        }

        #[test]
        fn test_clone() {
            let customer_info1 = UnvalidatedCustomerInfo::new(
                "Jane".to_string(),
                "Smith".to_string(),
                "jane@example.com".to_string(),
                "Normal".to_string(),
            );
            let customer_info2 = customer_info1.clone();
            assert_eq!(customer_info1, customer_info2);
        }
    }

    mod unvalidated_address_tests {
        use super::*;

        #[test]
        fn test_new_and_getters() {
            let address = UnvalidatedAddress::new(
                "123 Main St".to_string(),
                "Apt 4".to_string(),
                "Floor 2".to_string(),
                "Building A".to_string(),
                "New York".to_string(),
                "10001".to_string(),
                "NY".to_string(),
                "USA".to_string(),
            );

            assert_eq!(address.address_line1(), "123 Main St");
            assert_eq!(address.address_line2(), "Apt 4");
            assert_eq!(address.address_line3(), "Floor 2");
            assert_eq!(address.address_line4(), "Building A");
            assert_eq!(address.city(), "New York");
            assert_eq!(address.zip_code(), "10001");
            assert_eq!(address.state(), "NY");
            assert_eq!(address.country(), "USA");
        }

        #[test]
        fn test_with_empty_optional_fields() {
            let address = UnvalidatedAddress::new(
                "456 Oak Ave".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
                "Los Angeles".to_string(),
                "90001".to_string(),
                "CA".to_string(),
                "USA".to_string(),
            );

            assert_eq!(address.address_line2(), "");
            assert_eq!(address.address_line3(), "");
            assert_eq!(address.address_line4(), "");
        }

        #[test]
        fn test_clone() {
            let address1 = UnvalidatedAddress::new(
                "123 Main St".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
                "Boston".to_string(),
                "02101".to_string(),
                "MA".to_string(),
                "USA".to_string(),
            );
            let address2 = address1.clone();
            assert_eq!(address1, address2);
        }
    }

    mod unvalidated_order_line_tests {
        use super::*;

        #[test]
        fn test_new_and_getters() {
            let quantity = Decimal::from_str("10").unwrap();
            let order_line =
                UnvalidatedOrderLine::new("line-001".to_string(), "W1234".to_string(), quantity);

            assert_eq!(order_line.order_line_id(), "line-001");
            assert_eq!(order_line.product_code(), "W1234");
            assert_eq!(order_line.quantity(), quantity);
        }

        #[test]
        fn test_with_gizmo_code() {
            let quantity = Decimal::from_str("2.50").unwrap();
            let order_line =
                UnvalidatedOrderLine::new("line-002".to_string(), "G123".to_string(), quantity);

            assert_eq!(order_line.product_code(), "G123");
            assert_eq!(order_line.quantity(), Decimal::from_str("2.50").unwrap());
        }

        #[test]
        fn test_clone() {
            let order_line1 = UnvalidatedOrderLine::new(
                "line-003".to_string(),
                "W5678".to_string(),
                Decimal::from(5),
            );
            let order_line2 = order_line1.clone();
            assert_eq!(order_line1, order_line2);
        }
    }

    mod unvalidated_order_tests {
        use super::*;

        fn create_customer_info() -> UnvalidatedCustomerInfo {
            UnvalidatedCustomerInfo::new(
                "John".to_string(),
                "Doe".to_string(),
                "john@example.com".to_string(),
                "Normal".to_string(),
            )
        }

        fn create_address() -> UnvalidatedAddress {
            UnvalidatedAddress::new(
                "123 Main St".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
                "New York".to_string(),
                "10001".to_string(),
                "NY".to_string(),
                "USA".to_string(),
            )
        }

        #[test]
        fn test_new_and_getters() {
            let customer_info = create_customer_info();
            let shipping_address = create_address();
            let billing_address = create_address();
            let lines = vec![
                UnvalidatedOrderLine::new(
                    "line-001".to_string(),
                    "W1234".to_string(),
                    Decimal::from(10),
                ),
                UnvalidatedOrderLine::new(
                    "line-002".to_string(),
                    "G123".to_string(),
                    Decimal::from_str("2.50").unwrap(),
                ),
            ];

            let order = UnvalidatedOrder::new(
                "order-001".to_string(),
                customer_info.clone(),
                shipping_address.clone(),
                billing_address.clone(),
                lines.clone(),
                "PROMO2024".to_string(),
            );

            assert_eq!(order.order_id(), "order-001");
            assert_eq!(order.customer_info(), &customer_info);
            assert_eq!(order.shipping_address(), &shipping_address);
            assert_eq!(order.billing_address(), &billing_address);
            assert_eq!(order.lines().len(), 2);
            assert_eq!(order.promotion_code(), "PROMO2024");
        }

        #[test]
        fn test_with_empty_promotion_code() {
            let order = UnvalidatedOrder::new(
                "order-002".to_string(),
                create_customer_info(),
                create_address(),
                create_address(),
                vec![],
                "".to_string(),
            );

            assert_eq!(order.promotion_code(), "");
        }

        #[test]
        fn test_clone() {
            let order1 = UnvalidatedOrder::new(
                "order-003".to_string(),
                create_customer_info(),
                create_address(),
                create_address(),
                vec![],
                "".to_string(),
            );
            let order2 = order1.clone();
            assert_eq!(order1, order2);
        }
    }
}
