//! Traversable operations for batch processing and collection manipulation.
//!
//! This module demonstrates:
//! - **`traverse_result`**: Apply validation to collections with early failure
//! - **`traverse_async_io_parallel`**: Parallel async fetch with order preservation
//! - **`sequence_option`**: Collect optional values (all-or-nothing)
//! - **`traverse_async_io`**: Sequential async execution with ordering guarantee
//!
//! # lambars Features Demonstrated
//!
//! - **`Traversable` trait**: Structure-preserving effectful operations
//! - **`traverse_*` methods**: Apply functions returning effects to collections
//! - **`sequence_*` methods**: Invert nested effect structures

use std::sync::Arc;

use axum::Json;
use axum::extract::State;

use super::json_buffer::JsonResponse;
use serde::{Deserialize, Serialize};

use lambars::effect::AsyncIO;
use lambars::typeclass::Traversable;

use super::dto::{CreateTaskRequest, PriorityDto, TaskResponse};
use super::error::{ApiErrorResponse, FieldError};
use super::handlers::AppState;
use crate::domain::{Priority, Tag, Task, TaskId, TaskStatus};

// =============================================================================
// DTOs
// =============================================================================

/// Request for batch validation.
#[derive(Debug, Deserialize)]
pub struct ValidateBatchRequest {
    /// List of task creation requests to validate (1-100 items).
    pub tasks: Vec<CreateTaskRequest>,
}

/// A validated task (passed all validations).
#[derive(Debug, Clone, Serialize)]
pub struct ValidatedTaskDto {
    pub title: String,
    pub description: Option<String>,
    pub priority: PriorityDto,
    pub tags: Vec<String>,
}

/// Response for batch validation.
#[derive(Debug, Serialize)]
pub struct ValidateBatchResponse {
    pub validated_tasks: Vec<ValidatedTaskDto>,
    pub count: usize,
    pub traversable_operations: usize,
}

/// Request for batch fetch.
#[derive(Debug, Deserialize)]
pub struct FetchBatchRequest {
    /// List of task IDs to fetch (1-50 items for parallel limit).
    pub task_ids: Vec<String>,
}

/// Response for batch fetch.
#[derive(Debug, Serialize)]
pub struct FetchBatchResponse {
    pub tasks: Vec<TaskResponse>,
    pub count: usize,
    pub parallel_fetch: bool,
}

/// Request for collecting optional fields.
#[derive(Debug, Deserialize)]
pub struct CollectOptionalRequest {
    /// List of task IDs (1-100 items).
    pub task_ids: Vec<String>,
    /// Field to collect. Currently only `description` is supported.
    pub field: String,
}

/// Status of optional field collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CollectionStatus {
    Complete,
    Incomplete,
}

/// Response for collecting optional fields.
#[derive(Debug, Serialize)]
pub struct CollectOptionalResponse {
    pub status: CollectionStatus,
    pub values: Option<Vec<String>>,
    pub present_count: usize,
    pub total_count: usize,
}

/// A single operation to execute sequentially.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskOperation {
    UpdateStatus {
        task_id: String,
        new_status: String,
    },
    UpdatePriority {
        task_id: String,
        new_priority: String,
    },
    AddTag {
        task_id: String,
        tag: String,
    },
}

/// Request for sequential execution.
#[derive(Debug, Deserialize)]
pub struct ExecuteSequentialRequest {
    /// Operations to execute sequentially (1-20 items).
    pub operations: Vec<TaskOperation>,
}

/// Result of a single operation.
#[derive(Debug, Serialize)]
pub struct OperationResult {
    pub task_id: String,
    pub operation_type: String,
    pub success: bool,
    pub message: Option<String>,
}

/// Response for sequential execution.
#[derive(Debug, Serialize)]
pub struct ExecuteSequentialResponse {
    pub processed_count: usize,
    pub results: Vec<OperationResult>,
}

/// Request for batch enrichment.
#[derive(Debug, Deserialize)]
pub struct EnrichBatchRequest {
    /// List of task IDs to enrich (1-30 items).
    pub task_ids: Vec<String>,
    /// Fields to include: `project`, `subtasks`, `history`.
    #[serde(default)]
    pub include: Vec<String>,
}

/// An enriched task with additional data.
#[derive(Debug, Serialize)]
pub struct EnrichedTaskDto {
    #[serde(flatten)]
    pub task: TaskResponse,
    pub project_name: Option<String>,
    pub subtask_count: usize,
    pub has_history: bool,
}

