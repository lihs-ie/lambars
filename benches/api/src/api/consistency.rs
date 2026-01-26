//! Consistency management for task operations with event sourcing.
//!
//! This module provides types and functions for handling consistency between
//! task persistence and event sourcing. It implements a best-effort approach
//! with proper error tracking and logging.
//!
//! # Design Principles
//!
//! - **Pure function generation**: Error construction is pure (no I/O)
//! - **I/O boundary separation**: Logging is performed at I/O boundaries
//! - **Security**: Internal details are never exposed to clients
//! - **Type safety**: Invariants are enforced through constructors

use thiserror::Error;

use super::error::ApiErrorResponse;
use super::handlers::AppState;
use crate::domain::{Task, TaskEvent, TaskId};

// =============================================================================
// ConsistencyError Build Error
// =============================================================================

/// Error returned when constructing a `ConsistencyError` with invalid parameters.
#[derive(Debug, Error, Clone)]
pub enum ConsistencyErrorBuildError {
    /// Invalid parameters for incomplete write: written must be < total, and total > 0.
    #[error("Invalid incomplete write: written ({written}) >= total ({total}) or total == 0")]
    InvalidIncompleteWrite {
        /// Number of events reported as written.
        written: usize,
        /// Total number of events.
        total: usize,
    },
}

// =============================================================================
// ConsistencyError
// =============================================================================

/// Consistency error representing partial or failed event synchronization.
///
/// # Design Principles
///
/// - `#[non_exhaustive]` prevents external crates from direct construction
/// - Variant fields are non-public; use constructors for invariant guarantees
/// - Constructors are pure functions (no logging or I/O)
/// - Logging is performed via `log_consistency_error()` at I/O boundaries
/// - `incomplete_write` validates invariants and returns `Result` for violations
///
/// # Security Considerations
///
/// - Client-facing information is minimal via `client_message()`
/// - Internal details (DB errors, stack traces) are kept in `internal_details`
/// - `Display` outputs only `client_message()` to prevent accidental leakage
/// - Custom `Debug` implementation redacts `internal_details`
/// - `internal_details()` is `pub(crate)` to prevent external access
///
/// # Client Exposure Policy
///
/// - `task_id`: Safe to expose (provided by client in request)
/// - `written/total`: Safe to expose (progress information)
/// - `internal_details`: Never exposed (redacted in Debug/Display)
///
/// # Serialization
///
/// `Serialize` is intentionally not derived to prevent accidental JSON exposure.
/// Use `client_message()` for response construction.
#[derive(Clone)]
#[non_exhaustive]
pub enum ConsistencyError {
    /// Event write failed for a single event.
    ///
    /// Constructed via `event_write_failed()`.
    #[non_exhaustive]
    EventWriteFailed {
        /// The task ID for which the event write failed.
        task_id: TaskId,
        /// Internal error details for logging (not exposed to clients).
        internal_details: String,
    },

    /// Event write was incomplete (partial success).
    ///
    /// Constructed via `incomplete_write()` with invariant validation.
    /// Invariant: `total > 0 && written < total`
    #[non_exhaustive]
    IncompleteWrite {
        /// The task ID for which the write was incomplete.
        task_id: TaskId,
        /// Number of events successfully written (0 <= written < total).
        written: usize,
        /// Total number of events to write (> 0).
        total: usize,
        /// Internal error details for logging (not exposed to clients).
        internal_details: String,
    },
}

// Custom Debug implementation that redacts internal_details
impl std::fmt::Debug for ConsistencyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EventWriteFailed { task_id, .. } => formatter
                .debug_struct("EventWriteFailed")
                .field("task_id", task_id)
                .field("internal_details", &"<redacted>")
                .finish(),
            Self::IncompleteWrite {
                task_id,
                written,
                total,
                ..
            } => formatter
                .debug_struct("IncompleteWrite")
                .field("task_id", task_id)
                .field("written", written)
                .field("total", total)
                .field("internal_details", &"<redacted>")
                .finish(),
        }
    }
}

