//! バリデーションロジック
//!
//! `UnvalidatedOrder` から `ValidatedOrder` への変換を行う。
//! F# の result computation expression に相当するパターンを
//! Rust の `Result` と `?` 演算子で実現する。
//!
//! # 設計原則
//!
//! - 純粋関数: 外部サービス呼び出しを除き、全て参照透過
//! - 早期リターン: ? 演算子によるエラー時の即座のリターン
//! - 依存性注入: 外部サービスを関数引数として受け取る
//! - 合成可能性: 小さな関数から大きな関数を組み立てる
//!
//! # 使用例
//!
//! ```
//! use order_taking_sample::workflow::{
//!     validate_order, UnvalidatedOrder, ValidatedOrder, PlaceOrderError,
//!     CheckedAddress, AddressValidationError, UnvalidatedAddress,
//!     UnvalidatedCustomerInfo, UnvalidatedOrderLine,
//! };
//! use order_taking_sample::simple_types::ProductCode;
//! use rust_decimal::Decimal;
//!
//! // 依存関数の定義
//! let check_product = |_: &ProductCode| true;
//! let check_address = |addr: &UnvalidatedAddress| {
//!     Ok(CheckedAddress::new(addr.clone()))
//! };
//!
//! // 注文データの作成
//! let customer_info = UnvalidatedCustomerInfo::new(
//!     "John".to_string(),
//!     "Doe".to_string(),
//!     "john@example.com".to_string(),
//!     "Normal".to_string(),
//! );
//! let address = UnvalidatedAddress::new(
//!     "123 Main St".to_string(),
//!     "".to_string(),
//!     "".to_string(),
//!     "".to_string(),
//!     "New York".to_string(),
//!     "10001".to_string(),
//!     "NY".to_string(),
//!     "USA".to_string(),
//! );
//! let lines = vec![
//!     UnvalidatedOrderLine::new("line-001".to_string(), "W1234".to_string(), Decimal::from(10)),
//! ];
//! let order = UnvalidatedOrder::new(
//!     "order-001".to_string(),
//!     customer_info,
//!     address.clone(),
//!     address,
//!     lines,
//!     "".to_string(),
//! );
//!
//! // バリデーション実行
//! let result = validate_order(&check_product, &check_address, &order);
//! assert!(result.is_ok());
//! ```

use crate::compound_types::{Address, CustomerInfo, PersonalName};
use crate::simple_types::{
    EmailAddress, OrderId, OrderLineId, OrderQuantity, ProductCode, PromotionCode, String50,
    ValidationError, VipStatus,
};
use crate::workflow::{
    AddressValidationError, CheckedAddress, PlaceOrderError, PricingMethod, UnvalidatedAddress,
    UnvalidatedCustomerInfo, UnvalidatedOrder, UnvalidatedOrderLine, ValidatedOrder,
    ValidatedOrderLine,
};
use rust_decimal::Decimal;

// =============================================================================
// to_order_id (REQ-049)
// =============================================================================

/// 未検証の注文ID文字列を `OrderId` に変換する
///
/// # Arguments
///
/// * `order_id` - 未検証の注文ID文字列
///
/// # Returns
///
/// * `Ok(OrderId)` - バリデーション成功時
/// * `Err(ValidationError)` - 空文字列または50文字超過時
///
/// # Errors
///
/// - 空文字列の場合: `"Must not be empty"`
/// - 50文字超過の場合: `"Must not be more than 50 chars"`
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::validation::to_order_id;
///
/// let order_id = to_order_id("order-001").unwrap();
/// assert_eq!(order_id.value(), "order-001");
///
/// let error = to_order_id("").unwrap_err();
/// assert_eq!(error.field_name, "OrderId");
/// ```
#[inline]
pub fn to_order_id(order_id: &str) -> Result<OrderId, ValidationError> {
    OrderId::create("OrderId", order_id)
}

// =============================================================================
// to_order_line_id (REQ-050)
// =============================================================================

/// 未検証の注文明細ID文字列を `OrderLineId` に変換する
///
/// # Arguments
///
/// * `order_line_id` - 未検証の注文明細ID文字列
///
/// # Returns
///
/// * `Ok(OrderLineId)` - バリデーション成功時
/// * `Err(ValidationError)` - 空文字列または50文字超過時
///
/// # Errors
///
/// - 空文字列の場合: `"Must not be empty"`
/// - 50文字超過の場合: `"Must not be more than 50 chars"`
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::validation::to_order_line_id;
///
/// let line_id = to_order_line_id("line-001").unwrap();
/// assert_eq!(line_id.value(), "line-001");
/// ```
#[inline]
pub fn to_order_line_id(order_line_id: &str) -> Result<OrderLineId, ValidationError> {
    OrderLineId::create("OrderLineId", order_line_id)
}

// =============================================================================
// to_customer_info (REQ-051)
// =============================================================================

