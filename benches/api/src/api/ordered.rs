//! Ordered data operations using `PersistentTreeMap`.
//!
//! This module demonstrates:
//! - `PersistentTreeMap`: Sorted key-value storage with O(log N) operations
//! - `range()`: Efficient range queries
//! - Composite keys with custom `Ord` implementations
//!
//! # lambars Features Demonstrated
//!
//! - **`PersistentTreeMap`**: Immutable sorted map with B-tree structure
//! - **`range()`**: Range queries returning sorted iterators
//! - **`Ord` composition**: Custom sort orders via composite keys

use std::cmp::Ordering;
use std::ops::Bound;

use axum::extract::{Query, State};

use super::json_buffer::JsonResponse;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use lambars::persistent::PersistentTreeMap;

use super::dto::TaskResponse;
use super::error::{ApiErrorResponse, FieldError};
use super::handlers::AppState;
use crate::domain::{Priority, Task, TaskId, TaskStatus};
use crate::infrastructure::Pagination;

// =============================================================================
// DTOs
// =============================================================================

/// Query parameters for by-deadline endpoint.
#[derive(Debug, Deserialize)]
pub struct DeadlineQuery {
    /// Start date (inclusive, YYYY-MM-DD format).
    pub from: Option<String>,
    /// End date (inclusive, YYYY-MM-DD format).
    pub to: Option<String>,
    /// Maximum number of results (default: 50, max: 1000).
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Number of results to skip (default: 0).
    #[serde(default)]
    pub offset: usize,
}

const fn default_limit() -> usize {
    50
}

/// Response for by-deadline endpoint.
///
/// Note: For benchmark purposes, data is limited to 10,000 tasks.
/// The `total_in_range` field reflects this subset, not the entire dataset.
#[derive(Debug, Serialize)]
pub struct DeadlineTasksResponse {
    pub tasks: Vec<TaskResponse>,
    pub total_in_range: usize,
    pub from: Option<String>,
    pub to: Option<String>,
    pub treemap_operations: usize,
}

/// Query parameters for timeline endpoint.
#[derive(Debug, Deserialize)]
pub struct TimelineQuery {
    /// Sort order: `priority_first` (default) or `created_first`.
    #[serde(default = "default_order")]
    pub order: String,
    /// Maximum number of results (default: 20, max: 100).
    #[serde(default = "default_timeline_limit")]
    pub limit: usize,
    /// Number of results to skip (default: 0).
    #[serde(default)]
    pub offset: usize,
    /// Filter by status (optional, comma-separated).
    /// Valid values: `pending`, `in_progress`, `completed`, `cancelled`.
    /// Invalid values are silently ignored.
    pub status_filter: Option<String>,
}

fn default_order() -> String {
    "priority_first".to_string()
}

const fn default_timeline_limit() -> usize {
    20
}

/// Response for timeline endpoint.
///
/// Note: For benchmark purposes, data is limited to 10,000 tasks.
/// The `total` and `has_more` fields reflect this subset, not the entire dataset.
#[derive(Debug, Serialize)]
pub struct TimelineResponse {
    pub tasks: Vec<TaskResponse>,
    pub total: usize,
    pub has_more: bool,
    pub order: String,
    pub treemap_operations: usize,
}

/// Query parameters for leaderboard endpoint.
#[derive(Debug, Deserialize)]
pub struct LeaderboardQuery {
    /// Number of top projects to return (default: 10, max: 100).
    #[serde(default = "default_top")]
    pub top: usize,
    /// Sort criteria: `completed_tasks` (default), `completion_rate`, or `total_tasks`.
    #[serde(default = "default_sort_by")]
    pub sort_by: String,
}

const fn default_top() -> usize {
    10
}

fn default_sort_by() -> String {
    "completed_tasks".to_string()
}

/// A project's ranking entry.
#[derive(Debug, Serialize)]
pub struct ProjectRanking {
    pub rank: usize,
    pub project_id: String,
    pub project_name: String,
    pub completed_tasks: usize,
    pub total_tasks: usize,
    pub completion_rate: f64,
}

/// Response for leaderboard endpoint.
///
/// Note: For benchmark purposes, data is limited to 10,000 projects.
/// The `total_projects` field reflects this subset, not the entire dataset.
#[derive(Debug, Serialize)]
pub struct LeaderboardResponse {
    pub rankings: Vec<ProjectRanking>,
    pub total_projects: usize,
    pub sort_by: String,
    pub treemap_operations: usize,
}

