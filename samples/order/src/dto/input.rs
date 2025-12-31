//! 入力 DTO
//!
//! API リクエストのデシリアライズに使用する DTO 型を定義する。
//!
//! # 型一覧
//!
//! - [`CustomerInfoDto`] - 顧客情報 DTO
//! - [`AddressDto`] - 住所 DTO
//! - [`OrderFormLineDto`] - 注文明細 DTO
//! - [`OrderFormDto`] - 注文フォーム DTO

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::compound_types::Address;
use crate::workflow::{
    UnvalidatedAddress, UnvalidatedCustomerInfo, UnvalidatedOrder, UnvalidatedOrderLine,
};

// =============================================================================
// CustomerInfoDto (REQ-075)
// =============================================================================

/// 顧客情報 DTO
///
/// API から受け取る顧客情報をデシリアライズするための型。
///
/// # Examples
///
/// ```
/// use order_taking_sample::dto::CustomerInfoDto;
///
/// let json = r#"{
///     "first_name": "John",
///     "last_name": "Doe",
///     "email_address": "john@example.com",
///     "vip_status": "Normal"
/// }"#;
///
/// let dto: CustomerInfoDto = serde_json::from_str(json).unwrap();
/// assert_eq!(dto.first_name, "John");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomerInfoDto {
    /// 名
    pub first_name: String,
    /// 姓
    pub last_name: String,
    /// メールアドレス
    pub email_address: String,
    /// VIP ステータス（"Normal" または "VIP"）
    pub vip_status: String,
}

impl CustomerInfoDto {
    /// `UnvalidatedCustomerInfo` に変換する
    ///
    /// 純粋関数としてドメイン型に変換する。バリデーションは行わない。
    ///
    /// # Returns
    ///
    /// `UnvalidatedCustomerInfo` インスタンス
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::dto::CustomerInfoDto;
    ///
    /// let dto = CustomerInfoDto {
    ///     first_name: "John".to_string(),
    ///     last_name: "Doe".to_string(),
    ///     email_address: "john@example.com".to_string(),
    ///     vip_status: "Normal".to_string(),
    /// };
    ///
    /// let unvalidated = dto.to_unvalidated_customer_info();
    /// assert_eq!(unvalidated.first_name(), "John");
    /// ```
    #[must_use]
    pub fn to_unvalidated_customer_info(&self) -> UnvalidatedCustomerInfo {
        UnvalidatedCustomerInfo::new(
            self.first_name.clone(),
            self.last_name.clone(),
            self.email_address.clone(),
            self.vip_status.clone(),
        )
    }
}

// =============================================================================
// AddressDto (REQ-076)
// =============================================================================

/// 住所 DTO
///
/// API から受け取る住所をデシリアライズするための型。
///
/// # Examples
///
/// ```
/// use order_taking_sample::dto::AddressDto;
///
/// let json = r#"{
///     "address_line1": "123 Main St",
///     "address_line2": "Apt 4B",
///     "address_line3": "",
///     "address_line4": "",
///     "city": "New York",
///     "zip_code": "10001",
///     "state": "NY",
///     "country": "USA"
/// }"#;
///
/// let dto: AddressDto = serde_json::from_str(json).unwrap();
/// assert_eq!(dto.city, "New York");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressDto {
    /// 住所行1（必須）
    pub address_line1: String,
    /// 住所行2（オプション、空文字列で None）
    pub address_line2: String,
    /// 住所行3（オプション、空文字列で None）
    pub address_line3: String,
    /// 住所行4（オプション、空文字列で None）
    pub address_line4: String,
    /// 市
    pub city: String,
    /// 郵便番号
    pub zip_code: String,
    /// 州コード
    pub state: String,
    /// 国名
    pub country: String,
}

