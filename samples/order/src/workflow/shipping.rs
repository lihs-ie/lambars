//! 配送コスト計算と確認メール送信
//!
//! Phase 6 の実装。配送地域の分類、配送コストの計算、
//! VIP 顧客への無料配送、および注文確認メールの送信を行う。
//!
//! # 機能一覧
//!
//! - [`ShippingRegion`] - 配送地域の分類（米国近隣州、米国遠方州、国際）
//! - [`classify_shipping_region`] - 住所から配送地域を分類
//! - [`calculate_shipping_cost`] - 配送地域に基づいてコストを計算
//! - [`add_shipping_info_to_order`] - 注文に配送情報を追加
//! - [`free_vip_shipping`] - VIP 顧客の配送料を無料化
//! - [`acknowledge_order`] - 注文確認メールを送信（IO モナド）
//!
//! # 使用例
//!
//! ```
//! use order_taking_sample::workflow::{
//!     ShippingRegion, classify_shipping_region, calculate_shipping_cost,
//!     add_shipping_info_to_order, free_vip_shipping,
//! };
//! use order_taking_sample::compound_types::Address;
//!
//! // 住所から配送地域を分類
//! let address = Address::create(
//!     "123 Main St", "", "", "", "Los Angeles", "90001", "CA", "US"
//! ).unwrap();
//! let region = classify_shipping_region(&address);
//! assert!(region.is_us_local_state());
//! ```

use functional_rusty::effect::IO;
use functional_rusty::optics::Lens;

use crate::compound_types::Address;
use crate::simple_types::{Price, VipStatus};
use crate::workflow::acknowledgment_types::{HtmlString, OrderAcknowledgment, SendResult};
use crate::workflow::output_types::OrderAcknowledgmentSent;
use crate::workflow::priced_types::PricedOrder;
use crate::workflow::shipping_types::{
    PricedOrderWithShippingMethod, ShippingInfo, ShippingMethod,
};

// =============================================================================
// 定数定義
// =============================================================================

/// 米国近隣州の州コードリスト
///
/// カリフォルニアを中心とした西海岸近隣州。
const US_LOCAL_STATES: [&str; 4] = ["CA", "OR", "AZ", "NV"];

/// 米国を表す国名のバリエーション
const US_COUNTRY_NAMES: [&str; 3] = ["US", "USA", "United States"];

/// 米国近隣州の配送コスト（ドル）
const LOCAL_STATE_SHIPPING_COST: u32 = 5;

/// 米国遠方州の配送コスト（ドル）
const REMOTE_STATE_SHIPPING_COST: u32 = 10;

/// 国際配送のコスト（ドル）
const INTERNATIONAL_SHIPPING_COST: u32 = 20;

// =============================================================================
// ShippingRegion 列挙型
// =============================================================================

/// 配送地域の分類
///
/// 配送先住所を3つのカテゴリに分類する。
/// 各カテゴリに応じて配送コストが決定される。
///
/// # カテゴリ
///
/// - [`UsLocalState`](ShippingRegion::UsLocalState) - 米国近隣州（CA, OR, AZ, NV）
/// - [`UsRemoteState`](ShippingRegion::UsRemoteState) - 米国遠方州（その他の米国州）
/// - [`International`](ShippingRegion::International) - 国際（米国以外）
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::ShippingRegion;
///
/// let region = ShippingRegion::UsLocalState;
/// assert!(region.is_us_local_state());
/// assert!(!region.is_us_remote_state());
/// assert!(!region.is_international());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShippingRegion {
    /// 米国近隣州（CA, OR, AZ, NV）
    ///
    /// 最も安い配送料金が適用される。
    UsLocalState,

    /// 米国遠方州（近隣州以外の米国内）
    ///
    /// 中程度の配送料金が適用される。
    UsRemoteState,

    /// 国際配送（米国外）
    ///
    /// 最も高い配送料金が適用される。
    International,
}

impl ShippingRegion {
    /// `UsLocalState` バリアントかどうかを返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::ShippingRegion;
    ///
    /// let region = ShippingRegion::UsLocalState;
    /// assert!(region.is_us_local_state());
    /// ```
    #[must_use]
    pub const fn is_us_local_state(&self) -> bool {
        matches!(self, Self::UsLocalState)
    }

