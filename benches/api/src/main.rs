//! Task Management Benchmark API
//!
//! A benchmark application demonstrating the lambars library
//! with task management functionality.
//!
//! # Environment Variables
//!
//! - `STORAGE_MODE`: `in_memory` (default) | `postgres`
//! - `CACHE_MODE`: `in_memory` (default) | `redis`
//! - `DATABASE_URL`: `PostgreSQL` connection URL (required when `STORAGE_MODE=postgres`)
//! - `REDIS_URL`: Redis connection URL (required when `CACHE_MODE=redis`)
//! - `RUST_LOG`: Logging level (e.g., `debug`, `info`, `task_management_benchmark_api=debug`)
//! - `HOST`: Server host address (default: `0.0.0.0`)
//! - `PORT`: Server port (default: `3000`)

use std::env;
use std::net::SocketAddr;

use axum::Router;
use axum::routing::{get, patch, post, put};
use tokio::net::TcpListener;
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use task_management_benchmark_api::api::{
    AppState, add_subtask, add_tag, async_pipeline, bulk_create_tasks, bulk_update_tasks,
    count_by_priority, create_project_handler, create_task, create_task_eff,
    execute_state_workflow, execute_workflow, flatten_demo, functor_mut_demo, get_project_handler,
    get_project_progress_handler, get_project_stats_handler, get_task_history, health_check,
    identity_demo, lazy_compute, list_tasks, monad_error_demo, monad_transformers, search_tasks,
    transform_task, update_status, update_task, update_with_optics,
};
use task_management_benchmark_api::infrastructure::{RepositoryConfig, RepositoryFactory};

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() {
    // Load .env file if present (for development)
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "task_management_benchmark_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Task Management Benchmark API");

    // Initialize configuration from environment
    let config = match RepositoryConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            tracing::error!("Configuration error: {}", error);
            std::process::exit(1);
        }
    };

    tracing::info!(
        storage_mode = ?config.storage_mode,
        cache_mode = ?config.cache_mode,
        "Repository configuration loaded"
    );

    // Create repository factory and initialize repositories
    let factory = RepositoryFactory::new(config);
    let repositories = match factory.create().await {
        Ok(repositories) => {
            tracing::info!("Repositories initialized successfully");
            repositories
        }
        Err(error) => {
            tracing::error!("Failed to initialize repositories: {}", error);
            std::process::exit(1);
        }
    };

    // Create application state
    let application_state = AppState::from_repositories(repositories);

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build the application router
    let application = Router::new()
        .route("/health", get(health_check))
        // Task CRUD
        // Task queries
        .route("/tasks", get(list_tasks).post(create_task))
        .route("/tasks/search", get(search_tasks))
        .route("/tasks/by-priority", get(count_by_priority))
        // Task mutations
        .route("/tasks-eff", post(create_task_eff))
        .route("/tasks/{id}", put(update_task))
        .route("/tasks/{id}/status", patch(update_status))
        .route("/tasks/{id}/subtasks", post(add_subtask))
        .route("/tasks/{id}/tags", post(add_tag))
        // Bulk operations
        .route(
            "/tasks/bulk",
            post(bulk_create_tasks).put(bulk_update_tasks),
        )
        // Project operations
        .route("/projects", post(create_project_handler))
        .route("/projects/{id}", get(get_project_handler))
        .route("/projects/{id}/progress", get(get_project_progress_handler))
        .route("/projects/{id}/stats", get(get_project_stats_handler))
        // Advanced operations
        .route("/tasks/{id}/history", get(get_task_history))
        .route("/tasks/transform", post(transform_task))
        .route("/tasks/async-pipeline", post(async_pipeline))
        .route("/tasks/lazy-compute", post(lazy_compute))
        // Effects and optics operations
        .route("/tasks/workflow", post(execute_workflow))
        .route("/tasks/{id}/optics", put(update_with_optics))
        .route("/tasks/state-workflow", post(execute_state_workflow))
        // Type class demonstrations
        .route("/tasks/monad-transformers", post(monad_transformers))
        .route("/tasks/functor-mut", post(functor_mut_demo))
        .route("/tasks/flatten", post(flatten_demo))
        .route("/tasks/monad-error", post(monad_error_demo))
        .route("/tasks/identity-type", post(identity_demo))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(application_state);

    // Parse server address from environment
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|port| port.parse().ok())
        .unwrap_or(3000);

    let address: SocketAddr = match format!("{host}:{port}").parse() {
        Ok(address) => address,
        Err(error) => {
            tracing::error!(%error, "Invalid server address: {}:{}", host, port);
            std::process::exit(1);
        }
    };

    // Start the server
    let listener = match TcpListener::bind(address).await {
        Ok(listener) => listener,
        Err(error) => {
            tracing::error!(%error, "Failed to bind to address {}", address);
            std::process::exit(1);
        }
    };

    match listener.local_addr() {
        Ok(address) => tracing::info!("Listening on {}", address),
        Err(error) => tracing::warn!(%error, "Could not determine local address"),
    }

    if let Err(error) = axum::serve(listener, application)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        tracing::error!(%error, "Server error");
        std::process::exit(1);
    }

    tracing::info!("Server shutdown complete");
}

/// Handles graceful shutdown signals (SIGINT, SIGTERM).
///
/// This function returns a future that completes when a shutdown signal
/// is received. On Unix systems, it listens for both SIGINT (Ctrl+C) and
/// SIGTERM. On other systems, it only listens for Ctrl+C.
async fn shutdown_signal() {
    let ctrl_c = async {
        match signal::ctrl_c().await {
            Ok(()) => {}
            Err(error) => {
                tracing::warn!(%error, "Failed to install Ctrl+C handler");
                // Fall through to wait for SIGTERM or never terminate
            }
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::warn!(%error, "Failed to install SIGTERM handler");
                // Wait forever if SIGTERM handler fails
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {
            tracing::info!("Received Ctrl+C, initiating graceful shutdown");
        }
        () = terminate => {
            tracing::info!("Received SIGTERM, initiating graceful shutdown");
        }
    }
}