impl AddressDto {
    /// `UnvalidatedAddress` に変換する
    ///
    /// 純粋関数としてドメイン型に変換する。バリデーションは行わない。
    ///
    /// # Returns
    ///
    /// `UnvalidatedAddress` インスタンス
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::dto::AddressDto;
    ///
    /// let dto = AddressDto {
    ///     address_line1: "123 Main St".to_string(),
    ///     address_line2: "".to_string(),
    ///     address_line3: "".to_string(),
    ///     address_line4: "".to_string(),
    ///     city: "New York".to_string(),
    ///     zip_code: "10001".to_string(),
    ///     state: "NY".to_string(),
    ///     country: "USA".to_string(),
    /// };
    ///
    /// let unvalidated = dto.to_unvalidated_address();
    /// assert_eq!(unvalidated.city(), "New York");
    /// ```
    #[must_use]
    pub fn to_unvalidated_address(&self) -> UnvalidatedAddress {
        UnvalidatedAddress::new(
            self.address_line1.clone(),
            self.address_line2.clone(),
            self.address_line3.clone(),
            self.address_line4.clone(),
            self.city.clone(),
            self.zip_code.clone(),
            self.state.clone(),
            self.country.clone(),
        )
    }

    /// ドメインの `Address` から `AddressDto` を生成する
    ///
    /// 純粋関数として DTO に変換する。
    ///
    /// # Arguments
    ///
    /// * `address` - 変換元の `Address`
    ///
    /// # Returns
    ///
    /// `AddressDto` インスタンス
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::compound_types::Address;
    /// use order_taking_sample::dto::AddressDto;
    ///
    /// let address = Address::create(
    ///     "123 Main St", "", "", "", "New York", "10001", "NY", "USA"
    /// ).unwrap();
    ///
    /// let dto = AddressDto::from_address(&address);
    /// assert_eq!(dto.city, "New York");
    /// ```
    #[must_use]
    pub fn from_address(address: &Address) -> Self {
        Self {
            address_line1: address.address_line1().value().to_string(),
            address_line2: address
                .address_line2()
                .map_or_else(String::new, |s| s.value().to_string()),
            address_line3: address
                .address_line3()
                .map_or_else(String::new, |s| s.value().to_string()),
            address_line4: address
                .address_line4()
                .map_or_else(String::new, |s| s.value().to_string()),
            city: address.city().value().to_string(),
            zip_code: address.zip_code().value().to_string(),
            state: address.state().value().to_string(),
            country: address.country().value().to_string(),
        }
    }
}

// =============================================================================
// OrderFormLineDto (REQ-077)
// =============================================================================

/// 注文明細 DTO
///
/// API から受け取る注文明細をデシリアライズするための型。
/// 数量は精度を保持するため文字列としてシリアライズされる。
///
/// # Examples
///
/// ```
/// use order_taking_sample::dto::OrderFormLineDto;
/// use rust_decimal::Decimal;
///
/// let json = r#"{
///     "order_line_id": "line-001",
///     "product_code": "W1234",
///     "quantity": "10"
/// }"#;
///
/// let dto: OrderFormLineDto = serde_json::from_str(json).unwrap();
/// assert_eq!(dto.product_code, "W1234");
/// assert_eq!(dto.quantity, Decimal::from(10));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderFormLineDto {
    /// 注文明細ID
    pub order_line_id: String,
    /// 製品コード
    pub product_code: String,
    /// 数量（文字列形式の Decimal）
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
}

impl OrderFormLineDto {
    /// `UnvalidatedOrderLine` に変換する
    ///
    /// 純粋関数としてドメイン型に変換する。バリデーションは行わない。
    ///
    /// # Returns
    ///
    /// `UnvalidatedOrderLine` インスタンス
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::dto::OrderFormLineDto;
    /// use rust_decimal::Decimal;
    ///
    /// let dto = OrderFormLineDto {
    ///     order_line_id: "line-001".to_string(),
    ///     product_code: "W1234".to_string(),
    ///     quantity: Decimal::from(10),
    /// };
    ///
    /// let unvalidated = dto.to_unvalidated_order_line();
    /// assert_eq!(unvalidated.product_code(), "W1234");
    /// ```
    #[must_use]
    pub fn to_unvalidated_order_line(&self) -> UnvalidatedOrderLine {
        UnvalidatedOrderLine::new(
            self.order_line_id.clone(),
            self.product_code.clone(),
            self.quantity,
        )
    }
}

// =============================================================================
// OrderFormDto (REQ-078)
// =============================================================================

