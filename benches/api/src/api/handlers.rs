//! HTTP handlers for the Task Management API.
//!
//! This module demonstrates the use of lambars features in HTTP handlers:
//! - Functor: Mapping over `Either` for DTO validation
//! - Monad: Chaining validation results
//! - Either: Representing validation success/failure
//! - `AsyncIO`: Encapsulating side effects
//!
//! # Note on Send bounds
//!
//! lambars' persistent data structures (`PersistentHashSet`, `PersistentList`)
//! use `Rc` internally and are not `Send`. Therefore, `Task` cannot cross
//! await boundaries. We handle this by:
//! 1. Creating/processing `Task` synchronously in a block
//! 2. Executing async operations separately
//! 3. Converting `Task` to `TaskResponse` (which is `Send`) before returning

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use arc_swap::ArcSwap;
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
};

use super::json_buffer::JsonResponse;
use lambars::control::Either;
use lambars::persistent::PersistentVector;
use uuid::Uuid;

use super::bulk::BulkConfig;
use super::cache_header::{CacheHeaderExtension, CacheSource};
use super::consistency::save_task_with_event;
use super::dto::{
    CreateTaskRequest, TaskResponse, validate_description, validate_tags, validate_title,
};
use super::error::ApiErrorResponse;
use super::query::{
    SearchCache, SearchIndex, SearchIndexConfig, TaskChange, measure_search_index_build,
};
use crate::domain::{EventId, Priority, Tag, Task, TaskId, Timestamp, create_task_created_event};
use crate::infrastructure::{
    CacheStatus, EventStore, ExternalDataSource, ExternalSources, Pagination, ProjectRepository,
    Repositories, RepositoryError, RngProvider, TaskRepository,
};

// =============================================================================
// Application Configuration
// =============================================================================

/// Application configuration for runtime settings.
///
/// This struct is used with `Reader` monad to demonstrate dependency injection
/// patterns in functional programming.
///
/// # lambars Features
///
/// - `Reader`: Configuration is accessed via `Reader<AppConfig, A>` for
///   composable dependency injection
#[derive(Clone, Debug)]
pub struct AppConfig {
    /// Maximum number of tasks allowed per project.
    pub max_tasks_per_project: usize,
    /// Default page size for pagination.
    pub default_page_size: u32,
}

/// Applied configuration values at runtime (ENV-REQ-030).
///
/// This struct stores the *actual* values applied at startup, not raw environment
/// variable strings. This ensures `/debug/config` returns accurate values that
/// reflect caps, defaults, and validation.
///
/// # Fields
///
/// - `worker_threads`: The actual worker thread count used by Tokio runtime
///   (after validation, caps, and defaults applied)
/// - `database_pool_size`: The actual database pool size (`None` means library default)
/// - `redis_pool_size`: The actual Redis pool size (`None` means library default)
/// - `storage_mode`: Storage backend mode (`in_memory` or `postgres`)
/// - `cache_mode`: Cache backend mode (`in_memory` or `redis`)
#[derive(Clone, Debug)]
pub struct AppliedConfig {
    /// Actual Tokio worker threads count (after caps/defaults).
    pub worker_threads: Option<usize>,
    /// Actual database pool size (`None` = library default).
    pub database_pool_size: Option<u32>,
    /// Actual Redis pool size (`None` = library default).
    pub redis_pool_size: Option<u32>,
    /// Storage mode as env-compatible string (`in_memory`, `postgres`).
    pub storage_mode: String,
    /// Cache mode as env-compatible string (`in_memory`, `redis`).
    pub cache_mode: String,
}

impl Default for AppliedConfig {
    fn default() -> Self {
        Self {
            worker_threads: std::thread::available_parallelism()
                .map(std::num::NonZero::get)
                .ok(),
            database_pool_size: None,
            redis_pool_size: None,
            storage_mode: "in_memory".to_string(),
            cache_mode: "in_memory".to_string(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            max_tasks_per_project: 100,
            default_page_size: 20,
        }
    }
}

// =============================================================================
// Application State
// =============================================================================

/// Shared application dependencies.
///
/// Uses trait objects (`dyn`) instead of generics to work seamlessly with
/// the `RepositoryFactory` which returns trait objects. This design allows
/// runtime selection of repository backends (in-memory, `PostgreSQL`, Redis).
///
/// # Search Index
///
/// The `search_index` field holds an immutable `SearchIndex` wrapped in `ArcSwap`.
/// This allows lock-free reads during search operations while supporting
/// atomic updates when tasks are created/updated/deleted.
///
/// - **Read**: `state.search_index.load()` returns a `Guard<Arc<SearchIndex>>`
/// - **Write**: `state.search_index.store(Arc::new(new_index))` atomically replaces the index
///
/// # Bulk Configuration
///
/// The `bulk_config` field holds configuration for bulk operations. This is loaded
/// from environment variables at application startup to ensure referential transparency
/// in handlers (I/O is isolated at the application boundary).
pub struct AppState {
    /// Task repository for persistence.
    pub task_repository: Arc<dyn TaskRepository + Send + Sync>,
    /// Project repository for project operations.
    pub project_repository: Arc<dyn ProjectRepository + Send + Sync>,
    /// Event store for event sourcing.
    pub event_store: Arc<dyn EventStore + Send + Sync>,
    /// Application configuration.
    pub config: AppConfig,
    /// Bulk operation configuration (chunk size, concurrency, feature flags).
    ///
    /// Loaded from environment variables at startup to isolate I/O at application boundary.
    pub bulk_config: BulkConfig,
    /// Search index for task search (lock-free reads via `ArcSwap`).
    ///
    /// This index is built once at startup and updated incrementally
    /// when tasks are created, updated, or deleted.
    pub search_index: Arc<ArcSwap<SearchIndex>>,
    /// Search result cache (TTL 5s, LRU 2000 entries).
    ///
    /// Caches search results to improve performance for repeated queries.
    /// The cache key is `(normalized_query, scope, limit, offset)`.
    pub search_cache: Arc<SearchCache>,
    /// Secondary data source (Redis) for Alternative patterns.
    ///
    /// Used by `aggregate_sources` and `search_fallback` handlers.
    pub secondary_source: Arc<dyn ExternalDataSource + Send + Sync>,
    /// External data source (HTTP) for Alternative patterns.
    ///
    /// Used by `aggregate_sources` and `search_fallback` handlers.
    pub external_source: Arc<dyn ExternalDataSource + Send + Sync>,
    /// RNG provider for fail injection.
    ///
    /// Enables deterministic behavior for testing/benchmarks when seeded.
    pub rng_provider: Arc<RngProvider>,
    /// Cache hit counter (CACHE-REQ-021).
    pub cache_hits: Arc<AtomicU64>,
    /// Cache miss counter (CACHE-REQ-021).
    pub cache_misses: Arc<AtomicU64>,
    /// Cache error counter (CACHE-REQ-021, fail-open).
    pub cache_errors: Arc<AtomicU64>,
    /// Cache strategy name (CACHE-REQ-021).
    pub cache_strategy: String,
    /// Cache TTL in seconds (CACHE-REQ-021).
    pub cache_ttl_seconds: u64,
    /// Applied configuration values (ENV-REQ-030).
    ///
    /// Stores actual runtime configuration for `/debug/config` endpoint.
    pub applied_config: AppliedConfig,
    /// Counter for RCU retry events due to CAS failure (testing/monitoring).
    pub search_index_rcu_retries: Arc<AtomicUsize>,
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            task_repository: Arc::clone(&self.task_repository),
            project_repository: Arc::clone(&self.project_repository),
            event_store: Arc::clone(&self.event_store),
            config: self.config.clone(),
            bulk_config: self.bulk_config,
            search_index: Arc::clone(&self.search_index),
            search_cache: Arc::clone(&self.search_cache),
            secondary_source: Arc::clone(&self.secondary_source),
            external_source: Arc::clone(&self.external_source),
            rng_provider: Arc::clone(&self.rng_provider),
            cache_hits: Arc::clone(&self.cache_hits),
            cache_misses: Arc::clone(&self.cache_misses),
            cache_errors: Arc::clone(&self.cache_errors),
            cache_strategy: self.cache_strategy.clone(),
            cache_ttl_seconds: self.cache_ttl_seconds,
            applied_config: self.applied_config.clone(),
            search_index_rcu_retries: Arc::clone(&self.search_index_rcu_retries),
        }
    }
}

impl AppState {
    /// Creates a new `AppState` from initialized repositories.
    ///
    /// This constructor takes ownership of the `Repositories` struct returned
    /// by `RepositoryFactory::create()`.
    ///
    /// Uses stub external sources and default `AppliedConfig` for backward compatibility.
    ///
    /// # Note
    ///
    /// This is an async function because it needs to fetch all tasks from the
    /// repository to build the initial search index. The index is built once
    /// at startup and updated incrementally thereafter.
    ///
    /// # Errors
    ///
    /// Returns an error if the task repository fails to list tasks.
    pub async fn from_repositories(
        repositories: Repositories,
    ) -> Result<Self, crate::infrastructure::RepositoryError> {
        Self::with_config(repositories, AppConfig::default()).await
    }

