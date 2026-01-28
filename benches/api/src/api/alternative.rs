//! Alternative operations for fallback and choice patterns.
//!
//! This module demonstrates:
//! - **`alt`**: Fallback chain for data sources
//! - **`choice`**: Select first available from multiple options
//! - **`guard`**: Conditional filtering with early exit
//! - **`optional`**: Tolerate individual failures in aggregation
//!
//! # lambars Features Demonstrated
//!
//! - **`Alternative` trait**: Monoid structure on Applicative functors
//! - **`empty`**: Represents failure or empty computation
//! - **`alt`**: Combines two alternatives, returning first success
//! - **`guard`**: Conditionally succeeds with `()` or fails
//! - **`optional`**: Makes computation optional, converting failure to `None`
//! - **`choice`**: Chooses from multiple alternatives

use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};

use lambars::typeclass::Alternative;

use super::dto::{PriorityDto, TaskResponse};
use super::error::{ApiErrorResponse, FieldError};
use super::handlers::AppState;
use crate::domain::{Priority, Tag, Task, TaskId, TaskStatus};
use crate::infrastructure::{ExternalError, ExternalTaskData, Pagination, RepositoryError};

// =============================================================================
// DTOs
// =============================================================================

/// Query parameters for fallback search.
#[derive(Debug, Deserialize)]
pub struct SearchFallbackQuery {
    /// Search query string (1-100 characters).
    pub query: String,
    /// Data sources to search (cache, database, external). Default: all.
    #[serde(default)]
    pub sources: Option<Vec<String>>,
}

/// Response for fallback search.
#[derive(Debug, Serialize)]
pub struct SearchFallbackResponse {
    pub task: TaskResponse,
    pub source: String,
}

/// Query parameters for config resolution.
#[derive(Debug, Deserialize)]
pub struct ResolveConfigQuery {
    /// Configuration key to resolve.
    pub key: String,
    /// Whether to include source information.
    #[serde(default)]
    pub include_source: Option<bool>,
}

/// Source of a configuration value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigSource {
    Task,
    Project,
    Global,
    Default,
}

/// Response for config resolution.
#[derive(Debug, Serialize)]
pub struct ResolveConfigResponse {
    pub key: String,
    pub value: serde_json::Value,
    pub source: Option<ConfigSource>,
}

/// Request for conditional filtering.
#[derive(Debug, Deserialize)]
pub struct FilterConditionalRequest {
    /// List of task IDs to filter (1-100 items).
    pub task_ids: Vec<String>,
    /// Filter conditions.
    pub conditions: FilterConditions,
}

/// Filter conditions for tasks.
#[derive(Debug, Clone, Deserialize)]
pub struct FilterConditions {
    /// Minimum priority (inclusive).
    pub min_priority: Option<PriorityDto>,
    /// Maximum priority (inclusive).
    pub max_priority: Option<PriorityDto>,
    /// Required status.
    pub status: Option<String>,
    /// Whether task must have a description.
    pub has_description: Option<bool>,
    /// Minimum number of tags.
    pub min_tags: Option<usize>,
}

/// Response for conditional filtering.
#[derive(Debug, Serialize)]
pub struct FilterConditionalResponse {
    pub tasks: Vec<TaskResponse>,
    pub total_input: usize,
    pub filtered_count: usize,
    pub excluded_count: usize,
}

/// Request for source aggregation.
#[derive(Debug, Deserialize)]
pub struct AggregateSourcesRequest {
    /// Task ID to aggregate data for.
    pub task_id: String,
    /// Sources to query (primary, secondary, external).
    pub sources: Vec<String>,
    /// Merge strategy.
    #[serde(default)]
    pub merge_strategy: MergeStrategy,
}

/// Strategy for merging data from multiple sources.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    #[default]
    PreferFirst,
    PreferLatest,
    MergeAll,
}

/// Aggregated task data from multiple sources.
#[derive(Debug, Clone, Serialize)]
pub struct AggregatedTaskDto {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<PriorityDto>,
    pub status: Option<String>,
    pub tags: Vec<String>,
}

/// Response for source aggregation.
#[derive(Debug, Serialize)]
pub struct AggregateSourcesResponse {
    pub task: AggregatedTaskDto,
    pub sources_used: Vec<String>,
    pub sources_failed: Vec<String>,
    pub completeness: f64,
}

/// Query parameters for first available task.
#[derive(Debug, Deserialize)]
pub struct FirstAvailableQuery {
    /// Queues to check (high, medium, low). Default: all in priority order.
    #[serde(default)]
    pub queues: Option<Vec<String>>,
    /// Exclude already assigned tasks.
    #[serde(default)]
    pub exclude_assigned: Option<bool>,
}

/// Response for first available task.
#[derive(Debug, Serialize)]
pub struct FirstAvailableResponse {
    pub task: TaskResponse,
    pub queue: String,
    pub queue_depths: HashMap<String, usize>,
}

// =============================================================================
// Internal Types
// =============================================================================

/// Error type for source data fetching.
///
/// Separates client-safe message from internal details for security.
/// - `client_message`: Safe to expose to API clients (no internal details)
/// - `internal_details`: Full error details for logging/debugging (not exposed)
#[derive(Debug, Clone)]
pub struct SourceError {
    /// Source name (e.g., "primary", "secondary", "external").
    pub source: String,
    /// Client-safe error message (no internal details).
    pub client_message: String,
    /// Internal error details for logging (not exposed to clients).
    /// Reserved for future observability/tracing integration.
    #[allow(dead_code)]
    pub(crate) internal_details: String,
}

