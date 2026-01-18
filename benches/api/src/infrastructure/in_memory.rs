//! In-memory repository implementations.
//!
//! This module provides in-memory implementations of the repository traits
//! using lambars' persistent data structures. These implementations are
//! suitable for testing and benchmarking purposes.
//!
//! # Features
//!
//! - Thread-safe with `Arc<RwLock<...>>`
//! - Persistent data structures with structural sharing
//! - Optimistic locking with version checking
//! - All operations return `AsyncIO` for effect encapsulation

use std::sync::Arc;
use tokio::sync::RwLock;

use lambars::effect::AsyncIO;
use lambars::persistent::PersistentHashMap;

use crate::domain::{Project, ProjectId, Task, TaskEvent, TaskHistory, TaskId};
use crate::infrastructure::{
    EventStore, PaginatedResult, Pagination, ProjectRepository, RepositoryError, TaskRepository,
};

// =============================================================================
// In-Memory Event Data
// =============================================================================

/// Internal structure to store events with version tracking.
#[derive(Debug, Clone)]
struct EventData {
    /// The list of events in chronological order (newest first due to cons).
    events: TaskHistory,
    /// The current version (incremented with each event).
    current_version: u64,
}

impl EventData {
    /// Creates a new empty event data.
    const fn new() -> Self {
        Self {
            events: TaskHistory::new(),
            current_version: 0,
        }
    }
}

// =============================================================================
// In-Memory Task Repository
// =============================================================================

/// In-memory implementation of `TaskRepository`.
///
/// Uses `PersistentHashMap` for storage with structural sharing,
/// wrapped in `Arc<RwLock<...>>` for thread safety.
///
/// # Example
///
/// ```ignore
/// use infrastructure::in_memory::InMemoryTaskRepository;
///
/// let repository = InMemoryTaskRepository::new();
/// let task = Task::new(TaskId::generate(), "My Task", Timestamp::now());
///
/// repository.save(&task).run_async().await?;
/// let found = repository.find_by_id(&task.task_id).run_async().await?;
/// ```
#[derive(Debug, Clone)]
pub struct InMemoryTaskRepository {
    /// Thread-safe storage using persistent hash map.
    tasks: Arc<RwLock<PersistentHashMap<TaskId, Task>>>,
}

impl InMemoryTaskRepository {
    /// Creates a new empty in-memory task repository.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(PersistentHashMap::new())),
        }
    }
}

impl Default for InMemoryTaskRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::significant_drop_tightening)]
impl TaskRepository for InMemoryTaskRepository {
    #[allow(clippy::future_not_send)]
    fn find_by_id(&self, id: &TaskId) -> AsyncIO<Result<Option<Task>, RepositoryError>> {
        let tasks = Arc::clone(&self.tasks);
        let id = id.clone();
        AsyncIO::new(move || async move {
            let guard = tasks.read().await;
            Ok(guard.get(&id).cloned())
        })
    }

    #[allow(clippy::future_not_send)]
    fn save(&self, task: &Task) -> AsyncIO<Result<(), RepositoryError>> {
        let tasks = Arc::clone(&self.tasks);
        let task = task.clone();
        AsyncIO::new(move || async move {
            let mut guard = tasks.write().await;

            // Check for version conflict if the task already exists
            if let Some(existing) = guard.get(&task.task_id) {
                // For updates, the task's version must be exactly one more than existing
                // This prevents version jumps and ensures strict sequential versioning
                if task.version != existing.version + 1 {
                    return Err(RepositoryError::VersionConflict {
                        expected: existing.version + 1,
                        found: task.version,
                    });
                }
            }

            // Insert the task (this creates a new map with structural sharing)
            *guard = guard.insert(task.task_id.clone(), task);
            Ok(())
        })
    }

    #[allow(clippy::future_not_send)]
    fn delete(&self, id: &TaskId) -> AsyncIO<Result<bool, RepositoryError>> {
        let tasks = Arc::clone(&self.tasks);
        let id = id.clone();
        AsyncIO::new(move || async move {
            let mut guard = tasks.write().await;
            let exists = guard.contains_key(&id);
            if exists {
                *guard = guard.remove(&id);
            }
            Ok(exists)
        })
    }

