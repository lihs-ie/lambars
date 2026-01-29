//! Demo endpoints for development and testing.
//!
//! This module contains demo handlers that generate mock history
//! without using the real `EventStore`.
//! These endpoints are conditionally compiled with the `demo` feature flag.
//!
//! # Usage
//!
//! Enable demo endpoints:
//! ```bash
//! cargo build --features demo
//! ENABLE_DEMO_ENDPOINTS=true cargo run --features demo
//! ```
//!
//! # Endpoints
//!
//! - `GET /demo/tasks/{id}/history`: Mock task event history

use axum::extract::{Path, Query, State};

use super::json_buffer::JsonResponse;
use lambars::control::Continuation;
use lambars::persistent::PersistentList;

use super::advanced::{HistoryQuery, TaskEvent, TaskHistoryResponse};
use super::dto::{PriorityDto, TaskStatusDto};
use super::error::ApiErrorResponse;
use super::handlers::AppState;
use crate::domain::{Priority, Task, TaskStatus, Timestamp};

// =============================================================================
// Constants
// =============================================================================

/// Maximum number of events per page for history endpoint.
const MAX_HISTORY_LIMIT: usize = 100;

/// Default number of events per page.
const DEFAULT_HISTORY_LIMIT: usize = 20;

// =============================================================================
// Helper Functions
// =============================================================================

/// Parses and validates a task ID string.
fn parse_task_id(task_id: &str) -> Result<crate::domain::TaskId, ApiErrorResponse> {
    uuid::Uuid::parse_str(task_id)
        .map(crate::domain::TaskId::from_uuid)
        .map_err(|_| ApiErrorResponse::bad_request("INVALID_TASK_ID", "Invalid task ID format"))
}

// =============================================================================
// Mock History Generation
// =============================================================================

/// Internal paginated result type.
#[derive(Debug, Clone)]
struct PaginatedResult<T> {
    items: Vec<T>,
    next_cursor: Option<usize>,
    has_more: bool,
    total: usize,
}

/// Pure: Core pagination logic.
fn paginate_items<T: Clone>(items: &[T], offset: usize, limit: usize) -> PaginatedResult<T> {
    let total = items.len();
    let paginated: Vec<T> = items.iter().skip(offset).take(limit).cloned().collect();
    let next_offset = offset + paginated.len();
    let has_more = next_offset < total;

    PaginatedResult {
        items: paginated,
        next_cursor: if has_more { Some(next_offset) } else { None },
        has_more,
        total,
    }
}

/// Pure: Paginates items using Continuation monad.
fn paginate_with_continuation<T: Clone + 'static>(
    items: &[T],
    offset: usize,
    limit: usize,
) -> Continuation<PaginatedResult<T>, PaginatedResult<T>> {
    Continuation::pure(paginate_items(items, offset, limit))
}

/// Builds mock task history using `PersistentList`.
///
/// This demonstrates efficient prepend operations with `PersistentList`.
/// Events are generated based on task state to simulate a realistic history.
///
/// # Arguments
///
/// * `task` - The task to generate history for
/// * `base_timestamp` - The base timestamp for events
///
/// # Returns
///
/// A `PersistentList` of `TaskEvent` representing the mock history
pub fn build_mock_history(task: &Task, base_timestamp: &Timestamp) -> PersistentList<TaskEvent> {
    let mut events = PersistentList::new();
    let base_time = base_timestamp.to_string();

    // Created event (always present)
    events = events.cons(TaskEvent::Created {
        timestamp: base_time.clone(),
        title: task.title.clone(),
    });

    // Status change events based on current status
    match task.status {
        TaskStatus::InProgress => {
            events = events.cons(TaskEvent::StatusChanged {
                timestamp: base_time.clone(),
                old_status: TaskStatusDto::Pending,
                new_status: TaskStatusDto::InProgress,
            });
        }
        TaskStatus::Completed => {
            events = events.cons(TaskEvent::StatusChanged {
                timestamp: base_time.clone(),
                old_status: TaskStatusDto::Pending,
                new_status: TaskStatusDto::InProgress,
            });
            events = events.cons(TaskEvent::StatusChanged {
                timestamp: base_time.clone(),
                old_status: TaskStatusDto::InProgress,
                new_status: TaskStatusDto::Completed,
            });
        }
        TaskStatus::Cancelled => {
            events = events.cons(TaskEvent::StatusChanged {
                timestamp: base_time.clone(),
                old_status: TaskStatusDto::Pending,
                new_status: TaskStatusDto::Cancelled,
            });
        }
        TaskStatus::Pending => {}
    }

    // Priority change if not default
    if task.priority != Priority::Low {
        events = events.cons(TaskEvent::PriorityChanged {
            timestamp: base_time.clone(),
            old_priority: PriorityDto::Low,
            new_priority: PriorityDto::from(task.priority),
        });
    }

    // Tag events
    for tag in &task.tags {
        events = events.cons(TaskEvent::TagAdded {
            timestamp: base_time.clone(),
            tag: tag.as_str().to_string(),
        });
    }

    events
}

