//! Query handlers for task retrieval and search.
//!
//! This module demonstrates lambars' functional programming patterns for data querying:
//! - **`Foldable`**: Aggregating data with fold operations
//! - **`Monoid`**: Combining search results with deduplication
//! - **`Traversable`**: Effectful traversal and filtering with validation
//! - **`PersistentVector`**: Immutable collections with structural sharing
//!
//! # Endpoints
//!
//! - `GET /tasks` - List tasks with pagination and filtering (uses `Traversable`)
//! - `GET /tasks/search` - Search tasks by title or tags
//! - `GET /tasks/by-priority` - Count tasks grouped by priority
//!
//! # Traversable Usage
//!
//! The `GET /tasks` endpoint demonstrates `Traversable::traverse_option` for:
//! - Filtering tasks by lifting filter predicates into the Option effect boundary
//! - Paginating with validation-aware transformation via `traverse_option`
//!
//! This pattern enables composition with other effectful operations and maintains
//! type safety through the effect boundary.
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
use lambars::persistent::{PersistentHashSet, PersistentTreeMap, PersistentVector};
use lambars::typeclass::{Semigroup, Traversable};

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
// Search Index with PersistentTreeMap
// =============================================================================

/// Search index using `PersistentTreeMap` for efficient prefix-based lookup.
///
/// The index maps normalized search terms (lowercase) to the tasks that
/// contain those terms. This enables efficient search operations without
/// full O(N) scans.
///
/// # Design Note
///
/// This implementation demonstrates:
/// - **`PersistentTreeMap`**: Efficient ordered map for prefix-based queries using `range`
/// - **`Semigroup::combine`**: Combining search results from different scopes with deduplication
///
/// # Index Strategy
///
/// - **Word index**: Maps individual words (split by whitespace) for efficient prefix search
/// - **Full title index**: Maps the complete normalized title to multiple task IDs
/// - **Word all-suffix index**: Maps ALL suffixes of each word for arbitrary position substring matching
///   - Example: `callback` generates suffixes: `callback`, `allback`, `llback`, `lback`, `back`, `ack`, `ck`, `k`
///   - This allows `all` query to match `callback` via `allback` prefix match
/// - **Tag index**: Maps normalized tag values for tag search
/// - **Tag all-suffix index**: Maps ALL suffixes of each tag for arbitrary position substring matching
/// - **Task order**: Preserves original task order for stable result ordering
///
/// # Complexity Analysis
///
/// - **Range query (index lookup)**: O(log N + m) where m is matching entries in index
/// - **ID resolution**: O(k log N) where k is matching tasks, N is total tasks
/// - **Result ordering**: O(k log k) for sorting by original order
/// - **Total search complexity**: O(k log N + k log k)
/// - No full O(N) scan is required for any search operation.
/// - **Space trade-off**: Stores O(L * W) entries where L is average word length and W is word count
#[derive(Debug, Clone)]
pub struct SearchIndex {
    /// Index mapping normalized title words to task IDs (for prefix search).
    title_word_index: PersistentTreeMap<String, PersistentVector<TaskId>>,
    /// Index mapping full normalized titles to task IDs (for multi-word substring match).
    /// Changed from `TaskId` to `PersistentVector<TaskId>` to support multiple tasks with same title.
    title_full_index: PersistentTreeMap<String, PersistentVector<TaskId>>,
    /// Index mapping ALL suffixes of normalized title words to task IDs (for arbitrary infix search).
    /// Example: `callback` generates `callback`, `allback`, `llback`, `lback`, `back`, `ack`, `ck`, `k`.
    /// This enables `all` query to match `callback` via `allback` prefix match.
    title_word_all_suffix_index: PersistentTreeMap<String, PersistentVector<TaskId>>,
    /// Index mapping normalized tag values to task IDs.
    tag_index: PersistentTreeMap<String, PersistentVector<TaskId>>,
    /// Index mapping ALL suffixes of normalized tag values to task IDs (for arbitrary infix search).
    tag_all_suffix_index: PersistentTreeMap<String, PersistentVector<TaskId>>,
    /// Reference to all tasks for lookup by ID.
    tasks_by_id: PersistentTreeMap<TaskId, Task>,
    /// Original task order for stable result ordering (`task_id` -> position).
    task_order: PersistentTreeMap<TaskId, usize>,
}

impl SearchIndex {
    /// Builds a search index from a collection of tasks (pure function).
    ///
    /// Creates normalized indexes for both title words and tags.
    /// Also creates all-suffix indexes for arbitrary position substring matching.
    /// Preserves task order for stable result ordering.
    #[must_use]
    pub fn build(tasks: &PersistentVector<Task>) -> Self {
        let mut title_word_index: PersistentTreeMap<String, PersistentVector<TaskId>> =
            PersistentTreeMap::new();
        let mut title_full_index: PersistentTreeMap<String, PersistentVector<TaskId>> =
            PersistentTreeMap::new();
        let mut title_word_all_suffix_index: PersistentTreeMap<String, PersistentVector<TaskId>> =
            PersistentTreeMap::new();
        let mut tag_index: PersistentTreeMap<String, PersistentVector<TaskId>> =
            PersistentTreeMap::new();
        let mut tag_all_suffix_index: PersistentTreeMap<String, PersistentVector<TaskId>> =
            PersistentTreeMap::new();
        let mut tasks_by_id: PersistentTreeMap<TaskId, Task> = PersistentTreeMap::new();
        let mut task_order: PersistentTreeMap<TaskId, usize> = PersistentTreeMap::new();

        for (position, task) in tasks.iter().enumerate() {
            // Store task position for stable ordering
            task_order = task_order.insert(task.task_id.clone(), position);

            // Index the task by ID
            tasks_by_id = tasks_by_id.insert(task.task_id.clone(), task.clone());

            // Index full normalized title for multi-word substring match
            // Now stores PersistentVector<TaskId> to support multiple tasks with same title
            let normalized_title = task.title.to_lowercase();
            let existing_ids = title_full_index
                .get(&normalized_title)
                .cloned()
                .unwrap_or_else(PersistentVector::new);
            title_full_index = title_full_index.insert(
                normalized_title.clone(),
                existing_ids.push_back(task.task_id.clone()),
            );

            // Index title words (normalized to lowercase) for prefix search
            for word in normalized_title.split_whitespace() {
                let word_key = word.to_string();
                let task_ids = title_word_index
                    .get(&word_key)
                    .cloned()
                    .unwrap_or_else(PersistentVector::new);
                title_word_index = title_word_index
                    .insert(word_key.clone(), task_ids.push_back(task.task_id.clone()));

                // Index ALL suffixes of the word for arbitrary position infix search
                // "callback" -> ["callback", "allback", "llback", "lback", "back", "ack", "ck", "k"]
                title_word_all_suffix_index =
                    Self::index_all_suffixes(title_word_all_suffix_index, word, &task.task_id);
            }

            // Index tags (normalized to lowercase)
            for tag in &task.tags {
                let tag_key = tag.as_str().to_lowercase();
                let task_ids = tag_index
                    .get(&tag_key)
                    .cloned()
                    .unwrap_or_else(PersistentVector::new);
                tag_index =
                    tag_index.insert(tag_key.clone(), task_ids.push_back(task.task_id.clone()));

                // Index ALL suffixes of the tag for arbitrary position infix search
                tag_all_suffix_index =
                    Self::index_all_suffixes(tag_all_suffix_index, &tag_key, &task.task_id);
            }
        }

        Self {
            title_word_index,
            title_full_index,
            title_word_all_suffix_index,
            tag_index,
            tag_all_suffix_index,
            tasks_by_id,
            task_order,
        }
    }

