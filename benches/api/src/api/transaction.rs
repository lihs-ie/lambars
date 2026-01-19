//! Task transaction handlers using Optics and functional patterns.
//!
//! This module demonstrates lambars' optics and functional patterns:
//! - **`Lens`**: Focusing on struct fields for immutable updates
//! - **`Either`**: Representing success/failure without exceptions
//! - **`PersistentList`**: Functional list for subtasks
//! - **Pattern matching**: Exhaustive case analysis for state transitions
//!
//! # Endpoints
//!
//! - `PUT /tasks/{id}` - Update task fields using `Lens`
//! - `PATCH /tasks/{id}/status` - Transition status using `Either` + pattern matching
//! - `POST /tasks/{id}/subtasks` - Add subtask using `PersistentList`
//! - `POST /tasks/{id}/tags` - Add tag using `PersistentHashSet`

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use lambars::control::Either;
use lambars::lens;
use lambars::optics::Lens;
use uuid::Uuid;

use super::dto::{
    TaskResponse, TaskStatusDto, UpdateTaskRequest, validate_description, validate_tags,
    validate_title,
};
use super::error::{ApiErrorResponse, ValidationError};
use super::handlers::AppState;
use crate::domain::{SubTask, SubTaskId, Tag, Task, TaskId, TaskStatus, Timestamp};

// =============================================================================
// Path Extractors
// =============================================================================

/// Path parameter for task ID.
#[derive(Debug, serde::Deserialize)]
pub struct TaskPath {
    /// The task ID.
    pub id: Uuid,
}

// =============================================================================
// Request DTOs
// =============================================================================

/// Request DTO for updating task status.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct UpdateStatusRequest {
    /// New status for the task.
    pub status: TaskStatusDto,
    /// Expected version for optimistic locking.
    pub version: u64,
}

/// Request DTO for adding a subtask.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AddSubtaskRequest {
    /// Title of the subtask.
    pub title: String,
    /// Expected version for optimistic locking.
    pub version: u64,
}

/// Request DTO for adding a tag.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AddTagRequest {
    /// Tag to add.
    pub tag: String,
    /// Expected version for optimistic locking.
    pub version: u64,
}

// =============================================================================
// Domain Error Types
// =============================================================================

/// Error type for invalid status transitions.
///
/// Using a domain-specific enum provides type safety and makes error handling
/// more explicit than raw strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusTransitionError {
    /// Attempted transition from one status to another that is not allowed.
    InvalidTransition {
        /// The current status of the task.
        from: TaskStatus,
        /// The requested new status.
        to: TaskStatus,
    },
}

impl std::fmt::Display for StatusTransitionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(formatter, "Cannot transition from {from:?} to {to:?}")
            }
        }
    }
}

impl From<StatusTransitionError> for ApiErrorResponse {
    fn from(error: StatusTransitionError) -> Self {
        Self::bad_request("INVALID_TRANSITION", error.to_string())
    }
}

/// Result of a status transition validation.
///
/// - `NoChange`: Same status, no update needed (no-op)
/// - `Transition(TaskStatus)`: Valid transition to new status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusTransitionResult {
    /// No change needed (same status).
    NoChange,
    /// Valid transition to new status.
    Transition(TaskStatus),
}

// =============================================================================
// PUT /tasks/{id} - Update Task with Lens
// =============================================================================

/// Updates a task's fields using `Lens` optics for immutable field access.
///
/// This handler demonstrates:
/// - **`lens!` macro**: Creating lenses for struct fields
/// - **`Lens::set`**: Immutable field updates
/// - **`Bifunctor::bimap`**: Mapping over success/error in `Either`
///
/// # Path Parameters
///
/// - `id`: Task UUID
///
/// # Request Body
///
/// ```json
/// {
///   "title": "New title",
///   "description": "New description",
///   "priority": "high",
///   "version": 1
/// }
/// ```
///
/// # Response
///
/// - **200 OK**: Task updated successfully
/// - **400 Bad Request**: Validation error
/// - **404 Not Found**: Task not found
/// - **409 Conflict**: Version conflict
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] in the following cases:
/// - **400 Bad Request**: Invalid title, description, or priority
/// - **404 Not Found**: Task with given ID does not exist
/// - **409 Conflict**: Version mismatch (optimistic locking failure)
/// - **500 Internal Server Error**: Repository operation failed
#[allow(clippy::future_not_send)]
pub async fn update_task(
    State(state): State<AppState>,
    Path(path): Path<TaskPath>,
    Json(request): Json<UpdateTaskRequest>,
) -> Result<(StatusCode, Json<TaskResponse>), ApiErrorResponse> {
    let task_id = TaskId::from_uuid(path.id);

    // Fetch existing task
    let task = state
        .task_repository
        .find_by_id(&task_id)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?
        .ok_or_else(|| ApiErrorResponse::not_found(format!("Task {task_id} not found")))?;

    // Check version (optimistic locking)
    if task.version != request.version {
        let expected = request.version;
        let found = task.version;
        return Err(ApiErrorResponse::conflict(format!(
            "Expected version {expected}, found {found}"
        )));
    }

    // Get current timestamp (side effect captured at handler level)
    let now = Timestamp::now();

    // Validate and apply updates using Lens (pure function)
    let updated_task = apply_updates_with_lens(task, &request, now)?;

    // Build response before save (save returns ())
    let response = TaskResponse::from(&updated_task);

    // Save updated task
    state
        .task_repository
        .save(&updated_task)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    Ok((StatusCode::OK, Json(response)))
}

