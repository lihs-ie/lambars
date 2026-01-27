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
//! - `WORKER_THREADS`: Number of tokio worker threads (default: logical CPU count)

use std::env;
use std::net::SocketAddr;

use axum::Router;
use axum::routing::{get, patch, post, put};
use tokio::net::TcpListener;
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(feature = "demo")]
use task_management_benchmark_api::api::demo::get_task_history_demo;
use task_management_benchmark_api::api::{
    AppState, add_subtask, add_tag, aggregate_numeric, aggregate_sources, aggregate_tree,
    async_pipeline, batch_process_async, batch_transform_results, batch_update_field,
    build_from_parts, bulk_create_tasks, bulk_update_tasks, collect_optional, compute_parallel,
    concurrent_lazy, conditional_pipeline, convert_error_domain, count_by_priority,
    create_project_handler, create_task, create_task_eff, dashboard, delete_task, deque_operations,
    enrich_batch, enrich_error, execute_sequential, execute_state_workflow, execute_workflow,
    fetch_batch, filter_conditional, first_available, flatten_demo, flatten_subtasks,
    freer_workflow, functor_mut_demo, get_project_handler, get_project_progress_handler,
    get_project_stats_handler, get_task, get_task_history, health_check, identity_demo,
    lazy_compute, list_tasks, monad_error_demo, monad_transformers, nested_access, partial_apply,
    process_with_error_transform, projects_leaderboard, resolve_config, resolve_dependencies,
    search_fallback, search_tasks, tasks_by_deadline, tasks_timeline, transform_async,
    transform_pair, transform_task, update_filtered, update_metadata_key, update_optional,
    update_status, update_task, update_with_optics, validate_batch, validate_collect_all,
    workflow_async,
};
use task_management_benchmark_api::infrastructure::{
    ExternalSources, RepositoryConfig, RepositoryFactory,
};

/// Result of parsing `WORKER_THREADS` environment variable.
struct WorkerThreadsResult {
    threads: Option<usize>,
    warning_emitted: bool,
}

fn parse_worker_threads() -> WorkerThreadsResult {
    let Ok(value) = std::env::var("WORKER_THREADS") else {
        return WorkerThreadsResult {
            threads: None,
            warning_emitted: false,
        };
    };

    let trimmed = value.trim();

    if trimmed.is_empty() {
        return WorkerThreadsResult {
            threads: None,
            warning_emitted: false,
        };
    }

    match trimmed.parse::<usize>() {
        Ok(0) => {
            eprintln!("Warning: WORKER_THREADS=0 is invalid (must be > 0), using default");
            WorkerThreadsResult {
                threads: None,
                warning_emitted: true,
            }
        }
        Ok(n) => {
            let max_threads = std::thread::available_parallelism()
                .map(|parallelism| parallelism.get().saturating_mul(4))
                .unwrap_or(64);
            if n > max_threads {
                eprintln!(
                    "Warning: WORKER_THREADS={n} exceeds recommended limit ({max_threads}), capping to {max_threads}"
                );
                WorkerThreadsResult {
                    threads: Some(max_threads),
                    warning_emitted: true,
                }
            } else {
                WorkerThreadsResult {
                    threads: Some(n),
                    warning_emitted: false,
                }
            }
        }
        Err(error) => {
            eprintln!(
                "Warning: WORKER_THREADS='{trimmed}' is not a valid number ({error}), using default"
            );
            WorkerThreadsResult {
                threads: None,
                warning_emitted: true,
            }
        }
    }
}

fn main() {
    dotenvy::dotenv().ok();

    let result = parse_worker_threads();
    let mut builder = tokio::runtime::Builder::new_multi_thread();
    builder.enable_all();

    if let Some(threads) = result.threads {
        builder.worker_threads(threads);
        if !result.warning_emitted {
            eprintln!("Tokio worker_threads set to: {threads}");
        }
    } else if !result.warning_emitted {
        eprintln!("Tokio worker_threads: using default (logical CPU count)");
    }

    let runtime = builder.build().expect("Failed to create tokio runtime");
    runtime.block_on(async_main());
}