    /// Creates a new `AppState` from repositories and external sources.
    ///
    /// This constructor is intended for production use where real external
    /// data sources (Redis, HTTP) are configured.
    ///
    /// # Arguments
    ///
    /// * `repositories` - Initialized repositories from `RepositoryFactory`
    /// * `external_sources` - External data sources (Redis, HTTP)
    /// * `applied_config` - Applied configuration values for `/debug/config` endpoint
    ///
    /// # Errors
    ///
    /// Returns an error if the task repository fails to list tasks.
    pub async fn with_external_sources(
        repositories: Repositories,
        external_sources: ExternalSources,
        applied_config: AppliedConfig,
    ) -> Result<Self, crate::infrastructure::RepositoryError> {
        Self::with_full_config(
            repositories,
            AppConfig::default(),
            BulkConfig::from_env(),
            external_sources,
            applied_config,
        )
        .await
    }

    /// Creates a new `AppState` from repositories and custom configuration.
    ///
    /// Uses stub external sources for backward compatibility.
    /// Uses default `AppliedConfig` (for tests that don't care about applied config).
    ///
    /// # Errors
    ///
    /// Returns an error if the task repository fails to list tasks.
    pub async fn with_config(
        repositories: Repositories,
        config: AppConfig,
    ) -> Result<Self, crate::infrastructure::RepositoryError> {
        let external_sources = create_stub_external_sources();
        Self::with_full_config(
            repositories,
            config,
            BulkConfig::from_env(),
            external_sources,
            AppliedConfig::default(),
        )
        .await
    }

    /// Creates a new `AppState` from repositories and all configurations.
    ///
    /// This method allows explicit injection of `BulkConfig` and `ExternalSources`
    /// for testing purposes.
    ///
    /// # Backfill Processing
    ///
    /// On startup, this method performs backfill processing for existing tasks
    /// that do not have any events in the event store. For each such task, a
    /// `TaskCreated` event is generated and stored.
    ///
    /// Backfill behavior is controlled by the `SKIP_BACKFILL` environment variable:
    /// - `SKIP_BACKFILL=true` or `SKIP_BACKFILL=1`: Skip backfill (for development/debugging)
    /// - Otherwise: Perform backfill (default behavior)
    ///
    /// # Idempotency
    ///
    /// The backfill process is idempotent:
    /// - Only tasks with `get_current_version() == 0` are backfilled
    /// - Optimistic locking (`expected_version=0`) prevents duplicate writes on concurrent startup
    ///
    /// # Errors
    ///
    /// Returns an error if the task repository fails to list tasks.
    pub async fn with_full_config(
        repositories: Repositories,
        config: AppConfig,
        bulk_config: BulkConfig,
        external_sources: ExternalSources,
        applied_config: AppliedConfig,
    ) -> Result<Self, RepositoryError> {
        // Fetch all tasks to build the initial search index
        let all_tasks = repositories
            .task_repository
            .list(Pagination::all())
            .await?;

        // Build the search index from all tasks (pure function)
        // If SEARCH_INDEX_METRICS_PATH is set, measure build performance and output metrics
        let tasks: PersistentVector<Task> = all_tasks.items.clone().into_iter().collect();
        let metrics_output_path = std::env::var("SEARCH_INDEX_METRICS_PATH").ok();

        let search_index = metrics_output_path.as_ref().map_or_else(
            || SearchIndex::build(&tasks),
            |path| {
                // Measure build with performance metrics (I/O boundary)
                let (index, metrics) =
                    measure_search_index_build(&tasks, SearchIndexConfig::default());

                // Write metrics to JSON file
                if let Ok(json) = serde_json::to_string_pretty(&metrics) {
                    let output_path = std::path::Path::new(path);
                    if let Some(parent) = output_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Err(error) = std::fs::write(output_path, json) {
                        tracing::warn!(
                            path = %path,
                            %error,
                            "Failed to write SearchIndex build metrics"
                        );
                    } else {
                        tracing::info!(
                            path = %path,
                            elapsed_ms = metrics.elapsed_ms,
                            peak_rss_mb = metrics.peak_rss_mb,
                            ngram_entries = metrics.ngram_entries,
                            "SearchIndex build metrics written"
                        );
                    }
                }

                index
            },
        );

        // Perform backfill processing (unless SKIP_BACKFILL=true)
        let skip_backfill = std::env::var("SKIP_BACKFILL")
            .map(|value| matches!(value.to_lowercase().as_str(), "true" | "1" | "yes"))
            .unwrap_or(false);

        if skip_backfill {
            tracing::info!("Skipping backfill (SKIP_BACKFILL=true)");
        } else {
            let backfill_result =
                backfill_existing_tasks(&all_tasks.items, repositories.event_store.as_ref()).await;

            match backfill_result {
                Ok((backfilled, skipped)) => {
                    if backfilled > 0 || skipped > 0 {
                        tracing::info!(
                            backfilled = backfilled,
                            skipped = skipped,
                            "Backfill complete"
                        );
                    }
                }
                Err(error) => {
                    tracing::error!(%error, "Backfill failed");
                    return Err(error);
                }
            }
        }

        // Create RNG provider from environment (I/O boundary)
        let rng_provider = Arc::new(RngProvider::from_env().unwrap_or_else(|error| {
            tracing::warn!(%error, "Failed to create RNG provider from env, using random");
            RngProvider::new_random()
        }));

        // Load cache configuration for metadata (CACHE-REQ-021)
        let cache_config = crate::infrastructure::CacheConfig::from_env();

        Ok(Self {
            task_repository: repositories.task_repository,
            project_repository: repositories.project_repository,
            event_store: repositories.event_store,
            config,
            bulk_config,
            search_index: Arc::new(ArcSwap::from_pointee(search_index)),
            search_cache: Arc::new(SearchCache::with_default_config()),
            secondary_source: external_sources.secondary_source,
            external_source: external_sources.external_source,
            rng_provider,
            cache_hits: Arc::new(AtomicU64::new(0)),
            cache_misses: Arc::new(AtomicU64::new(0)),
            cache_errors: Arc::new(AtomicU64::new(0)),
            cache_strategy: cache_config.strategy.to_string(),
            cache_ttl_seconds: cache_config.ttl_seconds,
            applied_config,
            search_index_rcu_retries: Arc::new(AtomicUsize::new(0)),
        })
    }

    /// Updates the search index with a task change.
    ///
    /// This method atomically replaces the search index with a new version
    /// that reflects the given change using Read-Copy-Update (RCU) pattern.
    /// The RCU pattern ensures that concurrent updates are handled correctly
    /// through CAS (Compare-And-Swap) retry, preventing lost updates.
    ///
    /// # Arguments
    ///
    /// * `change` - The task change to apply (Add, Update, or Remove).
    ///   Takes ownership because `rcu` may retry the closure multiple times,
    ///   requiring the change to be cloned on each retry.
    ///
    /// # Concurrency
    ///
    /// The `rcu` method provides atomic updates:
    /// 1. Read current value
    /// 2. Apply transformation (copy with modification)
    /// 3. Attempt CAS to replace the old value
    /// 4. If CAS fails (another thread updated), retry from step 1
    ///
    /// This ensures no updates are lost even under concurrent modifications.
    #[allow(clippy::needless_pass_by_value)] // Ownership needed for rcu retry via clone
    pub fn update_search_index(&self, change: TaskChange) {
        self.search_index.rcu(|current| {
            let updated = current.apply_change(change.clone());
            Arc::new(updated)
        });
    }

    /// Updates the search index with multiple task changes in a single RCU operation.
    ///
    /// This method atomically applies all changes in a single RCU update, reducing
    /// CAS (Compare-And-Swap) retries compared to calling `update_search_index`
    /// individually for each change.
    ///
    /// # Arguments
    ///
    /// * `changes` - A slice of `TaskChange` items to apply (Add, Update, or Remove).
    ///   If empty, returns immediately without modifying the index.
    ///
    /// # Concurrency
    ///
    /// The `rcu` method provides atomic updates:
    /// 1. Read current value
    /// 2. Apply all transformations in a single pass (copy with modifications)
    /// 3. Attempt CAS to replace the old value
    /// 4. If CAS fails (another thread updated), retry from step 1
    ///
    /// By batching changes into a single RCU operation, this method minimizes
    /// the number of CAS retries during bulk processing.
    ///
    /// # Timing Logs
    ///
    /// Emits a `tracing::info` log with timing breakdown:
    /// - `change_count`: Number of changes processed
    /// - `apply_changes_us`: Time spent in `apply_changes` (microseconds)
    /// - `total_us`: Total elapsed time (microseconds)
    /// - `total_ms`: Total elapsed time (milliseconds)
    ///
    /// # Performance
    ///
    /// Uses `SearchIndex::apply_changes` internally, which computes a
    /// `SearchIndexDelta` to batch index modifications efficiently.
    pub fn update_search_index_batch(&self, changes: &[TaskChange]) {
        if changes.is_empty() {
            return;
        }

        let change_count = changes.len();
        let total_start = std::time::Instant::now();
        let mut retry_count: u64 = 0;
        let apply_changes_us;

        loop {
            let current = self.search_index.load();
            let apply_start = std::time::Instant::now();
            let updated = Arc::new(current.apply_changes(changes));
            let elapsed = apply_start.elapsed().as_micros();

            let previous = self
                .search_index
                .compare_and_swap(&current, Arc::clone(&updated));

            if Arc::ptr_eq(&previous, &current) {
                apply_changes_us = elapsed;
                break;
            }
            retry_count += 1;
            self.search_index_rcu_retries
                .fetch_add(1, Ordering::Relaxed);
        }

        let total_elapsed = total_start.elapsed();

        tracing::info!(
            target: "search_index_batch",
            change_count = change_count,
            apply_changes_us = apply_changes_us,
            total_us = total_elapsed.as_micros(),
            total_ms = total_elapsed.as_millis(),
            retry_count = retry_count,
            "update_search_index_batch completed"
        );
    }