impl ConsistencyError {
    /// Constructs an event write failed error (pure function).
    ///
    /// **Note**: This function does not perform logging.
    /// Call `log_consistency_error()` at the I/O boundary.
    pub fn event_write_failed(task_id: &TaskId, internal_error: impl std::fmt::Display) -> Self {
        Self::EventWriteFailed {
            task_id: task_id.clone(),
            internal_details: internal_error.to_string(),
        }
    }

    /// Constructs an incomplete write error (pure function) with invariant validation.
    ///
    /// # Invariants
    ///
    /// - `total > 0`: At least one event was being written
    /// - `written < total`: Not all events were written (otherwise it's a success)
    ///
    /// # Errors
    ///
    /// Returns `Err(ConsistencyErrorBuildError::InvalidIncompleteWrite)` if invariants
    /// are violated.
    ///
    /// **Note**: This function does not perform logging.
    /// Call `log_consistency_error()` at the I/O boundary.
    pub fn incomplete_write(
        task_id: &TaskId,
        written: usize,
        total: usize,
        internal_error: impl std::fmt::Display,
    ) -> Result<Self, ConsistencyErrorBuildError> {
        // Invariant: total > 0 && written < total
        if total == 0 || written >= total {
            return Err(ConsistencyErrorBuildError::InvalidIncompleteWrite { written, total });
        }
        Ok(Self::IncompleteWrite {
            task_id: task_id.clone(),
            written,
            total,
            internal_details: internal_error.to_string(),
        })
    }

    /// Returns the task ID associated with this error.
    #[must_use]
    pub const fn task_id(&self) -> &TaskId {
        match self {
            Self::EventWriteFailed { task_id, .. } | Self::IncompleteWrite { task_id, .. } => {
                task_id
            }
        }
    }

    /// Returns internal error details (for logging, not client exposure).
    ///
    /// **Visibility**: `pub(crate)` to prevent external access.
    /// Use `log_consistency_error()` for logging at I/O boundaries.
    #[must_use]
    #[allow(dead_code)] // Used in tests and future metrics recording
    pub(crate) fn internal_details(&self) -> &str {
        match self {
            Self::EventWriteFailed {
                internal_details, ..
            }
            | Self::IncompleteWrite {
                internal_details, ..
            } => internal_details,
        }
    }

    /// Returns a client-safe message (no internal details).
    ///
    /// This is the only method that should be used when constructing
    /// responses to clients.
    #[must_use]
    pub fn client_message(&self) -> String {
        match self {
            Self::EventWriteFailed { task_id, .. } => {
                format!("Event synchronization pending for task {task_id}")
            }
            Self::IncompleteWrite {
                task_id,
                written,
                total,
                ..
            } => {
                format!(
                    "Partial event synchronization for task {task_id}: {written}/{total} events recorded"
                )
            }
        }
    }
}

impl std::fmt::Display for ConsistencyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Display outputs client_message() to prevent accidental leakage
        write!(formatter, "{}", self.client_message())
    }
}

impl std::error::Error for ConsistencyError {}

// =============================================================================
// Logging Function (I/O Boundary)
// =============================================================================

/// Logs a consistency error at the I/O boundary.
///
/// This function is the designated place for logging consistency errors.
/// It safely accesses internal details for structured logging.
///
/// # Design
///
/// - Constructors are pure (no logging)
/// - This function performs the I/O (logging)
/// - Called at handler level (I/O boundary)
pub fn log_consistency_error(error: &ConsistencyError) {
    match error {
        ConsistencyError::EventWriteFailed {
            task_id,
            internal_details,
        } => {
            tracing::warn!(
                task_id = %task_id,
                internal_error = %internal_details,
                "Event write failed - consistency error"
            );
        }
        ConsistencyError::IncompleteWrite {
            task_id,
            written,
            total,
            internal_details,
        } => {
            tracing::warn!(
                task_id = %task_id,
                written = written,
                total = total,
                internal_error = %internal_details,
                "Partial event write - consistency error"
            );
        }
    }
}

// =============================================================================
// SaveTaskResult
// =============================================================================