impl SourceError {
    /// Creates a new `SourceError` with explicit client message and internal details.
    fn new(
        source: impl Into<String>,
        client_message: impl Into<String>,
        internal_details: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into(),
            client_message: client_message.into(),
            internal_details: internal_details.into(),
        }
    }

    /// Creates a `SourceError` from a `RepositoryError`.
    ///
    /// Converts the internal error to a client-safe message while
    /// preserving the full error details for logging.
    fn from_repository_error(source: &str, error: &RepositoryError) -> Self {
        let client_message = match &error {
            RepositoryError::NotFound(_) => "Resource not found".to_string(),
            RepositoryError::VersionConflict { .. } => "Version conflict".to_string(),
            RepositoryError::DatabaseError(_) => "Database error".to_string(),
            RepositoryError::SerializationError(_) => "Data processing error".to_string(),
            RepositoryError::CacheError(_) => "Cache error".to_string(),
        };

        let internal_details = error.to_string();

        tracing::warn!(
            source = source,
            error = %error,
            "Repository source fetch failed"
        );

        Self::new(source, client_message, internal_details)
    }

    /// Creates a `SourceError` from an `ExternalError`.
    ///
    /// Converts the internal error to a client-safe message while
    /// preserving the full error details for logging.
    fn from_external_error(source: &str, error: &ExternalError) -> Self {
        let client_message = match &error {
            ExternalError::ConnectionFailed(_) => "Connection failed".to_string(),
            ExternalError::Timeout(ms) => format!("Timeout after {ms}ms"),
            ExternalError::ServiceUnavailable(_) => "Service unavailable".to_string(),
            ExternalError::InjectedFailure(_) => "Operation failed".to_string(),
        };

        let internal_details = error.to_string();

        tracing::warn!(
            source = source,
            error = %error,
            "External source fetch failed"
        );

        Self::new(source, client_message, internal_details)
    }
}

/// Simulated data from a source.
#[derive(Debug, Clone)]
struct SourceData {
    title: Option<String>,
    description: Option<String>,
    priority: Option<Priority>,
    status: Option<TaskStatus>,
    tags: Vec<String>,
}

impl SourceData {
    const fn empty() -> Self {
        Self {
            title: None,
            description: None,
            priority: None,
            status: None,
            tags: Vec::new(),
        }
    }

    fn merge(self, other: Self, strategy: MergeStrategy) -> Self {
        match strategy {
            MergeStrategy::PreferFirst => Self {
                title: self.title.alt(other.title),
                description: self.description.alt(other.description),
                priority: self.priority.alt(other.priority),
                status: self.status.alt(other.status),
                tags: if self.tags.is_empty() {
                    other.tags
                } else {
                    self.tags
                },
            },
            MergeStrategy::PreferLatest => Self {
                title: other.title.alt(self.title),
                description: other.description.alt(self.description),
                priority: other.priority.alt(self.priority),
                status: other.status.alt(self.status),
                tags: if other.tags.is_empty() {
                    self.tags
                } else {
                    other.tags
                },
            },
            MergeStrategy::MergeAll => {
                let mut tags = self.tags;
                for tag in other.tags {
                    if !tags.contains(&tag) {
                        tags.push(tag);
                    }
                }
                Self {
                    title: self.title.alt(other.title),
                    description: self
                        .description
                        .map(|d1| {
                            other
                                .description
                                .as_ref()
                                .map_or_else(|| d1.clone(), |d2| format!("{d1} | {d2}"))
                        })
                        .alt(other.description),
                    priority: self.priority.alt(other.priority),
                    status: self.status.alt(other.status),
                    tags,
                }
            }
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Parses a task ID string into `TaskId`.
fn parse_task_id(s: &str) -> Result<TaskId, String> {
    uuid::Uuid::parse_str(s)
        .map(TaskId::from_uuid)
        .map_err(|_| format!("Invalid task ID format: {s}"))
}

/// Parses a status string.
fn parse_status(s: &str) -> Option<TaskStatus> {
    match s.to_lowercase().as_str() {
        "pending" => Some(TaskStatus::Pending),
        "in_progress" => Some(TaskStatus::InProgress),
        "completed" => Some(TaskStatus::Completed),
        "cancelled" => Some(TaskStatus::Cancelled),
        _ => None,
    }
}

// =============================================================================
// GET /tasks/search-fallback - Fallback search with alt
// =============================================================================

/// Searches for a task using fallback sources.
///
/// This handler demonstrates:
/// - **`alt`**: Fallback chain for data sources (cache -> database -> external)
///
/// # Query Parameters
///
/// - `query`: Search query string (1-100 characters)
/// - `sources`: Optional list of sources to search (cache, database, external)
///
/// # Errors
///
/// - `400 Bad Request`: Invalid query
/// - `404 Not Found`: Task not found in any source
pub async fn search_fallback(
    State(state): State<AppState>,
    Query(query): Query<SearchFallbackQuery>,
) -> Result<Json<SearchFallbackResponse>, ApiErrorResponse> {
    // Validate query
    if query.query.trim().is_empty() {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("query", "query cannot be empty")],
        ));
    }