    /// Increments cache counter based on status (CACHE-REQ-021). Bypass does not increment.
    pub fn record_cache_status(&self, status: CacheStatus) {
        match status {
            CacheStatus::Hit => self.cache_hits.fetch_add(1, Ordering::Relaxed),
            CacheStatus::Miss => self.cache_misses.fetch_add(1, Ordering::Relaxed),
            CacheStatus::Error => self.cache_errors.fetch_add(1, Ordering::Relaxed),
            CacheStatus::Bypass => 0, // No counter for bypass
        };
    }
}

// =============================================================================
// External Sources Helper
// =============================================================================

use crate::infrastructure::StubExternalDataSource;

/// Creates stub external sources for testing and backward compatibility.
///
/// The stub sources return `Ok(None)` for all fetch operations, simulating
/// external sources that always report "not found".
#[must_use]
pub fn create_stub_external_sources() -> ExternalSources {
    ExternalSources {
        secondary_source: Arc::new(StubExternalDataSource::not_found("secondary")),
        external_source: Arc::new(StubExternalDataSource::not_found("external")),
    }
}

// =============================================================================
// Backfill Processing
// =============================================================================

/// Backfills existing tasks with `TaskCreated` events.
///
/// This function is called at startup to ensure all existing tasks have at least
/// one event in the event store. This is necessary for the event sourcing model
/// to work correctly.
///
/// # Idempotency
///
/// The backfill process is idempotent:
/// - Only tasks with `get_current_version() == 0` are backfilled
/// - Optimistic locking (`expected_version=0`) prevents duplicate writes on concurrent startup
///
/// # Concurrent Startup Handling
///
/// When multiple instances start concurrently:
/// 1. Each instance checks `get_current_version()` - returns 0 for tasks without events
/// 2. Instance attempts `append(event, expected_version=0)`
/// 3. First instance succeeds, others get `VersionConflict` error (expected: 0, found: 1)
/// 4. `VersionConflict` is treated as "already backfilled" and skipped
///
/// # Arguments
///
/// * `tasks` - Slice of tasks to potentially backfill
/// * `event_store` - The event store to write events to
///
/// # Returns
///
/// Returns `Ok((backfilled_count, skipped_count))` on success, or an error if
/// a non-recoverable error occurs.
async fn backfill_existing_tasks(
    tasks: &[Task],
    event_store: &dyn EventStore,
) -> Result<(u64, u64), RepositoryError> {
    let mut backfilled_count = 0u64;
    let mut skipped_count = 0u64;

    for task in tasks {
        // Idempotency check: skip tasks that already have events
        let current_version = event_store
            .get_current_version(&task.task_id)
            .await?;

        if current_version > 0 {
            skipped_count += 1;
            continue;
        }

        // Create the TaskCreated event (pure function)
        // Version semantics: event.version = expected_version + 1
        // For initial creation: expected_version = 0, so event.version = 1
        let event = create_task_created_event(
            task,
            EventId::generate_v7(),  // I/O boundary: generate event ID
            task.created_at.clone(), // Use task's creation timestamp for event
            1,                       // First event version (expected_version=0 + 1)
        );

        // Attempt to append with optimistic locking (expected_version=0)
        match event_store.append(&event, 0).await {
            Ok(()) => {
                backfilled_count += 1;
                tracing::debug!(task_id = %task.task_id, "Backfilled task");
            }
            Err(RepositoryError::VersionConflict {
                expected: 0,
                found: _,
            }) => {
                // Only skip VersionConflict when expected_version=0 (concurrent startup case)
                // Other version conflicts should be propagated as errors
                tracing::debug!(
                    task_id = %task.task_id,
                    "Task already backfilled by another process"
                );
                skipped_count += 1;
            }
            Err(error) => {
                // Non-recoverable error (including non-zero VersionConflict)
                return Err(error);
            }
        }
    }

    Ok((backfilled_count, skipped_count))
}

// =============================================================================
// POST /tasks Handler
// =============================================================================

/// Creates a new task.
///
/// This handler demonstrates the use of:
/// - **Functor**: Mapping over `Either` for DTO validation
/// - **Monad**: Chaining validation results with `map_left`
/// - **Either**: Representing validation success/failure
/// - **`AsyncIO`**: Encapsulating repository side effects
///
/// # Request Body
///
/// ```json
/// {
///   "title": "Task title",
///   "description": "Optional description",
///   "priority": "low|medium|high|critical",
///   "tags": ["tag1", "tag2"]
/// }
/// ```
///
/// # Response
///
/// - **201 Created**: Task created successfully
/// - **400 Bad Request**: Validation error
/// - **500 Internal Server Error**: Database error
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] in the following cases:
/// - Validation error (400 Bad Request): Invalid title, description, or tags
/// - Database error (500 Internal Server Error): Repository operation failed
///
/// # Note on Send bounds
///
/// `Task` is not `Send` because it contains `PersistentHashSet` and `PersistentList`
/// which use `Rc`. We handle this by:
/// 1. Creating the task synchronously
/// 2. Converting to `TaskResponse` before the async boundary
/// 3. Executing the repository save in a separate async block
///
/// # Event Sourcing
///
/// This handler writes a `TaskCreated` event to the event store alongside
/// the task persistence. Event write failures are treated as warnings
/// (best-effort consistency) rather than errors.
#[allow(clippy::future_not_send)]
pub async fn create_task(
    State(state): State<AppState>,
    Json(request): Json<CreateTaskRequest>,
) -> Result<(StatusCode, JsonResponse<TaskResponse>), ApiErrorResponse> {
    // Step 1: Validate using Either (demonstrates Monad chaining)
    let validated = validate_create_request(&request)?;

    // Step 2: Create task synchronously (Task is not Send)
    // Generate IDs and timestamp within this block (impure operations)
    let (task, mut response, event) = {
        let ids = generate_task_ids();
        let task = build_task(ids, validated);
        let response = TaskResponse::from(&task);

        // Generate event ID and create the task created event (pure function)
        // Version semantics: event.version = expected_version + 1
        // For initial creation: expected_version = 0, so event.version = 1
        let event = create_task_created_event(
            &task,
            EventId::generate_v7(), // I/O boundary: generate event ID
            Timestamp::now(),       // I/O boundary: current timestamp
            1,                      // First event version (expected_version=0 + 1)
        );

        (task, response, event)
    };

    // Step 3: Save task and write event using best-effort consistency
    let save_result = save_task_with_event(&state, &task, event, 0).await?;

    // Step 4: Add consistency warning to response if event write failed
    if let Some(warning) = &save_result.consistency_warning {
        response.warnings.push(warning.client_message());
    }

    // Step 5: Update search index with the new task (lock-free write)
    state.update_search_index(TaskChange::Add(task));

    Ok((StatusCode::CREATED, JsonResponse(response)))
}

// =============================================================================
// Helper Types and Functions
// =============================================================================

/// Validated create task data.
#[derive(Debug)]
struct ValidatedCreateTask {
    title: String,
    description: Option<String>,
    priority: Priority,
    tags: Vec<Tag>,
}

/// Generated IDs for a new task.
struct TaskIds {
    task_id: TaskId,
    timestamp: Timestamp,
}

/// Validates a create task request.
///
/// Uses `Either` monad for validation, converting to `Result` at the boundary.
fn validate_create_request(
    request: &CreateTaskRequest,
) -> Result<ValidatedCreateTask, ApiErrorResponse> {
    // Chain validations using Either's monadic properties
    let title_result = validate_title(&request.title);
    let desc_result = validate_description(request.description.as_deref());
    let tags_result = validate_tags(&request.tags);

    // Combine validation results
    // Using map_left to convert ValidationError to ApiErrorResponse
    let title: Result<String, ApiErrorResponse> =
        title_result.map_left(ApiErrorResponse::from).into();
    let title = title?;

    let description: Result<Option<String>, ApiErrorResponse> =
        desc_result.map_left(ApiErrorResponse::from).into();
    let description = description?;

    let tags: Result<Vec<Tag>, ApiErrorResponse> =
        tags_result.map_left(ApiErrorResponse::from).into();
    let tags = tags?;

    Ok(ValidatedCreateTask {
        title,
        description,
        priority: Priority::from(request.priority),
        tags,
    })
}