// =============================================================================
// Composite Keys for PersistentTreeMap
// =============================================================================

/// Composite key for deadline-based indexing.
/// Sorted by (deadline, `task_id`) to ensure uniqueness.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DeadlineKey {
    deadline: NaiveDate,
    task_id: String,
}

impl PartialOrd for DeadlineKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DeadlineKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.deadline
            .cmp(&other.deadline)
            .then_with(|| self.task_id.cmp(&other.task_id))
    }
}

/// Composite key for timeline indexing.
/// Sorted by (`priority_rank`, `inverted_timestamp`, `task_id`).
/// - `priority_rank`: 0=Critical, 1=High, 2=Medium, 3=Low (ascending for higher priority first)
/// - `inverted_timestamp`: `i64::MAX` - timestamp (descending for newer first)
#[derive(Debug, Clone, PartialEq, Eq)]
struct TimelineKey {
    priority_rank: u8,
    inverted_timestamp: i64,
    task_id: String,
}

impl TimelineKey {
    fn from_task(task: &Task) -> Self {
        Self {
            priority_rank: match task.priority {
                Priority::Critical => 0,
                Priority::High => 1,
                Priority::Medium => 2,
                Priority::Low => 3,
            },
            inverted_timestamp: i64::MAX - task.created_at.as_datetime().timestamp(),
            task_id: task.task_id.to_string(),
        }
    }

    fn from_task_created_first(task: &Task) -> Self {
        Self {
            priority_rank: 0, // Ignored for created_first order
            inverted_timestamp: i64::MAX - task.created_at.as_datetime().timestamp(),
            task_id: task.task_id.to_string(),
        }
    }
}

impl PartialOrd for TimelineKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimelineKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority_rank
            .cmp(&other.priority_rank)
            .then_with(|| self.inverted_timestamp.cmp(&other.inverted_timestamp))
            .then_with(|| self.task_id.cmp(&other.task_id))
    }
}

/// Composite key for leaderboard indexing.
/// Sorted by (`inverted_score`, `project_id`) for descending score order.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LeaderboardKey {
    inverted_score: i64,
    project_id: String,
}

impl PartialOrd for LeaderboardKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LeaderboardKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inverted_score
            .cmp(&other.inverted_score)
            .then_with(|| self.project_id.cmp(&other.project_id))
    }
}

// =============================================================================
// Type Aliases for PersistentTreeMap Indices
// =============================================================================

type DeadlineIndex = PersistentTreeMap<DeadlineKey, TaskId>;
type TimelineIndex = PersistentTreeMap<TimelineKey, TaskId>;
type LeaderboardIndex = PersistentTreeMap<LeaderboardKey, ProjectStats>;

/// Project statistics for leaderboard.
#[derive(Debug, Clone)]
struct ProjectStats {
    project_id: String,
    project_name: String,
    completed_tasks: usize,
    total_tasks: usize,
}

// =============================================================================
// GET /tasks/by-deadline - Deadline range search
// =============================================================================