/// `UnvalidatedCustomerInfo` を `CustomerInfo` に変換する
///
/// 各フィールドのバリデーションを順次実行し、最初のエラーで失敗する。
///
/// # Arguments
///
/// * `unvalidated` - 未検証の顧客情報
///
/// # Returns
///
/// * `Ok(CustomerInfo)` - 全フィールドが有効な場合
/// * `Err(ValidationError)` - いずれかのフィールドが無効な場合
///
/// # Errors
///
/// - `FirstName` が無効: `"Must not be empty"` または `"Must not be more than 50 chars"`
/// - `LastName` が無効: 同上
/// - `EmailAddress` が無効: `"Must match the pattern ..."`
/// - `VipStatus` が無効: `"Must be 'Normal' or 'VIP'"`
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{UnvalidatedCustomerInfo, validation::to_customer_info};
///
/// let unvalidated = UnvalidatedCustomerInfo::new(
///     "John".to_string(),
///     "Doe".to_string(),
///     "john@example.com".to_string(),
///     "Normal".to_string(),
/// );
/// let customer_info = to_customer_info(&unvalidated).unwrap();
/// assert_eq!(customer_info.name().first_name().value(), "John");
/// ```
pub fn to_customer_info(
    unvalidated: &UnvalidatedCustomerInfo,
) -> Result<CustomerInfo, ValidationError> {
    let first_name = String50::create("FirstName", unvalidated.first_name())?;
    let last_name = String50::create("LastName", unvalidated.last_name())?;
    let email_address = EmailAddress::create("EmailAddress", unvalidated.email_address())?;
    let vip_status = VipStatus::create("VipStatus", unvalidated.vip_status())?;

    let personal_name = PersonalName::create_from_parts(first_name, last_name);
    Ok(CustomerInfo::create_from_parts(
        personal_name,
        email_address,
        vip_status,
    ))
}

// =============================================================================
// to_address (REQ-052)
// =============================================================================

