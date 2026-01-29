//! Task transaction handlers using Optics and functional patterns.
//!
//! This module demonstrates lambars' optics and functional patterns:
//! - **`Lens`**: Focusing on struct fields for immutable updates
//! - **`Optional`**: Safely handling `Option<T>` fields (Lens + Prism composition)
//! - **`Either`**: Representing success/failure without exceptions
//! - **`PersistentList`**: Functional list for subtasks
//! - **Pattern matching**: Exhaustive case analysis for state transitions
//!
//! # Endpoints
//!
//! - `PUT /tasks/{id}` - Update task fields using `Lens` and `Optional` optics
//! - `PATCH /tasks/{id}/status` - Transition status using `Either` + pattern matching
//! - `POST /tasks/{id}/subtasks` - Add subtask using `PersistentList`
//! - `POST /tasks/{id}/tags` - Add tag using `PersistentHashSet`

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use super::json_buffer::JsonResponse;
use lambars::control::Either;
use lambars::optics::{Lens, LensComposeExtension, Optional, Prism};
use lambars::{lens, prism};
use uuid::Uuid;

use super::consistency::{save_task_with_event, save_task_with_events};
use super::dto::{
    TaskResponse, TaskStatusDto, UpdateTaskRequest, validate_description, validate_tags,
    validate_title,
};
use super::error::{ApiErrorResponse, ValidationError};
use super::handlers::AppState;
use super::query::TaskChange;
use crate::domain::{
    EventId, Priority, SubTask, SubTaskId, Tag, Task, TaskEvent, TaskId, TaskStatus, Timestamp,
    create_description_updated_event, create_priority_changed_event, create_status_changed_event,
    create_tag_added_event, create_title_updated_event,
};

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
// Status Transition Prism
// =============================================================================

/// Represents a valid status transition as a tuple of (from, to) statuses.
///
/// This type is used as the target of the transition prism. When the prism
/// successfully previews a transition, it means the transition is valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidTransition {
    /// The source status.
    pub from: TaskStatus,
    /// The target status.
    pub to: TaskStatus,
}

impl ValidTransition {
    /// Creates a new valid transition.
    #[must_use]
    pub const fn new(from: TaskStatus, to: TaskStatus) -> Self {
        Self { from, to }
    }
}

/// Represents an attempt to transition between two statuses.
///
/// This enum serves as the Source type for the `ValidTransitionPrism`.
/// It encapsulates all possible outcomes of a status transition attempt:
/// - `Valid`: A valid transition that can proceed
/// - `Invalid`: An invalid transition that should be rejected
/// - `NoChange`: Same status, a no-op
///
/// # Prism Law Compliance
///
/// By using this enum as the Source type (instead of a raw tuple), the Prism
/// can satisfy the law `preview(&review(value)) == Some(&value)` because
/// `review` constructs a `StatusTransitionAttempt::Valid(ValidTransition)`
/// and `preview` can return a reference to the inner `ValidTransition`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusTransitionAttempt {
    /// A valid transition that can proceed.
    Valid(ValidTransition),
    /// An invalid transition that should be rejected.
    Invalid {
        /// The current status.
        from: TaskStatus,
        /// The requested target status.
        to: TaskStatus,
    },
    /// Same status, no change needed (no-op).
    NoChange(TaskStatus),
}

impl StatusTransitionAttempt {
    /// Creates a new `StatusTransitionAttempt` from a (from, to) status pair.
    ///
    /// This function classifies the transition attempt:
    /// - Same status -> `NoChange`
    /// - Valid transition -> `Valid`
    /// - Invalid transition -> `Invalid`
    #[must_use]
    pub const fn from_statuses(from: TaskStatus, to: TaskStatus) -> Self {
        if from as u8 == to as u8 {
            Self::NoChange(from)
        } else if is_valid_transition(from, to) {
            Self::Valid(ValidTransition::new(from, to))
        } else {
            Self::Invalid { from, to }
        }
    }

    /// Returns `true` if this is a valid transition.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        matches!(self, Self::Valid(_))
    }

    /// Returns `true` if this is an invalid transition.
    #[must_use]
    pub const fn is_invalid(&self) -> bool {
        matches!(self, Self::Invalid { .. })
    }

    /// Returns `true` if this is a no-change operation.
    #[must_use]
    pub const fn is_no_change(&self) -> bool {
        matches!(self, Self::NoChange(_))
    }
}

/// Checks if the given transition is valid.
///
/// Valid transitions are:
/// - `Pending` -> `InProgress`
/// - `InProgress` -> `Completed`
/// - `InProgress` -> `Pending` (reopen)
/// - `Completed` -> `Pending` (reopen)
/// - `Pending` -> `Cancelled`
/// - `InProgress` -> `Cancelled`
///
/// Same status transitions (no-op) return `false` and should be handled separately.
#[must_use]
pub const fn is_valid_transition(from: TaskStatus, to: TaskStatus) -> bool {
    matches!(
        (from, to),
        (
            TaskStatus::Pending,
            TaskStatus::InProgress | TaskStatus::Cancelled
        ) | (
            TaskStatus::InProgress,
            TaskStatus::Completed | TaskStatus::Pending | TaskStatus::Cancelled
        ) | (TaskStatus::Completed, TaskStatus::Pending)
    )
}

/// A Prism implementation for valid status transitions.
///
/// This prism focuses on the `Valid` variant of `StatusTransitionAttempt`,
/// extracting the inner `ValidTransition` when the transition is valid.
///
/// # Prism Laws
///
/// This implementation satisfies both Prism laws:
///
/// 1. **`PreviewReview` Law**: `preview(&review(value)) == Some(&value)`
///    - `review` creates `StatusTransitionAttempt::Valid(value)`
///    - `preview` extracts a reference to the inner `ValidTransition`
///
/// 2. **`ReviewPreview` Law**: If `preview(source).is_some()`, then
///    `review(preview(source).unwrap().clone()) == source`
///    - For `Valid(transition)`, preview returns `Some(&transition)`
///    - Reviewing that transition recreates the same `Valid(transition)`
///
/// # Example
///
/// ```ignore
/// let prism = valid_transition_prism();
/// let attempt = StatusTransitionAttempt::from_statuses(TaskStatus::Pending, TaskStatus::InProgress);
///
/// // Valid transition: preview succeeds
/// assert!(prism.preview(&attempt).is_some());
///
/// // Invalid transition: preview fails
/// let invalid = StatusTransitionAttempt::from_statuses(TaskStatus::Pending, TaskStatus::Completed);
/// assert!(prism.preview(&invalid).is_none());
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidTransitionPrism;

