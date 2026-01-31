//! Integration tests for demo endpoints.
//!
//! These tests verify that demo handlers correctly generate mock data
//! without using the real `EventStore`.
//!
//! # Feature Gate
//!
//! These tests are only compiled when the `demo` feature is enabled.
//!
//! # Tests Covered
//!
//! - `get_task_history_demo`: Verifies mock history generation
//! - Comparison with production endpoints to ensure different behavior

#![cfg(feature = "demo")]

mod common;

use rstest::rstest;

use common::{create_test_app_state, save_task_without_events};
use task_management_benchmark_api::domain::{Priority, Tag, Task, TaskId, TaskStatus, Timestamp};

use axum::extract::{Path, Query, State};
use task_management_benchmark_api::api::JsonResponse;

// =============================================================================
// Demo: GET /demo/tasks/{id}/history Tests
// =============================================================================

/// Test that `get_task_history_demo` generates mock history.
#[rstest]
#[tokio::test]
async fn test_get_task_history_demo_generates_mock() {
    use task_management_benchmark_api::api::advanced::{HistoryQuery, TaskEvent};
    use task_management_benchmark_api::api::demo::get_task_history_demo;

    let state = create_test_app_state();

    // Create a task WITHOUT writing events to EventStore
    let task_id = TaskId::generate_v7();
    let timestamp = Timestamp::now();
    let task = Task::new(task_id.clone(), "Demo Task", timestamp);
    save_task_without_events(&state, &task).await;

    // Call the demo endpoint
    let result = get_task_history_demo(
        State(state.clone()),
        Path(task.task_id.to_string()),
        Query(HistoryQuery {
            limit: Some(20),
            cursor: None,
        }),
    )
    .await;

    // Should succeed
    assert!(result.is_ok(), "get_task_history_demo should succeed");
    let JsonResponse(response) = result.unwrap();

    // Should have mock history even though EventStore is empty
    assert!(!response.events.is_empty(), "Should have mock events");
    assert_eq!(response.task_id, task.task_id.to_string());

    // Verify Created event is present
    let has_created = response
        .events
        .iter()
        .any(|event| matches!(event, TaskEvent::Created { title, .. } if title == "Demo Task"));
    assert!(has_created, "Should have Created event for 'Demo Task'");
}

/// Test that demo endpoint generates status change events based on task status.
#[rstest]
#[tokio::test]
async fn test_get_task_history_demo_with_status_changes() {
    use task_management_benchmark_api::api::advanced::{HistoryQuery, TaskEvent};
    use task_management_benchmark_api::api::demo::get_task_history_demo;
    use task_management_benchmark_api::api::dto::TaskStatusDto;

    let state = create_test_app_state();

    // Create a completed task (should generate multiple status events)
    let task_id = TaskId::generate_v7();
    let timestamp = Timestamp::now();
    let task =
        Task::new(task_id.clone(), "Completed Task", timestamp).with_status(TaskStatus::Completed);
    save_task_without_events(&state, &task).await;

    // Call the demo endpoint
    let result = get_task_history_demo(
        State(state),
        Path(task.task_id.to_string()),
        Query(HistoryQuery {
            limit: Some(50),
            cursor: None,
        }),
    )
    .await;

    assert!(result.is_ok());
    let JsonResponse(response) = result.unwrap();

    // Completed task should have: Created, StatusChanged (Pending->InProgress), StatusChanged (InProgress->Completed)
    let status_change_count = response
        .events
        .iter()
        .filter(|e| matches!(e, TaskEvent::StatusChanged { .. }))
        .count();
    assert_eq!(
        status_change_count, 2,
        "Completed task should have 2 status change events"
    );

    // Verify the final status change is to Completed
    let has_completed_change = response.events.iter().any(|event| {
        matches!(
            event,
            TaskEvent::StatusChanged {
                new_status: TaskStatusDto::Completed,
                ..
            }
        )
    });
    assert!(
        has_completed_change,
        "Should have StatusChanged event to Completed"
    );
}

/// Test that demo endpoint generates tag events for tasks with tags.
#[rstest]
#[tokio::test]
async fn test_get_task_history_demo_with_tags() {
    use task_management_benchmark_api::api::advanced::{HistoryQuery, TaskEvent};
    use task_management_benchmark_api::api::demo::get_task_history_demo;

    let state = create_test_app_state();

    // Create a task with tags
    let task_id = TaskId::generate_v7();
    let timestamp = Timestamp::now();
    let task = Task::new(task_id.clone(), "Tagged Task", timestamp)
        .add_tag(Tag::new("rust"))
        .add_tag(Tag::new("functional"));
    save_task_without_events(&state, &task).await;

    // Call the demo endpoint
    let result = get_task_history_demo(
        State(state),
        Path(task.task_id.to_string()),
        Query(HistoryQuery {
            limit: Some(50),
            cursor: None,
        }),
    )
    .await;

    assert!(result.is_ok());
    let JsonResponse(response) = result.unwrap();

    // Should have TagAdded events
    let tag_events: Vec<_> = response
        .events
        .iter()
        .filter_map(|e| match e {
            TaskEvent::TagAdded { tag, .. } => Some(tag.as_str()),
            _ => None,
        })
        .collect();

    assert_eq!(tag_events.len(), 2, "Should have 2 TagAdded events");
    assert!(tag_events.contains(&"rust"));
    assert!(tag_events.contains(&"functional"));
}