/// Result of a task save operation with event sourcing.
///
/// This type represents the outcome of saving a task along with its event:
/// - `task_saved`: Whether the task was successfully persisted
/// - `consistency_warning`: Optional warning if event write failed
///
/// # Usage Pattern
///
/// ```ignore
/// let result = save_task_with_event(&state, &task, event, 0).await?;
///
/// if let Some(warning) = &result.consistency_warning {
///     response.warnings.push(warning.client_message());
/// }
/// ```
#[derive(Debug)]
pub struct SaveTaskResult {
    /// Whether the task was saved successfully.
    pub task_saved: bool,
    /// Consistency warning if event write failed (task still saved).
    pub consistency_warning: Option<ConsistencyError>,
}

impl SaveTaskResult {
    /// Creates a successful result (both task and event saved).
    #[must_use]
    pub const fn success() -> Self {
        Self {
            task_saved: true,
            consistency_warning: None,
        }
    }

    /// Creates a partial success result (task saved, event failed).
    #[must_use]
    pub const fn partial_success(warning: ConsistencyError) -> Self {
        Self {
            task_saved: true,
            consistency_warning: Some(warning),
        }
    }

    /// Returns true if both task and event were saved successfully.
    #[must_use]
    pub const fn is_complete_success(&self) -> bool {
        self.task_saved && self.consistency_warning.is_none()
    }
}

// =============================================================================
// save_task_with_event Function
// =============================================================================

/// Saves a task and writes the corresponding event (best-effort).
///
/// This function implements the best-effort consistency strategy:
/// 1. Save the task to the repository (fails fast on error)
/// 2. Write the event to the event store (failure becomes a warning)
///
/// # Return Value Semantics
///
/// - `Ok(SaveTaskResult::success())`: Both task and event saved
/// - `Ok(SaveTaskResult::partial_success(warning))`: Task saved, event failed
/// - `Err(ApiErrorResponse)`: Task save failed (event not attempted)
///
/// # Error Observability
///
/// Event write failures are:
/// - Logged via `log_consistency_error()` (structured logging)
/// - Returned as `consistency_warning` for response inclusion
/// - Available for metrics recording at call site
///
/// # Design Trade-offs
///
/// - No Outbox pattern: Simpler implementation, acceptable for benchmarking
/// - No Saga: Cross-store transactions not needed for this use case
/// - Best-effort: Event writes can be verified via `load_events()` later
///
/// # Arguments
///
/// * `state` - Application state containing repositories
/// * `task` - The task to save
/// * `event` - The event to write
/// * `expected_version` - Expected version for optimistic locking (0 for new tasks)
///
/// # Errors
///
/// Returns `ApiErrorResponse` if task save fails (database error, etc.).
pub async fn save_task_with_event(
    state: &AppState,
    task: &Task,
    event: TaskEvent,
    expected_version: u64,
) -> Result<SaveTaskResult, ApiErrorResponse> {
    // 1. Save task (fail fast on error)
    state
        .task_repository
        .save(task)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    // 2. Write event (failure becomes warning)
    match state
        .event_store
        .append(&event, expected_version)
        .run_async()
        .await
    {
        Ok(()) => Ok(SaveTaskResult::success()),
        Err(error) => {
            // Construct warning (pure function)
            let warning = ConsistencyError::event_write_failed(&task.task_id, error);

            // Log at I/O boundary
            log_consistency_error(&warning);

            // Return partial success with warning
            Ok(SaveTaskResult::partial_success(warning))
        }
    }
}

// =============================================================================
// Multiple Event Write Support
// =============================================================================

/// Result of writing multiple events sequentially.
///
/// This struct tracks the progress of writing multiple events to the event store,
/// distinguishing between full success and partial writes.
#[derive(Debug)]
pub struct MultipleEventWriteResult {
    /// Number of events successfully written.
    pub written: usize,
    /// Total number of events attempted.
    pub total: usize,
    /// Consistency warning if partial write occurred.
    pub warning: Option<ConsistencyError>,
}

impl MultipleEventWriteResult {
    /// Returns true if all events were written successfully.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.written == self.total && self.warning.is_none()
    }

    /// Creates a result representing full success.
    #[must_use]
    pub const fn success(count: usize) -> Self {
        Self {
            written: count,
            total: count,
            warning: None,
        }
    }

    /// Creates a result representing no events to write.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            written: 0,
            total: 0,
            warning: None,
        }
    }
}