impl ValidTransitionPrism {
    /// Creates a new `ValidTransitionPrism`.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Prism<StatusTransitionAttempt, ValidTransition> for ValidTransitionPrism {
    fn preview<'a>(&self, source: &'a StatusTransitionAttempt) -> Option<&'a ValidTransition> {
        match source {
            StatusTransitionAttempt::Valid(transition) => Some(transition),
            StatusTransitionAttempt::Invalid { .. } | StatusTransitionAttempt::NoChange(_) => None,
        }
    }

    fn review(&self, value: ValidTransition) -> StatusTransitionAttempt {
        StatusTransitionAttempt::Valid(value)
    }

    fn preview_owned(&self, source: StatusTransitionAttempt) -> Option<ValidTransition> {
        match source {
            StatusTransitionAttempt::Valid(transition) => Some(transition),
            StatusTransitionAttempt::Invalid { .. } | StatusTransitionAttempt::NoChange(_) => None,
        }
    }
}

/// Creates a Prism that focuses on valid status transitions.
///
/// This prism succeeds (returns `Some`) only for valid transitions:
/// - `Pending` -> `InProgress`
/// - `InProgress` -> `Completed`
/// - `InProgress` -> `Pending` (reopen)
/// - `Completed` -> `Pending` (reopen)
/// - `Pending` -> `Cancelled`
/// - `InProgress` -> `Cancelled`
///
/// Invalid transitions (e.g., `Pending` -> `Completed`, `Cancelled` -> any)
/// will return `None` from both `preview` and `preview_owned`.
///
/// # Prism Laws
///
/// This prism fully satisfies both Prism laws because the Source type
/// (`StatusTransitionAttempt`) contains the Target type (`ValidTransition`)
/// as an enum variant.
///
/// # Example
///
/// ```ignore
/// let prism = valid_transition_prism();
/// let attempt = StatusTransitionAttempt::from_statuses(TaskStatus::Pending, TaskStatus::InProgress);
///
/// // Valid transition: preview succeeds
/// assert!(prism.preview(&attempt).is_some());
///
/// // Prism law: preview(review(value)) == Some(&value)
/// let value = ValidTransition::new(TaskStatus::Pending, TaskStatus::InProgress);
/// let reviewed = prism.review(value);
/// assert_eq!(prism.preview(&reviewed), Some(&value));
/// ```
#[must_use]
pub const fn valid_transition_prism() -> ValidTransitionPrism {
    ValidTransitionPrism::new()
}

/// Checks if the given transition is a no-change operation (same status).
#[must_use]
pub const fn is_no_change_transition(from: TaskStatus, to: TaskStatus) -> bool {
    matches!(
        (from, to),
        (TaskStatus::Pending, TaskStatus::Pending)
            | (TaskStatus::InProgress, TaskStatus::InProgress)
            | (TaskStatus::Completed, TaskStatus::Completed)
            | (TaskStatus::Cancelled, TaskStatus::Cancelled)
    )
}

// =============================================================================
// Optional Optic for Option<String> fields
// =============================================================================

/// Creates an Optional optic for safely accessing the description field.
///
/// This function creates an Optional that:
/// - Returns `Some(&String)` when `description` is `Some`
/// - Returns `None` when `description` is `None`
/// - Can modify the String inside `Some` without affecting the `None` case
///
/// This combines:
/// - `lens!(Task, description)`: Focuses on the `description: Option<String>` field
/// - `prism!(Option<String>, Some)`: Focuses on the `Some` variant, extracting `String`
///
/// # Example
///
/// ```ignore
/// let optional = description_optional();
/// let task = Task::new(...).with_description("Hello");
///
/// // Get the description if present
/// assert_eq!(optional.get_option(&task), Some(&"Hello".to_string()));
///
/// // Modify the description if present
/// let updated = optional.modify(task, |d| d.to_uppercase());
/// assert_eq!(updated.description, Some("HELLO".to_string()));
/// ```
#[must_use]
pub fn description_optional() -> impl Optional<Task, String> + Clone {
    let description_lens = lens!(Task, description);
    let some_prism = prism!(Option<String>, Some);
    description_lens.compose_prism(some_prism)
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
) -> Result<(StatusCode, JsonResponse<TaskResponse>), ApiErrorResponse> {
    let task_id = TaskId::from_uuid(path.id);

    // Fetch existing task
    let old_task = state
        .task_repository
        .find_by_id(&task_id)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?
        .ok_or_else(|| ApiErrorResponse::not_found(format!("Task {task_id} not found")))?;

    // Check version (optimistic locking)
    if old_task.version != request.version {
        let expected = request.version;
        let found = old_task.version;
        return Err(ApiErrorResponse::conflict(format!(
            "Expected version {expected}, found {found}"
        )));
    }

    // Get current timestamp (side effect captured at handler level)
    let now = Timestamp::now();

    // Get current event version from EventStore (I/O boundary)
    let current_event_version = state
        .event_store
        .get_current_version(&task_id)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    // Validate and apply updates using Lens (pure function)
    // Clone timestamp since it will be used for events later
    let updated_task = apply_updates_with_lens(old_task.clone(), &request, now.clone())?;

    // Detect changes (pure function)
    let changes = detect_task_changes(&old_task, &updated_task);

    // Build events from changes (I/O boundary: generates event IDs)
    let events = build_events_from_changes(&task_id, &changes, &now, current_event_version);

    // Save task and write events using best-effort consistency
    let write_result =
        save_task_with_events(&state, &updated_task, events, current_event_version).await?;

    // Build response with any consistency warnings
    // Note: logging is done in save_task_with_events, so we only add to response here
    let mut response = TaskResponse::from(&updated_task);
    if let Some(warning) = &write_result.warning {
        response.warnings.push(warning.client_message());
    }

    // Update search index with the task change (lock-free write)
    state.update_search_index(TaskChange::Update {
        old: old_task,
        new: updated_task,
    });

    Ok((StatusCode::OK, JsonResponse(response)))
}

/// Describes a detected change in a task (pure value).
///
/// This enum represents what changed, without event IDs or timestamps.
/// Event generation is done at the I/O boundary using this information.
#[derive(Debug, Clone)]
enum DetectedChange {
    /// Title was changed from old to new value.
    TitleUpdated {
        old_title: String,
        new_title: String,
    },
    /// Description was changed from old to new value.
    DescriptionUpdated {
        old_description: Option<String>,
        new_description: Option<String>,
    },
    /// Priority was changed from old to new value.
    PriorityChanged {
        old_priority: Priority,
        new_priority: Priority,
    },
}