/// 注文フォーム DTO
///
/// API から受け取る注文全体をデシリアライズするための型。
/// `PlaceOrder` ワークフローの入力として使用される。
///
/// # Examples
///
/// ```
/// use order_taking_sample::dto::{OrderFormDto, CustomerInfoDto, AddressDto, OrderFormLineDto};
/// use rust_decimal::Decimal;
///
/// let dto = OrderFormDto {
///     order_id: "order-001".to_string(),
///     customer_info: CustomerInfoDto {
///         first_name: "John".to_string(),
///         last_name: "Doe".to_string(),
///         email_address: "john@example.com".to_string(),
///         vip_status: "Normal".to_string(),
///     },
///     shipping_address: AddressDto {
///         address_line1: "123 Main St".to_string(),
///         address_line2: "".to_string(),
///         address_line3: "".to_string(),
///         address_line4: "".to_string(),
///         city: "New York".to_string(),
///         zip_code: "10001".to_string(),
///         state: "NY".to_string(),
///         country: "USA".to_string(),
///     },
///     billing_address: AddressDto {
///         address_line1: "123 Main St".to_string(),
///         address_line2: "".to_string(),
///         address_line3: "".to_string(),
///         address_line4: "".to_string(),
///         city: "New York".to_string(),
///         zip_code: "10001".to_string(),
///         state: "NY".to_string(),
///         country: "USA".to_string(),
///     },
///     lines: vec![OrderFormLineDto {
///         order_line_id: "line-001".to_string(),
///         product_code: "W1234".to_string(),
///         quantity: Decimal::from(10),
///     }],
///     promotion_code: "".to_string(),
/// };
///
/// let unvalidated = dto.to_unvalidated_order();
/// assert_eq!(unvalidated.order_id(), "order-001");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderFormDto {
    /// 注文ID
    pub order_id: String,
    /// 顧客情報
    pub customer_info: CustomerInfoDto,
    /// 配送先住所
    pub shipping_address: AddressDto,
    /// 請求先住所
    pub billing_address: AddressDto,
    /// 注文明細リスト
    pub lines: Vec<OrderFormLineDto>,
    /// プロモーションコード（空文字列の場合もあり）
    pub promotion_code: String,
}

impl OrderFormDto {
    /// `UnvalidatedOrder` に変換する
    ///
    /// 純粋関数としてドメイン型に変換する。バリデーションは行わない。
    ///
    /// # Returns
    ///
    /// `UnvalidatedOrder` インスタンス
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::dto::{OrderFormDto, CustomerInfoDto, AddressDto, OrderFormLineDto};
    /// use rust_decimal::Decimal;
    ///
    /// let dto = OrderFormDto {
    ///     order_id: "order-001".to_string(),
    ///     customer_info: CustomerInfoDto {
    ///         first_name: "John".to_string(),
    ///         last_name: "Doe".to_string(),
    ///         email_address: "john@example.com".to_string(),
    ///         vip_status: "Normal".to_string(),
    ///     },
    ///     shipping_address: AddressDto {
    ///         address_line1: "123 Main St".to_string(),
    ///         address_line2: "".to_string(),
    ///         address_line3: "".to_string(),
    ///         address_line4: "".to_string(),
    ///         city: "New York".to_string(),
    ///         zip_code: "10001".to_string(),
    ///         state: "NY".to_string(),
    ///         country: "USA".to_string(),
    ///     },
    ///     billing_address: AddressDto {
    ///         address_line1: "123 Main St".to_string(),
    ///         address_line2: "".to_string(),
    ///         address_line3: "".to_string(),
    ///         address_line4: "".to_string(),
    ///         city: "New York".to_string(),
    ///         zip_code: "10001".to_string(),
    ///         state: "NY".to_string(),
    ///         country: "USA".to_string(),
    ///     },
    ///     lines: vec![],
    ///     promotion_code: "".to_string(),
    /// };
    ///
    /// let unvalidated = dto.to_unvalidated_order();
    /// assert_eq!(unvalidated.order_id(), "order-001");
    /// ```
    #[must_use]
    pub fn to_unvalidated_order(&self) -> UnvalidatedOrder {
        let customer_info = self.customer_info.to_unvalidated_customer_info();
        let shipping_address = self.shipping_address.to_unvalidated_address();
        let billing_address = self.billing_address.to_unvalidated_address();
        let lines: Vec<UnvalidatedOrderLine> = self
            .lines
            .iter()
            .map(OrderFormLineDto::to_unvalidated_order_line)
            .collect();

        UnvalidatedOrder::new(
            self.order_id.clone(),
            customer_info,
            shipping_address,
            billing_address,
            lines,
            self.promotion_code.clone(),
        )
    }
}