/// Applies updates to a task using Lens optics.
///
/// Uses `Bifunctor::map_left` to transform `Either<ValidationError, T>` into
/// `Either<ApiErrorResponse, T>` for consistent error handling.
///
/// # Arguments
///
/// * `task` - The task to update
/// * `request` - The update request
/// * `now` - The current timestamp (passed in for referential transparency)
///
/// # Purity Note
///
/// This function is pure - the timestamp is passed as an argument rather than
/// obtained inside the function, preserving referential transparency.
fn apply_updates_with_lens(
    task: Task,
    request: &UpdateTaskRequest,
    now: Timestamp,
) -> Result<Task, ApiErrorResponse> {
    // Define lenses for Task fields
    let title_lens = lens!(Task, title);
    let description_lens = lens!(Task, description);
    let priority_lens = lens!(Task, priority);

    // Start with the current task
    let mut updated = task;

    // Apply title update if present, using Bifunctor::map_left to transform errors
    if let Some(ref new_title) = request.title {
        let validation_result: Either<ValidationError, String> = validate_title(new_title);

        // Use Bifunctor::map_left to transform error type only
        let mapped: Either<ApiErrorResponse, String> =
            validation_result.map_left(ApiErrorResponse::from);

        // Convert Either to Result and propagate error with ?
        let title: String = Result::<String, ApiErrorResponse>::from(mapped)?;
        updated = title_lens.set(updated, title);
    }

    // Apply description update if present
    if let Some(ref desc) = request.description {
        let validation_result = validate_description(Some(desc));
        let mapped: Either<ApiErrorResponse, Option<String>> =
            validation_result.map_left(ApiErrorResponse::from);
        let description: Option<String> = Result::from(mapped)?;
        updated = description_lens.set(updated, description);
    }

    // Apply priority update if present
    if let Some(priority) = request.priority {
        updated = priority_lens.set(updated, priority.into());
    }

    // Update timestamp and version
    let updated_at_lens = lens!(Task, updated_at);
    let version_lens = lens!(Task, version);

    let updated = updated_at_lens.set(updated, now);
    let new_version = updated.version + 1;
    let updated = version_lens.set(updated, new_version);

    Ok(updated)
}

// =============================================================================
// PATCH /tasks/{id}/status - Status Transition with Either
// =============================================================================