/// Detects changes between old and new task states (pure function).
///
/// This function compares the two task states and returns a list of
/// detected changes. It does not generate event IDs or timestamps,
/// making it referentially transparent.
///
/// # Arguments
///
/// * `old_task` - The task before updates
/// * `new_task` - The task after updates
///
/// # Returns
///
/// A vector of detected changes, possibly empty if no changes were detected.
fn detect_task_changes(old_task: &Task, new_task: &Task) -> Vec<DetectedChange> {
    let mut changes = Vec::new();

    // Check title change
    if old_task.title != new_task.title {
        changes.push(DetectedChange::TitleUpdated {
            old_title: old_task.title.clone(),
            new_title: new_task.title.clone(),
        });
    }

    // Check description change
    if old_task.description != new_task.description {
        changes.push(DetectedChange::DescriptionUpdated {
            old_description: old_task.description.clone(),
            new_description: new_task.description.clone(),
        });
    }

    // Check priority change
    if old_task.priority != new_task.priority {
        changes.push(DetectedChange::PriorityChanged {
            old_priority: old_task.priority,
            new_priority: new_task.priority,
        });
    }

    changes
}

/// Builds events from detected changes (I/O boundary function).
///
/// This function generates event IDs and creates `TaskEvent` instances
/// from the detected changes. Event IDs are generated here because
/// they are non-deterministic (UUID generation).
///
/// # Arguments
///
/// * `task_id` - The task ID
/// * `changes` - Detected changes from `detect_task_changes`
/// * `timestamp` - Timestamp for all events (generated at I/O boundary)
/// * `current_version` - Current event version (for version calculation)
///
/// # Returns
///
/// A vector of events to write.
fn build_events_from_changes(
    task_id: &TaskId,
    changes: &[DetectedChange],
    timestamp: &Timestamp,
    current_version: u64,
) -> Vec<TaskEvent> {
    changes
        .iter()
        .enumerate()
        .map(|(index, change)| {
            let version = current_version + 1 + index as u64;
            let event_id = EventId::generate_v7(); // I/O: generate event ID

            match change {
                DetectedChange::TitleUpdated {
                    old_title,
                    new_title,
                } => create_title_updated_event(
                    task_id,
                    old_title,
                    new_title,
                    event_id,
                    timestamp.clone(),
                    version,
                ),
                DetectedChange::DescriptionUpdated {
                    old_description,
                    new_description,
                } => create_description_updated_event(
                    task_id,
                    old_description.as_deref(),
                    new_description.as_deref(),
                    event_id,
                    timestamp.clone(),
                    version,
                ),
                DetectedChange::PriorityChanged {
                    old_priority,
                    new_priority,
                } => create_priority_changed_event(
                    task_id,
                    *old_priority,
                    *new_priority,
                    event_id,
                    timestamp.clone(),
                    version,
                ),
            }
        })
        .collect()
}

/// Applies updates to a task using Lens and Optional optics.
///
/// Uses `Bifunctor::map_left` to transform `Either<ValidationError, T>` into
/// `Either<ApiErrorResponse, T>` for consistent error handling.
///
/// # Optics Used
///
/// - **`Lens`**: For required fields (title, priority, `updated_at`, version)
/// - **`Optional`**: For `Option<String>` fields (description) to safely handle
///   the case where the field may or may not exist
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
    let priority_lens = lens!(Task, priority);

    // Create Optional optic for description field (Option<String>)
    // This safely handles the case where description may or may not exist
    let description_optional = description_optional();

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

    // Apply description update if present using Optional optic and Lens
    // - When validated_description is Some: Use Optional::set to update the value
    // - When validated_description is None: Use Lens::set to clear the field explicitly
    if let Some(ref new_description) = request.description {
        let validation_result = validate_description(Some(new_description));
        let mapped: Either<ApiErrorResponse, Option<String>> =
            validation_result.map_left(ApiErrorResponse::from);
        let validated_description: Option<String> = Result::from(mapped)?;

        // Use pattern matching to handle both update and clear cases
        updated = match validated_description {
            Some(description) => description_optional.set(updated, description),
            None => lens!(Task, description).set(updated, None),
        };
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
) -> Result<(StatusCode, JsonResponse<TaskResponse>), ApiErrorResponse> {
    let task_id = TaskId::from_uuid(path.id);

    // Fetch existing task
    let old_task = state
        .task_repository
        .find_by_id(&task_id)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?
        .ok_or_else(|| ApiErrorResponse::not_found(format!("Task {task_id} not found")))?;

    // Check version
    if old_task.version != request.version {
        let expected = request.version;
        let found = old_task.version;
        return Err(ApiErrorResponse::conflict(format!(
            "Expected version {expected}, found {found}"
        )));
    }

    // Capture timestamp at handler level for referential transparency
    let now = Timestamp::now();

    // Validate and apply status transition (pure function)
    let new_status = TaskStatus::from(request.status);
    let transition_result = validate_status_transition(old_task.status, new_status);

    // Handle transition result
    match transition_result {
        Either::Right(StatusTransitionResult::NoChange) => {
            // Same status - no update needed (no-op), return current task
            Ok((StatusCode::OK, JsonResponse(TaskResponse::from(&old_task))))
        }
        Either::Right(StatusTransitionResult::Transition(valid_status)) => {
            // Get current event version from EventStore (I/O boundary)
            let current_event_version = state
                .event_store
                .get_current_version(&task_id)
                .run_async()
                .await
                .map_err(ApiErrorResponse::from)?;

            // Apply status update using pure function
            let updated_task = apply_status_update(old_task.clone(), valid_status, now.clone());

            // Create status changed event (pure function)
            // Version semantics: event.version = current_version + 1
            let event = create_status_changed_event(
                &task_id,
                old_task.status,
                valid_status,
                EventId::generate_v7(),    // I/O boundary: generate event ID
                now,                       // Use same timestamp as task update
                current_event_version + 1, // This event's version
            );

            // Save task and write event using best-effort consistency
            let save_result =
                save_task_with_event(&state, &updated_task, event, current_event_version).await?;

            // Build response with any consistency warnings
            // Note: logging is done in save_task_with_event, so we only add to response here
            let mut response = TaskResponse::from(&updated_task);
            if let Some(warning) = &save_result.consistency_warning {
                response.warnings.push(warning.client_message());
            }

            // Update search index with the task change (lock-free write)
            state.update_search_index(TaskChange::Update {
                old: old_task,
                new: updated_task,
            });

            Ok((StatusCode::OK, JsonResponse(response)))
        }
        Either::Left(error) => Err(ApiErrorResponse::from(error)),
    }
}

