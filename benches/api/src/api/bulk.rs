//! Bulk operation handlers for task management.
//!
//! This module demonstrates lambars' functional programming patterns for bulk operations:
//! - **`Either`**: Representing success/failure for individual items
//! - **`Bifunctor::bimap`**: Transforming both success and failure cases
//! - **`PersistentHashSet`**: Deduplication of IDs
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
use lambars::persistent::PersistentHashSet;

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

/// Validates all create requests (pure function).
fn validate_create_requests(
    requests: &[CreateTaskRequest],
) -> Vec<Either<ItemError, ValidatedCreate>> {
    requests.iter().map(validate_single_create).collect()
}

/// Validates a single create request (pure function).
fn validate_single_create(request: &CreateTaskRequest) -> Either<ItemError, ValidatedCreate> {
    // Validate title
    let title = match validate_title(&request.title) {
        Either::Right(t) => t,
        Either::Left(e) => {
            let first_error = e.errors.first().map_or_else(
                || ("title".to_string(), "validation error".to_string()),
                |f| (f.field.clone(), f.message.clone()),
            );
            return Either::Left(ItemError::Validation {
                field: first_error.0,
                message: first_error.1,
            });
        }
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

    // Validate tags
    let tags = match validate_tags(&request.tags) {
        Either::Right(t) => t,
        Either::Left(e) => {
            let first_error = e.errors.first().map_or_else(
                || ("tags".to_string(), "validation error".to_string()),
                |f| (f.field.clone(), f.message.clone()),
            );
            return Either::Left(ItemError::Validation {
                field: first_error.0,
                message: first_error.1,
            });
        }
    };

    Either::Right(ValidatedCreate {
        title,
        description,
        priority: Priority::from(request.priority),
        tags,
    })
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
fn aggregate_create_results(results: Vec<Either<ItemError, Task>>) -> BulkResponse {
    let total = results.len();
    let mut succeeded = 0;
    let mut failed = 0;

    let item_results: Vec<BulkItemResult> = results
        .into_iter()
        .map(|result| match result {
            Either::Right(task) => {
                succeeded += 1;
                BulkItemResult::Created {
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

/// Saves tasks in bulk (I/O boundary).
async fn save_tasks_bulk(
    repository: Arc<dyn TaskRepository + Send + Sync>,
    tasks: Vec<Either<ItemError, Task>>,
) -> Vec<Either<ItemError, Task>> {
    let mut results = Vec::with_capacity(tasks.len());

    for task_result in tasks {
        let result = match task_result {
            Either::Left(error) => Either::Left(error),
            Either::Right(task) => match repository.save(&task).run_async().await {
                Ok(()) => Either::Right(task),
                Err(e) => {
                    tracing::error!(error = %e, "Repository save failed");
                    Either::Left(ItemError::RepositoryError)
                }
            },
        };
        results.push(result);
    }

    results
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

        let results = vec![Either::Right(task1), Either::Right(task2)];

        let response = aggregate_create_results(results);

        assert_eq!(response.summary.total, 2);
        assert_eq!(response.summary.succeeded, 2);
        assert_eq!(response.summary.failed, 0);
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

        let results: Vec<Either<ItemError, Task>> =
            vec![Either::Left(error1), Either::Left(error2)];

        let response = aggregate_create_results(results);

        assert_eq!(response.summary.total, 2);
        assert_eq!(response.summary.succeeded, 0);
        assert_eq!(response.summary.failed, 2);
    }

    #[rstest]
    fn test_aggregate_create_results_mixed() {
        let task = Task::new(TaskId::generate(), "Task 1", Timestamp::now());
        let error = ItemError::Validation {
            field: "title".to_string(),
            message: "empty".to_string(),
        };

        let results: Vec<Either<ItemError, Task>> = vec![Either::Right(task), Either::Left(error)];

        let response = aggregate_create_results(results);

        assert_eq!(response.summary.total, 2);
        assert_eq!(response.summary.succeeded, 1);
        assert_eq!(response.summary.failed, 1);
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
}
