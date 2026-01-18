//! Project domain model.
//!
//! This module contains the project entity which serves as a container
//! for multiple tasks, demonstrating the use of `PersistentHashMap`.

use lambars::persistent::PersistentHashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::task::{Priority, TaskId, TaskStatus, Timestamp};

// =============================================================================
// Value Objects - Newtypes
// =============================================================================

/// Unique identifier for a project.
///
/// This is a newtype wrapper around UUID to provide type safety.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(Uuid);

impl ProjectId {
    /// Creates a `ProjectId` from an existing UUID.
    ///
    /// This is a pure function - it does not generate a new UUID.
    /// Use `ProjectId::generate()` in an effect boundary (e.g., `AsyncIO`)
    /// to create a new random ID.
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the inner UUID.
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Generates a new `ProjectId` with a randomly generated UUID (v4).
    ///
    /// **Note**: This is an impure function (side effect: random number generation).
    /// It should be called within an effect boundary (e.g., `AsyncIO::new`).
    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    /// Generates a new `ProjectId` with a time-ordered UUID (v7).
    ///
    /// **Note**: This is an impure function (side effect: time + random).
    /// It should be called within an effect boundary (e.g., `AsyncIO::new`).
    #[must_use]
    pub fn generate_v7() -> Self {
        Self(Uuid::now_v7())
    }
}

impl std::fmt::Display for ProjectId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

// =============================================================================
// TaskSummary
// =============================================================================

/// A summary view of a task for project-level aggregation.
///
/// This is a lightweight representation of a task that contains only
/// the essential information needed for project-level operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSummary {
    /// The task's unique identifier.
    pub task_id: TaskId,
    /// Title of the task.
    pub title: String,
    /// Current status of the task.
    pub status: TaskStatus,
    /// Priority level of the task.
    pub priority: Priority,
}

impl TaskSummary {
    /// Creates a new task summary.
    ///
    /// This is a pure function.
    #[must_use]
    pub fn new(
        task_id: TaskId,
        title: impl Into<String>,
        status: TaskStatus,
        priority: Priority,
    ) -> Self {
        Self {
            task_id,
            title: title.into(),
            status,
            priority,
        }
    }
}

// =============================================================================
// Project
// =============================================================================

/// The project domain model.
///
/// A project serves as a container for multiple tasks, providing
/// organization and aggregation capabilities.
///
/// # Lambars Features Used
///
/// - `PersistentHashMap`: For managing task references as an immutable map
///
/// # Examples
///
/// ```ignore
/// use domain::project::{Project, ProjectId, TaskSummary};
/// use domain::task::{TaskId, TaskStatus, Priority, Timestamp};
///
/// let project = Project::new(project_id, "My Project", timestamp)
///     .with_description("Project description")
///     .add_task(task_summary);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_field_names)]
pub struct Project {
    /// Unique identifier for the project.
    pub project_id: ProjectId,
    /// Name of the project.
    pub name: String,
    /// Optional detailed description.
    pub description: Option<String>,
    /// Map of tasks in this project (using persistent hash map).
    pub tasks: PersistentHashMap<TaskId, TaskSummary>,
    /// Timestamp when the project was created.
    pub created_at: Timestamp,
    /// Timestamp when the project was last updated.
    pub updated_at: Timestamp,
    /// Version number for optimistic locking.
    pub version: u64,
}

impl Project {
    /// Creates a new project with the given ID, name, and timestamp.
    ///
    /// This is a pure function. Use `ProjectId::generate()` and `Timestamp::now()`
    /// within an effect boundary to create the required parameters.
    ///
    /// The project is initialized with:
    /// - Empty task map
    /// - Version: 1
    ///
    /// # Example (within effect boundary)
    ///
    /// ```ignore
    /// let workflow = AsyncIO::new(|| async {
    ///     let project_id = ProjectId::generate_v7();
    ///     let now = Timestamp::now();
    ///     Ok(Project::new(project_id, "My Project", now))
    /// });
    /// ```
    #[must_use]
    pub fn new(project_id: ProjectId, name: impl Into<String>, timestamp: Timestamp) -> Self {
        Self {
            project_id,
            name: name.into(),
            description: None,
            tasks: PersistentHashMap::new(),
            created_at: timestamp.clone(),
            updated_at: timestamp,
            version: 1,
        }
    }

    // -------------------------------------------------------------------------
    // Builder-style methods (pure immutable updates)
    // -------------------------------------------------------------------------