/// Validates a status transition using `Prism` and `Either` for functional error handling.
///
/// This function uses `valid_transition_prism()` to check if a transition is valid:
/// - First checks if it's a no-change operation (same status)
/// - Then uses `Prism::preview_owned` to verify the transition is allowed
/// - Returns `Either::Left` with error for invalid transitions
///
/// # Returns
///
/// - `Either::Right(StatusTransitionResult::NoChange)` if same status (no-op)
/// - `Either::Right(StatusTransitionResult::Transition(status))` if valid transition
/// - `Either::Left(StatusTransitionError)` if invalid transition
///
/// # Example
///
/// ```ignore
/// // Valid transition
/// let result = validate_status_transition(TaskStatus::Pending, TaskStatus::InProgress);
/// assert!(result.is_right());
///
/// // Invalid transition
/// let result = validate_status_transition(TaskStatus::Pending, TaskStatus::Completed);
/// assert!(result.is_left());
/// ```
fn validate_status_transition(
    current: TaskStatus,
    new_status: TaskStatus,
) -> Either<StatusTransitionError, StatusTransitionResult> {
    // Create a StatusTransitionAttempt from the status pair
    let attempt = StatusTransitionAttempt::from_statuses(current, new_status);

    // Use Prism to validate the transition
    let prism = valid_transition_prism();

    // Handle based on the attempt classification
    match attempt {
        StatusTransitionAttempt::NoChange(_) => Either::Right(StatusTransitionResult::NoChange),
        StatusTransitionAttempt::Valid(_) => {
            // Use preview_owned to extract the valid transition
            // This is guaranteed to succeed because we already know it's Valid
            let valid = prism
                .preview_owned(attempt)
                .expect("Valid attempt should preview successfully");
            Either::Right(StatusTransitionResult::Transition(valid.to))
        }
        StatusTransitionAttempt::Invalid { from, to } => {
            Either::Left(StatusTransitionError::InvalidTransition { from, to })
        }
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
) -> Result<(StatusCode, JsonResponse<TaskResponse>), ApiErrorResponse> {
    let task_id = TaskId::from_uuid(path.id);

    // Validate subtask title
    let validation_result = validate_title(&request.title);
    let mapped = validation_result.map_left(ApiErrorResponse::from);
    let title: String = Result::from(mapped)?;

    // Fetch existing task
    let old_task = state
        .task_repository
        .find_by_id(&task_id)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?
        .ok_or_else(|| ApiErrorResponse::not_found(format!("Task {task_id} not found")))?;

    // Check version (optimistic locking)
    if old_task.version != request.version {
        let expected = request.version;
        let found = old_task.version;
        return Err(ApiErrorResponse::conflict(format!(
            "Expected version {expected}, found {found}"
        )));
    }

    // Capture timestamp at handler level for referential transparency
    let now = Timestamp::now();

    // Create new subtask (impure: generates ID)
    let subtask = SubTask::new(SubTaskId::generate(), title);

    // Apply updates using pure function
    let updated_task = apply_subtask_update(old_task.clone(), subtask, now);

    // Build response before save (save returns ())
    let response = TaskResponse::from(&updated_task);

    // Save updated task
    state
        .task_repository
        .save(&updated_task)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    // Update search index with the task change (lock-free write)
    // Note: Subtasks don't affect search index directly since we only index title/tags,
    // but we update it to keep the tasks_by_id map synchronized.
    state.update_search_index(TaskChange::Update {
        old: old_task,
        new: updated_task,
    });

    Ok((StatusCode::CREATED, JsonResponse(response)))
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
) -> Result<(StatusCode, JsonResponse<TaskResponse>), ApiErrorResponse> {
    let task_id = TaskId::from_uuid(path.id);

    // Validate tag
    let validation_result = validate_tags(std::slice::from_ref(&request.tag));
    let mapped = validation_result.map_left(ApiErrorResponse::from);
    let tags: Vec<Tag> = Result::from(mapped)?;

    let tag = tags.into_iter().next().ok_or_else(|| {
        ApiErrorResponse::bad_request("INVALID_TAG", "Tag validation produced no result")
    })?;

    // Fetch existing task
    let old_task = state
        .task_repository
        .find_by_id(&task_id)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?
        .ok_or_else(|| ApiErrorResponse::not_found(format!("Task {task_id} not found")))?;

    // Check version (optimistic locking)
    if old_task.version != request.version {
        let expected = request.version;
        let found = old_task.version;
        return Err(ApiErrorResponse::conflict(format!(
            "Expected version {expected}, found {found}"
        )));
    }

    // Capture timestamp at handler level for referential transparency
    let now = Timestamp::now();

    // Get current event version from EventStore (I/O boundary)
    let current_event_version = state
        .event_store
        .get_current_version(&task_id)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    // Clone tag for event creation before it's moved to apply_tag_update
    let tag_for_event = tag.clone();

    // Apply updates using pure function
    let updated_task = apply_tag_update(old_task.clone(), tag, now.clone());

    // Create tag added event (pure function)
    // Version semantics: event.version = current_version + 1
    let event = create_tag_added_event(
        &task_id,
        &tag_for_event,
        EventId::generate_v7(),    // I/O boundary: generate event ID
        now,                       // Use same timestamp as task update
        current_event_version + 1, // This event's version
    );

    // Save task and write event using best-effort consistency
    let save_result =
        save_task_with_event(&state, &updated_task, event, current_event_version).await?;

    // Build response with any consistency warnings
    // Note: logging is done in save_task_with_event, so we only add to response here
    let mut response = TaskResponse::from(&updated_task);
    if let Some(warning) = &save_result.consistency_warning {
        response.warnings.push(warning.client_message());
    }

    // Update search index with the task change (lock-free write)
    // Tags are indexed, so this update is necessary for search consistency.
    state.update_search_index(TaskChange::Update {
        old: old_task,
        new: updated_task,
    });

    Ok((StatusCode::OK, JsonResponse(response)))
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

    /// Tests all invalid transitions using parametrized test.
    ///
    /// Invalid transitions include:
    /// - `Pending` -> `Completed` (must go through `InProgress` first)
    /// - `Completed` -> `InProgress` (can only reopen to `Pending`)
    /// - `Completed` -> `Cancelled` (completed tasks cannot be cancelled)
    /// - `Cancelled` -> any status (cancelled is a terminal state)
    #[rstest]
    #[case(TaskStatus::Pending, TaskStatus::Completed)]
    #[case(TaskStatus::Completed, TaskStatus::InProgress)]
    #[case(TaskStatus::Completed, TaskStatus::Cancelled)]
    #[case(TaskStatus::Cancelled, TaskStatus::Pending)]
    #[case(TaskStatus::Cancelled, TaskStatus::InProgress)]
    #[case(TaskStatus::Cancelled, TaskStatus::Completed)]
    fn test_validate_status_transition_all_invalid_transitions(
        #[case] from: TaskStatus,
        #[case] to: TaskStatus,
    ) {
        let result = validate_status_transition(from, to);

        assert!(
            result.is_left(),
            "Expected invalid transition from {from:?} to {to:?}, but got Right"
        );
        assert_eq!(
            result.unwrap_left(),
            StatusTransitionError::InvalidTransition { from, to },
            "Error should contain correct from/to statuses"
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
    // StatusTransitionAttempt Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_status_transition_attempt_from_statuses_valid() {
        let attempt =
            StatusTransitionAttempt::from_statuses(TaskStatus::Pending, TaskStatus::InProgress);

        assert!(attempt.is_valid());
        assert!(!attempt.is_invalid());
        assert!(!attempt.is_no_change());
        assert!(matches!(
            attempt,
            StatusTransitionAttempt::Valid(ValidTransition {
                from: TaskStatus::Pending,
                to: TaskStatus::InProgress
            })
        ));
    }

    #[rstest]
    fn test_status_transition_attempt_from_statuses_invalid() {
        let attempt =
            StatusTransitionAttempt::from_statuses(TaskStatus::Pending, TaskStatus::Completed);

        assert!(!attempt.is_valid());
        assert!(attempt.is_invalid());
        assert!(!attempt.is_no_change());
        assert!(matches!(
            attempt,
            StatusTransitionAttempt::Invalid {
                from: TaskStatus::Pending,
                to: TaskStatus::Completed
            }
        ));
    }

    #[rstest]
    fn test_status_transition_attempt_from_statuses_no_change() {
        let attempt =
            StatusTransitionAttempt::from_statuses(TaskStatus::Pending, TaskStatus::Pending);

        assert!(!attempt.is_valid());
        assert!(!attempt.is_invalid());
        assert!(attempt.is_no_change());
        assert!(matches!(
            attempt,
            StatusTransitionAttempt::NoChange(TaskStatus::Pending)
        ));
    }

    #[rstest]
    #[case(TaskStatus::Pending, TaskStatus::InProgress)]
    #[case(TaskStatus::InProgress, TaskStatus::Completed)]
    #[case(TaskStatus::InProgress, TaskStatus::Pending)]
    #[case(TaskStatus::Completed, TaskStatus::Pending)]
    #[case(TaskStatus::Pending, TaskStatus::Cancelled)]
    #[case(TaskStatus::InProgress, TaskStatus::Cancelled)]
    fn test_status_transition_attempt_all_valid_transitions(
        #[case] from: TaskStatus,
        #[case] to: TaskStatus,
    ) {
        let attempt = StatusTransitionAttempt::from_statuses(from, to);

        assert!(
            attempt.is_valid(),
            "Expected valid transition from {from:?} to {to:?}"
        );
    }

    #[rstest]
    #[case(TaskStatus::Pending, TaskStatus::Completed)]
    #[case(TaskStatus::Completed, TaskStatus::InProgress)]
    #[case(TaskStatus::Completed, TaskStatus::Cancelled)]
    #[case(TaskStatus::Cancelled, TaskStatus::Pending)]
    #[case(TaskStatus::Cancelled, TaskStatus::InProgress)]
    #[case(TaskStatus::Cancelled, TaskStatus::Completed)]
    fn test_status_transition_attempt_all_invalid_transitions(
        #[case] from: TaskStatus,
        #[case] to: TaskStatus,
    ) {
        let attempt = StatusTransitionAttempt::from_statuses(from, to);

        assert!(
            attempt.is_invalid(),
            "Expected invalid transition from {from:?} to {to:?}"
        );
    }

    #[rstest]
    #[case(TaskStatus::Pending)]
    #[case(TaskStatus::InProgress)]
    #[case(TaskStatus::Completed)]
    #[case(TaskStatus::Cancelled)]
    fn test_status_transition_attempt_all_no_change(#[case] status: TaskStatus) {
        let attempt = StatusTransitionAttempt::from_statuses(status, status);

        assert!(
            attempt.is_no_change(),
            "Expected no-change for same status {status:?}"
        );
    }

    // -------------------------------------------------------------------------
    // Prism-based Status Transition Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_valid_transition_prism_preview_valid() {
        let prism = valid_transition_prism();
        let attempt =
            StatusTransitionAttempt::from_statuses(TaskStatus::Pending, TaskStatus::InProgress);

        let result = prism.preview(&attempt);

        assert!(result.is_some());
        let valid = result.unwrap();
        assert_eq!(valid.from, TaskStatus::Pending);
        assert_eq!(valid.to, TaskStatus::InProgress);
    }

    #[rstest]
    fn test_valid_transition_prism_preview_invalid() {
        let prism = valid_transition_prism();
        let attempt =
            StatusTransitionAttempt::from_statuses(TaskStatus::Pending, TaskStatus::Completed);

        let result = prism.preview(&attempt);

        assert!(result.is_none());
    }

    #[rstest]
    fn test_valid_transition_prism_preview_no_change() {
        let prism = valid_transition_prism();
        let attempt =
            StatusTransitionAttempt::from_statuses(TaskStatus::Pending, TaskStatus::Pending);

        let result = prism.preview(&attempt);

        // NoChange is not a "valid transition" - it's handled separately
        assert!(result.is_none());
    }

    #[rstest]
    fn test_valid_transition_prism_preview_owned_valid() {
        let prism = valid_transition_prism();
        let attempt =
            StatusTransitionAttempt::from_statuses(TaskStatus::Pending, TaskStatus::InProgress);

        let result = prism.preview_owned(attempt);

        assert!(result.is_some());
        let valid = result.unwrap();
        assert_eq!(valid.from, TaskStatus::Pending);
        assert_eq!(valid.to, TaskStatus::InProgress);
    }

    #[rstest]
    fn test_valid_transition_prism_preview_owned_invalid() {
        let prism = valid_transition_prism();
        let attempt =
            StatusTransitionAttempt::from_statuses(TaskStatus::Pending, TaskStatus::Completed);

        let result = prism.preview_owned(attempt);

        assert!(result.is_none());
    }

    #[rstest]
    fn test_valid_transition_prism_preview_owned_no_change() {
        let prism = valid_transition_prism();
        let attempt =
            StatusTransitionAttempt::from_statuses(TaskStatus::Pending, TaskStatus::Pending);

        let result = prism.preview_owned(attempt);

        // NoChange is not a "valid transition" - it's handled separately
        assert!(result.is_none());
    }

    #[rstest]
    #[case(TaskStatus::Pending, TaskStatus::InProgress)]
    #[case(TaskStatus::InProgress, TaskStatus::Completed)]
    #[case(TaskStatus::InProgress, TaskStatus::Pending)]
    #[case(TaskStatus::Completed, TaskStatus::Pending)]
    #[case(TaskStatus::Pending, TaskStatus::Cancelled)]
    #[case(TaskStatus::InProgress, TaskStatus::Cancelled)]
    fn test_valid_transition_prism_all_valid_transitions(
        #[case] from: TaskStatus,
        #[case] to: TaskStatus,
    ) {
        let prism = valid_transition_prism();
        let attempt = StatusTransitionAttempt::from_statuses(from, to);

        let result = prism.preview(&attempt);

        assert!(
            result.is_some(),
            "Expected valid transition from {from:?} to {to:?}"
        );
        let valid = result.unwrap();
        assert_eq!(valid.from, from);
        assert_eq!(valid.to, to);
    }

    #[rstest]
    #[case(TaskStatus::Pending, TaskStatus::Completed)]
    #[case(TaskStatus::Completed, TaskStatus::InProgress)]
    #[case(TaskStatus::Completed, TaskStatus::Cancelled)]
    #[case(TaskStatus::Cancelled, TaskStatus::Pending)]
    #[case(TaskStatus::Cancelled, TaskStatus::InProgress)]
    #[case(TaskStatus::Cancelled, TaskStatus::Completed)]
    fn test_valid_transition_prism_all_invalid_transitions(
        #[case] from: TaskStatus,
        #[case] to: TaskStatus,
    ) {
        let prism = valid_transition_prism();
        let attempt = StatusTransitionAttempt::from_statuses(from, to);

        let result = prism.preview(&attempt);

        assert!(
            result.is_none(),
            "Expected invalid transition from {from:?} to {to:?}"
        );
    }

    #[rstest]
    fn test_valid_transition_prism_review_creates_valid_attempt() {
        let prism = valid_transition_prism();
        let valid = ValidTransition::new(TaskStatus::Pending, TaskStatus::InProgress);

        let reviewed = prism.review(valid);

        assert!(matches!(
            reviewed,
            StatusTransitionAttempt::Valid(ValidTransition {
                from: TaskStatus::Pending,
                to: TaskStatus::InProgress
            })
        ));
    }

    // -------------------------------------------------------------------------
    // Prism Law Tests for ValidTransitionPrism (using preview)
    // -------------------------------------------------------------------------

    /// Tests the `PreviewReview` law for the `ValidTransitionPrism`.
    ///
    /// Law: `preview(&review(value)) == Some(&value)`
    ///
    /// This verifies that reviewing a value and then previewing it
    /// yields a reference to the original value.
    #[rstest]
    #[case(TaskStatus::Pending, TaskStatus::InProgress)]
    #[case(TaskStatus::InProgress, TaskStatus::Completed)]
    #[case(TaskStatus::InProgress, TaskStatus::Pending)]
    #[case(TaskStatus::Completed, TaskStatus::Pending)]
    #[case(TaskStatus::Pending, TaskStatus::Cancelled)]
    #[case(TaskStatus::InProgress, TaskStatus::Cancelled)]
    fn test_prism_law_preview_review(#[case] from: TaskStatus, #[case] to: TaskStatus) {
        let prism = valid_transition_prism();
        let value = ValidTransition::new(from, to);

        // review creates the source (StatusTransitionAttempt::Valid)
        let reviewed = prism.review(value);

        // preview on the reviewed source should return a reference to the value
        let previewed = prism.preview(&reviewed);
        assert!(
            previewed.is_some(),
            "PreviewReview law: preview(&review({value:?})) should succeed"
        );
        assert_eq!(
            *previewed.unwrap(),
            value,
            "PreviewReview law: preview(&review({value:?})) should equal &{value:?}"
        );
    }

    /// Tests the `ReviewPreview` law for the `ValidTransitionPrism`.
    ///
    /// Law: If `preview(source).is_some()`, then
    /// `review(preview(source).unwrap().clone()) == source`
    ///
    /// This verifies that for valid transitions, reviewing the previewed value
    /// reconstructs the original source.
    #[rstest]
    #[case(TaskStatus::Pending, TaskStatus::InProgress)]
    #[case(TaskStatus::InProgress, TaskStatus::Completed)]
    #[case(TaskStatus::InProgress, TaskStatus::Pending)]
    #[case(TaskStatus::Completed, TaskStatus::Pending)]
    #[case(TaskStatus::Pending, TaskStatus::Cancelled)]
    #[case(TaskStatus::InProgress, TaskStatus::Cancelled)]
    fn test_prism_law_review_preview(#[case] from: TaskStatus, #[case] to: TaskStatus) {
        let prism = valid_transition_prism();
        let source = StatusTransitionAttempt::from_statuses(from, to);

        // preview should succeed for valid transitions
        let previewed = prism.preview(&source);
        assert!(
            previewed.is_some(),
            "preview should succeed for valid source {source:?}"
        );

        // review(preview(source).clone()) should equal source
        let reconstructed = prism.review(*previewed.unwrap());
        assert_eq!(
            reconstructed, source,
            "ReviewPreview law: review(preview({source:?}).clone()) should equal {source:?}"
        );
    }

    /// Tests the `PreviewOwnedReview` law for the `ValidTransitionPrism`.
    ///
    /// Law: `preview_owned(review(value)).is_some()` and
    /// the previewed value equals the original value.
    ///
    /// This verifies that reviewing then `preview_owned` yields the original value.
    #[rstest]
    #[case(TaskStatus::Pending, TaskStatus::InProgress)]
    #[case(TaskStatus::InProgress, TaskStatus::Completed)]
    #[case(TaskStatus::InProgress, TaskStatus::Pending)]
    #[case(TaskStatus::Completed, TaskStatus::Pending)]
    #[case(TaskStatus::Pending, TaskStatus::Cancelled)]
    #[case(TaskStatus::InProgress, TaskStatus::Cancelled)]
    fn test_prism_law_preview_owned_review(#[case] from: TaskStatus, #[case] to: TaskStatus) {
        let prism = valid_transition_prism();
        let value = ValidTransition::new(from, to);

        // review creates the source (StatusTransitionAttempt::Valid)
        let source = prism.review(value);

        // preview_owned on the reviewed source should succeed and return the original value
        let previewed = prism.preview_owned(source);
        assert!(
            previewed.is_some(),
            "preview_owned(review({value:?})) should succeed"
        );
        assert_eq!(
            previewed.unwrap(),
            value,
            "PreviewOwnedReview law: preview_owned(review({value:?})) should equal {value:?}"
        );
    }

    /// Tests the `ReviewPreviewOwned` law for the `ValidTransitionPrism`.
    ///
    /// Law: If `preview_owned(source).is_some()`, then
    /// `review(preview_owned(source).unwrap()) == source`
    ///
    /// This verifies that for valid transitions, reviewing the `preview_owned` value
    /// reconstructs the original source.
    #[rstest]
    #[case(TaskStatus::Pending, TaskStatus::InProgress)]
    #[case(TaskStatus::InProgress, TaskStatus::Completed)]
    #[case(TaskStatus::InProgress, TaskStatus::Pending)]
    #[case(TaskStatus::Completed, TaskStatus::Pending)]
    #[case(TaskStatus::Pending, TaskStatus::Cancelled)]
    #[case(TaskStatus::InProgress, TaskStatus::Cancelled)]
    fn test_prism_law_review_preview_owned(#[case] from: TaskStatus, #[case] to: TaskStatus) {
        let prism = valid_transition_prism();
        let source = StatusTransitionAttempt::from_statuses(from, to);

        // preview_owned should succeed for valid transitions
        let previewed = prism.preview_owned(source);
        assert!(
            previewed.is_some(),
            "preview_owned should succeed for {source:?}"
        );

        // review(preview_owned(source)) should equal source
        let reconstructed = prism.review(previewed.unwrap());
        assert_eq!(
            reconstructed, source,
            "ReviewPreviewOwned law: review(preview_owned({source:?})) should equal {source:?}"
        );
    }

    /// Tests the `is_valid_transition` helper function for all valid transitions.
    #[rstest]
    #[case(TaskStatus::Pending, TaskStatus::InProgress, true)]
    #[case(TaskStatus::InProgress, TaskStatus::Completed, true)]
    #[case(TaskStatus::InProgress, TaskStatus::Pending, true)]
    #[case(TaskStatus::Completed, TaskStatus::Pending, true)]
    #[case(TaskStatus::Pending, TaskStatus::Cancelled, true)]
    #[case(TaskStatus::InProgress, TaskStatus::Cancelled, true)]
    #[case(TaskStatus::Pending, TaskStatus::Completed, false)]
    #[case(TaskStatus::Completed, TaskStatus::InProgress, false)]
    #[case(TaskStatus::Completed, TaskStatus::Cancelled, false)]
    #[case(TaskStatus::Cancelled, TaskStatus::Pending, false)]
    #[case(TaskStatus::Cancelled, TaskStatus::InProgress, false)]
    #[case(TaskStatus::Cancelled, TaskStatus::Completed, false)]
    #[case(TaskStatus::Pending, TaskStatus::Pending, false)]
    fn test_is_valid_transition_function(
        #[case] from: TaskStatus,
        #[case] to: TaskStatus,
        #[case] expected: bool,
    ) {
        let result = is_valid_transition(from, to);
        assert_eq!(
            result, expected,
            "is_valid_transition({from:?}, {to:?}) should be {expected}"
        );
    }

    #[rstest]
    #[case(TaskStatus::Pending, TaskStatus::Pending, true)]
    #[case(TaskStatus::InProgress, TaskStatus::InProgress, true)]
    #[case(TaskStatus::Completed, TaskStatus::Completed, true)]
    #[case(TaskStatus::Cancelled, TaskStatus::Cancelled, true)]
    #[case(TaskStatus::Pending, TaskStatus::InProgress, false)]
    #[case(TaskStatus::InProgress, TaskStatus::Completed, false)]
    fn test_is_no_change_transition(
        #[case] from: TaskStatus,
        #[case] to: TaskStatus,
        #[case] expected: bool,
    ) {
        let result = is_no_change_transition(from, to);
        assert_eq!(
            result, expected,
            "is_no_change_transition({from:?}, {to:?}) should be {expected}"
        );
    }

    #[rstest]
    fn test_valid_transition_new() {
        let valid = ValidTransition::new(TaskStatus::Pending, TaskStatus::InProgress);

        assert_eq!(valid.from, TaskStatus::Pending);
        assert_eq!(valid.to, TaskStatus::InProgress);
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

    // -------------------------------------------------------------------------
    // Optional Optic Tests for Option<String> fields
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_optional_optic_get_option_when_some() {
        let optional = description_optional();
        let task = Task::new(TaskId::generate(), "Task", Timestamp::now())
            .with_description("Existing description");

        let result = optional.get_option(&task);

        assert_eq!(result, Some(&"Existing description".to_string()));
    }

    #[rstest]
    fn test_optional_optic_get_option_when_none() {
        let optional = description_optional();
        let task = Task::new(TaskId::generate(), "Task", Timestamp::now());

        let result = optional.get_option(&task);

        assert_eq!(result, None);
    }

    #[rstest]
    fn test_optional_optic_modify_when_some() {
        let optional = description_optional();
        let task =
            Task::new(TaskId::generate(), "Task", Timestamp::now()).with_description("hello");

        let updated = optional.modify(task, |description| description.to_uppercase());

        assert_eq!(updated.description, Some("HELLO".to_string()));
    }

    #[rstest]
    fn test_optional_optic_modify_when_none() {
        let optional = description_optional();
        let task = Task::new(TaskId::generate(), "Task", Timestamp::now());

        // When None, modify should return the original unchanged
        let updated = optional.modify(task, |description| description.to_uppercase());

        assert_eq!(updated.description, None);
    }

    #[rstest]
    fn test_optional_optic_set_when_some() {
        let optional = description_optional();
        let task = Task::new(TaskId::generate(), "Task", Timestamp::now())
            .with_description("Old description");

        let updated = optional.set(task, "New description".to_string());

        assert_eq!(updated.description, Some("New description".to_string()));
    }

    #[rstest]
    fn test_optional_optic_set_when_none() {
        let optional = description_optional();
        let task = Task::new(TaskId::generate(), "Task", Timestamp::now());

        // Set creates the value even when None
        let updated = optional.set(task, "New description".to_string());

        assert_eq!(updated.description, Some("New description".to_string()));
    }

    #[rstest]
    fn test_optional_optic_is_present_when_some() {
        let optional = description_optional();
        let task =
            Task::new(TaskId::generate(), "Task", Timestamp::now()).with_description("Description");

        assert!(optional.is_present(&task));
    }

    #[rstest]
    fn test_optional_optic_is_present_when_none() {
        let optional = description_optional();
        let task = Task::new(TaskId::generate(), "Task", Timestamp::now());

        assert!(!optional.is_present(&task));
    }

    #[rstest]
    fn test_apply_updates_with_lens_uses_optional_for_description() {
        let task = Task::new(TaskId::generate(), "Task", Timestamp::now())
            .with_description("Old description");
        let request = UpdateTaskRequest {
            title: None,
            description: Some("New description".to_string()),
            status: None,
            priority: None,
            version: 1,
        };
        let now = Timestamp::now();

        let updated = apply_updates_with_lens(task, &request, now).unwrap();

        assert_eq!(updated.description, Some("New description".to_string()));
    }

    #[rstest]
    fn test_apply_updates_with_lens_sets_description_when_none() {
        let task = Task::new(TaskId::generate(), "Task", Timestamp::now());
        let request = UpdateTaskRequest {
            title: None,
            description: Some("New description".to_string()),
            status: None,
            priority: None,
            version: 1,
        };
        let now = Timestamp::now();

        let updated = apply_updates_with_lens(task, &request, now).unwrap();

        assert_eq!(updated.description, Some("New description".to_string()));
    }

    #[rstest]
    fn test_apply_updates_with_lens_clears_description_with_empty_string() {
        // Given: A task with an existing description
        let task = Task::new(TaskId::generate(), "Task", Timestamp::now())
            .with_description("Existing description");
        assert!(task.description.is_some());

        // When: Updating with an empty string description (clear request)
        let request = UpdateTaskRequest {
            title: None,
            description: Some(String::new()), // Empty string = clear request
            status: None,
            priority: None,
            version: 1,
        };
        let now = Timestamp::now();

        let updated = apply_updates_with_lens(task, &request, now).unwrap();

        // Then: Description should be cleared (None)
        assert_eq!(updated.description, None);
    }

    #[rstest]
    fn test_apply_updates_with_lens_clears_description_with_whitespace_only() {
        // Given: A task with an existing description
        let task = Task::new(TaskId::generate(), "Task", Timestamp::now())
            .with_description("Existing description");

        // When: Updating with whitespace-only description (also a clear request)
        let request = UpdateTaskRequest {
            title: None,
            description: Some("   ".to_string()), // Whitespace only = clear request
            status: None,
            priority: None,
            version: 1,
        };
        let now = Timestamp::now();

        let updated = apply_updates_with_lens(task, &request, now).unwrap();

        // Then: Description should be cleared (None)
        assert_eq!(updated.description, None);
    }

    // -------------------------------------------------------------------------
    // Change Detection Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_detect_task_changes_no_changes() {
        let task = Task::new(TaskId::generate(), "Task", Timestamp::now());
        let changes = detect_task_changes(&task, &task);
        assert!(changes.is_empty());
    }

    #[rstest]
    fn test_detect_task_changes_title_change() {
        let old_task = Task::new(TaskId::generate(), "Old Title", Timestamp::now());
        // Create new task with different title by constructing manually
        let new_task = Task {
            title: "New Title".to_string(),
            ..old_task.clone()
        };

        let changes = detect_task_changes(&old_task, &new_task);

        assert_eq!(changes.len(), 1);
        assert!(matches!(
            &changes[0],
            DetectedChange::TitleUpdated { old_title, new_title }
            if old_title == "Old Title" && new_title == "New Title"
        ));
    }

    #[rstest]
    fn test_detect_task_changes_description_change() {
        let old_task = Task::new(TaskId::generate(), "Task", Timestamp::now())
            .with_description("Old description");
        let new_task = old_task.clone().with_description("New description");

        let changes = detect_task_changes(&old_task, &new_task);

        assert_eq!(changes.len(), 1);
        assert!(matches!(
            &changes[0],
            DetectedChange::DescriptionUpdated { old_description, new_description }
            if old_description.as_deref() == Some("Old description")
                && new_description.as_deref() == Some("New description")
        ));
    }

    #[rstest]
    fn test_detect_task_changes_priority_change() {
        let old_task =
            Task::new(TaskId::generate(), "Task", Timestamp::now()).with_priority(Priority::Low);
        let new_task = old_task.clone().with_priority(Priority::Critical);

        let changes = detect_task_changes(&old_task, &new_task);

        assert_eq!(changes.len(), 1);
        assert!(matches!(
            &changes[0],
            DetectedChange::PriorityChanged { old_priority, new_priority }
            if *old_priority == Priority::Low && *new_priority == Priority::Critical
        ));
    }

    #[rstest]
    fn test_detect_task_changes_multiple_changes() {
        let old_task = Task::new(TaskId::generate(), "Old Title", Timestamp::now())
            .with_description("Old description")
            .with_priority(Priority::Low);
        // Create new task with multiple changes
        let new_task = Task {
            title: "New Title".to_string(),
            description: Some("New description".to_string()),
            priority: Priority::High,
            ..old_task.clone()
        };

        let changes = detect_task_changes(&old_task, &new_task);

        assert_eq!(changes.len(), 3);
        // Verify order: title, description, priority
        assert!(matches!(&changes[0], DetectedChange::TitleUpdated { .. }));
        assert!(matches!(
            &changes[1],
            DetectedChange::DescriptionUpdated { .. }
        ));
        assert!(matches!(
            &changes[2],
            DetectedChange::PriorityChanged { .. }
        ));
    }
}
