//! order-taking-server
//!
//! axum を使用した注文処理 HTTP サーバー。
//!
//! # 概要
//!
//! このサーバーは `PlaceOrder` ワークフローを HTTP API として公開する。
//! 既存の IO モナドパターンを維持しつつ、axum の async/await 環境と統合する。
//!
//! # エンドポイント
//!
//! - `POST /place-order` - 注文を処理し、イベントを返す
//!
//! # 使用方法
//!
//! ```bash
//! # サーバー起動
//! cargo run --bin order-taking-server
//!
//! # リクエスト送信
//! curl -X POST http://localhost:8080/place-order \
//!   -H "Content-Type: application/json" \
//!   -d '{"order_id": "order-001", ...}'
//! ```

use axum::{Router, routing::post};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use order_taking_sample::api::axum_handler::place_order_handler;

#[tokio::main]
async fn main() {
    // トレーシング初期化
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "order_taking_server=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // ルーター構築
    let app = Router::new().route("/place-order", post(place_order_handler));

    // サーバー起動
    let address = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("Starting server on {}", address);

    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
