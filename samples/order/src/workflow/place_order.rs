//! `PlaceOrder` ワークフロー
//!
//! Phase 7 の実装。ワークフロー全体を統合する。
//!
//! # 設計原則
//!
//! - 依存性注入: 全ての外部依存を関数引数として受け取る
//! - エラーハンドリング: Result と IO モナドによる関数型エラー処理
//! - 合成可能性: 各フェーズの関数を順次合成
//!
//! # 機能一覧
//!
//! - [`place_order`] - `PlaceOrder` ワークフローの実行
//!
//! # 使用例
//!
//! ```ignore
//! use order_taking_sample::workflow::place_order;
//! // let io_result = place_order(&check_product, &check_address, &get_price, ...);
//! // let result = io_result.run_unsafe();
//! ```

use lambars::compose;
use lambars::effect::IO;
use std::rc::Rc;

use crate::simple_types::{Price, ProductCode};
use crate::workflow::UnvalidatedAddress;
use crate::workflow::acknowledgment_types::{HtmlString, OrderAcknowledgment, SendResult};
use crate::workflow::error_types::PlaceOrderError;
use crate::workflow::events::create_events;
use crate::workflow::output_types::PlaceOrderEvent;
use crate::workflow::priced_types::PricedOrder;
use crate::workflow::pricing::price_order;
use crate::workflow::shipping::{acknowledge_order, add_shipping_info_to_order, free_vip_shipping};
use crate::workflow::shipping_types::PricedOrderWithShippingMethod;
use crate::workflow::unvalidated_types::UnvalidatedOrder;
use crate::workflow::validated_types::{AddressValidationError, CheckedAddress, PricingMethod};
use crate::workflow::validation::validate_order;

// =============================================================================
// place_order (REQ-074)
// =============================================================================

/// `PlaceOrder` ワークフロー全体を統合する関数
///
/// 未検証注文を受け取り、全ての処理を順次実行して
/// イベントリストまたはエラーを返す。
///
/// # 処理フロー
///
/// 1. `validate_order` - 未検証注文を検証（エラー: Validation）
/// 2. `price_order` - 価格計算（エラー: Pricing）
/// 3. `add_shipping_info_to_order` - 配送情報追加
/// 4. `free_vip_shipping` - VIP 無料配送適用
/// 5. `acknowledge_order` - 確認メール送信（IO モナド）
/// 6. `create_events` - イベント生成
///
/// # Type Parameters
///
/// * `CheckProduct` - 製品存在確認関数型
/// * `CheckAddress` - 住所検証関数型
/// * `GetPricingFn` - 価格取得関数を返す関数型
/// * `CalculateShipping` - 配送コスト計算関数型
/// * `CreateLetter` - 確認メール生成関数型
/// * `SendAcknowledgment` - 確認メール送信関数型（IO を返す）
///
/// # Arguments
///
/// * `check_product_exists` - 製品存在確認関数
/// * `check_address_exists` - 住所検証関数
/// * `get_pricing_function` - 価格取得関数を返す関数
/// * `calculate_shipping_cost` - 配送コスト計算関数
/// * `create_acknowledgment_letter` - 確認メール生成関数
/// * `send_acknowledgment` - 確認メール送信関数
/// * `unvalidated_order` - 未検証注文
///
/// # Returns
///
/// `IO<Result<Vec<PlaceOrderEvent>, PlaceOrderError>>`
/// - 成功時: `Ok(Vec<PlaceOrderEvent>)`（1-3イベント）
/// - 失敗時: `Err(PlaceOrderError)`（Validation または Pricing）
///
/// # Examples
///
/// ```ignore
/// use order_taking_sample::workflow::place_order;
/// use lambars::effect::IO;
///
/// let io_result = place_order(
///     &check_product,
///     &check_address,
///     &get_pricing_fn,
///     &calculate_shipping,
///     &create_letter,
///     &send_ack,
///     unvalidated_order,
/// );
///
/// // IO モナドを実行
/// let result = io_result.run_unsafe();
/// match result {
///     Ok(events) => println!("Events: {:?}", events),
///     Err(error) => println!("Error: {:?}", error),
/// }
/// ```
pub fn place_order<
    CheckProduct,
    CheckAddress,
    GetPricingFn,
    CalculateShipping,
    CreateLetter,
    SendAcknowledgment,