    if query.query.len() > 100 {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new(
                "query",
                "query cannot exceed 100 characters",
            )],
        ));
    }

    // Determine which sources to search
    let sources = query.sources.unwrap_or_else(|| {
        vec![
            "cache".to_string(),
            "database".to_string(),
            "external".to_string(),
        ]
    });

    // Validate source names
    let valid_sources = ["cache", "database", "external"];
    for source in &sources {
        if !valid_sources.contains(&source.as_str()) {
            return Err(ApiErrorResponse::validation_error(
                "Validation failed",
                vec![FieldError::new(
                    "sources",
                    format!("invalid source: {source}. Must be one of: cache, database, external"),
                )],
            ));
        }
    }

    // Build search chain using alt
    let search_query = query.query.trim().to_lowercase();

    // Search in each source and use alt for fallback
    let mut result: Option<(Task, String)> = None;

    for source in &sources {
        if result.is_some() {
            break;
        }

        let found = match source.as_str() {
            "cache" => {
                // Cache lookup: uses search cache for quick results
                search_in_cache(&state, &search_query)
            }
            "database" => {
                // Database: uses primary repository
                search_in_database(&state.task_repository, &search_query).await
            }
            "external" => {
                // External API: uses real ExternalDataSource (HTTP)
                // Note: External search requires a task ID, so we search by title in database first
                // then fetch additional data from external source
                search_in_external(&state, &search_query).await
            }
            _ => None,
        };

        // Use alt pattern: result = result.alt(found)
        result = result.alt(found);
    }

    match result {
        Some((task, source)) => Ok(Json(SearchFallbackResponse {
            task: TaskResponse::from(&task),
            source,
        })),
        None => Err(ApiErrorResponse::not_found(format!(
            "No task found matching query: {}",
            query.query
        ))),
    }
}

/// Searches for a task in the search cache.
///
/// Uses the in-memory search index for fast lookups.
fn search_in_cache(state: &AppState, query: &str) -> Option<(Task, String)> {
    // Use the search index for cache lookups
    let search_index = state.search_index.load();

    // Search by title in the index
    let search_result = search_index.search_by_title(query)?;

    // Get the first matching task
    search_result
        .tasks()
        .iter()
        .next()
        .cloned()
        .map(|task| (task, "cache".to_string()))
}

/// Searches for a task in the database.
///
/// Uses the primary repository for database lookups.
async fn search_in_database(
    repository: &Arc<dyn crate::infrastructure::TaskRepository + Send + Sync>,
    query: &str,
) -> Option<(Task, String)> {
    // Get tasks with pagination (simplified search)
    let pagination = Pagination::new(0, 100);
    let result = repository.list(pagination).run_async().await.ok()?;

    result
        .items
        .into_iter()
        .find(|task| task.title.to_lowercase().contains(query))
        .map(|task| (task, "database".to_string()))
}

/// Searches for a task using external data source.
///
/// This demonstrates real I/O with the external HTTP source.
/// Since external sources typically require a task ID, we first search
/// in the database to find matching tasks, then enrich with external data.
async fn search_in_external(state: &AppState, query: &str) -> Option<(Task, String)> {
    // First, find a task by title in the database
    let pagination = Pagination::new(0, 100);
    let result = state
        .task_repository
        .list(pagination)
        .run_async()
        .await
        .ok()?;

    let matching_task = result
        .items
        .into_iter()
        .find(|task| task.title.to_lowercase().contains(query))?;

    // Try to fetch additional data from external source
    // This demonstrates real I/O via ExternalDataSource
    let external_result = state
        .external_source
        .fetch_task_data(&matching_task.task_id)
        .run_async()
        .await;

    match external_result {
        Ok(Some(external_data)) => {
            // Merge external data with local task
            let enriched_task = enrich_task_with_external_data(&matching_task, &external_data);
            Some((enriched_task, "external".to_string()))
        }
        Ok(None) => {
            // External source returned no data, return local task
            Some((matching_task, "external".to_string()))
        }
        Err(error) => {
            // External source failed, log and return None
            tracing::warn!(
                task_id = %matching_task.task_id,
                error = %error,
                "External source fetch failed in search_fallback"
            );
            None
        }
    }
}

/// Enriches a task with data from an external source (pure function).
fn enrich_task_with_external_data(task: &Task, external: &ExternalTaskData) -> Task {
    let mut enriched = task.clone();

    // Override fields if external data is present
    if let Some(description) = &external.description {
        enriched.description = Some(description.clone());
    }
    if let Some(priority) = external.priority {
        enriched.priority = priority;
    }
    if let Some(status) = external.status {
        enriched.status = status;
    }
    if !external.tags.is_empty() {
        // Merge tags from both sources using functional style
        // Convert existing tags to a set of strings for deduplication
        let existing_tag_strings: std::collections::HashSet<String> = enriched
            .tags
            .iter()
            .map(|tag| tag.as_str().to_string())
            .collect();

        // Add new tags that don't already exist
        for tag_string in &external.tags {
            if !existing_tag_strings.contains(tag_string) {
                enriched.tags = enriched.tags.insert(Tag::new(tag_string));
            }
        }
    }

    enriched
}