/// Generates task IDs within an effect boundary.
///
/// Note: This function contains impure operations (UUID generation, timestamp).
fn generate_task_ids() -> TaskIds {
    TaskIds {
        task_id: TaskId::generate_v7(),
        timestamp: Timestamp::now(),
    }
}

/// Builds a task from validated data and generated IDs.
///
/// This is a pure function.
fn build_task(ids: TaskIds, validated: ValidatedCreateTask) -> Task {
    let mut task = Task::new(ids.task_id, validated.title, ids.timestamp);

    if let Some(desc) = validated.description {
        task = task.with_description(desc);
    }

    task = task.with_priority(validated.priority);

    for tag in validated.tags {
        task = task.add_tag(tag);
    }

    task
}

// =============================================================================
// GET /tasks/{id} Handler
// =============================================================================

/// Gets a task by its ID.
///
/// This handler demonstrates the use of:
/// - **Either**: Lifting `Option<Task>` to `Either<ApiErrorResponse, Task>`
/// - **Pattern matching**: Functional error handling without exceptions
/// - **`AsyncIO`**: Encapsulating repository side effects
///
/// # Path Parameters
///
/// * `id` - The UUID of the task to retrieve
///
/// # Response
///
/// - **200 OK**: Task found and returned
/// - **404 Not Found**: Task with the given ID does not exist
/// - **500 Internal Server Error**: Database error
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] in the following cases:
/// - Not found error (404 Not Found): Task does not exist
/// - Database error (500 Internal Server Error): Repository operation failed
///
/// # lambars Features
///
/// The handler uses `Either<ApiErrorResponse, Task>` to represent the result
/// of the lookup operation. `Option<Task>` from `find_by_id` is lifted to
/// `Either` using pattern matching:
/// - `Some(task)` becomes `Either::Right(task)` (success)
/// - `None` becomes `Either::Left(ApiErrorResponse::not_found(...))` (failure)
#[allow(clippy::future_not_send)]
pub async fn get_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<(HeaderMap, JsonResponse<TaskResponse>), ApiErrorResponse> {
    // Step 1: Convert Uuid to TaskId (pure)
    let task_id = TaskId::from_uuid(task_id);

    // Step 2: Fetch task from repository with cache status using AsyncIO
    let cache_result = state
        .task_repository
        .find_by_id_with_status(&task_id)
        .await
        .map_err(ApiErrorResponse::from)?;

    // Step 3: Extract cache status and value, record metrics
    let cache_status = cache_result.cache_status;
    let maybe_task = cache_result.value;

    // Record cache metrics (CACHE-REQ-021)
    state.record_cache_status(cache_status);

    // Step 4: Lift Option<Task> to Either<ApiErrorResponse, Task>
    // This demonstrates functional error handling using Either
    let task_result: Either<ApiErrorResponse, Task> = lift_option_to_either(maybe_task, || {
        ApiErrorResponse::not_found(format!("Task not found: {task_id}"))
    });

    // Step 5: Convert Either to Result and map to response
    // Task is not Send, so we convert to TaskResponse (which is Send) immediately
    let result: Result<Task, ApiErrorResponse> = task_result.into();
    let task = result?;
    let response = TaskResponse::from(&task);

    // Step 6: Build cache headers (pure transformation)
    let headers = build_cache_headers(cache_status, CacheSource::Redis);

    Ok((headers, JsonResponse(response)))
}

/// Builds HTTP headers for cache status (X-Cache, X-Cache-Status, X-Cache-Source).
///
/// For `Bypass` status, source is overridden to `None` because no cache was consulted.
pub fn build_cache_headers(cache_status: CacheStatus, cache_source: CacheSource) -> HeaderMap {
    let effective_source = match cache_status {
        CacheStatus::Bypass => CacheSource::None,
        CacheStatus::Hit | CacheStatus::Miss | CacheStatus::Error => cache_source,
    };

    let extension = CacheHeaderExtension::new(cache_status, effective_source);
    let mut headers = HeaderMap::new();

    headers.insert(
        "X-Cache",
        HeaderValue::from_static(extension.x_cache_value()),
    );
    headers.insert(
        "X-Cache-Status",
        HeaderValue::from_static(extension.x_cache_status_value()),
    );
    headers.insert(
        "X-Cache-Source",
        HeaderValue::from_static(extension.x_cache_source_value()),
    );

    headers
}

/// Lifts an `Option<T>` to `Either<L, T>`.
///
/// This is a pure function that converts `Option` to `Either`:
/// - `Some(value)` becomes `Either::Right(value)`
/// - `None` becomes `Either::Left(left_value())` where `left_value` is lazily evaluated
///
/// # Type Parameters
///
/// * `L` - The type for the Left case (typically an error type)
/// * `T` - The type for the Right case (the success value)
/// * `F` - A function that produces the Left value when None is encountered
///
/// # Examples
///
/// ```ignore
/// let some_value = Some(42);
/// let result = lift_option_to_either(some_value, || "not found");
/// assert_eq!(result, Either::Right(42));
///
/// let none_value: Option<i32> = None;
/// let result = lift_option_to_either(none_value, || "not found");
/// assert_eq!(result, Either::Left("not found"));
/// ```
fn lift_option_to_either<L, T, F>(option: Option<T>, left_value: F) -> Either<L, T>
where
    F: FnOnce() -> L,
{
    option.map_or_else(|| Either::Left(left_value()), Either::Right)
}

// =============================================================================
// DELETE /tasks/{id} Handler
// =============================================================================

/// Deletes a task by its ID.
///
/// This handler demonstrates the use of:
/// - **Either**: Lifting `Option<Task>` to `Either<ApiErrorResponse, Task>`
/// - **Pattern matching**: Functional error handling without exceptions
/// - **`AsyncIO`**: Encapsulating repository side effects
/// - **Search index update**: Incremental index maintenance via RCU
///
/// # Path Parameters
///
/// * `id` - The UUID of the task to delete
///
/// # Response
///
/// - **204 No Content**: Task deleted successfully
/// - **404 Not Found**: Task with the given ID does not exist
/// - **500 Internal Server Error**: Database error
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] in the following cases:
/// - Not found error (404 Not Found): Task does not exist
/// - Database error (500 Internal Server Error): Repository operation failed
///
/// # Search Index
///
/// On successful deletion, the search index is updated atomically using the
/// RCU (Read-Copy-Update) pattern to remove the deleted task. This ensures
/// that search results are consistent with the actual data store.
#[allow(clippy::future_not_send)]
pub async fn delete_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<StatusCode, ApiErrorResponse> {
    // Step 1: Convert Uuid to TaskId (pure)
    let task_id = TaskId::from_uuid(task_id);

    // Step 2: Delete from repository using AsyncIO
    // The delete operation returns true if the task was found and deleted
    let deleted = state
        .task_repository
        .delete(&task_id)
        .await
        .map_err(ApiErrorResponse::from)?;

    // Step 3: Check if deletion was successful
    if !deleted {
        return Err(ApiErrorResponse::not_found(format!(
            "Task not found: {task_id}"
        )));
    }

    // Step 4: Update search index to remove the deleted task (lock-free write via RCU)
    state.update_search_index(TaskChange::Remove(task_id));

    Ok(StatusCode::NO_CONTENT)
}

// =============================================================================
// GET /health Handler
// =============================================================================

/// Health check response body.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthResponse {
    /// Service status.
    pub status: &'static str,
    /// Service version.
    pub version: &'static str,
}

