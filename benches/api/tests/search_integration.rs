//! Integration tests for /tasks/search endpoint.
//!
//! REQ-SEARCH-API-001: API contract verification
//! REQ-SEARCH-INDEX-001: Index differential updates
//! REQ-SEARCH-CACHE-001: Query normalization and caching

use lambars::persistent::PersistentVector;
use rstest::rstest;
use task_management_benchmark_api::api::query::{
    SEARCH_DEFAULT_LIMIT, SEARCH_MAX_LIMIT, SearchCacheKey, SearchIndex, SearchScope, TaskChange,
    normalize_query, normalize_search_pagination,
};
use task_management_benchmark_api::domain::{Priority, Task, TaskId, Timestamp};

// =============================================================================
// REQ-SEARCH-API-001: API Contract Tests
// =============================================================================

/// Test default pagination values.
#[rstest]
fn test_pagination_defaults() {
    let (limit, offset) = normalize_search_pagination(None, None);
    assert_eq!(limit, SEARCH_DEFAULT_LIMIT);
    assert_eq!(offset, 0);
}

/// Test limit capping at `SEARCH_MAX_LIMIT`.
#[rstest]
fn test_pagination_max_limit() {
    let (limit, _) = normalize_search_pagination(Some(500), None);
    assert_eq!(limit, SEARCH_MAX_LIMIT);
}

/// Test normal pagination values are preserved.
#[rstest]
fn test_pagination_normal_values() {
    let (limit, offset) = normalize_search_pagination(Some(100), Some(20));
    assert_eq!(limit, 100);
    assert_eq!(offset, 20);
}

/// Test limit=0 is preserved (explicit user intent for empty result).
#[rstest]
fn test_pagination_zero_limit() {
    let (limit, offset) = normalize_search_pagination(Some(0), Some(10));
    assert_eq!(limit, 0);
    assert_eq!(offset, 10);
}

// =============================================================================
// REQ-SEARCH-INDEX-001: Index Integration Tests
// =============================================================================

/// Test that index updates are reflected in search results.
#[rstest]
fn test_index_update_integration() {
    // Build empty index
    let empty_tasks: PersistentVector<Task> = PersistentVector::new();
    let index = SearchIndex::build(&empty_tasks);

    // Add a task
    let task = Task::new(
        TaskId::generate(),
        "Integration test task",
        Timestamp::now(),
    )
    .with_priority(Priority::High);
    let updated_index = index.apply_change(TaskChange::Add(task.clone()));

    // Search should find the task
    let result = updated_index.search_by_title("integration");
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 1);

    // Remove the task
    let final_index = updated_index.apply_change(TaskChange::Remove(task.task_id));

    // Search should not find the task
    let result = final_index.search_by_title("integration");
    assert!(result.is_none() || result.unwrap().is_empty());
}

/// Test that multiple tasks can be indexed and searched.
#[rstest]
fn test_index_multiple_tasks() {
    let tasks: PersistentVector<Task> = vec![
        Task::new(
            TaskId::generate(),
            "First integration task",
            Timestamp::now(),
        ),
        Task::new(
            TaskId::generate(),
            "Second integration task",
            Timestamp::now(),
        ),
        Task::new(TaskId::generate(), "Unrelated work item", Timestamp::now()),
    ]
    .into_iter()
    .collect();

    let index = SearchIndex::build(&tasks);

    // Search for "integration" should find 2 tasks
    let result = index.search_by_title("integration");
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 2);

    // Search for "unrelated" should find 1 task
    let result = index.search_by_title("unrelated");
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 1);
}

/// Test task update operation.
#[rstest]
fn test_index_update_task() {
    let task_id = TaskId::generate();
    let original_task = Task::new(task_id, "Original title", Timestamp::now());
    let tasks: PersistentVector<Task> = vec![original_task.clone()].into_iter().collect();

    let index = SearchIndex::build(&tasks);

    // Original title should be findable
    let result = index.search_by_title("original");
    assert!(result.is_some());

    // Update the task with new title
    let updated_task = Task::new(task_id, "Updated title", Timestamp::now());
    let updated_index = index.apply_change(TaskChange::Update {
        old: original_task,
        new: updated_task,
    });

    // Original title should NOT be findable
    let result = updated_index.search_by_title("original");
    assert!(result.is_none() || result.unwrap().is_empty());

    // Updated title should be findable
    let result = updated_index.search_by_title("updated");
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 1);
}

// =============================================================================
// REQ-SEARCH-CACHE-001: Cache Integration Tests
// =============================================================================

