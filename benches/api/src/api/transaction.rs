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

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use tokio::sync::Mutex;

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

const DEFAULT_MAX_RETRIES: u8 = 1;
const DEFAULT_BASE_DELAY_MS: u64 = 5;
const MAX_BACKOFF_MS: u64 = 10;

/// Error code for retryable version conflicts (repository-level CAS failure).
const RETRYABLE_CONFLICT_CODE: &str = "VERSION_CONFLICT_RETRYABLE";

/// Error code for stale-version conflicts (handler-level version mismatch).
const STALE_VERSION_CONFLICT_CODE: &str = "VERSION_CONFLICT";

// =============================================================================
// Conflict Classification
// =============================================================================

/// Classifies the kind of 409 Conflict error.
///
/// This enum distinguishes between different types of conflict errors:
/// - `StaleVersion`: The client sent an outdated version (handler-level)
/// - `RetryableCas`: A CAS failure between read and write (repository-level)
/// - `Other`: Any other error (non-409, or 409 with unknown code)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKind {
    /// Handler-level stale version mismatch (code = `VERSION_CONFLICT`).
    StaleVersion,
    /// Repository-level CAS failure (code = `VERSION_CONFLICT_RETRYABLE`).
    RetryableCas,
    /// Any other error (non-409, or 409 with unrecognized code).
    Other,
}

/// Classifies a conflict error into its kind (pure function).
///
/// This function examines the HTTP status and error code to determine
/// the conflict type. Non-409 errors always return `Other`.
///
/// # Arguments
///
/// * `error` - The error response to classify
///
/// # Returns
///
/// The [`ConflictKind`] classification of the error.
#[must_use]
pub fn classify_conflict_kind(error: &ApiErrorResponse) -> ConflictKind {
    if error.status != StatusCode::CONFLICT {
        return ConflictKind::Other;
    }
    match error.error.code.as_str() {
        STALE_VERSION_CONFLICT_CODE => ConflictKind::StaleVersion,
        RETRYABLE_CONFLICT_CODE => ConflictKind::RetryableCas,
        _ => ConflictKind::Other,
    }
}

/// Returns `true` if the error is a stale-version 409 Conflict.
///
/// Stale-version conflicts occur when the client sends an outdated version
/// in the update request. These are candidates for read-repair.
#[must_use]
pub fn is_stale_version_conflict(error: &ApiErrorResponse) -> bool {
    classify_conflict_kind(error) == ConflictKind::StaleVersion
}

// =============================================================================
// Read-Repair Rebase Logic
// =============================================================================

/// Error type for non-commutative field conflicts during rebase.
///
/// When two concurrent updates modify the same field to different values,
/// the conflict cannot be automatically resolved and must be reported
/// back to the client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RebaseError {
    /// A non-commutative field conflict where both the original update
    /// and a concurrent update modified the same field.
    NonCommutativeConflict {
        /// The name of the conflicting field.
        field: &'static str,
    },
}

impl std::fmt::Display for RebaseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonCommutativeConflict { field } => {
                write!(
                    formatter,
                    "Non-commutative conflict on field '{field}': concurrent update detected"
                )
            }
        }
    }
}

/// Rebases an update request against the latest task state (pure function).
///
/// This function implements a 3-way merge: given the original base task
/// (what the client saw), the latest task (current DB state), and the
/// client's update request, it produces a new request with the version
/// updated to match the latest task.
///
/// # Conflict Detection
///
/// For each field the client wants to update (`request.field.is_some()`):
/// - If `original_base.field != latest.field` (another user changed it)
///   AND `latest.field` differs from what the client wants to set,
///   it is a non-commutative conflict.
/// - If `original_base.field == latest.field` (no concurrent change),
///   the rebase succeeds.
///
/// # Arguments
///
/// * `original_base` - The task state the client based their update on
/// * `latest` - The current task state from the database
/// * `request` - The client's update request
///
/// # Returns
///
/// `Ok(UpdateTaskRequest)` with version rebased to `latest.version`,
/// or `Err(RebaseError)` if a non-commutative conflict is detected.
#[must_use]
pub fn rebase_update_request(
    original_base: &Task,
    latest: &Task,
    request: &UpdateTaskRequest,
) -> Result<UpdateTaskRequest, RebaseError> {
    // Check title conflict
    if request.title.is_some()
        && original_base.title != latest.title
        && request.title.as_deref() != Some(latest.title.as_str())
    {
        return Err(RebaseError::NonCommutativeConflict { field: "title" });
    }

    // Check description conflict
    if request.description.is_some()
        && original_base.description != latest.description
        && request.description != latest.description
    {
        return Err(RebaseError::NonCommutativeConflict {
            field: "description",
        });
    }

    // Check priority conflict
    if request.priority.is_some() && original_base.priority != latest.priority {
        let request_priority: Priority = request
            .priority
            .expect("priority is_some checked above")
            .into();
        if request_priority != latest.priority {
            return Err(RebaseError::NonCommutativeConflict { field: "priority" });
        }
    }

    // No conflicts: rebase version to latest
    Ok(UpdateTaskRequest {
        title: request.title.clone(),
        description: request.description.clone(),
        status: request.status,
        priority: request.priority,
        version: latest.version,
    })
}

/// Computes the backoff cap for a given retry index (pure function).
///
/// Formula: `base_delay_ms * 2^retry_index`, clamped to avoid overflow.
/// `retry_index` is 0-based (first retry = 0).
///
/// # Examples
///
/// ```ignore
/// assert_eq!(compute_backoff_cap(1, 0), 1);  // 1 * 2^0 = 1
/// assert_eq!(compute_backoff_cap(1, 1), 2);  // 1 * 2^1 = 2
/// assert_eq!(compute_backoff_cap(1, 2), 4);  // 1 * 2^2 = 4
/// assert_eq!(compute_backoff_cap(10, 3), 80); // 10 * 2^3 = 80
/// ```
#[must_use]
pub const fn compute_backoff_cap(base_delay_ms: u64, retry_index: u8) -> u64 {
    let factor = match 1u64.checked_shl(retry_index as u32) {
        Some(value) => value,
        None => u64::MAX,
    };
    base_delay_ms.saturating_mul(factor)
}

/// Samples a jitter delay from `[0, cap]` given a random value (pure function).
///
/// Returns 0 if `cap` is 0. Returns `rand_value` unchanged if `cap` is `u64::MAX`
/// (avoids overflow from `cap + 1`). Otherwise returns `rand_value % (cap + 1)`.
///
/// This function isolates the non-deterministic part of backoff calculation,
/// making the delay logic testable with deterministic inputs.
#[must_use]
pub const fn sample_delay(rand_value: u64, cap: u64) -> u64 {
    if cap == 0 {
        0
    } else if cap == u64::MAX {
        rand_value
    } else {
        rand_value % (cap + 1)
    }
}