    /// Indexes all suffixes of a word for arbitrary position substring matching.
    ///
    /// For "callback", this generates:
    /// - "callback" (full word - matches prefix "call")
    /// - "allback" (matches prefix "all")
    /// - "llback" (matches prefix "ll")
    /// - "lback" (matches prefix "l")
    /// - "back" (matches prefix "back")
    /// - "ack" (matches prefix "ack")
    /// - "ck" (matches prefix "ck")
    /// - "k" (matches prefix "k")
    ///
    /// This enables efficient infix search by converting infix to prefix lookup.
    /// Index operation: O(log N) per suffix insertion.
    fn index_all_suffixes(
        mut index: PersistentTreeMap<String, PersistentVector<TaskId>>,
        word: &str,
        task_id: &TaskId,
    ) -> PersistentTreeMap<String, PersistentVector<TaskId>> {
        // Generate all suffixes by taking substrings from each character position
        for (byte_index, _) in word.char_indices() {
            let suffix = &word[byte_index..];
            let existing_ids = index
                .get(suffix)
                .cloned()
                .unwrap_or_else(PersistentVector::new);
            index = index.insert(suffix.to_string(), existing_ids.push_back(task_id.clone()));
        }
        index
    }

    /// Searches the title index for tasks containing the query (pure function).
    ///
    /// Returns `Some(SearchResult)` if any matches are found, `None` otherwise.
    ///
    /// # Search Strategy
    ///
    /// 1. First, try full title substring match (for multi-word queries like "important meeting")
    /// 2. Then, use prefix-based word index search (for single word or prefix queries)
    /// 3. Combine results with deduplication, maintaining original task order
    #[must_use]
    pub fn search_by_title(&self, query: &str) -> Option<SearchResult> {
        let query_lower = query.to_lowercase();
        let matching_ids = self.find_matching_ids_from_title(&query_lower);

        if matching_ids.is_empty() {
            None
        } else {
            let tasks = self.resolve_task_ids_ordered(&matching_ids);
            Some(SearchResult::from_tasks(tasks))
        }
    }

    /// Searches the tag index for tasks containing the query (pure function).
    ///
    /// Returns `Some(SearchResult)` if any matches are found, `None` otherwise.
    #[must_use]
    pub fn search_by_tags(&self, query: &str) -> Option<SearchResult> {
        let query_lower = query.to_lowercase();
        let matching_ids = self.find_matching_ids_from_tags(&query_lower);

        if matching_ids.is_empty() {
            None
        } else {
            let tasks = self.resolve_task_ids_ordered(&matching_ids);
            Some(SearchResult::from_tasks(tasks))
        }
    }

    /// Finds task IDs from the title index that match the query (substring match).
    ///
    /// Uses a three-phase strategy:
    /// 1. Full title substring match using prefix range on full title index
    /// 2. Prefix-based range search on word index
    /// 3. Suffix-based range search on all-suffix index (for infix matches)
    ///
    /// # Complexity
    ///
    /// Each phase uses O(log N + m) range query where m is matching index entries.
    /// Combined with ID resolution and ordering, total is O(k log N + k log k).
    /// No O(N) full scan is performed.
    fn find_matching_ids_from_title(&self, query_lower: &str) -> PersistentHashSet<TaskId> {
        let mut matching_ids = PersistentHashSet::new();

        // Phase 1: Full title prefix search (for multi-word queries)
        // Use range query on title_full_index for O(log N + m) lookup
        // This finds titles that START WITH the query (e.g., "important meeting" in "important meeting tomorrow")
        matching_ids = Self::find_matching_ids_with_prefix_range_multi(
            &self.title_full_index,
            query_lower,
            matching_ids,
        );

        // Phase 2: Word index prefix search (for single word or prefix queries)
        // Finds words that START WITH the query (e.g., "imp" matches "important")
        matching_ids = Self::find_matching_ids_with_prefix_range(
            &self.title_word_index,
            query_lower,
            matching_ids,
        );

        // Phase 3: All-suffix index search (for arbitrary infix matches)
        // The all-suffix index contains ALL suffixes of each word, so we can find
        // any infix by prefix-searching on the suffix that starts with the query.
        // E.g., "all" matches "callback" because "allback" is in the index and starts with "all"
        matching_ids = Self::find_matching_ids_with_prefix_range(
            &self.title_word_all_suffix_index,
            query_lower,
            matching_ids,
        );

        matching_ids
    }

    /// Finds task IDs from the tag index that match the query (substring match).
    ///
    /// Uses prefix search on tag index and all-suffix index.
    /// Complexity: O(log N + m) per phase, total O(k log N + k log k) with ID resolution.
    fn find_matching_ids_from_tags(&self, query_lower: &str) -> PersistentHashSet<TaskId> {
        let mut matching_ids = PersistentHashSet::new();

        // Prefix search: finds tags starting with query
        matching_ids =
            Self::find_matching_ids_with_prefix_range(&self.tag_index, query_lower, matching_ids);

        // All-suffix search: finds tags containing query at any position
        // E.g., "cke" matches "backend" because "ckend" is in the all-suffix index
        matching_ids = Self::find_matching_ids_with_prefix_range(
            &self.tag_all_suffix_index,
            query_lower,
            matching_ids,
        );

        matching_ids
    }

    /// Uses `PersistentTreeMap::range` for efficient prefix-based search.
    ///
    /// Complexity: O(log N + m) where m is the number of matching index entries.
    fn find_matching_ids_with_prefix_range(
        index: &PersistentTreeMap<String, PersistentVector<TaskId>>,
        query_lower: &str,
        mut matching_ids: PersistentHashSet<TaskId>,
    ) -> PersistentHashSet<TaskId> {
        // For prefix search, we use range [query, query + char::MAX)
        // Using char::MAX ('\u{10ffff}') to cover all Unicode including BMP-external chars (emoji, etc.)
        let end_key = format!("{query_lower}\u{10ffff}");
        for (_key, task_ids) in index.range(query_lower.to_string()..end_key) {
            for task_id in task_ids {
                matching_ids = matching_ids.insert(task_id.clone());
            }
        }

        matching_ids
    }

    /// Uses `PersistentTreeMap::range` on full title index for prefix-based search.
    ///
    /// This variant handles `PersistentVector<TaskId>` values for same-title support.
    /// Complexity: O(k log N + k log k) where k is the number of matching tasks
    /// (log N for each `tasks_by_id` lookup, k log k for ordering sort).
    fn find_matching_ids_with_prefix_range_multi(
        index: &PersistentTreeMap<String, PersistentVector<TaskId>>,
        query_lower: &str,
        mut matching_ids: PersistentHashSet<TaskId>,
    ) -> PersistentHashSet<TaskId> {
        // For prefix search on full titles
        // Using char::MAX ('\u{10ffff}') to cover all Unicode including BMP-external chars (emoji, etc.)
        let end_key = format!("{query_lower}\u{10ffff}");
        for (_title, task_ids) in index.range(query_lower.to_string()..end_key) {
            for task_id in task_ids {
                matching_ids = matching_ids.insert(task_id.clone());
            }
        }

        matching_ids
    }

    /// Resolves task IDs to their corresponding Task objects, maintaining original order.
    ///
    /// This ensures stable result ordering based on task registration order.
    fn resolve_task_ids_ordered(
        &self,
        task_ids: &PersistentHashSet<TaskId>,
    ) -> PersistentVector<Task> {
        // Collect tasks with their positions
        let mut tasks_with_positions: Vec<(usize, Task)> = task_ids
            .iter()
            .filter_map(|id| {
                self.tasks_by_id.get(id).cloned().map(|task| {
                    let position = self.task_order.get(id).copied().unwrap_or(usize::MAX);
                    (position, task)
                })
            })
            .collect();

        // Sort by original position for stable ordering
        tasks_with_positions.sort_by_key(|(position, _)| *position);

        // Extract tasks in order
        tasks_with_positions
            .into_iter()
            .map(|(_, task)| task)
            .collect()
    }

