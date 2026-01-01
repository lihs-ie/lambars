//! 注文ドメインで使用する基本型（Simple Types）
//!
//! F# の Single Case Discriminated Union パターンを Rust の newtype パターンで実現し、
//! 不正な状態を型レベルで防ぐ「Make Illegal States Unrepresentable」の原則に従う。
//!
//! # 概要
//!
//! このモジュールは、注文処理ドメインで使用する基本型を提供する。
//! 各型は Smart Constructor パターンを使用し、バリデーション済みの値のみを保持することを保証する。
//!
//! # 型カテゴリ
//!
//! - **文字列制約型**: `String50`, `EmailAddress`, `ZipCode`, `UsStateCode`
//! - **ID 型**: `OrderId`, `OrderLineId`
//! - **製品コード型**: `WidgetCode`, `GizmoCode`, `ProductCode`
//! - **数量型**: `UnitQuantity`, `KilogramQuantity`, `OrderQuantity`
//! - **金額型**: `Price`, `BillingAmount`
//! - **その他**: `VipStatus`, `PromotionCode`, `PdfAttachment`
//!
//! # 使用例
//!
//! ```
//! use order_taking_sample::simple_types::{
//!     OrderId, ProductCode, Price, BillingAmount
//! };
//! use rust_decimal::Decimal;
//! use std::str::FromStr;
//!
//! // OrderId の生成（バリデーション付き）
//! let order_id = OrderId::create("OrderId", "ORD-2024-001").unwrap();
//! assert_eq!(order_id.value(), "ORD-2024-001");
//!
//! // ProductCode の生成（Widget または Gizmo を自動判定）
//! let widget = ProductCode::create("ProductCode", "W1234").unwrap();
//! let gizmo = ProductCode::create("ProductCode", "G123").unwrap();
//!
//! // Price の生成と合計計算
//! let price1 = Price::create(Decimal::from_str("100.00").unwrap()).unwrap();
//! let price2 = Price::create(Decimal::from_str("200.00").unwrap()).unwrap();
//! let total = BillingAmount::sum_prices(&[price1, price2]).unwrap();
//! assert_eq!(total.value(), Decimal::from_str("300.00").unwrap());
//! ```
//!
//! # lambars との統合
//!
//! このモジュールは lambars ライブラリの機能を活用している:
//!
//! - `Result<T, ValidationError>` による Monad 的なエラーハンドリング
//! - `Foldable` トレイトを使用した `BillingAmount::sum_prices` の畳み込み操作

pub mod constrained_type;
mod error;
mod identifier_types;
mod misc_types;
mod price_types;
mod product_types;
mod quantity_types;
mod string_types;

// =============================================================================
// 型の再エクスポート
// =============================================================================

// エラー型
pub use error::ValidationError;

// 文字列型
pub use string_types::{EmailAddress, String50, UsStateCode, ZipCode};

// ID 型
pub use identifier_types::{OrderId, OrderLineId};

// 製品コード型
pub use product_types::{GizmoCode, ProductCode, WidgetCode};

// 数量型
pub use quantity_types::{KilogramQuantity, OrderQuantity, UnitQuantity};

// 金額型
pub use price_types::{BillingAmount, Price};

// その他の型
pub use misc_types::{PdfAttachment, PromotionCode, VipStatus};