/// Test that demo endpoint generates priority change events for non-default priority.
#[rstest]
#[tokio::test]
async fn test_get_task_history_demo_with_priority_change() {
    use task_management_benchmark_api::api::advanced::{HistoryQuery, TaskEvent};
    use task_management_benchmark_api::api::demo::get_task_history_demo;
    use task_management_benchmark_api::api::dto::PriorityDto;

    let state = create_test_app_state();

    // Create a task with high priority (non-default)
    let task_id = TaskId::generate_v7();
    let timestamp = Timestamp::now();
    let task =
        Task::new(task_id.clone(), "High Priority Task", timestamp).with_priority(Priority::High);
    save_task_without_events(&state, &task).await;

    // Call the demo endpoint
    let result = get_task_history_demo(
        State(state),
        Path(task.task_id.to_string()),
        Query(HistoryQuery {
            limit: Some(50),
            cursor: None,
        }),
    )
    .await;

    assert!(result.is_ok());
    let JsonResponse(response) = result.unwrap();

    // Should have PriorityChanged event
    let has_priority_change = response.events.iter().any(|event| {
        matches!(
            event,
            TaskEvent::PriorityChanged {
                new_priority: PriorityDto::High,
                ..
            }
        )
    });
    assert!(
        has_priority_change,
        "Should have PriorityChanged event to High"
    );
}

/// Test that demo endpoint uses mock data, NOT the `EventStore`.
#[rstest]
#[tokio::test]
async fn test_demo_endpoints_use_mock_not_event_store() {
    use task_management_benchmark_api::api::advanced::{HistoryQuery, get_task_history};
    use task_management_benchmark_api::api::demo::get_task_history_demo;

    let state = create_test_app_state();

    // Create a task WITHOUT writing any events
    let task_id = TaskId::generate_v7();
    let timestamp = Timestamp::now();
    let task = Task::new(task_id.clone(), "Test Task", timestamp);
    save_task_without_events(&state, &task).await;

    // Verify EventStore is actually empty
    let event_count = state
        .event_store
        .get_current_version(&task.task_id)
        .await
        .expect("Failed to get version");
    assert_eq!(event_count, 0, "EventStore should be empty");

    // Demo endpoint should return mock history even with empty EventStore
    let demo_result = get_task_history_demo(
        State(state.clone()),
        Path(task.task_id.to_string()),
        Query(HistoryQuery {
            limit: None,
            cursor: None,
        }),
    )
    .await;

    assert!(demo_result.is_ok(), "Demo endpoint should succeed");
    let JsonResponse(demo_response) = demo_result.unwrap();
    assert!(
        !demo_response.events.is_empty(),
        "Demo should return mock events"
    );

    // Production endpoint should return empty history
    let production_result = get_task_history(
        State(state),
        Path(task.task_id.to_string()),
        Query(HistoryQuery {
            limit: None,
            cursor: None,
        }),
    )
    .await;

    assert!(
        production_result.is_ok(),
        "Production endpoint should succeed"
    );
    let JsonResponse(production_response) = production_result.unwrap();
    assert!(
        production_response.events.is_empty(),
        "Production endpoint should return empty list when EventStore is empty"
    );
}

/// Test that demo endpoint returns 404 for non-existent task.
#[rstest]
#[tokio::test]
async fn test_get_task_history_demo_not_found() {
    use task_management_benchmark_api::api::advanced::HistoryQuery;
    use task_management_benchmark_api::api::demo::get_task_history_demo;

    let state = create_test_app_state();
    let nonexistent_id = TaskId::generate_v7();

    let result = get_task_history_demo(
        State(state),
        Path(nonexistent_id.to_string()),
        Query(HistoryQuery {
            limit: None,
            cursor: None,
        }),
    )
    .await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.status, axum::http::StatusCode::NOT_FOUND);
}

/// Test that demo endpoint respects pagination parameters.
#[rstest]
#[tokio::test]
async fn test_get_task_history_demo_pagination() {
    use task_management_benchmark_api::api::advanced::HistoryQuery;
    use task_management_benchmark_api::api::demo::get_task_history_demo;

    let state = create_test_app_state();

    // Create a task with many tags to generate multiple events
    let task_id = TaskId::generate_v7();
    let timestamp = Timestamp::now();
    let mut task = Task::new(task_id.clone(), "Multi-event Task", timestamp)
        .with_priority(Priority::High)
        .with_status(TaskStatus::Completed);
    for i in 0..5 {
        task = task.add_tag(Tag::new(format!("tag{i}")));
    }
    save_task_without_events(&state, &task).await;

    // Request only 2 events
    let result = get_task_history_demo(
        State(state.clone()),
        Path(task.task_id.to_string()),
        Query(HistoryQuery {
            limit: Some(2),
            cursor: None,
        }),
    )
    .await;

    assert!(result.is_ok());
    let JsonResponse(response) = result.unwrap();

    // Should return only 2 events but total should be higher
    assert_eq!(response.events.len(), 2, "Should return exactly 2 events");
    assert!(response.total_events > 2, "Total should be more than 2");
    assert!(response.has_more, "Should have more events");
    assert!(response.next_cursor.is_some(), "Should have next cursor");
}
