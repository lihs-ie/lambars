//! DTO (Data Transfer Object) モジュール
//!
//! API との境界で使用するシリアライズ/デシリアライズ可能な型を定義する。
//! ドメイン型との相互変換関数も提供する。
//!
//! # モジュール構成
//!
//! - [`input`] - 入力 DTO（`OrderFormDto` 等）
//! - [`output`] - 出力 DTO（`PlaceOrderEventDto` 等）
//! - [`error`] - エラー DTO（`PlaceOrderErrorDto`）
//!
//! # 設計原則
//!
//! - 全ての DTO は `Serialize` と `Deserialize` を実装
//! - ドメイン型への変換は純粋関数として実装
//! - Decimal は文字列としてシリアライズ（精度保持のため）

pub mod error;
pub mod input;
pub mod output;

// 再エクスポート
pub use error::PlaceOrderErrorDto;
pub use input::{AddressDto, CustomerInfoDto, OrderFormDto, OrderFormLineDto};
pub use output::{
    BillableOrderPlacedDto, OrderAcknowledgmentSentDto, PlaceOrderEventDto, PricedOrderLineDto,
    PricedOrderProductLineDto, ShippableOrderLineDto, ShippableOrderPlacedDto,
};
