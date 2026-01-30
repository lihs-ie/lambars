//! Bulk operation handlers for task management.
//!
//! This module demonstrates lambars' functional programming patterns for bulk operations:
//! - **`Either`**: Representing success/failure for individual items
//! - **`Bifunctor::bimap`**: Transforming both success and failure cases
//! - **`PersistentHashSet`**: Deduplication of IDs
//! - **`Alternative`**: Fallback patterns for validation and save operations
//!
//! # Endpoints
//!
//! - `POST /tasks/bulk` - Create multiple tasks at once
//! - `PUT /tasks/bulk` - Update multiple tasks at once
//!
//! # Design Note
//!
//! Bulk operations use 207 Multi-Status to report partial success.
//! Each item is processed independently - one failure doesn't affect others.
//!
//! # Alternative Pattern Usage
//!
//! - **Validation**: `alt` for field-level fallback (e.g., default priority)
//! - **Save**: `alt` for fallback save strategy on primary failure
//! - **Choice**: Select first successful result from multiple strategies
//!
//! # Future Work (Phase 5)
//!
//! The following improvements are planned for Phase 5:
//! - **Production-ready fallback**: Currently fallback retries the same repository.
//!   In production, this should use an alternative repository, message queue, or
//!   circuit breaker pattern.
//! - **Retry with backoff**: Add exponential backoff for transient failures.
//! - **Partial failure recovery**: Store failed items for later retry.

use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};

use super::json_buffer::JsonResponse;
use serde::{Deserialize, Serialize};

use super::consistency::{ConsistencyError, log_consistency_error};
use super::dto::{
    CreateTaskRequest, PriorityDto, TaskResponse, TaskStatusDto, validate_description,
    validate_tags, validate_title,
};
use super::error::ApiErrorResponse;
use super::handlers::AppState;
use super::query::TaskChange;
use crate::domain::{
    EventId, Priority, Tag, Task, TaskId, TaskStatus, Timestamp, create_task_created_event,
};
use crate::infrastructure::TaskRepository;
use lambars::control::Either;
use lambars::effect::AsyncIO;
use lambars::for_;
use lambars::persistent::PersistentHashSet;
use lambars::typeclass::Alternative;

// =============================================================================
// Constants
// =============================================================================

/// Maximum number of items in a bulk request.
const BULK_LIMIT: usize = 100;

// =============================================================================
// Chunk Configuration
// =============================================================================

/// Configuration for bulk operation chunking.
///
/// This struct controls how large bulk operations are split into smaller
/// chunks for processing. Chunking helps:
/// - Avoid database connection pool exhaustion
/// - Maintain responsive error handling
/// - Enable parallel processing with controlled concurrency
///
/// # Examples
///
/// ```ignore
/// // Use default configuration (chunk_size: 50, concurrency_limit: 4)
/// let config = BulkConfig::default();
///
/// // Create from environment variables
/// let config = BulkConfig::from_env();
///
/// // Custom configuration
/// let config = BulkConfig {
///     chunk_size: 100,
///     concurrency_limit: 8,
///     use_bulk_optimization: true,
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BulkConfig {
    /// Maximum number of tasks per chunk for bulk operations.
    ///
    /// Default: 50 (optimized for `PostgreSQL` batch operations)
    pub chunk_size: usize,
    /// Maximum number of concurrent chunk operations.
    ///
    /// Default: 4 (conservative to avoid DB connection pool exhaustion)
    pub concurrency_limit: usize,
    /// Whether to use the optimized bulk processing implementation.
    ///
    /// When `true` (default), uses `save_tasks_bulk_optimized` with parallel chunk processing.
    /// When `false`, uses the legacy `save_tasks_bulk` for sequential processing.
    ///
    /// This flag enables rollback to the legacy implementation if issues are discovered.
    ///
    /// Environment variable: `USE_BULK_OPTIMIZATION` (default: `true`)
    pub use_bulk_optimization: bool,
}

impl Default for BulkConfig {
    fn default() -> Self {
        Self {
            chunk_size: 50,              // PostgreSQL optimal batch size
            concurrency_limit: 4,        // DB connection pool consideration
            use_bulk_optimization: true, // Use optimized implementation by default
        }
    }
}

impl BulkConfig {
    /// Default chunk size for bulk operations.
    const DEFAULT_CHUNK_SIZE: usize = 50;
    /// Default concurrency limit for bulk operations.
    const DEFAULT_CONCURRENCY_LIMIT: usize = 4;
    /// Default value for bulk optimization feature flag.
    const DEFAULT_USE_BULK_OPTIMIZATION: bool = true;

    /// Creates configuration from environment variables or defaults.
    ///
    /// Environment variables:
    /// - `BULK_CHUNK_SIZE`: Maximum tasks per chunk (default: 50)
    /// - `BULK_CONCURRENCY_LIMIT`: Maximum concurrent chunks (default: 4)
    /// - `USE_BULK_OPTIMIZATION`: Enable optimized bulk processing (default: `true`)
    ///
    /// Invalid values (including 0) are silently ignored and defaults are used.
    /// This prevents configuration mistakes from causing all tasks to be dropped.
    pub fn from_env() -> Self {
        let chunk_size = std::env::var("BULK_CHUNK_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .filter(|&size| size > 0)
            .unwrap_or(Self::DEFAULT_CHUNK_SIZE);

        let concurrency_limit = std::env::var("BULK_CONCURRENCY_LIMIT")
            .ok()
            .and_then(|s| s.parse().ok())
            .filter(|&limit| limit > 0)
            .unwrap_or(Self::DEFAULT_CONCURRENCY_LIMIT);

        // Parse USE_BULK_OPTIMIZATION: "false", "0", or "no" (case-insensitive) disables optimization
        // Apply trim() to handle values with trailing/leading whitespace
        let use_bulk_optimization = std::env::var("USE_BULK_OPTIMIZATION").ok().map_or(
            Self::DEFAULT_USE_BULK_OPTIMIZATION,
            |s| {
                let trimmed = s.trim().to_lowercase();
                !matches!(trimmed.as_str(), "false" | "0" | "no")
            },
        );

        Self {
            chunk_size,
            concurrency_limit,
            use_bulk_optimization,
        }
    }
}

// =============================================================================
// Chunk Utilities (Pure Functions)
// =============================================================================

/// Splits tasks into chunks with their original indices for result mapping.
///
/// This is a pure function that preserves the original order of tasks
/// through index tracking. Each element in the output contains its original
/// index paired with the task data.
///
/// # Arguments
///
/// * `tasks` - Slice of tasks to chunk
/// * `chunk_size` - Maximum number of tasks per chunk (0 is clamped to 1)
///
/// # Returns
///
/// Vector of chunks, where each chunk contains `(original_index, task)` pairs.
///
/// # Examples
///
/// ```ignore
/// let tasks = vec!["a", "b", "c", "d", "e"];
/// let chunks = chunk_tasks_with_indices(&tasks, 2);
///
/// assert_eq!(chunks.len(), 3);
/// assert_eq!(chunks[0], vec![(0, "a"), (1, "b")]);
/// assert_eq!(chunks[1], vec![(2, "c"), (3, "d")]);
/// assert_eq!(chunks[2], vec![(4, "e")]);
/// ```
///
/// # Edge Cases
///
/// - Empty input: Returns empty vector
/// - `chunk_size` of 0: Clamped to 1 to ensure all tasks are processed
/// - `chunk_size` >= `tasks.len()`: Returns single chunk with all tasks
pub fn chunk_tasks_with_indices<T: Clone>(tasks: &[T], chunk_size: usize) -> Vec<Vec<(usize, T)>> {
    if tasks.is_empty() {
        return Vec::new();
    }

    // Clamp chunk_size to 1 if 0 to prevent configuration mistakes from dropping tasks
    let effective_chunk_size = if chunk_size == 0 { 1 } else { chunk_size };

    // Use iterator directly to avoid double clone:
    // Previously: enumerate -> clone -> collect -> chunks -> to_vec (double clone)
    // Now: enumerate -> chunks on indices -> clone only once per element
    let indexed_refs: Vec<(usize, &T)> = tasks.iter().enumerate().collect();

    indexed_refs
        .chunks(effective_chunk_size)
        .map(|chunk| {
            chunk
                .iter()
                .map(|(index, task)| (*index, (*task).clone()))
                .collect()
        })
        .collect()
}

/// Merges chunked results back into original order.
///
/// This is a pure function that reconstructs the result vector
/// in the same order as the original input. Each result is placed
/// at its original index position.
///
/// # Arguments
///
/// * `chunked_results` - Vector of chunks containing `(original_index, result)` pairs
/// * `total_count` - Total number of expected results (original input length)
///
/// # Returns
///
/// Vector of `Option<T>` in original order. Missing indices are `None`,
/// allowing the caller to detect and handle missing results explicitly.
///
/// # Type Parameters
///
/// * `T` - Result type, must implement `Clone`
///
/// # Examples
///
/// ```ignore
/// let chunked: Vec<Vec<(usize, i32)>> = vec![
///     vec![(2, 30), (0, 10)],  // Out of order
///     vec![(1, 20)],
/// ];
/// let merged = merge_chunked_results(chunked, 3);
/// assert_eq!(merged, vec![Some(10), Some(20), Some(30)]);  // Restored to original order
///
/// // Missing indices are None
/// let incomplete: Vec<Vec<(usize, i32)>> = vec![vec![(0, 10), (2, 30)]];
/// let merged = merge_chunked_results(incomplete, 3);
/// assert_eq!(merged, vec![Some(10), None, Some(30)]);  // Index 1 is missing
/// ```
///
/// # Edge Cases
///
/// - Empty input: Returns vector of `total_count` `None` values
/// - Duplicate indices: Later values overwrite earlier ones
/// - Out of bounds indices: Silently ignored (index >= `total_count`)
pub fn merge_chunked_results<T: Clone>(
    chunked_results: Vec<Vec<(usize, T)>>,
    total_count: usize,
) -> Vec<Option<T>> {
    let mut results: Vec<Option<T>> = vec![None; total_count];

    for chunk in chunked_results {
        for (index, result) in chunk {
            if index < total_count {
                results[index] = Some(result);
            }
        }
    }

    results
}

// =============================================================================
// Request/Response DTOs
// =============================================================================

/// Request for bulk task creation.
#[derive(Debug, Deserialize)]
pub struct BulkCreateRequest {
    /// List of tasks to create.
    pub tasks: Vec<CreateTaskRequest>,
}

/// Request for bulk task update.
#[derive(Debug, Deserialize)]
pub struct BulkUpdateRequest {
    /// List of updates to apply.
    pub updates: Vec<BulkUpdateItem>,
}

/// Individual update item in a bulk update request.
#[derive(Debug, Clone, Deserialize)]
pub struct BulkUpdateItem {
    /// Task ID to update.
    pub id: uuid::Uuid,
    /// New priority (optional).
    pub priority: Option<PriorityDto>,
    /// New status (optional).
    pub status: Option<TaskStatusDto>,
    /// Current version for optimistic locking.
    pub version: u64,
}

/// Response for bulk operations.
#[derive(Debug, Serialize)]
pub struct BulkResponse {
    /// Results for each item.
    pub results: Vec<BulkItemResult>,
    /// Summary of the operation.
    pub summary: BulkSummary,
}

/// Result for a single item in bulk operation.
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum BulkItemResult {
    /// Item was successfully created.
    Created { task: TaskResponse },
    /// Item was successfully updated.
    Updated { task: TaskResponse },
    /// Item operation failed.
    Error { error: BulkItemError },
}

/// Error information for a failed bulk item.
#[derive(Debug, Clone, Serialize)]
pub struct BulkItemError {
    /// Error code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Summary of bulk operation results.
#[derive(Debug, Clone, Default, Serialize)]
pub struct BulkSummary {
    /// Total number of items processed.
    pub total: usize,
    /// Number of successful operations.
    pub succeeded: usize,
    /// Number of failed operations.
    pub failed: usize,
    /// Number of operations that used fallback strategies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_used: Option<usize>,
}

// =============================================================================
// Error Types
// =============================================================================

/// Bulk operation level errors.
#[derive(Debug, Clone)]
pub enum BulkError {
    /// Request exceeds the bulk limit.
    LimitExceeded { max: usize, actual: usize },
}

impl From<BulkError> for ApiErrorResponse {
    fn from(error: BulkError) -> Self {
        match error {
            BulkError::LimitExceeded { max, actual } => Self::bad_request(
                "BULK_LIMIT_EXCEEDED",
                format!("Bulk request exceeds limit: {actual} items (max: {max})"),
            ),
        }
    }
}

/// Individual item errors.
#[derive(Debug, Clone)]
pub enum ItemError {
    /// Validation failed.
    Validation { field: String, message: String },
    /// Task not found.
    NotFound { id: TaskId },
    /// Version conflict.
    VersionConflict {
        id: TaskId,
        expected: u64,
        actual: u64,
    },
    /// Duplicate ID in request.
    DuplicateId { id: TaskId },
    /// Repository operation failed with details.
    ///
    /// Contains the original `RepositoryError` for debugging and proper error handling.
    Repository {
        /// The underlying repository error.
        error: crate::infrastructure::RepositoryError,
    },
    /// Both primary and fallback save strategies failed.
    ///
    /// This variant preserves both error contexts for debugging and proper error handling:
    /// - `original_error`: The error from the primary (bulk) save attempt
    /// - `fallback_error`: The error from the individual fallback save attempt
    RepositoryFallbackFailed {
        /// The error from the primary save attempt.
        original_error: crate::infrastructure::RepositoryError,
        /// The error from the fallback save attempt.
        fallback_error: crate::infrastructure::RepositoryError,
    },
}

/// Result of a save operation with fallback tracking.
#[derive(Debug, Clone)]
pub struct SaveResult {
    /// The saved task.
    pub task: Task,
    /// Whether a fallback strategy was used.
    pub used_fallback: bool,
}

impl From<ItemError> for BulkItemError {
    fn from(error: ItemError) -> Self {
        match error {
            ItemError::Validation { field, message } => Self {
                code: "VALIDATION_ERROR".to_string(),
                message: format!("{field}: {message}"),
            },
            ItemError::NotFound { id } => Self {
                code: "NOT_FOUND".to_string(),
                message: format!("Task {id} not found"),
            },
            ItemError::VersionConflict {
                id,
                expected,
                actual,
            } => Self {
                code: "VERSION_CONFLICT".to_string(),
                message: format!("Task {id}: expected version {expected}, got {actual}"),
            },
            ItemError::DuplicateId { id } => Self {
                code: "DUPLICATE_ID".to_string(),
                message: format!("Duplicate task ID in request: {id}"),
            },
            ItemError::Repository { error } => Self {
                code: "REPOSITORY_ERROR".to_string(),
                message: format!("Internal error: {error}"),
            },
            ItemError::RepositoryFallbackFailed {
                original_error,
                fallback_error,
            } => Self {
                code: "REPOSITORY_FALLBACK_FAILED".to_string(),
                message: format!(
                    "Both primary and fallback save failed. Primary: {original_error}, Fallback: {fallback_error}"
                ),
            },
        }
    }
}

// =============================================================================
// Validated Types
// =============================================================================

/// Validated create task data.
#[derive(Debug, Clone)]
struct ValidatedCreate {
    title: String,
    description: Option<String>,
    priority: Priority,
    tags: Vec<Tag>,
}

// =============================================================================
// POST /tasks/bulk - Bulk Create
// =============================================================================

/// Creates multiple tasks in a single request.
///
/// This handler demonstrates:
/// - **`Either`**: Representing validation success/failure per item
/// - **Partial success**: 207 Multi-Status for mixed results
/// - **Pure functions**: Validation and aggregation are side-effect free
///
/// # Request Body
///
/// ```json
/// {
///   "tasks": [
///     { "title": "Task 1", "priority": "high" },
///     { "title": "Task 2", "priority": "low" }
///   ]
/// }
/// ```
///
/// # Response
///
/// - **207 Multi-Status**: Mixed success/failure results
/// - **400 Bad Request**: Request exceeds bulk limit
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] if the request exceeds the bulk limit (100 items).
/// Individual item errors are returned in the response body.
#[allow(clippy::future_not_send)]
pub async fn bulk_create_tasks(
    State(state): State<AppState>,
    Json(request): Json<BulkCreateRequest>,
) -> Result<(StatusCode, JsonResponse<BulkResponse>), ApiErrorResponse> {
    // Step 1: Check bulk limit (pure)
    check_bulk_limit(request.tasks.len())?;

    // Step 2: Validate all requests (pure)
    let validated: Vec<Either<ItemError, ValidatedCreate>> =
        validate_create_requests(&request.tasks);

    // Step 3: Generate IDs and timestamp (I/O boundary)
    let now = Timestamp::now();
    let ids: Vec<TaskId> = (0..request.tasks.len())
        .map(|_| TaskId::generate_v7())
        .collect();

    // Step 4: Build tasks from validated data (pure)
    let tasks_to_save: Vec<Either<ItemError, Task>> = build_tasks(&validated, &ids, &now);

    // Step 5: Save tasks (I/O boundary)
    // Uses optimized or legacy implementation based on feature flag
    // BulkConfig is injected via AppState to ensure referential transparency
    tracing::debug!(
        use_bulk_optimization = state.bulk_config.use_bulk_optimization,
        chunk_size = state.bulk_config.chunk_size,
        concurrency_limit = state.bulk_config.concurrency_limit,
        tasks_count = tasks_to_save.len(),
        "bulk_create_tasks: selecting save strategy"
    );
    let results = if state.bulk_config.use_bulk_optimization {
        tracing::info!(
            tasks_count = tasks_to_save.len(),
            chunk_size = state.bulk_config.chunk_size,
            "Using optimized bulk save with chunking"
        );
        save_tasks_bulk_optimized(
            state.task_repository.clone(),
            tasks_to_save,
            state.bulk_config,
        )
        .await
    } else {
        save_tasks_bulk(state.task_repository.clone(), tasks_to_save).await
    };

    // Step 5.5: Write events to EventStore for successfully created tasks (I/O boundary)
    // Best-effort: event write failures are logged but don't affect the response
    for result in &results {
        if let Either::Right(save_result) = result {
            let event = create_task_created_event(
                &save_result.task,
                EventId::generate_v7(),
                now.clone(),
                1, // Initial event version (expected_version=0 + 1)
            );
            // For new tasks, expected_version is 0 (no events exist yet)
            match state.event_store.append(&event, 0).run_async().await {
                Ok(()) => {}
                Err(e) => {
                    let warning =
                        ConsistencyError::event_write_failed(&save_result.task.task_id, e);
                    log_consistency_error(&warning);
                }
            }
        }
    }

    // Step 6: Update search index for successfully created tasks (batch update)
    let changes: Vec<TaskChange> = results
        .iter()
        .filter_map(|result| match result {
            Either::Right(save_result) => Some(TaskChange::Add(save_result.task.clone())),
            Either::Left(_) => None,
        })
        .collect();

    state.update_search_index_batch(&changes);

    // Step 7: Aggregate results (pure)
    let response = aggregate_create_results(results);

    Ok((StatusCode::MULTI_STATUS, JsonResponse(response)))
}

// =============================================================================
// PUT /tasks/bulk - Bulk Update
// =============================================================================

/// Updates multiple tasks in a single request.
///
/// This handler demonstrates:
/// - **Deduplication**: Using `PersistentHashSet` to detect duplicate IDs
/// - **Optimistic locking**: Version-based conflict detection
/// - **Partial success**: 207 Multi-Status for mixed results
///
/// # Request Body
///
/// ```json
/// {
///   "updates": [
///     { "id": "uuid-1", "priority": "critical", "version": 1 },
///     { "id": "uuid-2", "status": "completed", "version": 2 }
///   ]
/// }
/// ```
///
/// # Response
///
/// - **207 Multi-Status**: Mixed success/failure results
/// - **400 Bad Request**: Request exceeds bulk limit
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] if the request exceeds the bulk limit (100 items).
/// Individual item errors (not found, version conflict, duplicate ID) are returned
/// in the response body.
#[allow(clippy::future_not_send)]
pub async fn bulk_update_tasks(
    State(state): State<AppState>,
    Json(request): Json<BulkUpdateRequest>,
) -> Result<(StatusCode, JsonResponse<BulkResponse>), ApiErrorResponse> {
    // Step 1: Check bulk limit (pure)
    check_bulk_limit(request.updates.len())?;

    // Step 2: Deduplicate IDs (pure)
    let (unique_indices, duplicate_errors) = deduplicate_updates(&request.updates);

    // Step 3: Get timestamp (I/O boundary)
    let now = Timestamp::now();

    // Step 4: Process unique updates (I/O boundary)
    let unique_results = process_unique_updates(
        state.task_repository.clone(),
        &request.updates,
        &unique_indices,
        now,
    )
    .await;

    // Step 5: Merge results with duplicate errors (pure)
    let merged = merge_update_results(&request.updates, unique_results, duplicate_errors);

    // Step 6: Update search index for successfully updated tasks (batch update)
    let changes: Vec<TaskChange> = merged
        .update_pairs
        .iter()
        .map(|(old_task, new_task)| TaskChange::Update {
            old: old_task.clone(),
            new: new_task.clone(),
        })
        .collect();

    state.update_search_index_batch(&changes);

    // Step 7: Aggregate results (pure)
    let response = aggregate_update_results(merged.results);

    Ok((StatusCode::MULTI_STATUS, JsonResponse(response)))
}

