//! Repository traits for domain entities.
//!
//! This module defines repository traits that return `AsyncIO` for
//! effect-based operations, following functional programming principles.

use lambars::effect::AsyncIO;
use thiserror::Error;

use crate::domain::{Project, ProjectId, Task, TaskEvent, TaskHistory, TaskId};

// =============================================================================
// Repository Error
// =============================================================================

/// Errors that can occur during repository operations.
#[derive(Debug, Error, Clone)]
pub enum RepositoryError {
    /// Entity was not found.
    #[error("Entity not found: {0}")]
    NotFound(String),

    /// Optimistic locking conflict.
    #[error("Version conflict: expected {expected}, found {found}")]
    VersionConflict { expected: u64, found: u64 },

    /// Database connection error.
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Cache operation error.
    #[error("Cache error: {0}")]
    CacheError(String),
}

// =============================================================================
// Pagination
// =============================================================================

/// Pagination parameters for list queries.
#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    /// Page number (0-indexed).
    pub page: u32,
    /// Number of items per page.
    pub page_size: u32,
}

impl Pagination {
    /// Creates new pagination parameters.
    ///
    /// # Panics
    ///
    /// Panics if `page_size` is 0.
    #[must_use]
    pub const fn new(page: u32, page_size: u32) -> Self {
        assert!(page_size > 0, "page_size must be greater than 0");
        Self { page, page_size }
    }

    /// Creates pagination that fetches all records.
    ///
    /// This is useful for operations that need to retrieve all items
    /// without pagination, such as aggregation queries or full exports.
    ///
    /// # Example
    ///
    /// ```
    /// use task_management_benchmark_api::infrastructure::Pagination;
    ///
    /// let all_pagination = Pagination::all();
    /// assert_eq!(all_pagination.page, 0);
    /// assert_eq!(all_pagination.page_size, u32::MAX);
    /// ```
    #[must_use]
    pub const fn all() -> Self {
        Self {
            page: 0,
            page_size: u32::MAX,
        }
    }

    /// Creates new pagination parameters without validation.
    ///
    /// This is useful for constructing pagination from untrusted input
    /// where you want to handle invalid values yourself.
    #[must_use]
    pub const fn new_unchecked(page: u32, page_size: u32) -> Self {
        Self { page, page_size }
    }

    /// Returns the offset for database queries.
    ///
    /// # Panics
    ///
    /// Panics if the offset calculation would overflow `u64`.
    #[must_use]
    pub const fn offset(&self) -> u64 {
        match (self.page as u64).checked_mul(self.page_size as u64) {
            Some(offset) => offset,
            None => panic!("Pagination offset overflow"),
        }
    }

    /// Returns the limit for database queries.
    #[must_use]
    pub const fn limit(&self) -> u32 {
        self.page_size
    }

    /// Returns true if the pagination parameters are valid.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.page_size > 0
    }
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: 20,
        }
    }
}

/// Paginated result containing items and total count.
#[derive(Debug, Clone)]
pub struct PaginatedResult<T> {
    /// The items in the current page.
    pub items: Vec<T>,
    /// Total number of items across all pages.
    pub total: u64,
    /// Current page (0-indexed).
    pub page: u32,
    /// Number of items per page.
    pub page_size: u32,
}

impl<T> PaginatedResult<T> {
    /// Creates a new paginated result.
    #[must_use]
    pub const fn new(items: Vec<T>, total: u64, page: u32, page_size: u32) -> Self {
        Self {
            items,
            total,
            page,
            page_size,
        }
    }

    /// Returns the total number of pages.
    #[must_use]
    pub const fn total_pages(&self) -> u64 {
        if self.page_size == 0 {
            return 0;
        }
        self.total.div_ceil(self.page_size as u64)
    }

    /// Returns true if there is a next page.
    #[must_use]
    pub const fn has_next(&self) -> bool {
        (self.page as u64 + 1) < self.total_pages()
    }