/// Transitions a task's status using pattern matching on valid transitions.
///
/// This handler demonstrates:
/// - **Pattern matching**: Exhaustive case analysis for state transitions
/// - **`Either`**: Representing success/failure without exceptions
/// - **`Lens`**: Immutable field updates for status, timestamp, and version
///
/// # Path Parameters
///
/// - `id`: Task UUID
///
/// # Request Body
///
/// ```json
/// {
///   "status": "in_progress",
///   "version": 1
/// }
/// ```
///
/// # Response
///
/// - **200 OK**: Status updated successfully
/// - **400 Bad Request**: Invalid status transition
/// - **404 Not Found**: Task not found
/// - **409 Conflict**: Version conflict
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] in the following cases:
/// - **400 Bad Request**: Invalid status transition (e.g., Pending → Completed)
/// - **404 Not Found**: Task with given ID does not exist
/// - **409 Conflict**: Version mismatch (optimistic locking failure)
/// - **500 Internal Server Error**: Repository operation failed
#[allow(clippy::future_not_send)]
pub async fn update_status(
    State(state): State<AppState>,
    Path(path): Path<TaskPath>,
    Json(request): Json<UpdateStatusRequest>,
) -> Result<(StatusCode, Json<TaskResponse>), ApiErrorResponse> {
    let task_id = TaskId::from_uuid(path.id);

    // Fetch existing task
    let task = state
        .task_repository
        .find_by_id(&task_id)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?
        .ok_or_else(|| ApiErrorResponse::not_found(format!("Task {task_id} not found")))?;

    // Check version
    if task.version != request.version {
        let expected = request.version;
        let found = task.version;
        return Err(ApiErrorResponse::conflict(format!(
            "Expected version {expected}, found {found}"
        )));
    }

    // Capture timestamp at handler level for referential transparency
    let now = Timestamp::now();

    // Validate and apply status transition (pure function)
    let new_status = TaskStatus::from(request.status);
    let transition_result = validate_status_transition(task.status, new_status);

    // Handle transition result
    match transition_result {
        Either::Right(StatusTransitionResult::NoChange) => {
            // Same status - no update needed (no-op), return current task
            Ok((StatusCode::OK, Json(TaskResponse::from(&task))))
        }
        Either::Right(StatusTransitionResult::Transition(valid_status)) => {
            // Apply status update using pure function
            let updated_task = apply_status_update(task, valid_status, now);

            // Build response before save (save returns ())
            let response = TaskResponse::from(&updated_task);

            // Save updated task
            state
                .task_repository
                .save(&updated_task)
                .run_async()
                .await
                .map_err(ApiErrorResponse::from)?;

            Ok((StatusCode::OK, Json(response)))
        }
        Either::Left(error) => Err(ApiErrorResponse::from(error)),
    }
}

/// Validates a status transition using `Either` for functional error handling.
///
/// Returns:
/// - `Either::Right(StatusTransitionResult::NoChange)` if same status (no-op)
/// - `Either::Right(StatusTransitionResult::Transition(status))` if valid transition
/// - `Either::Left(StatusTransitionError)` if invalid transition
fn validate_status_transition(
    current: TaskStatus,
    new_status: TaskStatus,
) -> Either<StatusTransitionError, StatusTransitionResult> {
    // Define valid transitions using pattern matching
    match (current, new_status) {
        // Same status is a no-op (no update needed)
        (current, new) if current == new => Either::Right(StatusTransitionResult::NoChange),

        // Transition to InProgress (only from Pending)
        (TaskStatus::Pending, TaskStatus::InProgress) => {
            Either::Right(StatusTransitionResult::Transition(TaskStatus::InProgress))
        }

        // Transition to Completed (only from InProgress)
        (TaskStatus::InProgress, TaskStatus::Completed) => {
            Either::Right(StatusTransitionResult::Transition(TaskStatus::Completed))
        }

        // Transition to Pending (from InProgress or Completed - reopen)
        (TaskStatus::InProgress | TaskStatus::Completed, TaskStatus::Pending) => {
            Either::Right(StatusTransitionResult::Transition(TaskStatus::Pending))
        }

        // Transition to Cancelled (from Pending or InProgress)
        (TaskStatus::Pending | TaskStatus::InProgress, TaskStatus::Cancelled) => {
            Either::Right(StatusTransitionResult::Transition(TaskStatus::Cancelled))
        }

        // Invalid transitions
        (from, to) => Either::Left(StatusTransitionError::InvalidTransition { from, to }),
    }
}

/// Applies a status update to a task (pure function).
///
/// # Arguments
///
/// * `task` - The task to update
/// * `new_status` - The new status to apply
/// * `now` - The current timestamp (passed in for referential transparency)
fn apply_status_update(task: Task, new_status: TaskStatus, now: Timestamp) -> Task {
    let status_lens = lens!(Task, status);
    let updated_at_lens = lens!(Task, updated_at);
    let version_lens = lens!(Task, version);

    let updated = status_lens.set(task, new_status);
    let updated = updated_at_lens.set(updated, now);
    let new_version = updated.version + 1;
    version_lens.set(updated, new_version)
}

// =============================================================================
// POST /tasks/{id}/subtasks - Add Subtask with PersistentList
// =============================================================================