    /// Returns all tasks when the query is empty, in original order.
    #[must_use]
    pub fn all_tasks(&self) -> PersistentVector<Task> {
        // Collect tasks with their positions
        let mut tasks_with_positions: Vec<(usize, Task)> = self
            .tasks_by_id
            .iter()
            .map(|(id, task)| {
                let position = self.task_order.get(id).copied().unwrap_or(usize::MAX);
                (position, task.clone())
            })
            .collect();

        // Sort by original position
        tasks_with_positions.sort_by_key(|(position, _)| *position);

        // Extract tasks in order
        tasks_with_positions
            .into_iter()
            .map(|(_, task)| task)
            .collect()
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
    // I/O boundary: Fetch all tasks from repository (use Pagination::all() for full dataset)
    let all_tasks = state
        .task_repository
        .list(Pagination::all())
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    // Convert to PersistentVector for functional operations
    let tasks: PersistentVector<Task> = all_tasks.items.into_iter().collect();

    // Pure computation: Filter and paginate
    let status_filter = query.status.map(TaskStatus::from);
    let priority_filter = query.priority.map(Priority::from);

    // Phase 1: Validate and filter tasks (propagates nil UUID errors)
    let filtered = filter_tasks(&tasks, status_filter, priority_filter).map_err(|error| {
        ApiErrorResponse::internal_error(format!("Data integrity error: {error}"))
    })?;

    // Phase 2: Paginate filtered tasks
    let response = paginate_tasks(&filtered, query.page, query.limit).map_err(|error| {
        ApiErrorResponse::internal_error(format!("Data integrity error: {error}"))
    })?;

    Ok(Json(response))
}

/// Task filter predicate that validates and returns matching tasks.
///
/// Returns `Some(task)` if the task matches all filter criteria,
/// `None` otherwise. This predicate is designed to be used with
/// `Iterator::filter_map` for filtering.
fn task_filter_predicate(
    task: Task,
    status_filter: Option<TaskStatus>,
    priority_filter: Option<Priority>,
) -> Option<Task> {
    let status_matches = status_filter.is_none_or(|s| task.status == s);
    let priority_matches = priority_filter.is_none_or(|p| task.priority == p);

    if status_matches && priority_matches {
        Some(task)
    } else {
        None
    }
}

/// Validates that all tasks have valid (non-nil) UUIDs using `Traversable::traverse_option`.
///
/// This function demonstrates `Traversable::traverse_option` for batch validation.
/// If all tasks pass validation, returns `Ok(tasks)`. If any task has a nil UUID,
/// returns `Err(PaginationValidationError)` with the invalid task ID.
///
/// # Arguments
///
/// * `tasks` - The collection of tasks to validate
///
/// # Returns
///
/// `Ok(Vec<Task>)` if all tasks are valid, `Err(PaginationValidationError)` if any task has a nil UUID.
fn validate_tasks_with_traversable(tasks: &[Task]) -> Result<Vec<Task>, PaginationValidationError> {
    // First, try traverse_option to check if all are valid
    let validated = tasks.to_owned().traverse_option(|task| {
        if task.task_id.as_uuid().is_nil() {
            None
        } else {
            Some(task)
        }
    });

    validated.ok_or_else(|| {
        // Find the first invalid task for error reporting
        let invalid_task = tasks
            .iter()
            .find(|task| task.task_id.as_uuid().is_nil())
            .expect("traverse_option returned None, so there must be an invalid task");
        PaginationValidationError {
            invalid_task_id: invalid_task.task_id.clone(),
            message: "Task has invalid (nil) UUID".to_string(),
        }
    })
}

/// Filters tasks by status and priority with validation using Traversable.
///
/// This function demonstrates a two-phase approach:
/// 1. **Validation phase**: Uses `Traversable::traverse_option` to validate all tasks
///    have valid (non-nil) UUIDs. This demonstrates the all-or-nothing semantics
///    of `traverse_option`.
/// 2. **Filtering phase**: Uses `Iterator::filter_map` to apply filter predicates.
///
/// Note: `traverse_option` has "all succeed or all fail" semantics, which differs
/// from filtering. We use it for validation, then use `filter_map` for filtering.
///
/// # Arguments
///
/// * `tasks` - The collection of tasks to filter
/// * `status_filter` - Optional status to filter by
/// * `priority_filter` - Optional priority to filter by
///
/// # Returns
///
/// - `Ok(PersistentVector<Task>)` containing only the tasks that:
///   1. Have valid (non-nil) UUIDs (validation via `traverse_option`)
///   2. Match all filter criteria (filtering via `filter_map`)
/// - `Err(PaginationValidationError)` if any task has a nil UUID
///
/// # Errors
///
/// Returns `Err(PaginationValidationError)` if any task has a nil UUID.
fn filter_tasks_with_traversable(
    tasks: &PersistentVector<Task>,
    status_filter: Option<TaskStatus>,
    priority_filter: Option<Priority>,
) -> Result<PersistentVector<Task>, PaginationValidationError> {
    // Phase 1: Validate all tasks have valid UUIDs using traverse_option
    let task_vec: Vec<Task> = tasks.iter().cloned().collect();
    let valid_tasks = validate_tasks_with_traversable(&task_vec)?;

    // Phase 2: Apply filter predicates using filter_map
    Ok(valid_tasks
        .into_iter()
        .filter_map(|task| task_filter_predicate(task, status_filter, priority_filter))
        .collect())
}

/// Filters tasks by status and priority (pure function).
///
/// This function wraps `filter_tasks_with_traversable` and propagates validation errors.
///
/// # Errors
///
/// Returns `Err(PaginationValidationError)` if any task has an invalid (nil) UUID.
fn filter_tasks(
    tasks: &PersistentVector<Task>,
    status_filter: Option<TaskStatus>,
    priority_filter: Option<Priority>,
) -> Result<PersistentVector<Task>, PaginationValidationError> {
    filter_tasks_with_traversable(tasks, status_filter, priority_filter)
}

/// Error type for pagination validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaginationValidationError {
    /// The task ID that failed validation.
    pub invalid_task_id: TaskId,
    /// Human-readable error message.
    pub message: String,
}

impl std::fmt::Display for PaginationValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.message, self.invalid_task_id)
    }
}

impl std::error::Error for PaginationValidationError {}

/// Paginates a vector of tasks using Traversable for effectful transformation.
///
/// This function demonstrates `Traversable::traverse_option` for transforming
/// tasks within a page range. The traverse operation applies a validation-aware
/// transformation that lifts results into the Option effect boundary.
///
/// # Arguments
///
/// * `tasks` - The collection of tasks to paginate
/// * `page` - The page number (1-based, clamped to minimum 1)
/// * `limit` - Items per page (clamped to range 1-100)
///
/// # Returns
///
/// - `Ok(PaginatedResponse)` - Successfully paginated and transformed tasks
/// - `Err(PaginationValidationError)` - A task has an invalid (nil) UUID
///
/// # Errors
///
/// Returns `Err` if any task in the page has a nil UUID. This ensures that
/// `total` and `total_pages` remain consistent with the actual data returned.
fn paginate_tasks(
    tasks: &PersistentVector<Task>,
    page: u32,
    limit: u32,
) -> Result<PaginatedResponse<TaskResponse>, PaginationValidationError> {
    // Clamp inputs to valid ranges
    let page = page.max(1);
    let limit = limit.clamp(1, 100);

    // Use saturating arithmetic to prevent overflow
    // If (page - 1) * limit would overflow, saturating_mul returns u32::MAX
    // which effectively means "skip all items" - a safe behavior for edge cases
    let offset = (page - 1).saturating_mul(limit) as usize;
    let total = tasks.len() as u64;

    // Extract page slice as Vec for Traversable operations
    let page_tasks: Vec<Task> = tasks
        .iter()
        .skip(offset)
        .take(limit as usize)
        .cloned()
        .collect();

    // Use Traversable::traverse_option to transform tasks with validation.
    // The transformation validates each task has a non-nil ID and converts
    // to TaskResponse, lifting the result into Option effect boundary.
    //
    // If any task fails validation, we need to find which one and return an error
    // with proper context for debugging.
    let traverse_result = page_tasks.clone().traverse_option(|task| {
        if task.task_id.as_uuid().is_nil() {
            None
        } else {
            Some(TaskResponse::from(&task))
        }
    });

    let data: Vec<TaskResponse> = if let Some(responses) = traverse_result {
        responses
    } else {
        // Find the first invalid task for error reporting
        let invalid_task = page_tasks
            .iter()
            .find(|task| task.task_id.as_uuid().is_nil())
            .expect("traverse_option returned None, so there must be an invalid task");
        return Err(PaginationValidationError {
            invalid_task_id: invalid_task.task_id.clone(),
            message: "Task has invalid (nil) UUID".to_string(),
        });
    };

    // Calculate total pages (0 if no items)
    let total_pages = if total == 0 {
        0
    } else {
        total.div_ceil(u64::from(limit))
    };

    Ok(PaginatedResponse {
        data,
        page,
        limit,
        total,
        total_pages,
    })
}