    /// Returns true if there is a previous page.
    #[must_use]
    pub const fn has_previous(&self) -> bool {
        self.page > 0
    }
}

// =============================================================================
// Task Repository
// =============================================================================

/// Repository trait for Task entities.
///
/// All methods return `AsyncIO` to encapsulate side effects and enable
/// composition with other effectful operations using `eff_async!` or `flat_map`.
///
/// # Example
///
/// ```ignore
/// use lambars::eff_async;
///
/// let workflow = eff_async! {
///     task <= repository.find_by_id(&task_id);
///     // Handle the result...
/// };
/// ```
pub trait TaskRepository: Send + Sync {
    /// Finds a task by its ID.
    ///
    /// Returns `Ok(Some(task))` if found, `Ok(None)` if not found,
    /// or an error if the operation fails.
    fn find_by_id(&self, id: &TaskId) -> AsyncIO<Result<Option<Task>, RepositoryError>>;

    /// Saves a task (insert or update).
    ///
    /// If a task with the same ID exists, it will be updated.
    /// The version field is used for optimistic locking.
    fn save(&self, task: &Task) -> AsyncIO<Result<(), RepositoryError>>;

    /// Deletes a task by its ID.
    ///
    /// Returns `Ok(true)` if the task was deleted, `Ok(false)` if it didn't exist.
    fn delete(&self, id: &TaskId) -> AsyncIO<Result<bool, RepositoryError>>;

    /// Lists all tasks with pagination.
    fn list(
        &self,
        pagination: Pagination,
    ) -> AsyncIO<Result<PaginatedResult<Task>, RepositoryError>>;

    /// Counts all tasks.
    fn count(&self) -> AsyncIO<Result<u64, RepositoryError>>;
}

// =============================================================================
// Project Repository
// =============================================================================

/// Repository trait for Project entities.
///
/// Similar to `TaskRepository`, all methods return `AsyncIO` for effect encapsulation.
pub trait ProjectRepository: Send + Sync {
    /// Finds a project by its ID.
    fn find_by_id(&self, id: &ProjectId) -> AsyncIO<Result<Option<Project>, RepositoryError>>;

    /// Saves a project (insert or update).
    fn save(&self, project: &Project) -> AsyncIO<Result<(), RepositoryError>>;

    /// Deletes a project by its ID.
    fn delete(&self, id: &ProjectId) -> AsyncIO<Result<bool, RepositoryError>>;

    /// Lists all projects with pagination.
    fn list(
        &self,
        pagination: Pagination,
    ) -> AsyncIO<Result<PaginatedResult<Project>, RepositoryError>>;

    /// Counts all projects.
    fn count(&self) -> AsyncIO<Result<u64, RepositoryError>>;
}

// =============================================================================
// Event Store
// =============================================================================

/// Event store trait for Task events (Event Sourcing).
///
/// The event store is responsible for persisting and retrieving task events.
/// Events are immutable and append-only.
pub trait EventStore: Send + Sync {
    /// Appends an event to the event store with optimistic locking.
    ///
    /// The event is appended to the task's event stream only if the current
    /// version matches `expected_version`. This ensures consistency when
    /// multiple clients are modifying the same task concurrently.
    ///
    /// # Errors
    ///
    /// Returns `RepositoryError::VersionConflict` if the current version
    /// does not match `expected_version`.
    fn append(
        &self,
        event: &TaskEvent,
        expected_version: u64,
    ) -> AsyncIO<Result<(), RepositoryError>>;

    /// Loads all events for a task.
    ///
    /// Events are returned in chronological order (oldest first).
    fn load_events(&self, task_id: &TaskId) -> AsyncIO<Result<TaskHistory, RepositoryError>>;

    /// Loads events for a task starting from a specific version.
    ///
    /// Useful for incremental event loading and event replay.
    fn load_events_from_version(
        &self,
        task_id: &TaskId,
        from_version: u64,
    ) -> AsyncIO<Result<TaskHistory, RepositoryError>>;