/// Health check endpoint.
///
/// Returns a simple JSON response indicating the service is running.
/// This endpoint can be used by load balancers and orchestration systems
/// to verify service availability.
///
/// # Response
///
/// - **200 OK**: Service is healthy
///
/// ```json
/// {
///   "status": "healthy",
///   "version": "0.1.0"
/// }
/// ```
pub async fn health_check() -> JsonResponse<HealthResponse> {
    JsonResponse(HealthResponse {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    use lambars::effect::AsyncIO;

    use crate::domain::{Priority, TaskStatus};
    use crate::infrastructure::{
        InMemoryEventStore, InMemoryProjectRepository, InMemoryTaskRepository, RepositoryError,
        SearchScope,
    };

    // -------------------------------------------------------------------------
    // Mock TaskRepository for Error Simulation
    // -------------------------------------------------------------------------

    /// A mock `TaskRepository` that can be configured to return errors.
    ///
    /// This mock is used to test error handling paths in handlers.
    struct MockTaskRepository {
        /// The error to return from `find_by_id`, if any.
        find_by_id_error: Option<RepositoryError>,
        /// The task to return from `find_by_id`, if no error is configured.
        find_by_id_result: Option<Task>,
    }

    impl MockTaskRepository {
        /// Creates a mock that returns `Some(task)` from `find_by_id`.
        fn with_task(task: Task) -> Self {
            Self {
                find_by_id_error: None,
                find_by_id_result: Some(task),
            }
        }

        /// Creates a mock that returns `None` from `find_by_id`.
        fn not_found() -> Self {
            Self {
                find_by_id_error: None,
                find_by_id_result: None,
            }
        }

        /// Creates a mock that returns an error from `find_by_id`.
        fn with_error(error: RepositoryError) -> Self {
            Self {
                find_by_id_error: Some(error),
                find_by_id_result: None,
            }
        }
    }

    impl TaskRepository for MockTaskRepository {
        fn find_by_id(&self, _id: &TaskId) -> AsyncIO<Result<Option<Task>, RepositoryError>> {
            let error = self.find_by_id_error.clone();
            let result = self.find_by_id_result.clone();
            AsyncIO::new(move || async move { error.map_or_else(|| Ok(result), Err) })
        }

        fn save(&self, _task: &Task) -> AsyncIO<Result<(), RepositoryError>> {
            AsyncIO::new(|| async { Ok(()) })
        }

        fn save_bulk(&self, tasks: &[Task]) -> AsyncIO<Vec<Result<(), RepositoryError>>> {
            let count = tasks.len();
            AsyncIO::new(move || async move { vec![Ok(()); count] })
        }

        fn delete(&self, _id: &TaskId) -> AsyncIO<Result<bool, RepositoryError>> {
            AsyncIO::new(|| async { Ok(false) })
        }

        fn list(
            &self,
            pagination: crate::infrastructure::Pagination,
        ) -> AsyncIO<Result<crate::infrastructure::PaginatedResult<Task>, RepositoryError>>
        {
            AsyncIO::new(move || async move {
                Ok(crate::infrastructure::PaginatedResult::new(
                    vec![],
                    0,
                    pagination.page,
                    pagination.page_size,
                ))
            })
        }

        fn list_filtered(
            &self,
            _status: Option<TaskStatus>,
            _priority: Option<Priority>,
            pagination: crate::infrastructure::Pagination,
        ) -> AsyncIO<Result<crate::infrastructure::PaginatedResult<Task>, RepositoryError>>
        {
            AsyncIO::new(move || async move {
                Ok(crate::infrastructure::PaginatedResult::new(
                    vec![],
                    0,
                    pagination.page,
                    pagination.page_size,
                ))
            })
        }

        fn search(
            &self,
            _query: &str,
            _scope: SearchScope,
            _limit: u32,
            _offset: u32,
        ) -> AsyncIO<Result<Vec<Task>, RepositoryError>> {
            AsyncIO::new(|| async { Ok(vec![]) })
        }

        fn count(&self) -> AsyncIO<Result<u64, RepositoryError>> {
            AsyncIO::new(|| async { Ok(0) })
        }
    }

    // -------------------------------------------------------------------------
    // Helper Functions for AppState Creation
    // -------------------------------------------------------------------------

    /// Creates an `AppState` with the given mock task repository.
    fn create_app_state_with_mock_task_repository(
        task_repository: impl TaskRepository + 'static,
    ) -> AppState {
        use crate::api::bulk::BulkConfig;
        use crate::api::query::{SearchCache, SearchIndex};
        use crate::infrastructure::RngProvider;
        use arc_swap::ArcSwap;
        use lambars::persistent::PersistentVector;
        use std::sync::atomic::{AtomicU64, AtomicUsize};

        let external_sources = super::create_stub_external_sources();

        AppState {
            task_repository: Arc::new(task_repository),
            project_repository: Arc::new(InMemoryProjectRepository::new()),
            event_store: Arc::new(InMemoryEventStore::new()),
            config: AppConfig::default(),
            bulk_config: BulkConfig::default(),
            search_index: Arc::new(ArcSwap::from_pointee(SearchIndex::build(
                &PersistentVector::new(),
            ))),
            search_cache: Arc::new(SearchCache::with_default_config()),
            secondary_source: external_sources.secondary_source,
            external_source: external_sources.external_source,
            rng_provider: Arc::new(RngProvider::new_random()),
            cache_hits: Arc::new(AtomicU64::new(0)),
            cache_misses: Arc::new(AtomicU64::new(0)),
            cache_errors: Arc::new(AtomicU64::new(0)),
            cache_strategy: "read-through".to_string(),
            cache_ttl_seconds: 60,
            applied_config: AppliedConfig::default(),
            search_index_rcu_retries: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Creates an `AppState` with the default in-memory repositories.
    fn create_default_app_state() -> AppState {
        use crate::api::bulk::BulkConfig;
        use crate::api::query::{SearchCache, SearchIndex};
        use crate::infrastructure::RngProvider;
        use arc_swap::ArcSwap;
        use lambars::persistent::PersistentVector;
        use std::sync::atomic::{AtomicU64, AtomicUsize};

        let external_sources = super::create_stub_external_sources();

        AppState {
            task_repository: Arc::new(InMemoryTaskRepository::new()),
            project_repository: Arc::new(InMemoryProjectRepository::new()),
            event_store: Arc::new(InMemoryEventStore::new()),
            config: AppConfig::default(),
            bulk_config: BulkConfig::default(),
            search_index: Arc::new(ArcSwap::from_pointee(SearchIndex::build(
                &PersistentVector::new(),
            ))),
            search_cache: Arc::new(SearchCache::with_default_config()),
            secondary_source: external_sources.secondary_source,
            external_source: external_sources.external_source,
            rng_provider: Arc::new(RngProvider::new_random()),
            cache_hits: Arc::new(AtomicU64::new(0)),
            cache_misses: Arc::new(AtomicU64::new(0)),
            cache_errors: Arc::new(AtomicU64::new(0)),
            cache_strategy: "read-through".to_string(),
            cache_ttl_seconds: 60,
            applied_config: AppliedConfig::default(),
            search_index_rcu_retries: Arc::new(AtomicUsize::new(0)),
        }
    }

    // -------------------------------------------------------------------------
    // get_task Handler Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_get_task_returns_200_when_task_found() {
        // Arrange
        let task_id = TaskId::generate();
        let task = Task::new(task_id.clone(), "Test Task", Timestamp::now())
            .with_description("Test description")
            .with_priority(Priority::High);
        let state = create_app_state_with_mock_task_repository(MockTaskRepository::with_task(task));

        // Act
        let result = get_task(State(state), Path(*task_id.as_uuid())).await;

        // Assert
        assert!(result.is_ok());
        let (headers, JsonResponse(response)) = result.unwrap();
        assert_eq!(response.title, "Test Task");
        assert_eq!(response.description, Some("Test description".to_string()));
        assert_eq!(response.priority, super::super::dto::PriorityDto::High);

        // Verify cache headers are present with correct values
        assert!(headers.contains_key("X-Cache"));
        assert!(headers.contains_key("X-Cache-Status"));
        assert!(headers.contains_key("X-Cache-Source"));

        // Verify header values (mock returns CacheStatus::Bypass since no Redis layer)
        assert_eq!(headers.get("X-Cache").unwrap(), "MISS");
        assert_eq!(headers.get("X-Cache-Status").unwrap(), "bypass");
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_task_returns_404_when_task_not_found() {
        // Arrange
        let task_id = TaskId::generate();
        let state = create_app_state_with_mock_task_repository(MockTaskRepository::not_found());

        // Act
        let result = get_task(State(state), Path(*task_id.as_uuid())).await;

        // Assert
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.status, StatusCode::NOT_FOUND);
        assert_eq!(error.error.code, "NOT_FOUND");
        assert!(error.error.message.contains("Task not found"));
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_task_returns_500_when_repository_error() {
        // Arrange
        let task_id = TaskId::generate();
        let state = create_app_state_with_mock_task_repository(MockTaskRepository::with_error(
            RepositoryError::DatabaseError("Connection failed".to_string()),
        ));

        // Act
        let result = get_task(State(state), Path(*task_id.as_uuid())).await;

        // Assert
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(error.error.code, "INTERNAL_ERROR");
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_task_converts_task_to_task_response_correctly() {
        // Arrange
        let task_id = TaskId::generate();
        let timestamp = Timestamp::now();
        let task = Task::new(task_id.clone(), "Complete Task", timestamp)
            .with_description("Detailed description")
            .with_priority(Priority::Critical)
            .add_tag(Tag::new("urgent"))
            .add_tag(Tag::new("backend"));
        let state = create_app_state_with_mock_task_repository(MockTaskRepository::with_task(task));

        // Act
        let result = get_task(State(state), Path(*task_id.as_uuid())).await;

        // Assert
        assert!(result.is_ok());
        let (_headers, JsonResponse(response)) = result.unwrap();
        assert_eq!(response.id, task_id.to_string());
        assert_eq!(response.title, "Complete Task");
        assert_eq!(
            response.description,
            Some("Detailed description".to_string())
        );
        assert_eq!(response.priority, super::super::dto::PriorityDto::Critical);
        assert_eq!(response.tags.len(), 2);
        assert!(response.tags.contains(&"urgent".to_string()));
        assert!(response.tags.contains(&"backend".to_string()));
        assert_eq!(response.version, 1);
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_task_with_real_repository_integration() {
        // Arrange
        let state = create_default_app_state();
        let task_id = TaskId::generate();
        let task = Task::new(task_id.clone(), "Integration Test Task", Timestamp::now());

        // Save the task first
        state
            .task_repository
            .save(&task)
            .await
            .expect("Failed to save task");

        // Act
        let result = get_task(State(state), Path(*task_id.as_uuid())).await;

        // Assert
        assert!(result.is_ok());
        let (_headers, JsonResponse(response)) = result.unwrap();
        assert_eq!(response.title, "Integration Test Task");
    }

    // -------------------------------------------------------------------------
    // Validation Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_validate_create_request_valid() {
        let request = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: Some("Description".to_string()),
            priority: super::super::dto::PriorityDto::High,
            tags: vec!["backend".to_string()],
        };

        let result = validate_create_request(&request);
        assert!(result.is_ok());

        let validated = result.unwrap();
        assert_eq!(validated.title, "Test Task");
        assert_eq!(validated.description, Some("Description".to_string()));
        assert_eq!(validated.priority, Priority::High);
        assert_eq!(validated.tags.len(), 1);
    }

    #[rstest]
    fn test_validate_create_request_empty_title() {
        let request = CreateTaskRequest {
            title: String::new(),
            description: None,
            priority: super::super::dto::PriorityDto::Low,
            tags: vec![],
        };

        let result = validate_create_request(&request);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
    }

    #[rstest]
    fn test_validate_create_request_invalid_tags() {
        let request = CreateTaskRequest {
            title: "Valid Title".to_string(),
            description: None,
            priority: super::super::dto::PriorityDto::Low,
            tags: vec![String::new()], // Empty tag
        };

        let result = validate_create_request(&request);
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // Build Task Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_build_task() {
        let ids = TaskIds {
            task_id: TaskId::generate(),
            timestamp: Timestamp::now(),
        };

        let validated = ValidatedCreateTask {
            title: "Test Task".to_string(),
            description: Some("Description".to_string()),
            priority: Priority::High,
            tags: vec![Tag::new("backend"), Tag::new("urgent")],
        };

        let task = build_task(ids, validated);

        assert_eq!(task.title, "Test Task");
        assert_eq!(task.description, Some("Description".to_string()));
        assert_eq!(task.priority, Priority::High);
        assert_eq!(task.tags.len(), 2);
        assert_eq!(task.version, 1);
    }

    #[rstest]
    fn test_build_task_no_description() {
        let ids = TaskIds {
            task_id: TaskId::generate(),
            timestamp: Timestamp::now(),
        };

        let validated = ValidatedCreateTask {
            title: "Test Task".to_string(),
            description: None,
            priority: Priority::Low,
            tags: vec![],
        };

        let task = build_task(ids, validated);

        assert!(task.description.is_none());
        assert!(task.tags.is_empty());
    }

    // -------------------------------------------------------------------------
    // Generate IDs Test
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_generate_task_ids_unique() {
        let ids1 = generate_task_ids();
        let ids2 = generate_task_ids();

        assert_ne!(ids1.task_id, ids2.task_id);
    }

    // -------------------------------------------------------------------------
    // lift_option_to_either Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_lift_option_to_either_some_returns_right() {
        let some_value: Option<i32> = Some(42);
        let result = lift_option_to_either(some_value, || "error");

        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), 42);
    }

    #[rstest]
    fn test_lift_option_to_either_none_returns_left() {
        let none_value: Option<i32> = None;
        let result = lift_option_to_either(none_value, || "not found");

        assert!(result.is_left());
        assert_eq!(result.unwrap_left(), "not found");
    }

    #[rstest]
    fn test_lift_option_to_either_left_value_is_lazy() {
        use std::cell::Cell;

        let call_count = Cell::new(0);
        let some_value: Option<i32> = Some(42);

        let _result = lift_option_to_either(some_value, || {
            call_count.set(call_count.get() + 1);
            "error"
        });

        // Left value function should not be called for Some case
        assert_eq!(call_count.get(), 0);
    }

    #[rstest]
    fn test_lift_option_to_either_with_api_error_response() {
        let none_value: Option<Task> = None;
        let task_id = TaskId::generate();

        let result: Either<ApiErrorResponse, Task> = lift_option_to_either(none_value, || {
            ApiErrorResponse::not_found(format!("Task not found: {task_id}"))
        });

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert_eq!(error.status, StatusCode::NOT_FOUND);
        assert_eq!(error.error.code, "NOT_FOUND");
    }

    #[rstest]
    fn test_lift_option_to_either_with_task() {
        let task = Task::new(TaskId::generate(), "Test Task", Timestamp::now());
        let some_task: Option<Task> = Some(task);

        let result: Either<ApiErrorResponse, Task> =
            lift_option_to_either(some_task, || ApiErrorResponse::not_found("Not found"));

        assert!(result.is_right());
        let returned_task = result.unwrap_right();
        assert_eq!(returned_task.title, "Test Task");
    }

    // -------------------------------------------------------------------------
    // record_cache_status Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_record_cache_status_hit() {
        let state = create_default_app_state();

        state.record_cache_status(CacheStatus::Hit);
        state.record_cache_status(CacheStatus::Hit);

        assert_eq!(
            state.cache_hits.load(std::sync::atomic::Ordering::Relaxed),
            2
        );
        assert_eq!(
            state
                .cache_misses
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        assert_eq!(
            state
                .cache_errors
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
    }

    #[rstest]
    fn test_record_cache_status_miss() {
        let state = create_default_app_state();

        state.record_cache_status(CacheStatus::Miss);

        assert_eq!(
            state.cache_hits.load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        assert_eq!(
            state
                .cache_misses
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );
        assert_eq!(
            state
                .cache_errors
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
    }

    #[rstest]
    fn test_record_cache_status_error() {
        let state = create_default_app_state();

        state.record_cache_status(CacheStatus::Error);

        assert_eq!(
            state.cache_hits.load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        assert_eq!(
            state
                .cache_misses
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        assert_eq!(
            state
                .cache_errors
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );
    }

    #[rstest]
    fn test_record_cache_status_bypass_does_not_increment() {
        let state = create_default_app_state();

        state.record_cache_status(CacheStatus::Bypass);

        // Bypass should not increment any counter
        assert_eq!(
            state.cache_hits.load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        assert_eq!(
            state
                .cache_misses
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        assert_eq!(
            state
                .cache_errors
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
    }

    // -------------------------------------------------------------------------
    // Backfill Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_backfill_existing_tasks_backfills_tasks_without_events() {
        // Arrange
        let event_store = InMemoryEventStore::new();
        let task1 = Task::new(TaskId::generate(), "Task 1", Timestamp::now());
        let task2 = Task::new(TaskId::generate(), "Task 2", Timestamp::now());
        let tasks = vec![task1.clone(), task2.clone()];

        // Act
        let result = super::backfill_existing_tasks(&tasks, &event_store).await;

        // Assert
        assert!(result.is_ok());
        let (backfilled, skipped) = result.unwrap();
        assert_eq!(backfilled, 2);
        assert_eq!(skipped, 0);

        // Verify events were created
        let version1 = event_store
            .get_current_version(&task1.task_id)
            .await
            .unwrap();
        let version2 = event_store
            .get_current_version(&task2.task_id)
            .await
            .unwrap();
        assert_eq!(version1, 1);
        assert_eq!(version2, 1);
    }

    #[rstest]
    #[tokio::test]
    async fn test_backfill_existing_tasks_skips_tasks_with_events() {
        // Arrange
        let event_store = InMemoryEventStore::new();
        let task1 = Task::new(TaskId::generate(), "Task 1", Timestamp::now());
        let task2 = Task::new(TaskId::generate(), "Task 2", Timestamp::now());

        // Pre-create an event for task1
        let event = crate::domain::create_task_created_event(
            &task1,
            crate::domain::EventId::generate_v7(),
            Timestamp::now(),
            1,
        );
        event_store.append(&event, 0).await.unwrap();

        let tasks = vec![task1.clone(), task2.clone()];

        // Act
        let result = super::backfill_existing_tasks(&tasks, &event_store).await;

        // Assert
        assert!(result.is_ok());
        let (backfilled, skipped) = result.unwrap();
        assert_eq!(backfilled, 1); // Only task2 was backfilled
        assert_eq!(skipped, 1); // task1 was skipped

        // Verify task1 still has only 1 event (not duplicated)
        let version1 = event_store
            .get_current_version(&task1.task_id)
            .await
            .unwrap();
        assert_eq!(version1, 1);
    }

    #[rstest]
    #[tokio::test]
    async fn test_backfill_existing_tasks_empty_list() {
        // Arrange
        let event_store = InMemoryEventStore::new();
        let tasks: Vec<Task> = vec![];

        // Act
        let result = super::backfill_existing_tasks(&tasks, &event_store).await;

        // Assert
        assert!(result.is_ok());
        let (backfilled, skipped) = result.unwrap();
        assert_eq!(backfilled, 0);
        assert_eq!(skipped, 0);
    }

    // -------------------------------------------------------------------------
    // delete_task Handler Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_delete_task_returns_204_when_task_exists() {
        // Arrange
        let state = create_default_app_state();
        let task_id = TaskId::generate();
        let task = Task::new(task_id.clone(), "Task to Delete", Timestamp::now());

        // Save the task first
        state
            .task_repository
            .save(&task)
            .await
            .expect("Failed to save task");

        // Act
        let result = delete_task(State(state), Path(*task_id.as_uuid())).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
    }

    #[rstest]
    #[tokio::test]
    async fn test_delete_task_returns_404_when_task_not_found() {
        // Arrange
        let state = create_default_app_state();
        let nonexistent_task_id = TaskId::generate();

        // Act
        let result = delete_task(State(state), Path(*nonexistent_task_id.as_uuid())).await;

        // Assert
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.status, StatusCode::NOT_FOUND);
        assert_eq!(error.error.code, "NOT_FOUND");
        assert!(error.error.message.contains("Task not found"));
    }

    #[rstest]
    #[tokio::test]
    async fn test_delete_task_removes_from_repository() {
        // Arrange
        let state = create_default_app_state();
        let task_id = TaskId::generate();
        let task = Task::new(task_id.clone(), "Task to Delete", Timestamp::now());

        // Save the task first
        state
            .task_repository
            .save(&task)
            .await
            .expect("Failed to save task");

        // Act
        let result = delete_task(State(state.clone()), Path(*task_id.as_uuid())).await;
        assert!(result.is_ok());

        // Assert: Task should no longer exist in repository
        let find_result = state
            .task_repository
            .find_by_id(&task_id)
            .await
            .expect("Failed to find task");
        assert!(find_result.is_none());
    }

    // -------------------------------------------------------------------------
    // update_search_index_batch Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_update_search_index_batch_applies_all_changes() {
        // Arrange
        let state = create_default_app_state();
        let task1 = Task::new(TaskId::generate(), "First Task", Timestamp::now());
        let task2 = Task::new(TaskId::generate(), "Second Task", Timestamp::now());
        let task3 = Task::new(TaskId::generate(), "Third Task", Timestamp::now());

        let changes = vec![
            TaskChange::Add(task1.clone()),
            TaskChange::Add(task2.clone()),
            TaskChange::Add(task3.clone()),
        ];

        // Act
        state.update_search_index_batch(&changes);

        // Assert: Verify that all tasks were added to the search index
        let index = state.search_index.load();

        // Search for each task by title using search_by_title
        let results1 = index.search_by_title("First");
        let results2 = index.search_by_title("Second");
        let results3 = index.search_by_title("Third");

        // Verify each search returns a result and extract tasks
        let result1 = results1.expect("First task should be findable");
        let result2 = results2.expect("Second task should be findable");
        let result3 = results3.expect("Third task should be findable");

        // Get task references
        let tasks1 = result1.tasks();
        let tasks2 = result2.tasks();
        let tasks3 = result3.tasks();

        assert_eq!(tasks1.len(), 1);
        assert_eq!(tasks2.len(), 1);
        assert_eq!(tasks3.len(), 1);
        assert_eq!(tasks1.iter().next().unwrap().task_id, task1.task_id);
        assert_eq!(tasks2.iter().next().unwrap().task_id, task2.task_id);
        assert_eq!(tasks3.iter().next().unwrap().task_id, task3.task_id);
    }

    #[rstest]
    fn test_update_search_index_batch_with_empty_changes_returns_immediately() {
        // Arrange
        let state = create_default_app_state();

        // Add an initial task to verify index is not modified
        let initial_task = Task::new(TaskId::generate(), "Initial Task", Timestamp::now());
        state.update_search_index(TaskChange::Add(initial_task.clone()));

        // Act: Call with empty slice
        state.update_search_index_batch(&[]);

        // Assert: Index should still contain the initial task
        let index_after = state.search_index.load();
        let results = index_after.search_by_title("Initial");

        let result = results.expect("Initial task should still be findable");
        let tasks = result.tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks.iter().next().unwrap().task_id, initial_task.task_id);
    }

    #[rstest]
    fn test_update_search_index_batch_with_mixed_operations() {
        // Arrange
        let state = create_default_app_state();

        // Add initial tasks
        let task_to_update = Task::new(TaskId::generate(), "Update Me", Timestamp::now());
        let task_to_remove = Task::new(TaskId::generate(), "Remove Me", Timestamp::now());
        state.update_search_index(TaskChange::Add(task_to_update.clone()));
        state.update_search_index(TaskChange::Add(task_to_remove.clone()));

        // Prepare batch with mixed operations
        let task_to_add = Task::new(TaskId::generate(), "New Task", Timestamp::now());
        let updated_task = task_to_update
            .clone()
            .with_description("Updated description");

        let changes = vec![
            TaskChange::Add(task_to_add.clone()),
            TaskChange::Update {
                old: task_to_update,
                new: updated_task.clone(),
            },
            TaskChange::Remove(task_to_remove.task_id),
        ];

        // Act
        state.update_search_index_batch(&changes);

        // Assert
        let index = state.search_index.load();

        // New task should be findable
        let new_result = index
            .search_by_title("New")
            .expect("New task should be findable");
        let new_tasks = new_result.tasks();
        assert_eq!(new_tasks.len(), 1);
        assert_eq!(
            new_tasks.iter().next().unwrap().task_id,
            task_to_add.task_id
        );

        // Updated task should still be findable (title unchanged)
        let update_result = index
            .search_by_title("Update")
            .expect("Updated task should be findable");
        let update_tasks = update_result.tasks();
        assert_eq!(update_tasks.len(), 1);
        assert_eq!(
            update_tasks.iter().next().unwrap().task_id,
            updated_task.task_id
        );

        // Removed task should not be findable
        let remove_results = index.search_by_title("Remove");
        assert!(
            remove_results.is_none(),
            "Removed task should not be findable"
        );
    }

    #[rstest]
    fn test_update_search_index_batch_single_change() {
        // Arrange
        let state = create_default_app_state();
        let task = Task::new(TaskId::generate(), "Single Task", Timestamp::now());

        // Act
        state.update_search_index_batch(&[TaskChange::Add(task.clone())]);

        // Assert
        let index = state.search_index.load();
        let result = index
            .search_by_title("Single")
            .expect("Single task should be findable");
        let tasks = result.tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks.iter().next().unwrap().task_id, task.task_id);
    }

    /// RCU リトライ時に変更が欠落しないことを検証する競合テスト
    ///
    /// # テスト戦略
    ///
    /// 1. Barrier で全スレッドの同期開始を保証
    /// 2. 各スレッドが複数回のバッチ更新を実行（リトライ発生を誘発）
    /// 3. 各スレッドで生成した `TaskId` を収集（事後検証用）
    /// 4. 全スレッド完了後、`all_tasks()` の `TaskId` セットと照合して欠落を検出
    ///
    /// # RCU の競合とリトライ
    ///
    /// `ArcSwap::rcu` は Compare-And-Swap (CAS) を使用した楽観的並行制御を行う。
    /// 複数スレッドが同時に更新を試みると、一部のスレッドは CAS 失敗により
    /// リトライを行う。このテストは、リトライ時に変更が欠落しないことを検証する。
    ///
    /// # RCU リトライの確認方法
    ///
    /// `RUST_LOG=search_index_batch=trace` でテストを実行すると、
    /// `update_search_index_batch completed` ログで各バッチ更新の詳細を確認可能。
    /// 競合が発生した場合、`apply_changes_us` が長くなる傾向がある。
    ///
    /// ```bash
    /// RUST_LOG=search_index_batch=trace cargo test test_update_search_index_batch_concurrent_rcu_no_lost_changes -- --nocapture
    /// ```
    ///
    /// # 実装ノート
    ///
    /// `std::sync::Barrier` を使用するため、`std::thread::spawn` で OS スレッドを
    /// 生成して並行実行する。Tokio の非同期コンテキストでは `Barrier::wait()` が
    /// ブロッキング呼び出しとなりデッドロックを引き起こすため、OS スレッドを使用する。
    #[rstest]
    #[tokio::test]
    async fn test_update_search_index_batch_concurrent_rcu_no_lost_changes() {
        use std::collections::HashSet;
        use std::sync::{Arc, Barrier, Mutex};
        use std::thread;

        // Configuration: 8 threads x 20 batches x 5 tasks = 800 total tasks
        // High thread count and batch count to maximize contention probability
        let thread_count: usize = 8;
        let batches_per_thread: usize = 20;
        let tasks_per_batch: usize = 5;
        let total_expected_tasks = thread_count * batches_per_thread * tasks_per_batch;

        // Create empty AppState (no initial tasks)
        let state = Arc::new(create_default_app_state());

        // Barrier to synchronize all threads to start at the same time
        // Note: Using std::sync::Barrier (not tokio::sync::Barrier) as specified
        let barrier = Arc::new(Barrier::new(thread_count));

        // Collect all generated TaskIds from each thread for verification
        // Using Mutex<Vec<(TaskId, String)>> to store (task_id, title) pairs
        let generated_tasks: Arc<Mutex<Vec<(TaskId, String)>>> = Arc::new(Mutex::new(Vec::new()));

        // Spawn OS threads for true concurrent execution with blocking Barrier
        let handles: Vec<_> = (0..thread_count)
            .map(|thread_index| {
                let state = Arc::clone(&state);
                let barrier = Arc::clone(&barrier);
                let generated_tasks = Arc::clone(&generated_tasks);

                thread::spawn(move || {
                    // Wait for all threads to be ready
                    barrier.wait();

                    for batch_index in 0..batches_per_thread {
                        // Create unique tasks for this batch and collect their info
                        let mut batch_task_info: Vec<(TaskId, String)> = Vec::new();
                        let changes: Vec<TaskChange> = (0..tasks_per_batch)
                            .map(|task_index| {
                                // Generate unique task with identifiable title
                                // Format: "Task_T{thread}_B{batch}_I{task}"
                                let title =
                                    format!("Task_T{thread_index}_B{batch_index}_I{task_index}");
                                let task_id = TaskId::generate();
                                let task =
                                    Task::new(task_id.clone(), title.clone(), Timestamp::now());

                                // Record task info for later verification
                                batch_task_info.push((task_id, title));

                                TaskChange::Add(task)
                            })
                            .collect();

                        // Store task info before applying (in case of panic during update)
                        {
                            let mut guard = generated_tasks.lock().unwrap();
                            guard.extend(batch_task_info);
                        }

                        // Apply batch update (may trigger RCU retry under contention)
                        state.update_search_index_batch(&changes);
                    }
                })
            })
            .collect();

        // Wait for all threads to complete
        for handle in handles {
            handle.join().expect("Thread should complete without panic");
        }

        // Retrieve all generated task info
        let all_generated_tasks = generated_tasks.lock().unwrap().clone();
        let expected_task_ids: HashSet<TaskId> = all_generated_tasks
            .iter()
            .map(|(task_id, _)| task_id.clone())
            .collect();

        // Verify: All tasks must be present in the search index using all_tasks()
        let index = state.search_index.load();
        let all_tasks_in_index = index.all_tasks();
        let actual_task_ids: HashSet<TaskId> = all_tasks_in_index
            .iter()
            .map(|task| task.task_id.clone())
            .collect();

        // Primary assertion: count matches
        assert_eq!(
            actual_task_ids.len(),
            total_expected_tasks,
            "Expected {total_expected_tasks} tasks but found {}. RCU retry may have lost changes.",
            actual_task_ids.len()
        );

        // Secondary assertion: all expected TaskIds are present
        let missing_task_ids: Vec<&TaskId> = expected_task_ids
            .iter()
            .filter(|id| !actual_task_ids.contains(*id))
            .collect();

        if !missing_task_ids.is_empty() {
            // Find titles of missing tasks for better error messages
            let missing_tasks: Vec<&str> = all_generated_tasks
                .iter()
                .filter(|(id, _)| missing_task_ids.contains(&id))
                .map(|(_, title)| title.as_str())
                .collect();

            panic!(
                "Missing {} tasks in search index. RCU retry lost these changes: {:?}",
                missing_task_ids.len(),
                missing_tasks
            );
        }

        // Tertiary assertion: no unexpected tasks exist
        let unexpected_task_ids: Vec<&TaskId> = actual_task_ids
            .iter()
            .filter(|id| !expected_task_ids.contains(*id))
            .collect();

        assert!(
            unexpected_task_ids.is_empty(),
            "Found {} unexpected tasks in search index: {:?}",
            unexpected_task_ids.len(),
            unexpected_task_ids
        );

        // Verify that at least one RCU retry occurred (proving contention)
        let retry_count = state.search_index_rcu_retries.load(Ordering::Relaxed);
        assert!(
            retry_count > 0,
            "Expected at least one RCU retry due to contention, but got {retry_count}. \
             Increase thread_count or batches_per_thread to force contention."
        );
    }

    // -------------------------------------------------------------------------
    // health_check Handler Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_health_check_response() {
        use axum::http::header;
        use axum::response::IntoResponse;

        // Act
        let response = health_check().await.into_response();

        // Assert: Status code should be 200 OK
        assert_eq!(response.status(), StatusCode::OK);

        // Assert: Content-Type should be application/json
        let content_type = response.headers().get(header::CONTENT_TYPE);
        assert!(content_type.is_some(), "Content-Type header should be set");
        assert_eq!(
            content_type.unwrap().to_str().unwrap(),
            "application/json",
            "Content-Type should be application/json"
        );

        // Assert: Body should be valid JSON with expected fields
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");
        let health: serde_json::Value =
            serde_json::from_slice(&body).expect("Failed to parse response body as JSON");
        assert_eq!(health["status"], "healthy");
        assert_eq!(health["version"], env!("CARGO_PKG_VERSION"));
    }

    #[rstest]
    #[tokio::test]
    async fn test_health_check_returns_service_version() {
        // Act
        let JsonResponse(response) = health_check().await;

        // Assert
        assert_eq!(response.version, env!("CARGO_PKG_VERSION"));
    }

    // -------------------------------------------------------------------------
    // create_task Handler Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_create_task_returns_201_with_valid_request() {
        use axum::http::header;
        use axum::response::IntoResponse;

        // Arrange
        let state = create_default_app_state();
        let request = CreateTaskRequest {
            title: "New Task".to_string(),
            description: Some("Task description".to_string()),
            priority: super::super::dto::PriorityDto::High,
            tags: vec!["backend".to_string(), "urgent".to_string()],
        };

        // Act
        let result = create_task(State(state), Json(request)).await;

        // Assert
        assert!(result.is_ok(), "create_task should succeed");
        let (status, json_response) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);

        // Verify the combined response (StatusCode, JsonResponse) returns 201
        let response = (status, json_response).into_response();
        assert_eq!(response.status(), StatusCode::CREATED);

        // Verify Content-Type header
        let content_type = response.headers().get(header::CONTENT_TYPE);
        assert!(content_type.is_some());
        assert_eq!(content_type.unwrap().to_str().unwrap(), "application/json");
    }

    #[rstest]
    #[tokio::test]
    async fn test_create_task_response_body_contains_expected_fields() {
        // Arrange
        let state = create_default_app_state();
        let request = CreateTaskRequest {
            title: "Test Task Title".to_string(),
            description: Some("Test description".to_string()),
            priority: super::super::dto::PriorityDto::Critical,
            tags: vec!["tag1".to_string()],
        };

        // Act
        let result = create_task(State(state), Json(request)).await;

        // Assert
        assert!(result.is_ok());
        let (status, JsonResponse(response)) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);

        // Verify response fields
        assert_eq!(response.title, "Test Task Title");
        assert_eq!(response.description, Some("Test description".to_string()));
        assert_eq!(response.priority, super::super::dto::PriorityDto::Critical);
        assert_eq!(response.tags, vec!["tag1".to_string()]);
        assert_eq!(response.status, super::super::dto::TaskStatusDto::Pending);
        assert_eq!(response.version, 1);

        // ID should be a valid UUID
        assert!(
            uuid::Uuid::parse_str(&response.id).is_ok(),
            "Response ID should be a valid UUID"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_create_task_adds_task_to_search_index() {
        // Arrange
        let state = create_default_app_state();
        let request = CreateTaskRequest {
            title: "Searchable Task".to_string(),
            description: None,
            priority: super::super::dto::PriorityDto::Medium,
            tags: vec![],
        };

        // Act
        let result = create_task(State(state.clone()), Json(request)).await;

        // Assert
        assert!(result.is_ok());
        let (_, JsonResponse(response)) = result.unwrap();

        // Verify task is findable in search index
        let index = state.search_index.load();
        let search_result = index.search_by_title("Searchable");
        assert!(search_result.is_some(), "Task should be in search index");

        let result_ref = search_result.unwrap();
        let tasks = result_ref.tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(
            tasks.iter().next().unwrap().task_id.to_string(),
            response.id
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_create_task_returns_400_with_empty_title() {
        // Arrange
        let state = create_default_app_state();
        let request = CreateTaskRequest {
            title: String::new(),
            description: None,
            priority: super::super::dto::PriorityDto::Low,
            tags: vec![],
        };

        // Act
        let result = create_task(State(state), Json(request)).await;

        // Assert
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
    }

    #[rstest]
    #[tokio::test]
    async fn test_create_task_returns_400_with_invalid_tag() {
        // Arrange
        let state = create_default_app_state();
        let request = CreateTaskRequest {
            title: "Valid Title".to_string(),
            description: None,
            priority: super::super::dto::PriorityDto::Low,
            tags: vec![String::new()], // Empty tag is invalid
        };

        // Act
        let result = create_task(State(state), Json(request)).await;

        // Assert
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
    }

    #[rstest]
    #[tokio::test]
    async fn test_create_task_saves_to_repository() {
        // Arrange
        let state = create_default_app_state();
        let request = CreateTaskRequest {
            title: "Persisted Task".to_string(),
            description: Some("Should be saved".to_string()),
            priority: super::super::dto::PriorityDto::High,
            tags: vec![],
        };

        // Act
        let result = create_task(State(state.clone()), Json(request)).await;

        // Assert
        assert!(result.is_ok());
        let (_, JsonResponse(response)) = result.unwrap();

        // Verify task was saved to repository
        let task_id = TaskId::from_uuid(
            uuid::Uuid::parse_str(&response.id).expect("Response ID should be a valid UUID"),
        );
        let found_task = state
            .task_repository
            .find_by_id(&task_id)
            .await
            .expect("Repository should not fail")
            .expect("Task should be found in repository");

        assert_eq!(found_task.title, "Persisted Task");
        assert_eq!(found_task.description, Some("Should be saved".to_string()));
        assert_eq!(found_task.priority, Priority::High);
    }
}