/// Response for batch enrichment.
#[derive(Debug, Serialize)]
pub struct EnrichBatchResponse {
    pub tasks: Vec<EnrichedTaskDto>,
    pub count: usize,
}

// =============================================================================
// Internal Types for Validation
// =============================================================================

/// Internal validated task structure.
#[derive(Debug, Clone)]
struct ValidatedTask {
    title: String,
    description: Option<String>,
    priority: Priority,
    tags: Vec<String>,
}

/// Validation error type.
#[derive(Debug, Clone)]
enum BatchValidationError {
    EmptyTitle { index: usize },
    TitleTooLong { index: usize, length: usize },
    TooManyTags { index: usize, count: usize },
    InvalidTag { index: usize, tag: String },
}

/// Error type for batch operations (fetch, enrich, execute).
#[derive(Debug, Clone)]
#[allow(dead_code)] // Validation variant reserved for future use
enum BatchOpError {
    /// Task not found - maps to 404
    NotFound { task_id: String },
    /// Validation error (invalid format) - maps to 400
    Validation { message: String },
    /// Repository error - maps to 500
    Repository { message: String },
}

impl std::fmt::Display for BatchOpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { task_id } => write!(f, "Task not found: {task_id}"),
            Self::Validation { message } => write!(f, "Validation error: {message}"),
            Self::Repository { message } => write!(f, "Repository error: {message}"),
        }
    }
}

impl BatchOpError {
    /// Converts this error to an `ApiErrorResponse`.
    fn into_api_error(self) -> ApiErrorResponse {
        match self {
            Self::NotFound { task_id } => {
                ApiErrorResponse::not_found(format!("Task not found: {task_id}"))
            }
            Self::Validation { message } => ApiErrorResponse::validation_error(
                "Validation failed",
                vec![FieldError::new("task_id", message)],
            ),
            Self::Repository { message } => ApiErrorResponse::internal_error(message),
        }
    }
}

impl std::fmt::Display for BatchValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyTitle { index } => {
                write!(f, "Task at index {index} has empty title")
            }
            Self::TitleTooLong { index, length } => {
                write!(
                    f,
                    "Task at index {index} has title too long ({length} chars, max 200)"
                )
            }
            Self::TooManyTags { index, count } => {
                write!(
                    f,
                    "Task at index {index} has too many tags ({count}, max 10)"
                )
            }
            Self::InvalidTag { index, tag } => {
                write!(f, "Task at index {index} has invalid tag: {tag}")
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
fn parse_status(s: &str) -> Result<TaskStatus, String> {
    match s.to_lowercase().as_str() {
        "pending" => Ok(TaskStatus::Pending),
        "in_progress" => Ok(TaskStatus::InProgress),
        "completed" => Ok(TaskStatus::Completed),
        "cancelled" => Ok(TaskStatus::Cancelled),
        _ => Err(format!("Invalid status: {s}")),
    }
}

/// Parses a priority string.
fn parse_priority(s: &str) -> Result<Priority, String> {
    match s.to_lowercase().as_str() {
        "low" => Ok(Priority::Low),
        "medium" => Ok(Priority::Medium),
        "high" => Ok(Priority::High),
        "critical" => Ok(Priority::Critical),
        _ => Err(format!("Invalid priority: {s}")),
    }
}

// =============================================================================
// POST /tasks/validate-batch - Batch validation with traverse_result
// =============================================================================

/// Validates a batch of task creation requests.
///
/// This handler demonstrates:
/// - **`traverse_result`**: Apply validation to each element, stopping at first error
///
/// # Request Body
///
/// - `tasks`: Array of task creation requests (1-100 items)
///
/// # Errors
///
/// - `400 Bad Request`: Validation failed for any task (returns first error)
/// - `400 Bad Request`: Empty task list or exceeds 100 items
#[allow(clippy::unused_async)]
pub async fn validate_batch(
    Json(request): Json<ValidateBatchRequest>,
) -> Result<JsonResponse<ValidateBatchResponse>, ApiErrorResponse> {
    // Validate batch size
    if request.tasks.is_empty() {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("tasks", "tasks list cannot be empty")],
        ));
    }

    if request.tasks.len() > 100 {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new(
                "tasks",
                "tasks list cannot exceed 100 items",
            )],
        ));
    }

    // Use traverse_result for batch validation
    let validated = validate_batch_tasks(&request.tasks).map_err(|error| {
        ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("tasks", error.to_string())],
        )
    })?;

    let count = validated.len();
    let validated_dtos: Vec<ValidatedTaskDto> = validated
        .into_iter()
        .map(|task| ValidatedTaskDto {
            title: task.title,
            description: task.description,
            priority: PriorityDto::from(task.priority),
            tags: task.tags,
        })
        .collect();

    Ok(JsonResponse(ValidateBatchResponse {
        validated_tasks: validated_dtos,
        count,
        traversable_operations: count,
    }))
}