    /// Gets the current version (latest event version) for a task.
    ///
    /// Returns 0 if no events exist for the task.
    fn get_current_version(&self, task_id: &TaskId) -> AsyncIO<Result<u64, RepositoryError>>;
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // Pagination Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_pagination_new() {
        let pagination = Pagination::new(2, 10);
        assert_eq!(pagination.page, 2);
        assert_eq!(pagination.page_size, 10);
    }

    #[rstest]
    fn test_pagination_offset() {
        let pagination = Pagination::new(3, 20);
        assert_eq!(pagination.offset(), 60);
    }

    #[rstest]
    fn test_pagination_limit() {
        let pagination = Pagination::new(0, 15);
        assert_eq!(pagination.limit(), 15);
    }

    #[rstest]
    fn test_pagination_default() {
        let pagination = Pagination::default();
        assert_eq!(pagination.page, 0);
        assert_eq!(pagination.page_size, 20);
    }

    // -------------------------------------------------------------------------
    // PaginatedResult Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_paginated_result_new() {
        let result: PaginatedResult<i32> = PaginatedResult::new(vec![1, 2, 3], 100, 0, 10);
        assert_eq!(result.items.len(), 3);
        assert_eq!(result.total, 100);
        assert_eq!(result.page, 0);
        assert_eq!(result.page_size, 10);
    }

    #[rstest]
    fn test_paginated_result_total_pages() {
        // 100 items, 10 per page = 10 pages
        let result: PaginatedResult<i32> = PaginatedResult::new(vec![], 100, 0, 10);
        assert_eq!(result.total_pages(), 10);

        // 101 items, 10 per page = 11 pages
        let result: PaginatedResult<i32> = PaginatedResult::new(vec![], 101, 0, 10);
        assert_eq!(result.total_pages(), 11);

        // 0 items = 0 pages
        let result: PaginatedResult<i32> = PaginatedResult::new(vec![], 0, 0, 10);
        assert_eq!(result.total_pages(), 0);
    }

    #[rstest]
    fn test_paginated_result_total_pages_zero_page_size() {
        let result: PaginatedResult<i32> = PaginatedResult::new(vec![], 100, 0, 0);
        assert_eq!(result.total_pages(), 0);
    }

    #[rstest]
    fn test_paginated_result_has_next() {
        // On page 0 of 10 pages, has next
        let result: PaginatedResult<i32> = PaginatedResult::new(vec![], 100, 0, 10);
        assert!(result.has_next());

        // On page 9 of 10 pages, no next
        let result: PaginatedResult<i32> = PaginatedResult::new(vec![], 100, 9, 10);
        assert!(!result.has_next());

        // On page 8 of 10 pages, has next
        let result: PaginatedResult<i32> = PaginatedResult::new(vec![], 100, 8, 10);
        assert!(result.has_next());
    }

    #[rstest]
    fn test_paginated_result_has_previous() {
        // On page 0, no previous
        let result: PaginatedResult<i32> = PaginatedResult::new(vec![], 100, 0, 10);
        assert!(!result.has_previous());

        // On page 1, has previous
        let result: PaginatedResult<i32> = PaginatedResult::new(vec![], 100, 1, 10);
        assert!(result.has_previous());
    }

    // -------------------------------------------------------------------------
    // RepositoryError Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_repository_error_display() {
        let error = RepositoryError::NotFound("task-123".to_string());
        assert_eq!(format!("{error}"), "Entity not found: task-123");

        let error = RepositoryError::VersionConflict {
            expected: 1,
            found: 2,
        };
        assert_eq!(format!("{error}"), "Version conflict: expected 1, found 2");

        let error = RepositoryError::DatabaseError("connection refused".to_string());
        assert_eq!(format!("{error}"), "Database error: connection refused");
    }
}
