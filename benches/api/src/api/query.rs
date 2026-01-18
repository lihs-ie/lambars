//! Query handlers for task retrieval and search.
//!
//! This module demonstrates lambars' functional programming patterns for data querying:
//! - **`Foldable`**: Aggregating data with fold operations
//! - **`Monoid`**: Combining search results with deduplication
//! - **`PersistentVector`**: Immutable collections with structural sharing
//!
//! # Endpoints
//!
//! - `GET /tasks` - List tasks with pagination and filtering
//! - `GET /tasks/search` - Search tasks by title or tags
//! - `GET /tasks/by-priority` - Count tasks grouped by priority
//!
//! # Design Note
//!
//! This implementation intentionally fetches all data and processes it in-memory
//! to showcase functional programming patterns (filter, fold, combine). In production,
//! filtering and aggregation should be delegated to the repository layer for better
//! performance with large datasets.

use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};

use super::dto::{PriorityDto, TaskResponse, TaskStatusDto};
use super::error::ApiErrorResponse;
use super::handlers::AppState;
use crate::domain::{Priority, Task, TaskId, TaskStatus};
use crate::infrastructure::Pagination;
use lambars::persistent::{PersistentHashSet, PersistentVector};
use lambars::typeclass::Semigroup;

// =============================================================================
// Query Parameters
// =============================================================================

/// Query parameters for listing tasks.
#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    /// Page number (default: 1, minimum: 1).
    #[serde(default = "default_page")]
    pub page: u32,
    /// Items per page (default: 20, range: 1-100).
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// Filter by task status.
    pub status: Option<TaskStatusDto>,
    /// Filter by task priority.
    pub priority: Option<PriorityDto>,
}

const fn default_page() -> u32 {
    1
}

const fn default_limit() -> u32 {
    20
}

/// Query parameters for searching tasks.
#[derive(Debug, Deserialize)]
pub struct SearchTasksQuery {
    /// Search query string (case-insensitive substring match).
    pub q: String,
    /// Search scope: "title", "tags", or "all" (default: "all").
    #[serde(rename = "in", default)]
    pub scope: SearchScope,
}

/// Search scope enum.
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchScope {
    /// Search only in task titles.
    Title,
    /// Search only in task tags.
    Tags,
    /// Search in both titles and tags.
    #[default]
    All,
}

// =============================================================================
// Response DTOs
// =============================================================================

/// Paginated result container.
#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResponse<T> {
    /// The data items for the current page.
    pub data: Vec<T>,
    /// Current page number.
    pub page: u32,
    /// Items per page.
    pub limit: u32,
    /// Total number of items.
    pub total: u64,
    /// Total number of pages (0 if no items).
    pub total_pages: u64,
}

/// Priority count result.
#[derive(Debug, Clone, Default, Serialize)]
pub struct PriorityCountResponse {
    /// Count of low priority tasks.
    pub low: u64,
    /// Count of medium priority tasks.
    pub medium: u64,
    /// Count of high priority tasks.
    pub high: u64,
    /// Count of critical priority tasks.
    pub critical: u64,
    /// Total count (derived from individual counts).
    pub total: u64,
}

impl PriorityCountResponse {
    /// Increment the low priority count.
    #[must_use]
    const fn increment_low(self) -> Self {
        Self {
            low: self.low + 1,
            medium: self.medium,
            high: self.high,
            critical: self.critical,
            total: self.total,
        }
    }

    /// Increment the medium priority count.
    #[must_use]
    const fn increment_medium(self) -> Self {
        Self {
            low: self.low,
            medium: self.medium + 1,
            high: self.high,
            critical: self.critical,
            total: self.total,
        }
    }

    /// Increment the high priority count.
    #[must_use]
    const fn increment_high(self) -> Self {
        Self {
            low: self.low,
            medium: self.medium,
            high: self.high + 1,
            critical: self.critical,
            total: self.total,
        }
    }

    /// Increment the critical priority count.
    #[must_use]
    const fn increment_critical(self) -> Self {
        Self {
            low: self.low,
            medium: self.medium,
            high: self.high,
            critical: self.critical + 1,
            total: self.total,
        }
    }

    /// Finalize the response by computing the total.
    #[must_use]
    const fn finalize(self) -> Self {
        Self {
            low: self.low,
            medium: self.medium,
            high: self.high,
            critical: self.critical,
            total: self.low + self.medium + self.high + self.critical,
        }
    }
}

