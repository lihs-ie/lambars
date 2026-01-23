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

use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};

use super::dto::{
    CreateTaskRequest, PriorityDto, TaskResponse, TaskStatusDto, validate_description,
    validate_tags, validate_title,
};
use super::error::ApiErrorResponse;
use super::handlers::AppState;
use crate::domain::{Priority, Tag, Task, TaskId, TaskStatus, Timestamp};
use crate::infrastructure::TaskRepository;
use lambars::control::Either;
use lambars::for_;
use lambars::persistent::PersistentHashSet;
use lambars::typeclass::Alternative;

// =============================================================================
// Constants
// =============================================================================

/// Maximum number of items in a bulk request.
const BULK_LIMIT: usize = 100;

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
    /// Repository operation failed (internal error, details logged).
    RepositoryError,
    /// All save strategies failed (primary and fallback).
    AllStrategiesFailed,
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
            ItemError::RepositoryError => Self {
                code: "REPOSITORY_ERROR".to_string(),
                message: "Internal error occurred".to_string(),
            },
            ItemError::AllStrategiesFailed => Self {
                code: "ALL_STRATEGIES_FAILED".to_string(),
                message: "All save strategies failed (primary and fallback)".to_string(),
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
) -> Result<(StatusCode, Json<BulkResponse>), ApiErrorResponse> {
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
    let results = save_tasks_bulk(state.task_repository.clone(), tasks_to_save).await;

    // Step 6: Aggregate results (pure)
    let response = aggregate_create_results(results);

    Ok((StatusCode::MULTI_STATUS, Json(response)))
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
) -> Result<(StatusCode, Json<BulkResponse>), ApiErrorResponse> {
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
    let all_results = merge_update_results(&request.updates, unique_results, duplicate_errors);

    // Step 6: Aggregate results (pure)
    let response = aggregate_update_results(all_results);

    Ok((StatusCode::MULTI_STATUS, Json(response)))
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
fn merge_update_results(
    original_updates: &[BulkUpdateItem],
    unique_results: Vec<(usize, Either<ItemError, Task>)>,
    duplicate_errors: Vec<(usize, ItemError)>,
) -> Vec<Either<ItemError, Task>> {
    let mut results: Vec<Option<Either<ItemError, Task>>> = vec![None; original_updates.len()];

    // Place unique results
    for (index, result) in unique_results {
        results[index] = Some(result);
    }

    // Place duplicate errors
    for (index, error) in duplicate_errors {
        results[index] = Some(Either::Left(error));
    }

    // All should be filled
    results.into_iter().flatten().collect()
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
async fn save_with_alternative_fallback(
    repository: Arc<dyn TaskRepository + Send + Sync>,
    task: Task,
) -> Either<ItemError, SaveResult> {
    // Primary save attempt
    let primary_result: Option<SaveResult> = try_primary_save(&repository, &task).await;

    // Lazy fallback: only execute if primary failed (short-circuit semantics)
    // This demonstrates the proper behavior of Alternative::alt with lazy evaluation
    let combined_result = match primary_result {
        Some(result) => Some(result), // Primary succeeded - no fallback needed
        None => try_fallback_save(&repository, &task).await, // Primary failed - try fallback
    };

    combined_result.map_or_else(
        || Either::Left(ItemError::AllStrategiesFailed),
        Either::Right,
    )
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

/// Attempts primary save strategy (I/O boundary).
///
/// Returns `Some(SaveResult)` if successful, `None` if failed.
async fn try_primary_save(
    repository: &Arc<dyn TaskRepository + Send + Sync>,
    task: &Task,
) -> Option<SaveResult> {
    match repository.save(task).run_async().await {
        Ok(()) => Some(SaveResult {
            task: task.clone(),
            used_fallback: false,
        }),
        Err(e) => {
            tracing::warn!(error = %e, "Primary save failed, attempting fallback");
            None
        }
    }
}

/// Attempts fallback save strategy (I/O boundary).
///
/// This demonstrates an alternative save path that could:
/// - Use a different repository
/// - Apply data transformation before save
/// - Use a queue for deferred processing
///
/// For this demo, it retries the same repository (simulating a retry strategy).
async fn try_fallback_save(
    repository: &Arc<dyn TaskRepository + Send + Sync>,
    task: &Task,
) -> Option<SaveResult> {
    // Simulate a retry or alternative strategy
    // In production, this could be a different repository, cache, or queue
    match repository.save(task).run_async().await {
        Ok(()) => {
            tracing::info!("Fallback save succeeded");
            Some(SaveResult {
                task: task.clone(),
                used_fallback: true,
            })
        }
        Err(e) => {
            tracing::error!(error = %e, "Fallback save also failed");
            None
        }
    }
}

/// Combines multiple save strategies using `Alternative::choice` (pure helper).
///
/// This demonstrates selecting the first successful result from multiple strategies.
#[allow(dead_code)]
fn combine_save_strategies(strategies: Vec<Option<SaveResult>>) -> Option<SaveResult> {
    Option::choice(strategies)
}

/// Processes unique updates (I/O boundary).
async fn process_unique_updates(
    repository: Arc<dyn TaskRepository + Send + Sync>,
    updates: &[BulkUpdateItem],
    unique_indices: &[usize],
    now: Timestamp,
) -> Vec<(usize, Either<ItemError, Task>)> {
    let mut results = Vec::with_capacity(unique_indices.len());

    for &index in unique_indices {
        let update = &updates[index];
        let task_id = TaskId::from_uuid(update.id);

        let result = match repository.find_by_id(&task_id).run_async().await {
            Ok(Some(task)) => {
                // Check version (positive condition first)
                if task.version == update.version {
                    // Apply update (returns None if no changes)
                    match apply_update(task.clone(), update, now.clone()) {
                        Some(updated_task) => {
                            match repository.save(&updated_task).run_async().await {
                                Ok(()) => Either::Right(updated_task),
                                Err(e) => {
                                    tracing::error!(error = %e, task_id = %task_id, "Repository save failed");
                                    Either::Left(ItemError::RepositoryError)
                                }
                            }
                        }
                        None => Either::Right(task), // No changes, return original
                    }
                } else {
                    Either::Left(ItemError::VersionConflict {
                        id: task_id,
                        expected: update.version,
                        actual: task.version,
                    })
                }
            }
            Ok(None) => Either::Left(ItemError::NotFound { id: task_id }),
            Err(e) => {
                tracing::error!(error = %e, task_id = %task_id, "Repository find failed");
                Either::Left(ItemError::RepositoryError)
            }
        };

        results.push((index, result));
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
        let unique_results = vec![(0, Either::Right(task.clone())), (2, Either::Right(task))];

        let duplicate_errors = vec![(
            1,
            ItemError::DuplicateId {
                id: TaskId::from_uuid(id1),
            },
        )];

        let merged = merge_update_results(&updates, unique_results, duplicate_errors);

        assert_eq!(merged.len(), 3);
        assert!(merged[0].is_right()); // First id1 - success
        assert!(merged[1].is_left()); // Second id1 - duplicate error
        assert!(merged[2].is_right()); // id2 - success
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
    fn test_item_error_all_strategies_failed() {
        let error = ItemError::AllStrategiesFailed;
        let bulk_error: BulkItemError = error.into();

        assert_eq!(bulk_error.code, "ALL_STRATEGIES_FAILED");
        assert!(bulk_error.message.contains("All save strategies failed"));
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
}