// =============================================================================
// Pure Functions - Validation
// =============================================================================

/// Checks if the bulk request size is within limits.
fn check_bulk_limit(count: usize) -> Result<(), ApiErrorResponse> {
    if count > BULK_LIMIT {
        Err(BulkError::LimitExceeded {
            max: BULK_LIMIT,
            actual: count,
        }
        .into())
    } else {
        Ok(())
    }
}

/// Validates all create requests using `for_!` macro (pure function).
///
/// Demonstrates lambars' `for_!` macro for list comprehension style iteration.
fn validate_create_requests(
    requests: &[CreateTaskRequest],
) -> Vec<Either<ItemError, ValidatedCreate>> {
    for_! {
        request <= requests.iter();
        yield validate_single_create(request)
    }
}

/// Validates a single create request (pure function).
///
/// Uses `Alternative` patterns for validation with fallback:
/// - Title validation with `guard` for conditional checks
/// - Tags validation with `alt` for fallback to empty tags
fn validate_single_create(request: &CreateTaskRequest) -> Either<ItemError, ValidatedCreate> {
    // Validate title using Alternative::guard pattern
    let title = match validate_title_with_alternative(&request.title) {
        Either::Right(t) => t,
        Either::Left(e) => return Either::Left(e),
    };

    // Validate description
    let description = match validate_description(request.description.as_deref()) {
        Either::Right(d) => d,
        Either::Left(e) => {
            let first_error = e.errors.first().map_or_else(
                || ("description".to_string(), "validation error".to_string()),
                |f| (f.field.clone(), f.message.clone()),
            );
            return Either::Left(ItemError::Validation {
                field: first_error.0,
                message: first_error.1,
            });
        }
    };

    // Validate tags using Alternative::alt for fallback to empty tags
    let tags = validate_tags_with_alternative(&request.tags);

    Either::Right(ValidatedCreate {
        title,
        description,
        priority: Priority::from(request.priority),
        tags,
    })
}

/// Validates title using `Alternative::guard` (pure function).
///
/// Demonstrates `Alternative::guard` for conditional validation.
/// Returns `Either::Left` if validation fails, `Either::Right` with the title otherwise.
fn validate_title_with_alternative(title: &str) -> Either<ItemError, String> {
    // Use guard to check if title is non-empty
    let non_empty_check: Option<()> = <Option<()>>::guard(!title.trim().is_empty());

    match non_empty_check {
        Some(()) => {
            // Further validation using original validate_title
            match validate_title(title) {
                Either::Right(t) => Either::Right(t),
                Either::Left(e) => {
                    let first_error = e.errors.first().map_or_else(
                        || ("title".to_string(), "validation error".to_string()),
                        |f| (f.field.clone(), f.message.clone()),
                    );
                    Either::Left(ItemError::Validation {
                        field: first_error.0,
                        message: first_error.1,
                    })
                }
            }
        }
        None => Either::Left(ItemError::Validation {
            field: "title".to_string(),
            message: "title cannot be empty".to_string(),
        }),
    }
}

/// Validates tags using `Alternative::alt` for fallback (pure function).
///
/// Demonstrates `Alternative::alt` for fallback pattern:
/// - Primary: validate provided tags
/// - Fallback: return empty tags if validation fails (tolerant mode)
fn validate_tags_with_alternative(tags: &[String]) -> Vec<Tag> {
    // Primary validation attempt
    let primary_result: Option<Vec<Tag>> = match validate_tags(tags) {
        Either::Right(valid_tags) => Some(valid_tags),
        Either::Left(_) => None,
    };

    // Fallback to empty tags using Alternative::alt
    let fallback: Option<Vec<Tag>> = Some(Vec::new());

    // Use alt: if primary succeeds, use it; otherwise use fallback
    primary_result.alt(fallback).unwrap_or_default()
}

/// Validates multiple field values and combines results using `Alternative::choice` (pure function).
///
/// Demonstrates `Alternative::choice` to select the first successful validation
/// from multiple validation strategies.
///
/// **Note**: This implementation evaluates all validators eagerly. For short-circuit
/// evaluation, use `validate_with_choice_lazy` instead.
#[cfg(test)]
fn validate_with_choice<T: Clone + 'static>(validators: Vec<impl Fn() -> Option<T>>) -> Option<T> {
    let results: Vec<Option<T>> = validators.into_iter().map(|v| v()).collect();
    Option::choice(results)
}

/// Validates with short-circuit evaluation using iterator's `find_map` (pure function).
///
/// This is the preferred implementation that stops evaluation at the first successful
/// validation result, demonstrating proper lazy semantics for `Alternative::choice`.
///
/// # Examples
///
/// ```ignore
/// let result = validate_with_choice_lazy(vec![
///     Box::new(|| None),       // Evaluated, returns None
///     Box::new(|| Some(42)),   // Evaluated, returns Some(42) - stops here
///     Box::new(|| Some(100)),  // NOT evaluated due to short-circuit
/// ]);
/// assert_eq!(result, Some(42));
/// ```
#[cfg(test)]
fn validate_with_choice_lazy<T>(validators: Vec<Box<dyn Fn() -> Option<T>>>) -> Option<T> {
    validators.into_iter().find_map(|validator| validator())
}

// =============================================================================
// Pure Functions - Task Building
// =============================================================================

/// Builds tasks from validated data (pure function).
fn build_tasks(
    validated: &[Either<ItemError, ValidatedCreate>],
    ids: &[TaskId],
    timestamp: &Timestamp,
) -> Vec<Either<ItemError, Task>> {
    validated
        .iter()
        .zip(ids.iter())
        .map(
            |(result, id): (&Either<ItemError, ValidatedCreate>, &TaskId)| match result {
                Either::Left(error) => Either::Left(error.clone()),
                Either::Right(v) => {
                    Either::Right(build_single_task(v, id.clone(), timestamp.clone()))
                }
            },
        )
        .collect()
}

/// Builds a single task from validated data (pure function).
fn build_single_task(validated: &ValidatedCreate, id: TaskId, now: Timestamp) -> Task {
    let base = Task::new(id, validated.title.clone(), now);

    let with_description = match &validated.description {
        Some(desc) => base.with_description(desc.clone()),
        None => base,
    };

    let with_priority = with_description.with_priority(validated.priority);

    validated
        .tags
        .iter()
        .fold(with_priority, |task, tag| task.add_tag(tag.clone()))
}

// =============================================================================
// Pure Functions - Deduplication
// =============================================================================

/// Detects duplicate IDs in update requests (pure function).
///
/// Returns:
/// - Indices of unique (first occurrence) items
/// - Errors for duplicate items
fn deduplicate_updates(updates: &[BulkUpdateItem]) -> (Vec<usize>, Vec<(usize, ItemError)>) {
    let mut seen: PersistentHashSet<uuid::Uuid> = PersistentHashSet::new();
    let mut unique_indices = Vec::new();
    let mut duplicate_errors = Vec::new();

    for (index, update) in updates.iter().enumerate() {
        if seen.contains(&update.id) {
            duplicate_errors.push((
                index,
                ItemError::DuplicateId {
                    id: TaskId::from_uuid(update.id),
                },
            ));
        } else {
            seen = seen.insert(update.id);
            unique_indices.push(index);
        }
    }

    (unique_indices, duplicate_errors)
}

// =============================================================================
// Pure Functions - Update Application
// =============================================================================

/// Applies an update to a task (pure function).
///
/// Returns `None` if no changes were requested (both priority and status are None).
/// Returns `Some(updated_task)` with incremented version and updated timestamp.
fn apply_update(task: Task, update: &BulkUpdateItem, now: Timestamp) -> Option<Task> {
    // Check if there are any changes to apply
    if update.priority.is_none() && update.status.is_none() {
        return None;
    }

    let mut result = task;

    if let Some(priority) = update.priority {
        result = result.with_priority(Priority::from(priority));
    }

    if let Some(status) = update.status {
        result = result.with_status(TaskStatus::from(status));
    }

    // Update timestamp and increment version using pure methods
    Some(result.with_updated_at(now).increment_version())
}