/// Searches tasks by deadline range using `PersistentTreeMap`.
///
/// This handler demonstrates:
/// - **`PersistentTreeMap`**: Building a sorted index by deadline
/// - **`range()`**: Efficient range queries
///
/// # Query Parameters
///
/// - `from`: Start date (YYYY-MM-DD, inclusive)
/// - `to`: End date (YYYY-MM-DD, inclusive)
/// - `limit`: Maximum results (default: 50, max: 1000)
/// - `offset`: Skip count (default: 0)
///
/// # Errors
///
/// - `400 Bad Request`: Invalid date format or range
#[allow(clippy::unused_async)]
pub async fn tasks_by_deadline(
    State(state): State<AppState>,
    Query(query): Query<DeadlineQuery>,
) -> Result<JsonResponse<DeadlineTasksResponse>, ApiErrorResponse> {
    // Validate limit
    if query.limit > 1000 {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("limit", "limit must be at most 1000")],
        ));
    }

    // Parse dates
    let from_date = query
        .from
        .as_ref()
        .map(|s| parse_date(s))
        .transpose()
        .map_err(|e| {
            ApiErrorResponse::validation_error(
                "Validation failed",
                vec![FieldError::new("from", e)],
            )
        })?;

    let to_date = query
        .to
        .as_ref()
        .map(|s| parse_date(s))
        .transpose()
        .map_err(|e| {
            ApiErrorResponse::validation_error("Validation failed", vec![FieldError::new("to", e)])
        })?;

    // Validate range
    if let (Some(from), Some(to)) = (from_date, to_date)
        && from > to
    {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new(
                "from",
                "from date must be before or equal to to date",
            )],
        ));
    }

    // Fetch all tasks.
    // Note: Using page size 10,000 for benchmark purposes.
    // In production, consider streaming or cursor-based pagination.
    let pagination = Pagination::new(0, 10000);
    let tasks = state
        .task_repository
        .list(pagination)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    // Build deadline index (simulating deadline with `created_at` for demo)
    let (index, operations) = build_deadline_index(&tasks.items);

    // Query by range
    let (task_ids, total_in_range) =
        query_deadline_range(&index, from_date, to_date, query.limit, query.offset);

    // Convert to responses
    let task_responses: Vec<TaskResponse> = task_ids
        .into_iter()
        .filter_map(|id| {
            tasks
                .items
                .iter()
                .find(|t| t.task_id == id)
                .map(TaskResponse::from)
        })
        .collect();

    Ok(JsonResponse(DeadlineTasksResponse {
        tasks: task_responses,
        total_in_range,
        from: query.from,
        to: query.to,
        treemap_operations: operations,
    }))
}

/// Parses a date string in YYYY-MM-DD format.
fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| format!("Invalid date format: {s}. Expected YYYY-MM-DD"))
}

/// Pure: Builds a deadline index from tasks.
/// For demo purposes, we simulate deadlines using `created_at` date.
fn build_deadline_index(tasks: &[Task]) -> (DeadlineIndex, usize) {
    let mut operations = 0;
    let index = tasks.iter().fold(PersistentTreeMap::new(), |map, task| {
        // Simulate deadline as created_at date for demonstration
        let deadline = task.created_at.as_datetime().date_naive();
        let key = DeadlineKey {
            deadline,
            task_id: task.task_id.to_string(),
        };
        operations += 1;
        map.insert(key, task.task_id.clone())
    });
    (index, operations)
}

/// Pure: Queries tasks by deadline range.
///
/// # Arguments
///
/// * `index` - The deadline index to query
/// * `from` - Start date (inclusive)
/// * `to` - End date (inclusive)
/// * `limit` - Maximum results to return
/// * `offset` - Number of results to skip
///
/// # Returns
///
/// A tuple of (paginated task IDs, total count in range)
fn query_deadline_range(
    index: &DeadlineIndex,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    limit: usize,
    offset: usize,
) -> (Vec<TaskId>, usize) {
    let start = from.map_or(Bound::Unbounded, |d| {
        Bound::Included(DeadlineKey {
            deadline: d,
            task_id: String::new(),
        })
    });

    let end = to.map_or(Bound::Unbounded, |d| {
        // Include all tasks on the `to` date by using the next day as exclusive bound.
        // Handle edge case: if `to` is the maximum date (9999-12-31), use unbounded.
        d.succ_opt().map_or(Bound::Unbounded, |next_day| {
            Bound::Excluded(DeadlineKey {
                deadline: next_day,
                task_id: String::new(),
            })
        })
    });

    // Count total first, then paginate using iterator
    let total = index.range((start.clone(), end.clone())).count();
    let paginated: Vec<TaskId> = index
        .range((start, end))
        .skip(offset)
        .take(limit)
        .map(|(_, task_id)| task_id.clone())
        .collect();

    (paginated, total)
}

// =============================================================================
// GET /tasks/timeline - Priority-ordered timeline
// =============================================================================