    #[allow(clippy::future_not_send)]
    fn list(
        &self,
        pagination: Pagination,
    ) -> AsyncIO<Result<PaginatedResult<Task>, RepositoryError>> {
        let tasks = Arc::clone(&self.tasks);
        AsyncIO::new(move || async move {
            let guard = tasks.read().await;

            // Collect all tasks
            let all_tasks: Vec<Task> = guard.iter().map(|(_, task)| task.clone()).collect();
            drop(guard);

            let total = all_tasks.len() as u64;
            #[allow(clippy::cast_possible_truncation)]
            let offset = pagination.offset() as usize;
            let limit = pagination.limit() as usize;

            // Apply pagination
            let items: Vec<Task> = all_tasks.into_iter().skip(offset).take(limit).collect();

            Ok(PaginatedResult::new(
                items,
                total,
                pagination.page,
                pagination.page_size,
            ))
        })
    }

    #[allow(clippy::future_not_send)]
    fn count(&self) -> AsyncIO<Result<u64, RepositoryError>> {
        let tasks = Arc::clone(&self.tasks);
        AsyncIO::new(move || async move {
            let guard = tasks.read().await;
            Ok(guard.len() as u64)
        })
    }
}

// =============================================================================
// In-Memory Project Repository
// =============================================================================

/// In-memory implementation of `ProjectRepository`.
///
/// Uses `PersistentHashMap` for storage with structural sharing,
/// wrapped in `Arc<RwLock<...>>` for thread safety.
///
/// # Example
///
/// ```ignore
/// use infrastructure::in_memory::InMemoryProjectRepository;
///
/// let repository = InMemoryProjectRepository::new();
/// let project = Project::new(ProjectId::generate(), "My Project", Timestamp::now());
///
/// repository.save(&project).run_async().await?;
/// let found = repository.find_by_id(&project.project_id).run_async().await?;
/// ```
#[derive(Debug, Clone)]
pub struct InMemoryProjectRepository {
    /// Thread-safe storage using persistent hash map.
    projects: Arc<RwLock<PersistentHashMap<ProjectId, Project>>>,
}

impl InMemoryProjectRepository {
    /// Creates a new empty in-memory project repository.
    #[must_use]
    pub fn new() -> Self {
        Self {
            projects: Arc::new(RwLock::new(PersistentHashMap::new())),
        }
    }
}

impl Default for InMemoryProjectRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::significant_drop_tightening)]
impl ProjectRepository for InMemoryProjectRepository {
    #[allow(clippy::future_not_send)]
    fn find_by_id(&self, id: &ProjectId) -> AsyncIO<Result<Option<Project>, RepositoryError>> {
        let projects = Arc::clone(&self.projects);
        let id = id.clone();
        AsyncIO::new(move || async move {
            let guard = projects.read().await;
            Ok(guard.get(&id).cloned())
        })
    }

    #[allow(clippy::future_not_send)]
    fn save(&self, project: &Project) -> AsyncIO<Result<(), RepositoryError>> {
        let projects = Arc::clone(&self.projects);
        let project = project.clone();
        AsyncIO::new(move || async move {
            let mut guard = projects.write().await;

            // Check for version conflict if the project already exists
            // Version must be exactly one more than existing to prevent jumps
            if let Some(existing) = guard.get(&project.project_id)
                && project.version != existing.version + 1
            {
                return Err(RepositoryError::VersionConflict {
                    expected: existing.version + 1,
                    found: project.version,
                });
            }

            // Insert the project (this creates a new map with structural sharing)
            *guard = guard.insert(project.project_id.clone(), project);
            Ok(())
        })
    }

    #[allow(clippy::future_not_send)]
    fn delete(&self, id: &ProjectId) -> AsyncIO<Result<bool, RepositoryError>> {
        let projects = Arc::clone(&self.projects);
        let id = id.clone();
        AsyncIO::new(move || async move {
            let mut guard = projects.write().await;
            let exists = guard.contains_key(&id);
            if exists {
                *guard = guard.remove(&id);
            }
            Ok(exists)
        })
    }