// =============================================================================
// GET /tasks/{id}/config - Hierarchical config resolution with choice
// =============================================================================

/// Resolves a configuration value for a task using hierarchical lookup.
///
/// This handler demonstrates:
/// - **`choice`**: Select first available from multiple config sources
///
/// Config resolution order: task -> project -> global -> default
///
/// # Path Parameters
///
/// - `id`: Task ID
///
/// # Query Parameters
///
/// - `key`: Configuration key to resolve
/// - `include_source`: Whether to include source information
///
/// # Errors
///
/// - `400 Bad Request`: Invalid key
/// - `404 Not Found`: Task not found
#[allow(clippy::unused_async)]
pub async fn resolve_config(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Query(query): Query<ResolveConfigQuery>,
) -> Result<Json<ResolveConfigResponse>, ApiErrorResponse> {
    // Validate key
    if query.key.trim().is_empty() {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("key", "key cannot be empty")],
        ));
    }

    // Parse and validate task ID
    let id = parse_task_id(&task_id).map_err(|error| {
        ApiErrorResponse::validation_error("Validation failed", vec![FieldError::new("id", error)])
    })?;

    // Verify task exists
    let repository = Arc::clone(&state.task_repository);
    let task = repository
        .find_by_id(&id)
        .run_async()
        .await
        .map_err(|e| ApiErrorResponse::internal_error(e.to_string()))?
        .ok_or_else(|| ApiErrorResponse::not_found(format!("Task not found: {task_id}")))?;

    // Resolve config using choice pattern
    let (value, source) = resolve_config_hierarchy(&task, &query.key);

    Ok(Json(ResolveConfigResponse {
        key: query.key,
        value,
        source: if query.include_source.unwrap_or(false) {
            Some(source)
        } else {
            None
        },
    }))
}

/// Pure: Resolves a configuration value using hierarchical lookup.
///
/// Uses `choice` to select the first available configuration from:
/// task config -> project config -> global config -> default
///
/// If no configuration is found at any level, returns `Alternative::empty()` which is `None`,
/// and falls back to `(Null, Default)`.
fn resolve_config_hierarchy(task: &Task, key: &str) -> (serde_json::Value, ConfigSource) {
    // Simulate config sources (in real app, these would come from config stores)
    let task_config = get_task_config(task, key);
    let project_config = get_project_config(key);
    let global_config = get_global_config(key);
    let default_config = get_default_config(key);

    // Use choice to find first available
    // All sources can return None, demonstrating Alternative::empty usage
    let configs = vec![
        task_config.map(|v| (v, ConfigSource::Task)),
        project_config.map(|v| (v, ConfigSource::Project)),
        global_config.map(|v| (v, ConfigSource::Global)),
        default_config.map(|v| (v, ConfigSource::Default)),
    ];

    // When all configs are None, choice returns empty() (None)
    // Then unwrap_or provides the final fallback
    Option::choice(configs).unwrap_or((serde_json::Value::Null, ConfigSource::Default))
}

/// Simulates task-level configuration lookup.
fn get_task_config(task: &Task, key: &str) -> Option<serde_json::Value> {
    // Use task properties as config values for demo
    match key {
        "priority" => Some(serde_json::json!(format!("{:?}", task.priority))),
        "status" => Some(serde_json::json!(format!("{:?}", task.status))),
        "title" => Some(serde_json::json!(task.title.clone())),
        _ => None,
    }
}

/// Simulates project-level configuration lookup.
fn get_project_config(key: &str) -> Option<serde_json::Value> {
    match key {
        "default_priority" => Some(serde_json::json!("Medium")),
        "max_subtasks" => Some(serde_json::json!(10)),
        _ => None,
    }
}

/// Simulates global configuration lookup.
fn get_global_config(key: &str) -> Option<serde_json::Value> {
    match key {
        "max_title_length" => Some(serde_json::json!(200)),
        "default_priority" => Some(serde_json::json!("Low")),
        _ => None,
    }
}

/// Simulates default configuration values.
///
/// Returns `None` for unknown keys, demonstrating that `Alternative::empty`
/// can be returned when no default exists.
fn get_default_config(key: &str) -> Option<serde_json::Value> {
    match key {
        "timeout_seconds" => Some(serde_json::json!(30)),
        "retry_count" => Some(serde_json::json!(3)),
        _ => None, // Unknown keys have no default, returns Alternative::empty()
    }
}

// =============================================================================
// POST /tasks/filter-conditional - Guard-based filtering
// =============================================================================

