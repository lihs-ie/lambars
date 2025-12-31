//! API モジュール
//!
//! HTTP API のエントリーポイントとなる関数・型を定義する。
//!
//! # モジュール構成
//!
//! - [`types`] - HTTP リクエスト/レスポンス型
//! - [`dependencies`] - ダミー依存関数
//! - [`place_order_api`] - `PlaceOrder` API エンドポイント
//! - [`axum_handler`] - axum フレームワーク用ハンドラ
//!
//! # 設計原則
//!
//! - 全ての API 関数は `IO<HttpResponse>` を返す
//! - 依存関数は引数として注入（テスタビリティ向上）
//! - DTO とドメイン型の変換は純粋関数
//! - axum ハンドラは「世界の端」として IO の `run_unsafe()` を呼び出す

pub mod axum_handler;
pub mod dependencies;
pub mod place_order_api;
pub mod types;

// 再エクスポート
pub use dependencies::{
    calculate_shipping_cost, check_address_exists, check_product_exists,
    create_acknowledgment_letter, get_pricing_function, send_acknowledgment,
};
pub use place_order_api::place_order_api;
pub use types::{HttpRequest, HttpResponse};