    /// Returns a new project with the updated timestamp.
    ///
    /// Use this method to update the `updated_at` field when modifying the project.
    /// This is a pure function - the timestamp should be obtained within an
    /// effect boundary.
    #[must_use]
    pub fn with_updated_at(self, timestamp: Timestamp) -> Self {
        Self {
            updated_at: timestamp,
            ..self
        }
    }

    /// Returns a new project with the given description.
    #[must_use]
    pub fn with_description(self, description: impl Into<String>) -> Self {
        Self {
            description: Some(description.into()),
            ..self
        }
    }

    /// Returns a new project with the given name.
    #[must_use]
    pub fn with_name(self, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..self
        }
    }

    /// Returns a new project with the given task map (replacing existing tasks).
    #[must_use]
    pub fn with_tasks(self, tasks: PersistentHashMap<TaskId, TaskSummary>) -> Self {
        Self { tasks, ..self }
    }

    /// Returns a new project with an incremented version.
    ///
    /// # Panics
    ///
    /// Panics if the version number overflows `u64::MAX`.
    #[must_use]
    pub fn increment_version(self) -> Self {
        Self {
            version: self
                .version
                .checked_add(1)
                .expect("Version overflow: version number exceeded u64::MAX"),
            ..self
        }
    }

    // -------------------------------------------------------------------------
    // Task operations (pure)
    // -------------------------------------------------------------------------

    /// Returns a new project with the given task summary added or updated.
    ///
    /// If a task with the same ID already exists, it will be replaced.
    #[must_use]
    pub fn add_task(self, task_summary: TaskSummary) -> Self {
        Self {
            tasks: self
                .tasks
                .insert(task_summary.task_id.clone(), task_summary),
            ..self
        }
    }

    /// Returns a new project with the specified task removed.
    ///
    /// If the task doesn't exist, the project is returned unchanged.
    #[must_use]
    pub fn remove_task(self, task_id: &TaskId) -> Self {
        Self {
            tasks: self.tasks.remove(task_id),
            ..self
        }
    }

    /// Returns the task summary for the given task ID, if it exists.
    #[must_use]
    pub fn get_task(&self, task_id: &TaskId) -> Option<&TaskSummary> {
        self.tasks.get(task_id)
    }

    /// Returns `true` if the project contains the given task.
    #[must_use]
    pub fn has_task(&self, task_id: &TaskId) -> bool {
        self.tasks.contains_key(task_id)
    }

    /// Returns the number of tasks in the project.
    #[must_use]
    pub const fn task_count(&self) -> usize {
        self.tasks.len()
    }

    // -------------------------------------------------------------------------
    // Aggregation operations (pure)
    // -------------------------------------------------------------------------

    /// Returns the number of tasks with the given status.
    #[must_use]
    pub fn count_by_status(&self, status: TaskStatus) -> usize {
        self.tasks
            .iter()
            .filter(|(_, summary)| summary.status == status)
            .count()
    }

    /// Returns the number of completed tasks.
    #[must_use]
    pub fn completed_count(&self) -> usize {
        self.count_by_status(TaskStatus::Completed)
    }

    /// Returns the number of pending tasks.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.count_by_status(TaskStatus::Pending)
    }

    /// Returns the number of in-progress tasks.
    #[must_use]
    pub fn in_progress_count(&self) -> usize {
        self.count_by_status(TaskStatus::InProgress)
    }

    /// Returns the project progress as a ratio (0.0 to 1.0).
    ///
    /// Progress is calculated as the number of completed tasks divided by
    /// the total number of non-cancelled tasks.
    ///
    /// Returns 1.0 if there are no tasks or all tasks are cancelled.
    #[must_use]
    pub fn progress(&self) -> f64 {
        let total = self.task_count();
        let cancelled = self.count_by_status(TaskStatus::Cancelled);
        let active_total = total.saturating_sub(cancelled);

        if active_total == 0 {
            return 1.0;
        }

        let completed = self.completed_count();
        #[allow(clippy::cast_precision_loss)]
        let progress = completed as f64 / active_total as f64;
        progress
    }

    /// Returns the number of tasks with the given priority.
    #[must_use]
    pub fn count_by_priority(&self, priority: Priority) -> usize {
        self.tasks
            .iter()
            .filter(|(_, summary)| summary.priority == priority)
            .count()
    }
}

impl PartialEq for Project {
    fn eq(&self, other: &Self) -> bool {
        self.project_id == other.project_id
    }
}

impl Eq for Project {}

impl std::hash::Hash for Project {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.project_id.hash(state);
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // Helper functions for tests
    // -------------------------------------------------------------------------