// =============================================================================
// Search Result with Monoid
// =============================================================================

/// Search result with deduplication by task ID.
///
/// Implements `Semigroup` for combining results from different sources.
/// Title matches are prioritized by adding them first.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Tasks ordered by match quality (title matches first, then tags).
    tasks: PersistentVector<Task>,
    /// Set of task IDs for deduplication.
    seen_ids: PersistentHashSet<TaskId>,
}

impl SearchResult {
    /// Creates an empty search result.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            tasks: PersistentVector::new(),
            seen_ids: PersistentHashSet::new(),
        }
    }

    /// Creates a search result from a vector of tasks.
    ///
    /// Builds the `seen_ids` set from the task IDs for deduplication.
    #[must_use]
    pub fn from_tasks(tasks: PersistentVector<Task>) -> Self {
        let seen_ids = tasks
            .iter()
            .map(|task| task.task_id.clone())
            .collect::<PersistentHashSet<TaskId>>();
        Self { tasks, seen_ids }
    }

    /// Returns the tasks as a vector.
    #[must_use]
    pub fn into_tasks(self) -> PersistentVector<Task> {
        self.tasks
    }
}

impl Semigroup for SearchResult {
    /// Combines two search results with deduplication.
    ///
    /// Tasks from `self` appear first (higher priority), followed by
    /// tasks from `other` that are not already present.
    fn combine(self, other: Self) -> Self {
        let mut tasks = self.tasks;
        let mut seen_ids = self.seen_ids;

        for task in &other.tasks {
            if !seen_ids.contains(&task.task_id) {
                tasks = tasks.push_back(task.clone());
                seen_ids = seen_ids.insert(task.task_id.clone());
            }
        }

        Self { tasks, seen_ids }
    }
}

// =============================================================================
// GET /tasks - List Tasks
// =============================================================================

/// Lists tasks with pagination and optional filtering.
///
/// This handler demonstrates:
/// - **Pure filtering**: Using iterator combinators for status/priority filters
/// - **Pagination**: Skip/take pattern for efficient page extraction
///
/// # Query Parameters
///
/// - `page`: Page number (default: 1)
/// - `limit`: Items per page (default: 20, max: 100)
/// - `status`: Optional filter by task status
/// - `priority`: Optional filter by task priority
///
/// # Response
///
/// - **200 OK**: Paginated list of tasks
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] in the following cases:
/// - **500 Internal Server Error**: Repository operation failed
#[allow(clippy::future_not_send)]
pub async fn list_tasks(
    State(state): State<AppState>,
    Query(query): Query<ListTasksQuery>,
) -> Result<Json<PaginatedResponse<TaskResponse>>, ApiErrorResponse> {
    // I/O boundary: Fetch all tasks from repository
    let all_tasks = state
        .task_repository
        .list(Pagination::default())
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    // Convert to PersistentVector for functional operations
    let tasks: PersistentVector<Task> = all_tasks.items.into_iter().collect();

    // Pure computation: Filter and paginate
    let status_filter = query.status.map(TaskStatus::from);
    let priority_filter = query.priority.map(Priority::from);

    let filtered = filter_tasks(&tasks, status_filter, priority_filter);
    let response = paginate_tasks(&filtered, query.page, query.limit);

    Ok(Json(response))
}

/// Filters tasks by status and priority (pure function).
fn filter_tasks(
    tasks: &PersistentVector<Task>,
    status_filter: Option<TaskStatus>,
    priority_filter: Option<Priority>,
) -> PersistentVector<Task> {
    tasks
        .iter()
        .filter(|task| status_filter.is_none_or(|s| task.status == s))
        .filter(|task| priority_filter.is_none_or(|p| task.priority == p))
        .cloned()
        .collect()
}

/// Paginates a vector of tasks (pure function).
fn paginate_tasks(tasks: &PersistentVector<Task>, page: u32, limit: u32) -> PaginatedResponse<TaskResponse> {
    // Clamp inputs to valid ranges
    let page = page.max(1);
    let limit = limit.clamp(1, 100);
    let offset = ((page - 1) * limit) as usize;
    let total = tasks.len() as u64;

    // Extract page slice
    let data: Vec<TaskResponse> = tasks
        .iter()
        .skip(offset)
        .take(limit as usize)
        .map(TaskResponse::from)
        .collect();

    // Calculate total pages (0 if no items)
    let total_pages = if total == 0 {
        0
    } else {
        total.div_ceil(u64::from(limit))
    };

    PaginatedResponse {
        data,
        page,
        limit,
        total,
        total_pages,
    }
}