    #[allow(clippy::future_not_send)]
    fn list(
        &self,
        pagination: Pagination,
    ) -> AsyncIO<Result<PaginatedResult<Project>, RepositoryError>> {
        let projects = Arc::clone(&self.projects);
        AsyncIO::new(move || async move {
            let guard = projects.read().await;

            // Collect all projects
            let all_projects: Vec<Project> =
                guard.iter().map(|(_, project)| project.clone()).collect();
            drop(guard);

            let total = all_projects.len() as u64;
            #[allow(clippy::cast_possible_truncation)]
            let offset = pagination.offset() as usize;
            let limit = pagination.limit() as usize;

            // Apply pagination
            let items: Vec<Project> = all_projects.into_iter().skip(offset).take(limit).collect();

            Ok(PaginatedResult::new(
                items,
                total,
                pagination.page,
                pagination.page_size,
            ))
        })
    }

    #[allow(clippy::future_not_send)]
    fn count(&self) -> AsyncIO<Result<u64, RepositoryError>> {
        let projects = Arc::clone(&self.projects);
        AsyncIO::new(move || async move {
            let guard = projects.read().await;
            Ok(guard.len() as u64)
        })
    }
}

// =============================================================================
// In-Memory Event Store
// =============================================================================

/// In-memory implementation of `EventStore` for Task events.
///
/// Stores events per task using `PersistentHashMap` with version tracking
/// for optimistic locking. Events are stored in reverse chronological order
/// (newest first) using `PersistentList::cons`.
///
/// # Example
///
/// ```ignore
/// use infrastructure::in_memory::InMemoryEventStore;
///
/// let store = InMemoryEventStore::new();
///
/// // Append an event with expected version 0 (for first event)
/// store.append(&event, 0).run_async().await?;
///
/// // Load all events
/// let history = store.load_events(&task_id).run_async().await?;
/// ```
#[derive(Debug, Clone)]
pub struct InMemoryEventStore {
    /// Thread-safe storage: `task_id` -> event data.
    events: Arc<RwLock<PersistentHashMap<TaskId, EventData>>>,
}

impl InMemoryEventStore {
    /// Creates a new empty in-memory event store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(PersistentHashMap::new())),
        }
    }
}

impl Default for InMemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::significant_drop_tightening)]
impl EventStore for InMemoryEventStore {
    #[allow(clippy::future_not_send)]
    fn append(
        &self,
        event: &TaskEvent,
        expected_version: u64,
    ) -> AsyncIO<Result<(), RepositoryError>> {
        let events = Arc::clone(&self.events);
        let event = event.clone();
        AsyncIO::new(move || async move {
            let mut guard = events.write().await;

            // Get or create event data for this task
            let current_data = guard
                .get(&event.task_id)
                .cloned()
                .unwrap_or_else(EventData::new);

            // Check version conflict
            if current_data.current_version != expected_version {
                return Err(RepositoryError::VersionConflict {
                    expected: expected_version,
                    found: current_data.current_version,
                });
            }

            // Create new event data with the appended event
            let new_data = EventData {
                events: current_data.events.cons(event.clone()),
                current_version: current_data.current_version + 1,
            };

            // Update the map
            *guard = guard.insert(event.task_id.clone(), new_data);
            Ok(())
        })
    }

    #[allow(clippy::future_not_send)]
    fn load_events(&self, task_id: &TaskId) -> AsyncIO<Result<TaskHistory, RepositoryError>> {
        let events = Arc::clone(&self.events);
        let task_id = task_id.clone();
        AsyncIO::new(move || async move {
            let guard = events.read().await;
            let history = guard.get(&task_id).map_or_else(TaskHistory::new, |data| {
                // Events are stored newest-first (cons prepends), but we need to return
                // oldest-first as per trait specification.
                // By folding over newest-first with cons, we get oldest-first:
                // [3,2,1] -> cons(3,[]) -> cons(2,[3]) -> cons(1,[2,3]) = [1,2,3]
                data.events
                    .iter()
                    .cloned()
                    .fold(TaskHistory::new(), |acc, event| acc.cons(event))
            });
            Ok(history)
        })
    }

    #[allow(clippy::future_not_send)]
    fn load_events_from_version(
        &self,
        task_id: &TaskId,
        from_version: u64,
    ) -> AsyncIO<Result<TaskHistory, RepositoryError>> {
        let events = Arc::clone(&self.events);
        let task_id = task_id.clone();
        AsyncIO::new(move || async move {
            let guard = events.read().await;
            let history = guard.get(&task_id).map_or_else(TaskHistory::new, |data| {
                // Filter events to only include those with version > from_version
                // Events are stored newest-first, but we return oldest-first
                // By folding with cons, the order is reversed to oldest-first
                data.events
                    .iter()
                    .filter(|event| event.version > from_version)
                    .cloned()
                    .fold(TaskHistory::new(), |acc, event| acc.cons(event))
            });
            Ok(history)
        })
    }