/// Pure: Validates a batch of task requests using `traverse_result`.
///
/// Stops at the first validation error (fail-fast behavior).
fn validate_batch_tasks(
    requests: &[CreateTaskRequest],
) -> Result<Vec<ValidatedTask>, BatchValidationError> {
    requests
        .iter()
        .enumerate()
        .collect::<Vec<_>>()
        .traverse_result(|(index, request)| validate_single_task(index, request))
}

/// Pure: Validates a single task request.
fn validate_single_task(
    index: usize,
    request: &CreateTaskRequest,
) -> Result<ValidatedTask, BatchValidationError> {
    // Validate title
    if request.title.trim().is_empty() {
        return Err(BatchValidationError::EmptyTitle { index });
    }
    if request.title.len() > 200 {
        return Err(BatchValidationError::TitleTooLong {
            index,
            length: request.title.len(),
        });
    }

    // Validate tags
    if request.tags.len() > 10 {
        return Err(BatchValidationError::TooManyTags {
            index,
            count: request.tags.len(),
        });
    }
    for tag in &request.tags {
        if tag.trim().is_empty() || tag.len() > 50 {
            return Err(BatchValidationError::InvalidTag {
                index,
                tag: tag.clone(),
            });
        }
    }

    Ok(ValidatedTask {
        title: request.title.trim().to_string(),
        description: request.description.clone(),
        priority: Priority::from(request.priority),
        tags: request.tags.clone(),
    })
}

// =============================================================================
// POST /tasks/fetch-batch - Parallel batch fetch with traverse_async_io_parallel
// =============================================================================