// =============================================================================
// Pure Functions - Result Aggregation
// =============================================================================

/// Aggregates create results into a response (pure function).
///
/// Tracks fallback usage for partial failure measurement.
fn aggregate_create_results(results: Vec<Either<ItemError, SaveResult>>) -> BulkResponse {
    let total = results.len();
    let mut succeeded = 0;
    let mut failed = 0;
    let mut fallback_used = 0;

    let item_results: Vec<BulkItemResult> = results
        .into_iter()
        .map(|result| match result {
            Either::Right(save_result) => {
                succeeded += 1;
                if save_result.used_fallback {
                    fallback_used += 1;
                }
                BulkItemResult::Created {
                    task: TaskResponse::from(&save_result.task),
                }
            }
            Either::Left(error) => {
                failed += 1;
                BulkItemResult::Error {
                    error: error.into(),
                }
            }
        })
        .collect();

    BulkResponse {
        results: item_results,
        summary: BulkSummary {
            total,
            succeeded,
            failed,
            fallback_used: if fallback_used > 0 {
                Some(fallback_used)
            } else {
                None
            },
        },
    }
}

/// Aggregates update results into a response (pure function).
fn aggregate_update_results(results: Vec<Either<ItemError, Task>>) -> BulkResponse {
    let total = results.len();
    let mut succeeded = 0;
    let mut failed = 0;

    let item_results: Vec<BulkItemResult> = results
        .into_iter()
        .map(|result| match result {
            Either::Right(task) => {
                succeeded += 1;
                BulkItemResult::Updated {
                    task: TaskResponse::from(&task),
                }
            }
            Either::Left(error) => {
                failed += 1;
                BulkItemResult::Error {
                    error: error.into(),
                }
            }
        })
        .collect();

    BulkResponse {
        results: item_results,
        summary: BulkSummary {
            total,
            succeeded,
            failed,
            fallback_used: None,
        },
    }
}

/// Merges unique results with duplicate errors (pure function).
/// Result of merging update results, including old/new task pairs for index updates.
#[derive(Debug)]
struct MergedUpdateResults {
    /// The merged results for response generation.
    results: Vec<Either<ItemError, Task>>,
    /// Old/new task pairs for successful updates (for search index updates).
    update_pairs: Vec<(Task, Task)>,
}

fn merge_update_results(
    original_updates: &[BulkUpdateItem],
    unique_results: Vec<(usize, BulkUpdateResult)>,
    duplicate_errors: Vec<(usize, ItemError)>,
) -> MergedUpdateResults {
    let mut results: Vec<Option<Either<ItemError, Task>>> = vec![None; original_updates.len()];
    let mut update_pairs: Vec<(Task, Task)> = Vec::new();

    // Place unique results and collect update pairs
    for (index, bulk_result) in unique_results {
        // If update was successful and we have the old task, track the pair
        if let (Either::Right(new_task), Some(old_task)) =
            (&bulk_result.result, &bulk_result.old_task)
        {
            update_pairs.push((old_task.clone(), new_task.clone()));
        }
        results[index] = Some(bulk_result.result);
    }

    // Place duplicate errors
    for (index, error) in duplicate_errors {
        results[index] = Some(Either::Left(error));
    }

    // All should be filled
    MergedUpdateResults {
        results: results.into_iter().flatten().collect(),
        update_pairs,
    }
}

// =============================================================================
// I/O Functions
// =============================================================================

/// Saves tasks in bulk with Alternative-based fallback (I/O boundary).
///
/// Demonstrates `Alternative::alt` for fallback save strategies:
/// - Primary: Save to main repository
/// - Fallback: Attempt save with retry or alternative strategy
///
/// Uses `Alternative::choice` to select the first successful save strategy.
async fn save_tasks_bulk(
    repository: Arc<dyn TaskRepository + Send + Sync>,
    tasks: Vec<Either<ItemError, Task>>,
) -> Vec<Either<ItemError, SaveResult>> {
    let mut results = Vec::with_capacity(tasks.len());

    for task_result in tasks {
        let result = match task_result {
            Either::Left(error) => Either::Left(error),
            Either::Right(task) => save_with_alternative_fallback(repository.clone(), task).await,
        };
        results.push(result);
    }

    results
}

/// Saves a single task using Alternative-based fallback strategies (I/O boundary).
///
/// Demonstrates `Alternative::alt` for fallback patterns:
/// 1. Primary strategy: Direct save
/// 2. Fallback strategy: Save with retry (simulated as alternative path)
///
/// Returns `SaveResult` tracking whether fallback was used.
///
/// **Important**: Uses lazy evaluation - fallback is only executed when primary fails.
/// This prevents unnecessary double-save operations.
///
/// If both strategies fail, returns `RepositoryFallbackFailed` containing both error contexts.
async fn save_with_alternative_fallback(
    repository: Arc<dyn TaskRepository + Send + Sync>,
    task: Task,
) -> Either<ItemError, SaveResult> {
    // Primary save attempt with error tracking
    let primary_result = try_primary_save_with_error(&repository, &task).await;

    match primary_result {
        Ok(result) => Either::Right(result), // Primary succeeded - no fallback needed
        Err(primary_error) => {
            // Primary failed - try fallback with error tracking
            let fallback_result = try_fallback_save_with_error(&repository, &task).await;

            match fallback_result {
                Ok(result) => Either::Right(result), // Fallback succeeded
                Err(fallback_error) => {
                    // Both strategies failed - preserve both errors
                    Either::Left(ItemError::RepositoryFallbackFailed {
                        original_error: primary_error,
                        fallback_error,
                    })
                }
            }
        }
    }
}

/// Combines two save strategies with lazy evaluation (pure function).
///
/// The fallback function is only called when the primary function returns `None`.
/// This ensures proper short-circuit semantics for `Alternative::alt`.
///
/// # Type Parameters
///
/// - `Primary`: Function returning primary save result
/// - `Fallback`: Function returning fallback save result (only called if primary fails)
///
/// # Examples
///
/// ```ignore
/// let result = save_with_lazy_fallback(
///     || Some(SaveResult { task, used_fallback: false }),  // Primary succeeds
///     || Some(SaveResult { task, used_fallback: true }),   // NOT called
/// );
/// assert!(!result.unwrap().used_fallback);
/// ```
///
/// # Note
///
/// This function is used in tests to verify short-circuit behavior.
/// The async counterpart `save_with_alternative_fallback` uses the same logic
/// but with async/await syntax for actual save operations.
#[cfg(test)]
fn save_with_lazy_fallback<Primary, Fallback>(
    primary: Primary,
    fallback: Fallback,
) -> Option<SaveResult>
where
    Primary: FnOnce() -> Option<SaveResult>,
    Fallback: FnOnce() -> Option<SaveResult>,
{
    // Short-circuit: if primary succeeds, don't call fallback
    primary().map_or_else(fallback, Some)
}

/// Attempts primary save strategy with error tracking (I/O boundary).
///
/// Returns `Ok(SaveResult)` if successful, `Err(RepositoryError)` if failed.
/// This version preserves the error information for proper error handling.
async fn try_primary_save_with_error(
    repository: &Arc<dyn TaskRepository + Send + Sync>,
    task: &Task,
) -> Result<SaveResult, crate::infrastructure::RepositoryError> {
    match repository.save(task).run_async().await {
        Ok(()) => Ok(SaveResult {
            task: task.clone(),
            used_fallback: false,
        }),
        Err(e) => {
            tracing::warn!(error = %e, "Primary save failed, attempting fallback");
            Err(e)
        }
    }
}

/// Attempts fallback save strategy with error tracking (I/O boundary).
///
/// This demonstrates an alternative save path that could:
/// - Use a different repository
/// - Apply data transformation before save
/// - Use a queue for deferred processing
///
/// For this demo, it retries the same repository (simulating a retry strategy).
/// Returns `Ok(SaveResult)` if successful, `Err(RepositoryError)` if failed.
async fn try_fallback_save_with_error(
    repository: &Arc<dyn TaskRepository + Send + Sync>,
    task: &Task,
) -> Result<SaveResult, crate::infrastructure::RepositoryError> {
    // Simulate a retry or alternative strategy
    // In production, this could be a different repository, cache, or queue
    match repository.save(task).run_async().await {
        Ok(()) => {
            tracing::info!("Fallback save succeeded");
            Ok(SaveResult {
                task: task.clone(),
                used_fallback: true,
            })
        }
        Err(e) => {
            tracing::error!(error = %e, "Fallback save also failed");
            Err(e)
        }
    }
}
/// Combines multiple save strategies using `Alternative::choice` (pure helper).
///
/// This demonstrates selecting the first successful result from multiple strategies.
#[cfg(test)]
fn combine_save_strategies(strategies: Vec<Option<SaveResult>>) -> Option<SaveResult> {
    Option::choice(strategies)
}

// =============================================================================
// Chunk-based Parallel Processing (Pure Functions + I/O Boundary)
// =============================================================================

/// Type alias for indexed save results.
///
/// Each element is a tuple of (`original_index`, `save_result`).
pub type IndexedSaveResult = (usize, Either<ItemError, SaveResult>);

/// Saves a chunk of tasks using bulk repository operation.
///
/// This function creates an `AsyncIO` that saves all tasks in the chunk
/// using the repository's `save_bulk` method. Results are paired with
/// their original indices for later merging.
///
/// # Arguments
///
/// * `repository` - Task repository for saving
/// * `chunk` - Vector of `(original_index, task)` pairs
///
/// # Returns
///
/// `AsyncIO` that produces a vector of `(original_index, result)` pairs.
pub fn save_chunk(
    repository: Arc<dyn TaskRepository + Send + Sync>,
    chunk: Vec<(usize, Task)>,
) -> AsyncIO<Vec<IndexedSaveResult>> {
    // Ownership is moved into the closure, avoiding extra clones outside
    AsyncIO::new(move || {
        let repository = repository.clone();
        // Clone chunk only once inside the async closure
        let indexed_tasks = chunk;

        async move {
            // Extract indices and tasks for save_bulk call
            // We need to separate them because save_bulk takes &[Task]
            let (indices, tasks): (Vec<usize>, Vec<Task>) = indexed_tasks.into_iter().unzip();

            // Call repository's save_bulk (takes &[Task], so we keep ownership of tasks)
            let save_results = repository.save_bulk(&tasks).run_async().await;

            // Pair results with indices, consuming tasks to avoid clone
            indices
                .into_iter()
                .zip(tasks)
                .zip(save_results)
                .map(|((index, task), result)| {
                    let either_result = match result {
                        Ok(()) => Either::Right(SaveResult {
                            task,
                            used_fallback: false,
                        }),
                        Err(e) => {
                            tracing::warn!(error = %e, "Chunk save failed for task");
                            Either::Left(ItemError::Repository { error: e })
                        }
                    };
                    (index, either_result)
                })
                .collect()
        }
    })
}

/// Saves a chunk of tasks using bulk repository operation with individual fallback.
///
/// This function extends `save_chunk` with a fallback strategy:
/// 1. First, attempt bulk save (`save_bulk`) for all tasks
/// 2. For tasks that failed in bulk save, attempt individual save (`save`) as fallback
/// 3. Track which tasks used the fallback strategy via `SaveResult::used_fallback`
///
/// # Fallback Strategy
///
/// - **All succeeded**: Return results with `used_fallback: false`
/// - **Partial failure**: Failed tasks are retried individually with `used_fallback: true`
/// - **All failed in bulk**: All tasks are retried individually
/// - **Fallback also failed**: Return `ItemError::RepositoryFallbackFailed` with both errors
///
/// This ensures that both `RepositoryError` details are preserved, allowing callers
/// to understand what went wrong in both the primary and fallback attempts.
///
/// # Arguments
///
/// * `repository` - Task repository for saving
/// * `chunk` - Vector of `(original_index, task)` pairs
///
/// # Returns
///
/// `AsyncIO` that produces a vector of `(original_index, result)` pairs.
/// Each result contains either:
/// - `Either::Right(SaveResult)` with `used_fallback` flag indicating fallback usage
/// - `Either::Left(ItemError::RepositoryFallbackFailed)` with both error details
pub fn save_chunk_with_fallback(
    repository: Arc<dyn TaskRepository + Send + Sync>,
    chunk: Vec<(usize, Task)>,
) -> AsyncIO<Vec<IndexedSaveResult>> {
    AsyncIO::new(move || {
        let repository = repository.clone();
        let indexed_tasks = chunk;

        async move {
            // Extract indices and tasks for save_bulk call
            let indices: Vec<usize> = indexed_tasks.iter().map(|(index, _)| *index).collect();
            let tasks: Vec<Task> = indexed_tasks.into_iter().map(|(_, task)| task).collect();

            // Step 1: Attempt bulk save
            let bulk_results = repository.save_bulk(&tasks).run_async().await;

            // Step 2: Separate successes from failures
            let mut final_results: Vec<(usize, Either<ItemError, SaveResult>)> =
                Vec::with_capacity(tasks.len());
            let mut failed_tasks: Vec<(usize, Task, crate::infrastructure::RepositoryError)> =
                Vec::new();

            for ((index, task), result) in indices.iter().zip(tasks).zip(bulk_results) {
                match result {
                    Ok(()) => {
                        // Bulk save succeeded
                        final_results.push((
                            *index,
                            Either::Right(SaveResult {
                                task,
                                used_fallback: false,
                            }),
                        ));
                    }
                    Err(error) => {
                        // Bulk save failed - queue for individual fallback
                        tracing::warn!(
                            error = %error,
                            task_id = %task.task_id,
                            "Bulk save failed, will attempt individual fallback"
                        );
                        failed_tasks.push((*index, task, error));
                    }
                }
            }

            // Step 3: Attempt individual save for failed tasks (fallback)
            for (index, task, original_error) in failed_tasks {
                match repository.save(&task).run_async().await {
                    Ok(()) => {
                        // Fallback succeeded
                        tracing::info!(
                            task_id = %task.task_id,
                            "Individual fallback save succeeded"
                        );
                        final_results.push((
                            index,
                            Either::Right(SaveResult {
                                task,
                                used_fallback: true,
                            }),
                        ));
                    }
                    Err(fallback_error) => {
                        // Both bulk and individual save failed
                        // Preserve both errors for debugging and proper error handling
                        tracing::error!(
                            task_id = %task.task_id,
                            original_error = %original_error,
                            fallback_error = %fallback_error,
                            "Both bulk and individual save failed"
                        );
                        final_results.push((
                            index,
                            Either::Left(ItemError::RepositoryFallbackFailed {
                                original_error,
                                fallback_error,
                            }),
                        ));
                    }
                }
            }

            final_results
        }
    })
}