    /// `UsRemoteState` バリアントかどうかを返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::ShippingRegion;
    ///
    /// let region = ShippingRegion::UsRemoteState;
    /// assert!(region.is_us_remote_state());
    /// ```
    #[must_use]
    pub const fn is_us_remote_state(&self) -> bool {
        matches!(self, Self::UsRemoteState)
    }

    /// `International` バリアントかどうかを返す
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::workflow::ShippingRegion;
    ///
    /// let region = ShippingRegion::International;
    /// assert!(region.is_international());
    /// ```
    #[must_use]
    pub const fn is_international(&self) -> bool {
        matches!(self, Self::International)
    }
}

// =============================================================================
// classify_shipping_region 関数
// =============================================================================

/// 住所から配送地域を分類する
///
/// 配送先住所の国と州コードに基づいて、[`ShippingRegion`] を決定する。
///
/// # 分類ルール
///
/// 1. 国が "US", "USA", "United States" のいずれかでない場合 -> [`International`](ShippingRegion::International)
/// 2. 州コードが "CA", "OR", "AZ", "NV" のいずれか -> [`UsLocalState`](ShippingRegion::UsLocalState)
/// 3. それ以外の米国内 -> [`UsRemoteState`](ShippingRegion::UsRemoteState)
///
/// # Arguments
///
/// * `address` - 分類対象の住所
///
/// # Returns
///
/// 配送地域を表す [`ShippingRegion`]
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::classify_shipping_region;
/// use order_taking_sample::compound_types::Address;
///
/// // カリフォルニア州は近隣州
/// let ca_address = Address::create(
///     "123 Main St", "", "", "", "Los Angeles", "90001", "CA", "US"
/// ).unwrap();
/// let region = classify_shipping_region(&ca_address);
/// assert!(region.is_us_local_state());
///
/// // ニューヨーク州は遠方州
/// let ny_address = Address::create(
///     "456 Broadway", "", "", "", "New York", "10001", "NY", "US"
/// ).unwrap();
/// let region = classify_shipping_region(&ny_address);
/// assert!(region.is_us_remote_state());
///
/// // 日本は国際（UsStateCode には有効な米国州コードが必要だが、country で判定される）
/// let jp_address = Address::create(
///     "1-1-1 Shibuya", "", "", "", "Tokyo", "15000", "CA", "Japan"
/// ).unwrap();
/// let region = classify_shipping_region(&jp_address);
/// assert!(region.is_international());
/// ```
#[must_use]
pub fn classify_shipping_region(address: &Address) -> ShippingRegion {
    let country = address.country().value();
    let state = address.state().value();

    // 国が米国でない場合は国際配送
    let is_us = US_COUNTRY_NAMES.contains(&country);
    if !is_us {
        return ShippingRegion::International;
    }

    // 近隣州かどうかを判定
    let is_local = US_LOCAL_STATES.contains(&state);
    if is_local {
        ShippingRegion::UsLocalState
    } else {
        ShippingRegion::UsRemoteState
    }
}

// =============================================================================
// calculate_shipping_cost 関数
// =============================================================================

/// 価格計算済み注文から配送コストを計算する
///
/// 配送先住所の地域分類に基づいて、配送コストを決定する。
///
/// # 配送コスト
///
/// | 地域 | コスト |
/// |------|--------|
/// | 米国近隣州 (CA, OR, AZ, NV) | $5 |
/// | 米国遠方州 | $10 |
/// | 国際 | $20 |
///
/// # Arguments
///
/// * `priced_order` - 価格計算済み注文
///
/// # Returns
///
/// 配送コストを表す [`Price`]
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::calculate_shipping_cost;
/// use order_taking_sample::workflow::{PricedOrder, PricingMethod};
/// use order_taking_sample::compound_types::{CustomerInfo, Address};
/// use order_taking_sample::simple_types::{OrderId, BillingAmount};
/// use rust_decimal::Decimal;
///
/// let order_id = OrderId::create("OrderId", "order-001").unwrap();
/// let customer = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
/// let address = Address::create(
///     "123 Main St", "", "", "", "Los Angeles", "90001", "CA", "US"
/// ).unwrap();
/// let amount = BillingAmount::create(Decimal::from(100)).unwrap();
///
/// let priced_order = PricedOrder::new(
///     order_id, customer, address.clone(), address, amount, vec![], PricingMethod::Standard
/// );
///
/// let cost = calculate_shipping_cost(&priced_order);
/// assert_eq!(cost.value(), Decimal::from(5)); // CA is UsLocalState
/// ```
#[must_use]
pub fn calculate_shipping_cost(priced_order: &PricedOrder) -> Price {
    let region = classify_shipping_region(priced_order.shipping_address());

    let cost = match region {
        ShippingRegion::UsLocalState => LOCAL_STATE_SHIPPING_COST,
        ShippingRegion::UsRemoteState => REMOTE_STATE_SHIPPING_COST,
        ShippingRegion::International => INTERNATIONAL_SHIPPING_COST,
    };

    Price::unsafe_create(rust_decimal::Decimal::from(cost))
}

