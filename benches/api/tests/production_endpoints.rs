//! Integration tests for production endpoints.
//!
//! These tests verify that production handlers correctly use
//! real repository implementations (`EventStore`, `TaskRepository`).
//!
//! # Tests Covered
//!
//! - `get_task_history`: Verifies `EventStore` integration
//! - `search_tasks`: Verifies `TaskRepository` search
//! - `aggregate_sources`: Verifies external source failure handling
//! - `list_tasks`: Verifies filtered listing

mod common;

use rstest::rstest;

use common::{
    create_and_save_task, create_task_with_status_priority, create_test_app_state,
    create_test_app_state_with_fail_injection,
};
use task_management_benchmark_api::api::{create_task, dto::CreateTaskRequest, dto::PriorityDto};
use task_management_benchmark_api::domain::{
    EventId, Priority, TaskEventKind, TaskId, TaskStatus, Timestamp,
};

use axum::Json;
use axum::extract::{Path, Query, State};
use task_management_benchmark_api::api::JsonResponse;

// =============================================================================
// GET /tasks/{id}/history Tests
// =============================================================================

/// Test that `get_task_history` retrieves events from the `EventStore`.
#[rstest]
#[tokio::test]
async fn test_get_task_history_uses_event_store() {
    use task_management_benchmark_api::api::advanced::{HistoryQuery, get_task_history};

    // 1. Create test AppState
    let state = create_test_app_state();

    // 2. Create and save a task (includes event writing)
    let task = create_and_save_task(&state, "Test Task for History").await;

    // 3. Call get_task_history
    let result = get_task_history(
        State(state.clone()),
        Path(task.task_id.to_string()),
        Query(HistoryQuery {
            limit: Some(20),
            cursor: None,
        }),
    )
    .await;

    // 4. Verify the response
    assert!(result.is_ok(), "get_task_history should succeed");
    let JsonResponse(response) = result.unwrap();

    // 5. Verify task_id matches
    assert_eq!(response.task_id, task.task_id.to_string());

    // 6. Verify events were retrieved from EventStore
    assert!(
        !response.events.is_empty(),
        "Should have at least one event"
    );
    assert_eq!(response.total_events, 1, "Should have exactly one event");

    // 7. Verify the event is a Created event
    let first_event = &response.events[0];
    match first_event {
        task_management_benchmark_api::api::advanced::TaskEvent::Created { title, .. } => {
            assert_eq!(title, "Test Task for History");
        }
        _ => panic!("Expected Created event, got {first_event:?}"),
    }
}