/// Promotes a `VERSION_CONFLICT` error to `VERSION_CONFLICT_RETRYABLE` (pure function).
///
/// This function is used inside `update_task_inner` to mark repository-level CAS
/// failures as retryable. The default `From<RepositoryError>` conversion produces
/// `VERSION_CONFLICT` (non-retryable) so that other handlers are unaffected.
/// Only the retry-wrapped `update_task` path promotes the code.
///
/// Non-conflict errors pass through unchanged.
#[must_use]
pub fn promote_to_retryable_conflict(mut error: ApiErrorResponse) -> ApiErrorResponse {
    if error.status == StatusCode::CONFLICT && error.error.code == "VERSION_CONFLICT" {
        error.error.code = RETRYABLE_CONFLICT_CODE.to_string();
    }
    error
}

/// Returns `true` if the error is a retryable 409 Conflict.
///
/// Only repository-level CAS failures (code = `VERSION_CONFLICT_RETRYABLE`) are
/// retryable. Handler-level stale version mismatches use the standard
/// `VERSION_CONFLICT` code and are **not** retried.
///
/// This is a convenience wrapper around [`classify_conflict_kind`].
#[must_use]
pub fn is_retryable_conflict(error: &ApiErrorResponse) -> bool {
    classify_conflict_kind(error) == ConflictKind::RetryableCas
}

/// Returns `true` if the retry budget was exhausted and the final result is an error (pure function).
///
/// This predicate is used after `retry_on_conflict` completes to decide whether
/// the `retry_exhausted` counter should be incremented.
///
/// # Arguments
///
/// * `result_is_err` - Whether the final result of the retry loop was an error
/// * `retries_used` - Number of retries actually triggered (from `on_retry` callback)
/// * `max_retries` - Maximum retries configured for the retry loop
#[must_use]
pub const fn should_count_retry_exhausted(
    result_is_err: bool,
    retries_used: usize,
    max_retries: u8,
) -> bool {
    result_is_err && max_retries > 0 && retries_used >= max_retries as usize
}

/// Retries an async operation on retryable 409 Conflict with full jitter backoff.
///
/// Full jitter backoff: `delay = random(0, base_delay_ms * 2^retry_index)`.
/// Only retryable 409 Conflict errors (code = `VERSION_CONFLICT_RETRYABLE`) trigger
/// a retry; all other errors -- including stale-version 409s -- are returned immediately.
///
/// The optional `on_retry` callback is invoked each time a retry is triggered,
/// receiving the current retry index (0-based). This enables observability
/// (e.g. incrementing an atomic counter) without coupling the retry logic to
/// application state.
///
/// # Errors
///
/// Returns the final [`ApiErrorResponse`] if all retries are exhausted (for retryable 409)
/// or immediately for non-retryable errors.
pub async fn retry_on_conflict<F, Fut, T, R>(
    operation: F,
    max_retries: u8,
    base_delay_ms: u64,
    on_retry: R,
) -> Result<T, ApiErrorResponse>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, ApiErrorResponse>>,
    R: Fn(u8),
{
    let mut attempt = 0u8;
    loop {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(error) if is_retryable_conflict(&error) && attempt < max_retries => {
                on_retry(attempt);
                let cap = compute_backoff_cap(base_delay_ms, attempt).min(MAX_BACKOFF_MS);
                attempt += 1;
                let delay = sample_delay(rand::random::<u64>(), cap);
                if delay > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                }
            }
            Err(error) => return Err(error),
        }
    }
}

/// Cleanup interval: purge dead `Weak` entries every N acquisitions.
const CLEANUP_INTERVAL: usize = 64;

/// Map size threshold below which cleanup is skipped entirely.
const CLEANUP_THRESHOLD: usize = 128;

/// Per-key lock map that serializes concurrent updates to the same `task_id`.
///
/// Different `task_id` values are processed in parallel without contention.
/// Entries are stored as `Weak` references and automatically cleaned up when
/// no active guards remain, preventing unbounded memory growth.
///
/// Cleanup is amortized: dead entries are purged every [`CLEANUP_INTERVAL`]
/// acquisitions and only when the map exceeds [`CLEANUP_THRESHOLD`] entries,
/// avoiding O(N) scans on every call.
pub struct KeyedUpdateQueue {
    locks: std::sync::Mutex<HashMap<TaskId, std::sync::Weak<Mutex<()>>>>,
    acquire_counter: AtomicUsize,
}

impl KeyedUpdateQueue {
    #[must_use]
    pub fn new() -> Self {
        Self {
            locks: std::sync::Mutex::new(HashMap::new()),
            acquire_counter: AtomicUsize::new(0),
        }
    }

    /// Acquires a per-`task_id` lock, serializing concurrent updates.
    ///
    /// Dead entries (where all guards have been dropped) are periodically
    /// cleaned up to prevent memory leaks without incurring O(N) cost on
    /// every acquisition.
    pub async fn acquire(&self, task_id: &TaskId) -> KeyedGuard {
        let mutex = {
            let mut map = self
                .locks
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            // Try to upgrade existing Weak; create new Arc if absent or expired
            let mutex = map
                .get(task_id)
                .and_then(std::sync::Weak::upgrade)
                .unwrap_or_else(|| {
                    let new_mutex = Arc::new(Mutex::new(()));
                    map.insert(task_id.clone(), Arc::downgrade(&new_mutex));
                    new_mutex
                });

            // Amortized cleanup: purge dead entries periodically
            let tick = self.acquire_counter.fetch_add(1, Ordering::Relaxed);
            if map.len() > CLEANUP_THRESHOLD && tick.is_multiple_of(CLEANUP_INTERVAL) {
                map.retain(|_, weak| weak.strong_count() > 0);
            }

            mutex
        };
        let guard = mutex.lock_owned().await;
        KeyedGuard { _guard: guard }
    }

    /// Returns the number of entries currently in the internal map (for testing).
    #[cfg(test)]
    pub fn debug_entry_count(&self) -> usize {
        self.locks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }

    /// Forces a cleanup of dead entries (for testing).
    #[cfg(test)]
    pub fn force_cleanup(&self) {
        let mut map = self
            .locks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        map.retain(|_, weak| weak.strong_count() > 0);
    }
}