    #[allow(clippy::future_not_send)]
    fn get_current_version(&self, task_id: &TaskId) -> AsyncIO<Result<u64, RepositoryError>> {
        let events = Arc::clone(&self.events);
        let task_id = task_id.clone();
        AsyncIO::new(move || async move {
            let guard = events.read().await;
            let version = guard.get(&task_id).map_or(0, |data| data.current_version);
            Ok(version)
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{EventId, Priority, TaskCreated, TaskEventKind, TaskHistoryExt, Timestamp};
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // Helper functions for tests
    // -------------------------------------------------------------------------

    fn test_task(title: &str) -> Task {
        Task::new(TaskId::generate(), title, Timestamp::now())
    }

    fn test_task_with_id(task_id: TaskId, title: &str) -> Task {
        Task::new(task_id, title, Timestamp::now())
    }

    fn test_project(name: &str) -> Project {
        Project::new(ProjectId::generate(), name, Timestamp::now())
    }

    fn test_project_with_id(project_id: ProjectId, name: &str) -> Project {
        Project::new(project_id, name, Timestamp::now())
    }

    fn test_task_event(task_id: TaskId, version: u64) -> TaskEvent {
        TaskEvent::new(
            EventId::generate(),
            task_id,
            Timestamp::now(),
            version,
            TaskEventKind::Created(TaskCreated {
                title: "Test Task".to_string(),
                description: None,
                priority: Priority::Low,
            }),
        )
    }

    // -------------------------------------------------------------------------
    // InMemoryTaskRepository Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_task_repository_find_by_id_not_found() {
        let repository = InMemoryTaskRepository::new();
        let task_id = TaskId::generate();

        let result = repository.find_by_id(&task_id).run_async().await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[rstest]
    #[tokio::test]
    async fn test_task_repository_save_and_find() {
        let repository = InMemoryTaskRepository::new();
        let task = test_task("Test Task");
        let task_id = task.task_id.clone();

        // Save the task
        let save_result = repository.save(&task).run_async().await;
        assert!(save_result.is_ok());

        // Find the task
        let find_result = repository.find_by_id(&task_id).run_async().await;
        assert!(find_result.is_ok());
        let found_task = find_result.unwrap();
        assert!(found_task.is_some());
        assert_eq!(found_task.unwrap().title, "Test Task");
    }

    #[rstest]
    #[tokio::test]
    async fn test_task_repository_save_update() {
        let repository = InMemoryTaskRepository::new();
        let task = test_task("Original Title");
        let task_id = task.task_id.clone();

        // Save the original task
        repository.save(&task).run_async().await.unwrap();

        // Update the task with incremented version
        let updated_task = test_task_with_id(task_id.clone(), "Updated Title").increment_version();
        let update_result = repository.save(&updated_task).run_async().await;
        assert!(update_result.is_ok());

        // Verify the update
        let found = repository
            .find_by_id(&task_id)
            .run_async()
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.title, "Updated Title");
        assert_eq!(found.version, 2);
    }

    #[rstest]
    #[tokio::test]
    async fn test_task_repository_save_version_conflict() {
        let repository = InMemoryTaskRepository::new();
        let task = test_task("Test Task");
        let task_id = task.task_id.clone();

        // Save the original task
        repository.save(&task).run_async().await.unwrap();

        // Try to save with same version (should fail)
        let conflicting_task = test_task_with_id(task_id, "Conflicting Task");
        let result = repository.save(&conflicting_task).run_async().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            RepositoryError::VersionConflict { expected, found } => {
                assert_eq!(expected, 2);
                assert_eq!(found, 1);
            }
            _ => panic!("Expected VersionConflict error"),
        }
    }

    #[rstest]
    #[tokio::test]
    async fn test_task_repository_delete_existing() {
        let repository = InMemoryTaskRepository::new();
        let task = test_task("Test Task");
        let task_id = task.task_id.clone();

        // Save the task
        repository.save(&task).run_async().await.unwrap();

        // Delete the task
        let delete_result = repository.delete(&task_id).run_async().await;
        assert!(delete_result.is_ok());
        assert!(delete_result.unwrap());

        // Verify deletion
        let find_result = repository.find_by_id(&task_id).run_async().await.unwrap();
        assert!(find_result.is_none());
    }

    #[rstest]
    #[tokio::test]
    async fn test_task_repository_delete_not_found() {
        let repository = InMemoryTaskRepository::new();
        let task_id = TaskId::generate();

        let result = repository.delete(&task_id).run_async().await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[rstest]
    #[tokio::test]
    async fn test_task_repository_list_empty() {
        let repository = InMemoryTaskRepository::new();
        let pagination = Pagination::new(0, 10);

        let result = repository.list(pagination).run_async().await;
        assert!(result.is_ok());
        let paginated = result.unwrap();
        assert!(paginated.items.is_empty());
        assert_eq!(paginated.total, 0);
    }

    #[rstest]
    #[tokio::test]
    async fn test_task_repository_list_with_pagination() {
        let repository = InMemoryTaskRepository::new();

        // Save 5 tasks
        for i in 0..5 {
            let task = test_task(&format!("Task {i}"));
            repository.save(&task).run_async().await.unwrap();
        }

        // Get first page (2 items)
        let pagination = Pagination::new(0, 2);
        let result = repository.list(pagination).run_async().await.unwrap();
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.total, 5);
        assert_eq!(result.page, 0);
        assert_eq!(result.page_size, 2);

        // Get second page
        let pagination = Pagination::new(1, 2);
        let result = repository.list(pagination).run_async().await.unwrap();
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.total, 5);

        // Get third page (only 1 item left)
        let pagination = Pagination::new(2, 2);
        let result = repository.list(pagination).run_async().await.unwrap();
        assert_eq!(result.items.len(), 1);
    }