    fn test_project(name: &str) -> Project {
        Project::new(ProjectId::generate(), name, Timestamp::now())
    }

    fn test_task_summary(title: &str) -> TaskSummary {
        TaskSummary::new(
            TaskId::generate(),
            title,
            TaskStatus::Pending,
            Priority::Low,
        )
    }

    fn test_task_summary_with_status(title: &str, status: TaskStatus) -> TaskSummary {
        TaskSummary::new(TaskId::generate(), title, status, Priority::Low)
    }

    // -------------------------------------------------------------------------
    // ProjectId Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_project_id_generate_creates_unique_ids() {
        let id1 = ProjectId::generate();
        let id2 = ProjectId::generate();
        assert_ne!(id1, id2);
    }

    #[rstest]
    fn test_project_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let project_id = ProjectId::from_uuid(uuid);
        assert_eq!(project_id.as_uuid(), &uuid);
    }

    #[rstest]
    fn test_project_id_display() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let project_id = ProjectId::from_uuid(uuid);
        assert_eq!(
            format!("{project_id}"),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    // -------------------------------------------------------------------------
    // TaskSummary Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_task_summary_new() {
        let task_id = TaskId::generate();
        let summary = TaskSummary::new(
            task_id.clone(),
            "Test Task",
            TaskStatus::Pending,
            Priority::High,
        );
        assert_eq!(summary.task_id, task_id);
        assert_eq!(summary.title, "Test Task");
        assert_eq!(summary.status, TaskStatus::Pending);
        assert_eq!(summary.priority, Priority::High);
    }

    // -------------------------------------------------------------------------
    // Project Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_project_new() {
        let project = test_project("Test Project");
        assert_eq!(project.name, "Test Project");
        assert!(project.description.is_none());
        assert!(project.tasks.is_empty());
        assert_eq!(project.version, 1);
    }

    #[rstest]
    fn test_project_with_description() {
        let project = test_project("Test").with_description("Description");
        assert_eq!(project.description, Some("Description".to_string()));
    }

    #[rstest]
    fn test_project_with_name() {
        let project = test_project("Test").with_name("New Name");
        assert_eq!(project.name, "New Name");
    }

    #[rstest]
    fn test_project_add_task() {
        let summary = test_task_summary("Task 1");
        let task_id = summary.task_id.clone();
        let project = test_project("Test").add_task(summary);

        assert_eq!(project.task_count(), 1);
        assert!(project.has_task(&task_id));
        assert_eq!(
            project.get_task(&task_id).map(|s| &s.title),
            Some(&"Task 1".to_string())
        );
    }

    #[rstest]
    fn test_project_add_multiple_tasks() {
        let project = test_project("Test")
            .add_task(test_task_summary("Task 1"))
            .add_task(test_task_summary("Task 2"))
            .add_task(test_task_summary("Task 3"));

        assert_eq!(project.task_count(), 3);
    }

    #[rstest]
    fn test_project_remove_task() {
        let summary = test_task_summary("Task 1");
        let task_id = summary.task_id.clone();
        let project = test_project("Test").add_task(summary).remove_task(&task_id);

        assert_eq!(project.task_count(), 0);
        assert!(!project.has_task(&task_id));
    }

    #[rstest]
    fn test_project_update_existing_task() {
        let task_id = TaskId::generate();
        let summary1 = TaskSummary::new(
            task_id.clone(),
            "Task 1",
            TaskStatus::Pending,
            Priority::Low,
        );
        let summary2 = TaskSummary::new(
            task_id.clone(),
            "Updated Task",
            TaskStatus::InProgress,
            Priority::High,
        );

        let project = test_project("Test").add_task(summary1).add_task(summary2);

        assert_eq!(project.task_count(), 1);
        let task = project.get_task(&task_id).unwrap();
        assert_eq!(task.title, "Updated Task");
        assert_eq!(task.status, TaskStatus::InProgress);
        assert_eq!(task.priority, Priority::High);
    }

    #[rstest]
    fn test_project_increment_version() {
        let project = test_project("Test");
        assert_eq!(project.version, 1);
        let project = project.increment_version();
        assert_eq!(project.version, 2);
    }

    // -------------------------------------------------------------------------
    // Aggregation Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_project_count_by_status() {
        let project = test_project("Test")
            .add_task(test_task_summary_with_status("Task 1", TaskStatus::Pending))
            .add_task(test_task_summary_with_status("Task 2", TaskStatus::Pending))
            .add_task(test_task_summary_with_status(
                "Task 3",
                TaskStatus::InProgress,
            ))
            .add_task(test_task_summary_with_status(
                "Task 4",
                TaskStatus::Completed,
            ));

        assert_eq!(project.pending_count(), 2);
        assert_eq!(project.in_progress_count(), 1);
        assert_eq!(project.completed_count(), 1);
    }

    #[rstest]
    fn test_project_progress_empty() {
        let project = test_project("Test");
        assert!((project.progress() - 1.0).abs() < f64::EPSILON);
    }

    #[rstest]
    fn test_project_progress_all_pending() {
        let project = test_project("Test")
            .add_task(test_task_summary_with_status("Task 1", TaskStatus::Pending))
            .add_task(test_task_summary_with_status("Task 2", TaskStatus::Pending));

        assert!((project.progress() - 0.0).abs() < f64::EPSILON);
    }

    #[rstest]
    fn test_project_progress_half_completed() {
        let project = test_project("Test")
            .add_task(test_task_summary_with_status(
                "Task 1",
                TaskStatus::Completed,
            ))
            .add_task(test_task_summary_with_status("Task 2", TaskStatus::Pending));

        assert!((project.progress() - 0.5).abs() < f64::EPSILON);
    }

    #[rstest]
    fn test_project_progress_excludes_cancelled() {
        let project = test_project("Test")
            .add_task(test_task_summary_with_status(
                "Task 1",
                TaskStatus::Completed,
            ))
            .add_task(test_task_summary_with_status(
                "Task 2",
                TaskStatus::Cancelled,
            ))
            .add_task(test_task_summary_with_status("Task 3", TaskStatus::Pending));

        // 2 active tasks (1 completed, 1 pending), so 50% progress
        assert!((project.progress() - 0.5).abs() < f64::EPSILON);
    }

    #[rstest]
    fn test_project_progress_all_cancelled() {
        let project = test_project("Test")
            .add_task(test_task_summary_with_status(
                "Task 1",
                TaskStatus::Cancelled,
            ))
            .add_task(test_task_summary_with_status(
                "Task 2",
                TaskStatus::Cancelled,
            ));

        assert!((project.progress() - 1.0).abs() < f64::EPSILON);
    }

    #[rstest]
    fn test_project_count_by_priority() {
        let project = test_project("Test")
            .add_task(TaskSummary::new(
                TaskId::generate(),
                "Task 1",
                TaskStatus::Pending,
                Priority::Low,
            ))
            .add_task(TaskSummary::new(
                TaskId::generate(),
                "Task 2",
                TaskStatus::Pending,
                Priority::High,
            ))
            .add_task(TaskSummary::new(
                TaskId::generate(),
                "Task 3",
                TaskStatus::Pending,
                Priority::High,
            ))
            .add_task(TaskSummary::new(
                TaskId::generate(),
                "Task 4",
                TaskStatus::Pending,
                Priority::Critical,
            ));

        assert_eq!(project.count_by_priority(Priority::Low), 1);
        assert_eq!(project.count_by_priority(Priority::Medium), 0);
        assert_eq!(project.count_by_priority(Priority::High), 2);
        assert_eq!(project.count_by_priority(Priority::Critical), 1);
    }

    // -------------------------------------------------------------------------
    // Equality Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_project_equality_by_id() {
        let id = ProjectId::generate();
        let timestamp = Timestamp::now();
        let project1 = Project::new(id.clone(), "Project 1", timestamp.clone());
        let project2 = Project::new(id, "Project 2", timestamp);
        assert_eq!(project1, project2);
    }

    #[rstest]
    fn test_project_inequality_by_id() {
        let project1 = test_project("Project 1");
        let project2 = test_project("Project 2");
        assert_ne!(project1, project2);
    }

    // -------------------------------------------------------------------------
    // Serialization Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_project_serialization() {
        let project = test_project("Test Project")
            .with_description("Test description")
            .add_task(test_task_summary("Task 1"));

        let json = serde_json::to_string(&project).expect("Failed to serialize");
        let deserialized: Project = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.name, project.name);
        assert_eq!(deserialized.description, project.description);
        assert_eq!(deserialized.task_count(), project.task_count());
    }

    #[rstest]
    fn test_task_summary_serialization() {
        let summary = test_task_summary("Test Task");

        let json = serde_json::to_string(&summary).expect("Failed to serialize");
        let deserialized: TaskSummary = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.title, summary.title);
        assert_eq!(deserialized.status, summary.status);
        assert_eq!(deserialized.priority, summary.priority);
    }
}