/// Saves tasks in bulk with chunked parallel processing and fallback strategy.
///
/// This function demonstrates functional programming patterns:
/// - **Pure function composition**: Chunking and merging are pure
/// - **Effect isolation**: I/O operations are contained in `AsyncIO`
/// - **Parallel processing**: Uses `batch_run_buffered` for bounded concurrency
/// - **Fallback strategy**: Individual save fallback for failed bulk operations
///
/// # Processing Flow
///
/// 1. Separate error tasks from valid tasks (pure)
/// 2. Split valid tasks into chunks with indices (pure, uses `chunk_tasks_with_indices`)
/// 3. Create `AsyncIO` for each chunk with fallback (pure, uses `save_chunk_with_fallback`)
/// 4. Execute chunks in parallel with bounded concurrency (I/O boundary)
/// 5. Merge results back to original order (pure, uses `merge_chunked_results`)
///
/// # Fallback Strategy
///
/// Each chunk uses `save_chunk_with_fallback` which:
/// - First attempts bulk save (`save_bulk`) for all tasks in the chunk
/// - For any failed tasks, attempts individual save (`save`) as fallback
/// - Tracks which tasks used fallback via `SaveResult::used_fallback`
/// - Preserves `RepositoryError` details on final failure (never loses error context)
///
/// # Arguments
///
/// * `repository` - Task repository for saving
/// * `tasks` - Vector of tasks (`Either` error or valid task)
/// * `config` - Bulk operation configuration
///
/// # Returns
///
/// Vector of results in the same order as input.
///
/// # Example
///
/// ```ignore
/// let config = BulkConfig::default();
/// let results = save_tasks_bulk_optimized(repository, tasks, config).await;
/// ```
pub async fn save_tasks_bulk_optimized(
    repository: Arc<dyn TaskRepository + Send + Sync>,
    tasks: Vec<Either<ItemError, Task>>,
    config: BulkConfig,
) -> Vec<Either<ItemError, SaveResult>> {
    if tasks.is_empty() {
        return Vec::new();
    }

    // Clamp concurrency_limit to 1 if 0 to prevent configuration mistakes
    // from causing all tasks to be dropped. This mirrors the behavior of
    // chunk_tasks_with_indices for chunk_size.
    let effective_concurrency_limit = if config.concurrency_limit == 0 {
        1
    } else {
        config.concurrency_limit
    };

    let total_count = tasks.len();

    // Step 1: Separate error tasks from valid tasks (pure)
    // Collect indices and either errors or valid tasks
    let mut error_indices: Vec<(usize, ItemError)> = Vec::new();
    let mut valid_tasks: Vec<(usize, Task)> = Vec::new();

    for (index, task_result) in tasks.into_iter().enumerate() {
        match task_result {
            Either::Left(error) => error_indices.push((index, error)),
            Either::Right(task) => valid_tasks.push((index, task)),
        }
    }

    // If no valid tasks, just return errors in order
    if valid_tasks.is_empty() {
        let mut results: Vec<Option<Either<ItemError, SaveResult>>> = vec![None; total_count];
        for (index, error) in error_indices {
            results[index] = Some(Either::Left(error));
        }
        return results.into_iter().flatten().collect();
    }

    // Step 2: Split valid tasks into chunks with indices (pure)
    let chunks = chunk_tasks_with_indices(&valid_tasks, config.chunk_size);

    // Step 3: Create AsyncIO for each chunk with fallback strategy (pure - just builds the computation)
    // Uses save_chunk_with_fallback to ensure individual fallback for failed bulk saves
    let chunk_ios: Vec<AsyncIO<Vec<IndexedSaveResult>>> = chunks
        .into_iter()
        .map(|chunk| {
            // Extract inner (usize, Task) from ((usize, (usize, Task)))
            let inner_chunk: Vec<(usize, Task)> = chunk
                .into_iter()
                .map(|(_, (idx, task))| (idx, task))
                .collect();
            save_chunk_with_fallback(repository.clone(), inner_chunk)
        })
        .collect();

    // Step 4: Execute chunks in parallel with bounded concurrency (I/O boundary)
    // Uses effective_concurrency_limit which is clamped to at least 1
    let chunked_results = AsyncIO::batch_run_buffered(chunk_ios, effective_concurrency_limit)
        .await
        .unwrap_or_else(|batch_error| {
            // If batch execution fails, log the actual error and return empty results
            // This should not happen with valid config (concurrency_limit is clamped to >= 1)
            // The batch_error provides context about what went wrong (e.g., concurrency limit was 0)
            tracing::error!(
                error = %batch_error,
                "batch_run_buffered failed - this indicates a configuration or runtime error"
            );
            Vec::new()
        });

    // Step 5: Merge results back to original order (pure)
    let merged = merge_chunked_results(chunked_results, total_count);

    // Step 6: Fill in error results at their original positions
    let mut final_results: Vec<Option<Either<ItemError, SaveResult>>> = merged;

    for (index, error) in error_indices {
        final_results[index] = Some(Either::Left(error));
    }

    // Convert Option<T> to T, using Repository error for any missing entries
    // Missing entries indicate a failure in batch processing (should not happen in normal operation)
    final_results
        .into_iter()
        .map(|opt| {
            opt.unwrap_or_else(|| {
                Either::Left(ItemError::Repository {
                    error: crate::infrastructure::RepositoryError::DatabaseError(
                        "Task result missing from batch operation".to_string(),
                    ),
                })
            })
        })
        .collect()
}

/// Result of a bulk update operation with old/new task tracking for index updates.
#[derive(Debug, Clone)]
pub struct BulkUpdateResult {
    /// The result of the update operation.
    pub result: Either<ItemError, Task>,
    /// The old task before update (for search index), if update was successful.
    pub old_task: Option<Task>,
}