// =============================================================================
// Demo Handlers
// =============================================================================

/// Demo: Gets task history with mock data.
///
/// This endpoint generates mock history based on the current task state.
/// Use this for development and testing without real `EventStore`.
///
/// # Endpoint
///
/// `GET /demo/tasks/{id}/history`
///
/// # Query Parameters
///
/// - `limit`: Number of events per page (default: 20, max: 100)
/// - `cursor`: Pagination cursor (event offset)
///
/// # Errors
///
/// - Task not found (404)
/// - Invalid task ID format (400)
/// - Repository error (500)
pub async fn get_task_history_demo(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Query(query): Query<HistoryQuery>,
) -> Result<JsonResponse<TaskHistoryResponse>, ApiErrorResponse> {
    // Parse and validate task ID
    let task_id = parse_task_id(&task_id)?;

    // Fetch task from repository
    let task = state
        .task_repository
        .find_by_id(&task_id)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?
        .ok_or_else(|| ApiErrorResponse::not_found("Task not found"))?;

    // Compute pagination parameters
    // Use clamp(1, MAX) to ensure limit is at least 1 (prevents infinite pagination loops)
    let limit = query
        .limit
        .unwrap_or(DEFAULT_HISTORY_LIMIT)
        .clamp(1, MAX_HISTORY_LIMIT);
    let cursor = query.cursor.unwrap_or(0);

    // Build response synchronously (Continuation/PersistentList are not Send)
    let response = {
        // Build mock history using PersistentList (demonstrates prepend efficiency)
        let base_timestamp = task.created_at.clone();
        let history_list = build_mock_history(&task, &base_timestamp);

        // Convert to Vec for pagination (reverse to get chronological order: oldest first)
        // PersistentList.cons() prepends, so most recent events are at the front
        let mut history_vec: Vec<TaskEvent> = history_list.iter().cloned().collect();
        history_vec.reverse();

        // Paginate using Continuation monad
        let paginated =
            paginate_with_continuation(&history_vec, cursor, limit).run(|result| result);

        TaskHistoryResponse {
            task_id: task.task_id.to_string(),
            events: paginated.items,
            next_cursor: paginated.next_cursor,
            has_more: paginated.has_more,
            total_events: paginated.total,
        }
    };

    Ok(JsonResponse(response))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn test_build_mock_history_basic() {
        let task_id = crate::domain::TaskId::generate();
        let now = Timestamp::now();
        let task = Task::new(task_id, "Test Task", now.clone());

        let history = build_mock_history(&task, &now);

        // Should have at least the Created event
        assert!(!history.is_empty());
    }

    #[rstest]
    fn test_build_mock_history_with_status() {
        let task_id = crate::domain::TaskId::generate();
        let now = Timestamp::now();
        let task = Task {
            status: TaskStatus::Completed,
            ..Task::new(task_id, "Test Task", now.clone())
        };

        let history = build_mock_history(&task, &now);

        // Completed status should have Created + 2 StatusChanged events
        assert!(history.len() >= 3);
    }

    #[rstest]
    fn test_build_mock_history_with_tags() {
        use crate::domain::Tag;

        let task_id = crate::domain::TaskId::generate();
        let now = Timestamp::now();
        let task = Task::new(task_id, "Test Task", now.clone())
            .add_tag(Tag::new("tag1"))
            .add_tag(Tag::new("tag2"));

        let history = build_mock_history(&task, &now)
            .iter()
            .cloned()
            .collect::<Vec<_>>();

        // Should have Created + 2 TagAdded events
        let tag_added_count = history
            .iter()
            .filter(|e| matches!(e, TaskEvent::TagAdded { .. }))
            .count();
        assert_eq!(tag_added_count, 2);
    }

    #[rstest]
    fn test_build_mock_history_with_priority() {
        let task_id = crate::domain::TaskId::generate();
        let now = Timestamp::now();
        let task = Task {
            priority: Priority::High,
            ..Task::new(task_id, "Test Task", now.clone())
        };

        let history = build_mock_history(&task, &now)
            .iter()
            .cloned()
            .collect::<Vec<_>>();

        // Should have PriorityChanged event
        let priority_changed_count = history
            .iter()
            .filter(|e| matches!(e, TaskEvent::PriorityChanged { .. }))
            .count();
        assert_eq!(priority_changed_count, 1);
    }

    #[rstest]
    fn test_paginate_items_basic() {
        let items = vec![1, 2, 3, 4, 5];
        let result = paginate_items(&items, 0, 2);

        assert_eq!(result.items, vec![1, 2]);
        assert_eq!(result.next_cursor, Some(2));
        assert!(result.has_more);
        assert_eq!(result.total, 5);
    }

    #[rstest]
    fn test_paginate_items_last_page() {
        let items = vec![1, 2, 3, 4, 5];
        let result = paginate_items(&items, 4, 2);

        assert_eq!(result.items, vec![5]);
        assert_eq!(result.next_cursor, None);
        assert!(!result.has_more);
        assert_eq!(result.total, 5);
    }
}