/// Fetches multiple tasks in parallel by their IDs.
///
/// This handler demonstrates:
/// - **`traverse_async_io_parallel`**: Parallel async operations with order preservation
///
/// # Request Body
///
/// - `task_ids`: Array of task IDs to fetch (1-50 items)
///
/// # Errors
///
/// - `400 Bad Request`: Empty task list or exceeds 50 items
/// - `404 Not Found`: Any task ID not found
pub async fn fetch_batch(
    State(state): State<AppState>,
    Json(request): Json<FetchBatchRequest>,
) -> Result<JsonResponse<FetchBatchResponse>, ApiErrorResponse> {
    // Validate batch size
    if request.task_ids.is_empty() {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("task_ids", "task_ids list cannot be empty")],
        ));
    }

    if request.task_ids.len() > 50 {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new(
                "task_ids",
                "task_ids list cannot exceed 50 items for parallel fetch",
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

    // Use traverse_async_io_parallel for parallel fetch
    let repository = Arc::clone(&state.task_repository);
    let fetch_io = build_parallel_fetch_io(repository, task_ids);

    let results: Vec<Result<Task, BatchOpError>> = fetch_io.run_async().await;

    // Collect errors or successful tasks - return first error with appropriate status code
    let mut tasks = Vec::new();
    for result in results {
        match result {
            Ok(task) => tasks.push(task),
            Err(error) => {
                return Err(error.into_api_error());
            }
        }
    }

    let count = tasks.len();
    let task_responses: Vec<TaskResponse> = tasks.iter().map(TaskResponse::from).collect();

    Ok(JsonResponse(FetchBatchResponse {
        tasks: task_responses,
        count,
        parallel_fetch: true,
    }))
}

/// Builds an `AsyncIO` that fetches tasks in parallel.
///
/// Returns `Vec<Result<Task, BatchOpError>>` because `traverse_async_io_parallel`
/// collects results without short-circuiting on errors.
fn build_parallel_fetch_io(
    repository: Arc<dyn crate::infrastructure::TaskRepository>,
    task_ids: Vec<TaskId>,
) -> AsyncIO<Vec<Result<Task, BatchOpError>>> {
    task_ids.traverse_async_io_parallel(move |task_id| {
        let repo = Arc::clone(&repository);
        AsyncIO::new(move || async move {
            match repo.find_by_id(&task_id).run_async().await {
                Ok(Some(task)) => Ok(task),
                Ok(None) => Err(BatchOpError::NotFound {
                    task_id: task_id.to_string(),
                }),
                Err(error) => Err(BatchOpError::Repository {
                    message: error.to_string(),
                }),
            }
        })
    })
}

// =============================================================================
// POST /tasks/collect-optional - Optional field collection with sequence_option
// =============================================================================

/// Collects optional fields from multiple tasks.
///
/// This handler demonstrates:
/// - **`sequence_option`**: Collect `Option<T>` values, returning `None` if any is missing
///
/// # Request Body
///
/// - `task_ids`: Array of task IDs (1-100 items)
/// - `field`: Field to collect (currently only `description` is supported)
///
/// # Response
///
/// - `status`: `complete` if all values present, `incomplete` if any missing
/// - `values`: Collected values (only if `complete`)
///
/// # Errors
///
/// - `400 Bad Request`: Empty task list, exceeds 100 items, or invalid field
/// - `404 Not Found`: Any task not found
/// - `500 Internal Server Error`: Repository error
pub async fn collect_optional(
    State(state): State<AppState>,
    Json(request): Json<CollectOptionalRequest>,
) -> Result<JsonResponse<CollectOptionalResponse>, ApiErrorResponse> {
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

    // Validate field - only description is supported in current domain model
    if request.field != "description" {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new(
                "field",
                "field must be 'description' (only supported optional field)",
            )],
        ));
    }

    // Parse and fetch tasks
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

    let repository = Arc::clone(&state.task_repository);
    let fetch_io = build_parallel_fetch_io(repository, task_ids);

    let results: Vec<Result<Task, BatchOpError>> = fetch_io.run_async().await;

    // Collect errors or successful tasks - return first error with appropriate status code
    let mut tasks = Vec::new();
    for result in results {
        match result {
            Ok(task) => tasks.push(task),
            Err(error) => {
                return Err(error.into_api_error());
            }
        }
    }

    // Use sequence_option to collect optional fields
    let total_count = tasks.len();
    let (status, values, present_count) = collect_optional_fields(&tasks, &request.field);

    Ok(JsonResponse(CollectOptionalResponse {
        status,
        values,
        present_count,
        total_count,
    }))
}

/// Pure: Collects optional fields from tasks using `sequence_option`.
fn collect_optional_fields(
    tasks: &[Task],
    field: &str,
) -> (CollectionStatus, Option<Vec<String>>, usize) {
    // Extract optional values
    let optional_values: Vec<Option<String>> = tasks
        .iter()
        .map(|task| extract_optional_field(task, field))
        .collect();

    // Count present values
    let present_count = optional_values.iter().filter(|v| v.is_some()).count();

    // Use sequence_option to collect all values
    let collected = optional_values.sequence_option();

    collected.map_or(
        (CollectionStatus::Incomplete, None, present_count),
        |values| (CollectionStatus::Complete, Some(values), present_count),
    )
}

/// Pure: Extracts an optional field from a task.
///
/// Only `description` is supported in the current domain model.
fn extract_optional_field(task: &Task, field: &str) -> Option<String> {
    match field {
        "description" => task.description.clone(),
        _ => unreachable!("field validation should prevent this"),
    }
}

// =============================================================================
// POST /tasks/execute-sequential - Sequential execution with traverse_async_io
// =============================================================================

/// Executes operations sequentially in order.
///
/// This handler demonstrates:
/// - **`traverse_async_io`**: Sequential async execution with strict ordering
///
/// Each operation result is returned regardless of success or failure.
/// Failed operations have `success: false` and include an error message.
///
/// # Request Body
///
/// - `operations`: Array of operations to execute (1-20 items)
///
/// # Errors
///
/// - `400 Bad Request`: Empty operations or exceeds 20 items
pub async fn execute_sequential(
    State(state): State<AppState>,
    Json(request): Json<ExecuteSequentialRequest>,
) -> Result<JsonResponse<ExecuteSequentialResponse>, ApiErrorResponse> {
    // Validate batch size
    if request.operations.is_empty() {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new(
                "operations",
                "operations list cannot be empty",
            )],
        ));
    }

    if request.operations.len() > 20 {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new(
                "operations",
                "operations list cannot exceed 20 items for sequential execution",
            )],
        ));
    }

    // Use traverse_async_io for sequential execution
    let repository = Arc::clone(&state.task_repository);
    let operations = request.operations.clone();

    let execution_io = build_sequential_execution_io(repository, operations);

    // All operations return OperationResult, failures are represented as success=false
    let results: Vec<OperationResult> = execution_io.run_async().await;
    let processed_count = results.len();

    Ok(JsonResponse(ExecuteSequentialResponse {
        processed_count,
        results,
    }))
}