// =============================================================================
// GET /tasks/search - Search Tasks
// =============================================================================

/// Searches tasks by title or tags.
///
/// This handler demonstrates:
/// - **Monoid pattern**: Combining search results with `Semigroup::combine`
/// - **Deduplication**: Using `PersistentHashSet` for tracking seen IDs
///
/// # Query Parameters
///
/// - `q`: Search query (case-insensitive substring match)
/// - `in`: Search scope - "title", "tags", or "all" (default)
///
/// # Response
///
/// - **200 OK**: List of matching tasks
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] in the following cases:
/// - **500 Internal Server Error**: Repository operation failed
#[allow(clippy::future_not_send)]
pub async fn search_tasks(
    State(state): State<AppState>,
    Query(query): Query<SearchTasksQuery>,
) -> Result<Json<Vec<TaskResponse>>, ApiErrorResponse> {
    // I/O boundary: Fetch all tasks from repository
    let all_tasks = state
        .task_repository
        .list(Pagination::default())
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    // Convert to PersistentVector for functional operations
    let tasks: PersistentVector<Task> = all_tasks.items.into_iter().collect();

    // Pure computation: Search with scope
    let results = search_with_scope(&tasks, &query.q, query.scope);

    // Convert to response
    let response: Vec<TaskResponse> = results
        .into_tasks()
        .iter()
        .map(TaskResponse::from)
        .collect();

    Ok(Json(response))
}

/// Searches tasks based on query and scope (pure function).
fn search_with_scope(
    tasks: &PersistentVector<Task>,
    query: &str,
    scope: SearchScope,
) -> SearchResult {
    // Empty query returns all tasks (early return to avoid double traversal)
    if query.is_empty() {
        return SearchResult::from_tasks(tasks.clone());
    }

    match scope {
        SearchScope::Title => search_by_title(tasks, query),
        SearchScope::Tags => search_by_tags(tasks, query),
        SearchScope::All => {
            // Title matches come first (higher priority), then tag matches
            let title_results = search_by_title(tasks, query);
            let tag_results = search_by_tags(tasks, query);
            // Use Semigroup::combine for deduplication
            title_results.combine(tag_results)
        }
    }
}

/// Searches tasks by title (case-insensitive substring match).
///
/// Note: In production, consider storing pre-normalized titles for better performance.
/// This implementation normalizes on each comparison for simplicity and demonstration.
fn search_by_title(tasks: &PersistentVector<Task>, query: &str) -> SearchResult {
    let query_lower = query.to_lowercase();
    let matching: PersistentVector<Task> = tasks
        .iter()
        .filter(|task| task.title.to_lowercase().contains(&query_lower))
        .cloned()
        .collect();

    SearchResult::from_tasks(matching)
}

/// Searches tasks by tag (case-insensitive substring match).
///
/// Note: In production, consider storing pre-normalized tags for better performance.
/// This implementation normalizes on each comparison for simplicity and demonstration.
fn search_by_tags(tasks: &PersistentVector<Task>, query: &str) -> SearchResult {
    let query_lower = query.to_lowercase();
    let matching: PersistentVector<Task> = tasks
        .iter()
        .filter(|task| {
            task.tags
                .iter()
                .any(|tag| tag.as_str().to_lowercase().contains(&query_lower))
        })
        .cloned()
        .collect();

    SearchResult::from_tasks(matching)
}

// =============================================================================
// GET /tasks/by-priority - Count by Priority
// =============================================================================

/// Counts tasks grouped by priority level.
///
/// This handler demonstrates:
/// - **Foldable pattern**: Using `fold` for single-pass aggregation
/// - **Sum Monoid**: Deriving total from individual counts
///
/// # Response
///
/// - **200 OK**: Priority counts with total
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] in the following cases:
/// - **500 Internal Server Error**: Repository operation failed
#[allow(clippy::future_not_send)]
pub async fn count_by_priority(
    State(state): State<AppState>,
) -> Result<Json<PriorityCountResponse>, ApiErrorResponse> {
    // I/O boundary: Fetch all tasks from repository
    let all_tasks = state
        .task_repository
        .list(Pagination::default())
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    // Pure computation: Count by priority using fold
    let counts = count_tasks_by_priority(&all_tasks.items);

    Ok(Json(counts))
}