/// Returns a timeline of tasks sorted by priority and creation time.
///
/// This handler demonstrates:
/// - **`PersistentTreeMap` with composite key**: Custom sort order
/// - **`iter()`**: Sorted iteration
///
/// # Query Parameters
///
/// - `order`: `priority_first` (default) or `created_first`
/// - `limit`: Maximum results (default: 20, max: 100)
/// - `offset`: Skip count (default: 0)
/// - `status_filter`: Comma-separated status filter (optional)
///
/// # Errors
///
/// - `400 Bad Request`: Invalid order parameter
#[allow(clippy::unused_async)]
pub async fn tasks_timeline(
    State(state): State<AppState>,
    Query(query): Query<TimelineQuery>,
) -> Result<JsonResponse<TimelineResponse>, ApiErrorResponse> {
    // Validate order
    if query.order != "priority_first" && query.order != "created_first" {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new(
                "order",
                "order must be 'priority_first' or 'created_first'",
            )],
        ));
    }

    // Validate limit
    if query.limit > 100 {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("limit", "limit must be at most 100")],
        ));
    }

    // Parse status filter.
    // Invalid status values are silently ignored (design decision for API flexibility).
    // If all values are invalid, treat as "no filter" (return all tasks).
    let status_filter: Option<Vec<TaskStatus>> = query.status_filter.as_ref().and_then(|s| {
        let statuses: Vec<TaskStatus> = s
            .split(',')
            .filter_map(|status| match status.trim().to_lowercase().as_str() {
                "pending" => Some(TaskStatus::Pending),
                "in_progress" => Some(TaskStatus::InProgress),
                "completed" => Some(TaskStatus::Completed),
                "cancelled" => Some(TaskStatus::Cancelled),
                _ => None, // Invalid values ignored
            })
            .collect();
        // Return None if no valid statuses found (treat as "no filter")
        if statuses.is_empty() {
            None
        } else {
            Some(statuses)
        }
    });

    // Fetch all tasks.
    // Note: Using page size 10,000 for benchmark purposes.
    let pagination = Pagination::new(0, 10000);
    let tasks = state
        .task_repository
        .list(pagination)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    // Filter by status if specified
    let filtered_tasks: Vec<&Task> = if let Some(ref statuses) = status_filter {
        tasks
            .items
            .iter()
            .filter(|t| statuses.contains(&t.status))
            .collect()
    } else {
        tasks.items.iter().collect()
    };

    // Build timeline index
    let (index, operations) = build_timeline_index(&filtered_tasks, &query.order);

    // Get paginated results
    let total = index.iter().count();
    let task_ids: Vec<TaskId> = index
        .iter()
        .skip(query.offset)
        .take(query.limit)
        .map(|(_, task_id)| task_id.clone())
        .collect();

    let has_more = query.offset + task_ids.len() < total;

    // Convert to responses
    let task_responses: Vec<TaskResponse> = task_ids
        .into_iter()
        .filter_map(|id| {
            tasks
                .items
                .iter()
                .find(|t| t.task_id == id)
                .map(TaskResponse::from)
        })
        .collect();

    Ok(JsonResponse(TimelineResponse {
        tasks: task_responses,
        total,
        has_more,
        order: query.order,
        treemap_operations: operations,
    }))
}

/// Pure: Builds a timeline index from tasks.
fn build_timeline_index(tasks: &[&Task], order: &str) -> (TimelineIndex, usize) {
    let mut operations = 0;
    let index = tasks.iter().fold(PersistentTreeMap::new(), |map, task| {
        let key = if order == "priority_first" {
            TimelineKey::from_task(task)
        } else {
            TimelineKey::from_task_created_first(task)
        };
        operations += 1;
        map.insert(key, task.task_id.clone())
    });
    (index, operations)
}

// =============================================================================
// GET /projects/leaderboard - Project ranking
// =============================================================================