/// Adds a subtask using `PersistentList` prepend operation.
///
/// This handler demonstrates:
/// - **`PersistentList::prepend`**: Adding to the front of an immutable list
/// - **Functional list operations**: No mutation of existing data
/// - **Optimistic locking**: Version check to prevent concurrent update conflicts
///
/// # Path Parameters
///
/// - `id`: Task UUID
///
/// # Request Body
///
/// ```json
/// {
///   "title": "Subtask title",
///   "version": 1
/// }
/// ```
///
/// # Response
///
/// - **201 Created**: Subtask added successfully
/// - **400 Bad Request**: Validation error
/// - **404 Not Found**: Task not found
/// - **409 Conflict**: Version conflict
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] in the following cases:
/// - **400 Bad Request**: Invalid subtask title (empty or too long)
/// - **404 Not Found**: Task with given ID does not exist
/// - **409 Conflict**: Version mismatch (optimistic locking failure)
/// - **500 Internal Server Error**: Repository operation failed
#[allow(clippy::future_not_send)]
pub async fn add_subtask(
    State(state): State<AppState>,
    Path(path): Path<TaskPath>,
    Json(request): Json<AddSubtaskRequest>,
) -> Result<(StatusCode, Json<TaskResponse>), ApiErrorResponse> {
    let task_id = TaskId::from_uuid(path.id);

    // Validate subtask title
    let validation_result = validate_title(&request.title);
    let mapped = validation_result.map_left(ApiErrorResponse::from);
    let title: String = Result::from(mapped)?;

    // Fetch existing task
    let task = state
        .task_repository
        .find_by_id(&task_id)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?
        .ok_or_else(|| ApiErrorResponse::not_found(format!("Task {task_id} not found")))?;

    // Check version (optimistic locking)
    if task.version != request.version {
        let expected = request.version;
        let found = task.version;
        return Err(ApiErrorResponse::conflict(format!(
            "Expected version {expected}, found {found}"
        )));
    }

    // Capture timestamp at handler level for referential transparency
    let now = Timestamp::now();

    // Create new subtask (impure: generates ID)
    let subtask = SubTask::new(SubTaskId::generate(), title);

    // Apply updates using pure function
    let updated_task = apply_subtask_update(task, subtask, now);

    // Build response before save (save returns ())
    let response = TaskResponse::from(&updated_task);

    // Save updated task
    state
        .task_repository
        .save(&updated_task)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    Ok((StatusCode::CREATED, Json(response)))
}

/// Applies a subtask addition to a task (pure function).
///
/// # Arguments
///
/// * `task` - The task to update
/// * `subtask` - The subtask to add
/// * `now` - The current timestamp (passed in for referential transparency)
fn apply_subtask_update(task: Task, subtask: SubTask, now: Timestamp) -> Task {
    let updated_at_lens = lens!(Task, updated_at);
    let version_lens = lens!(Task, version);

    // Use PersistentList prepend (Task::prepend_subtask uses this internally)
    let updated = task.prepend_subtask(subtask);
    let updated = updated_at_lens.set(updated, now);
    let new_version = updated.version + 1;
    version_lens.set(updated, new_version)
}

// =============================================================================
// POST /tasks/{id}/tags - Add Tag with Monoid
// =============================================================================

/// Adds a tag using set union (Monoid combine operation).
///
/// This handler demonstrates:
/// - **`PersistentHashSet`**: Immutable set with structural sharing
/// - **Set union**: Adding elements without duplicates
/// - **Optimistic locking**: Version check to prevent concurrent update conflicts
///
/// # Path Parameters
///
/// - `id`: Task UUID
///
/// # Request Body
///
/// ```json
/// {
///   "tag": "backend",
///   "version": 1
/// }
/// ```
///
/// # Response
///
/// - **200 OK**: Tag added (or already exists)
/// - **400 Bad Request**: Validation error
/// - **404 Not Found**: Task not found
/// - **409 Conflict**: Version conflict
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] in the following cases:
/// - **400 Bad Request**: Invalid tag (empty or too long)
/// - **404 Not Found**: Task with given ID does not exist
/// - **409 Conflict**: Version mismatch (optimistic locking failure)
/// - **500 Internal Server Error**: Repository operation failed
#[allow(clippy::future_not_send)]
pub async fn add_tag(
    State(state): State<AppState>,
    Path(path): Path<TaskPath>,
    Json(request): Json<AddTagRequest>,
) -> Result<(StatusCode, Json<TaskResponse>), ApiErrorResponse> {
    let task_id = TaskId::from_uuid(path.id);

    // Validate tag
    let validation_result = validate_tags(std::slice::from_ref(&request.tag));
    let mapped = validation_result.map_left(ApiErrorResponse::from);
    let tags: Vec<Tag> = Result::from(mapped)?;

    let tag = tags.into_iter().next().ok_or_else(|| {
        ApiErrorResponse::bad_request("INVALID_TAG", "Tag validation produced no result")
    })?;

    // Fetch existing task
    let task = state
        .task_repository
        .find_by_id(&task_id)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?
        .ok_or_else(|| ApiErrorResponse::not_found(format!("Task {task_id} not found")))?;

    // Check version (optimistic locking)
    if task.version != request.version {
        let expected = request.version;
        let found = task.version;
        return Err(ApiErrorResponse::conflict(format!(
            "Expected version {expected}, found {found}"
        )));
    }

    // Capture timestamp at handler level for referential transparency
    let now = Timestamp::now();

    // Apply updates using pure function
    let updated_task = apply_tag_update(task, tag, now);

    // Build response before save (save returns ())
    let response = TaskResponse::from(&updated_task);

    // Save updated task
    state
        .task_repository
        .save(&updated_task)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    Ok((StatusCode::OK, Json(response)))
}

