//! Bank Sample Application Entry Point
//!
//! This is the entry point for the Bank Sample Application demonstrating
//! lambars library features with Event Sourcing / CQRS patterns.

use std::sync::Arc;

use bank::api::routes::create_router;
use bank::infrastructure::{
    AppConfig, AppDependencies, InMemoryEventStore, InMemoryReadModelCache,
};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,bank=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Bank Sample Application...");

    // Load configuration
    let config = match AppConfig::from_env() {
        Ok(config) => {
            tracing::info!(
                "Configuration loaded: host={}, port={}",
                config.app_host,
                config.app_port
            );
            config
        }
        Err(e) => {
            tracing::warn!("Failed to load configuration from environment: {e}");
            tracing::info!("Using default configuration");
            AppConfig::default()
        }
    };

    let bind_address = format!("{}:{}", config.app_host, config.app_port);

    // Initialize infrastructure (using in-memory implementations for demo)
    let event_store = Arc::new(InMemoryEventStore::new());
    let read_model = Arc::new(InMemoryReadModelCache::new());

    tracing::info!("Infrastructure initialized (in-memory mode)");

    // Create dependencies container
    let deps = AppDependencies::new(config, event_store, read_model);

    // Create router with middleware
    let app = create_router(deps).layer(TraceLayer::new_for_http());

    // Start server
    let listener = TcpListener::bind(&bind_address).await.unwrap();
    tracing::info!("Bank Sample Application started on http://{bind_address}");
    tracing::info!("Available endpoints:");
    tracing::info!("  POST /accounts           - Create account");
    tracing::info!("  GET  /accounts/:id       - Get account");
    tracing::info!("  GET  /accounts/:id/balance - Get balance");
    tracing::info!("  POST /accounts/:id/deposit - Deposit");
    tracing::info!("  POST /accounts/:id/withdraw - Withdraw");
    tracing::info!("  POST /accounts/:id/transfer - Transfer");
    tracing::info!("  GET  /accounts/:id/transactions - Transaction history");
    tracing::info!("  GET  /health             - Health check");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    tracing::info!("Bank Sample Application stopped");
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler");
    tracing::info!("Shutdown signal received");
}