/// Writes multiple events sequentially with version tracking.
///
/// This function writes events one by one, incrementing the version for each.
/// If any write fails, it stops and returns the partial result with a warning.
///
/// # Version Semantics
///
/// - `initial_version` is obtained from `get_current_version()` before calling
/// - Each event uses version `initial_version + 1 + index`
/// - The `expected_version` for `append` is `initial_version + index`
///
/// # Empty Event List
///
/// If `events` is empty, returns immediately with `MultipleEventWriteResult::empty()`.
///
/// # Arguments
///
/// * `state` - Application state containing the event store
/// * `task_id` - Task ID for error reporting
/// * `events` - Events to write (should have correct versions already set)
/// * `initial_version` - Current version before writing (for optimistic locking)
///
/// # Panics
///
/// This function uses `.expect()` on `ConsistencyError::incomplete_write` which
/// should never panic under normal conditions because:
/// - `total > 0` is guaranteed (checked before the loop)
/// - `written < total` is guaranteed (loop terminates on first error)
///
/// If this panic occurs, it indicates a programming error in the invariant logic.
///
/// # I/O Boundary Notes
///
/// This function performs I/O operations (event writes) and should be called
/// within an async handler. The `mut written` counter is a local mutable state
/// that's acceptable at the I/O boundary for efficiency.
pub async fn write_events_sequentially(
    state: &AppState,
    task_id: &TaskId,
    events: Vec<TaskEvent>,
    initial_version: u64,
) -> MultipleEventWriteResult {
    let total = events.len();

    // Empty list: nothing to write
    if total == 0 {
        return MultipleEventWriteResult::empty();
    }

    // I/O boundary: local mutable counter for efficiency
    let mut written = 0;

    for (index, event) in events.into_iter().enumerate() {
        let expected_version = initial_version + index as u64;

        match state
            .event_store
            .append(&event, expected_version)
            .run_async()
            .await
        {
            Ok(()) => {
                written += 1;
            }
            Err(error) => {
                // Construct warning using incomplete_write
                // Invariant: total > 0 (checked above), written < total (loop not complete)
                let warning = ConsistencyError::incomplete_write(task_id, written, total, error)
                    .expect("invariant: total > 0 && written < total");

                // Log at I/O boundary
                log_consistency_error(&warning);

                return MultipleEventWriteResult {
                    written,
                    total,
                    warning: Some(warning),
                };
            }
        }
    }

    MultipleEventWriteResult::success(total)
}