    #[rstest]
    #[tokio::test]
    async fn test_task_repository_count() {
        let repository = InMemoryTaskRepository::new();

        assert_eq!(repository.count().run_async().await.unwrap(), 0);

        for i in 0..3 {
            let task = test_task(&format!("Task {i}"));
            repository.save(&task).run_async().await.unwrap();
        }

        assert_eq!(repository.count().run_async().await.unwrap(), 3);
    }

    // -------------------------------------------------------------------------
    // InMemoryProjectRepository Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_project_repository_find_by_id_not_found() {
        let repository = InMemoryProjectRepository::new();
        let project_id = ProjectId::generate();

        let result = repository.find_by_id(&project_id).run_async().await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[rstest]
    #[tokio::test]
    async fn test_project_repository_save_and_find() {
        let repository = InMemoryProjectRepository::new();
        let project = test_project("Test Project");
        let project_id = project.project_id.clone();

        // Save the project
        let save_result = repository.save(&project).run_async().await;
        assert!(save_result.is_ok());

        // Find the project
        let find_result = repository.find_by_id(&project_id).run_async().await;
        assert!(find_result.is_ok());
        let found_project = find_result.unwrap();
        assert!(found_project.is_some());
        assert_eq!(found_project.unwrap().name, "Test Project");
    }

