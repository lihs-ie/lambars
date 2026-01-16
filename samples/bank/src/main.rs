//! Bank Sample Application Entry Point

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,bank=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Bank Sample Application...");

    // TODO: Initialize infrastructure
    // TODO: Start Axum server

    tracing::info!("Bank Sample Application started on http://0.0.0.0:8080");
}