>(
    check_product_exists: &CheckProduct,
    check_address_exists: &CheckAddress,
    get_pricing_function: &GetPricingFn,
    calculate_shipping_cost: &CalculateShipping,
    create_acknowledgment_letter: &CreateLetter,
    send_acknowledgment: &SendAcknowledgment,
    unvalidated_order: &UnvalidatedOrder,
) -> IO<Result<Vec<PlaceOrderEvent>, PlaceOrderError>>
where
    CheckProduct: Fn(&ProductCode) -> bool,
    CheckAddress: Fn(&UnvalidatedAddress) -> Result<CheckedAddress, AddressValidationError>,
    GetPricingFn: Fn(&PricingMethod) -> Rc<dyn Fn(&ProductCode) -> Price>,
    CalculateShipping: Fn(&PricedOrder) -> Price,
    CreateLetter: Fn(&PricedOrderWithShippingMethod) -> HtmlString,
    SendAcknowledgment: Fn(&OrderAcknowledgment) -> IO<SendResult>,
{
    // Step 1: バリデーション
    let validated_order = match validate_order(
        check_product_exists,
        check_address_exists,
        unvalidated_order,
    ) {
        Ok(order) => order,
        Err(error) => return IO::pure(Err(error)),
    };

    // Step 2: 価格計算
    let priced_order = match price_order(get_pricing_function, &validated_order) {
        Ok(order) => order,
        Err(error) => return IO::pure(Err(error)),
    };

    // Step 3-4: 配送処理パイプラインを合成関数として定義
    // add_shipping_info_to_order を部分適用
    let add_shipping =
        |order: &PricedOrder| add_shipping_info_to_order(calculate_shipping_cost, order);

    // free_vip_shipping と add_shipping を合成
    // compose! は右から左: free_vip_shipping(add_shipping(order))
    let process_shipping = compose!(free_vip_shipping, add_shipping);

    // 合成関数を適用
    let priced_order_with_shipping = process_shipping(&priced_order);

    // Step 5: 確認メール送信（IO モナド）
    let acknowledgment_io = acknowledge_order(
        create_acknowledgment_letter,
        send_acknowledgment,
        &priced_order_with_shipping,
    );

    // Step 6: イベント生成（IO 内で実行）
    acknowledgment_io
        .fmap(move |acknowledgment_option| Ok(create_events(&priced_order, acknowledgment_option)))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::{UnvalidatedAddress, UnvalidatedCustomerInfo, UnvalidatedOrderLine};
    use rstest::rstest;
    use rust_decimal::Decimal;

    // =========================================================================
    // テストヘルパー
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
            "US".to_string(),
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

    fn always_exists_product() -> impl Fn(&ProductCode) -> bool {
        |_: &ProductCode| true
    }

    fn always_valid_address()
    -> impl Fn(&UnvalidatedAddress) -> Result<CheckedAddress, AddressValidationError> {
        |addr: &UnvalidatedAddress| Ok(CheckedAddress::new(addr.clone()))
    }

    fn fixed_price_function(
        price: Decimal,
    ) -> impl Fn(&PricingMethod) -> Rc<dyn Fn(&ProductCode) -> Price> {
        move |_: &PricingMethod| {
            let price_clone = price;
            Rc::new(move |_: &ProductCode| Price::unsafe_create(price_clone))
        }
    }

    fn calculate_shipping_cost_mock() -> impl Fn(&PricedOrder) -> Price {
        |_: &PricedOrder| Price::unsafe_create(Decimal::from(10))
    }

    fn create_letter_mock() -> impl Fn(&PricedOrderWithShippingMethod) -> HtmlString {
        |_: &PricedOrderWithShippingMethod| HtmlString::new("<p>Order confirmed</p>".to_string())
    }

    fn always_send() -> impl Fn(&OrderAcknowledgment) -> IO<SendResult> {
        |_: &OrderAcknowledgment| IO::pure(SendResult::Sent)
    }

    fn never_send() -> impl Fn(&OrderAcknowledgment) -> IO<SendResult> {
        |_: &OrderAcknowledgment| IO::pure(SendResult::NotSent)
    }

    // =========================================================================
    // place_order のテスト
    // =========================================================================

    #[rstest]
    fn test_place_order_success() {
        let order = create_valid_order();
        let check_product = always_exists_product();
        let check_address = always_valid_address();
        let get_pricing_fn = fixed_price_function(Decimal::from(100));
        let calculate_shipping = calculate_shipping_cost_mock();
        let create_letter = create_letter_mock();
        let send_acknowledgment = always_send();

        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &calculate_shipping,
            &create_letter,
            &send_acknowledgment,
            &order,
        );

        let result = io_result.run_unsafe();
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 3);
    }

    #[rstest]
    fn test_place_order_validation_error() {
        let order = UnvalidatedOrder::new(
            "".to_string(), // 無効な注文ID
            create_valid_customer_info(),
            create_valid_address(),
            create_valid_address(),
            vec![create_valid_order_line()],
            "".to_string(),
        );
        let check_product = always_exists_product();
        let check_address = always_valid_address();
        let get_pricing_fn = fixed_price_function(Decimal::from(100));
        let calculate_shipping = calculate_shipping_cost_mock();
        let create_letter = create_letter_mock();
        let send_acknowledgment = always_send();

        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &calculate_shipping,
            &create_letter,
            &send_acknowledgment,
            &order,
        );

        let result = io_result.run_unsafe();
        assert!(result.is_err());
        assert!(result.unwrap_err().is_validation());
    }

    #[rstest]
    fn test_place_order_mail_not_sent() {
        let order = create_valid_order();
        let check_product = always_exists_product();
        let check_address = always_valid_address();
        let get_pricing_fn = fixed_price_function(Decimal::from(100));
        let calculate_shipping = calculate_shipping_cost_mock();
        let create_letter = create_letter_mock();
        let send_acknowledgment = never_send();

        let io_result = place_order(
            &check_product,
            &check_address,
            &get_pricing_fn,
            &calculate_shipping,
            &create_letter,
            &send_acknowledgment,
            &order,
        );

        let result = io_result.run_unsafe();
        assert!(result.is_ok());
        let events = result.unwrap();
        // メール送信失敗でも成功、AcknowledgmentSent なし
        assert_eq!(events.len(), 2);
    }
}