#[allow(clippy::too_many_lines)]
async fn async_main() {
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

    // Create external sources (Redis, HTTP) from environment
    let external_sources = match ExternalSources::from_env() {
        Ok(sources) => {
            tracing::info!("External sources initialized from environment");
            sources
        }
        Err(error) => {
            // Determine if this is a critical configuration error (parse failure)
            // or a recoverable error (missing optional config)
            let is_parse_error = matches!(
                &error,
                task_management_benchmark_api::infrastructure::fail_injection::ConfigError::EnvParseError(_)
                    | task_management_benchmark_api::infrastructure::fail_injection::ConfigError::InvalidRngSeed { .. }
                    | task_management_benchmark_api::infrastructure::fail_injection::ConfigError::InvalidFailureRate(_)
                    | task_management_benchmark_api::infrastructure::fail_injection::ConfigError::InvalidTimeoutRate(_)
                    | task_management_benchmark_api::infrastructure::fail_injection::ConfigError::InvalidDelayRange { .. }
            );

            if is_parse_error {
                tracing::error!(%error, "Critical configuration error in external sources");
                std::process::exit(1);
            }

            tracing::warn!(%error, "Failed to create external sources, using stubs");
            ExternalSources::stub()
        }
    };

    // Create application state (async: initializes search index from repository)
    let application_state =
        match AppState::with_external_sources(repositories, external_sources).await {
            Ok(state) => {
                tracing::info!("Application state initialized with search index");
                state
            }
            Err(error) => {
                tracing::error!("Failed to initialize application state: {}", error);
                std::process::exit(1);
            }
        };

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
        .route(
            "/tasks/{id}",
            get(get_task).put(update_task).delete(delete_task),
        )
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
        // Recursive operations (Trampoline demonstrations)
        .route("/tasks/{id}/flatten-subtasks", get(flatten_subtasks))
        .route("/tasks/resolve-dependencies", post(resolve_dependencies))
        .route("/projects/{id}/aggregate-tree", get(aggregate_tree))
        // Ordered operations (PersistentTreeMap demonstrations)
        .route("/tasks/by-deadline", get(tasks_by_deadline))
        .route("/tasks/timeline", get(tasks_timeline))
        .route("/projects/leaderboard", get(projects_leaderboard))
        // Traversable operations (batch processing demonstrations)
        .route("/tasks/validate-batch", post(validate_batch))
        .route("/tasks/fetch-batch", post(fetch_batch))
        .route("/tasks/collect-optional", post(collect_optional))
        .route("/tasks/execute-sequential", post(execute_sequential))
        .route("/tasks/enrich-batch", post(enrich_batch))
        // Alternative operations (fallback and choice demonstrations)
        .route("/tasks/search-fallback", get(search_fallback))
        .route("/tasks/{id}/config", get(resolve_config))
        .route("/tasks/filter-conditional", post(filter_conditional))
        .route("/tasks/aggregate-sources", post(aggregate_sources))
        .route("/tasks/first-available", get(first_available))
        // pipe_async! operations (async pipeline demonstrations)
        .route("/tasks/{id}/transform-async", post(transform_async))
        .route("/tasks/workflow-async", post(workflow_async))
        .route("/tasks/batch-process-async", post(batch_process_async))
        .route(
            "/tasks/{id}/conditional-pipeline",
            post(conditional_pipeline),
        )
        // Bifunctor operations (two-parameter type transformations)
        .route(
            "/tasks/process-with-error-transform",
            post(process_with_error_transform),
        )
        .route("/tasks/transform-pair", post(transform_pair))
        .route("/tasks/enrich-error", post(enrich_error))
        .route("/tasks/convert-error-domain", post(convert_error_domain))
        .route(
            "/tasks/batch-transform-results",
            post(batch_transform_results),
        )
        // Applicative operations (independent computation combining)
        .route("/tasks/validate-collect-all", post(validate_collect_all))
        .route("/dashboard", get(dashboard))
        .route("/tasks/build-from-parts", post(build_from_parts))
        .route("/tasks/compute-parallel", post(compute_parallel))
        // Advanced Optics operations (Traversal, At, Filtered)
        .route("/tasks/batch-update-field", put(batch_update_field))
        .route("/tasks/{id}/update-optional", put(update_optional))
        .route("/projects/{id}/metadata/{key}", put(update_metadata_key))
        .route("/tasks/update-filtered", put(update_filtered))
        .route("/tasks/nested-access", get(nested_access))
        // Miscellaneous operations (partial!, ConcurrentLazy, PersistentDeque, Sum/Product, Freer)
        .route("/tasks/partial-apply", post(partial_apply))
        .route("/tasks/concurrent-lazy", post(concurrent_lazy))
        .route("/tasks/deque-operations", post(deque_operations))
        .route("/tasks/aggregate-numeric", get(aggregate_numeric))
        .route("/tasks/freer-workflow", post(freer_workflow));

    // Demo endpoints: Feature Flag (compile-time) + ENV (runtime) double-gate
    // Both conditions must be met for demo endpoints to be enabled:
    // 1. Compiled with `--features demo`
    // 2. ENABLE_DEMO_ENDPOINTS environment variable is "true", "1", or "yes"
    #[cfg(feature = "demo")]
    let application = {
        let demo_enabled = env::var("ENABLE_DEMO_ENDPOINTS")
            .map(|value| matches!(value.to_lowercase().as_str(), "true" | "1" | "yes"))
            .unwrap_or(false);

        if demo_enabled {
            tracing::info!("Demo endpoints enabled");
            application.route("/demo/tasks/{id}/history", get(get_task_history_demo))
        } else {
            application
        }
    };

    let application = application
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