/// Saves a task and writes multiple events (best-effort).
///
/// Similar to `save_task_with_event` but for multiple events.
/// Task save fails fast; event write failures become warnings.
///
/// # Arguments
///
/// * `state` - Application state
/// * `task` - The task to save
/// * `events` - Events to write (already with correct versions)
/// * `initial_version` - Current event version before writing
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] if the task save fails. Event write failures
/// do not cause an error; they are returned as warnings in the result.
pub async fn save_task_with_events(
    state: &AppState,
    task: &Task,
    events: Vec<TaskEvent>,
    initial_version: u64,
) -> Result<MultipleEventWriteResult, ApiErrorResponse> {
    // 1. Save task (fail fast)
    state
        .task_repository
        .save(task)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    // 2. Write events sequentially
    let result = write_events_sequentially(state, &task.task_id, events, initial_version).await;

    Ok(result)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // ConsistencyError Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_event_write_failed_construction() {
        let task_id = TaskId::generate();
        let error = ConsistencyError::event_write_failed(&task_id, "database connection lost");

        assert_eq!(error.task_id(), &task_id);
        assert_eq!(error.internal_details(), "database connection lost");
    }

    #[rstest]
    fn test_event_write_failed_client_message() {
        let task_id = TaskId::generate();
        let error = ConsistencyError::event_write_failed(&task_id, "secret error");

        let message = error.client_message();
        assert!(message.contains(&task_id.to_string()));
        assert!(message.contains("Event synchronization pending"));
        assert!(!message.contains("secret error"));
    }

    #[rstest]
    fn test_incomplete_write_valid_construction() {
        let task_id = TaskId::generate();
        let result = ConsistencyError::incomplete_write(&task_id, 2, 5, "network error");

        assert!(result.is_ok());
        let error = result.unwrap();
        if let ConsistencyError::IncompleteWrite { written, total, .. } = error {
            assert_eq!(written, 2);
            assert_eq!(total, 5);
        } else {
            panic!("Expected IncompleteWrite variant");
        }
    }

    #[rstest]
    fn test_incomplete_write_rejects_total_zero() {
        let task_id = TaskId::generate();
        let result = ConsistencyError::incomplete_write(&task_id, 0, 0, "error");

        assert!(matches!(
            result,
            Err(ConsistencyErrorBuildError::InvalidIncompleteWrite {
                written: 0,
                total: 0
            })
        ));
    }

    #[rstest]
    fn test_incomplete_write_rejects_written_equals_total() {
        let task_id = TaskId::generate();
        let result = ConsistencyError::incomplete_write(&task_id, 3, 3, "error");

        assert!(matches!(
            result,
            Err(ConsistencyErrorBuildError::InvalidIncompleteWrite {
                written: 3,
                total: 3
            })
        ));
    }

    #[rstest]
    fn test_incomplete_write_rejects_written_greater_than_total() {
        let task_id = TaskId::generate();
        let result = ConsistencyError::incomplete_write(&task_id, 5, 3, "error");

        assert!(matches!(
            result,
            Err(ConsistencyErrorBuildError::InvalidIncompleteWrite {
                written: 5,
                total: 3
            })
        ));
    }

    #[rstest]
    fn test_incomplete_write_accepts_written_zero() {
        let task_id = TaskId::generate();
        // written = 0 is valid (no events written out of total > 0)
        let result = ConsistencyError::incomplete_write(&task_id, 0, 5, "error");

        assert!(result.is_ok());
    }

    #[rstest]
    fn test_incomplete_write_client_message() {
        let task_id = TaskId::generate();
        let error = ConsistencyError::incomplete_write(&task_id, 2, 5, "secret error").unwrap();

        let message = error.client_message();
        assert!(message.contains(&task_id.to_string()));
        assert!(message.contains("2/5"));
        assert!(message.contains("Partial event synchronization"));
        assert!(!message.contains("secret error"));
    }

    // -------------------------------------------------------------------------
    // Debug Redaction Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_debug_redacts_internal_details_event_write_failed() {
        let task_id = TaskId::generate();
        let error = ConsistencyError::event_write_failed(&task_id, "secret database error");

        let debug_output = format!("{error:?}");

        assert!(!debug_output.contains("secret database error"));
        assert!(debug_output.contains("<redacted>"));
        assert!(debug_output.contains("EventWriteFailed"));
    }

    #[rstest]
    fn test_debug_redacts_internal_details_incomplete_write() {
        let task_id = TaskId::generate();
        let error = ConsistencyError::incomplete_write(&task_id, 1, 3, "secret network error")
            .expect("valid params");

        let debug_output = format!("{error:?}");

        assert!(!debug_output.contains("secret network error"));
        assert!(debug_output.contains("<redacted>"));
        assert!(debug_output.contains("IncompleteWrite"));
        assert!(debug_output.contains("written: 1"));
        assert!(debug_output.contains("total: 3"));
    }

    // -------------------------------------------------------------------------
    // Display Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_display_matches_client_message() {
        let task_id = TaskId::generate();
        let error = ConsistencyError::event_write_failed(&task_id, "internal error");

        let display_output = format!("{error}");
        let client_message = error.client_message();

        assert_eq!(display_output, client_message);
    }

    // -------------------------------------------------------------------------
    // SaveTaskResult Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_save_task_result_success() {
        let result = SaveTaskResult::success();

        assert!(result.task_saved);
        assert!(result.consistency_warning.is_none());
        assert!(result.is_complete_success());
    }

    #[rstest]
    fn test_save_task_result_partial_success() {
        let task_id = TaskId::generate();
        let warning = ConsistencyError::event_write_failed(&task_id, "error");
        let result = SaveTaskResult::partial_success(warning);

        assert!(result.task_saved);
        assert!(result.consistency_warning.is_some());
        assert!(!result.is_complete_success());
    }
}