/// Returns a leaderboard of projects ranked by task completion metrics.
///
/// This handler demonstrates:
/// - **`PersistentTreeMap`**: Building a sorted ranking
/// - **Inverted score**: Descending order via score inversion
///
/// # Query Parameters
///
/// - `top`: Number of top projects (default: 10, max: 100)
/// - `sort_by`: `completed_tasks` (default), `completion_rate`, or `total_tasks`
///
/// # Errors
///
/// - `400 Bad Request`: Invalid `sort_by` parameter
#[allow(clippy::unused_async)]
pub async fn projects_leaderboard(
    State(state): State<AppState>,
    Query(query): Query<LeaderboardQuery>,
) -> Result<JsonResponse<LeaderboardResponse>, ApiErrorResponse> {
    // Validate sort_by
    if query.sort_by != "completed_tasks"
        && query.sort_by != "completion_rate"
        && query.sort_by != "total_tasks"
    {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new(
                "sort_by",
                "sort_by must be 'completed_tasks', 'completion_rate', or 'total_tasks'",
            )],
        ));
    }

    // Validate top
    if query.top > 100 {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("top", "top must be at most 100")],
        ));
    }

    // Fetch all projects.
    // Note: Using page size 10,000 for benchmark purposes.
    let pagination = Pagination::new(0, 10000);
    let projects = state
        .project_repository
        .list(pagination)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    // Calculate project stats
    let project_stats: Vec<ProjectStats> = projects
        .items
        .iter()
        .map(|project| {
            let total = project.tasks.len();
            let completed = project
                .tasks
                .iter()
                .filter(|(_, summary)| summary.status == TaskStatus::Completed)
                .count();
            ProjectStats {
                project_id: project.project_id.to_string(),
                project_name: project.name.clone(),
                completed_tasks: completed,
                total_tasks: total,
            }
        })
        .collect();

    // Build leaderboard index
    let (index, operations) = build_leaderboard_index(&project_stats, &query.sort_by);

    // Get top N rankings
    let total_projects = projects.items.len();
    let rankings: Vec<ProjectRanking> = index
        .iter()
        .take(query.top)
        .enumerate()
        .map(|(i, (_, stats))| {
            #[allow(clippy::cast_precision_loss)]
            let completion_rate = if stats.total_tasks > 0 {
                stats.completed_tasks as f64 / stats.total_tasks as f64
            } else {
                0.0
            };
            ProjectRanking {
                rank: i + 1,
                project_id: stats.project_id.clone(),
                project_name: stats.project_name.clone(),
                completed_tasks: stats.completed_tasks,
                total_tasks: stats.total_tasks,
                completion_rate,
            }
        })
        .collect();

    Ok(JsonResponse(LeaderboardResponse {
        rankings,
        total_projects,
        sort_by: query.sort_by,
        treemap_operations: operations,
    }))
}