/// Processes unique updates (I/O boundary).
///
/// Returns both the result and the old task for successful updates,
/// enabling efficient search index updates.
async fn process_unique_updates(
    repository: Arc<dyn TaskRepository + Send + Sync>,
    updates: &[BulkUpdateItem],
    unique_indices: &[usize],
    now: Timestamp,
) -> Vec<(usize, BulkUpdateResult)> {
    let mut results = Vec::with_capacity(unique_indices.len());

    for &index in unique_indices {
        let update = &updates[index];
        let task_id = TaskId::from_uuid(update.id);

        let bulk_result = match repository.find_by_id(&task_id).run_async().await {
            Ok(Some(task)) => {
                // Check version (positive condition first)
                if task.version == update.version {
                    // Apply update (returns None if no changes)
                    match apply_update(task.clone(), update, now.clone()) {
                        Some(updated_task) => {
                            match repository.save(&updated_task).run_async().await {
                                Ok(()) => BulkUpdateResult {
                                    result: Either::Right(updated_task),
                                    old_task: Some(task), // Track old task for index update
                                },
                                Err(e) => {
                                    tracing::error!(error = %e, task_id = %task_id, "Repository save failed");
                                    BulkUpdateResult {
                                        result: Either::Left(ItemError::Repository { error: e }),
                                        old_task: None,
                                    }
                                }
                            }
                        }
                        None => BulkUpdateResult {
                            result: Either::Right(task), // No changes, return original
                            old_task: None,              // No index update needed
                        },
                    }
                } else {
                    BulkUpdateResult {
                        result: Either::Left(ItemError::VersionConflict {
                            id: task_id,
                            expected: update.version,
                            actual: task.version,
                        }),
                        old_task: None,
                    }
                }
            }
            Ok(None) => BulkUpdateResult {
                result: Either::Left(ItemError::NotFound { id: task_id }),
                old_task: None,
            },
            Err(e) => {
                tracing::error!(error = %e, task_id = %task_id, "Repository find failed");
                BulkUpdateResult {
                    result: Either::Left(ItemError::Repository { error: e }),
                    old_task: None,
                }
            }
        };

        results.push((index, bulk_result));
    }

    results
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    use crate::api::dto::PriorityDto;

    // -------------------------------------------------------------------------
    // Bulk Limit Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_check_bulk_limit_within_limit() {
        assert!(check_bulk_limit(50).is_ok());
        assert!(check_bulk_limit(100).is_ok());
    }

    #[rstest]
    fn test_check_bulk_limit_exceeds() {
        let result = check_bulk_limit(101);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_check_bulk_limit_zero() {
        assert!(check_bulk_limit(0).is_ok());
    }

    // -------------------------------------------------------------------------
    // Validation Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_validate_single_create_valid() {
        let request = CreateTaskRequest {
            title: "Valid Task".to_string(),
            description: Some("Description".to_string()),
            priority: PriorityDto::High,
            tags: vec!["backend".to_string()],
        };

        let result = validate_single_create(&request);
        assert!(result.is_right());
    }

    #[rstest]
    fn test_validate_single_create_empty_title() {
        let request = CreateTaskRequest {
            title: String::new(),
            description: None,
            priority: PriorityDto::Low,
            tags: vec![],
        };

        let result = validate_single_create(&request);
        assert!(result.is_left());
    }

    #[rstest]
    fn test_validate_create_requests_mixed() {
        let requests = vec![
            CreateTaskRequest {
                title: "Valid".to_string(),
                description: None,
                priority: PriorityDto::Medium,
                tags: vec![],
            },
            CreateTaskRequest {
                title: String::new(), // Invalid
                description: None,
                priority: PriorityDto::Low,
                tags: vec![],
            },
        ];

        let results = validate_create_requests(&requests);
        assert_eq!(results.len(), 2);
        assert!(results[0].is_right());
        assert!(results[1].is_left());
    }

    // -------------------------------------------------------------------------
    // Deduplication Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_deduplicate_updates_no_duplicates() {
        let id1 = uuid::Uuid::new_v4();
        let id2 = uuid::Uuid::new_v4();

        let updates = vec![
            BulkUpdateItem {
                id: id1,
                priority: Some(PriorityDto::High),
                status: None,
                version: 1,
            },
            BulkUpdateItem {
                id: id2,
                priority: None,
                status: Some(TaskStatusDto::Completed),
                version: 1,
            },
        ];

        let (unique, duplicates) = deduplicate_updates(&updates);

        assert_eq!(unique.len(), 2);
        assert!(duplicates.is_empty());
    }

    #[rstest]
    fn test_deduplicate_updates_with_duplicates() {
        let id1 = uuid::Uuid::new_v4();

        let updates = vec![
            BulkUpdateItem {
                id: id1,
                priority: Some(PriorityDto::High),
                status: None,
                version: 1,
            },
            BulkUpdateItem {
                id: id1, // Duplicate
                priority: Some(PriorityDto::Low),
                status: None,
                version: 2,
            },
        ];

        let (unique, duplicates) = deduplicate_updates(&updates);

        assert_eq!(unique.len(), 1);
        assert_eq!(unique[0], 0); // First occurrence
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].0, 1); // Second occurrence is duplicate
    }

    // -------------------------------------------------------------------------
    // Task Building Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_build_single_task() {
        let validated = ValidatedCreate {
            title: "Test Task".to_string(),
            description: Some("Description".to_string()),
            priority: Priority::High,
            tags: vec![Tag::new("backend")],
        };

        let id = TaskId::generate();
        let now = Timestamp::now();

        let task = build_single_task(&validated, id.clone(), now);

        assert_eq!(task.task_id, id);
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.description, Some("Description".to_string()));
        assert_eq!(task.priority, Priority::High);
        assert_eq!(task.tags.len(), 1);
    }

    // -------------------------------------------------------------------------
    // Update Application Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_apply_update_priority() {
        let task = Task::new(TaskId::generate(), "Task", Timestamp::now());
        assert_eq!(task.version, 1); // Initial version

        let update = BulkUpdateItem {
            id: uuid::Uuid::new_v4(),
            priority: Some(PriorityDto::Critical),
            status: None,
            version: 1, // Match current version
        };
        let now = Timestamp::now();

        let updated = apply_update(task, &update, now);

        assert!(updated.is_some());
        let updated = updated.unwrap();
        assert_eq!(updated.priority, Priority::Critical);
        assert_eq!(updated.version, 2); // Incremented
    }

    #[rstest]
    fn test_apply_update_status() {
        let task = Task::new(TaskId::generate(), "Task", Timestamp::now());
        assert_eq!(task.version, 1); // Initial version

        let update = BulkUpdateItem {
            id: uuid::Uuid::new_v4(),
            priority: None,
            status: Some(TaskStatusDto::InProgress),
            version: 1, // Match current version
        };
        let now = Timestamp::now();

        let updated = apply_update(task, &update, now);

        assert!(updated.is_some());
        let updated = updated.unwrap();
        assert_eq!(updated.status, TaskStatus::InProgress);
        assert_eq!(updated.version, 2); // Incremented
    }

    #[rstest]
    fn test_apply_update_no_changes_returns_none() {
        let task = Task::new(TaskId::generate(), "Task", Timestamp::now());

        let update = BulkUpdateItem {
            id: uuid::Uuid::new_v4(),
            priority: None,
            status: None,
            version: 1,
        };
        let now = Timestamp::now();

        let result = apply_update(task, &update, now);

        assert!(result.is_none());
    }

    // -------------------------------------------------------------------------
    // Aggregation Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_aggregate_create_results_all_success() {
        let task1 = Task::new(TaskId::generate(), "Task 1", Timestamp::now());
        let task2 = Task::new(TaskId::generate(), "Task 2", Timestamp::now());

        let results = vec![
            Either::Right(SaveResult {
                task: task1,
                used_fallback: false,
            }),
            Either::Right(SaveResult {
                task: task2,
                used_fallback: false,
            }),
        ];

        let response = aggregate_create_results(results);

        assert_eq!(response.summary.total, 2);
        assert_eq!(response.summary.succeeded, 2);
        assert_eq!(response.summary.failed, 0);
        assert!(response.summary.fallback_used.is_none());
    }

    #[rstest]
    fn test_aggregate_create_results_all_failure() {
        let error1 = ItemError::Validation {
            field: "title".to_string(),
            message: "empty".to_string(),
        };
        let error2 = ItemError::Validation {
            field: "title".to_string(),
            message: "too long".to_string(),
        };

        let results: Vec<Either<ItemError, SaveResult>> =
            vec![Either::Left(error1), Either::Left(error2)];

        let response = aggregate_create_results(results);

        assert_eq!(response.summary.total, 2);
        assert_eq!(response.summary.succeeded, 0);
        assert_eq!(response.summary.failed, 2);
        assert!(response.summary.fallback_used.is_none());
    }

    #[rstest]
    fn test_aggregate_create_results_mixed() {
        let task = Task::new(TaskId::generate(), "Task 1", Timestamp::now());
        let error = ItemError::Validation {
            field: "title".to_string(),
            message: "empty".to_string(),
        };

        let results: Vec<Either<ItemError, SaveResult>> = vec![
            Either::Right(SaveResult {
                task,
                used_fallback: false,
            }),
            Either::Left(error),
        ];

        let response = aggregate_create_results(results);

        assert_eq!(response.summary.total, 2);
        assert_eq!(response.summary.succeeded, 1);
        assert_eq!(response.summary.failed, 1);
        assert!(response.summary.fallback_used.is_none());
    }

    #[rstest]
    fn test_aggregate_create_results_with_fallback() {
        let task1 = Task::new(TaskId::generate(), "Task 1", Timestamp::now());
        let task2 = Task::new(TaskId::generate(), "Task 2", Timestamp::now());

        let results = vec![
            Either::Right(SaveResult {
                task: task1,
                used_fallback: false,
            }),
            Either::Right(SaveResult {
                task: task2,
                used_fallback: true,
            }),
        ];

        let response = aggregate_create_results(results);

        assert_eq!(response.summary.total, 2);
        assert_eq!(response.summary.succeeded, 2);
        assert_eq!(response.summary.failed, 0);
        assert_eq!(response.summary.fallback_used, Some(1));
    }

    #[rstest]
    fn test_aggregate_update_results_all_success() {
        let task1 = Task::new(TaskId::generate(), "Task 1", Timestamp::now());
        let task2 = Task::new(TaskId::generate(), "Task 2", Timestamp::now());

        let results = vec![Either::Right(task1), Either::Right(task2)];

        let response = aggregate_update_results(results);

        assert_eq!(response.summary.total, 2);
        assert_eq!(response.summary.succeeded, 2);
        assert_eq!(response.summary.failed, 0);
        // Check that results are marked as "updated"
        assert!(matches!(
            &response.results[0],
            BulkItemResult::Updated { .. }
        ));
    }

    #[rstest]
    fn test_aggregate_update_results_all_failure() {
        let error1 = ItemError::NotFound {
            id: TaskId::generate(),
        };
        let error2 = ItemError::VersionConflict {
            id: TaskId::generate(),
            expected: 1,
            actual: 2,
        };

        let results: Vec<Either<ItemError, Task>> =
            vec![Either::Left(error1), Either::Left(error2)];

        let response = aggregate_update_results(results);

        assert_eq!(response.summary.total, 2);
        assert_eq!(response.summary.succeeded, 0);
        assert_eq!(response.summary.failed, 2);
    }

    #[rstest]
    fn test_aggregate_update_results_mixed() {
        let task = Task::new(TaskId::generate(), "Task 1", Timestamp::now());
        let error = ItemError::NotFound {
            id: TaskId::generate(),
        };

        let results: Vec<Either<ItemError, Task>> = vec![Either::Right(task), Either::Left(error)];

        let response = aggregate_update_results(results);

        assert_eq!(response.summary.total, 2);
        assert_eq!(response.summary.succeeded, 1);
        assert_eq!(response.summary.failed, 1);
    }

    // -------------------------------------------------------------------------
    // Merge Results Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_merge_update_results() {
        let id1 = uuid::Uuid::new_v4();
        let id2 = uuid::Uuid::new_v4();

        let updates = vec![
            BulkUpdateItem {
                id: id1,
                priority: None,
                status: None,
                version: 1,
            },
            BulkUpdateItem {
                id: id1, // Duplicate
                priority: None,
                status: None,
                version: 1,
            },
            BulkUpdateItem {
                id: id2,
                priority: None,
                status: None,
                version: 1,
            },
        ];

        let task = Task::new(TaskId::generate(), "Task", Timestamp::now());
        let unique_results = vec![
            (
                0,
                BulkUpdateResult {
                    result: Either::Right(task.clone()),
                    old_task: None, // No actual update (no version change)
                },
            ),
            (
                2,
                BulkUpdateResult {
                    result: Either::Right(task),
                    old_task: None,
                },
            ),
        ];

        let duplicate_errors = vec![(
            1,
            ItemError::DuplicateId {
                id: TaskId::from_uuid(id1),
            },
        )];

        let merged = merge_update_results(&updates, unique_results, duplicate_errors);

        assert_eq!(merged.results.len(), 3);
        assert!(merged.results[0].is_right()); // First id1 - success
        assert!(merged.results[1].is_left()); // Second id1 - duplicate error
        assert!(merged.results[2].is_right()); // id2 - success
        assert!(merged.update_pairs.is_empty()); // No actual updates with old_task
    }

    // -------------------------------------------------------------------------
    // Alternative Pattern Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_validate_title_with_alternative_valid() {
        let result = validate_title_with_alternative("Valid Title");
        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), "Valid Title");
    }

    #[rstest]
    fn test_validate_title_with_alternative_empty() {
        let result = validate_title_with_alternative("");
        assert!(result.is_left());
        match result.unwrap_left() {
            ItemError::Validation { field, message } => {
                assert_eq!(field, "title");
                assert!(message.contains("empty"));
            }
            _ => panic!("Expected Validation error"),
        }
    }

    #[rstest]
    fn test_validate_title_with_alternative_whitespace_only() {
        let result = validate_title_with_alternative("   ");
        assert!(result.is_left());
    }

    #[rstest]
    fn test_validate_tags_with_alternative_valid() {
        let tags = vec!["backend".to_string(), "api".to_string()];
        let result = validate_tags_with_alternative(&tags);
        assert_eq!(result.len(), 2);
    }

    #[rstest]
    fn test_validate_tags_with_alternative_fallback_to_empty() {
        // Invalid tags (too long) should fallback to empty
        let tags = vec!["a".repeat(101)]; // Assuming tag length limit is 100
        let result = validate_tags_with_alternative(&tags);
        // Fallback to empty tags if validation fails
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_validate_tags_with_alternative_empty_input() {
        let tags: Vec<String> = vec![];
        let result = validate_tags_with_alternative(&tags);
        assert!(result.is_empty());
    }

    #[rstest]
    fn test_validate_with_choice_first_succeeds() {
        let validators: Vec<Box<dyn Fn() -> Option<i32>>> = vec![
            Box::new(|| Some(1)),
            Box::new(|| Some(2)),
            Box::new(|| None),
        ];
        let result = validate_with_choice(validators.into_iter().map(|v| move || v()).collect());
        assert_eq!(result, Some(1));
    }

    #[rstest]
    fn test_validate_with_choice_all_fail() {
        let validators: Vec<Box<dyn Fn() -> Option<i32>>> =
            vec![Box::new(|| None), Box::new(|| None)];
        let result = validate_with_choice(validators.into_iter().map(|v| move || v()).collect());
        assert_eq!(result, None);
    }

    #[rstest]
    fn test_validate_with_choice_later_succeeds() {
        let validators: Vec<Box<dyn Fn() -> Option<i32>>> =
            vec![Box::new(|| None), Box::new(|| None), Box::new(|| Some(42))];
        let result = validate_with_choice(validators.into_iter().map(|v| move || v()).collect());
        assert_eq!(result, Some(42));
    }

    // -------------------------------------------------------------------------
    // SaveResult and Fallback Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_save_result_no_fallback() {
        let task = Task::new(TaskId::generate(), "Task", Timestamp::now());
        let result = SaveResult {
            task: task.clone(),
            used_fallback: false,
        };

        assert_eq!(result.task.task_id, task.task_id);
        assert!(!result.used_fallback);
    }

    #[rstest]
    fn test_save_result_with_fallback() {
        let task = Task::new(TaskId::generate(), "Task", Timestamp::now());
        let result = SaveResult {
            task: task.clone(),
            used_fallback: true,
        };

        assert_eq!(result.task.task_id, task.task_id);
        assert!(result.used_fallback);
    }

    #[rstest]
    fn test_combine_save_strategies_first_succeeds() {
        let task1 = Task::new(TaskId::generate(), "Task", Timestamp::now());
        let task2 = Task::new(TaskId::generate(), "Task", Timestamp::now());
        let strategies = vec![
            Some(SaveResult {
                task: task1,
                used_fallback: false,
            }),
            Some(SaveResult {
                task: task2,
                used_fallback: true,
            }),
        ];

        let result = combine_save_strategies(strategies);
        assert!(result.is_some());
        assert!(!result.unwrap().used_fallback);
    }

    #[rstest]
    fn test_combine_save_strategies_fallback_used() {
        let task = Task::new(TaskId::generate(), "Task", Timestamp::now());
        let strategies = vec![
            None,
            Some(SaveResult {
                task,
                used_fallback: true,
            }),
        ];

        let result = combine_save_strategies(strategies);
        assert!(result.is_some());
        assert!(result.unwrap().used_fallback);
    }

    #[rstest]
    fn test_combine_save_strategies_all_fail() {
        let strategies: Vec<Option<SaveResult>> = vec![None, None, None];

        let result = combine_save_strategies(strategies);
        assert!(result.is_none());
    }

    #[rstest]
    fn test_item_error_repository_fallback_failed() {
        let original_error =
            crate::infrastructure::RepositoryError::DatabaseError("primary failed".to_string());
        let fallback_error =
            crate::infrastructure::RepositoryError::DatabaseError("fallback failed".to_string());
        let error = ItemError::RepositoryFallbackFailed {
            original_error,
            fallback_error,
        };
        let bulk_error: BulkItemError = error.into();

        assert_eq!(bulk_error.code, "REPOSITORY_FALLBACK_FAILED");
        assert!(bulk_error.message.contains("primary failed"));
        assert!(bulk_error.message.contains("fallback failed"));
    }

    // -------------------------------------------------------------------------
    // Alternative::guard Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_alternative_guard_true() {
        let result: Option<()> = <Option<()>>::guard(true);
        assert_eq!(result, Some(()));
    }

    #[rstest]
    fn test_alternative_guard_false() {
        let result: Option<()> = <Option<()>>::guard(false);
        assert_eq!(result, None);
    }

    #[rstest]
    fn test_alternative_guard_with_validation() {
        fn validate_non_empty(s: &str) -> Option<&str> {
            <Option<()>>::guard(!s.is_empty()).map(|()| s)
        }

        assert_eq!(validate_non_empty("hello"), Some("hello"));
        assert_eq!(validate_non_empty(""), None);
    }

    // -------------------------------------------------------------------------
    // Alternative::alt Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_alternative_alt_first_some() {
        let first: Option<i32> = Some(1);
        let second: Option<i32> = Some(2);
        assert_eq!(first.alt(second), Some(1));
    }

    #[rstest]
    fn test_alternative_alt_first_none() {
        let first: Option<i32> = None;
        let second: Option<i32> = Some(2);
        assert_eq!(first.alt(second), Some(2));
    }

    #[rstest]
    fn test_alternative_alt_both_none() {
        let first: Option<i32> = None;
        let second: Option<i32> = None;
        assert_eq!(first.alt(second), None);
    }

    // -------------------------------------------------------------------------
    // Alternative::choice Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_alternative_choice_finds_first_some() {
        let alternatives = vec![None, Some(1), Some(2)];
        let result: Option<i32> = Option::choice(alternatives);
        assert_eq!(result, Some(1));
    }

    #[rstest]
    fn test_alternative_choice_all_none() {
        let alternatives: Vec<Option<i32>> = vec![None, None, None];
        let result: Option<i32> = Option::choice(alternatives);
        assert_eq!(result, None);
    }

    #[rstest]
    fn test_alternative_choice_empty() {
        let alternatives: Vec<Option<i32>> = vec![];
        let result: Option<i32> = Option::choice(alternatives);
        assert_eq!(result, None);
    }

    // -------------------------------------------------------------------------
    // Bulk Summary Fallback Tracking Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_bulk_summary_no_fallback() {
        let summary = BulkSummary {
            total: 10,
            succeeded: 8,
            failed: 2,
            fallback_used: None,
        };

        assert_eq!(summary.total, 10);
        assert_eq!(summary.succeeded, 8);
        assert_eq!(summary.failed, 2);
        assert!(summary.fallback_used.is_none());
    }

    #[rstest]
    fn test_bulk_summary_with_fallback() {
        let summary = BulkSummary {
            total: 10,
            succeeded: 8,
            failed: 2,
            fallback_used: Some(3),
        };

        assert_eq!(summary.total, 10);
        assert_eq!(summary.succeeded, 8);
        assert_eq!(summary.failed, 2);
        assert_eq!(summary.fallback_used, Some(3));
    }

    // -------------------------------------------------------------------------
    // Lazy Fallback Tests (Short-circuit evaluation)
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_save_with_lazy_fallback_primary_success_no_fallback_call() {
        // Test: when primary succeeds, fallback should NOT be called
        use std::sync::atomic::{AtomicUsize, Ordering};

        let fallback_call_count = AtomicUsize::new(0);

        let primary = || {
            Some(SaveResult {
                task: Task::new(TaskId::generate(), "Task", Timestamp::now()),
                used_fallback: false,
            })
        };

        let fallback = || {
            fallback_call_count.fetch_add(1, Ordering::SeqCst);
            Some(SaveResult {
                task: Task::new(TaskId::generate(), "Task", Timestamp::now()),
                used_fallback: true,
            })
        };

        let result = save_with_lazy_fallback(primary, fallback);

        assert!(result.is_some());
        assert!(!result.as_ref().unwrap().used_fallback);
        // Fallback should NOT have been called
        assert_eq!(fallback_call_count.load(Ordering::SeqCst), 0);
    }

    #[rstest]
    fn test_save_with_lazy_fallback_primary_fails_fallback_called() {
        // Test: when primary fails, fallback should be called
        use std::sync::atomic::{AtomicUsize, Ordering};

        let fallback_call_count = AtomicUsize::new(0);

        let primary = || None;

        let fallback = || {
            fallback_call_count.fetch_add(1, Ordering::SeqCst);
            Some(SaveResult {
                task: Task::new(TaskId::generate(), "Task", Timestamp::now()),
                used_fallback: true,
            })
        };

        let result = save_with_lazy_fallback(primary, fallback);

        assert!(result.is_some());
        assert!(result.as_ref().unwrap().used_fallback);
        // Fallback should have been called exactly once
        assert_eq!(fallback_call_count.load(Ordering::SeqCst), 1);
    }

    #[rstest]
    fn test_save_with_lazy_fallback_both_fail() {
        // Test: when both primary and fallback fail, returns None
        let primary = || None::<SaveResult>;
        let fallback = || None::<SaveResult>;

        let result = save_with_lazy_fallback(primary, fallback);

        assert!(result.is_none());
    }

    // -------------------------------------------------------------------------
    // Short-circuit Choice Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_validate_with_choice_lazy_short_circuits() {
        // Test: choice should stop evaluating after first success
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_count = Arc::new(AtomicUsize::new(0));

        let count1 = Arc::clone(&call_count);
        let count2 = Arc::clone(&call_count);
        let count3 = Arc::clone(&call_count);

        let result = validate_with_choice_lazy(vec![
            Box::new(move || {
                count1.fetch_add(1, Ordering::SeqCst);
                None
            }) as Box<dyn Fn() -> Option<i32>>,
            Box::new(move || {
                count2.fetch_add(1, Ordering::SeqCst);
                Some(42) // First success
            }),
            Box::new(move || {
                count3.fetch_add(1, Ordering::SeqCst);
                Some(100) // Should NOT be called
            }),
        ]);

        assert_eq!(result, Some(42));
        // Only first two validators should be called (first fails, second succeeds)
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[rstest]
    fn test_validate_with_choice_lazy_all_fail() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_count = Arc::new(AtomicUsize::new(0));

        let count1 = Arc::clone(&call_count);
        let count2 = Arc::clone(&call_count);
        let count3 = Arc::clone(&call_count);

        let result = validate_with_choice_lazy(vec![
            Box::new(move || {
                count1.fetch_add(1, Ordering::SeqCst);
                None
            }) as Box<dyn Fn() -> Option<i32>>,
            Box::new(move || {
                count2.fetch_add(1, Ordering::SeqCst);
                None
            }),
            Box::new(move || {
                count3.fetch_add(1, Ordering::SeqCst);
                None
            }),
        ]);

        assert_eq!(result, None);
        // All validators should be called since all fail
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[rstest]
    fn test_validate_with_choice_lazy_first_succeeds() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_count = Arc::new(AtomicUsize::new(0));

        let count1 = Arc::clone(&call_count);
        let count2 = Arc::clone(&call_count);

        let result = validate_with_choice_lazy(vec![
            Box::new(move || {
                count1.fetch_add(1, Ordering::SeqCst);
                Some(1) // First succeeds immediately
            }) as Box<dyn Fn() -> Option<i32>>,
            Box::new(move || {
                count2.fetch_add(1, Ordering::SeqCst);
                Some(2)
            }),
        ]);

        assert_eq!(result, Some(1));
        // Only first validator should be called
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[rstest]
    fn test_validate_with_choice_lazy_empty() {
        let result = validate_with_choice_lazy::<i32>(vec![]);
        assert_eq!(result, None);
    }

    // -------------------------------------------------------------------------
    // BulkConfig Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_bulk_config_default() {
        let config = BulkConfig::default();

        assert_eq!(config.chunk_size, 50);
        assert_eq!(config.concurrency_limit, 4);
        assert!(config.use_bulk_optimization);
    }

    #[rstest]
    fn test_bulk_config_custom() {
        let config = BulkConfig {
            chunk_size: 100,
            concurrency_limit: 8,
            use_bulk_optimization: false,
        };

        assert_eq!(config.chunk_size, 100);
        assert_eq!(config.concurrency_limit, 8);
        assert!(!config.use_bulk_optimization);
    }

    #[rstest]
    fn test_bulk_config_clone() {
        let config = BulkConfig::default();
        let cloned = config;

        assert_eq!(config, cloned);
    }

    #[rstest]
    fn test_bulk_config_debug() {
        let config = BulkConfig::default();
        let debug_str = format!("{config:?}");

        assert!(debug_str.contains("BulkConfig"));
        assert!(debug_str.contains("chunk_size"));
        assert!(debug_str.contains("concurrency_limit"));
    }

    // -------------------------------------------------------------------------
    // chunk_tasks_with_indices Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_chunk_tasks_with_indices_empty() {
        let tasks: Vec<String> = vec![];
        let chunks = chunk_tasks_with_indices(&tasks, 10);

        assert!(chunks.is_empty());
    }

    #[rstest]
    fn test_chunk_tasks_with_indices_single_chunk() {
        let tasks = vec!["a", "b", "c"];
        let chunks = chunk_tasks_with_indices(&tasks, 10);

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], vec![(0, "a"), (1, "b"), (2, "c")]);
    }

    #[rstest]
    fn test_chunk_tasks_with_indices_exact_chunk_size() {
        let tasks = vec!["a", "b", "c", "d"];
        let chunks = chunk_tasks_with_indices(&tasks, 2);

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0], vec![(0, "a"), (1, "b")]);
        assert_eq!(chunks[1], vec![(2, "c"), (3, "d")]);
    }

    #[rstest]
    fn test_chunk_tasks_with_indices_multiple_chunks() {
        let tasks = vec!["a", "b", "c", "d", "e"];
        let chunks = chunk_tasks_with_indices(&tasks, 2);

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], vec![(0, "a"), (1, "b")]);
        assert_eq!(chunks[1], vec![(2, "c"), (3, "d")]);
        assert_eq!(chunks[2], vec![(4, "e")]);
    }

    #[rstest]
    fn test_chunk_tasks_with_indices_chunk_size_one() {
        let tasks = vec!["a", "b", "c"];
        let chunks = chunk_tasks_with_indices(&tasks, 1);

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], vec![(0, "a")]);
        assert_eq!(chunks[1], vec![(1, "b")]);
        assert_eq!(chunks[2], vec![(2, "c")]);
    }

    #[rstest]
    fn test_chunk_tasks_with_indices_chunk_size_zero() {
        let tasks = vec!["a", "b", "c"];
        let chunks = chunk_tasks_with_indices(&tasks, 0);

        // Zero chunk size should be clamped to 1, processing all tasks one per chunk
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], vec![(0, "a")]);
        assert_eq!(chunks[1], vec![(1, "b")]);
        assert_eq!(chunks[2], vec![(2, "c")]);
    }

    #[rstest]
    fn test_chunk_tasks_with_indices_preserves_order() {
        let tasks: Vec<i32> = (0..100).collect();
        let chunks = chunk_tasks_with_indices(&tasks, 25);

        assert_eq!(chunks.len(), 4);

        // Verify all indices are preserved in order
        let mut all_indices: Vec<usize> = Vec::new();
        for chunk in chunks {
            for (index, value) in chunk {
                #[allow(clippy::cast_sign_loss)]
                {
                    assert_eq!(index, value as usize); // Index matches value
                }
                all_indices.push(index);
            }
        }

        // Should have all indices from 0 to 99
        assert_eq!(all_indices, (0..100).collect::<Vec<_>>());
    }

    // -------------------------------------------------------------------------
    // merge_chunked_results Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_merge_chunked_results_empty() {
        let chunked: Vec<Vec<(usize, i32)>> = vec![];
        let merged = merge_chunked_results(chunked, 0);

        assert!(merged.is_empty());
    }

    #[rstest]
    fn test_merge_chunked_results_single_chunk() {
        let chunked = vec![vec![(0, 10), (1, 20), (2, 30)]];
        let merged = merge_chunked_results(chunked, 3);

        assert_eq!(merged, vec![Some(10), Some(20), Some(30)]);
    }

    #[rstest]
    fn test_merge_chunked_results_multiple_chunks() {
        let chunked = vec![
            vec![(0, 10), (1, 20)],
            vec![(2, 30), (3, 40)],
            vec![(4, 50)],
        ];
        let merged = merge_chunked_results(chunked, 5);

        assert_eq!(
            merged,
            vec![Some(10), Some(20), Some(30), Some(40), Some(50)]
        );
    }

    #[rstest]
    fn test_merge_chunked_results_out_of_order() {
        // Results arrive in different order than original
        let chunked = vec![vec![(2, 30), (0, 10)], vec![(1, 20)]];
        let merged = merge_chunked_results(chunked, 3);

        // Should still be in original order
        assert_eq!(merged, vec![Some(10), Some(20), Some(30)]);
    }

    #[rstest]
    fn test_merge_chunked_results_with_gaps() {
        // Some indices missing (represented as None for explicit detection)
        let chunked = vec![vec![(0, 10), (2, 30)]];
        let merged = merge_chunked_results(chunked, 3);

        // Index 1 is None, allowing caller to detect missing result
        assert_eq!(merged, vec![Some(10), None, Some(30)]);
    }

    #[rstest]
    fn test_merge_chunked_results_with_strings() {
        let chunked = vec![
            vec![(0, "first".to_string())],
            vec![(2, "third".to_string())],
            vec![(1, "second".to_string())],
        ];
        let merged = merge_chunked_results(chunked, 3);

        assert_eq!(
            merged,
            vec![
                Some("first".to_string()),
                Some("second".to_string()),
                Some("third".to_string())
            ]
        );
    }

    #[rstest]
    fn test_merge_chunked_results_index_out_of_bounds_ignored() {
        // Index beyond total_count should be ignored
        let chunked = vec![
            vec![(0, 10), (1, 20), (100, 999)], // 100 is out of bounds
        ];
        let merged = merge_chunked_results(chunked, 3);

        // Index 100 ignored, index 2 is None (missing)
        assert_eq!(merged, vec![Some(10), Some(20), None]);
    }

    #[rstest]
    fn test_chunk_and_merge_roundtrip() {
        // Test that chunking and merging returns original order
        let original: Vec<i32> = (0..100).collect();
        let chunks = chunk_tasks_with_indices(&original, 15);

        // Simulate processing: just keep the same values
        let processed: Vec<Vec<(usize, i32)>> = chunks;

        let merged = merge_chunked_results(processed, 100);

        // All values should be Some and in original order
        let unwrapped: Vec<i32> = merged.into_iter().flatten().collect();
        assert_eq!(unwrapped, original);
    }

    #[rstest]
    fn test_merge_chunked_results_duplicate_indices_last_wins() {
        // Duplicate indices: later values overwrite earlier ones
        let chunked = vec![
            vec![(0, 10), (1, 20)],
            vec![(1, 99)], // Duplicate index 1, should overwrite
        ];
        let merged = merge_chunked_results(chunked, 2);

        // Index 1 should have the later value (99)
        assert_eq!(merged, vec![Some(10), Some(99)]);
    }

    #[rstest]
    fn test_merge_chunked_results_all_missing() {
        // No results provided, all should be None
        let chunked: Vec<Vec<(usize, i32)>> = vec![];
        let merged = merge_chunked_results(chunked, 3);

        assert_eq!(merged, vec![None, None, None]);
    }

    // -------------------------------------------------------------------------
    // chunk_tasks_with_indices with zero chunk_size Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_chunk_tasks_with_indices_zero_chunk_size_clamps_to_one() {
        // chunk_size of 0 should be clamped to 1, not return empty
        let tasks = vec!["a", "b", "c"];
        let chunks = chunk_tasks_with_indices(&tasks, 0);

        // Should process all tasks, one per chunk
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], vec![(0, "a")]);
        assert_eq!(chunks[1], vec![(1, "b")]);
        assert_eq!(chunks[2], vec![(2, "c")]);
    }

    // -------------------------------------------------------------------------
    // BulkConfig::from_env Tests for zero values
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_bulk_config_from_env_zero_chunk_size_uses_default() {
        // This test verifies the filter logic conceptually
        // In practice, from_env reads from environment variables
        // Here we test the filter behavior directly
        let chunk_size: Option<usize> = Some(0);
        let filtered = chunk_size.filter(|&size| size > 0);
        assert!(filtered.is_none());

        let chunk_size: Option<usize> = Some(50);
        let filtered = chunk_size.filter(|&size| size > 0);
        assert_eq!(filtered, Some(50));
    }

    #[rstest]
    fn test_bulk_config_use_bulk_optimization_parsing_logic() {
        // Test the parsing logic for USE_BULK_OPTIMIZATION
        // "false", "0", "no" (case-insensitive) should return false
        // Also test that values with whitespace are handled correctly after trim()
        let test_cases = vec![
            ("false", false),
            ("FALSE", false),
            ("False", false),
            ("0", false),
            ("no", false),
            ("NO", false),
            ("No", false),
            ("true", true),
            ("TRUE", true),
            ("1", true),
            ("yes", true),
            ("anything_else", true),
            // Whitespace handling test cases (should be handled by trim())
            ("false ", false),
            (" false", false),
            (" false ", false),
            ("  0  ", false),
            ("\tno\t", false),
            (" true ", true),
        ];

        for (input, expected) in test_cases {
            let trimmed = input.trim().to_lowercase();
            let result = !matches!(trimmed.as_str(), "false" | "0" | "no");
            assert_eq!(
                result, expected,
                "Input '{input}' should result in {expected}"
            );
        }
    }

    // -------------------------------------------------------------------------
    // save_tasks_bulk_optimized Tests
    // -------------------------------------------------------------------------

    mod save_tasks_bulk_optimized_tests {
        use super::*;
        use crate::domain::{Task, TaskId, Timestamp};
        use crate::infrastructure::InMemoryTaskRepository;

        fn create_test_task(title: &str) -> Task {
            Task::new(TaskId::generate(), title, Timestamp::now())
        }

        #[rstest]
        #[tokio::test]
        async fn test_save_tasks_bulk_optimized_empty() {
            let repository = Arc::new(InMemoryTaskRepository::new());
            let config = BulkConfig::default();

            let results = save_tasks_bulk_optimized(repository, Vec::new(), config).await;

            assert!(results.is_empty());
        }

        #[rstest]
        #[tokio::test]
        async fn test_save_tasks_bulk_optimized_single_chunk() {
            let repository = Arc::new(InMemoryTaskRepository::new());
            let config = BulkConfig {
                chunk_size: 50,
                concurrency_limit: 4,
                use_bulk_optimization: true,
            };

            // Create 10 tasks (fits in single chunk)
            let tasks: Vec<Either<ItemError, Task>> = (0..10)
                .map(|i| Either::Right(create_test_task(&format!("Task {i}"))))
                .collect();

            let results = save_tasks_bulk_optimized(repository.clone(), tasks, config).await;

            assert_eq!(results.len(), 10);

            // All should be successful
            for (i, result) in results.iter().enumerate() {
                assert!(result.is_right(), "Task {i} should be saved successfully");
            }
        }

        #[rstest]
        #[tokio::test]
        async fn test_save_tasks_bulk_optimized_multiple_chunks() {
            let repository = Arc::new(InMemoryTaskRepository::new());
            let config = BulkConfig {
                chunk_size: 5, // Small chunk size to create multiple chunks
                concurrency_limit: 2,
                use_bulk_optimization: true,
            };

            // Create 20 tasks (will be split into 4 chunks of 5)
            let tasks: Vec<Either<ItemError, Task>> = (0..20)
                .map(|i| Either::Right(create_test_task(&format!("Task {i}"))))
                .collect();

            let results = save_tasks_bulk_optimized(repository.clone(), tasks, config).await;

            assert_eq!(results.len(), 20);

            // All should be successful
            for (i, result) in results.iter().enumerate() {
                assert!(result.is_right(), "Task {i} should be saved successfully");
            }

            // Verify tasks are actually in the repository
            let count = repository.count().run_async().await.unwrap();
            assert_eq!(count, 20);
        }

        #[rstest]
        #[tokio::test]
        async fn test_save_tasks_bulk_optimized_with_errors() {
            let repository = Arc::new(InMemoryTaskRepository::new());
            let config = BulkConfig {
                chunk_size: 5,
                concurrency_limit: 2,
                use_bulk_optimization: true,
            };

            // Create mixed tasks: some valid, some errors
            let tasks: Vec<Either<ItemError, Task>> = vec![
                Either::Right(create_test_task("Task 0")),
                Either::Left(ItemError::Validation {
                    field: "title".to_string(),
                    message: "empty".to_string(),
                }),
                Either::Right(create_test_task("Task 2")),
                Either::Left(ItemError::Validation {
                    field: "title".to_string(),
                    message: "too long".to_string(),
                }),
                Either::Right(create_test_task("Task 4")),
            ];

            let results = save_tasks_bulk_optimized(repository.clone(), tasks, config).await;

            assert_eq!(results.len(), 5);

            // Check expected pattern: success, error, success, error, success
            assert!(results[0].is_right(), "Task 0 should succeed");
            assert!(results[1].is_left(), "Task 1 should be error");
            assert!(results[2].is_right(), "Task 2 should succeed");
            assert!(results[3].is_left(), "Task 3 should be error");
            assert!(results[4].is_right(), "Task 4 should succeed");

            // Verify error types are preserved
            if let Either::Left(ItemError::Validation { field, message }) = &results[1] {
                assert_eq!(field, "title");
                assert_eq!(message, "empty");
            } else {
                panic!("Expected Validation error for task 1");
            }

            // Verify only valid tasks are in repository
            let count = repository.count().run_async().await.unwrap();
            assert_eq!(count, 3);
        }

        #[rstest]
        #[tokio::test]
        async fn test_save_tasks_bulk_optimized_all_errors() {
            let repository = Arc::new(InMemoryTaskRepository::new());
            let config = BulkConfig::default();

            // All errors, no valid tasks
            let tasks: Vec<Either<ItemError, Task>> = vec![
                Either::Left(ItemError::Validation {
                    field: "title".to_string(),
                    message: "error 1".to_string(),
                }),
                Either::Left(ItemError::Validation {
                    field: "title".to_string(),
                    message: "error 2".to_string(),
                }),
            ];

            let results = save_tasks_bulk_optimized(repository.clone(), tasks, config).await;

            assert_eq!(results.len(), 2);
            assert!(results[0].is_left());
            assert!(results[1].is_left());

            // Nothing should be in repository
            let count = repository.count().run_async().await.unwrap();
            assert_eq!(count, 0);
        }

        #[rstest]
        #[tokio::test]
        async fn test_save_tasks_bulk_optimized_preserves_order() {
            let repository = Arc::new(InMemoryTaskRepository::new());
            let config = BulkConfig {
                chunk_size: 3,
                concurrency_limit: 10, // High concurrency to encourage reordering
                use_bulk_optimization: true,
            };

            // Create 15 tasks to test order preservation
            let tasks: Vec<Either<ItemError, Task>> = (0..15)
                .map(|i| Either::Right(create_test_task(&format!("Task {i:02}"))))
                .collect();

            let original_titles: Vec<String> = tasks
                .iter()
                .filter_map(|t| match t {
                    Either::Right(task) => Some(task.title.clone()),
                    Either::Left(_) => None,
                })
                .collect();

            let results = save_tasks_bulk_optimized(repository.clone(), tasks, config).await;

            // Verify order is preserved by comparing titles
            let result_titles: Vec<String> = results
                .iter()
                .filter_map(|r| match r {
                    Either::Right(save_result) => Some(save_result.task.title.clone()),
                    Either::Left(_) => None,
                })
                .collect();

            assert_eq!(original_titles, result_titles, "Order should be preserved");
        }

        #[rstest]
        #[tokio::test]
        async fn test_save_tasks_bulk_optimized_concurrency_limit_zero_clamps_to_one() {
            // Test that concurrency_limit of 0 is clamped to 1 and all tasks are processed
            let repository = Arc::new(InMemoryTaskRepository::new());
            let config = BulkConfig {
                chunk_size: 5,
                concurrency_limit: 0, // Invalid value, should be clamped to 1
                use_bulk_optimization: true,
            };

            // Create 10 tasks
            let tasks: Vec<Either<ItemError, Task>> = (0..10)
                .map(|i| Either::Right(create_test_task(&format!("Task {i}"))))
                .collect();

            let results = save_tasks_bulk_optimized(repository.clone(), tasks, config).await;

            // All 10 tasks should be processed (not dropped)
            assert_eq!(results.len(), 10);

            // All should be successful
            for (i, result) in results.iter().enumerate() {
                assert!(
                    result.is_right(),
                    "Task {i} should be saved successfully despite concurrency_limit=0"
                );
            }

            // Verify tasks are actually in the repository
            let count = repository.count().run_async().await.unwrap();
            assert_eq!(count, 10);
        }

        #[rstest]
        #[tokio::test]
        async fn test_save_tasks_bulk_optimized_both_zero_values() {
            // Test that both chunk_size=0 and concurrency_limit=0 are handled correctly
            let repository = Arc::new(InMemoryTaskRepository::new());
            let config = BulkConfig {
                chunk_size: 0,        // Invalid, clamped to 1 in chunk_tasks_with_indices
                concurrency_limit: 0, // Invalid, clamped to 1 in save_tasks_bulk_optimized
                use_bulk_optimization: true,
            };

            // Create 5 tasks
            let tasks: Vec<Either<ItemError, Task>> = (0..5)
                .map(|i| Either::Right(create_test_task(&format!("Task {i}"))))
                .collect();

            let results = save_tasks_bulk_optimized(repository.clone(), tasks, config).await;

            // All 5 tasks should be processed
            assert_eq!(results.len(), 5);

            // All should be successful
            for (i, result) in results.iter().enumerate() {
                assert!(
                    result.is_right(),
                    "Task {i} should be saved successfully despite zero config values"
                );
            }

            // Verify tasks are in repository
            let count = repository.count().run_async().await.unwrap();
            assert_eq!(count, 5);
        }
    }

    // -------------------------------------------------------------------------
    // Feature Flag Switching Tests
    // -------------------------------------------------------------------------

    mod feature_flag_tests {
        use super::*;
        use crate::api::dto::PriorityDto;
        use crate::api::handlers::AppState;
        use crate::infrastructure::{
            InMemoryEventStore, InMemoryProjectRepository, InMemoryTaskRepository,
        };
        use arc_swap::ArcSwap;
        use axum::Json;
        use axum::extract::State;
        use lambars::persistent::PersistentVector;
        use std::sync::atomic::AtomicU64;

        use crate::api::handlers::{AppliedConfig, create_stub_external_sources};
        use crate::api::query::{SearchCache, SearchIndex};
        use crate::infrastructure::RngProvider;
        use std::sync::atomic::AtomicUsize;

        fn create_app_state_with_bulk_config(bulk_config: BulkConfig) -> AppState {
            let external_sources = create_stub_external_sources();

            AppState {
                task_repository: Arc::new(InMemoryTaskRepository::new()),
                project_repository: Arc::new(InMemoryProjectRepository::new()),
                event_store: Arc::new(InMemoryEventStore::new()),
                config: crate::api::handlers::AppConfig::default(),
                bulk_config,
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

        #[rstest]
        #[tokio::test]
        async fn test_bulk_create_tasks_uses_optimized_when_flag_enabled() {
            // Test that bulk_create_tasks uses save_tasks_bulk_optimized when flag is true
            let config = BulkConfig {
                chunk_size: 10,
                concurrency_limit: 2,
                use_bulk_optimization: true,
            };
            let state = create_app_state_with_bulk_config(config);

            let request = BulkCreateRequest {
                tasks: vec![
                    CreateTaskRequest {
                        title: "Task 1".to_string(),
                        description: None,
                        priority: PriorityDto::Medium,
                        tags: vec![],
                    },
                    CreateTaskRequest {
                        title: "Task 2".to_string(),
                        description: None,
                        priority: PriorityDto::Medium,
                        tags: vec![],
                    },
                ],
            };

            let result = bulk_create_tasks(State(state.clone()), Json(request)).await;

            assert!(result.is_ok());
            let (status, JsonResponse(response)) = result.unwrap();
            assert_eq!(status, StatusCode::MULTI_STATUS);
            assert_eq!(response.summary.total, 2);
            assert_eq!(response.summary.succeeded, 2);
            assert_eq!(response.summary.failed, 0);

            // Verify tasks were saved
            let count = state.task_repository.count().run_async().await.unwrap();
            assert_eq!(count, 2);
        }

        #[rstest]
        #[tokio::test]
        async fn test_bulk_create_tasks_uses_legacy_when_flag_disabled() {
            // Test that bulk_create_tasks uses save_tasks_bulk when flag is false
            let config = BulkConfig {
                chunk_size: 10,
                concurrency_limit: 2,
                use_bulk_optimization: false,
            };
            let state = create_app_state_with_bulk_config(config);

            let request = BulkCreateRequest {
                tasks: vec![
                    CreateTaskRequest {
                        title: "Task A".to_string(),
                        description: None,
                        priority: PriorityDto::Low,
                        tags: vec![],
                    },
                    CreateTaskRequest {
                        title: "Task B".to_string(),
                        description: None,
                        priority: PriorityDto::High,
                        tags: vec![],
                    },
                ],
            };

            let result = bulk_create_tasks(State(state.clone()), Json(request)).await;

            assert!(result.is_ok());
            let (status, JsonResponse(response)) = result.unwrap();
            assert_eq!(status, StatusCode::MULTI_STATUS);
            assert_eq!(response.summary.total, 2);
            assert_eq!(response.summary.succeeded, 2);
            assert_eq!(response.summary.failed, 0);

            // Verify tasks were saved (legacy path)
            let count = state.task_repository.count().run_async().await.unwrap();
            assert_eq!(count, 2);
        }

        #[rstest]
        #[tokio::test]
        async fn test_bulk_create_tasks_respects_bulk_config_from_state() {
            // Verify that AppState.bulk_config is used, not BulkConfig::from_env()
            // This is verified by using a custom config that differs from defaults
            let custom_config = BulkConfig {
                chunk_size: 3, // Different from default 50
                concurrency_limit: 1,
                use_bulk_optimization: true,
            };
            let state = create_app_state_with_bulk_config(custom_config);

            // Create more tasks than chunk_size to ensure chunking is applied
            let request = BulkCreateRequest {
                tasks: (0..10)
                    .map(|i| CreateTaskRequest {
                        title: format!("Task {i}"),
                        description: None,
                        priority: PriorityDto::Medium,
                        tags: vec![],
                    })
                    .collect(),
            };

            let result = bulk_create_tasks(State(state.clone()), Json(request)).await;

            assert!(result.is_ok());
            let (status, JsonResponse(response)) = result.unwrap();
            assert_eq!(status, StatusCode::MULTI_STATUS);
            assert_eq!(response.summary.total, 10);
            assert_eq!(response.summary.succeeded, 10);

            // Verify all tasks were saved correctly with custom config
            let count = state.task_repository.count().run_async().await.unwrap();
            assert_eq!(count, 10);
        }

        #[rstest]
        #[tokio::test]
        async fn test_bulk_create_tasks_both_paths_handle_validation_errors() {
            // Test that both optimized and legacy paths correctly handle validation errors
            for use_bulk_optimization in [true, false] {
                let config = BulkConfig {
                    chunk_size: 10,
                    concurrency_limit: 2,
                    use_bulk_optimization,
                };
                let state = create_app_state_with_bulk_config(config);

                let request = BulkCreateRequest {
                    tasks: vec![
                        CreateTaskRequest {
                            title: "Valid Task".to_string(),
                            description: None,
                            priority: PriorityDto::Medium,
                            tags: vec![],
                        },
                        CreateTaskRequest {
                            title: String::new(), // Invalid: empty title
                            description: None,
                            priority: PriorityDto::Medium,
                            tags: vec![],
                        },
                    ],
                };

                let result = bulk_create_tasks(State(state.clone()), Json(request)).await;

                assert!(
                    result.is_ok(),
                    "use_bulk_optimization={use_bulk_optimization}: should not return error"
                );
                let (status, JsonResponse(response)) = result.unwrap();
                assert_eq!(
                    status,
                    StatusCode::MULTI_STATUS,
                    "use_bulk_optimization={use_bulk_optimization}"
                );
                assert_eq!(
                    response.summary.total, 2,
                    "use_bulk_optimization={use_bulk_optimization}"
                );
                assert_eq!(
                    response.summary.succeeded, 1,
                    "use_bulk_optimization={use_bulk_optimization}"
                );
                assert_eq!(
                    response.summary.failed, 1,
                    "use_bulk_optimization={use_bulk_optimization}"
                );
            }
        }
    }

    // -------------------------------------------------------------------------
    // save_chunk Tests
    // -------------------------------------------------------------------------

    mod save_chunk_tests {
        use super::*;
        use crate::domain::{Task, TaskId, Timestamp};
        use crate::infrastructure::InMemoryTaskRepository;

        fn create_test_task(title: &str) -> Task {
            Task::new(TaskId::generate(), title, Timestamp::now())
        }

        #[rstest]
        #[tokio::test]
        async fn test_save_chunk_empty() {
            let repository = Arc::new(InMemoryTaskRepository::new());
            let chunk: Vec<(usize, Task)> = vec![];

            let results = save_chunk(repository, chunk).await;

            assert!(results.is_empty());
        }

        #[rstest]
        #[tokio::test]
        async fn test_save_chunk_single_task() {
            let repository = Arc::new(InMemoryTaskRepository::new());
            let task = create_test_task("Single Task");
            let chunk = vec![(5, task.clone())]; // Index 5

            let results = save_chunk(repository.clone(), chunk).await;

            assert_eq!(results.len(), 1);
            let (index, result) = &results[0];
            assert_eq!(*index, 5);
            assert!(result.is_right());
        }

        #[rstest]
        #[tokio::test]
        async fn test_save_chunk_multiple_tasks() {
            let repository = Arc::new(InMemoryTaskRepository::new());
            let chunk = vec![
                (0, create_test_task("Task 0")),
                (3, create_test_task("Task 3")),
                (7, create_test_task("Task 7")),
            ];

            let results = save_chunk(repository.clone(), chunk).await;

            assert_eq!(results.len(), 3);

            // Check indices are preserved
            let indices: Vec<usize> = results.iter().map(|(i, _)| *i).collect();
            assert_eq!(indices, vec![0, 3, 7]);

            // Check all succeeded
            for (_, result) in &results {
                assert!(result.is_right());
            }
        }

        #[rstest]
        #[tokio::test]
        async fn test_save_chunk_returns_save_results() {
            let repository = Arc::new(InMemoryTaskRepository::new());
            let task = create_test_task("Test Task");
            let task_id = task.task_id.clone();
            let chunk = vec![(42, task)];

            let results = save_chunk(repository.clone(), chunk).await;

            let (index, result) = &results[0];
            assert_eq!(*index, 42);

            match result {
                Either::Right(save_result) => {
                    assert_eq!(save_result.task.task_id, task_id);
                    assert!(!save_result.used_fallback);
                }
                Either::Left(_) => panic!("Expected success"),
            }
        }
    }

    // -------------------------------------------------------------------------
    // save_chunk_with_fallback Tests
    // -------------------------------------------------------------------------

    mod save_chunk_with_fallback_tests {
        use super::*;
        use crate::domain::{Priority, Task, TaskId, TaskStatus, Timestamp};
        use crate::infrastructure::{InMemoryTaskRepository, RepositoryError, SearchScope};
        use lambars::effect::AsyncIO;

        fn create_test_task(title: &str) -> Task {
            Task::new(TaskId::generate(), title, Timestamp::now())
        }

        /// Mock repository that fails bulk save but succeeds on individual save.
        /// This simulates scenarios where bulk operations fail but individual retries work.
        struct BulkFailIndividualSuccessRepository {
            inner: InMemoryTaskRepository,
        }

        impl BulkFailIndividualSuccessRepository {
            fn new() -> Self {
                Self {
                    inner: InMemoryTaskRepository::new(),
                }
            }
        }

        impl TaskRepository for BulkFailIndividualSuccessRepository {
            fn find_by_id(&self, id: &TaskId) -> AsyncIO<Result<Option<Task>, RepositoryError>> {
                self.inner.find_by_id(id)
            }

            fn save(&self, task: &Task) -> AsyncIO<Result<(), RepositoryError>> {
                self.inner.save(task)
            }

            fn save_bulk(&self, tasks: &[Task]) -> AsyncIO<Vec<Result<(), RepositoryError>>> {
                // Always fail bulk save
                let count = tasks.len();
                AsyncIO::pure(
                    (0..count)
                        .map(|_| {
                            Err(RepositoryError::DatabaseError(
                                "Bulk save simulated failure".to_string(),
                            ))
                        })
                        .collect(),
                )
            }

            fn delete(&self, id: &TaskId) -> AsyncIO<Result<bool, RepositoryError>> {
                self.inner.delete(id)
            }

            fn list(
                &self,
                pagination: crate::infrastructure::Pagination,
            ) -> AsyncIO<Result<crate::infrastructure::PaginatedResult<Task>, RepositoryError>>
            {
                self.inner.list(pagination)
            }

            fn list_filtered(
                &self,
                status: Option<TaskStatus>,
                priority: Option<Priority>,
                pagination: crate::infrastructure::Pagination,
            ) -> AsyncIO<Result<crate::infrastructure::PaginatedResult<Task>, RepositoryError>>
            {
                self.inner.list_filtered(status, priority, pagination)
            }

            fn search(
                &self,
                query: &str,
                scope: SearchScope,
                limit: u32,
                offset: u32,
            ) -> AsyncIO<Result<Vec<Task>, RepositoryError>> {
                self.inner.search(query, scope, limit, offset)
            }

            fn count(&self) -> AsyncIO<Result<u64, RepositoryError>> {
                self.inner.count()
            }
        }

        /// Mock repository that fails both bulk and individual save.
        struct AllFailRepository;

        impl TaskRepository for AllFailRepository {
            fn find_by_id(&self, _id: &TaskId) -> AsyncIO<Result<Option<Task>, RepositoryError>> {
                AsyncIO::pure(Ok(None))
            }

            fn save(&self, _task: &Task) -> AsyncIO<Result<(), RepositoryError>> {
                AsyncIO::pure(Err(RepositoryError::DatabaseError(
                    "Individual save simulated failure".to_string(),
                )))
            }

            fn save_bulk(&self, tasks: &[Task]) -> AsyncIO<Vec<Result<(), RepositoryError>>> {
                let count = tasks.len();
                AsyncIO::pure(
                    (0..count)
                        .map(|_| {
                            Err(RepositoryError::DatabaseError(
                                "Bulk save simulated failure".to_string(),
                            ))
                        })
                        .collect(),
                )
            }

            fn delete(&self, _id: &TaskId) -> AsyncIO<Result<bool, RepositoryError>> {
                AsyncIO::pure(Ok(false))
            }

            fn list(
                &self,
                _pagination: crate::infrastructure::Pagination,
            ) -> AsyncIO<Result<crate::infrastructure::PaginatedResult<Task>, RepositoryError>>
            {
                AsyncIO::pure(Ok(crate::infrastructure::PaginatedResult::new(
                    vec![],
                    0,
                    0,
                    10,
                )))
            }

            fn list_filtered(
                &self,
                _status: Option<TaskStatus>,
                _priority: Option<Priority>,
                _pagination: crate::infrastructure::Pagination,
            ) -> AsyncIO<Result<crate::infrastructure::PaginatedResult<Task>, RepositoryError>>
            {
                AsyncIO::pure(Ok(crate::infrastructure::PaginatedResult::new(
                    vec![],
                    0,
                    0,
                    10,
                )))
            }

            fn search(
                &self,
                _query: &str,
                _scope: SearchScope,
                _limit: u32,
                _offset: u32,
            ) -> AsyncIO<Result<Vec<Task>, RepositoryError>> {
                AsyncIO::pure(Ok(vec![]))
            }

            fn count(&self) -> AsyncIO<Result<u64, RepositoryError>> {
                AsyncIO::pure(Ok(0))
            }
        }

        /// Mock repository that fails bulk save for specific tasks (partial failure).
        struct PartialBulkFailRepository {
            inner: InMemoryTaskRepository,
            fail_indices: Vec<usize>,
        }

        impl PartialBulkFailRepository {
            fn new(fail_indices: Vec<usize>) -> Self {
                Self {
                    inner: InMemoryTaskRepository::new(),
                    fail_indices,
                }
            }
        }

        impl TaskRepository for PartialBulkFailRepository {
            fn find_by_id(&self, id: &TaskId) -> AsyncIO<Result<Option<Task>, RepositoryError>> {
                self.inner.find_by_id(id)
            }

            fn save(&self, task: &Task) -> AsyncIO<Result<(), RepositoryError>> {
                self.inner.save(task)
            }

            fn save_bulk(&self, tasks: &[Task]) -> AsyncIO<Vec<Result<(), RepositoryError>>> {
                let fail_indices = self.fail_indices.clone();
                let inner = self.inner.clone();
                // Clone tasks before moving into the closure to avoid lifetime issues
                let tasks_owned = tasks.to_vec();

                AsyncIO::new(move || {
                    let fail_indices = fail_indices.clone();
                    let inner = inner.clone();
                    let tasks = tasks_owned;

                    async move {
                        let mut results = Vec::with_capacity(tasks.len());
                        for (index, task) in tasks.iter().enumerate() {
                            if fail_indices.contains(&index) {
                                results.push(Err(RepositoryError::DatabaseError(format!(
                                    "Bulk save simulated failure for index {index}"
                                ))));
                            } else {
                                // Actually save to inner repository
                                let save_result = inner.save(task).run_async().await;
                                results.push(save_result);
                            }
                        }
                        results
                    }
                })
            }

            fn delete(&self, id: &TaskId) -> AsyncIO<Result<bool, RepositoryError>> {
                self.inner.delete(id)
            }

            fn list(
                &self,
                pagination: crate::infrastructure::Pagination,
            ) -> AsyncIO<Result<crate::infrastructure::PaginatedResult<Task>, RepositoryError>>
            {
                self.inner.list(pagination)
            }

            fn list_filtered(
                &self,
                status: Option<TaskStatus>,
                priority: Option<Priority>,
                pagination: crate::infrastructure::Pagination,
            ) -> AsyncIO<Result<crate::infrastructure::PaginatedResult<Task>, RepositoryError>>
            {
                self.inner.list_filtered(status, priority, pagination)
            }

            fn search(
                &self,
                query: &str,
                scope: SearchScope,
                limit: u32,
                offset: u32,
            ) -> AsyncIO<Result<Vec<Task>, RepositoryError>> {
                self.inner.search(query, scope, limit, offset)
            }

            fn count(&self) -> AsyncIO<Result<u64, RepositoryError>> {
                self.inner.count()
            }
        }

        // -------------------------------------------------------------------------
        // Test: All tasks succeed in bulk save (no fallback needed)
        // -------------------------------------------------------------------------

        #[rstest]
        #[tokio::test]
        async fn test_save_chunk_with_fallback_all_success() {
            let repository = Arc::new(InMemoryTaskRepository::new());
            let chunk = vec![
                (0, create_test_task("Task 0")),
                (1, create_test_task("Task 1")),
                (2, create_test_task("Task 2")),
            ];

            let results = save_chunk_with_fallback(repository.clone(), chunk).await;

            assert_eq!(results.len(), 3);

            // All should succeed without fallback
            for (index, result) in &results {
                match result {
                    Either::Right(save_result) => {
                        assert!(
                            !save_result.used_fallback,
                            "Task at index {index} should not have used fallback"
                        );
                    }
                    Either::Left(error) => {
                        panic!("Task at index {index} failed unexpectedly: {error:?}");
                    }
                }
            }
        }

        // -------------------------------------------------------------------------
        // Test: Partial bulk failure - failed tasks use fallback
        // -------------------------------------------------------------------------

        #[rstest]
        #[tokio::test]
        async fn test_save_chunk_with_fallback_partial_failure() {
            // Fail bulk save for index 1 only
            let repository = Arc::new(PartialBulkFailRepository::new(vec![1]));
            let chunk = vec![
                (0, create_test_task("Task 0")),
                (1, create_test_task("Task 1")),
                (2, create_test_task("Task 2")),
            ];

            let results = save_chunk_with_fallback(repository.clone(), chunk).await;

            assert_eq!(results.len(), 3);

            // Sort results by index for predictable assertions
            let mut sorted_results = results;
            sorted_results.sort_by_key(|(index, _)| *index);

            // Index 0: should succeed without fallback
            match &sorted_results[0].1 {
                Either::Right(save_result) => {
                    assert!(
                        !save_result.used_fallback,
                        "Index 0 should not use fallback"
                    );
                }
                Either::Left(error) => panic!("Index 0 failed unexpectedly: {error:?}"),
            }

            // Index 1: should succeed WITH fallback (bulk failed, individual succeeded)
            match &sorted_results[1].1 {
                Either::Right(save_result) => {
                    assert!(save_result.used_fallback, "Index 1 should use fallback");
                }
                Either::Left(error) => panic!("Index 1 failed unexpectedly: {error:?}"),
            }

            // Index 2: should succeed without fallback
            match &sorted_results[2].1 {
                Either::Right(save_result) => {
                    assert!(
                        !save_result.used_fallback,
                        "Index 2 should not use fallback"
                    );
                }
                Either::Left(error) => panic!("Index 2 failed unexpectedly: {error:?}"),
            }
        }

        // -------------------------------------------------------------------------
        // Test: All bulk save fail, all use fallback (all succeed via individual)
        // -------------------------------------------------------------------------

        #[rstest]
        #[tokio::test]
        async fn test_save_chunk_with_fallback_all_bulk_fail_individual_succeed() {
            let repository = Arc::new(BulkFailIndividualSuccessRepository::new());
            let chunk = vec![
                (0, create_test_task("Task 0")),
                (1, create_test_task("Task 1")),
            ];

            let results = save_chunk_with_fallback(repository.clone(), chunk).await;

            assert_eq!(results.len(), 2);

            // All should succeed WITH fallback
            for (index, result) in &results {
                match result {
                    Either::Right(save_result) => {
                        assert!(
                            save_result.used_fallback,
                            "Task at index {index} should have used fallback"
                        );
                    }
                    Either::Left(error) => {
                        panic!("Task at index {index} failed unexpectedly: {error:?}");
                    }
                }
            }
        }

        // -------------------------------------------------------------------------
        // Test: Both bulk and individual fail - returns RepositoryFallbackFailed
        // -------------------------------------------------------------------------

        #[rstest]
        #[tokio::test]
        async fn test_save_chunk_with_fallback_all_fail_returns_repository_fallback_failed() {
            let repository = Arc::new(AllFailRepository);
            let chunk = vec![
                (0, create_test_task("Task 0")),
                (1, create_test_task("Task 1")),
            ];

            let results = save_chunk_with_fallback(repository.clone(), chunk).await;

            assert_eq!(results.len(), 2);

            // All should fail with RepositoryFallbackFailed containing both errors
            for (index, result) in &results {
                match result {
                    Either::Left(ItemError::RepositoryFallbackFailed {
                        original_error,
                        fallback_error,
                    }) => {
                        // Verify both errors contain meaningful information
                        let original_message = format!("{original_error}");
                        let fallback_message = format!("{fallback_error}");
                        assert!(
                            original_message.contains("simulated failure"),
                            "Original error at index {index} should contain error details: {original_message}"
                        );
                        assert!(
                            fallback_message.contains("simulated failure"),
                            "Fallback error at index {index} should contain error details: {fallback_message}"
                        );
                    }
                    Either::Left(other_error) => {
                        panic!("Task at index {index} returned wrong error type: {other_error:?}");
                    }
                    Either::Right(_) => {
                        panic!("Task at index {index} succeeded unexpectedly");
                    }
                }
            }
        }

        // -------------------------------------------------------------------------
        // Test: Empty chunk
        // -------------------------------------------------------------------------

        #[rstest]
        #[tokio::test]
        async fn test_save_chunk_with_fallback_empty() {
            let repository = Arc::new(InMemoryTaskRepository::new());
            let chunk: Vec<(usize, Task)> = vec![];

            let results = save_chunk_with_fallback(repository, chunk).await;

            assert!(results.is_empty());
        }

        // -------------------------------------------------------------------------
        // Test: Preserves original indices
        // -------------------------------------------------------------------------

        #[rstest]
        #[tokio::test]
        async fn test_save_chunk_with_fallback_preserves_indices() {
            let repository = Arc::new(InMemoryTaskRepository::new());
            // Non-sequential indices
            let chunk = vec![
                (5, create_test_task("Task 5")),
                (10, create_test_task("Task 10")),
                (3, create_test_task("Task 3")),
            ];

            let results = save_chunk_with_fallback(repository.clone(), chunk).await;

            // Verify all original indices are present
            let indices: std::collections::HashSet<usize> =
                results.iter().map(|(index, _)| *index).collect();

            assert!(indices.contains(&5));
            assert!(indices.contains(&10));
            assert!(indices.contains(&3));
            assert_eq!(indices.len(), 3);
        }
    }
}