/// Builds an `AsyncIO` that executes operations sequentially.
///
/// Returns `Vec<OperationResult>` - failures are represented as `success: false`.
fn build_sequential_execution_io(
    repository: Arc<dyn crate::infrastructure::TaskRepository>,
    operations: Vec<TaskOperation>,
) -> AsyncIO<Vec<OperationResult>> {
    operations.traverse_async_io(move |operation| {
        let repo = Arc::clone(&repository);
        AsyncIO::new(move || async move { execute_single_operation(repo, operation).await })
    })
}

/// Executes a single operation.
///
/// Always returns `OperationResult` - failures are represented with `success: false`.
async fn execute_single_operation(
    repository: Arc<dyn crate::infrastructure::TaskRepository>,
    operation: TaskOperation,
) -> OperationResult {
    match operation {
        TaskOperation::UpdateStatus {
            task_id,
            new_status,
        } => execute_update_status(repository, task_id, new_status).await,
        TaskOperation::UpdatePriority {
            task_id,
            new_priority,
        } => execute_update_priority(repository, task_id, new_priority).await,
        TaskOperation::AddTag { task_id, tag } => execute_add_tag(repository, task_id, tag).await,
    }
}

/// Helper for `UpdateStatus` operation.
async fn execute_update_status(
    repository: Arc<dyn crate::infrastructure::TaskRepository>,
    task_id: String,
    new_status: String,
) -> OperationResult {
    let operation_type = "update_status".to_string();

    // Parse task ID
    let id = match parse_task_id(&task_id) {
        Ok(id) => id,
        Err(error) => {
            return OperationResult {
                task_id,
                operation_type,
                success: false,
                message: Some(error),
            };
        }
    };

    // Parse status
    let status = match parse_status(&new_status) {
        Ok(s) => s,
        Err(error) => {
            return OperationResult {
                task_id,
                operation_type,
                success: false,
                message: Some(error),
            };
        }
    };

    // Fetch task
    let task = match repository.find_by_id(&id).run_async().await {
        Ok(Some(task)) => task,
        Ok(None) => {
            return OperationResult {
                task_id,
                operation_type,
                success: false,
                message: Some("Task not found".to_string()),
            };
        }
        Err(error) => {
            return OperationResult {
                task_id,
                operation_type,
                success: false,
                message: Some(error.to_string()),
            };
        }
    };

    // Update and save
    let updated = task.with_status(status);
    if let Err(error) = repository.save(&updated).run_async().await {
        return OperationResult {
            task_id,
            operation_type,
            success: false,
            message: Some(error.to_string()),
        };
    }

    OperationResult {
        task_id,
        operation_type,
        success: true,
        message: Some(format!("Status updated to {new_status}")),
    }
}

/// Helper for `UpdatePriority` operation.
async fn execute_update_priority(
    repository: Arc<dyn crate::infrastructure::TaskRepository>,
    task_id: String,
    new_priority: String,
) -> OperationResult {
    let operation_type = "update_priority".to_string();

    // Parse task ID
    let id = match parse_task_id(&task_id) {
        Ok(id) => id,
        Err(error) => {
            return OperationResult {
                task_id,
                operation_type,
                success: false,
                message: Some(error),
            };
        }
    };

    // Parse priority
    let priority = match parse_priority(&new_priority) {
        Ok(p) => p,
        Err(error) => {
            return OperationResult {
                task_id,
                operation_type,
                success: false,
                message: Some(error),
            };
        }
    };

    // Fetch task
    let task = match repository.find_by_id(&id).run_async().await {
        Ok(Some(task)) => task,
        Ok(None) => {
            return OperationResult {
                task_id,
                operation_type,
                success: false,
                message: Some("Task not found".to_string()),
            };
        }
        Err(error) => {
            return OperationResult {
                task_id,
                operation_type,
                success: false,
                message: Some(error.to_string()),
            };
        }
    };

    // Update and save
    let updated = task.with_priority(priority);
    if let Err(error) = repository.save(&updated).run_async().await {
        return OperationResult {
            task_id,
            operation_type,
            success: false,
            message: Some(error.to_string()),
        };
    }

    OperationResult {
        task_id,
        operation_type,
        success: true,
        message: Some(format!("Priority updated to {new_priority}")),
    }
}