/// Filters tasks based on conditions using guard.
///
/// This handler demonstrates:
/// - **`guard`**: Conditional filtering with early exit
///
/// # Request Body
///
/// - `task_ids`: List of task IDs to filter (1-100 items)
/// - `conditions`: Filter conditions
///
/// # Errors
///
/// - `400 Bad Request`: Invalid request
pub async fn filter_conditional(
    State(state): State<AppState>,
    Json(request): Json<FilterConditionalRequest>,
) -> Result<Json<FilterConditionalResponse>, ApiErrorResponse> {
    // Validate batch size
    if request.task_ids.is_empty() {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("task_ids", "task_ids list cannot be empty")],
        ));
    }

    if request.task_ids.len() > 100 {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new(
                "task_ids",
                "task_ids list cannot exceed 100 items",
            )],
        ));
    }

    // Parse task IDs
    let task_ids: Result<Vec<TaskId>, _> = request
        .task_ids
        .iter()
        .map(|id| parse_task_id(id))
        .collect();
    let task_ids = task_ids.map_err(|error| {
        ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("task_ids", error)],
        )
    })?;

    // Fetch tasks
    let repository = Arc::clone(&state.task_repository);
    let mut tasks = Vec::new();
    for task_id in &task_ids {
        if let Ok(Some(task)) = repository.find_by_id(task_id).run_async().await {
            tasks.push(task);
        }
    }

    let total_input = tasks.len();

    // Apply guard-based filtering
    let filtered_tasks = filter_tasks_with_guard(&tasks, &request.conditions);

    let filtered_count = filtered_tasks.len();
    let excluded_count = total_input - filtered_count;

    let task_responses: Vec<TaskResponse> = filtered_tasks
        .iter()
        .map(|t| TaskResponse::from(*t))
        .collect();

    Ok(Json(FilterConditionalResponse {
        tasks: task_responses,
        total_input,
        filtered_count,
        excluded_count,
    }))
}

/// Pure: Filters tasks using guard-based conditions.
fn filter_tasks_with_guard<'a>(tasks: &'a [Task], conditions: &FilterConditions) -> Vec<&'a Task> {
    tasks
        .iter()
        .filter_map(|task| apply_guard_conditions(task, conditions))
        .collect()
}

/// Pure: Applies guard conditions to a single task.
///
/// Returns `Some(task)` if all conditions pass, `None` otherwise.
fn apply_guard_conditions<'a>(task: &'a Task, conditions: &FilterConditions) -> Option<&'a Task> {
    // Check minimum priority
    <Option<()>>::guard(
        conditions
            .min_priority
            .as_ref()
            .is_none_or(|min| task.priority >= Priority::from(*min)),
    )?;

    // Check maximum priority
    <Option<()>>::guard(
        conditions
            .max_priority
            .as_ref()
            .is_none_or(|max| task.priority <= Priority::from(*max)),
    )?;

    // Check status
    <Option<()>>::guard(
        conditions
            .status
            .as_ref()
            .is_none_or(|s| parse_status(s).is_some_and(|expected| task.status == expected)),
    )?;

    // Check has_description
    <Option<()>>::guard(
        conditions
            .has_description
            .is_none_or(|has| task.description.is_some() == has),
    )?;

    // Check minimum tags
    <Option<()>>::guard(conditions.min_tags.is_none_or(|min| task.tags.len() >= min))?;

    Some(task)
}

// =============================================================================
// POST /tasks/aggregate-sources - Optional-based aggregation
// =============================================================================

/// Aggregates task data from multiple sources.
///
/// This handler demonstrates:
/// - **`optional`**: Tolerate individual source failures
///
/// # Request Body
///
/// - `task_id`: Task ID to aggregate data for
/// - `sources`: Sources to query (primary, secondary, external)
/// - `merge_strategy`: How to merge data from sources
///
/// # Errors
///
/// - `400 Bad Request`: Invalid request
/// - `503 Service Unavailable`: All sources failed
pub async fn aggregate_sources(
    State(state): State<AppState>,
    Json(request): Json<AggregateSourcesRequest>,
) -> Result<Json<AggregateSourcesResponse>, ApiErrorResponse> {
    // Validate task ID
    let task_id = parse_task_id(&request.task_id).map_err(|error| {
        ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("task_id", error)],
        )
    })?;

    // Validate sources
    if request.sources.is_empty() {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("sources", "sources list cannot be empty")],
        ));
    }

    let valid_sources = ["primary", "secondary", "external"];
    for source in &request.sources {
        if !valid_sources.contains(&source.as_str()) {
            return Err(ApiErrorResponse::validation_error(
                "Validation failed",
                vec![FieldError::new(
                    "sources",
                    format!(
                        "invalid source: {source}. Must be one of: primary, secondary, external"
                    ),
                )],
            ));
        }
    }

    // Fetch from each source using optional pattern
    let mut sources_used = Vec::new();
    let mut sources_failed = Vec::new();
    let mut aggregated_data = SourceData::empty();

    for source in &request.sources {
        // fetch_from_source returns Result<SourceData, SourceError>
        let source_result = fetch_from_source(&state, &task_id, source).await;

        // Use optional: convert Err to Some(None), Ok to Some(Some)
        // This demonstrates Alternative::optional which tolerates failures
        let optional_result = source_result.ok().optional();

        match optional_result {
            Some(Some(data)) => {
                sources_used.push(source.clone());
                aggregated_data = aggregated_data.merge(data, request.merge_strategy);
            }
            Some(None) | None => {
                sources_failed.push(source.clone());
            }
        }
    }

    // Check if any source succeeded - return 503 Service Unavailable if all failed
    if sources_used.is_empty() {
        return Err(ApiErrorResponse::service_unavailable(
            "All sources failed to provide data",
        ));
    }

    // Calculate completeness
    let completeness = calculate_completeness(&aggregated_data);

    Ok(Json(AggregateSourcesResponse {
        task: AggregatedTaskDto {
            id: request.task_id,
            title: aggregated_data.title,
            description: aggregated_data.description,
            priority: aggregated_data.priority.map(PriorityDto::from),
            status: aggregated_data.status.map(|s| format!("{s:?}")),
            tags: aggregated_data.tags,
        },
        sources_used,
        sources_failed,
        completeness,
    }))
}

