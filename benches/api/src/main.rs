//! Task Management Benchmark API
//!
//! A benchmark application demonstrating the lambars library
//! with task management functionality.

use axum::Router;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "task_management_benchmark_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Task Management Benchmark API");

    // Build the application router
    let application = Router::new();

    // Start the server
    let listener = TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to address");

    tracing::info!("Listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, application)
        .await
        .expect("Failed to start server");
}
