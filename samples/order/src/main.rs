//! order-taking-server
//!
//! HTTP server for order processing using axum.
//!
//! # Overview
//!
//! This server exposes the `PlaceOrder` workflow as an HTTP API.
//! It integrates with the axum async/await environment while maintaining the existing IO monad pattern.
//!
//! # Endpoints
//!
//! - `POST /place-order` - Processes an order and returns events
//!
//! # Usage
//!
//! ```bash
//! # Start the server
//! cargo run --bin order-taking-server
//!
//! # Send a request
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
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "order_taking_server=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Build router
    let app = Router::new().route("/place-order", post(place_order_handler));

    // Start server
    let address = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("Starting server on {}", address);

    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
