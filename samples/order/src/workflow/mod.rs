//! ワークフロー型定義モジュール
//!
//! `PlaceOrder` ワークフローで使用する型を定義する。
//! 型による状態遷移を表現し、不正な状態を型レベルで防ぐ。
//!
//! # 状態遷移図
//!
//! ```text
//! UnvalidatedOrder -> ValidatedOrder -> PricedOrder -> PricedOrderWithShippingMethod -> PlaceOrderEvent[]
//! ```
//!
//! # 関数合成マクロの使い分け
//!
//! lambars は2つの関数合成マクロを提供する。
//!
//! ## pipe! マクロ
//!
//! 値を即座に変換する場合に使用（データフロースタイル）。
//!
//! ```ignore
//! // 左から右へ値が流れる
//! let result = pipe!(value, f, g, h); // h(g(f(value)))
//! ```
//!
//! **適用場面**:
//! - 一度きりの変換チェーン
//! - 中間変数を省略したい場合
//! - データの流れを左から右で表現したい場合
//!
//! ## compose! マクロ
//!
//! 再利用可能な合成関数を生成する場合に使用（関数合成スタイル）。
//!
//! ```ignore
//! // 右から左への関数合成（数学的合成）
//! let composed = compose!(h, g, f);
//! let result = composed(value); // h(g(f(value)))
//! ```
//!
//! **適用場面**:
//! - 合成関数を複数箇所で再利用する場合
//! - 合成関数に名前を付けて意図を明確化したい場合
//! - 高階関数（`map`, `filter_map` 等）に関数を渡す場合
//! - Point-free スタイルを適用したい場合
//!
//! ## 使い分けの判断基準
//!
//! | 状況 | 推奨マクロ |
//! |------|-----------|
//! | 一度きりの変換 | pipe! |
//! | 複数箇所で再利用 | compose! |
//! | 合成関数に名前を付けたい | compose! |
//! | `map`/`filter_map` に渡す | compose!（複数関数の場合） |
//! | データフローを明示したい | pipe! |
//!
//! ## 例: イベント作成での使い分け
//!
//! ```ignore
//! // compose! で再利用可能な合成関数を定義
//! let to_shipping_event = compose!(
//!     PlaceOrderEvent::ShippableOrderPlaced,
//!     create_shipping_event
//! );
//!
//! // 合成関数を適用
//! let shipping_events = vec![to_shipping_event(priced_order)];
//!
//! // pipe! で一度きりの変換（compose! を使わない場合）
//! let shipping_events = vec![pipe!(
//!     priced_order,
//!     create_shipping_event,
//!     PlaceOrderEvent::ShippableOrderPlaced
//! )];
//! ```
//!
//! # モジュール構成
//!
//! - [`error_types`] - エラー型（バリデーション、価格計算、リモートサービス）
//! - [`unvalidated_types`] - 未検証の入力型
//! - [`validated_types`] - 検証済み型
//! - [`priced_types`] - 価格付き型
//! - [`shipping_types`] - 配送関連型
//! - [`acknowledgment_types`] - 確認メール関連型
//! - [`output_types`] - 出力イベント型
//! - [`events`] - イベント生成関数
//! - [`place_order`] - ワークフロー統合関数

pub mod acknowledgment_types;
pub mod error_types;
pub mod events;
pub mod output_types;
pub mod place_order;
pub mod priced_types;
pub mod pricing;
pub mod pricing_catalog;
pub mod shipping;
pub mod shipping_types;
pub mod unvalidated_types;
pub mod validated_types;
pub mod validation;

// =============================================================================
// 型の再エクスポート
// =============================================================================

pub use acknowledgment_types::{HtmlString, OrderAcknowledgment, SendResult};
pub use error_types::{
    PlaceOrderError, PricingError, RemoteServiceError, ServiceInfo, WorkflowValidationError,
};
pub use events::{create_billing_event, create_events, create_shipping_event, make_shipment_line};
pub use output_types::{
    BillableOrderPlaced, OrderAcknowledgmentSent, PlaceOrderEvent, ShippableOrderLine,
    ShippableOrderPlaced,
};
pub use place_order::place_order;
pub use priced_types::{PricedOrder, PricedOrderLine, PricedOrderProductLine};
pub use pricing::price_order;
pub use shipping::{
    ShippingRegion, acknowledge_order, acknowledge_order_with_logging, add_shipping_info_to_order,
    calculate_shipping_cost, classify_shipping_region, free_vip_shipping,
};
pub use shipping_types::{PricedOrderWithShippingMethod, ShippingInfo, ShippingMethod};
pub use unvalidated_types::{
    UnvalidatedAddress, UnvalidatedCustomerInfo, UnvalidatedOrder, UnvalidatedOrderLine,
};
pub use validated_types::{
    AddressValidationError, CheckedAddress, PricingMethod, ValidatedOrder, ValidatedOrderLine,
};
pub use validation::validate_order;

// Phase 10: PricingCatalog
pub use pricing_catalog::{PricingCatalog, create_catalog_pricing_function};