/// `CheckedAddress` を `Address` に変換する
///
/// `CheckedAddress` は既に外部サービスで検証済みなので、
/// 内部データの形式変換のみを行う。
///
/// # Arguments
///
/// * `checked_address` - 外部サービスで検証済みの住所
///
/// # Returns
///
/// * `Ok(Address)` - 変換成功時
/// * `Err(ValidationError)` - 形式変換時のエラー
///
/// # Errors
///
/// - `AddressLine1` が無効: `"Must not be empty"`
/// - `City` が無効: `"Must not be empty"`
/// - `ZipCode` が無効: `"must match the pattern..."`
/// - `State` が無効: `"must match the pattern..."`
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{
///     CheckedAddress, UnvalidatedAddress, validation::to_address
/// };
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
/// let checked = CheckedAddress::new(unvalidated);
/// let address = to_address(&checked).unwrap();
/// assert_eq!(address.city().value(), "New York");
/// ```
pub fn to_address(checked_address: &CheckedAddress) -> Result<Address, ValidationError> {
    let unvalidated = checked_address.value();
    Address::create(
        unvalidated.address_line1(),
        unvalidated.address_line2(),
        unvalidated.address_line3(),
        unvalidated.address_line4(),
        unvalidated.city(),
        unvalidated.zip_code(),
        unvalidated.state(),
        unvalidated.country(),
    )
}

// =============================================================================
// to_checked_address (REQ-053)
// =============================================================================

/// `UnvalidatedAddress` を外部サービスで検証し、`CheckedAddress` に変換する
///
/// 検証関数は依存性として注入される。
/// `AddressValidationError` は `ValidationError` に変換される。
///
/// # Type Parameters
///
/// * `CheckAddress` - 住所検証関数の型
///
/// # Arguments
///
/// * `check_address_exists` - 住所検証関数
/// * `address` - 未検証の住所
///
/// # Returns
///
/// * `Ok(CheckedAddress)` - 検証成功時
/// * `Err(ValidationError)` - 検証失敗時
///
/// # Errors
///
/// - `AddressNotFound`: `"Address not found"`
/// - `InvalidFormat`: `"Address has bad format"`
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{
///     CheckedAddress, UnvalidatedAddress, AddressValidationError,
///     validation::to_checked_address,
/// };
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
/// // 常に成功するモック
/// let check_address = |addr: &UnvalidatedAddress| Ok(CheckedAddress::new(addr.clone()));
/// let checked = to_checked_address(&check_address, &address).unwrap();
/// ```
pub fn to_checked_address<CheckAddress>(
    check_address_exists: &CheckAddress,
    address: &UnvalidatedAddress,
) -> Result<CheckedAddress, ValidationError>
where
    CheckAddress: Fn(&UnvalidatedAddress) -> Result<CheckedAddress, AddressValidationError>,
{
    check_address_exists(address).map_err(|error| match error {
        AddressValidationError::AddressNotFound => {
            ValidationError::new("Address", "Address not found")
        }
        AddressValidationError::InvalidFormat => {
            ValidationError::new("Address", "Address has bad format")
        }
    })
}

// =============================================================================
// to_product_code (REQ-054)
// =============================================================================

/// 未検証の商品コード文字列を `ProductCode` に変換し、存在チェックを行う
///
/// 1. まず `ProductCode::create` で形式を検証
/// 2. 次に `check_product_code_exists` で存在を確認
///
/// # Type Parameters
///
/// * `CheckProduct` - 商品コード存在チェック関数の型
///
/// # Arguments
///
/// * `check_product_code_exists` - 商品コードがシステムに存在するかをチェックする関数
/// * `product_code` - 未検証の商品コード文字列
///
/// # Returns
///
/// * `Ok(ProductCode)` - 形式が有効かつ存在する場合
/// * `Err(ValidationError)` - 形式が無効または存在しない場合
///
/// # Errors
///
/// - 形式が無効: `"Format not recognized: ..."`
/// - 存在しない: `"Invalid: {code}"`
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::validation::to_product_code;
/// use order_taking_sample::simple_types::ProductCode;
///
/// // 常に存在するモック
/// let check_product = |_: &ProductCode| true;
/// let product_code = to_product_code(&check_product, "W1234").unwrap();
///
/// // 存在しない場合
/// let check_product_none = |_: &ProductCode| false;
/// let error = to_product_code(&check_product_none, "W9999").unwrap_err();
/// assert!(error.message.contains("Invalid"));
/// ```
pub fn to_product_code<CheckProduct>(
    check_product_code_exists: &CheckProduct,
    product_code: &str,
) -> Result<ProductCode, ValidationError>
where
    CheckProduct: Fn(&ProductCode) -> bool,
{
    let product_code = ProductCode::create("ProductCode", product_code)?;

    if check_product_code_exists(&product_code) {
        Ok(product_code)
    } else {
        Err(ValidationError::new(
            "ProductCode",
            &format!("Invalid: {}", product_code.value()),
        ))
    }
}

// =============================================================================
// to_order_quantity (REQ-055)
// =============================================================================

/// 未検証の数量を `OrderQuantity` に変換する
///
/// `ProductCode` によって `UnitQuantity`（Widget）または
/// `KilogramQuantity`（Gizmo）が選択される。
///
/// # Arguments
///
/// * `product_code` - 製品コード（数量型の選択に使用）
/// * `quantity` - 未検証の数量
///
/// # Returns
///
/// * `Ok(OrderQuantity)` - 有効な数量の場合
/// * `Err(ValidationError)` - 範囲外の場合
///
/// # Errors
///
/// - Widget (`UnitQuantity`): 1-1000 の整数以外
/// - Gizmo (`KilogramQuantity`): 0.05-100.00 の範囲外
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::validation::to_order_quantity;
/// use order_taking_sample::simple_types::ProductCode;
/// use rust_decimal::Decimal;
///
/// let widget_code = ProductCode::create("ProductCode", "W1234").unwrap();
/// let quantity = to_order_quantity(&widget_code, Decimal::from(10)).unwrap();
/// ```
#[inline]
pub fn to_order_quantity(
    product_code: &ProductCode,
    quantity: Decimal,
) -> Result<OrderQuantity, ValidationError> {
    OrderQuantity::create("Quantity", product_code, quantity)
}

// =============================================================================
// create_pricing_method (REQ-057)
// =============================================================================

/// プロモーションコード文字列から `PricingMethod` を生成する
///
/// 空文字列の場合は `Standard`、それ以外は `Promotion` を返す。
/// この関数はバリデーションエラーを返さない。
///
/// # Arguments
///
/// * `promotion_code` - プロモーションコード文字列（空の場合もあり）
///
/// # Returns
///
/// * `PricingMethod::Standard` - 空文字列の場合
/// * `PricingMethod::Promotion(PromotionCode)` - それ以外の場合
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{PricingMethod, validation::create_pricing_method};
///
/// let standard = create_pricing_method("");
/// assert!(standard.is_standard());
///
/// let promotion = create_pricing_method("SUMMER2024");
/// assert!(promotion.is_promotion());
/// ```
#[must_use]
pub fn create_pricing_method(promotion_code: &str) -> PricingMethod {
    if promotion_code.is_empty() {
        PricingMethod::Standard
    } else {
        PricingMethod::Promotion(PromotionCode::new(promotion_code.to_string()))
    }
}

// =============================================================================
// to_validated_order_line (REQ-056)
// =============================================================================

/// `UnvalidatedOrderLine` を `ValidatedOrderLine` に変換する
///
/// 商品コードの存在チェックを含む。
///
/// # Type Parameters
///
/// * `CheckProduct` - 商品コード存在チェック関数の型
///
/// # Arguments
///
/// * `check_product_code_exists` - 商品コード存在チェック関数
/// * `unvalidated` - 未検証の注文明細
///
/// # Returns
///
/// * `Ok(ValidatedOrderLine)` - 全フィールドが有効な場合
/// * `Err(ValidationError)` - いずれかのフィールドが無効な場合
///
/// # Errors
///
/// - `OrderLineId` が無効
/// - `ProductCode` が無効または存在しない
/// - `Quantity` が範囲外
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{
///     UnvalidatedOrderLine, validation::to_validated_order_line
/// };
/// use order_taking_sample::simple_types::ProductCode;
/// use rust_decimal::Decimal;
///
/// let unvalidated = UnvalidatedOrderLine::new(
///     "line-001".to_string(),
///     "W1234".to_string(),
///     Decimal::from(10),
/// );
/// let check_product = |_: &ProductCode| true;
/// let validated = to_validated_order_line(&check_product, &unvalidated).unwrap();
/// ```
pub fn to_validated_order_line<CheckProduct>(
    check_product_code_exists: &CheckProduct,
    unvalidated: &UnvalidatedOrderLine,
) -> Result<ValidatedOrderLine, ValidationError>
where
    CheckProduct: Fn(&ProductCode) -> bool,
{
    let order_line_id = to_order_line_id(unvalidated.order_line_id())?;
    let product_code = to_product_code(check_product_code_exists, unvalidated.product_code())?;
    let quantity = to_order_quantity(&product_code, unvalidated.quantity())?;

    Ok(ValidatedOrderLine::new(
        order_line_id,
        product_code,
        quantity,
    ))
}

// =============================================================================
// validate_order (REQ-058)
// =============================================================================

/// `UnvalidatedOrder` を `ValidatedOrder` に変換するメイン関数
///
/// 全てのサブバリデーションを統合し、依存関数を注入する。
///
/// # Type Parameters
///
/// * `CheckProduct` - 商品コード存在チェック関数の型
/// * `CheckAddress` - 住所検証関数の型
///
/// # Arguments
///
/// * `check_product_code_exists` - 商品コードがシステムに存在するかをチェックする関数
/// * `check_address_exists` - 住所を外部サービスで検証する関数
/// * `unvalidated_order` - 未検証の注文
///
/// # Returns
///
/// * `Ok(ValidatedOrder)` - 全バリデーション成功時
/// * `Err(PlaceOrderError::Validation)` - いずれかのバリデーション失敗時
///
/// # Errors
///
/// - `OrderId` が無効
/// - `CustomerInfo` が無効
/// - `ShippingAddress` が無効または見つからない
/// - `BillingAddress` が無効または見つからない
/// - `OrderLine` が無効（商品コードが無効または存在しない、数量が範囲外）
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{
///     validate_order, UnvalidatedOrder, UnvalidatedCustomerInfo,
///     UnvalidatedAddress, UnvalidatedOrderLine, CheckedAddress,
///     AddressValidationError,
/// };
/// use order_taking_sample::simple_types::ProductCode;
/// use rust_decimal::Decimal;
///
/// let customer_info = UnvalidatedCustomerInfo::new(
///     "John".to_string(),
///     "Doe".to_string(),
///     "john@example.com".to_string(),
///     "Normal".to_string(),
/// );
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
/// let lines = vec![
///     UnvalidatedOrderLine::new("line-001".to_string(), "W1234".to_string(), Decimal::from(10)),
/// ];
/// let order = UnvalidatedOrder::new(
///     "order-001".to_string(),
///     customer_info,
///     address.clone(),
///     address,
///     lines,
///     "".to_string(),
/// );
///
/// let check_product = |_: &ProductCode| true;
/// let check_address = |addr: &UnvalidatedAddress| Ok(CheckedAddress::new(addr.clone()));
///
/// let result = validate_order(&check_product, &check_address, &order);
/// assert!(result.is_ok());
/// ```
pub fn validate_order<CheckProduct, CheckAddress>(
    check_product_code_exists: &CheckProduct,
    check_address_exists: &CheckAddress,
    unvalidated_order: &UnvalidatedOrder,
) -> Result<ValidatedOrder, PlaceOrderError>
where
    CheckProduct: Fn(&ProductCode) -> bool,
    CheckAddress: Fn(&UnvalidatedAddress) -> Result<CheckedAddress, AddressValidationError>,
{
    // 注文ID
    let order_id = to_order_id(unvalidated_order.order_id())?;

    // 顧客情報
    let customer_info = to_customer_info(unvalidated_order.customer_info())?;

    // 配送先住所
    let checked_shipping_address =
        to_checked_address(check_address_exists, unvalidated_order.shipping_address())?;
    let shipping_address = to_address(&checked_shipping_address)?;

    // 請求先住所
    let checked_billing_address =
        to_checked_address(check_address_exists, unvalidated_order.billing_address())?;
    let billing_address = to_address(&checked_billing_address)?;

    // 注文明細
    let lines: Result<Vec<ValidatedOrderLine>, ValidationError> = unvalidated_order
        .lines()
        .iter()
        .map(|line| to_validated_order_line(check_product_code_exists, line))
        .collect();
    let lines = lines?;

    // 価格計算方法
    let pricing_method = create_pricing_method(unvalidated_order.promotion_code());

    Ok(ValidatedOrder::new(
        order_id,
        customer_info,
        shipping_address,
        billing_address,
        lines,
        pricing_method,
    ))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // モックヘルパー関数
    // =========================================================================

    fn always_exists_product() -> impl Fn(&ProductCode) -> bool {
        |_: &ProductCode| true
    }

    fn never_exists_product() -> impl Fn(&ProductCode) -> bool {
        |_: &ProductCode| false
    }

    fn always_valid_address()
    -> impl Fn(&UnvalidatedAddress) -> Result<CheckedAddress, AddressValidationError> {
        |addr: &UnvalidatedAddress| Ok(CheckedAddress::new(addr.clone()))
    }

    fn address_not_found()
    -> impl Fn(&UnvalidatedAddress) -> Result<CheckedAddress, AddressValidationError> {
        |_: &UnvalidatedAddress| Err(AddressValidationError::AddressNotFound)
    }

    fn address_invalid_format()
    -> impl Fn(&UnvalidatedAddress) -> Result<CheckedAddress, AddressValidationError> {
        |_: &UnvalidatedAddress| Err(AddressValidationError::InvalidFormat)
    }

    // =========================================================================
    // テストデータ作成ヘルパー
    // =========================================================================

    fn create_valid_customer_info() -> UnvalidatedCustomerInfo {
        UnvalidatedCustomerInfo::new(
            "John".to_string(),
            "Doe".to_string(),
            "john@example.com".to_string(),
            "Normal".to_string(),
        )
    }

    fn create_valid_address() -> UnvalidatedAddress {
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

    fn create_valid_order_line() -> UnvalidatedOrderLine {
        UnvalidatedOrderLine::new(
            "line-001".to_string(),
            "W1234".to_string(),
            Decimal::from(10),
        )
    }

    fn create_valid_order() -> UnvalidatedOrder {
        UnvalidatedOrder::new(
            "order-001".to_string(),
            create_valid_customer_info(),
            create_valid_address(),
            create_valid_address(),
            vec![create_valid_order_line()],
            "".to_string(),
        )
    }

    // =========================================================================
    // to_order_id テスト
    // =========================================================================

    #[rstest]
    fn test_to_order_id_valid() {
        let result = to_order_id("order-001");

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), "order-001");
    }

    #[rstest]
    fn test_to_order_id_empty() {
        let result = to_order_id("");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "OrderId");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_to_order_id_too_long() {
        let long_id = "a".repeat(51);
        let result = to_order_id(&long_id);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "OrderId");
        assert_eq!(error.message, "Must not be more than 50 chars");
    }

    #[rstest]
    fn test_to_order_id_boundary_50_chars() {
        let max_id = "a".repeat(50);
        let result = to_order_id(&max_id);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value().len(), 50);
    }

    // =========================================================================
    // to_order_line_id テスト
    // =========================================================================

    #[rstest]
    fn test_to_order_line_id_valid() {
        let result = to_order_line_id("line-001");

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), "line-001");
    }

    #[rstest]
    fn test_to_order_line_id_empty() {
        let result = to_order_line_id("");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "OrderLineId");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_to_order_line_id_too_long() {
        let long_id = "a".repeat(51);
        let result = to_order_line_id(&long_id);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "OrderLineId");
        assert_eq!(error.message, "Must not be more than 50 chars");
    }

    // =========================================================================
    // to_customer_info テスト
    // =========================================================================

    #[rstest]
    fn test_to_customer_info_valid() {
        let unvalidated = create_valid_customer_info();
        let result = to_customer_info(&unvalidated);

        assert!(result.is_ok());
        let customer_info = result.unwrap();
        assert_eq!(customer_info.name().first_name().value(), "John");
        assert_eq!(customer_info.name().last_name().value(), "Doe");
        assert_eq!(customer_info.email_address().value(), "john@example.com");
        assert!(matches!(customer_info.vip_status(), VipStatus::Normal));
    }

    #[rstest]
    fn test_to_customer_info_vip() {
        let unvalidated = UnvalidatedCustomerInfo::new(
            "Jane".to_string(),
            "Smith".to_string(),
            "jane@example.com".to_string(),
            "VIP".to_string(),
        );
        let result = to_customer_info(&unvalidated);

        assert!(result.is_ok());
        let customer_info = result.unwrap();
        assert!(matches!(customer_info.vip_status(), VipStatus::Vip));
    }

    #[rstest]
    fn test_to_customer_info_invalid_first_name() {
        let unvalidated = UnvalidatedCustomerInfo::new(
            "".to_string(),
            "Doe".to_string(),
            "john@example.com".to_string(),
            "Normal".to_string(),
        );
        let result = to_customer_info(&unvalidated);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "FirstName");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_to_customer_info_invalid_last_name() {
        let unvalidated = UnvalidatedCustomerInfo::new(
            "John".to_string(),
            "".to_string(),
            "john@example.com".to_string(),
            "Normal".to_string(),
        );
        let result = to_customer_info(&unvalidated);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "LastName");
        assert_eq!(error.message, "Must not be empty");
    }

    #[rstest]
    fn test_to_customer_info_invalid_email() {
        let unvalidated = UnvalidatedCustomerInfo::new(
            "John".to_string(),
            "Doe".to_string(),
            "invalid-email".to_string(),
            "Normal".to_string(),
        );
        let result = to_customer_info(&unvalidated);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "EmailAddress");
    }

    #[rstest]
    fn test_to_customer_info_invalid_vip_status() {
        let unvalidated = UnvalidatedCustomerInfo::new(
            "John".to_string(),
            "Doe".to_string(),
            "john@example.com".to_string(),
            "Premium".to_string(),
        );
        let result = to_customer_info(&unvalidated);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "VipStatus");
    }

    // =========================================================================
    // to_address テスト
    // =========================================================================

    #[rstest]
    fn test_to_address_valid_all_fields() {
        let unvalidated = UnvalidatedAddress::new(
            "123 Main St".to_string(),
            "Apt 4B".to_string(),
            "Building A".to_string(),
            "Floor 5".to_string(),
            "New York".to_string(),
            "10001".to_string(),
            "NY".to_string(),
            "USA".to_string(),
        );
        let checked = CheckedAddress::new(unvalidated);
        let result = to_address(&checked);

        assert!(result.is_ok());
        let address = result.unwrap();
        assert_eq!(address.address_line1().value(), "123 Main St");
        assert_eq!(address.address_line2().map(|s| s.value()), Some("Apt 4B"));
        assert_eq!(address.city().value(), "New York");
        assert_eq!(address.zip_code().value(), "10001");
        assert_eq!(address.state().value(), "NY");
    }

    #[rstest]
    fn test_to_address_valid_required_only() {
        let unvalidated = create_valid_address();
        let checked = CheckedAddress::new(unvalidated);
        let result = to_address(&checked);

        assert!(result.is_ok());
        let address = result.unwrap();
        assert!(address.address_line2().is_none());
        assert!(address.address_line3().is_none());
        assert!(address.address_line4().is_none());
    }

    #[rstest]
    fn test_to_address_invalid_address_line1() {
        let unvalidated = UnvalidatedAddress::new(
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "New York".to_string(),
            "10001".to_string(),
            "NY".to_string(),
            "USA".to_string(),
        );
        let checked = CheckedAddress::new(unvalidated);
        let result = to_address(&checked);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "AddressLine1");
    }

    #[rstest]
    fn test_to_address_invalid_zip_code() {
        let unvalidated = UnvalidatedAddress::new(
            "123 Main St".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "New York".to_string(),
            "1234".to_string(), // 4桁
            "NY".to_string(),
            "USA".to_string(),
        );
        let checked = CheckedAddress::new(unvalidated);
        let result = to_address(&checked);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "ZipCode");
    }

    #[rstest]
    fn test_to_address_invalid_state() {
        let unvalidated = UnvalidatedAddress::new(
            "123 Main St".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "New York".to_string(),
            "10001".to_string(),
            "XX".to_string(), // 無効な州コード
            "USA".to_string(),
        );
        let checked = CheckedAddress::new(unvalidated);
        let result = to_address(&checked);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "State");
    }

    // =========================================================================
    // to_checked_address テスト
    // =========================================================================

    #[rstest]
    fn test_to_checked_address_success() {
        let address = create_valid_address();
        let check_address = always_valid_address();
        let result = to_checked_address(&check_address, &address);

        assert!(result.is_ok());
    }

    #[rstest]
    fn test_to_checked_address_not_found() {
        let address = create_valid_address();
        let check_address = address_not_found();
        let result = to_checked_address(&check_address, &address);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Address");
        assert_eq!(error.message, "Address not found");
    }

    #[rstest]
    fn test_to_checked_address_invalid_format() {
        let address = create_valid_address();
        let check_address = address_invalid_format();
        let result = to_checked_address(&check_address, &address);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Address");
        assert_eq!(error.message, "Address has bad format");
    }

    // =========================================================================
    // to_product_code テスト
    // =========================================================================

    #[rstest]
    fn test_to_product_code_widget_exists() {
        let check_product = always_exists_product();
        let result = to_product_code(&check_product, "W1234");

        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), ProductCode::Widget(_)));
    }

    #[rstest]
    fn test_to_product_code_gizmo_exists() {
        let check_product = always_exists_product();
        let result = to_product_code(&check_product, "G123");

        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), ProductCode::Gizmo(_)));
    }

    #[rstest]
    fn test_to_product_code_invalid_format() {
        let check_product = always_exists_product();
        let result = to_product_code(&check_product, "X999");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "ProductCode");
        assert!(error.message.contains("Format not recognized"));
    }

    #[rstest]
    fn test_to_product_code_not_exists() {
        let check_product = never_exists_product();
        let result = to_product_code(&check_product, "W9999");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "ProductCode");
        assert!(error.message.contains("Invalid: W9999"));
    }

    // =========================================================================
    // to_order_quantity テスト
    // =========================================================================

    #[rstest]
    fn test_to_order_quantity_widget_valid() {
        let widget_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let result = to_order_quantity(&widget_code, Decimal::from(10));

        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), OrderQuantity::Unit(_)));
    }

    #[rstest]
    fn test_to_order_quantity_widget_invalid_zero() {
        let widget_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let result = to_order_quantity(&widget_code, Decimal::from(0));

        assert!(result.is_err());
    }

    #[rstest]
    fn test_to_order_quantity_widget_invalid_too_large() {
        let widget_code = ProductCode::create("ProductCode", "W1234").unwrap();
        let result = to_order_quantity(&widget_code, Decimal::from(1001));

        assert!(result.is_err());
    }

    #[rstest]
    fn test_to_order_quantity_gizmo_valid() {
        let gizmo_code = ProductCode::create("ProductCode", "G123").unwrap();
        let result = to_order_quantity(&gizmo_code, Decimal::new(55, 1)); // 5.5

        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), OrderQuantity::Kilogram(_)));
    }

    #[rstest]
    fn test_to_order_quantity_gizmo_invalid_too_small() {
        let gizmo_code = ProductCode::create("ProductCode", "G123").unwrap();
        let result = to_order_quantity(&gizmo_code, Decimal::new(1, 2)); // 0.01

        assert!(result.is_err());
    }

    #[rstest]
    fn test_to_order_quantity_gizmo_invalid_too_large() {
        let gizmo_code = ProductCode::create("ProductCode", "G123").unwrap();
        let result = to_order_quantity(&gizmo_code, Decimal::new(10001, 2)); // 100.01

        assert!(result.is_err());
    }

    // =========================================================================
    // create_pricing_method テスト
    // =========================================================================

    #[rstest]
    fn test_create_pricing_method_empty() {
        let result = create_pricing_method("");

        assert!(result.is_standard());
    }

    #[rstest]
    fn test_create_pricing_method_promotion() {
        let result = create_pricing_method("SUMMER2024");

        assert!(result.is_promotion());
        assert_eq!(result.promotion_code().unwrap().value(), "SUMMER2024");
    }

    #[rstest]
    fn test_create_pricing_method_any_string() {
        let result = create_pricing_method("ANY_CODE");

        assert!(result.is_promotion());
        assert_eq!(result.promotion_code().unwrap().value(), "ANY_CODE");
    }

    // =========================================================================
    // to_validated_order_line テスト
    // =========================================================================

    #[rstest]
    fn test_to_validated_order_line_widget_valid() {
        let unvalidated = create_valid_order_line();
        let check_product = always_exists_product();
        let result = to_validated_order_line(&check_product, &unvalidated);

        assert!(result.is_ok());
        let validated = result.unwrap();
        assert_eq!(validated.order_line_id().value(), "line-001");
        assert!(matches!(validated.product_code(), ProductCode::Widget(_)));
        assert!(matches!(validated.quantity(), OrderQuantity::Unit(_)));
    }

    #[rstest]
    fn test_to_validated_order_line_gizmo_valid() {
        let unvalidated = UnvalidatedOrderLine::new(
            "line-002".to_string(),
            "G123".to_string(),
            Decimal::new(55, 1),
        );
        let check_product = always_exists_product();
        let result = to_validated_order_line(&check_product, &unvalidated);

        assert!(result.is_ok());
        let validated = result.unwrap();
        assert!(matches!(validated.product_code(), ProductCode::Gizmo(_)));
        assert!(matches!(validated.quantity(), OrderQuantity::Kilogram(_)));
    }

    #[rstest]
    fn test_to_validated_order_line_invalid_order_line_id() {
        let unvalidated =
            UnvalidatedOrderLine::new("".to_string(), "W1234".to_string(), Decimal::from(10));
        let check_product = always_exists_product();
        let result = to_validated_order_line(&check_product, &unvalidated);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "OrderLineId");
    }

    #[rstest]
    fn test_to_validated_order_line_product_not_exists() {
        let unvalidated = create_valid_order_line();
        let check_product = never_exists_product();
        let result = to_validated_order_line(&check_product, &unvalidated);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "ProductCode");
    }

    #[rstest]
    fn test_to_validated_order_line_invalid_quantity() {
        let unvalidated = UnvalidatedOrderLine::new(
            "line-001".to_string(),
            "W1234".to_string(),
            Decimal::from(0),
        );
        let check_product = always_exists_product();
        let result = to_validated_order_line(&check_product, &unvalidated);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.field_name, "Quantity");
    }

    // =========================================================================
    // validate_order テスト
    // =========================================================================

    #[rstest]
    fn test_validate_order_success() {
        let order = create_valid_order();
        let check_product = always_exists_product();
        let check_address = always_valid_address();
        let result = validate_order(&check_product, &check_address, &order);

        assert!(result.is_ok());
        let validated = result.unwrap();
        assert_eq!(validated.order_id().value(), "order-001");
        assert_eq!(
            validated.customer_info().name().first_name().value(),
            "John"
        );
        assert_eq!(validated.shipping_address().city().value(), "New York");
        assert!(validated.pricing_method().is_standard());
    }

    #[rstest]
    fn test_validate_order_success_with_promotion() {
        let order = UnvalidatedOrder::new(
            "order-001".to_string(),
            create_valid_customer_info(),
            create_valid_address(),
            create_valid_address(),
            vec![create_valid_order_line()],
            "SUMMER2024".to_string(),
        );
        let check_product = always_exists_product();
        let check_address = always_valid_address();
        let result = validate_order(&check_product, &check_address, &order);

        assert!(result.is_ok());
        let validated = result.unwrap();
        assert!(validated.pricing_method().is_promotion());
    }

    #[rstest]
    fn test_validate_order_invalid_order_id() {
        let order = UnvalidatedOrder::new(
            "".to_string(),
            create_valid_customer_info(),
            create_valid_address(),
            create_valid_address(),
            vec![create_valid_order_line()],
            "".to_string(),
        );
        let check_product = always_exists_product();
        let check_address = always_valid_address();
        let result = validate_order(&check_product, &check_address, &order);

        assert!(result.is_err());
        assert!(result.unwrap_err().is_validation());
    }

    #[rstest]
    fn test_validate_order_invalid_customer_info() {
        let invalid_customer = UnvalidatedCustomerInfo::new(
            "".to_string(),
            "Doe".to_string(),
            "john@example.com".to_string(),
            "Normal".to_string(),
        );
        let order = UnvalidatedOrder::new(
            "order-001".to_string(),
            invalid_customer,
            create_valid_address(),
            create_valid_address(),
            vec![create_valid_order_line()],
            "".to_string(),
        );
        let check_product = always_exists_product();
        let check_address = always_valid_address();
        let result = validate_order(&check_product, &check_address, &order);

        assert!(result.is_err());
        assert!(result.unwrap_err().is_validation());
    }

    #[rstest]
    fn test_validate_order_shipping_address_not_found() {
        let order = create_valid_order();
        let check_product = always_exists_product();
        let check_address = address_not_found();
        let result = validate_order(&check_product, &check_address, &order);

        assert!(result.is_err());
        assert!(result.unwrap_err().is_validation());
    }

    #[rstest]
    fn test_validate_order_product_not_exists() {
        let order = create_valid_order();
        let check_product = never_exists_product();
        let check_address = always_valid_address();
        let result = validate_order(&check_product, &check_address, &order);

        assert!(result.is_err());
        assert!(result.unwrap_err().is_validation());
    }

    #[rstest]
    fn test_validate_order_multiple_lines() {
        let order = UnvalidatedOrder::new(
            "order-001".to_string(),
            create_valid_customer_info(),
            create_valid_address(),
            create_valid_address(),
            vec![
                UnvalidatedOrderLine::new(
                    "line-001".to_string(),
                    "W1234".to_string(),
                    Decimal::from(10),
                ),
                UnvalidatedOrderLine::new(
                    "line-002".to_string(),
                    "G123".to_string(),
                    Decimal::new(55, 1),
                ),
            ],
            "".to_string(),
        );
        let check_product = always_exists_product();
        let check_address = always_valid_address();
        let result = validate_order(&check_product, &check_address, &order);

        assert!(result.is_ok());
        let validated = result.unwrap();
        assert_eq!(validated.lines().len(), 2);
    }

    #[rstest]
    fn test_validate_order_first_line_invalid() {
        let order = UnvalidatedOrder::new(
            "order-001".to_string(),
            create_valid_customer_info(),
            create_valid_address(),
            create_valid_address(),
            vec![
                UnvalidatedOrderLine::new("".to_string(), "W1234".to_string(), Decimal::from(10)),
                UnvalidatedOrderLine::new(
                    "line-002".to_string(),
                    "G123".to_string(),
                    Decimal::new(55, 1),
                ),
            ],
            "".to_string(),
        );
        let check_product = always_exists_product();
        let check_address = always_valid_address();
        let result = validate_order(&check_product, &check_address, &order);

        assert!(result.is_err());
        assert!(result.unwrap_err().is_validation());
    }

    #[rstest]
    fn test_validate_order_empty_lines() {
        let order = UnvalidatedOrder::new(
            "order-001".to_string(),
            create_valid_customer_info(),
            create_valid_address(),
            create_valid_address(),
            vec![],
            "".to_string(),
        );
        let check_product = always_exists_product();
        let check_address = always_valid_address();
        let result = validate_order(&check_product, &check_address, &order);

        assert!(result.is_ok());
        let validated = result.unwrap();
        assert!(validated.lines().is_empty());
    }
}