    #[rstest]
    #[tokio::test]
    async fn test_project_repository_save_update() {
        let repository = InMemoryProjectRepository::new();
        let project = test_project("Original Name");
        let project_id = project.project_id.clone();

        // Save the original project
        repository.save(&project).run_async().await.unwrap();

        // Update the project with incremented version
        let updated_project =
            test_project_with_id(project_id.clone(), "Updated Name").increment_version();
        let update_result = repository.save(&updated_project).run_async().await;
        assert!(update_result.is_ok());

        // Verify the update
        let found = repository
            .find_by_id(&project_id)
            .run_async()
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.name, "Updated Name");
        assert_eq!(found.version, 2);
    }

    #[rstest]
    #[tokio::test]
    async fn test_project_repository_save_version_conflict() {
        let repository = InMemoryProjectRepository::new();
        let project = test_project("Test Project");
        let project_id = project.project_id.clone();

        // Save the original project
        repository.save(&project).run_async().await.unwrap();

        // Try to save with same version (should fail)
        let conflicting_project = test_project_with_id(project_id, "Conflicting Project");
        let result = repository.save(&conflicting_project).run_async().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            RepositoryError::VersionConflict { expected, found } => {
                assert_eq!(expected, 2);
                assert_eq!(found, 1);
            }
            _ => panic!("Expected VersionConflict error"),
        }
    }

    #[rstest]
    #[tokio::test]
    async fn test_project_repository_delete_existing() {
        let repository = InMemoryProjectRepository::new();
        let project = test_project("Test Project");
        let project_id = project.project_id.clone();

        // Save the project
        repository.save(&project).run_async().await.unwrap();

        // Delete the project
        let delete_result = repository.delete(&project_id).run_async().await;
        assert!(delete_result.is_ok());
        assert!(delete_result.unwrap());

        // Verify deletion
        let find_result = repository
            .find_by_id(&project_id)
            .run_async()
            .await
            .unwrap();
        assert!(find_result.is_none());
    }

    #[rstest]
    #[tokio::test]
    async fn test_project_repository_delete_not_found() {
        let repository = InMemoryProjectRepository::new();
        let project_id = ProjectId::generate();

        let result = repository.delete(&project_id).run_async().await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[rstest]
    #[tokio::test]
    async fn test_project_repository_list_empty() {
        let repository = InMemoryProjectRepository::new();
        let pagination = Pagination::new(0, 10);

        let result = repository.list(pagination).run_async().await;
        assert!(result.is_ok());
        let paginated = result.unwrap();
        assert!(paginated.items.is_empty());
        assert_eq!(paginated.total, 0);
    }

    #[rstest]
    #[tokio::test]
    async fn test_project_repository_list_with_pagination() {
        let repository = InMemoryProjectRepository::new();

        // Save 5 projects
        for i in 0..5 {
            let project = test_project(&format!("Project {i}"));
            repository.save(&project).run_async().await.unwrap();
        }

        // Get first page (2 items)
        let pagination = Pagination::new(0, 2);
        let result = repository.list(pagination).run_async().await.unwrap();
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.total, 5);
        assert_eq!(result.page, 0);
        assert_eq!(result.page_size, 2);

        // Get second page
        let pagination = Pagination::new(1, 2);
        let result = repository.list(pagination).run_async().await.unwrap();
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.total, 5);

        // Get third page (only 1 item left)
        let pagination = Pagination::new(2, 2);
        let result = repository.list(pagination).run_async().await.unwrap();
        assert_eq!(result.items.len(), 1);
    }

    #[rstest]
    #[tokio::test]
    async fn test_project_repository_count() {
        let repository = InMemoryProjectRepository::new();

        assert_eq!(repository.count().run_async().await.unwrap(), 0);

        for i in 0..3 {
            let project = test_project(&format!("Project {i}"));
            repository.save(&project).run_async().await.unwrap();
        }

        assert_eq!(repository.count().run_async().await.unwrap(), 3);
    }

    // -------------------------------------------------------------------------
    // InMemoryEventStore Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_event_store_append_first_event() {
        let store = InMemoryEventStore::new();
        let task_id = TaskId::generate();
        let event = test_task_event(task_id.clone(), 1);

        // Append first event with expected version 0
        let result = store.append(&event, 0).run_async().await;
        assert!(result.is_ok());

        // Verify current version
        let version = store
            .get_current_version(&task_id)
            .run_async()
            .await
            .unwrap();
        assert_eq!(version, 1);
    }

    #[rstest]
    #[tokio::test]
    async fn test_event_store_append_multiple_events() {
        let store = InMemoryEventStore::new();
        let task_id = TaskId::generate();

        // Append three events
        for i in 1..=3 {
            let event = test_task_event(task_id.clone(), i);
            store.append(&event, i - 1).run_async().await.unwrap();
        }

        // Verify current version
        let version = store
            .get_current_version(&task_id)
            .run_async()
            .await
            .unwrap();
        assert_eq!(version, 3);

        // Verify event count
        let history = store.load_events(&task_id).run_async().await.unwrap();
        assert_eq!(history.event_count(), 3);
    }

    #[rstest]
    #[tokio::test]
    async fn test_event_store_append_version_conflict() {
        let store = InMemoryEventStore::new();
        let task_id = TaskId::generate();
        let event1 = test_task_event(task_id.clone(), 1);

        // Append first event
        store.append(&event1, 0).run_async().await.unwrap();

        // Try to append with wrong expected version
        let event2 = test_task_event(task_id, 2);
        let result = store.append(&event2, 0).run_async().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            RepositoryError::VersionConflict { expected, found } => {
                assert_eq!(expected, 0);
                assert_eq!(found, 1);
            }
            _ => panic!("Expected VersionConflict error"),
        }
    }

    #[rstest]
    #[tokio::test]
    async fn test_event_store_load_events_empty() {
        let store = InMemoryEventStore::new();
        let task_id = TaskId::generate();

        let history = store.load_events(&task_id).run_async().await.unwrap();
        assert!(history.is_empty());
    }

    #[rstest]
    #[tokio::test]
    async fn test_event_store_load_events_with_data() {
        let store = InMemoryEventStore::new();
        let task_id = TaskId::generate();

        // Append events
        for i in 1..=3 {
            let event = test_task_event(task_id.clone(), i);
            store.append(&event, i - 1).run_async().await.unwrap();
        }

        // Load events
        let history = store.load_events(&task_id).run_async().await.unwrap();
        assert_eq!(history.event_count(), 3);

        // Oldest-first order: first event is version 1, last is version 3
        let versions: Vec<u64> = history.iter().map(|e| e.version).collect();
        assert_eq!(versions, vec![1, 2, 3]);
    }

    #[rstest]
    #[tokio::test]
    async fn test_event_store_load_events_from_version() {
        let store = InMemoryEventStore::new();
        let task_id = TaskId::generate();

        // Append 5 events
        for i in 1..=5 {
            let event = test_task_event(task_id.clone(), i);
            store.append(&event, i - 1).run_async().await.unwrap();
        }

        // Load events from version 2 (should get events 3, 4, 5)
        let history = store
            .load_events_from_version(&task_id, 2)
            .run_async()
            .await
            .unwrap();
        assert_eq!(history.event_count(), 3);

        // Verify versions
        let versions: Vec<u64> = history.iter().map(|event| event.version).collect();
        assert!(versions.contains(&3));
        assert!(versions.contains(&4));
        assert!(versions.contains(&5));
    }

    #[rstest]
    #[tokio::test]
    async fn test_event_store_load_events_from_version_all() {
        let store = InMemoryEventStore::new();
        let task_id = TaskId::generate();

        // Append 3 events
        for i in 1..=3 {
            let event = test_task_event(task_id.clone(), i);
            store.append(&event, i - 1).run_async().await.unwrap();
        }

        // Load events from version 0 (should get all events)
        let history = store
            .load_events_from_version(&task_id, 0)
            .run_async()
            .await
            .unwrap();
        assert_eq!(history.event_count(), 3);
    }

    #[rstest]
    #[tokio::test]
    async fn test_event_store_load_events_from_version_none() {
        let store = InMemoryEventStore::new();
        let task_id = TaskId::generate();

        // Append 3 events
        for i in 1..=3 {
            let event = test_task_event(task_id.clone(), i);
            store.append(&event, i - 1).run_async().await.unwrap();
        }

        // Load events from version 10 (should get no events)
        let history = store
            .load_events_from_version(&task_id, 10)
            .run_async()
            .await
            .unwrap();
        assert!(history.is_empty());
    }

    #[rstest]
    #[tokio::test]
    async fn test_event_store_get_current_version_empty() {
        let store = InMemoryEventStore::new();
        let task_id = TaskId::generate();

        let version = store
            .get_current_version(&task_id)
            .run_async()
            .await
            .unwrap();
        assert_eq!(version, 0);
    }

    #[rstest]
    #[tokio::test]
    async fn test_event_store_get_current_version_with_events() {
        let store = InMemoryEventStore::new();
        let task_id = TaskId::generate();

        // Append 3 events
        for i in 1..=3 {
            let event = test_task_event(task_id.clone(), i);
            store.append(&event, i - 1).run_async().await.unwrap();
        }

        let version = store
            .get_current_version(&task_id)
            .run_async()
            .await
            .unwrap();
        assert_eq!(version, 3);
    }

    // -------------------------------------------------------------------------
    // Thread Safety Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_task_repository_concurrent_access() {
        let repository = Arc::new(InMemoryTaskRepository::new());

        // Spawn multiple tasks to save concurrently
        let mut handles = vec![];
        for i in 0..10 {
            let repo = Arc::clone(&repository);
            handles.push(tokio::spawn(async move {
                let task = test_task(&format!("Task {i}"));
                repo.save(&task).run_async().await
            }));
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        // Verify all tasks were saved
        let count = repository.count().run_async().await.unwrap();
        assert_eq!(count, 10);
    }

    #[rstest]
    #[tokio::test]
    async fn test_event_store_concurrent_append() {
        let store = Arc::new(InMemoryEventStore::new());

        // Test concurrent appends to different tasks
        let mut handles = vec![];
        for _ in 0..10 {
            let store = Arc::clone(&store);
            handles.push(tokio::spawn(async move {
                let task_id = TaskId::generate();
                let event = test_task_event(task_id, 1);
                store.append(&event, 0).run_async().await
            }));
        }

        // Wait for all appends to complete
        for handle in handles {
            handle.await.unwrap().unwrap();
        }
    }

    // -------------------------------------------------------------------------
    // Default Implementation Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_task_repository_default() {
        let repository = InMemoryTaskRepository::default();
        assert!(repository.tasks.try_read().is_ok());
    }

    #[rstest]
    fn test_project_repository_default() {
        let repository = InMemoryProjectRepository::default();
        assert!(repository.projects.try_read().is_ok());
    }

    #[rstest]
    fn test_event_store_default() {
        let store = InMemoryEventStore::default();
        assert!(store.events.try_read().is_ok());
    }

    // -------------------------------------------------------------------------
    // Event Ordering Tests (oldest-first)
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_event_store_load_events_oldest_first() {
        let store = InMemoryEventStore::new();
        let task_id = TaskId::generate();

        // Append events 1, 2, 3 in order
        for i in 1..=3 {
            let event = test_task_event(task_id.clone(), i);
            store.append(&event, i - 1).run_async().await.unwrap();
        }

        // Load events - should be in oldest-first order
        let history = store.load_events(&task_id).run_async().await.unwrap();
        let versions: Vec<u64> = history.iter().map(|event| event.version).collect();

        // Verify oldest-first order: 1, 2, 3
        assert_eq!(versions, vec![1, 2, 3]);
    }

    #[rstest]
    #[tokio::test]
    async fn test_event_store_load_events_from_version_oldest_first() {
        let store = InMemoryEventStore::new();
        let task_id = TaskId::generate();

        // Append 5 events
        for i in 1..=5 {
            let event = test_task_event(task_id.clone(), i);
            store.append(&event, i - 1).run_async().await.unwrap();
        }

        // Load events from version 2 (should get events 3, 4, 5 in oldest-first order)
        let history = store
            .load_events_from_version(&task_id, 2)
            .run_async()
            .await
            .unwrap();
        let versions: Vec<u64> = history.iter().map(|event| event.version).collect();

        // Verify oldest-first order: 3, 4, 5
        assert_eq!(versions, vec![3, 4, 5]);
    }

    // -------------------------------------------------------------------------
    // Version Jump Rejection Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_task_repository_save_rejects_version_jump() {
        let repository = InMemoryTaskRepository::new();
        let task_id = TaskId::generate();
        let timestamp = Timestamp::now();

        // Save initial task (version 1)
        let task = Task::new(task_id.clone(), "Test Task", timestamp);
        repository.save(&task).run_async().await.unwrap();

        // Try to save with version 3 (jumping from 1 to 3) - should fail
        let mut jumped_task = task.clone();
        jumped_task.version = 3; // Skip version 2
        let result = repository.save(&jumped_task).run_async().await;

        assert!(result.is_err());
        match result {
            Err(RepositoryError::VersionConflict { expected, found }) => {
                assert_eq!(expected, 2); // Expected version 2
                assert_eq!(found, 3); // But got version 3
            }
            _ => panic!("Expected VersionConflict error"),
        }
    }

    #[rstest]
    #[tokio::test]
    async fn test_project_repository_save_rejects_version_jump() {
        let repository = InMemoryProjectRepository::new();
        let project_id = ProjectId::generate();
        let timestamp = Timestamp::now();

        // Save initial project (version 1)
        let project = Project::new(project_id.clone(), "Test Project", timestamp);
        repository.save(&project).run_async().await.unwrap();

        // Try to save with version 5 (jumping from 1 to 5) - should fail
        let mut jumped_project = project.clone();
        jumped_project.version = 5; // Skip versions 2, 3, 4
        let result = repository.save(&jumped_project).run_async().await;

        assert!(result.is_err());
        match result {
            Err(RepositoryError::VersionConflict { expected, found }) => {
                assert_eq!(expected, 2); // Expected version 2
                assert_eq!(found, 5); // But got version 5
            }
            _ => panic!("Expected VersionConflict error"),
        }
    }
}