// =============================================================================
// GET /tasks/search - Search Tasks
// =============================================================================

/// Searches tasks by title or tags.
///
/// This handler demonstrates:
/// - **`PersistentTreeMap`**: Building normalized search indexes for efficient lookup
/// - **`Semigroup::combine`**: Combining search results with deduplication
/// - **Deduplication**: Using `Semigroup::combine` for merging overlapping results
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
    // I/O boundary: Fetch all tasks from repository (use Pagination::all() for full dataset)
    let all_tasks = state
        .task_repository
        .list(Pagination::all())
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    // Convert to PersistentVector for functional operations
    let tasks: PersistentVector<Task> = all_tasks.items.into_iter().collect();

    // Build search index using PersistentTreeMap
    let index = SearchIndex::build(&tasks);

    // Pure computation: Search with scope using Semigroup::combine
    let results = search_with_scope_indexed(&index, &query.q, query.scope);

    // Convert to response
    let response: Vec<TaskResponse> = results
        .into_tasks()
        .iter()
        .map(TaskResponse::from)
        .collect();

    Ok(Json(response))
}

/// Searches tasks based on query and scope using index (pure function).
///
/// Uses `PersistentTreeMap`-based index for efficient lookup and
/// `Semigroup::combine` for combining search results from different scopes.
fn search_with_scope_indexed(index: &SearchIndex, query: &str, scope: SearchScope) -> SearchResult {
    // Empty query returns all tasks (early return)
    if query.is_empty() {
        return SearchResult::from_tasks(index.all_tasks());
    }

    match scope {
        SearchScope::Title => {
            // Search title index only, fallback to empty result
            index
                .search_by_title(query)
                .unwrap_or_else(SearchResult::empty)
        }
        SearchScope::Tags => {
            // Search tag index only, fallback to empty result
            index
                .search_by_tags(query)
                .unwrap_or_else(SearchResult::empty)
        }
        SearchScope::All => {
            // Combine title and tag search results using Semigroup.
            // Title matches get priority (appear first), then tag matches are added.
            let title_result = index.search_by_title(query);
            let tag_result = index.search_by_tags(query);

            match (title_result, tag_result) {
                (Some(title), Some(tags)) => {
                    // Both have results - combine with deduplication (title first)
                    title.combine(tags)
                }
                (Some(title), None) => title,
                (None, Some(tags)) => tags,
                (None, None) => SearchResult::empty(),
            }
        }
    }
}