// =============================================================================
// add_shipping_info_to_order 関数
// =============================================================================

/// 価格計算済み注文に配送情報を追加する
///
/// 引数で渡された配送コスト計算関数を使用して、
/// 配送情報を含む新しい注文を生成する。
///
/// # 配送方法
///
/// デフォルトで `Fedex24`（24時間配送）が使用される。
///
/// # Arguments
///
/// * `calculate_shipping_cost_function` - 配送コストを計算する関数
/// * `priced_order` - 価格計算済み注文
///
/// # Returns
///
/// 配送情報付きの [`PricedOrderWithShippingMethod`]
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{
///     add_shipping_info_to_order, calculate_shipping_cost,
///     PricedOrder, PricingMethod,
/// };
/// use order_taking_sample::compound_types::{CustomerInfo, Address};
/// use order_taking_sample::simple_types::{OrderId, BillingAmount};
/// use rust_decimal::Decimal;
///
/// let order_id = OrderId::create("OrderId", "order-001").unwrap();
/// let customer = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
/// let address = Address::create(
///     "123 Main St", "", "", "", "New York", "10001", "NY", "US"
/// ).unwrap();
/// let amount = BillingAmount::create(Decimal::from(100)).unwrap();
///
/// let priced_order = PricedOrder::new(
///     order_id, customer, address.clone(), address, amount, vec![], PricingMethod::Standard
/// );
///
/// let order_with_shipping = add_shipping_info_to_order(&calculate_shipping_cost, &priced_order);
/// assert!(order_with_shipping.shipping_info().shipping_method().is_fedex24());
/// assert_eq!(order_with_shipping.shipping_info().shipping_cost().value(), Decimal::from(10));
/// ```
#[must_use]
pub fn add_shipping_info_to_order<F>(
    calculate_shipping_cost_function: &F,
    priced_order: &PricedOrder,
) -> PricedOrderWithShippingMethod
where
    F: Fn(&PricedOrder) -> Price,
{
    let shipping_cost = calculate_shipping_cost_function(priced_order);
    let shipping_method = ShippingMethod::Fedex24;
    let shipping_info = ShippingInfo::new(shipping_method, shipping_cost);

    PricedOrderWithShippingMethod::new(shipping_info, priced_order.clone())
}

// =============================================================================
// free_vip_shipping 関数
// =============================================================================

/// VIP 顧客の配送料を無料にする
///
/// 顧客が VIP の場合、配送料を $0 に設定し、配送方法を `Fedex24` に変更する。
/// 通常顧客の場合は元の配送情報をそのまま保持する。
///
/// # Lens の使用
///
/// この関数は `PricedOrderWithShippingMethod::shipping_info_lens()` を使用して
/// 不変的に配送情報を更新する。
///
/// # Arguments
///
/// * `order` - 配送情報付き注文
///
/// # Returns
///
/// VIP 割引が適用された [`PricedOrderWithShippingMethod`]
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{
///     free_vip_shipping, PricedOrder, PricedOrderWithShippingMethod,
///     ShippingInfo, ShippingMethod, PricingMethod,
/// };
/// use order_taking_sample::compound_types::{CustomerInfo, Address};
/// use order_taking_sample::simple_types::{OrderId, BillingAmount, Price};
/// use rust_decimal::Decimal;
///
/// // VIP 顧客の注文を作成
/// let order_id = OrderId::create("OrderId", "order-001").unwrap();
/// let customer = CustomerInfo::create("John", "Doe", "john@example.com", "VIP").unwrap();
/// let address = Address::create(
///     "123 Main St", "", "", "", "New York", "10001", "NY", "US"
/// ).unwrap();
/// let amount = BillingAmount::create(Decimal::from(100)).unwrap();
/// let priced_order = PricedOrder::new(
///     order_id, customer, address.clone(), address, amount, vec![], PricingMethod::Standard
/// );
///
/// let shipping_info = ShippingInfo::new(
///     ShippingMethod::PostalService,
///     Price::create(Decimal::from(10)).unwrap()
/// );
/// let order = PricedOrderWithShippingMethod::new(shipping_info, priced_order);
///
/// let updated = free_vip_shipping(order);
/// assert_eq!(updated.shipping_info().shipping_cost().value(), Decimal::ZERO);
/// assert!(updated.shipping_info().shipping_method().is_fedex24());
/// ```
#[must_use]
pub fn free_vip_shipping(order: PricedOrderWithShippingMethod) -> PricedOrderWithShippingMethod {
    let vip_status = order.priced_order().customer_info().vip_status();

    match vip_status {
        VipStatus::Vip => {
            let free_shipping_info = ShippingInfo::new(
                ShippingMethod::Fedex24,
                Price::unsafe_create(rust_decimal::Decimal::ZERO),
            );
            PricedOrderWithShippingMethod::shipping_info_lens().set(order, free_shipping_info)
        }
        VipStatus::Normal => order,
    }
}

// =============================================================================
// acknowledge_order 関数
// =============================================================================

/// 注文確認メールを送信する
///
/// この関数は副作用を IO モナドでラップして返す。
/// 実際のメール送信は `run_unsafe()` が呼ばれるまで遅延される。
///
/// # 処理フロー
///
/// 1. 確認メール本文を生成する（`create_letter` 関数）
/// 2. メールアドレスと本文から `OrderAcknowledgment` を生成
/// 3. メール送信を実行する（`send_acknowledgment` 関数、IO モナド）
/// 4. 送信結果に基づいて `OrderAcknowledgmentSent` イベントを生成
///
/// # Arguments
///
/// * `create_letter` - 注文からメール本文（HTML）を生成する関数
/// * `send_acknowledgment` - 確認メールを送信する関数（IO モナドを返す）
/// * `order` - 配送情報付き注文
///
/// # Returns
///
/// `IO<Option<OrderAcknowledgmentSent>>` を返す。
/// - 送信成功時: `Some(OrderAcknowledgmentSent)` を含む IO
/// - 送信失敗時: `None` を含む IO
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{
///     acknowledge_order, PricedOrder, PricedOrderWithShippingMethod,
///     ShippingInfo, ShippingMethod, PricingMethod, HtmlString,
///     OrderAcknowledgment, SendResult,
/// };
/// use order_taking_sample::compound_types::{CustomerInfo, Address};
/// use order_taking_sample::simple_types::{OrderId, BillingAmount, Price};
/// use functional_rusty::effect::IO;
/// use rust_decimal::Decimal;
///
/// // 注文を作成
/// let order_id = OrderId::create("OrderId", "order-001").unwrap();
/// let customer = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
/// let address = Address::create(
///     "123 Main St", "", "", "", "New York", "10001", "NY", "US"
/// ).unwrap();
/// let amount = BillingAmount::create(Decimal::from(100)).unwrap();
/// let priced_order = PricedOrder::new(
///     order_id, customer, address.clone(), address, amount, vec![], PricingMethod::Standard
/// );
/// let shipping_info = ShippingInfo::new(
///     ShippingMethod::Fedex24,
///     Price::create(Decimal::from(10)).unwrap()
/// );
/// let order = PricedOrderWithShippingMethod::new(shipping_info, priced_order);
///
/// // モック関数を定義
/// let create_letter = |_: &PricedOrderWithShippingMethod| {
///     HtmlString::new("<p>Order confirmed</p>".to_string())
/// };
/// let send_acknowledgment = |_: &OrderAcknowledgment| {
///     IO::pure(SendResult::Sent)
/// };
///
/// // IO モナドを取得（まだ実行されない）
/// let io_result = acknowledge_order(&create_letter, &send_acknowledgment, &order);
///
/// // 実行して結果を取得
/// let result = io_result.run_unsafe();
/// assert!(result.is_some());
/// ```
pub fn acknowledge_order<CreateLetter, SendAcknowledgment>(
    create_letter: &CreateLetter,
    send_acknowledgment: &SendAcknowledgment,
    order: &PricedOrderWithShippingMethod,
) -> IO<Option<OrderAcknowledgmentSent>>
where
    CreateLetter: Fn(&PricedOrderWithShippingMethod) -> HtmlString,
    SendAcknowledgment: Fn(&OrderAcknowledgment) -> IO<SendResult>,
{
    // 確認メール本文を生成
    let letter = create_letter(order);

    // メールアドレスを取得
    let email_address = order.priced_order().customer_info().email_address().clone();

    // OrderAcknowledgment を生成
    let acknowledgment = OrderAcknowledgment::new(email_address.clone(), letter);

    // 注文 ID を取得
    let order_id = order.priced_order().order_id().clone();

    // メール送信を IO でラップ
    let send_result_io = send_acknowledgment(&acknowledgment);

    // 送信結果に基づいて OrderAcknowledgmentSent イベントを生成
    send_result_io.fmap(move |send_result| match send_result {
        SendResult::Sent => Some(OrderAcknowledgmentSent::new(order_id, email_address)),
        SendResult::NotSent => None,
    })
}

// =============================================================================
// acknowledge_order_with_logging 関数
// =============================================================================

/// 注文確認メールを送信する（ログ出力付き）
///
/// eff! マクロを使用して複数の IO 操作（ログ出力、メール送信）を
/// do 記法スタイルでチェーンする。
///
/// # 処理フロー
///
/// 1. "Creating acknowledgment letter" をログ出力
/// 2. 確認メール本文を生成（純粋）
/// 3. "Sending acknowledgment email" をログ出力
/// 4. メール送信を実行
/// 5. "Acknowledgment process completed" をログ出力
/// 6. 送信結果に基づいて `OrderAcknowledgmentSent` イベントを生成
///
/// # eff! マクロの構文
///
/// - `_ <= io_action;` - IO を実行して結果を無視
/// - `pattern <= io_action;` - IO から値を取り出して束縛
/// - `let pattern = expr;` - 純粋な値の束縛
/// - `IO::pure(...)` - 最終的な IO を返す
///
/// # Type Parameters
///
/// * `CreateLetter` - 確認メール本文を生成する関数型
/// * `SendAcknowledgment` - メールを送信する関数型（IO モナドを返す）
/// * `LogAction` - ログを出力する関数型（IO モナドを返す）
///
/// # Arguments
///
/// * `create_letter` - 確認メール本文生成関数
/// * `send_acknowledgment` - メール送信関数
/// * `log_action` - ログ出力関数
/// * `order` - 配送情報付き注文
///
/// # Returns
///
/// `IO<Option<OrderAcknowledgmentSent>>` - 遅延実行される結果
///
/// # Examples
///
/// ```
/// use order_taking_sample::workflow::{
///     acknowledge_order_with_logging, PricedOrder, PricedOrderWithShippingMethod,
///     PricingMethod, ShippingInfo, ShippingMethod, HtmlString,
///     OrderAcknowledgment, SendResult,
/// };
/// use order_taking_sample::compound_types::{CustomerInfo, Address};
/// use order_taking_sample::simple_types::{OrderId, BillingAmount, Price};
/// use functional_rusty::effect::IO;
/// use rust_decimal::Decimal;
///
/// let log_action = |message: &str| {
///     let message = message.to_string();
///     IO::new(move || println!("[LOG] {}", message))
/// };
///
/// let create_letter = |_: &PricedOrderWithShippingMethod| {
///     HtmlString::new("<p>Order confirmed</p>".to_string())
/// };
///
/// let send_acknowledgment = |_: &OrderAcknowledgment| {
///     IO::pure(SendResult::Sent)
/// };
///
/// // テストデータ作成は省略
/// // let io_result = acknowledge_order_with_logging(
/// //     &create_letter, &send_acknowledgment, &log_action, &order
/// // );
/// // let result = io_result.run_unsafe();
/// ```
pub fn acknowledge_order_with_logging<CreateLetter, SendAcknowledgment, LogAction>(
    create_letter: &CreateLetter,
    send_acknowledgment: &SendAcknowledgment,
    log_action: &LogAction,
    order: &PricedOrderWithShippingMethod,
) -> IO<Option<OrderAcknowledgmentSent>>
where
    CreateLetter: Fn(&PricedOrderWithShippingMethod) -> HtmlString,
    SendAcknowledgment: Fn(&OrderAcknowledgment) -> IO<SendResult>,
    LogAction: Fn(&str) -> IO<()>,
{
    // 事前に必要な値をクロージャ用にクローン
    let email_address = order.priced_order().customer_info().email_address().clone();
    let order_id = order.priced_order().order_id().clone();

    // ログ出力用の IO を事前に生成
    let log_creating = log_action("Creating acknowledgment letter");
    let letter = create_letter(order);
    let acknowledgment = OrderAcknowledgment::new(email_address.clone(), letter);
    let log_sending = log_action("Sending acknowledgment email");
    let send_io = send_acknowledgment(&acknowledgment);
    let log_completed = log_action("Acknowledgment process completed");

    // eff! マクロで IO 操作をチェーン
    functional_rusty::eff! {
        _ <= log_creating;
        _ <= log_sending;
        result <= send_io;
        _ <= log_completed;
        IO::pure(match result {
            SendResult::Sent => Some(OrderAcknowledgmentSent::new(order_id, email_address)),
            SendResult::NotSent => None,
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compound_types::CustomerInfo;
    use crate::simple_types::{BillingAmount, OrderId};
    use crate::workflow::PricingMethod;
    use rstest::rstest;
    use rust_decimal::Decimal;

    // =========================================================================
    // テストヘルパー
    // =========================================================================

    fn create_test_address(country: &str, state: &str) -> Address {
        Address::create("123 Main St", "", "", "", "City", "12345", state, country).unwrap()
    }

    fn create_test_priced_order(country: &str, state: &str, vip_status: &str) -> PricedOrder {
        let order_id = OrderId::create("OrderId", "order-001").unwrap();
        let customer_info =
            CustomerInfo::create("John", "Doe", "john@example.com", vip_status).unwrap();
        let address = create_test_address(country, state);
        let amount_to_bill = BillingAmount::create(Decimal::from(100)).unwrap();

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

    // =========================================================================
    // ShippingRegion のテスト
    // =========================================================================

    #[rstest]
    fn test_shipping_region_variants() {
        assert!(ShippingRegion::UsLocalState.is_us_local_state());
        assert!(!ShippingRegion::UsLocalState.is_us_remote_state());
        assert!(!ShippingRegion::UsLocalState.is_international());

        assert!(!ShippingRegion::UsRemoteState.is_us_local_state());
        assert!(ShippingRegion::UsRemoteState.is_us_remote_state());
        assert!(!ShippingRegion::UsRemoteState.is_international());

        assert!(!ShippingRegion::International.is_us_local_state());
        assert!(!ShippingRegion::International.is_us_remote_state());
        assert!(ShippingRegion::International.is_international());
    }

    // =========================================================================
    // classify_shipping_region のテスト
    // =========================================================================

    #[rstest]
    #[case("US", "CA", ShippingRegion::UsLocalState)]
    #[case("US", "OR", ShippingRegion::UsLocalState)]
    #[case("US", "AZ", ShippingRegion::UsLocalState)]
    #[case("US", "NV", ShippingRegion::UsLocalState)]
    #[case("USA", "CA", ShippingRegion::UsLocalState)]
    #[case("US", "NY", ShippingRegion::UsRemoteState)]
    #[case("US", "TX", ShippingRegion::UsRemoteState)]
    #[case("USA", "FL", ShippingRegion::UsRemoteState)]
    // 国際配送テストでは有効な州コード（NY）を使用し、国を変更してテスト
    #[case("Japan", "NY", ShippingRegion::International)]
    #[case("Canada", "NY", ShippingRegion::International)]
    #[case("UK", "NY", ShippingRegion::International)]
    fn test_classify_shipping_region(
        #[case] country: &str,
        #[case] state: &str,
        #[case] expected: ShippingRegion,
    ) {
        let address = create_test_address(country, state);
        let region = classify_shipping_region(&address);
        assert_eq!(region, expected);
    }

    // =========================================================================
    // calculate_shipping_cost のテスト
    // =========================================================================

    #[rstest]
    #[case("US", "CA", 5)]
    #[case("US", "NY", 10)]
    // 国際配送テストでは有効な州コード（NY）を使用し、国を変更してテスト
    #[case("Japan", "NY", 20)]
    fn test_calculate_shipping_cost(
        #[case] country: &str,
        #[case] state: &str,
        #[case] expected_cost: u32,
    ) {
        let priced_order = create_test_priced_order(country, state, "Normal");
        let cost = calculate_shipping_cost(&priced_order);
        assert_eq!(cost.value(), Decimal::from(expected_cost));
    }

    // =========================================================================
    // add_shipping_info_to_order のテスト
    // =========================================================================

    #[rstest]
    fn test_add_shipping_info_to_order() {
        let priced_order = create_test_priced_order("US", "NY", "Normal");
        let order_with_shipping =
            add_shipping_info_to_order(&calculate_shipping_cost, &priced_order);

        assert!(
            order_with_shipping
                .shipping_info()
                .shipping_method()
                .is_fedex24()
        );
        assert_eq!(
            order_with_shipping.shipping_info().shipping_cost().value(),
            Decimal::from(10)
        );
        assert_eq!(
            order_with_shipping.priced_order().order_id().value(),
            "order-001"
        );
    }

    // =========================================================================
    // free_vip_shipping のテスト
    // =========================================================================

    #[rstest]
    fn test_free_vip_shipping_for_vip() {
        let priced_order = create_test_priced_order("US", "NY", "VIP");
        let shipping_info = ShippingInfo::new(
            ShippingMethod::PostalService,
            Price::unsafe_create(Decimal::from(10)),
        );
        let order = PricedOrderWithShippingMethod::new(shipping_info, priced_order);

        let updated = free_vip_shipping(order);

        assert_eq!(
            updated.shipping_info().shipping_cost().value(),
            Decimal::ZERO
        );
        assert!(updated.shipping_info().shipping_method().is_fedex24());
    }

    #[rstest]
    fn test_free_vip_shipping_for_normal() {
        let priced_order = create_test_priced_order("US", "NY", "Normal");
        let shipping_info = ShippingInfo::new(
            ShippingMethod::PostalService,
            Price::unsafe_create(Decimal::from(10)),
        );
        let order = PricedOrderWithShippingMethod::new(shipping_info, priced_order);

        let updated = free_vip_shipping(order);

        assert_eq!(
            updated.shipping_info().shipping_cost().value(),
            Decimal::from(10)
        );
        assert!(
            updated
                .shipping_info()
                .shipping_method()
                .is_postal_service()
        );
    }

    // =========================================================================
    // acknowledge_order のテスト
    // =========================================================================

    #[rstest]
    fn test_acknowledge_order_sent() {
        let priced_order = create_test_priced_order("US", "NY", "Normal");
        let shipping_info = ShippingInfo::new(
            ShippingMethod::Fedex24,
            Price::unsafe_create(Decimal::from(10)),
        );
        let order = PricedOrderWithShippingMethod::new(shipping_info, priced_order);

        let create_letter =
            |_: &PricedOrderWithShippingMethod| HtmlString::new("<p>Test</p>".to_string());
        let send_acknowledgment = |_: &OrderAcknowledgment| IO::pure(SendResult::Sent);

        let io_result = acknowledge_order(&create_letter, &send_acknowledgment, &order);
        let result = io_result.run_unsafe();

        assert!(result.is_some());
        let event = result.unwrap();
        assert_eq!(event.order_id().value(), "order-001");
        assert_eq!(event.email_address().value(), "john@example.com");
    }

    #[rstest]
    fn test_acknowledge_order_not_sent() {
        let priced_order = create_test_priced_order("US", "NY", "Normal");
        let shipping_info = ShippingInfo::new(
            ShippingMethod::Fedex24,
            Price::unsafe_create(Decimal::from(10)),
        );
        let order = PricedOrderWithShippingMethod::new(shipping_info, priced_order);

        let create_letter =
            |_: &PricedOrderWithShippingMethod| HtmlString::new("<p>Test</p>".to_string());
        let send_acknowledgment = |_: &OrderAcknowledgment| IO::pure(SendResult::NotSent);

        let io_result = acknowledge_order(&create_letter, &send_acknowledgment, &order);
        let result = io_result.run_unsafe();

        assert!(result.is_none());
    }
}