/// Fetches data from a specific source.
///
/// This function uses real I/O adapters for secondary and external sources.
/// The sources are configured via `AppState` and may include fail injection.
///
/// Returns `Result<SourceData, SourceError>` to properly demonstrate `Alternative::optional`.
/// - `Ok(data)` when the source successfully provides data
/// - `Err(error)` when the source fails (timeout, unavailable, not found)
async fn fetch_from_source(
    state: &AppState,
    task_id: &TaskId,
    source: &str,
) -> Result<SourceData, SourceError> {
    match source {
        "primary" => {
            // Primary source: actual repository
            fetch_from_primary(&state.task_repository, task_id).await
        }
        "secondary" => {
            // Secondary source: Redis via ExternalDataSource
            fetch_from_external_source(&state.secondary_source, task_id, "secondary").await
        }
        "external" => {
            // External source: HTTP via ExternalDataSource
            fetch_from_external_source(&state.external_source, task_id, "external").await
        }
        _ => Err(SourceError::new(
            source,
            "Unknown source",
            format!("Unknown source: {source}"),
        )),
    }
}

/// Fetches data from the primary repository.
async fn fetch_from_primary(
    repository: &Arc<dyn crate::infrastructure::TaskRepository + Send + Sync>,
    task_id: &TaskId,
) -> Result<SourceData, SourceError> {
    let task = repository
        .find_by_id(task_id)
        .run_async()
        .await
        .map_err(|error| SourceError::from_repository_error("primary", &error))?
        .ok_or_else(|| {
            SourceError::new(
                "primary",
                "Task not found",
                format!("Task not found: {task_id}"),
            )
        })?;

    Ok(SourceData {
        title: Some(task.title.clone()),
        description: task.description.clone(),
        priority: Some(task.priority),
        status: Some(task.status),
        tags: task.tags.iter().map(ToString::to_string).collect(),
    })
}

/// Fetches data from an external source (secondary or external).
async fn fetch_from_external_source(
    source: &Arc<dyn crate::infrastructure::ExternalDataSource + Send + Sync>,
    task_id: &TaskId,
    source_name: &str,
) -> Result<SourceData, SourceError> {
    let external_data = source
        .fetch_task_data(task_id)
        .run_async()
        .await
        .map_err(|error| SourceError::from_external_error(source_name, &error))?
        .ok_or_else(|| {
            SourceError::new(
                source_name,
                "Task not found in external source",
                format!("Task not found in {source_name}: {task_id}"),
            )
        })?;

    Ok(convert_external_task_data_to_source_data(external_data))
}

/// Converts `ExternalTaskData` to `SourceData` (pure function).
fn convert_external_task_data_to_source_data(external: ExternalTaskData) -> SourceData {
    SourceData {
        title: external.title,
        description: external.description,
        priority: external.priority,
        status: external.status,
        tags: external.tags,
    }
}

/// Pure: Calculates data completeness score (0.0 - 1.0).
#[allow(clippy::cast_precision_loss)] // Small counts, precision loss is acceptable
fn calculate_completeness(data: &SourceData) -> f64 {
    let fields = [
        data.title.is_some(),
        data.description.is_some(),
        data.priority.is_some(),
        data.status.is_some(),
        !data.tags.is_empty(),
    ];

    let present = fields.iter().filter(|&&b| b).count();
    present as f64 / fields.len() as f64
}

// =============================================================================
// GET /tasks/first-available - Choice-based queue selection
// =============================================================================

/// Gets the first available task from priority queues.
///
/// This handler demonstrates:
/// - **`choice`**: Select first available from multiple queues
///
/// Queue priority order: high -> medium -> low
///
/// # Query Parameters
///
/// - `queues`: Optional list of queues to check
/// - `exclude_assigned`: Whether to exclude assigned tasks
///
/// # Errors
///
/// - `400 Bad Request`: Invalid queue name
/// - `404 Not Found`: No task available in any queue
pub async fn first_available(
    State(state): State<AppState>,
    Query(query): Query<FirstAvailableQuery>,
) -> Result<Json<FirstAvailableResponse>, ApiErrorResponse> {
    // Determine which queues to check
    let queues = query
        .queues
        .unwrap_or_else(|| vec!["high".to_string(), "medium".to_string(), "low".to_string()]);

    // Validate queue names
    let valid_queues = ["high", "medium", "low"];
    for queue in &queues {
        if !valid_queues.contains(&queue.as_str()) {
            return Err(ApiErrorResponse::validation_error(
                "Validation failed",
                vec![FieldError::new(
                    "queues",
                    format!("invalid queue: {queue}. Must be one of: high, medium, low"),
                )],
            ));
        }
    }

    // Get all tasks and simulate queues based on priority
    let repository = Arc::clone(&state.task_repository);
    let pagination = Pagination::new(0, 1000);
    let result = repository
        .list(pagination)
        .run_async()
        .await
        .map_err(|e| ApiErrorResponse::internal_error(e.to_string()))?;
    let all_tasks = result.items;

    let exclude_assigned = query.exclude_assigned.unwrap_or(false);

    // Build queue contents (owned tasks)
    let (high_queue, medium_queue, low_queue) = build_priority_queues(all_tasks, exclude_assigned);

    // Calculate queue depths
    let queue_depths = HashMap::from([
        ("high".to_string(), high_queue.len()),
        ("medium".to_string(), medium_queue.len()),
        ("low".to_string(), low_queue.len()),
    ]);

    // Use choice to select first available from requested queues
    // Build options with cloned tasks to avoid lifetime issues
    let queue_options: Vec<Option<(Task, String)>> = queues
        .iter()
        .map(|q| match q.as_str() {
            "high" => high_queue.first().map(|t| (t.clone(), "high".to_string())),
            "medium" => medium_queue
                .first()
                .map(|t| (t.clone(), "medium".to_string())),
            "low" => low_queue.first().map(|t| (t.clone(), "low".to_string())),
            _ => None,
        })
        .collect();

    let result = Option::choice(queue_options);

    match result {
        Some((task, queue)) => Ok(Json(FirstAvailableResponse {
            task: TaskResponse::from(&task),
            queue,
            queue_depths,
        })),
        None => Err(ApiErrorResponse::not_found(
            "No task available in any queue",
        )),
    }
}

