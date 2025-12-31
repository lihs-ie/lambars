//! ダミー依存関数
//!
//! `PlaceOrder` ワークフローに注入するダミーの依存関数を提供する。
//! 実際のアプリケーションでは、外部サービスとの統合実装に置き換える。

use std::rc::Rc;

use functional_rusty::effect::IO;
use rust_decimal::Decimal;

use crate::simple_types::{Price, ProductCode};
use crate::workflow::UnvalidatedAddress;
use crate::workflow::acknowledgment_types::{HtmlString, OrderAcknowledgment, SendResult};
use crate::workflow::priced_types::PricedOrder;
use crate::workflow::shipping_types::PricedOrderWithShippingMethod;
use crate::workflow::validated_types::{AddressValidationError, CheckedAddress, PricingMethod};

// =============================================================================
// check_product_exists (REQ-090)
// =============================================================================

/// 製品が存在するかどうかを確認するダミー関数
///
/// 製品コードの形式が正しければ存在するものとして `true` を返す。
///
/// # Arguments
///
/// * `product_code` - 確認する製品コード
///
/// # Returns
///
/// 製品が存在する場合 `true`
///
/// # Examples
///
/// ```
/// use order_taking_sample::api::check_product_exists;
/// use order_taking_sample::simple_types::ProductCode;
///
/// let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
/// assert!(check_product_exists(&product_code));
/// ```
#[must_use]
pub const fn check_product_exists(_product_code: &ProductCode) -> bool {
    // ダミー実装: 全ての製品が存在するものとする
    true
}

// =============================================================================
// check_address_exists (REQ-090)
// =============================================================================

/// 住所が有効かどうかを確認するダミー関数
///
/// 住所の形式が正しければ有効なものとして `Ok(CheckedAddress)` を返す。
///
/// # Arguments
///
/// * `address` - 確認する住所
///
/// # Returns
///
/// * `Ok(CheckedAddress)` - 住所が有効な場合
/// * `Err(AddressValidationError)` - 住所が無効な場合（このダミー実装では発生しない）
///
/// # Errors
///
/// このダミー実装ではエラーは返さない。
/// 実際の実装では、住所が見つからない場合に `AddressValidationError` を返す。
///
/// # Examples
///
/// ```
/// use order_taking_sample::api::check_address_exists;
/// use order_taking_sample::workflow::UnvalidatedAddress;
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
/// let result = check_address_exists(&address);
/// assert!(result.is_ok());
/// ```
pub fn check_address_exists(
    address: &UnvalidatedAddress,
) -> Result<CheckedAddress, AddressValidationError> {
    // ダミー実装: 全ての住所を有効として CheckedAddress にラップ
    Ok(CheckedAddress::new(address.clone()))
}

// =============================================================================
// get_pricing_function (REQ-090)
// =============================================================================

/// 価格計算関数を返すダミー関数
///
/// 製品コードに応じた固定価格を返す関数を生成する。
///
/// # Arguments
///
/// * `_pricing_method` - 価格計算方法（このダミー実装では使用しない）
///
/// # Returns
///
/// 製品コードを受け取り価格を返す関数
///
/// # Examples
///
/// ```
/// use order_taking_sample::api::get_pricing_function;
/// use order_taking_sample::simple_types::ProductCode;
/// use order_taking_sample::workflow::PricingMethod;
/// use rust_decimal::Decimal;
///
/// let pricing_fn = get_pricing_function(&PricingMethod::Standard);
/// let product_code = ProductCode::create("ProductCode", "W1234").unwrap();
/// let price = pricing_fn(&product_code);
///
/// assert_eq!(price.value(), Decimal::from(100));
/// ```
#[must_use]
pub fn get_pricing_function(_pricing_method: &PricingMethod) -> Rc<dyn Fn(&ProductCode) -> Price> {
    // ダミー実装: Widget は 100、Gizmo は 50 の固定価格
    Rc::new(|product_code: &ProductCode| match product_code {
        ProductCode::Widget(_) => Price::unsafe_create(Decimal::from(100)),
        ProductCode::Gizmo(_) => Price::unsafe_create(Decimal::from(50)),
    })
}

// =============================================================================
// calculate_shipping_cost (REQ-090)
// =============================================================================

/// 配送コストを計算するダミー関数
///
/// 固定の配送コストを返す。
///
/// # Arguments
///
/// * `_priced_order` - 価格計算済み注文（このダミー実装では使用しない）
///
/// # Returns
///
/// 配送コスト（固定 10）
///
/// # Examples
///
/// ```ignore
/// use order_taking_sample::api::calculate_shipping_cost;
///
/// // PricedOrder を生成して渡す（省略）
/// // let cost = calculate_shipping_cost(&priced_order);
/// // assert_eq!(cost.value(), Decimal::from(10));
/// ```
#[must_use]
pub fn calculate_shipping_cost(_priced_order: &PricedOrder) -> Price {
    // ダミー実装: 固定配送コスト
    Price::unsafe_create(Decimal::from(10))
}

// =============================================================================
// create_acknowledgment_letter (REQ-090)
// =============================================================================

/// 確認メールを生成するダミー関数
///
/// 注文情報から確認メールの HTML を生成する。
///
/// # Arguments
///
/// * `order` - 配送方法付きの価格計算済み注文
///
/// # Returns
///
/// 確認メールの HTML
///
/// # Examples
///
/// ```ignore
/// use order_taking_sample::api::create_acknowledgment_letter;
///
/// // PricedOrderWithShippingMethod を生成して渡す（省略）
/// // let html = create_acknowledgment_letter(&order);
/// // assert!(html.value().contains("<p>"));
/// ```
#[must_use]
pub fn create_acknowledgment_letter(order: &PricedOrderWithShippingMethod) -> HtmlString {
    // ダミー実装: シンプルな HTML を生成
    let order_id = order.priced_order().order_id().value();
    HtmlString::new(format!(
        "<h1>Order Confirmation</h1><p>Your order {order_id} has been received.</p>"
    ))
}

// =============================================================================
// send_acknowledgment (REQ-090)
// =============================================================================

/// 確認メールを送信するダミー関数
///
/// 確認メールを送信した結果を返す。
///
/// # Arguments
///
/// * `_acknowledgment` - 送信する確認メール情報
///
/// # Returns
///
/// 送信結果を返す `IO<SendResult>`
///
/// # Examples
///
/// ```ignore
/// use order_taking_sample::api::send_acknowledgment;
///
/// // OrderAcknowledgment を生成して渡す（省略）
/// // let io_result = send_acknowledgment(&acknowledgment);
/// // let result = io_result.run_unsafe();
/// // assert!(matches!(result, SendResult::Sent));
/// ```
#[must_use]
pub fn send_acknowledgment(_acknowledgment: &OrderAcknowledgment) -> IO<SendResult> {
    // ダミー実装: 常に送信成功を返す
    IO::pure(SendResult::Sent)
}