impl Default for KeyedUpdateQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard that holds a per-`task_id` lock until dropped.
pub struct KeyedGuard {
    _guard: tokio::sync::OwnedMutexGuard<()>,
}

/// Returns `true` if the request contains no non-commutative field changes
/// (title, description, status, priority are all `None`), allowing it to be
/// safely merged even when the version has changed.
#[must_use]
pub const fn can_merge_without_conflict(request: &UpdateTaskRequest) -> bool {
    request.title.is_none()
        && request.description.is_none()
        && request.status.is_none()
        && request.priority.is_none()
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

    let _keyed_guard = state.keyed_update_queue.acquire(&task_id).await;

    let retry_counter = Arc::clone(&state.retry_attempts);
    let exhausted_counter = Arc::clone(&state.retry_exhausted);
    let retries_used = Arc::new(AtomicUsize::new(0));
    let retries_used_inner = Arc::clone(&retries_used);
    let task_id_for_log = task_id.clone();

    let result = retry_on_conflict(
        || update_task_inner(state.clone(), task_id.clone(), request.clone()),
        DEFAULT_MAX_RETRIES,
        DEFAULT_BASE_DELAY_MS,
        move |attempt| {
            tracing::info!(
                task_id = %task_id_for_log,
                retry_index = attempt,
                "Retrying task update after retryable conflict"
            );
            retry_counter.fetch_add(1, Ordering::Relaxed);
            retries_used_inner.fetch_add(1, Ordering::Relaxed);
        },
    )
    .await;

    // Count exhausted retries: any failure after retries were attempted
    // covers both retryable and stale-version terminal errors.
    if should_count_retry_exhausted(
        result.is_err(),
        retries_used.load(Ordering::Relaxed),
        DEFAULT_MAX_RETRIES,
    ) {
        exhausted_counter.fetch_add(1, Ordering::Relaxed);
    }

    result
}