/// Pure: Builds priority-based task queues (returns owned tasks).
fn build_priority_queues(
    tasks: Vec<Task>,
    exclude_assigned: bool,
) -> (Vec<Task>, Vec<Task>, Vec<Task>) {
    let filtered: Vec<Task> = tasks
        .into_iter()
        .filter(|task| {
            // Only include pending tasks
            if task.status != TaskStatus::Pending {
                return false;
            }
            // Optionally exclude assigned (simulated by having subtasks)
            if exclude_assigned && !task.subtasks.is_empty() {
                return false;
            }
            true
        })
        .collect();

    let high: Vec<Task> = filtered
        .iter()
        .filter(|t| t.priority == Priority::Critical || t.priority == Priority::High)
        .cloned()
        .collect();

    let medium: Vec<Task> = filtered
        .iter()
        .filter(|t| t.priority == Priority::Medium)
        .cloned()
        .collect();

    let low: Vec<Task> = filtered
        .into_iter()
        .filter(|t| t.priority == Priority::Low)
        .collect();

    (high, medium, low)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Timestamp;
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // Guard Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_guard_true_returns_some() {
        let result: Option<()> = <Option<()>>::guard(true);
        assert_eq!(result, Some(()));
    }

    #[rstest]
    fn test_guard_false_returns_none() {
        let result: Option<()> = <Option<()>>::guard(false);
        assert_eq!(result, None);
    }

    #[rstest]
    fn test_guard_with_map() {
        fn filter_positive(n: i32) -> Option<i32> {
            <Option<()>>::guard(n > 0).map(move |()| n)
        }

        assert_eq!(filter_positive(5), Some(5));
        assert_eq!(filter_positive(-3), None);
        assert_eq!(filter_positive(0), None);
    }

    // -------------------------------------------------------------------------
    // Alt Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_alt_none_some() {
        let first: Option<i32> = None;
        let second: Option<i32> = Some(42);
        assert_eq!(first.alt(second), Some(42));
    }

    #[rstest]
    fn test_alt_some_none() {
        let first: Option<i32> = Some(1);
        let second: Option<i32> = None;
        assert_eq!(first.alt(second), Some(1));
    }

    #[rstest]
    fn test_alt_some_some() {
        let first: Option<i32> = Some(1);
        let second: Option<i32> = Some(2);
        assert_eq!(first.alt(second), Some(1));
    }

    // -------------------------------------------------------------------------
    // Choice Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_choice_finds_first_some() {
        let alternatives = vec![None, Some(1), Some(2)];
        let result: Option<i32> = Option::choice(alternatives);
        assert_eq!(result, Some(1));
    }

    #[rstest]
    fn test_choice_all_none() {
        let alternatives: Vec<Option<i32>> = vec![None, None, None];
        let result: Option<i32> = Option::choice(alternatives);
        assert_eq!(result, None);
    }

    #[rstest]
    fn test_choice_first_is_some() {
        let alternatives = vec![Some(1), Some(2), Some(3)];
        let result: Option<i32> = Option::choice(alternatives);
        assert_eq!(result, Some(1));
    }

    // -------------------------------------------------------------------------
    // Optional Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_optional_some() {
        let value: Option<i32> = Some(42);
        assert_eq!(value.optional(), Some(Some(42)));
    }

    #[rstest]
    fn test_optional_none() {
        let value: Option<i32> = None;
        assert_eq!(value.optional(), Some(None));
    }

    // -------------------------------------------------------------------------
    // Apply Guard Conditions Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_apply_guard_no_conditions() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());
        let conditions = FilterConditions {
            min_priority: None,
            max_priority: None,
            status: None,
            has_description: None,
            min_tags: None,
        };

        let result = apply_guard_conditions(&task, &conditions);
        assert!(result.is_some());
    }

    #[rstest]
    fn test_apply_guard_priority_pass() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now())
            .with_priority(Priority::High);
        let conditions = FilterConditions {
            min_priority: Some(PriorityDto::Medium),
            max_priority: None,
            status: None,
            has_description: None,
            min_tags: None,
        };

        let result = apply_guard_conditions(&task, &conditions);
        assert!(result.is_some());
    }

    #[rstest]
    fn test_apply_guard_priority_fail() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now())
            .with_priority(Priority::Low);
        let conditions = FilterConditions {
            min_priority: Some(PriorityDto::High),
            max_priority: None,
            status: None,
            has_description: None,
            min_tags: None,
        };

        let result = apply_guard_conditions(&task, &conditions);
        assert!(result.is_none());
    }

    #[rstest]
    fn test_apply_guard_description_required() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());
        let conditions = FilterConditions {
            min_priority: None,
            max_priority: None,
            status: None,
            has_description: Some(true),
            min_tags: None,
        };

        let result = apply_guard_conditions(&task, &conditions);
        assert!(result.is_none());

        let task_with_desc = task.with_description("Description".to_string());
        let result = apply_guard_conditions(&task_with_desc, &conditions);
        assert!(result.is_some());
    }

    // -------------------------------------------------------------------------
    // Config Resolution Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_resolve_config_task_level() {
        let task = Task::new(
            TaskId::generate(),
            "Test Task".to_string(),
            Timestamp::now(),
        )
        .with_priority(Priority::Critical);

        let (value, source) = resolve_config_hierarchy(&task, "priority");
        assert_eq!(source, ConfigSource::Task);
        assert!(value.as_str().is_some());
    }

    #[rstest]
    fn test_resolve_config_project_level() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());

        let (value, source) = resolve_config_hierarchy(&task, "default_priority");
        assert_eq!(source, ConfigSource::Project);
        assert_eq!(value.as_str(), Some("Medium"));
    }

    #[rstest]
    fn test_resolve_config_global_level() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());

        let (value, source) = resolve_config_hierarchy(&task, "max_title_length");
        assert_eq!(source, ConfigSource::Global);
        assert_eq!(value.as_i64(), Some(200));
    }

    #[rstest]
    fn test_resolve_config_default_level() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());

        let (value, source) = resolve_config_hierarchy(&task, "timeout_seconds");
        assert_eq!(source, ConfigSource::Default);
        assert_eq!(value.as_i64(), Some(30));
    }

    #[rstest]
    fn test_resolve_config_unknown_key_returns_null() {
        // Test that unknown keys exercise Alternative::empty / choice failure
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());

        let (value, source) = resolve_config_hierarchy(&task, "completely_unknown_key");
        // All sources return None, choice returns empty(), unwrap_or provides fallback
        assert_eq!(source, ConfigSource::Default);
        assert!(value.is_null());
    }

    // -------------------------------------------------------------------------
    // Source Data Merge Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_merge_prefer_first() {
        let first = SourceData {
            title: Some("First".to_string()),
            description: None,
            priority: Some(Priority::High),
            status: None,
            tags: vec!["a".to_string()],
        };
        let second = SourceData {
            title: Some("Second".to_string()),
            description: Some("Desc".to_string()),
            priority: Some(Priority::Low),
            status: Some(TaskStatus::Completed),
            tags: vec!["b".to_string()],
        };

        let merged = first.merge(second, MergeStrategy::PreferFirst);
        assert_eq!(merged.title, Some("First".to_string()));
        assert_eq!(merged.description, Some("Desc".to_string()));
        assert_eq!(merged.priority, Some(Priority::High));
        assert_eq!(merged.status, Some(TaskStatus::Completed));
        assert_eq!(merged.tags, vec!["a".to_string()]);
    }

    #[rstest]
    fn test_merge_prefer_latest() {
        let first = SourceData {
            title: Some("First".to_string()),
            description: None,
            priority: Some(Priority::High),
            status: None,
            tags: vec!["a".to_string()],
        };
        let second = SourceData {
            title: Some("Second".to_string()),
            description: Some("Desc".to_string()),
            priority: None,
            status: Some(TaskStatus::Completed),
            tags: vec!["b".to_string()],
        };

        let merged = first.merge(second, MergeStrategy::PreferLatest);
        assert_eq!(merged.title, Some("Second".to_string()));
        assert_eq!(merged.description, Some("Desc".to_string()));
        assert_eq!(merged.priority, Some(Priority::High));
        assert_eq!(merged.status, Some(TaskStatus::Completed));
        assert_eq!(merged.tags, vec!["b".to_string()]);
    }

    // -------------------------------------------------------------------------
    // Completeness Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_completeness_full() {
        let data = SourceData {
            title: Some("Title".to_string()),
            description: Some("Desc".to_string()),
            priority: Some(Priority::High),
            status: Some(TaskStatus::Pending),
            tags: vec!["tag".to_string()],
        };

        let completeness = calculate_completeness(&data);
        assert!((completeness - 1.0).abs() < f64::EPSILON);
    }

    #[rstest]
    fn test_completeness_empty() {
        let data = SourceData::empty();
        let completeness = calculate_completeness(&data);
        assert!((completeness - 0.0).abs() < f64::EPSILON);
    }

    #[rstest]
    fn test_completeness_partial() {
        let data = SourceData {
            title: Some("Title".to_string()),
            description: None,
            priority: Some(Priority::High),
            status: None,
            tags: vec![],
        };

        let completeness = calculate_completeness(&data);
        assert!((completeness - 0.4).abs() < f64::EPSILON);
    }
}