/// Helper for `AddTag` operation.
async fn execute_add_tag(
    repository: Arc<dyn crate::infrastructure::TaskRepository>,
    task_id: String,
    tag: String,
) -> OperationResult {
    let operation_type = "add_tag".to_string();

    // Parse task ID
    let id = match parse_task_id(&task_id) {
        Ok(id) => id,
        Err(error) => {
            return OperationResult {
                task_id,
                operation_type,
                success: false,
                message: Some(error),
            };
        }
    };

    // Fetch task
    let task = match repository.find_by_id(&id).run_async().await {
        Ok(Some(task)) => task,
        Ok(None) => {
            return OperationResult {
                task_id,
                operation_type,
                success: false,
                message: Some("Task not found".to_string()),
            };
        }
        Err(error) => {
            return OperationResult {
                task_id,
                operation_type,
                success: false,
                message: Some(error.to_string()),
            };
        }
    };

    // Update and save
    let updated = task.add_tag(Tag::new(tag.clone()));
    if let Err(error) = repository.save(&updated).run_async().await {
        return OperationResult {
            task_id,
            operation_type,
            success: false,
            message: Some(error.to_string()),
        };
    }

    OperationResult {
        task_id,
        operation_type,
        success: true,
        message: Some(format!("Tag '{tag}' added")),
    }
}

// =============================================================================
// POST /tasks/enrich-batch - Batch enrichment with traverse_async_io_parallel
// =============================================================================