/// Pure: Builds a leaderboard index from project stats.
#[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
fn build_leaderboard_index(stats: &[ProjectStats], sort_by: &str) -> (LeaderboardIndex, usize) {
    let mut operations = 0;
    let index = stats.iter().fold(PersistentTreeMap::new(), |map, stat| {
        let score: i64 = match sort_by {
            "completed_tasks" => stat.completed_tasks as i64,
            "total_tasks" => stat.total_tasks as i64,
            "completion_rate" => {
                // Convert rate to integer score (multiply by 10000 for precision)
                #[allow(clippy::cast_precision_loss)]
                if stat.total_tasks > 0 {
                    ((stat.completed_tasks as f64 / stat.total_tasks as f64) * 10000.0) as i64
                } else {
                    0
                }
            }
            _ => 0,
        };

        let key = LeaderboardKey {
            inverted_score: i64::MAX - score,
            project_id: stat.project_id.clone(),
        };
        operations += 1;
        map.insert(key, stat.clone())
    });
    (index, operations)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Timestamp;
    use chrono::Utc;
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // Helper Functions
    // -------------------------------------------------------------------------

    fn create_test_task(id: &str, priority: Priority, days_ago: i64) -> Task {
        let timestamp = Utc::now() - chrono::Duration::days(days_ago);
        Task::new(
            TaskId::from_uuid(uuid::Uuid::parse_str(id).unwrap()),
            format!("Task {id}"),
            Timestamp::from_datetime(timestamp),
        )
        .with_priority(priority)
    }

    // -------------------------------------------------------------------------
    // DeadlineKey Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_deadline_key_ordering() {
        let key1 = DeadlineKey {
            deadline: NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            task_id: "a".to_string(),
        };
        let key2 = DeadlineKey {
            deadline: NaiveDate::from_ymd_opt(2026, 1, 16).unwrap(),
            task_id: "a".to_string(),
        };
        let key3 = DeadlineKey {
            deadline: NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            task_id: "b".to_string(),
        };

        assert!(key1 < key2); // Earlier date comes first
        assert!(key1 < key3); // Same date, "a" < "b"
    }

    // -------------------------------------------------------------------------
    // TimelineKey Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_timeline_key_priority_ordering() {
        let key_critical = TimelineKey {
            priority_rank: 0,
            inverted_timestamp: 0,
            task_id: "a".to_string(),
        };
        let key_high = TimelineKey {
            priority_rank: 1,
            inverted_timestamp: 0,
            task_id: "a".to_string(),
        };
        let key_low = TimelineKey {
            priority_rank: 3,
            inverted_timestamp: 0,
            task_id: "a".to_string(),
        };

        assert!(key_critical < key_high);
        assert!(key_high < key_low);
    }

    #[rstest]
    fn test_timeline_key_timestamp_ordering() {
        // Lower inverted_timestamp = higher original timestamp = newer
        let key_newer = TimelineKey {
            priority_rank: 0,
            inverted_timestamp: i64::MAX - 1000,
            task_id: "a".to_string(),
        };
        let key_older = TimelineKey {
            priority_rank: 0,
            inverted_timestamp: i64::MAX - 500,
            task_id: "a".to_string(),
        };

        assert!(key_newer < key_older); // Newer comes first
    }

    // -------------------------------------------------------------------------
    // LeaderboardKey Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_leaderboard_key_ordering() {
        let key_high_score = LeaderboardKey {
            inverted_score: i64::MAX - 100, // score = 100
            project_id: "a".to_string(),
        };
        let key_low_score = LeaderboardKey {
            inverted_score: i64::MAX - 50, // score = 50
            project_id: "a".to_string(),
        };

        assert!(key_high_score < key_low_score); // Higher score comes first
    }

    // -------------------------------------------------------------------------
    // Build Index Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_build_deadline_index_empty() {
        let tasks: Vec<Task> = Vec::new();
        let (index, operations) = build_deadline_index(&tasks);

        assert_eq!(index.iter().count(), 0);
        assert_eq!(operations, 0);
    }

    #[rstest]
    fn test_build_deadline_index_multiple_tasks() {
        let tasks = vec![
            create_test_task("00000000-0000-0000-0000-000000000001", Priority::High, 1),
            create_test_task("00000000-0000-0000-0000-000000000002", Priority::Low, 2),
            create_test_task("00000000-0000-0000-0000-000000000003", Priority::Medium, 0),
        ];

        let (index, operations) = build_deadline_index(&tasks);

        assert_eq!(index.iter().count(), 3);
        assert_eq!(operations, 3);
    }

    #[rstest]
    fn test_build_timeline_index_priority_first() {
        let task_high = create_test_task("00000000-0000-0000-0000-000000000001", Priority::High, 0);
        let task_low = create_test_task("00000000-0000-0000-0000-000000000002", Priority::Low, 0);
        let task_critical = create_test_task(
            "00000000-0000-0000-0000-000000000003",
            Priority::Critical,
            0,
        );

        let tasks: Vec<&Task> = vec![&task_high, &task_low, &task_critical];
        let (index, _) = build_timeline_index(&tasks, "priority_first");

        let ids: Vec<String> = index.iter().map(|(_, id)| id.to_string()).collect();

        // Critical should come first, then High, then Low
        assert!(ids[0].contains("00000000-0000-0000-0000-000000000003")); // Critical
        assert!(ids[1].contains("00000000-0000-0000-0000-000000000001")); // High
        assert!(ids[2].contains("00000000-0000-0000-0000-000000000002")); // Low
    }

    #[rstest]
    fn test_build_leaderboard_index() {
        let stats = vec![
            ProjectStats {
                project_id: "p1".to_string(),
                project_name: "Project 1".to_string(),
                completed_tasks: 5,
                total_tasks: 10,
            },
            ProjectStats {
                project_id: "p2".to_string(),
                project_name: "Project 2".to_string(),
                completed_tasks: 10,
                total_tasks: 10,
            },
            ProjectStats {
                project_id: "p3".to_string(),
                project_name: "Project 3".to_string(),
                completed_tasks: 3,
                total_tasks: 10,
            },
        ];

        let (index, operations) = build_leaderboard_index(&stats, "completed_tasks");

        assert_eq!(operations, 3);

        let rankings: Vec<String> = index.iter().map(|(_, s)| s.project_id.clone()).collect();

        assert_eq!(rankings[0], "p2"); // 10 completed (highest)
        assert_eq!(rankings[1], "p1"); // 5 completed
        assert_eq!(rankings[2], "p3"); // 3 completed (lowest)
    }

    // -------------------------------------------------------------------------
    // Range Query Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_query_deadline_range_unbounded() {
        let tasks = vec![
            create_test_task("00000000-0000-0000-0000-000000000001", Priority::High, 1),
            create_test_task("00000000-0000-0000-0000-000000000002", Priority::Low, 2),
        ];

        let (index, _) = build_deadline_index(&tasks);
        let (result, total) = query_deadline_range(&index, None, None, 50, 0);

        assert_eq!(total, 2);
        assert_eq!(result.len(), 2);
    }

    #[rstest]
    fn test_query_deadline_range_with_pagination() {
        let tasks = vec![
            create_test_task("00000000-0000-0000-0000-000000000001", Priority::High, 1),
            create_test_task("00000000-0000-0000-0000-000000000002", Priority::Low, 2),
            create_test_task("00000000-0000-0000-0000-000000000003", Priority::Medium, 3),
        ];

        let (index, _) = build_deadline_index(&tasks);

        // Get first 2
        let (result, total) = query_deadline_range(&index, None, None, 2, 0);
        assert_eq!(total, 3);
        assert_eq!(result.len(), 2);

        // Get next 1 with offset
        let (result, total) = query_deadline_range(&index, None, None, 2, 2);
        assert_eq!(total, 3);
        assert_eq!(result.len(), 1);
    }

    // -------------------------------------------------------------------------
    // Parse Date Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_parse_date_valid() {
        let date = parse_date("2026-01-19").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 1, 19).unwrap());
    }

    #[rstest]
    fn test_parse_date_invalid() {
        let result = parse_date("invalid");
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // Edge Case Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_query_deadline_range_max_date() {
        // Test that `to = 9999-12-31` correctly uses `Bound::Unbounded`
        let tasks = vec![create_test_task(
            "00000000-0000-0000-0000-000000000001",
            Priority::High,
            0,
        )];
        let (index, _) = build_deadline_index(&tasks);

        // Query with max date as `to`
        let max_date = NaiveDate::from_ymd_opt(9999, 12, 31).unwrap();
        let (result, total) = query_deadline_range(&index, None, Some(max_date), 50, 0);

        // Should still return all tasks (max date should be included)
        assert_eq!(total, 1);
        assert_eq!(result.len(), 1);
    }

    #[rstest]
    fn test_status_filter_all_invalid_values_returns_all() {
        // When all status filter values are invalid, treat as "no filter"
        // This is tested via the parsing logic which returns None for empty vec

        let filter_string = "invalid,unknown,xyz";
        let parsed: Option<Vec<TaskStatus>> =
            Some(filter_string.to_string()).as_ref().and_then(|s| {
                let statuses: Vec<TaskStatus> = s
                    .split(',')
                    .filter_map(|status| match status.trim().to_lowercase().as_str() {
                        "pending" => Some(TaskStatus::Pending),
                        "in_progress" => Some(TaskStatus::InProgress),
                        "completed" => Some(TaskStatus::Completed),
                        "cancelled" => Some(TaskStatus::Cancelled),
                        _ => None,
                    })
                    .collect();
                if statuses.is_empty() {
                    None
                } else {
                    Some(statuses)
                }
            });

        // All invalid values should result in None (no filter)
        assert!(parsed.is_none());
    }

    #[rstest]
    fn test_status_filter_partial_valid_values() {
        // When some values are valid and some invalid, only valid values are used
        let filter_string = "pending,invalid,completed";
        let parsed: Option<Vec<TaskStatus>> =
            Some(filter_string.to_string()).as_ref().and_then(|s| {
                let statuses: Vec<TaskStatus> = s
                    .split(',')
                    .filter_map(|status| match status.trim().to_lowercase().as_str() {
                        "pending" => Some(TaskStatus::Pending),
                        "in_progress" => Some(TaskStatus::InProgress),
                        "completed" => Some(TaskStatus::Completed),
                        "cancelled" => Some(TaskStatus::Cancelled),
                        _ => None,
                    })
                    .collect();
                if statuses.is_empty() {
                    None
                } else {
                    Some(statuses)
                }
            });

        // Should have only valid statuses
        assert!(parsed.is_some());
        let statuses = parsed.unwrap();
        assert_eq!(statuses.len(), 2);
        assert!(statuses.contains(&TaskStatus::Pending));
        assert!(statuses.contains(&TaskStatus::Completed));
    }
}