/// Core update logic extracted from `update_task` for retry wrapping.
///
/// Visibility is `pub(crate)` to allow direct integration testing.
#[allow(clippy::future_not_send)]
pub(crate) async fn update_task_inner(
    state: AppState,
    task_id: TaskId,
    request: UpdateTaskRequest,
) -> Result<(StatusCode, JsonResponse<TaskResponse>), ApiErrorResponse> {
    // Reject status changes via PUT; use PATCH /tasks/{id}/status instead.
    if request.status.is_some() {
        return Err(ApiErrorResponse::bad_request(
            "UNSUPPORTED_FIELD",
            "Status updates are not supported via PUT. Use PATCH /tasks/{id}/status instead.",
        ));
    }

    let old_task = state
        .task_repository
        .find_by_id(&task_id)
        .await
        .map_err(ApiErrorResponse::from)?
        .ok_or_else(|| ApiErrorResponse::not_found(format!("Task {task_id} not found")))?;

    // Short-circuit: if the request contains no non-commutative fields (all None),
    // treat it as a no-op and return the current task without persisting anything.
    // This avoids unnecessary version bumps and write amplification.
    if can_merge_without_conflict(&request) {
        return Ok((StatusCode::OK, JsonResponse(TaskResponse::from(&old_task))));
    }

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
        .await
        .map_err(ApiErrorResponse::from)?;

    // Validate and apply updates using Lens (pure function)
    // Clone timestamp since it will be used for events later
    let updated_task = apply_updates_with_lens(old_task.clone(), &request, now.clone())?;

    // Detect changes (pure function)
    let changes = detect_task_changes(&old_task, &updated_task);

    // Build events from changes (I/O boundary: generates event IDs)
    let events = build_events_from_changes(&task_id, &changes, &now, current_event_version);

    // Save task and write events using best-effort consistency.
    // Promote VERSION_CONFLICT to VERSION_CONFLICT_RETRYABLE so that
    // the outer `retry_on_conflict` can distinguish CAS failures from
    // stale-version rejections at the handler level.
    let write_result = save_task_with_events(&state, &updated_task, events, current_event_version)
        .await
        .map_err(promote_to_retryable_conflict)?;

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
            let version = current_version
                .saturating_add(1)
                .saturating_add(index as u64);
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
    let new_version = updated
        .version
        .checked_add(1)
        .ok_or_else(|| ApiErrorResponse::internal_error("Version overflow"))?;
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
                EventId::generate_v7(), // I/O boundary: generate event ID
                now,                    // Use same timestamp as task update
                current_event_version.saturating_add(1), // This event's version
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
const fn validate_status_transition(
    current: TaskStatus,
    new_status: TaskStatus,
) -> Either<StatusTransitionError, StatusTransitionResult> {
    // Create a StatusTransitionAttempt from the status pair
    let attempt = StatusTransitionAttempt::from_statuses(current, new_status);

    // Handle based on the attempt classification.
    // All branches return values (no panics) to comply with FP error-as-value principle.
    // Note: `valid_transition_prism()` is available for callers who need the Prism interface;
    // here we destructure the enum directly for const-compatibility.
    match attempt {
        StatusTransitionAttempt::NoChange(_) => Either::Right(StatusTransitionResult::NoChange),
        StatusTransitionAttempt::Valid(valid) => {
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
    let new_version = updated.version.saturating_add(1);
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
    let new_version = updated.version.saturating_add(1);
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
        EventId::generate_v7(), // I/O boundary: generate event ID
        now,                    // Use same timestamp as task update
        current_event_version.saturating_add(1), // This event's version
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
    let new_version = updated.version.saturating_add(1);
    version_lens.set(updated, new_version)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU8, Ordering};

    use rstest::rstest;

    use super::*;
    use super::super::error::ApiError;

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

    // -------------------------------------------------------------------------
    // retry_on_conflict Tests
    // -------------------------------------------------------------------------

    /// Helper: creates an `Arc<AtomicU8>` counter and a clone for use in async closures.
    fn new_call_counter() -> (Arc<AtomicU8>, Arc<AtomicU8>) {
        let counter = Arc::new(AtomicU8::new(0));
        let counter_clone = Arc::clone(&counter);
        (counter, counter_clone)
    }

    #[rstest]
    #[tokio::test]
    async fn test_retry_on_conflict_succeeds_on_first_attempt() {
        let (call_count, call_count_inner) = new_call_counter();

        let result: Result<u32, ApiErrorResponse> = retry_on_conflict(
            || {
                let count = Arc::clone(&call_count_inner);
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Ok(42)
                }
            },
            3,
            1,
            |_| {},
        )
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[rstest]
    #[tokio::test]
    async fn test_retry_on_conflict_retries_on_retryable_409_then_succeeds() {
        let (call_count, call_count_inner) = new_call_counter();

        let result: Result<u32, ApiErrorResponse> = retry_on_conflict(
            || {
                let count = Arc::clone(&call_count_inner);
                async move {
                    let attempt = count.fetch_add(1, Ordering::SeqCst);
                    if attempt < 2 {
                        Err(ApiErrorResponse::retryable_conflict("CAS failure"))
                    } else {
                        Ok(42)
                    }
                }
            },
            3,
            0,
            |_| {},
        )
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[rstest]
    #[tokio::test]
    async fn test_retry_on_conflict_does_not_retry_on_non_409_error() {
        let (call_count, call_count_inner) = new_call_counter();

        let result: Result<u32, ApiErrorResponse> = retry_on_conflict(
            || {
                let count = Arc::clone(&call_count_inner);
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Err(ApiErrorResponse::not_found("task not found"))
                }
            },
            3,
            0,
            |_| {},
        )
        .await;

        assert_eq!(result.unwrap_err().status, StatusCode::NOT_FOUND);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[rstest]
    #[tokio::test]
    async fn test_retry_on_conflict_does_not_retry_on_stale_version_conflict() {
        let (call_count, call_count_inner) = new_call_counter();

        let result: Result<u32, ApiErrorResponse> = retry_on_conflict(
            || {
                let count = Arc::clone(&call_count_inner);
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    // Non-retryable conflict (stale version from handler)
                    Err(ApiErrorResponse::conflict("Expected version 1, found 2"))
                }
            },
            3,
            0,
            |_| {},
        )
        .await;

        assert_eq!(result.unwrap_err().status, StatusCode::CONFLICT);
        // Should NOT retry -- only 1 attempt
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[rstest]
    #[tokio::test]
    async fn test_retry_on_conflict_exhausts_retries_returns_409() {
        let (call_count, call_count_inner) = new_call_counter();

        let result: Result<u32, ApiErrorResponse> = retry_on_conflict(
            || {
                let count = Arc::clone(&call_count_inner);
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Err(ApiErrorResponse::retryable_conflict("CAS failure"))
                }
            },
            3,
            0,
            |_| {},
        )
        .await;

        assert_eq!(result.unwrap_err().status, StatusCode::CONFLICT);
        // 1 initial + 3 retries = 4 total attempts
        assert_eq!(call_count.load(Ordering::SeqCst), 4);
    }

    #[rstest]
    #[tokio::test]
    async fn test_retry_on_conflict_zero_retries_returns_immediately() {
        let (call_count, call_count_inner) = new_call_counter();

        let result: Result<u32, ApiErrorResponse> = retry_on_conflict(
            || {
                let count = Arc::clone(&call_count_inner);
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Err(ApiErrorResponse::retryable_conflict("CAS failure"))
                }
            },
            0,
            0,
            |_| {},
        )
        .await;

        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    // -------------------------------------------------------------------------
    // KeyedUpdateQueue Tests
    // -------------------------------------------------------------------------

    /// Spawns `count` tasks that each acquire the keyed lock, increment a counter,
    /// sleep, then decrement. Returns the observed maximum concurrency.
    async fn measure_max_concurrency(
        queue: &Arc<KeyedUpdateQueue>,
        task_ids: Vec<TaskId>,
        sleep_ms: u64,
    ) -> u32 {
        use std::sync::atomic::AtomicU32;

        let counter = Arc::new(AtomicU32::new(0));
        let max_concurrent = Arc::new(AtomicU32::new(0));

        let handles: Vec<_> = task_ids
            .into_iter()
            .map(|task_id| {
                let queue = Arc::clone(queue);
                let counter = Arc::clone(&counter);
                let max_concurrent = Arc::clone(&max_concurrent);
                tokio::spawn(async move {
                    let _guard = queue.acquire(&task_id).await;
                    let current = counter.fetch_add(1, Ordering::SeqCst) + 1;
                    max_concurrent.fetch_max(current, Ordering::SeqCst);
                    tokio::time::sleep(tokio::time::Duration::from_millis(sleep_ms)).await;
                    counter.fetch_sub(1, Ordering::SeqCst);
                })
            })
            .collect();

        for handle in handles {
            handle.await.unwrap();
        }

        max_concurrent.load(Ordering::SeqCst)
    }

    #[rstest]
    #[tokio::test]
    async fn test_keyed_update_queue_serializes_same_task_id() {
        let queue = Arc::new(KeyedUpdateQueue::new());
        let task_id = TaskId::generate();
        let task_ids = vec![task_id; 5];

        let max = measure_max_concurrency(&queue, task_ids, 1).await;
        assert_eq!(max, 1, "Same task_id updates should be serialized");
    }

    #[rstest]
    #[tokio::test]
    async fn test_keyed_update_queue_allows_parallel_different_task_ids() {
        let queue = Arc::new(KeyedUpdateQueue::new());
        let task_ids: Vec<_> = (0..5).map(|_| TaskId::generate()).collect();

        let max = measure_max_concurrency(&queue, task_ids, 10).await;
        assert!(max > 1, "Different task_ids should run in parallel");
    }

    // -------------------------------------------------------------------------
    // can_merge_without_conflict Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[case(None, None, None, None, true)]
    #[case(Some("Title".to_string()), None, None, None, false)]
    #[case(None, Some("Desc".to_string()), None, None, false)]
    #[case(None, None, Some(TaskStatusDto::InProgress), None, false)]
    #[case(None, None, None, Some(crate::api::dto::PriorityDto::High), false)]
    #[case(Some("T".to_string()), Some("D".to_string()), None, None, false)]
    fn test_can_merge_without_conflict(
        #[case] title: Option<String>,
        #[case] description: Option<String>,
        #[case] status: Option<TaskStatusDto>,
        #[case] priority: Option<crate::api::dto::PriorityDto>,
        #[case] expected: bool,
    ) {
        let request = UpdateTaskRequest {
            title,
            description,
            status,
            priority,
            version: 1,
        };
        assert_eq!(can_merge_without_conflict(&request), expected);
    }

    // -------------------------------------------------------------------------
    // compute_backoff_cap Tests (pure function)
    // -------------------------------------------------------------------------

    #[rstest]
    #[case(1, 0, 1)] // 1 * 2^0 = 1
    #[case(1, 1, 2)] // 1 * 2^1 = 2
    #[case(1, 2, 4)] // 1 * 2^2 = 4
    #[case(1, 3, 8)] // 1 * 2^3 = 8
    #[case(10, 0, 10)] // 10 * 2^0 = 10
    #[case(10, 3, 80)] // 10 * 2^3 = 80
    #[case(0, 5, 0)] // 0 * 2^5 = 0
    fn test_compute_backoff_cap(
        #[case] base_delay_ms: u64,
        #[case] retry_index: u8,
        #[case] expected: u64,
    ) {
        assert_eq!(compute_backoff_cap(base_delay_ms, retry_index), expected);
    }

    #[rstest]
    fn test_compute_backoff_cap_first_retry_is_base() {
        // First retry (index 0) should use base delay as the cap
        assert_eq!(compute_backoff_cap(5, 0), 5);
    }

    #[rstest]
    fn test_compute_backoff_cap_overflow_saturates() {
        // Large retry_index should saturate rather than panic
        let result = compute_backoff_cap(1, 64);
        assert_eq!(result, u64::MAX);

        let result = compute_backoff_cap(1, u8::MAX);
        assert_eq!(result, u64::MAX);
    }

    // -------------------------------------------------------------------------
    // is_retryable_conflict Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_is_retryable_conflict_with_retryable_conflict() {
        let error = ApiErrorResponse::retryable_conflict("CAS failure");
        assert!(is_retryable_conflict(&error));
    }

    #[rstest]
    fn test_is_retryable_conflict_with_stale_version_conflict() {
        let error = ApiErrorResponse::conflict("Expected version 1, found 2");
        assert!(!is_retryable_conflict(&error));
    }

    #[rstest]
    fn test_is_retryable_conflict_with_non_conflict_error() {
        let error = ApiErrorResponse::not_found("task not found");
        assert!(!is_retryable_conflict(&error));
    }

    // -------------------------------------------------------------------------
    // KeyedUpdateQueue entry cleanup Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_keyed_update_queue_releases_unused_keys() {
        let queue = KeyedUpdateQueue::new();

        // Acquire and drop guards for many unique task_ids
        for _ in 0..200 {
            let task_id = TaskId::generate();
            let _guard = queue.acquire(&task_id).await;
            // guard drops here
        }

        // Force cleanup to remove all dead Weak entries
        queue.force_cleanup();

        // After cleanup, the map should be empty because all guards have been dropped.
        let entry_count = queue.debug_entry_count();
        assert_eq!(
            entry_count, 0,
            "Expected 0 entries after forced cleanup, found {entry_count}"
        );
    }

    // -------------------------------------------------------------------------
    // No-op request early return Tests
    // -------------------------------------------------------------------------

    /// Verifies that a request with all `None` fields is considered a no-op.
    /// The `update_task_inner` function short-circuits and returns the current
    /// task without persisting any changes or bumping the version.
    #[rstest]
    fn test_noop_request_is_detected_as_merge_safe() {
        let request = UpdateTaskRequest {
            title: None,
            description: None,
            status: None,
            priority: None,
            version: 1,
        };
        assert!(
            can_merge_without_conflict(&request),
            "All-None request should be merge-safe (no-op)"
        );
    }

    /// Verifies that a request with at least one non-None field is NOT a no-op.
    #[rstest]
    fn test_request_with_title_is_not_noop() {
        let request = UpdateTaskRequest {
            title: Some("New Title".to_string()),
            description: None,
            status: None,
            priority: None,
            version: 1,
        };
        assert!(
            !can_merge_without_conflict(&request),
            "Request with title should not be a no-op"
        );
    }

    // -------------------------------------------------------------------------
    // Amortized cleanup interval Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_keyed_update_queue_amortized_cleanup_does_not_run_every_time() {
        let queue = KeyedUpdateQueue::new();

        // Acquire and drop 50 unique task_ids (below CLEANUP_THRESHOLD)
        for _ in 0..50 {
            let task_id = TaskId::generate();
            let _guard = queue.acquire(&task_id).await;
        }

        // Without forced cleanup, entries below threshold should accumulate
        let entry_count = queue.debug_entry_count();
        assert!(
            entry_count > 0,
            "Below threshold, dead entries should not be auto-purged"
        );

        // Force cleanup should remove them
        queue.force_cleanup();
        let entry_count = queue.debug_entry_count();
        assert_eq!(entry_count, 0);
    }

    // -------------------------------------------------------------------------
    // sample_delay Tests (pure function for jitter boundary)
    // -------------------------------------------------------------------------

    #[rstest]
    #[case(0, 0, 0)] // cap=0 always returns 0
    #[case(100, 0, 0)] // cap=0 always returns 0 regardless of rand
    #[case(0, 10, 0)] // rand=0 always returns 0
    #[case(10, 10, 10)] // 10 % 11 = 10
    #[case(11, 10, 0)] // 11 % 11 = 0 (wraps around)
    #[case(u64::MAX, 10, u64::MAX % 11)] // large rand value
    #[case(123, u64::MAX, 123)] // cap=u64::MAX returns rand_value unchanged
    fn test_sample_delay(#[case] rand_value: u64, #[case] cap: u64, #[case] expected: u64) {
        assert_eq!(sample_delay(rand_value, cap), expected);
    }

    #[rstest]
    fn test_sample_delay_always_within_range() {
        // For any random value, result should be in [0, cap]
        let cap = 10u64;
        for rand_value in 0..=100 {
            let result = sample_delay(rand_value, cap);
            assert!(
                result <= cap,
                "sample_delay({rand_value}, {cap}) = {result}, expected <= {cap}"
            );
        }
    }

    #[rstest]
    fn test_sample_delay_cap_zero_always_zero() {
        for rand_value in [0, 1, 100, u64::MAX] {
            assert_eq!(sample_delay(rand_value, 0), 0);
        }
    }

    // -------------------------------------------------------------------------
    // PUT /tasks/{id} status field rejection Tests
    // -------------------------------------------------------------------------

    /// PUT /tasks/{id} should reject requests that include a status field,
    /// directing clients to use PATCH /tasks/{id}/status instead.
    #[rstest]
    fn test_can_merge_without_conflict_with_status_is_not_noop() {
        let request = UpdateTaskRequest {
            title: None,
            description: None,
            status: Some(TaskStatusDto::InProgress),
            priority: None,
            version: 1,
        };
        // status is non-None, so this is not a no-op
        assert!(!can_merge_without_conflict(&request));
    }

    // -------------------------------------------------------------------------
    // Handler Integration Tests (update_task_inner)
    // -------------------------------------------------------------------------

    /// Helper to create an `AppState` with an in-memory repository containing a single task.
    ///
    /// Returns `(AppState, Task)` where the task is already persisted and the `AppState`
    /// is fully initialized with search index built from the persisted task.
    async fn create_test_app_state_with_task() -> (AppState, Task) {
        use crate::infrastructure::{
            InMemoryEventStore, InMemoryProjectRepository, InMemoryTaskRepository, Repositories,
            TaskRepository as _,
        };

        let task_repository = Arc::new(InMemoryTaskRepository::new());
        let task = Task::new(TaskId::generate(), "Test Task", Timestamp::now());
        task_repository.save(&task).await.unwrap();

        let repositories = Repositories {
            task_repository: task_repository
                as Arc<dyn crate::infrastructure::TaskRepository + Send + Sync>,
            project_repository: Arc::new(InMemoryProjectRepository::new())
                as Arc<dyn crate::infrastructure::ProjectRepository + Send + Sync>,
            event_store: Arc::new(InMemoryEventStore::new())
                as Arc<dyn crate::infrastructure::EventStore + Send + Sync>,
        };

        let state = super::super::handlers::AppState::from_repositories(repositories)
            .await
            .unwrap();

        (state, task)
    }

    /// `PUT /tasks/{id}` with a status field should return 400 `UNSUPPORTED_FIELD`.
    ///
    /// This tests the actual handler path through `update_task_inner`, not just
    /// the `can_merge_without_conflict` helper.
    #[rstest]
    #[tokio::test]
    async fn test_update_task_inner_rejects_status_field() {
        let (state, task) = create_test_app_state_with_task().await;

        let request = UpdateTaskRequest {
            title: None,
            description: None,
            status: Some(TaskStatusDto::InProgress),
            priority: None,
            version: task.version,
        };

        let result = update_task_inner(state, task.task_id.clone(), request).await;

        assert!(result.is_err(), "Expected Err for status field in PUT");
        let error = result.unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.error.code, "UNSUPPORTED_FIELD");
    }

    /// `PUT /tasks/{id}` with all-None fields (no-op) should return 200 OK
    /// with unchanged version and `updated_at`.
    ///
    /// This tests the actual handler path through `update_task_inner`, verifying
    /// that a stale no-op request does not bump the version or persist anything.
    #[rstest]
    #[tokio::test]
    async fn test_update_task_inner_noop_returns_ok_with_unchanged_version() {
        let (state, task) = create_test_app_state_with_task().await;
        let original_version = task.version;
        let original_updated_at = task.updated_at.to_string();

        let request = UpdateTaskRequest {
            title: None,
            description: None,
            status: None,
            priority: None,
            version: 999, // Stale version, but irrelevant for no-op
        };

        let result = update_task_inner(state, task.task_id.clone(), request).await;

        assert!(result.is_ok(), "Expected Ok for no-op request");
        let (status_code, json_response) = result.unwrap();
        assert_eq!(status_code, StatusCode::OK);
        assert_eq!(json_response.0.version, original_version);
        assert_eq!(json_response.0.updated_at, original_updated_at);
    }

    // -------------------------------------------------------------------------
    // promote_to_retryable_conflict Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_promote_to_retryable_conflict_converts_version_conflict() {
        let error = ApiErrorResponse::conflict("Expected version 1, found 2");
        let promoted = promote_to_retryable_conflict(error);
        assert_eq!(promoted.status, StatusCode::CONFLICT);
        assert_eq!(promoted.error.code, RETRYABLE_CONFLICT_CODE);
    }

    #[rstest]
    fn test_promote_to_retryable_conflict_leaves_retryable_unchanged() {
        let error = ApiErrorResponse::retryable_conflict("CAS failure");
        let promoted = promote_to_retryable_conflict(error);
        assert_eq!(promoted.error.code, RETRYABLE_CONFLICT_CODE);
    }

    #[rstest]
    fn test_promote_to_retryable_conflict_leaves_non_conflict_unchanged() {
        let error = ApiErrorResponse::not_found("task not found");
        let promoted = promote_to_retryable_conflict(error);
        assert_eq!(promoted.status, StatusCode::NOT_FOUND);
        assert_eq!(promoted.error.code, "NOT_FOUND");
    }

    // -------------------------------------------------------------------------
    // retry_on_conflict on_retry callback Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_retry_on_conflict_on_retry_callback_counts_retries() {
        let (call_count, call_count_inner) = new_call_counter();
        let (retry_count, _) = new_call_counter();
        let retry_count_inner = Arc::clone(&retry_count);

        let _result: Result<u32, ApiErrorResponse> = retry_on_conflict(
            || {
                let count = Arc::clone(&call_count_inner);
                async move {
                    let attempt = count.fetch_add(1, Ordering::SeqCst);
                    if attempt < 2 {
                        Err(ApiErrorResponse::retryable_conflict("CAS failure"))
                    } else {
                        Ok(42)
                    }
                }
            },
            3,
            0,
            move |_attempt| {
                retry_count_inner.fetch_add(1, Ordering::SeqCst);
            },
        )
        .await;

        assert_eq!(call_count.load(Ordering::SeqCst), 3);
        assert_eq!(retry_count.load(Ordering::SeqCst), 2);
    }

    // -------------------------------------------------------------------------
    // update_task_inner retry path integration test
    // -------------------------------------------------------------------------

    /// Tests that `update_task_inner` with a real title update succeeds
    /// and returns the updated task with incremented version.
    #[rstest]
    #[tokio::test]
    async fn test_update_task_inner_title_update_succeeds_with_correct_version() {
        let (state, task) = create_test_app_state_with_task().await;

        let request = UpdateTaskRequest {
            title: Some("Updated Title".to_string()),
            description: None,
            status: None,
            priority: None,
            version: task.version,
        };

        let result = update_task_inner(state, task.task_id.clone(), request).await;

        assert!(result.is_ok(), "Expected Ok for valid title update");
        let (status_code, json_response) = result.unwrap();
        assert_eq!(status_code, StatusCode::OK);
        assert_eq!(json_response.0.title, "Updated Title");
        assert_eq!(json_response.0.version, task.version + 1);
    }

    /// Tests that `update_task_inner` returns 409 (non-retryable) for stale
    /// version on a non-no-op request.
    #[rstest]
    #[tokio::test]
    async fn test_update_task_inner_stale_version_returns_conflict() {
        let (state, task) = create_test_app_state_with_task().await;

        let request = UpdateTaskRequest {
            title: Some("Updated Title".to_string()),
            description: None,
            status: None,
            priority: None,
            version: task.version + 999, // Stale version
        };

        let result = update_task_inner(state, task.task_id.clone(), request).await;

        assert!(result.is_err(), "Expected Err for stale version");
        let error = result.unwrap_err();
        assert_eq!(error.status, StatusCode::CONFLICT);
        assert_eq!(error.error.code, "VERSION_CONFLICT");
    }

    // -------------------------------------------------------------------------
    // should_count_retry_exhausted truth table Tests
    // -------------------------------------------------------------------------

    /// Truth table test: `should_count_retry_exhausted` returns `true` only when
    /// the result is an error AND `max_retries` > 0 AND `retries_used` >= `max_retries`.
    #[rstest]
    #[case(true, 1, 1, true)] // err + used==max -> exhausted
    #[case(true, 2, 1, true)] // err + used>max  -> exhausted
    #[case(true, 0, 1, false)] // err + no retries -> not exhausted
    #[case(false, 1, 1, false)] // ok  + used==max  -> not exhausted (success)
    #[case(false, 0, 1, false)] // ok  + no retries -> not exhausted
    #[case(true, 0, 0, false)] // err + max=0 (retry disabled) -> not exhausted
    fn test_should_count_retry_exhausted(
        #[case] result_is_err: bool,
        #[case] retries_used: usize,
        #[case] max_retries: u8,
        #[case] expected: bool,
    ) {
        assert_eq!(
            should_count_retry_exhausted(result_is_err, retries_used, max_retries),
            expected
        );
    }

    // -------------------------------------------------------------------------
    // build_events_from_changes u64::MAX saturation Tests
    // -------------------------------------------------------------------------

    /// Verifies that `build_events_from_changes` saturates at `u64::MAX` instead
    /// of panicking when `current_version` is already at the maximum value.
    #[rstest]
    fn test_build_events_from_changes_saturates_at_u64_max() {
        let task_id = TaskId::generate();
        let timestamp = Timestamp::now();
        let changes = vec![
            DetectedChange::TitleUpdated {
                old_title: "old".to_string(),
                new_title: "new".to_string(),
            },
            DetectedChange::PriorityChanged {
                old_priority: Priority::Low,
                new_priority: Priority::High,
            },
        ];
        let events = build_events_from_changes(&task_id, &changes, &timestamp, u64::MAX);
        assert_eq!(events.len(), 2);
        // Both events should saturate at u64::MAX, not panic
        assert_eq!(events[0].version, u64::MAX);
        assert_eq!(events[1].version, u64::MAX);
    }

    // -------------------------------------------------------------------------
    // ConflictKind / classify_conflict_kind Tests (IMPL-PRB1-001-001)
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_classify_conflict_kind_stale_version() {
        let error = ApiErrorResponse::conflict("Expected version 1, found 2");
        assert_eq!(classify_conflict_kind(&error), ConflictKind::StaleVersion);
    }

    #[rstest]
    fn test_classify_conflict_kind_retryable_cas() {
        let error = ApiErrorResponse::retryable_conflict("CAS failure");
        assert_eq!(classify_conflict_kind(&error), ConflictKind::RetryableCas);
    }

    #[rstest]
    fn test_classify_conflict_kind_other_409() {
        // A 409 with an unknown error code
        let error = ApiErrorResponse::new(
            StatusCode::CONFLICT,
            ApiError::new("UNKNOWN_CONFLICT", "something else"),
        );
        assert_eq!(classify_conflict_kind(&error), ConflictKind::Other);
    }

    #[rstest]
    fn test_classify_conflict_kind_non_409() {
        let error = ApiErrorResponse::not_found("task not found");
        assert_eq!(classify_conflict_kind(&error), ConflictKind::Other);
    }

    #[rstest]
    fn test_is_stale_version_conflict_true() {
        let error = ApiErrorResponse::conflict("Expected version 1, found 2");
        assert!(is_stale_version_conflict(&error));
    }

    #[rstest]
    fn test_is_stale_version_conflict_false_for_retryable() {
        let error = ApiErrorResponse::retryable_conflict("CAS failure");
        assert!(!is_stale_version_conflict(&error));
    }

    #[rstest]
    fn test_is_stale_version_conflict_false_for_non_409() {
        let error = ApiErrorResponse::not_found("task not found");
        assert!(!is_stale_version_conflict(&error));
    }

    /// is_retryable_conflict should be a wrapper around classify_conflict_kind
    #[rstest]
    fn test_is_retryable_conflict_consistent_with_classify() {
        let retryable = ApiErrorResponse::retryable_conflict("CAS failure");
        let stale = ApiErrorResponse::conflict("Expected version 1, found 2");
        let not_found = ApiErrorResponse::not_found("not found");

        assert_eq!(
            is_retryable_conflict(&retryable),
            classify_conflict_kind(&retryable) == ConflictKind::RetryableCas
        );
        assert_eq!(
            is_retryable_conflict(&stale),
            classify_conflict_kind(&stale) == ConflictKind::RetryableCas
        );
        assert_eq!(
            is_retryable_conflict(&not_found),
            classify_conflict_kind(&not_found) == ConflictKind::RetryableCas
        );
    }

    // -------------------------------------------------------------------------
    // RebaseError / rebase_update_request Tests (IMPL-PRB1-001-002)
    // -------------------------------------------------------------------------

    /// Helper: creates a Task with given fields for rebase testing.
    fn make_task_for_rebase(
        task_id: TaskId,
        title: &str,
        description: Option<&str>,
        priority: Priority,
        version: u64,
    ) -> Task {
        let mut task = Task::new(task_id, title, Timestamp::now());
        task.version = version;
        task.priority = priority;
        if let Some(desc) = description {
            task = task.with_description(desc);
        }
        task
    }

    /// Rebase succeeds when another user changed a different field.
    #[rstest]
    fn test_rebase_update_request_success_different_fields() {
        let task_id = TaskId::generate();
        let original_base = make_task_for_rebase(
            task_id.clone(),
            "Original Title",
            Some("Original Desc"),
            Priority::Low,
            1,
        );
        // Another user changed description (version bumped to 2)
        let latest = make_task_for_rebase(
            task_id,
            "Original Title",
            Some("Changed by another user"),
            Priority::Low,
            2,
        );
        // Our request changes title only
        let request = UpdateTaskRequest {
            title: Some("My New Title".to_string()),
            description: None,
            status: None,
            priority: None,
            version: 1,
        };

        let result = rebase_update_request(&original_base, &latest, &request);
        assert!(result.is_ok());
        let rebased = result.unwrap();
        assert_eq!(rebased.title, Some("My New Title".to_string()));
        assert_eq!(rebased.version, 2); // version rebased to latest
    }

    /// Rebase succeeds when another user changed to the same value (no-op change).
    #[rstest]
    fn test_rebase_update_request_success_same_value_update() {
        let task_id = TaskId::generate();
        let original_base = make_task_for_rebase(
            task_id.clone(),
            "Title",
            None,
            Priority::Low,
            1,
        );
        // Another user also set title to "Title" (same value)
        let latest = make_task_for_rebase(
            task_id,
            "Title",
            None,
            Priority::Low,
            2,
        );
        // Our request changes title
        let request = UpdateTaskRequest {
            title: Some("New Title".to_string()),
            description: None,
            status: None,
            priority: None,
            version: 1,
        };

        let result = rebase_update_request(&original_base, &latest, &request);
        assert!(result.is_ok());
        let rebased = result.unwrap();
        assert_eq!(rebased.title, Some("New Title".to_string()));
        assert_eq!(rebased.version, 2);
    }

    /// Rebase fails when both users changed the same field (title) to different values.
    #[rstest]
    fn test_rebase_update_request_fails_title_conflict() {
        let task_id = TaskId::generate();
        let original_base = make_task_for_rebase(
            task_id.clone(),
            "Original Title",
            None,
            Priority::Low,
            1,
        );
        // Another user changed title
        let latest = make_task_for_rebase(
            task_id,
            "Changed by another user",
            None,
            Priority::Low,
            2,
        );
        // Our request also changes title
        let request = UpdateTaskRequest {
            title: Some("My Title".to_string()),
            description: None,
            status: None,
            priority: None,
            version: 1,
        };

        let result = rebase_update_request(&original_base, &latest, &request);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            RebaseError::NonCommutativeConflict { field: "title" }
        );
    }

    /// Rebase fails when both users changed description.
    #[rstest]
    fn test_rebase_update_request_fails_description_conflict() {
        let task_id = TaskId::generate();
        let original_base = make_task_for_rebase(
            task_id.clone(),
            "Title",
            Some("Original Desc"),
            Priority::Low,
            1,
        );
        // Another user changed description
        let latest = make_task_for_rebase(
            task_id,
            "Title",
            Some("Another user desc"),
            Priority::Low,
            2,
        );
        // Our request also changes description
        let request = UpdateTaskRequest {
            title: None,
            description: Some("My Desc".to_string()),
            status: None,
            priority: None,
            version: 1,
        };

        let result = rebase_update_request(&original_base, &latest, &request);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            RebaseError::NonCommutativeConflict { field: "description" }
        );
    }

    /// Rebase fails when both users changed priority.
    #[rstest]
    fn test_rebase_update_request_fails_priority_conflict() {
        let task_id = TaskId::generate();
        let original_base = make_task_for_rebase(
            task_id.clone(),
            "Title",
            None,
            Priority::Low,
            1,
        );
        // Another user changed priority
        let latest = make_task_for_rebase(
            task_id,
            "Title",
            None,
            Priority::High,
            2,
        );
        // Our request also changes priority
        let request = UpdateTaskRequest {
            title: None,
            description: None,
            status: None,
            priority: Some(crate::api::dto::PriorityDto::Critical),
            version: 1,
        };

        let result = rebase_update_request(&original_base, &latest, &request);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            RebaseError::NonCommutativeConflict { field: "priority" }
        );
    }

    // -------------------------------------------------------------------------
    // Read-Repair Integration Tests (IMPL-PRB1-001-003)
    // -------------------------------------------------------------------------

    /// Read-repair constants should be defined.
    #[rstest]
    fn test_read_repair_constants() {
        assert_eq!(READ_REPAIR_MAX_RETRIES, 3);
        assert_eq!(READ_REPAIR_BASE_DELAY_MS, 2);
    }

    /// Read-repair success: client sends stale version, but changes a
    /// different field from the concurrent update, so rebase succeeds.
    #[rstest]
    #[tokio::test]
    async fn test_read_repair_success_different_field_update() {
        use crate::infrastructure::TaskRepository as _;

        let (state, task) = create_test_app_state_with_task().await;

        // Simulate another user updating description (bumps version to 2)
        let updated_by_other = Task {
            description: Some("Updated by another user".to_string()),
            version: 2,
            updated_at: Timestamp::now(),
            ..task.clone()
        };
        state.task_repository.save(&updated_by_other).await.unwrap();

        // Our request: update title with stale version 1
        let request = UpdateTaskRequest {
            title: Some("My New Title".to_string()),
            description: None,
            status: None,
            priority: None,
            version: 1, // stale: DB is now at version 2
        };

        // Call update_task_inner first - it should return stale-version 409
        let first_result =
            update_task_inner(state.clone(), task.task_id.clone(), request.clone()).await;
        assert!(first_result.is_err());
        assert!(is_stale_version_conflict(&first_result.unwrap_err()));

        // Now call the full update_task_with_read_repair - it should succeed via rebase
        let result = update_task_with_read_repair(
            &state,
            &task.task_id,
            &request,
        )
        .await;

        assert!(result.is_ok(), "Read-repair should succeed: {result:?}");
        let (status_code, json_response) = result.unwrap();
        assert_eq!(status_code, StatusCode::OK);
        assert_eq!(json_response.0.title, "My New Title");
        assert_eq!(json_response.0.version, 3); // version 2 + 1
    }

    /// Read-repair failure: client sends stale version, and the concurrent
    /// update changed the same field (title), causing NonCommutativeConflict.
    #[rstest]
    #[tokio::test]
    async fn test_read_repair_failure_non_commutative_conflict() {
        use crate::infrastructure::TaskRepository as _;

        let (state, task) = create_test_app_state_with_task().await;

        // Simulate another user updating title (bumps version to 2)
        let updated_by_other = Task {
            title: "Changed by another user".to_string(),
            version: 2,
            updated_at: Timestamp::now(),
            ..task.clone()
        };
        state.task_repository.save(&updated_by_other).await.unwrap();

        // Our request: also update title with stale version 1
        let request = UpdateTaskRequest {
            title: Some("My Title".to_string()),
            description: None,
            status: None,
            priority: None,
            version: 1, // stale: DB is now at version 2
        };

        let result = update_task_with_read_repair(
            &state,
            &task.task_id,
            &request,
        )
        .await;

        // Should fail with 409 (non-commutative conflict converted to 409)
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.status, StatusCode::CONFLICT);
    }

    /// Read-repair converts RebaseError to 409 Conflict response.
    #[rstest]
    fn test_rebase_error_display() {
        let error = RebaseError::NonCommutativeConflict { field: "title" };
        let display = error.to_string();
        assert!(display.contains("title"));
        assert!(display.contains("Non-commutative conflict"));
    }
}