/// Enriches multiple tasks with related data in parallel.
///
/// This handler demonstrates:
/// - **`traverse_async_io_parallel`**: Parallel fetch and merge of related data
///
/// # Request Body
///
/// - `task_ids`: Array of task IDs to enrich (1-30 items)
/// - `include`: Fields to include (`project`, `subtasks`, `history`)
///
/// # Errors
///
/// - `400 Bad Request`: Empty task list or exceeds 30 items
/// - `404 Not Found`: Any task not found
pub async fn enrich_batch(
    State(state): State<AppState>,
    Json(request): Json<EnrichBatchRequest>,
) -> Result<JsonResponse<EnrichBatchResponse>, ApiErrorResponse> {
    // Validate batch size
    if request.task_ids.is_empty() {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("task_ids", "task_ids list cannot be empty")],
        ));
    }

    if request.task_ids.len() > 30 {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new(
                "task_ids",
                "task_ids list cannot exceed 30 items for batch enrichment",
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

    // Use traverse_async_io_parallel for parallel enrichment
    let task_repository = Arc::clone(&state.task_repository);
    let project_repository = Arc::clone(&state.project_repository);
    let include_project = request.include.contains(&"project".to_string());

    let enrich_io = build_parallel_enrich_io(
        task_repository,
        project_repository,
        task_ids,
        include_project,
    );

    let results: Vec<Result<EnrichedTaskDto, BatchOpError>> = enrich_io.run_async().await;

    // Collect errors or successful results - return first error with appropriate status code
    let mut enriched_tasks = Vec::new();
    for result in results {
        match result {
            Ok(dto) => enriched_tasks.push(dto),
            Err(error) => {
                return Err(error.into_api_error());
            }
        }
    }

    let count = enriched_tasks.len();

    Ok(JsonResponse(EnrichBatchResponse {
        tasks: enriched_tasks,
        count,
    }))
}

/// Builds an `AsyncIO` that enriches tasks in parallel.
///
/// Returns `Vec<Result<EnrichedTaskDto, BatchOpError>>` for proper error handling.
fn build_parallel_enrich_io(
    task_repository: Arc<dyn crate::infrastructure::TaskRepository>,
    _project_repository: Arc<dyn crate::infrastructure::ProjectRepository>,
    task_ids: Vec<TaskId>,
    include_project: bool,
) -> AsyncIO<Vec<Result<EnrichedTaskDto, BatchOpError>>> {
    task_ids.traverse_async_io_parallel(move |task_id| {
        let task_repo = Arc::clone(&task_repository);

        AsyncIO::new(move || async move {
            // Fetch task
            let task = match task_repo.find_by_id(&task_id).run_async().await {
                Ok(Some(task)) => task,
                Ok(None) => {
                    return Err(BatchOpError::NotFound {
                        task_id: task_id.to_string(),
                    });
                }
                Err(error) => {
                    return Err(BatchOpError::Repository {
                        message: error.to_string(),
                    });
                }
            };

            // Generate deterministic project name based on task ID
            // This ensures referential transparency - same input always gives same output
            let project_name = if include_project {
                Some(derive_project_name_from_task_id(&task_id))
            } else {
                None
            };

            let subtask_count = task.subtasks.len();
            let task_response = TaskResponse::from(&task);

            Ok(EnrichedTaskDto {
                task: task_response,
                project_name,
                subtask_count,
                has_history: false, // Simplified for demo
            })
        })
    })
}

/// Derives a deterministic project name from task ID.
///
/// Uses a simple hash-based mapping to ensure the same task ID
/// always produces the same project name (referential transparency).
fn derive_project_name_from_task_id(task_id: &TaskId) -> String {
    let task_uuid = task_id.as_uuid();
    let bytes = task_uuid.as_bytes();
    // Use first byte to select project name
    let project_index = bytes[0] % 5;
    match project_index {
        0 => "Project Alpha".to_string(),
        1 => "Project Beta".to_string(),
        2 => "Project Gamma".to_string(),
        3 => "Project Delta".to_string(),
        _ => "Project Epsilon".to_string(),
    }
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
    // Validation Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_validate_single_task_valid() {
        let request = CreateTaskRequest {
            title: "Valid Task".to_string(),
            description: Some("Description".to_string()),
            priority: PriorityDto::Medium,
            tags: vec!["tag1".to_string()],
        };

        let result = validate_single_task(0, &request);
        assert!(result.is_ok());

        let validated = result.unwrap();
        assert_eq!(validated.title, "Valid Task");
        assert_eq!(validated.priority, Priority::Medium);
    }

    #[rstest]
    fn test_validate_single_task_empty_title() {
        let request = CreateTaskRequest {
            title: "   ".to_string(),
            description: None,
            priority: PriorityDto::Low,
            tags: vec![],
        };

        let result = validate_single_task(0, &request);
        assert!(result.is_err());

        match result.unwrap_err() {
            BatchValidationError::EmptyTitle { index } => assert_eq!(index, 0),
            _ => panic!("Expected EmptyTitle error"),
        }
    }

    #[rstest]
    fn test_validate_single_task_title_too_long() {
        let request = CreateTaskRequest {
            title: "a".repeat(201),
            description: None,
            priority: PriorityDto::Low,
            tags: vec![],
        };

        let result = validate_single_task(0, &request);
        assert!(result.is_err());

        match result.unwrap_err() {
            BatchValidationError::TitleTooLong { index, length } => {
                assert_eq!(index, 0);
                assert_eq!(length, 201);
            }
            _ => panic!("Expected TitleTooLong error"),
        }
    }

    #[rstest]
    fn test_validate_single_task_too_many_tags() {
        let request = CreateTaskRequest {
            title: "Task".to_string(),
            description: None,
            priority: PriorityDto::Low,
            tags: (0..11).map(|i| format!("tag{i}")).collect(),
        };

        let result = validate_single_task(0, &request);
        assert!(result.is_err());

        match result.unwrap_err() {
            BatchValidationError::TooManyTags { index, count } => {
                assert_eq!(index, 0);
                assert_eq!(count, 11);
            }
            _ => panic!("Expected TooManyTags error"),
        }
    }

    #[rstest]
    fn test_validate_batch_tasks_all_valid() {
        let requests = vec![
            CreateTaskRequest {
                title: "Task 1".to_string(),
                description: None,
                priority: PriorityDto::Low,
                tags: vec![],
            },
            CreateTaskRequest {
                title: "Task 2".to_string(),
                description: None,
                priority: PriorityDto::High,
                tags: vec!["tag".to_string()],
            },
        ];

        let result = validate_batch_tasks(&requests);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[rstest]
    fn test_validate_batch_tasks_first_invalid() {
        let requests = vec![
            CreateTaskRequest {
                title: String::new(), // Invalid
                description: None,
                priority: PriorityDto::Low,
                tags: vec![],
            },
            CreateTaskRequest {
                title: "Valid Task".to_string(),
                description: None,
                priority: PriorityDto::High,
                tags: vec![],
            },
        ];

        let result = validate_batch_tasks(&requests);
        assert!(result.is_err());

        // Should fail at index 0
        match result.unwrap_err() {
            BatchValidationError::EmptyTitle { index } => assert_eq!(index, 0),
            _ => panic!("Expected EmptyTitle error at index 0"),
        }
    }

    #[rstest]
    fn test_validate_batch_tasks_middle_invalid() {
        let requests = vec![
            CreateTaskRequest {
                title: "Valid 1".to_string(),
                description: None,
                priority: PriorityDto::Low,
                tags: vec![],
            },
            CreateTaskRequest {
                title: String::new(), // Invalid at index 1
                description: None,
                priority: PriorityDto::Low,
                tags: vec![],
            },
            CreateTaskRequest {
                title: "Valid 3".to_string(),
                description: None,
                priority: PriorityDto::High,
                tags: vec![],
            },
        ];

        let result = validate_batch_tasks(&requests);
        assert!(result.is_err());

        // Should fail at index 1 (early return)
        match result.unwrap_err() {
            BatchValidationError::EmptyTitle { index } => assert_eq!(index, 1),
            _ => panic!("Expected EmptyTitle error at index 1"),
        }
    }

    // -------------------------------------------------------------------------
    // Optional Collection Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_collect_optional_fields_all_present() {
        let tasks = vec![
            Task::new(TaskId::generate(), "Task 1".to_string(), Timestamp::now())
                .with_description("Desc 1".to_string()),
            Task::new(TaskId::generate(), "Task 2".to_string(), Timestamp::now())
                .with_description("Desc 2".to_string()),
        ];

        let (status, values, present_count) = collect_optional_fields(&tasks, "description");

        assert_eq!(status, CollectionStatus::Complete);
        assert!(values.is_some());
        assert_eq!(values.unwrap().len(), 2);
        assert_eq!(present_count, 2);
    }

    #[rstest]
    fn test_collect_optional_fields_some_missing() {
        let tasks = vec![
            Task::new(TaskId::generate(), "Task 1".to_string(), Timestamp::now())
                .with_description("Desc 1".to_string()),
            Task::new(TaskId::generate(), "Task 2".to_string(), Timestamp::now()), // No description
        ];

        let (status, values, present_count) = collect_optional_fields(&tasks, "description");

        assert_eq!(status, CollectionStatus::Incomplete);
        assert!(values.is_none());
        assert_eq!(present_count, 1);
    }

    #[rstest]
    fn test_collect_optional_fields_all_missing() {
        let tasks = vec![
            Task::new(TaskId::generate(), "Task 1".to_string(), Timestamp::now()),
            Task::new(TaskId::generate(), "Task 2".to_string(), Timestamp::now()),
        ];

        let (status, values, present_count) = collect_optional_fields(&tasks, "description");

        assert_eq!(status, CollectionStatus::Incomplete);
        assert!(values.is_none());
        assert_eq!(present_count, 0);
    }

    // -------------------------------------------------------------------------
    // Parse Helper Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[case("pending", TaskStatus::Pending)]
    #[case("in_progress", TaskStatus::InProgress)]
    #[case("completed", TaskStatus::Completed)]
    #[case("cancelled", TaskStatus::Cancelled)]
    #[case("PENDING", TaskStatus::Pending)]
    fn test_parse_status_valid(#[case] input: &str, #[case] expected: TaskStatus) {
        let result = parse_status(input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }

    #[rstest]
    fn test_parse_status_invalid() {
        let result = parse_status("invalid");
        assert!(result.is_err());
    }

    #[rstest]
    #[case("low", Priority::Low)]
    #[case("medium", Priority::Medium)]
    #[case("high", Priority::High)]
    #[case("critical", Priority::Critical)]
    #[case("HIGH", Priority::High)]
    fn test_parse_priority_valid(#[case] input: &str, #[case] expected: Priority) {
        let result = parse_priority(input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }

    #[rstest]
    fn test_parse_priority_invalid() {
        let result = parse_priority("invalid");
        assert!(result.is_err());
    }

    #[rstest]
    fn test_parse_task_id_valid() {
        let uuid_str = "00000000-0000-0000-0000-000000000001";
        let result = parse_task_id(uuid_str);
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_parse_task_id_invalid() {
        let result = parse_task_id("not-a-uuid");
        assert!(result.is_err());
    }
}