/// Applies a tag addition to a task (pure function).
///
/// # Arguments
///
/// * `task` - The task to update
/// * `tag` - The tag to add
/// * `now` - The current timestamp (passed in for referential transparency)
fn apply_tag_update(task: Task, tag: Tag, now: Timestamp) -> Task {
    let updated_at_lens = lens!(Task, updated_at);
    let version_lens = lens!(Task, version);

    // Add tag using Task::add_tag (uses PersistentHashSet internally)
    let updated = task.add_tag(tag);
    let updated = updated_at_lens.set(updated, now);
    let new_version = updated.version + 1;
    version_lens.set(updated, new_version)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // Status Transition Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_valid_transition_pending_to_in_progress() {
        let result = validate_status_transition(TaskStatus::Pending, TaskStatus::InProgress);
        assert!(result.is_right());
        assert_eq!(
            result.unwrap_right(),
            StatusTransitionResult::Transition(TaskStatus::InProgress)
        );
    }

    #[rstest]
    fn test_valid_transition_in_progress_to_completed() {
        let result = validate_status_transition(TaskStatus::InProgress, TaskStatus::Completed);
        assert!(result.is_right());
        assert_eq!(
            result.unwrap_right(),
            StatusTransitionResult::Transition(TaskStatus::Completed)
        );
    }

    #[rstest]
    fn test_valid_transition_same_status_returns_no_change() {
        let result = validate_status_transition(TaskStatus::Pending, TaskStatus::Pending);
        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), StatusTransitionResult::NoChange);
    }

    #[rstest]
    fn test_invalid_transition_pending_to_completed() {
        let result = validate_status_transition(TaskStatus::Pending, TaskStatus::Completed);
        assert!(result.is_left());
        assert_eq!(
            result.unwrap_left(),
            StatusTransitionError::InvalidTransition {
                from: TaskStatus::Pending,
                to: TaskStatus::Completed,
            }
        );
    }

    #[rstest]
    fn test_invalid_transition_cancelled_to_in_progress() {
        let result = validate_status_transition(TaskStatus::Cancelled, TaskStatus::InProgress);
        assert!(result.is_left());
        assert_eq!(
            result.unwrap_left(),
            StatusTransitionError::InvalidTransition {
                from: TaskStatus::Cancelled,
                to: TaskStatus::InProgress,
            }
        );
    }

    #[rstest]
    fn test_status_transition_error_display() {
        let error = StatusTransitionError::InvalidTransition {
            from: TaskStatus::Pending,
            to: TaskStatus::Completed,
        };
        assert_eq!(
            error.to_string(),
            "Cannot transition from Pending to Completed"
        );
    }

    // -------------------------------------------------------------------------
    // Lens Update Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_lens_update_title() {
        let title_lens = lens!(Task, title);
        let task = Task::new(TaskId::generate(), "Original", Timestamp::now());

        let updated = title_lens.set(task, "Updated".to_string());

        assert_eq!(updated.title, "Updated");
    }

    #[rstest]
    fn test_lens_update_description() {
        let desc_lens = lens!(Task, description);
        let task = Task::new(TaskId::generate(), "Task", Timestamp::now());

        let updated = desc_lens.set(task, Some("New description".to_string()));

        assert_eq!(updated.description, Some("New description".to_string()));
    }

    #[rstest]
    fn test_lens_modify() {
        let title_lens = lens!(Task, title);
        let task = Task::new(TaskId::generate(), "hello", Timestamp::now());

        let updated = title_lens.modify(task, |t| t.to_uppercase());

        assert_eq!(updated.title, "HELLO");
    }
}