/// Searches tasks based on query and scope (pure function).
///
/// Legacy implementation using direct iteration. Kept for reference and
/// backward compatibility in tests.
#[cfg(test)]
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
#[cfg(test)]
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
#[cfg(test)]
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
    // I/O boundary: Fetch all tasks from repository (use Pagination::all() for full dataset)
    let all_tasks = state
        .task_repository
        .list(Pagination::all())
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
        tags.iter()
            .fold(base, |task, tag| task.add_tag(Tag::new(*tag)))
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

        let filtered = filter_tasks(&tasks, None, None).expect("should succeed with valid UUIDs");
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

        let filtered = filter_tasks(&tasks, None, Some(Priority::Low))
            .expect("should succeed with valid UUIDs");
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

        let result = paginate_tasks(&tasks, 1, 10).expect("pagination should succeed");

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

        let result = paginate_tasks(&tasks, 3, 10).expect("pagination should succeed");

        assert_eq!(result.data.len(), 5);
        assert_eq!(result.page, 3);
        assert_eq!(result.total_pages, 3);
    }

    #[rstest]
    fn test_paginate_empty() {
        let tasks: PersistentVector<Task> = PersistentVector::new();

        let result = paginate_tasks(&tasks, 1, 10).expect("pagination should succeed");

        assert_eq!(result.data.len(), 0);
        assert_eq!(result.total, 0);
        assert_eq!(result.total_pages, 0);
    }

    #[rstest]
    fn test_paginate_clamps_page_zero() {
        let tasks: PersistentVector<Task> = (0..10)
            .map(|i| create_test_task(&format!("Task {i}"), Priority::Medium))
            .collect();

        let result = paginate_tasks(&tasks, 0, 10).expect("pagination should succeed");

        assert_eq!(result.page, 1);
    }

    #[rstest]
    fn test_paginate_clamps_limit() {
        let tasks: PersistentVector<Task> = (0..200)
            .map(|i| create_test_task(&format!("Task {i}"), Priority::Medium))
            .collect();

        let result = paginate_tasks(&tasks, 1, 200).expect("pagination should succeed");

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
        let task2_clone = task2.clone();

        let result1 = SearchResult::from_tasks(vec![task1, task2].into_iter().collect());
        let result2 =
            SearchResult::from_tasks(vec![task1_clone, task2_clone].into_iter().collect());

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
        let tasks: PersistentVector<Task> = vec![create_test_task_with_tags(
            "Task 1",
            Priority::Low,
            &["URGENT"],
        )]
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

        let tasks: PersistentVector<Task> = vec![tag_match, title_match].into_iter().collect();

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
        let result = paginate_tasks(&tasks, 5, 10).expect("pagination should succeed");

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

    // -------------------------------------------------------------------------
    // SearchIndex Tests (PersistentTreeMap + Semigroup)
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_search_index_build() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Important meeting", Priority::High),
            create_test_task_with_tags("Review code", Priority::Medium, &["backend", "rust"]),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);

        // Title word index should contain normalized words
        assert!(index.title_word_index.contains_key("important"));
        assert!(index.title_word_index.contains_key("meeting"));
        assert!(index.title_word_index.contains_key("review"));
        assert!(index.title_word_index.contains_key("code"));

        // Tag index should contain normalized tags
        assert!(index.tag_index.contains_key("backend"));
        assert!(index.tag_index.contains_key("rust"));
    }

    #[rstest]
    fn test_search_index_search_by_title() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Important meeting", Priority::High),
            create_test_task("Review code", Priority::Medium),
            create_test_task("Important deadline", Priority::Critical),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("important");

        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 2);
    }

    #[rstest]
    fn test_search_index_search_by_title_no_match() {
        let tasks: PersistentVector<Task> = vec![create_test_task("Hello world", Priority::Low)]
            .into_iter()
            .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("nonexistent");

        assert!(result.is_none());
    }

    #[rstest]
    fn test_search_index_search_by_tags() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task_with_tags("Task 1", Priority::Low, &["backend", "rust"]),
            create_test_task_with_tags("Task 2", Priority::Medium, &["frontend", "typescript"]),
            create_test_task_with_tags("Task 3", Priority::High, &["backend", "go"]),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_tags("backend");

        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 2);
    }

    #[rstest]
    fn test_search_index_search_by_tags_no_match() {
        let tasks: PersistentVector<Task> = vec![create_test_task_with_tags(
            "Task 1",
            Priority::Low,
            &["backend"],
        )]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_tags("frontend");

        assert!(result.is_none());
    }

    #[rstest]
    fn test_search_with_scope_indexed_title() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Important meeting", Priority::High),
            create_test_task("Regular task", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = search_with_scope_indexed(&index, "important", SearchScope::Title);

        assert_eq!(result.tasks.len(), 1);
    }

    #[rstest]
    fn test_search_with_scope_indexed_tags() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task_with_tags("Task 1", Priority::Low, &["urgent"]),
            create_test_task_with_tags("Task 2", Priority::Medium, &["normal"]),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = search_with_scope_indexed(&index, "urgent", SearchScope::Tags);

        assert_eq!(result.tasks.len(), 1);
    }

    #[rstest]
    fn test_search_with_scope_indexed_all_uses_alternative() {
        // Task with "important" in title only
        let title_match = create_test_task("Important meeting", Priority::High);
        // Task with "important" in tag only
        let tag_match = create_test_task_with_tags("Regular task", Priority::Low, &["important"]);

        let tasks: PersistentVector<Task> = vec![tag_match, title_match].into_iter().collect();

        let index = SearchIndex::build(&tasks);
        let result = search_with_scope_indexed(&index, "important", SearchScope::All);

        // Both tasks should be found (title and tag matches combined)
        assert_eq!(result.tasks.len(), 2);
    }

    #[rstest]
    fn test_search_with_scope_indexed_all_deduplicates() {
        // Task that matches both title and tag
        let both_match =
            create_test_task_with_tags("Important meeting", Priority::High, &["important"]);

        let tasks: PersistentVector<Task> = vec![both_match].into_iter().collect();

        let index = SearchIndex::build(&tasks);
        let result = search_with_scope_indexed(&index, "important", SearchScope::All);

        // Should have 1 result, not 2 (deduplicated)
        assert_eq!(result.tasks.len(), 1);
    }

    #[rstest]
    fn test_search_with_scope_indexed_empty_query() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::High),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = search_with_scope_indexed(&index, "", SearchScope::All);

        // Empty query returns all tasks
        assert_eq!(result.tasks.len(), 2);
    }

    #[rstest]
    fn test_search_with_scope_indexed_title_only_fallback() {
        // Test that when title search returns None, the result is empty (not tags)
        let tasks: PersistentVector<Task> = vec![create_test_task_with_tags(
            "Normal task",
            Priority::Low,
            &["urgent"],
        )]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = search_with_scope_indexed(&index, "urgent", SearchScope::Title);

        // Title doesn't contain "urgent", so result should be empty
        assert!(result.tasks.is_empty());
    }

    #[rstest]
    fn test_search_with_scope_indexed_tags_only_fallback() {
        // Test that when tag search returns None, the result is empty (not title)
        let tasks: PersistentVector<Task> = vec![create_test_task("Urgent task", Priority::High)]
            .into_iter()
            .collect();

        let index = SearchIndex::build(&tasks);
        let result = search_with_scope_indexed(&index, "urgent", SearchScope::Tags);

        // Tags are empty, so result should be empty
        assert!(result.tasks.is_empty());
    }

    #[rstest]
    fn test_search_index_all_tasks() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::High),
            create_test_task("Task 3", Priority::Medium),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let all = index.all_tasks();

        assert_eq!(all.len(), 3);
    }

    // -------------------------------------------------------------------------
    // Regression Tests (Codex Review Fix #225)
    // -------------------------------------------------------------------------

    /// Regression test: Multi-word query with space should match substring.
    /// E.g., query "important meeting" should match title "Important meeting".
    #[rstest]
    fn test_search_multi_word_query_matches_substring() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Important meeting tomorrow", Priority::High),
            create_test_task("Review code", Priority::Medium),
            create_test_task("Important deadline", Priority::Critical),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("important meeting");

        // "important meeting" should match "Important meeting tomorrow"
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 1);
        assert!(result.tasks.iter().any(|t| t.title.contains("meeting")));
    }

    /// Regression test: Result order should be stable across multiple searches.
    #[rstest]
    fn test_search_result_order_stability() {
        // Create tasks with predictable task_ids for stable ordering
        let task1 = create_test_task("Important task A", Priority::High);
        let task2 = create_test_task("Important task B", Priority::Medium);
        let task3 = create_test_task("Important task C", Priority::Low);

        let tasks: PersistentVector<Task> = vec![task1, task2, task3].into_iter().collect();

        let index = SearchIndex::build(&tasks);

        // Run the same search multiple times
        let results: Vec<Vec<TaskId>> = (0..5)
            .map(|_| {
                let result = index.search_by_title("important");
                result
                    .map(|r| r.tasks.iter().map(|t| t.task_id.clone()).collect())
                    .unwrap_or_default()
            })
            .collect();

        // All results should have the same order
        for i in 1..results.len() {
            assert_eq!(
                results[0], results[i],
                "Search results should be stable across multiple calls"
            );
        }
    }

    /// Regression test: Prefix-based search should still work.
    /// E.g., query "imp" should match "important".
    #[rstest]
    fn test_search_prefix_matches() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Important meeting", Priority::High),
            create_test_task("Impossible task", Priority::Medium),
            create_test_task("Regular task", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("imp");

        // "imp" should match "Important" and "Impossible"
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 2);
    }

    /// Regression test: Partial word match should work for tags.
    /// E.g., query "back" should match tag "backend".
    #[rstest]
    fn test_search_tag_partial_match() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task_with_tags("Task 1", Priority::Low, &["backend"]),
            create_test_task_with_tags("Task 2", Priority::Medium, &["frontend"]),
            create_test_task_with_tags("Task 3", Priority::High, &["backlog"]),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_tags("back");

        // "back" should match "backend" and "backlog"
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 2);
    }

    /// Regression test: All scope search should maintain task order.
    /// Title matches should come before tag matches.
    #[rstest]
    fn test_search_all_scope_title_matches_first() {
        // Task with "urgent" in title only
        let title_match = create_test_task("Urgent meeting", Priority::High);
        // Task with "urgent" in tag only
        let tag_match = create_test_task_with_tags("Regular task", Priority::Low, &["urgent"]);

        let tasks: PersistentVector<Task> = vec![tag_match, title_match].into_iter().collect();

        let index = SearchIndex::build(&tasks);
        let result = search_with_scope_indexed(&index, "urgent", SearchScope::All);

        assert_eq!(result.tasks.len(), 2);
        // Title match should come first
        let first = result.tasks.iter().next().unwrap();
        assert!(
            first.title.to_lowercase().contains("urgent"),
            "Title matches should appear before tag matches"
        );
    }

    /// Regression test: Search should handle empty title gracefully.
    #[rstest]
    fn test_search_handles_empty_title() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("", Priority::Low),
            create_test_task("Regular task", Priority::Medium),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("regular");

        assert!(result.is_some());
        assert_eq!(result.unwrap().tasks.len(), 1);
    }

    /// Regression test: Search should handle special characters in query.
    #[rstest]
    fn test_search_handles_special_characters() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Fix bug #123", Priority::High),
            create_test_task("Update README.md", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);

        // Search for "#123" - should not cause errors
        let result = index.search_by_title("#123");
        // May or may not find results depending on tokenization, but should not crash
        assert!(result.is_none() || result.is_some());
    }

    // -------------------------------------------------------------------------
    // Phase 1.1c: Same Title Multiple Tasks Tests (Codex Review #225)
    // -------------------------------------------------------------------------

    /// Regression test: Multiple tasks with the same title should all be found.
    /// This tests the fix for `title_full_index` overwriting issue.
    #[rstest]
    fn test_search_same_title_multiple_tasks() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Important meeting", Priority::High),
            create_test_task("Important meeting", Priority::Medium),
            create_test_task("Important meeting", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("important meeting");

        // All 3 tasks with the same title should be found
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            3,
            "All tasks with the same title should be returned"
        );
    }

    /// Regression test: Multi-word search should find all tasks with matching titles.
    #[rstest]
    fn test_search_multi_word_same_title_all_found() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Review code changes", Priority::High),
            create_test_task("Review code changes", Priority::Medium),
            create_test_task("Review API design", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("review code");

        // Both "Review code changes" tasks should be found
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            2,
            "Both tasks with 'review code' in title should be found"
        );
    }

    /// Regression test: Full substring search should work with same titles.
    #[rstest]
    fn test_search_substring_same_title() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Backend API implementation", Priority::High),
            create_test_task("Backend API implementation", Priority::Medium),
            create_test_task("Frontend implementation", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("backend api");

        // Both "Backend API implementation" tasks should be found
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            2,
            "Both tasks with 'backend api' in title should be found"
        );
    }

    /// Regression test: Contains search should not require full scan.
    /// E.g., "callback" should match "Add callback handler" without scanning all entries.
    #[rstest]
    fn test_search_contains_match_efficiency() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Add callback handler", Priority::High),
            create_test_task("Update backend service", Priority::Medium),
            create_test_task("Implement callback logic", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("callback");

        // Both tasks containing "callback" should be found
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            2,
            "Both tasks containing 'callback' should be found"
        );
    }

    /// Regression test: Infix search (not prefix) should work with reversed index.
    /// E.g., "back" should match both "backend" and "callback"
    #[rstest]
    fn test_search_infix_match_both_directions() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Update backend", Priority::High),
            create_test_task("Add callback", Priority::Medium),
            create_test_task("Fix feedback", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("back");

        // "backend", "callback", and "feedback" all contain "back"
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            3,
            "All tasks containing 'back' should be found"
        );
    }

    /// Regression test: Same tag multiple tasks should all be found.
    #[rstest]
    fn test_search_same_tag_multiple_tasks() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task_with_tags("Task 1", Priority::High, &["backend"]),
            create_test_task_with_tags("Task 2", Priority::Medium, &["backend"]),
            create_test_task_with_tags("Task 3", Priority::Low, &["backend"]),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_tags("backend");

        // All 3 tasks with the same tag should be found
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            3,
            "All tasks with the same tag should be returned"
        );
    }

    // -------------------------------------------------------------------------
    // Phase 1.1c: Infix Substring Match Tests (Codex Review #225)
    // -------------------------------------------------------------------------

    /// Infix match test: "callback" should be found when searching for "all".
    /// This tests the full suffix index that enables arbitrary position matching.
    #[rstest]
    fn test_search_infix_match_all_in_callback() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Handle callback events", Priority::High),
            create_test_task("Regular task", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("all");

        // "all" appears in the middle of "callback"
        assert!(result.is_some(), "Query 'all' should match 'callback'");
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            1,
            "Only the task with 'callback' should match"
        );
        assert!(result.tasks.iter().any(|t| t.title.contains("callback")));
    }

    /// Infix match test: "callback" should be found when searching for "llb".
    /// Note: "callback" contains "llb" in "ca[llb]ack" - wait, let's verify.
    /// "callback" = c-a-l-l-b-a-c-k. The substring "llb" is at positions 2-4.
    #[rstest]
    fn test_search_infix_match_llb_in_callback() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Implement callback handler", Priority::High),
            create_test_task("Other task", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("llb");

        // "llb" appears in the middle of "callback" (ca-llb-ack)
        assert!(result.is_some(), "Query 'llb' should match 'callback'");
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 1);
    }

    /// Infix match test: "callback" should be found when searching for "llba".
    /// Note: "llba" does NOT appear in "callback" (it's "llba" vs "llba").
    /// Actually "callback" contains "allb" but not "llba". Let's test "allb" instead.
    #[rstest]
    fn test_search_infix_match_allb_in_callback() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Fix callback issue", Priority::High),
            create_test_task("Normal task", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("allb");

        // "allb" appears in "callback"
        assert!(result.is_some(), "Query 'allb' should match 'callback'");
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 1);
    }

    /// Infix match test for tags: "backend" should be found when searching for "ack".
    #[rstest]
    fn test_search_tag_infix_match_ack_in_backend() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task_with_tags("Task 1", Priority::High, &["backend"]),
            create_test_task_with_tags("Task 2", Priority::Low, &["frontend"]),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_tags("cke");

        // "cke" appears in the middle of "backend"
        assert!(result.is_some(), "Query 'cke' should match 'backend'");
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 1);
    }

    /// Infix match test for tags: "callback" tag should be found when searching for "llb".
    #[rstest]
    fn test_search_tag_infix_match_llb_in_callback() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task_with_tags("Task 1", Priority::High, &["callback"]),
            create_test_task_with_tags("Task 2", Priority::Low, &["regular"]),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_tags("llb");

        // "llb" appears in the middle of "callback"
        assert!(result.is_some(), "Query 'llb' should match 'callback'");
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 1);
    }

    /// Infix match should work for multiple words in title.
    #[rstest]
    fn test_search_infix_match_multiple_words() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Important callback implementation", Priority::High),
            create_test_task("Frontend development", Priority::Medium),
            create_test_task("Feedback processing", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);

        // "all" should match "callback"
        let result_all = index.search_by_title("all");
        assert!(result_all.is_some());
        assert_eq!(result_all.unwrap().tasks.len(), 1);

        // "ort" should match "important"
        let result_ort = index.search_by_title("ort");
        assert!(result_ort.is_some());
        assert_eq!(result_ort.unwrap().tasks.len(), 1);

        // "ple" should match "implementation"
        let result_ple = index.search_by_title("ple");
        assert!(result_ple.is_some());
        assert_eq!(result_ple.unwrap().tasks.len(), 1);
    }

    // -------------------------------------------------------------------------
    // Phase 1.1c: BMP外文字（絵文字）検索テスト (Codex Review #225)
    // -------------------------------------------------------------------------

    /// BMP外文字テスト: 絵文字を含むタイトルで "call" を検索すると "call😀back" がヒットする。
    /// UTF-8 の `char_indices` で正しくバイト境界を処理することを確認。
    #[rstest]
    fn test_search_title_with_emoji_call_in_callback_emoji() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("call😀back handler", Priority::High),
            create_test_task("Regular callback", Priority::Medium),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("call");

        // "call" should match both "call😀back" and "callback"
        assert!(result.is_some(), "Query 'call' should match 'call😀back'");
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            2,
            "Both tasks containing 'call' should be found"
        );
    }

    /// BMP外文字テスト: 絵文字を含むタイトルで "task" を検索すると "task🎉done" がヒットする。
    #[rstest]
    fn test_search_title_with_emoji_task_in_task_emoji_done() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("task🎉done celebration", Priority::High),
            create_test_task("Normal task item", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("task");

        // "task" should match both "task🎉done" and "Normal task item"
        assert!(result.is_some(), "Query 'task' should match 'task🎉done'");
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            2,
            "Both tasks containing 'task' should be found"
        );
    }

    /// BMP外文字テスト: 絵文字を含むタグで "emoji" を検索すると "emoji😀tag" がヒットする。
    #[rstest]
    fn test_search_tag_with_emoji_emoji_in_emoji_tag() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task_with_tags("Task with emoji tag", Priority::High, &["emoji😀tag"]),
            create_test_task_with_tags("Task with normal tag", Priority::Low, &["normal"]),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_tags("emoji");

        // "emoji" should match "emoji😀tag"
        assert!(result.is_some(), "Query 'emoji' should match 'emoji😀tag'");
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            1,
            "Only the task with 'emoji😀tag' should be found"
        );
    }

    /// BMP外文字テスト: 絵文字の後ろの文字列 "back" で "call😀back" を検索できる。
    #[rstest]
    fn test_search_title_with_emoji_suffix_back_in_callback_emoji() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("call😀back function", Priority::High),
            create_test_task("Other task", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("back");

        // "back" should match "call😀back" via suffix index
        assert!(
            result.is_some(),
            "Query 'back' should match 'call😀back' via suffix index"
        );
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 1);
        assert!(result.tasks.iter().any(|t| t.title.contains("call😀back")));
    }

    /// BMP外文字テスト: 絵文字の後ろの文字列 "tag" で "emoji😀tag" タグを検索できる。
    #[rstest]
    fn test_search_tag_with_emoji_suffix_tag_in_emoji_tag() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task_with_tags("Task 1", Priority::High, &["emoji😀tag"]),
            create_test_task_with_tags("Task 2", Priority::Low, &["regular"]),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_tags("tag");

        // "tag" should match "emoji😀tag" via suffix index
        assert!(
            result.is_some(),
            "Query 'tag' should match 'emoji😀tag' via suffix index"
        );
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 1);
    }

    /// BMP外文字テスト: 複数の絵文字を含むタイトルでも正しく検索できる。
    #[rstest]
    fn test_search_title_with_multiple_emojis() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("🚀rocket🌟launch🎯target", Priority::High),
            create_test_task("Normal title", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);

        // "rocket" should match
        let result_rocket = index.search_by_title("rocket");
        assert!(
            result_rocket.is_some(),
            "Query 'rocket' should match '🚀rocket🌟launch🎯target'"
        );
        assert_eq!(result_rocket.unwrap().tasks.len(), 1);

        // "launch" should match
        let result_launch = index.search_by_title("launch");
        assert!(
            result_launch.is_some(),
            "Query 'launch' should match '🚀rocket🌟launch🎯target'"
        );
        assert_eq!(result_launch.unwrap().tasks.len(), 1);

        // "target" should match
        let result_target = index.search_by_title("target");
        assert!(
            result_target.is_some(),
            "Query 'target' should match '🚀rocket🌟launch🎯target'"
        );
        assert_eq!(result_target.unwrap().tasks.len(), 1);
    }

    // -------------------------------------------------------------------------
    // Phase 1.1g: Traversable Type Class Tests
    // -------------------------------------------------------------------------

    /// Test that `filter_tasks_with_traversable` correctly filters by status.
    #[rstest]
    fn test_filter_tasks_with_traversable_by_status() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::High).complete(),
            create_test_task("Task 3", Priority::Medium),
        ]
        .into_iter()
        .collect();

        let filtered = filter_tasks_with_traversable(&tasks, Some(TaskStatus::Pending), None)
            .expect("should succeed with valid UUIDs");
        assert_eq!(filtered.len(), 2, "Should filter out completed task");
    }

    /// Test that `filter_tasks_with_traversable` correctly filters by priority.
    #[rstest]
    fn test_filter_tasks_with_traversable_by_priority() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::High),
            create_test_task("Task 3", Priority::Low),
            create_test_task("Task 4", Priority::Critical),
        ]
        .into_iter()
        .collect();

        let filtered = filter_tasks_with_traversable(&tasks, None, Some(Priority::Low))
            .expect("should succeed with valid UUIDs");
        assert_eq!(
            filtered.len(),
            2,
            "Should filter to only Low priority tasks"
        );
    }

    /// Test that `filter_tasks_with_traversable` correctly filters by both status and priority.
    #[rstest]
    fn test_filter_tasks_with_traversable_by_status_and_priority() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::High).complete(),
            create_test_task("Task 3", Priority::Low).complete(),
            create_test_task("Task 4", Priority::Low),
        ]
        .into_iter()
        .collect();

        let filtered =
            filter_tasks_with_traversable(&tasks, Some(TaskStatus::Pending), Some(Priority::Low))
                .expect("should succeed with valid UUIDs");
        assert_eq!(
            filtered.len(),
            2,
            "Should filter to only pending Low priority tasks"
        );
    }

    /// Test that `filter_tasks_with_traversable` returns all tasks when no filters applied.
    #[rstest]
    fn test_filter_tasks_with_traversable_no_filter() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::High),
            create_test_task("Task 3", Priority::Medium),
        ]
        .into_iter()
        .collect();

        let filtered = filter_tasks_with_traversable(&tasks, None, None)
            .expect("should succeed with valid UUIDs");
        assert_eq!(filtered.len(), 3, "Should return all tasks when no filter");
    }

    /// Test that `task_filter_predicate` returns `Some` for matching tasks.
    #[rstest]
    fn test_task_filter_predicate_matches() {
        let task = create_test_task("Test Task", Priority::High);

        // No filter - should match
        let result = task_filter_predicate(task.clone(), None, None);
        assert!(result.is_some(), "Should match when no filters");

        // Matching priority filter
        let result = task_filter_predicate(task.clone(), None, Some(Priority::High));
        assert!(result.is_some(), "Should match when priority matches");

        // Matching status filter
        let result = task_filter_predicate(task, Some(TaskStatus::Pending), None);
        assert!(result.is_some(), "Should match when status matches");
    }

    /// Test that `task_filter_predicate` returns `None` for non-matching tasks.
    #[rstest]
    fn test_task_filter_predicate_not_matches() {
        let task = create_test_task("Test Task", Priority::High);

        // Non-matching priority
        let result = task_filter_predicate(task.clone(), None, Some(Priority::Low));
        assert!(result.is_none(), "Should not match when priority differs");

        // Non-matching status (pending task vs completed filter)
        let result = task_filter_predicate(task, Some(TaskStatus::Completed), None);
        assert!(result.is_none(), "Should not match when status differs");
    }

    /// Test `paginate_tasks` uses `Traversable::traverse_option` for transformation.
    #[rstest]
    fn test_paginate_tasks_with_traversable() {
        let tasks: PersistentVector<Task> = (0..25)
            .map(|i| create_test_task(&format!("Task {i}"), Priority::Medium))
            .collect();

        let result = paginate_tasks(&tasks, 1, 10).expect("pagination should succeed");

        assert_eq!(
            result.data.len(),
            10,
            "Should return 10 items on first page"
        );
        assert_eq!(result.page, 1);
        assert_eq!(result.limit, 10);
        assert_eq!(result.total, 25);
        assert_eq!(result.total_pages, 3);

        // Verify TaskResponse transformation via traverse_option
        for (index, response) in result.data.iter().enumerate() {
            assert_eq!(
                response.title,
                format!("Task {index}"),
                "Task should be correctly transformed"
            );
        }
    }

    /// Test that `paginate_tasks` handles empty input correctly with Traversable.
    #[rstest]
    fn test_paginate_tasks_with_traversable_empty() {
        let tasks: PersistentVector<Task> = PersistentVector::new();

        let result = paginate_tasks(&tasks, 1, 10).expect("pagination should succeed");

        assert!(
            result.data.is_empty(),
            "Should return empty data for empty input"
        );
        assert_eq!(result.total, 0);
        assert_eq!(result.total_pages, 0);
    }

    /// Test Traversable `traverse_option` directly on Vec with filter semantics.
    #[rstest]
    fn test_traversable_traverse_option_filter_pattern() {
        use lambars::typeclass::Traversable;

        let numbers = vec![1, 2, 3, 4, 5];

        // Use traverse_option to validate and transform in one pass
        // This returns Some only if ALL elements pass validation
        let all_positive: Option<Vec<i32>> =
            numbers.traverse_option(|n| if n > 0 { Some(n * 2) } else { None });
        assert_eq!(all_positive, Some(vec![2, 4, 6, 8, 10]));

        // If any element fails, the whole result is None
        let with_negative = vec![1, -2, 3];
        let result: Option<Vec<i32>> =
            with_negative.traverse_option(|n| if n > 0 { Some(n * 2) } else { None });
        assert_eq!(result, None, "Should return None if any validation fails");
    }

    /// Test Traversable `sequence_option` for turning `Vec<Option<T>>` into `Option<Vec<T>>`.
    #[rstest]
    fn test_traversable_sequence_option() {
        use lambars::typeclass::Traversable;

        // All Some values - should succeed
        let options: Vec<Option<i32>> = vec![Some(1), Some(2), Some(3)];
        let result: Option<Vec<i32>> = options.sequence_option();
        assert_eq!(result, Some(vec![1, 2, 3]));

        // Contains None - should fail
        let options_with_none: Vec<Option<i32>> = vec![Some(1), None, Some(3)];
        let result_with_none: Option<Vec<i32>> = options_with_none.sequence_option();
        assert_eq!(result_with_none, None);
    }

    /// Test that Traversable can be used for effectful validation in task processing.
    #[rstest]
    fn test_traversable_task_validation() {
        use lambars::typeclass::Traversable;

        let tasks: Vec<Task> = vec![
            create_test_task("Valid Task 1", Priority::High),
            create_test_task("Valid Task 2", Priority::Medium),
        ];

        // Validate all tasks have non-empty titles
        let validated: Option<Vec<String>> = tasks.traverse_option(|task| {
            if task.title.is_empty() {
                None
            } else {
                Some(task.title)
            }
        });

        assert!(validated.is_some(), "All tasks should pass validation");
        assert_eq!(validated.unwrap().len(), 2);
    }

    // -------------------------------------------------------------------------
    // Nil UUID Validation Tests
    // -------------------------------------------------------------------------

    /// Helper function to create a task with a nil UUID.
    fn create_task_with_nil_uuid(title: &str, priority: Priority) -> Task {
        Task::new(
            TaskId::from_uuid(uuid::Uuid::nil()),
            title,
            Timestamp::now(),
        )
        .with_priority(priority)
    }

    /// Test that `paginate_tasks` returns error when a task has nil UUID.
    #[rstest]
    fn test_paginate_tasks_with_nil_uuid_returns_error() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Valid Task 1", Priority::Low),
            create_task_with_nil_uuid("Invalid Task", Priority::Medium),
            create_test_task("Valid Task 2", Priority::High),
        ]
        .into_iter()
        .collect();

        let result = paginate_tasks(&tasks, 1, 10);

        assert!(result.is_err(), "Should return error when nil UUID present");
        let error = result.unwrap_err();
        assert!(
            error.message.contains("nil"),
            "Error message should mention nil UUID"
        );
    }

    /// Test that `paginate_tasks` succeeds when all tasks have valid UUIDs.
    #[rstest]
    fn test_paginate_tasks_without_nil_uuid_succeeds() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Valid Task 1", Priority::Low),
            create_test_task("Valid Task 2", Priority::Medium),
            create_test_task("Valid Task 3", Priority::High),
        ]
        .into_iter()
        .collect();

        let result = paginate_tasks(&tasks, 1, 10);

        assert!(result.is_ok(), "Should succeed when all UUIDs are valid");
        let response = result.unwrap();
        assert_eq!(response.data.len(), 3);
        assert_eq!(response.total, 3);
    }

    /// Test that `paginate_tasks` error contains the invalid task ID.
    #[rstest]
    fn test_paginate_tasks_error_contains_task_id() {
        let nil_uuid = uuid::Uuid::nil();
        let tasks: PersistentVector<Task> =
            vec![create_task_with_nil_uuid("Invalid Task", Priority::Medium)]
                .into_iter()
                .collect();

        let result = paginate_tasks(&tasks, 1, 10);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(
            *error.invalid_task_id.as_uuid(),
            nil_uuid,
            "Error should contain the invalid task ID"
        );
    }

    /// Test that `filter_tasks_with_traversable` returns error when nil UUID present.
    /// This test verifies that nil UUID validation error is propagated (not silently returning empty).
    #[rstest]
    fn test_filter_tasks_with_traversable_nil_uuid_returns_error() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Valid Task 1", Priority::Low),
            create_task_with_nil_uuid("Invalid Task", Priority::Medium),
            create_test_task("Valid Task 2", Priority::High),
        ]
        .into_iter()
        .collect();

        let result = filter_tasks_with_traversable(&tasks, None, None);

        // Should return Err when nil UUID present (not silently return empty)
        assert!(
            result.is_err(),
            "Should return error when nil UUID present in input"
        );
        let error = result.unwrap_err();
        assert!(
            error.invalid_task_id.as_uuid().is_nil(),
            "Error should contain the nil UUID task ID"
        );
    }

    /// Test that `validate_tasks_with_traversable` returns Err for nil UUID.
    #[rstest]
    fn test_validate_tasks_with_traversable_nil_uuid() {
        let tasks = vec![
            create_test_task("Valid Task", Priority::Low),
            create_task_with_nil_uuid("Invalid Task", Priority::Medium),
        ];

        let result = validate_tasks_with_traversable(&tasks);

        assert!(
            result.is_err(),
            "Should return Err when any task has nil UUID"
        );
        let error = result.unwrap_err();
        assert!(
            error.invalid_task_id.as_uuid().is_nil(),
            "Error should contain the nil UUID task ID"
        );
    }

    /// Test that `validate_tasks_with_traversable` returns Ok for valid UUIDs.
    #[rstest]
    fn test_validate_tasks_with_traversable_valid_uuids() {
        let tasks = vec![
            create_test_task("Valid Task 1", Priority::Low),
            create_test_task("Valid Task 2", Priority::High),
        ];

        let result = validate_tasks_with_traversable(&tasks);

        assert!(
            result.is_ok(),
            "Should return Ok when all tasks have valid UUIDs"
        );
        assert_eq!(result.unwrap().len(), 2);
    }

    /// Test that nil UUID on page boundary is correctly detected.
    #[rstest]
    fn test_paginate_tasks_nil_uuid_on_second_page() {
        // Create 15 valid tasks, then 1 invalid task, so the invalid one is on page 2
        let mut tasks: Vec<Task> = (0..15)
            .map(|i| create_test_task(&format!("Task {i}"), Priority::Medium))
            .collect();
        tasks.push(create_task_with_nil_uuid("Invalid Task", Priority::High));

        let tasks: PersistentVector<Task> = tasks.into_iter().collect();

        // Page 1 should succeed (only valid tasks)
        let page1_result = paginate_tasks(&tasks, 1, 10);
        assert!(page1_result.is_ok(), "Page 1 should succeed");

        // Page 2 should fail (contains invalid task)
        let page2_result = paginate_tasks(&tasks, 2, 10);
        assert!(page2_result.is_err(), "Page 2 should fail due to nil UUID");
    }

    // -------------------------------------------------------------------------
    // Phase 1.1g: Error Propagation Integration Tests (Codex Review #225)
    // -------------------------------------------------------------------------

    /// Integration test: `filter_tasks_with_traversable` error propagates through `filter_tasks`.
    /// Ensures that the wrapper function correctly propagates the validation error.
    #[rstest]
    fn test_filter_tasks_propagates_nil_uuid_error() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Valid Task", Priority::Low),
            create_task_with_nil_uuid("Invalid Task", Priority::High),
        ]
        .into_iter()
        .collect();

        let result = filter_tasks(&tasks, None, None);

        assert!(
            result.is_err(),
            "filter_tasks should propagate nil UUID error"
        );
    }

    /// Integration test: `filter_tasks` succeeds when all tasks have valid UUIDs.
    #[rstest]
    fn test_filter_tasks_succeeds_with_valid_uuids() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::High),
        ]
        .into_iter()
        .collect();

        let result = filter_tasks(&tasks, None, None);

        assert!(
            result.is_ok(),
            "filter_tasks should succeed with valid UUIDs"
        );
        assert_eq!(result.unwrap().len(), 2);
    }

    /// Integration test: `filter_tasks` with status filter still returns error for nil UUID.
    #[rstest]
    fn test_filter_tasks_with_status_filter_returns_error_for_nil_uuid() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Valid Task", Priority::Low),
            create_task_with_nil_uuid("Invalid Task", Priority::High),
        ]
        .into_iter()
        .collect();

        // Even with a status filter, nil UUID should cause an error
        let result = filter_tasks(&tasks, Some(TaskStatus::Pending), None);

        assert!(
            result.is_err(),
            "filter_tasks with status filter should still return error for nil UUID"
        );
    }
}