/// Counts tasks by priority level (pure function).
///
/// Uses fold for single-pass aggregation, then derives total.
fn count_tasks_by_priority(tasks: &[Task]) -> PriorityCountResponse {
    tasks
        .iter()
        .fold(PriorityCountResponse::default(), |acc, task| {
            match task.priority {
                Priority::Low => acc.increment_low(),
                Priority::Medium => acc.increment_medium(),
                Priority::High => acc.increment_high(),
                Priority::Critical => acc.increment_critical(),
            }
        })
        .finalize()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Tag, TaskId, Timestamp};
    use rstest::rstest;

    fn create_test_task(title: &str, priority: Priority) -> Task {
        Task::new(TaskId::generate(), title, Timestamp::now()).with_priority(priority)
    }

    fn create_test_task_with_tags(title: &str, priority: Priority, tags: &[&str]) -> Task {
        let base = create_test_task(title, priority);
        tags.iter().fold(base, |task, tag| task.add_tag(Tag::new(*tag)))
    }

    // -------------------------------------------------------------------------
    // Filter Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_filter_tasks_no_filter() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::High),
        ]
        .into_iter()
        .collect();

        let filtered = filter_tasks(&tasks, None, None);
        assert_eq!(filtered.len(), 2);
    }

    #[rstest]
    fn test_filter_tasks_by_priority() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::High),
            create_test_task("Task 3", Priority::Low),
        ]
        .into_iter()
        .collect();

        let filtered = filter_tasks(&tasks, None, Some(Priority::Low));
        assert_eq!(filtered.len(), 2);
    }

    // -------------------------------------------------------------------------
    // Pagination Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_paginate_first_page() {
        let tasks: PersistentVector<Task> = (0..25)
            .map(|i| create_test_task(&format!("Task {i}"), Priority::Medium))
            .collect();

        let result = paginate_tasks(&tasks, 1, 10);

        assert_eq!(result.data.len(), 10);
        assert_eq!(result.page, 1);
        assert_eq!(result.limit, 10);
        assert_eq!(result.total, 25);
        assert_eq!(result.total_pages, 3);
    }

    #[rstest]
    fn test_paginate_last_page() {
        let tasks: PersistentVector<Task> = (0..25)
            .map(|i| create_test_task(&format!("Task {i}"), Priority::Medium))
            .collect();

        let result = paginate_tasks(&tasks, 3, 10);

        assert_eq!(result.data.len(), 5);
        assert_eq!(result.page, 3);
        assert_eq!(result.total_pages, 3);
    }

    #[rstest]
    fn test_paginate_empty() {
        let tasks: PersistentVector<Task> = PersistentVector::new();

        let result = paginate_tasks(&tasks, 1, 10);

        assert_eq!(result.data.len(), 0);
        assert_eq!(result.total, 0);
        assert_eq!(result.total_pages, 0);
    }

    #[rstest]
    fn test_paginate_clamps_page_zero() {
        let tasks: PersistentVector<Task> = (0..10)
            .map(|i| create_test_task(&format!("Task {i}"), Priority::Medium))
            .collect();

        let result = paginate_tasks(&tasks, 0, 10);

        assert_eq!(result.page, 1);
    }

    #[rstest]
    fn test_paginate_clamps_limit() {
        let tasks: PersistentVector<Task> = (0..200)
            .map(|i| create_test_task(&format!("Task {i}"), Priority::Medium))
            .collect();

        let result = paginate_tasks(&tasks, 1, 200);

        assert_eq!(result.limit, 100);
        assert_eq!(result.data.len(), 100);
    }

    // -------------------------------------------------------------------------
    // Search Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_search_by_title() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Important meeting", Priority::High),
            create_test_task("Review code", Priority::Medium),
            create_test_task("Important deadline", Priority::Critical),
        ]
        .into_iter()
        .collect();

        let result = search_by_title(&tasks, "important");

        assert_eq!(result.tasks.len(), 2);
    }

    #[rstest]
    fn test_search_by_title_case_insensitive() {
        let tasks: PersistentVector<Task> = vec![create_test_task("URGENT Task", Priority::High)]
            .into_iter()
            .collect();

        let result = search_by_title(&tasks, "urgent");

        assert_eq!(result.tasks.len(), 1);
    }

    #[rstest]
    fn test_search_empty_query_returns_all() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::High),
        ]
        .into_iter()
        .collect();

        let result = search_with_scope(&tasks, "", SearchScope::All);

        assert_eq!(result.tasks.len(), 2);
    }

    #[rstest]
    fn test_search_result_combine_deduplicates() {
        let task1 = create_test_task("Task 1", Priority::Low);
        let task2 = create_test_task("Task 2", Priority::High);
        let task1_clone = task1.clone();

        let result1 =
            SearchResult::from_tasks(vec![task1.clone(), task2.clone()].into_iter().collect());
        let result2 = SearchResult::from_tasks(vec![task1_clone, task2].into_iter().collect());

        let combined = result1.combine(result2);

        // Should have 2 tasks, not 4 (deduplicated by ID)
        assert_eq!(combined.tasks.len(), 2);
    }

    #[rstest]
    fn test_search_by_tags() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task_with_tags("Task 1", Priority::Low, &["backend", "rust"]),
            create_test_task_with_tags("Task 2", Priority::Medium, &["frontend", "typescript"]),
            create_test_task_with_tags("Task 3", Priority::High, &["backend", "go"]),
        ]
        .into_iter()
        .collect();

        let result = search_by_tags(&tasks, "backend");

        assert_eq!(result.tasks.len(), 2);
    }

    #[rstest]
    fn test_search_by_tags_case_insensitive() {
        let tasks: PersistentVector<Task> =
            vec![create_test_task_with_tags("Task 1", Priority::Low, &["URGENT"])]
                .into_iter()
                .collect();

        let result = search_by_tags(&tasks, "urgent");

        assert_eq!(result.tasks.len(), 1);
    }

    #[rstest]
    fn test_search_scope_all_prioritizes_title_matches() {
        // Task with "important" in title
        let title_match = create_test_task("Important meeting", Priority::High);
        // Task with "important" in tag
        let tag_match = create_test_task_with_tags("Regular task", Priority::Low, &["important"]);

        let tasks: PersistentVector<Task> =
            vec![tag_match.clone(), title_match.clone()].into_iter().collect();

        let result = search_with_scope(&tasks, "important", SearchScope::All);

        // Should have 2 results
        assert_eq!(result.tasks.len(), 2);
        // Title match should come first (title_match has "Important" in title)
        let first = result.tasks.iter().next().unwrap();
        assert!(first.title.contains("Important"));
    }

    #[rstest]
    fn test_search_scope_all_deduplicates() {
        // Task that matches both title and tag
        let both_match =
            create_test_task_with_tags("Important meeting", Priority::High, &["important"]);

        let tasks: PersistentVector<Task> = vec![both_match].into_iter().collect();

        let result = search_with_scope(&tasks, "important", SearchScope::All);

        // Should have 1 result, not 2 (deduplicated)
        assert_eq!(result.tasks.len(), 1);
    }

    #[rstest]
    fn test_paginate_page_beyond_total() {
        let tasks: PersistentVector<Task> = (0..10)
            .map(|i| create_test_task(&format!("Task {i}"), Priority::Medium))
            .collect();

        // Request page 5, but only 1 page exists (10 items / 10 per page = 1 page)
        let result = paginate_tasks(&tasks, 5, 10);

        // Should return empty data since page 5 is beyond total_pages
        assert!(result.data.is_empty());
        assert_eq!(result.page, 5);
        assert_eq!(result.total, 10);
        assert_eq!(result.total_pages, 1);
    }

    // -------------------------------------------------------------------------
    // Priority Count Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_count_by_priority() {
        let tasks = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::Low),
            create_test_task("Task 3", Priority::Medium),
            create_test_task("Task 4", Priority::High),
            create_test_task("Task 5", Priority::Critical),
        ];

        let counts = count_tasks_by_priority(&tasks);

        assert_eq!(counts.low, 2);
        assert_eq!(counts.medium, 1);
        assert_eq!(counts.high, 1);
        assert_eq!(counts.critical, 1);
        assert_eq!(counts.total, 5);
    }

    #[rstest]
    fn test_count_by_priority_empty() {
        let tasks: Vec<Task> = vec![];

        let counts = count_tasks_by_priority(&tasks);

        assert_eq!(counts.low, 0);
        assert_eq!(counts.medium, 0);
        assert_eq!(counts.high, 0);
        assert_eq!(counts.critical, 0);
        assert_eq!(counts.total, 0);
    }
}