/// Test that `get_task_history` returns 404 for non-existent task.
#[rstest]
#[tokio::test]
async fn test_get_task_history_not_found() {
    use task_management_benchmark_api::api::advanced::{HistoryQuery, get_task_history};

    let state = create_test_app_state();
    let nonexistent_id = TaskId::generate_v7();

    let result = get_task_history(
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

/// Test that `get_task_history` supports pagination.
#[rstest]
#[tokio::test]
async fn test_get_task_history_pagination() {
    use task_management_benchmark_api::api::advanced::{HistoryQuery, get_task_history};
    use task_management_benchmark_api::domain::{StatusChanged, TaskEventKind};

    let state = create_test_app_state();

    // Create a task
    let task = create_and_save_task(&state, "Task with multiple events").await;

    // Add more events to the task
    let event2 = task_management_benchmark_api::domain::TaskEvent::new(
        EventId::generate_v7(),
        task.task_id.clone(),
        Timestamp::now(),
        2,
        TaskEventKind::StatusChanged(StatusChanged {
            old_status: TaskStatus::Pending,
            new_status: TaskStatus::InProgress,
        }),
    );
    state
        .event_store
        .append(&event2, 1)
        .await
        .expect("Failed to append second event");

    // Request with limit=1
    let result = get_task_history(
        State(state.clone()),
        Path(task.task_id.to_string()),
        Query(HistoryQuery {
            limit: Some(1),
            cursor: None,
        }),
    )
    .await;

    assert!(result.is_ok());
    let JsonResponse(response) = result.unwrap();

    assert_eq!(response.events.len(), 1, "Should return only 1 event");
    assert_eq!(response.total_events, 2, "Total should be 2 events");
    assert!(response.has_more, "Should have more events");
    assert!(response.next_cursor.is_some(), "Should have next cursor");
}

// =============================================================================
// GET /tasks/search Tests
// =============================================================================

/// Test that `search_tasks` uses repository search functionality.
#[rstest]
#[tokio::test]
async fn test_search_tasks_uses_repository_search() {
    use task_management_benchmark_api::api::query::{SearchScope, SearchTasksQuery, search_tasks};

    let state = create_test_app_state();

    // Create multiple tasks with different titles
    create_and_save_task(&state, "Alpha task").await;
    create_and_save_task(&state, "Beta task").await;
    create_and_save_task(&state, "Gamma task").await;
    create_and_save_task(&state, "Unrelated item").await;

    // Search for "task"
    let result = search_tasks(
        State(state),
        Query(SearchTasksQuery {
            q: "task".to_string(),
            scope: SearchScope::Title,
            limit: None,
            offset: None,
        }),
    )
    .await;

    assert!(result.is_ok(), "search_tasks should succeed");
    let JsonResponse(response) = result.unwrap();

    // Should find 3 tasks with "task" in the title
    assert_eq!(response.len(), 3, "Should find 3 tasks");

    // Verify titles
    let titles: Vec<&str> = response.iter().map(|t| t.title.as_str()).collect();
    assert!(titles.contains(&"Alpha task"));
    assert!(titles.contains(&"Beta task"));
    assert!(titles.contains(&"Gamma task"));
    assert!(!titles.contains(&"Unrelated item"));
}

/// Test that `search_tasks` returns empty for no matches.
#[rstest]
#[tokio::test]
async fn test_search_tasks_no_results() {
    use task_management_benchmark_api::api::query::{SearchScope, SearchTasksQuery, search_tasks};

    let state = create_test_app_state();

    // Create a task
    create_and_save_task(&state, "Test task").await;

    // Search for something that doesn't exist
    let result = search_tasks(
        State(state),
        Query(SearchTasksQuery {
            q: "nonexistent".to_string(),
            scope: SearchScope::Title,
            limit: None,
            offset: None,
        }),
    )
    .await;

    assert!(result.is_ok());
    let JsonResponse(response) = result.unwrap();
    assert!(response.is_empty(), "Should return empty list");
}

// =============================================================================
// POST /tasks/aggregate-sources Tests
// =============================================================================

/// Test that `aggregate_sources` handles source failures gracefully.
#[rstest]
#[tokio::test]
async fn test_aggregate_sources_handles_failure() {
    use task_management_benchmark_api::api::alternative::{
        AggregateSourcesRequest, MergeStrategy, aggregate_sources,
    };

    // Create AppState where secondary and external sources fail
    let state = create_test_app_state_with_fail_injection(true, true);

    // Create a task in the primary source
    let task = create_and_save_task(&state, "Task for aggregation").await;

    // Request aggregation from all sources
    let result = aggregate_sources(
        State(state),
        Json(AggregateSourcesRequest {
            task_id: task.task_id.to_string(),
            sources: vec![
                "primary".to_string(),
                "secondary".to_string(),
                "external".to_string(),
            ],
            merge_strategy: MergeStrategy::PreferFirst,
        }),
    )
    .await;

    // Should succeed because primary source works
    assert!(
        result.is_ok(),
        "aggregate_sources should succeed when primary works"
    );
    let JsonResponse(response) = result.unwrap();

    // Verify sources_used and sources_failed
    assert!(
        response.sources_used.contains(&"primary".to_string()),
        "Primary should be used"
    );
    assert!(
        response.sources_failed.contains(&"secondary".to_string()),
        "Secondary should have failed"
    );
    assert!(
        response.sources_failed.contains(&"external".to_string()),
        "External should have failed"
    );

    // Verify the task data was retrieved from primary
    assert_eq!(
        response.task.title,
        Some("Task for aggregation".to_string())
    );
}

/// Test that `aggregate_sources` returns 503 when all sources fail.
#[rstest]
#[tokio::test]
async fn test_aggregate_sources_all_fail() {
    use task_management_benchmark_api::api::alternative::{
        AggregateSourcesRequest, MergeStrategy, aggregate_sources,
    };

    // Create AppState where all external sources fail (and no task in primary)
    let state = create_test_app_state_with_fail_injection(true, true);

    // Use a task ID that doesn't exist in primary
    let nonexistent_id = TaskId::generate_v7();

    let result = aggregate_sources(
        State(state),
        Json(AggregateSourcesRequest {
            task_id: nonexistent_id.to_string(),
            sources: vec!["primary".to_string(), "secondary".to_string()],
            merge_strategy: MergeStrategy::PreferFirst,
        }),
    )
    .await;

    // Should fail with 503 Service Unavailable
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(
        error.status,
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "Should return 503 when all sources fail"
    );
}

// =============================================================================
// GET /tasks Tests (list_tasks with filters)
// =============================================================================

/// Test that `list_tasks` correctly filters by status.
#[rstest]
#[tokio::test]
async fn test_list_tasks_filtered_by_status() {
    use task_management_benchmark_api::api::dto::TaskStatusDto;
    use task_management_benchmark_api::api::query::{ListTasksQuery, list_tasks};

    let state = create_test_app_state();

    // Create tasks with different statuses
    create_task_with_status_priority(&state, "Pending task", TaskStatus::Pending, Priority::Low)
        .await;
    create_task_with_status_priority(
        &state,
        "In progress task",
        TaskStatus::InProgress,
        Priority::Medium,
    )
    .await;
    create_task_with_status_priority(
        &state,
        "Completed task",
        TaskStatus::Completed,
        Priority::High,
    )
    .await;
    create_task_with_status_priority(
        &state,
        "Another pending",
        TaskStatus::Pending,
        Priority::Low,
    )
    .await;

    // List only pending tasks
    let result = list_tasks(
        State(state),
        Query(ListTasksQuery {
            page: 1,
            limit: 50,
            status: Some(TaskStatusDto::Pending),
            priority: None,
        }),
    )
    .await;

    assert!(result.is_ok(), "list_tasks should succeed");
    let JsonResponse(response) = result.unwrap();

    // Should return only pending tasks
    assert_eq!(response.data.len(), 2, "Should find 2 pending tasks");
    for task_response in &response.data {
        assert_eq!(
            task_response.status,
            TaskStatusDto::Pending,
            "All returned tasks should be pending"
        );
    }
}

/// Test that `list_tasks` correctly filters by priority.
#[rstest]
#[tokio::test]
async fn test_list_tasks_filtered_by_priority() {
    use task_management_benchmark_api::api::dto::PriorityDto;
    use task_management_benchmark_api::api::query::{ListTasksQuery, list_tasks};

    let state = create_test_app_state();

    // Create tasks with different priorities
    create_task_with_status_priority(
        &state,
        "Low priority task",
        TaskStatus::Pending,
        Priority::Low,
    )
    .await;
    create_task_with_status_priority(
        &state,
        "High priority task 1",
        TaskStatus::Pending,
        Priority::High,
    )
    .await;
    create_task_with_status_priority(
        &state,
        "High priority task 2",
        TaskStatus::InProgress,
        Priority::High,
    )
    .await;

    // List only high priority tasks
    let result = list_tasks(
        State(state),
        Query(ListTasksQuery {
            page: 1,
            limit: 50,
            status: None,
            priority: Some(PriorityDto::High),
        }),
    )
    .await;

    assert!(result.is_ok());
    let JsonResponse(response) = result.unwrap();

    // Should return only high priority tasks
    assert_eq!(response.data.len(), 2, "Should find 2 high priority tasks");
    for task_response in &response.data {
        assert_eq!(
            task_response.priority,
            PriorityDto::High,
            "All returned tasks should be high priority"
        );
    }
}

/// Test that `list_tasks` with combined filters works correctly.
#[rstest]
#[tokio::test]
async fn test_list_tasks_combined_filters() {
    use task_management_benchmark_api::api::dto::{PriorityDto, TaskStatusDto};
    use task_management_benchmark_api::api::query::{ListTasksQuery, list_tasks};

    let state = create_test_app_state();

    // Create tasks with various combinations
    create_task_with_status_priority(&state, "Pending Low", TaskStatus::Pending, Priority::Low)
        .await;
    create_task_with_status_priority(&state, "Pending High", TaskStatus::Pending, Priority::High)
        .await;
    create_task_with_status_priority(
        &state,
        "InProgress High",
        TaskStatus::InProgress,
        Priority::High,
    )
    .await;

    // List only pending + high priority
    let result = list_tasks(
        State(state),
        Query(ListTasksQuery {
            page: 1,
            limit: 50,
            status: Some(TaskStatusDto::Pending),
            priority: Some(PriorityDto::High),
        }),
    )
    .await;

    assert!(result.is_ok());
    let JsonResponse(response) = result.unwrap();

    // Should return only the task that matches both filters
    assert_eq!(
        response.data.len(),
        1,
        "Should find 1 task matching both filters"
    );
    assert_eq!(response.data[0].title, "Pending High");
}

// =============================================================================
// Event Writing Integration Tests
// =============================================================================

/// Test that creating a task via `create_task` writes to `EventStore`.
#[rstest]
#[tokio::test]
async fn test_create_task_writes_to_event_store() {
    let state = create_test_app_state();

    // Create a task via the handler
    let request = CreateTaskRequest {
        title: "New task via handler".to_string(),
        description: Some("Test description".to_string()),
        priority: PriorityDto::High,
        tags: vec!["test".to_string()],
    };

    let result = create_task(State(state.clone()), Json(request)).await;

    assert!(result.is_ok(), "create_task should succeed");
    let (status, JsonResponse(response)) = result.unwrap();
    assert_eq!(status, axum::http::StatusCode::CREATED);

    // Parse the task ID from response
    let task_id =
        TaskId::from_uuid(uuid::Uuid::parse_str(&response.id).expect("Invalid UUID in response"));

    // Verify event was written to EventStore
    let current_version = state
        .event_store
        .get_current_version(&task_id)
        .await
        .expect("Failed to get version");

    assert_eq!(
        current_version, 1,
        "EventStore should have version 1 after creation"
    );

    // Load and verify the event
    let history = state
        .event_store
        .load_events(&task_id)
        .await
        .expect("Failed to load events");

    assert_eq!(history.iter().count(), 1, "Should have exactly 1 event");

    let event = history
        .iter()
        .next()
        .expect("Should have at least one event");
    match &event.kind {
        TaskEventKind::Created(payload) => {
            assert_eq!(payload.title, "New task via handler");
            assert_eq!(payload.priority, Priority::High);
        }
        _ => panic!("Expected Created event, got {:?}", &event.kind),
    }
}