/// Test cache key generation with normalization.
#[rstest]
fn test_cache_key_normalization() {
    let key1 = SearchCacheKey::from_raw("  Test Query  ", SearchScope::All, Some(50), Some(0));
    let key2 = SearchCacheKey::from_raw("test query", SearchScope::All, Some(50), Some(0));
    assert_eq!(key1, key2);
}

/// Test cache keys differ by scope.
#[rstest]
fn test_cache_key_scope_difference() {
    let key1 = SearchCacheKey::from_raw("test", SearchScope::Title, Some(50), Some(0));
    let key2 = SearchCacheKey::from_raw("test", SearchScope::All, Some(50), Some(0));
    assert_ne!(key1, key2);
}

/// Test cache keys differ by pagination.
#[rstest]
fn test_cache_key_pagination_difference() {
    let key1 = SearchCacheKey::from_raw("test", SearchScope::All, Some(50), Some(0));
    let key2 = SearchCacheKey::from_raw("test", SearchScope::All, Some(100), Some(0));
    assert_ne!(key1, key2);
}

/// Test cache key uses normalized defaults.
#[rstest]
fn test_cache_key_default_normalization() {
    // None for limit/offset should normalize to defaults
    let key1 = SearchCacheKey::from_raw("test", SearchScope::All, None, None);
    let key2 = SearchCacheKey::from_raw(
        "test",
        SearchScope::All,
        Some(SEARCH_DEFAULT_LIMIT),
        Some(0),
    );
    assert_eq!(key1, key2);
}

/// Test query normalization is idempotent.
#[rstest]
fn test_normalization_idempotent() {
    let raw = "  MULTIPLE   Spaces   Here  ";
    let first = normalize_query(raw);
    let second = normalize_query(first.key());
    assert_eq!(first.key(), second.key());
}

/// Test that cache key is idempotent - `from_raw` with already normalized input produces same key.
#[rstest]
fn test_cache_key_idempotent() {
    let raw_query = "  Multiple   SPACES   Here  ";

    // First key from raw query
    let key1 = SearchCacheKey::from_raw(raw_query, SearchScope::All, Some(50), Some(0));

    // Get the normalized query from key1
    let normalized = key1.normalized_query();

    // Second key from already normalized query
    let key2 = SearchCacheKey::from_raw(normalized, SearchScope::All, Some(50), Some(0));

    // Both keys should be equal
    assert_eq!(key1, key2, "Cache key should be idempotent");
    assert_eq!(key1.normalized_query(), key2.normalized_query());
}

/// Test empty query normalization.
#[rstest]
fn test_normalization_empty_query() {
    let result = normalize_query("   ");
    assert!(result.is_empty());
    assert_eq!(result.key(), "");
    assert!(result.tokens().is_empty());
}

/// Test single word normalization.
#[rstest]
fn test_normalization_single_word() {
    let result = normalize_query("  URGENT  ");
    assert_eq!(result.key(), "urgent");
    assert_eq!(result.tokens(), &["urgent"]);
}

// =============================================================================
// E2E Search Order Stability Test
// =============================================================================

/// Test that search results maintain stable order.
#[rstest]
fn test_search_order_stability() {
    let tasks: PersistentVector<Task> = vec![
        Task::new(TaskId::generate(), "Alpha task", Timestamp::now()),
        Task::new(TaskId::generate(), "Beta task", Timestamp::now()),
        Task::new(TaskId::generate(), "Gamma task", Timestamp::now()),
    ]
    .into_iter()
    .collect();

    let index = SearchIndex::build(&tasks);

    // Search multiple times
    let result1 = index.search_by_title("task");
    let result2 = index.search_by_title("task");

    assert!(result1.is_some() && result2.is_some());
    let ids1: Vec<_> = result1
        .unwrap()
        .tasks()
        .iter()
        .map(|task| task.task_id)
        .collect();
    let ids2: Vec<_> = result2
        .unwrap()
        .tasks()
        .iter()
        .map(|task| task.task_id)
        .collect();
    assert_eq!(ids1, ids2, "Search results should have stable order");
}

/// Test that empty search returns None.
#[rstest]
fn test_search_no_results() {
    let tasks: PersistentVector<Task> =
        vec![Task::new(TaskId::generate(), "Some task", Timestamp::now())]
            .into_iter()
            .collect();

    let index = SearchIndex::build(&tasks);

    let result = index.search_by_title("nonexistent");
    assert!(result.is_none() || result.unwrap().is_empty());
}
