//! Task domain model.
//!
//! This module contains the core domain model for task management,
//! demonstrating the use of lambars' persistent data structures.

use chrono::{DateTime, Utc};
use lambars::persistent::{PersistentHashSet, PersistentList};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// =============================================================================
// Value Objects - Newtypes
// =============================================================================

/// Unique identifier for a task.
///
/// This is a newtype wrapper around UUID to provide type safety.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TaskId(Uuid);

impl TaskId {
    /// Creates a `TaskId` from an existing UUID.
    ///
    /// This is a pure function - it does not generate a new UUID.
    /// Use `TaskId::generate()` in an effect boundary (e.g., `AsyncIO`)
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

    /// Generates a new `TaskId` with a randomly generated UUID (v4).
    ///
    /// **Note**: This is an impure function (side effect: random number generation).
    /// It should be called within an effect boundary (e.g., `AsyncIO::new`).
    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    /// Generates a new `TaskId` with a time-ordered UUID (v7).
    ///
    /// **Note**: This is an impure function (side effect: time + random).
    /// It should be called within an effect boundary (e.g., `AsyncIO::new`).
    #[must_use]
    pub fn generate_v7() -> Self {
        Self(Uuid::now_v7())
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Unique identifier for a subtask.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubTaskId(Uuid);

impl SubTaskId {
    /// Creates a `SubTaskId` from an existing UUID.
    ///
    /// This is a pure function.
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the inner UUID.
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Generates a new `SubTaskId` with a randomly generated UUID (v4).
    ///
    /// **Note**: This is an impure function. Call within an effect boundary.
    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    /// Generates a new `SubTaskId` with a time-ordered UUID (v7).
    ///
    /// **Note**: This is an impure function. Call within an effect boundary.
    #[must_use]
    pub fn generate_v7() -> Self {
        Self(Uuid::now_v7())
    }
}

impl std::fmt::Display for SubTaskId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// A tag for categorizing tasks.
///
/// This is a newtype wrapper around String to provide type safety
/// and validation capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tag(String);

impl Tag {
    /// Creates a new `Tag` from a string.
    ///
    /// The tag name is trimmed and converted to lowercase for consistency.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        let name: String = name.into();
        Self(name.trim().to_lowercase())
    }

    /// Returns the tag name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Tag {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl From<&str> for Tag {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for Tag {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// A timestamp wrapper for `DateTime<Utc>`.
///
/// This provides a consistent timestamp type throughout the domain model.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    /// Creates a `Timestamp` from a `DateTime<Utc>`.
    ///
    /// This is a pure function.
    #[must_use]
    pub const fn from_datetime(datetime: DateTime<Utc>) -> Self {
        Self(datetime)
    }

    /// Returns the inner `DateTime<Utc>`.
    #[must_use]
    pub const fn as_datetime(&self) -> &DateTime<Utc> {
        &self.0
    }

    /// Returns the current time as a `Timestamp`.
    ///
    /// **Note**: This is an impure function (side effect: system clock).
    /// It should be called within an effect boundary (e.g., `AsyncIO::new`).
    #[must_use]
    pub fn now() -> Self {
        Self(Utc::now())
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0.format("%Y-%m-%d %H:%M:%S UTC"))
    }
}

// =============================================================================
// Enums
// =============================================================================

/// The status of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task has not been started yet.
    #[default]
    Pending,
    /// Task is currently being worked on.
    InProgress,
    /// Task has been completed successfully.
    Completed,
    /// Task has been cancelled and will not be completed.
    Cancelled,
}

impl TaskStatus {
    /// Returns `true` if the task is in a terminal state (Completed or Cancelled).
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled)
    }

    /// Returns `true` if the task is active (`Pending` or `InProgress`).
    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Pending | Self::InProgress)
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(formatter, "Pending"),
            Self::InProgress => write!(formatter, "In Progress"),
            Self::Completed => write!(formatter, "Completed"),
            Self::Cancelled => write!(formatter, "Cancelled"),
        }
    }
}

/// The priority level of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    /// Low priority (value: 0).
    #[default]
    Low,
    /// Medium priority (value: 1).
    Medium,
    /// High priority (value: 2).
    High,
    /// Critical priority (value: 3).
    Critical,
}

impl Priority {
    /// Returns the numeric value of the priority.
    ///
    /// Higher values indicate higher priority.
    #[must_use]
    pub const fn value(self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Medium => 1,
            Self::High => 2,
            Self::Critical => 3,
        }
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(formatter, "Low"),
            Self::Medium => write!(formatter, "Medium"),
            Self::High => write!(formatter, "High"),
            Self::Critical => write!(formatter, "Critical"),
        }
    }
}

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Priority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value().cmp(&other.value())
    }
}

// =============================================================================
// SubTask
// =============================================================================

/// A subtask within a task.
///
/// Subtasks are simpler units of work that belong to a parent task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubTask {
    /// Unique identifier for the subtask.
    pub subtask_id: SubTaskId,
    /// Title of the subtask.
    pub title: String,
    /// Whether the subtask has been completed.
    pub completed: bool,
}

impl SubTask {
    /// Creates a new subtask with the given ID and title.
    ///
    /// This is a pure function. Use `SubTaskId::generate()` within an
    /// effect boundary to create the ID.
    #[must_use]
    pub fn new(subtask_id: SubTaskId, title: impl Into<String>) -> Self {
        Self {
            subtask_id,
            title: title.into(),
            completed: false,
        }
    }

    /// Returns a new subtask with the completed flag set to the given value.
    #[must_use]
    pub fn with_completed(self, completed: bool) -> Self {
        Self { completed, ..self }
    }

    /// Returns a new subtask marked as completed.
    #[must_use]
    pub fn complete(self) -> Self {
        self.with_completed(true)
    }
}

// =============================================================================
// Task
// =============================================================================

/// The main task domain model.
///
/// This is the central domain entity for task management, using lambars'
/// persistent data structures for immutable collections.
///
/// # Lambars Features Used
///
/// - `PersistentHashSet`: For managing tags as an immutable set
/// - `PersistentList`: For maintaining an ordered list of subtasks
///
/// # Examples
///
/// ```ignore
/// use domain::task::{Task, Tag, Priority};
///
/// let task = Task::new("Implement feature X")
///     .with_description("Detailed description here")
///     .with_priority(Priority::High)
///     .add_tag(Tag::new("backend"))
///     .add_tag(Tag::new("urgent"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_field_names)]
pub struct Task {
    /// Unique identifier for the task.
    pub task_id: TaskId,
    /// Title of the task.
    pub title: String,
    /// Optional detailed description.
    pub description: Option<String>,
    /// Current status of the task.
    pub status: TaskStatus,
    /// Priority level of the task.
    pub priority: Priority,
    /// Set of tags for categorization (using persistent hash set).
    pub tags: PersistentHashSet<Tag>,
    /// Ordered list of subtasks (using persistent list).
    pub subtasks: PersistentList<SubTask>,
    /// Timestamp when the task was created.
    pub created_at: Timestamp,
    /// Timestamp when the task was last updated.
    pub updated_at: Timestamp,
    /// Version number for optimistic locking.
    pub version: u64,
}

impl Task {
    /// Creates a new task with the given ID, title, and timestamp.
    ///
    /// This is a pure function. Use `TaskId::generate()` and `Timestamp::now()`
    /// within an effect boundary to create the required parameters.
    ///
    /// The task is initialized with:
    /// - Status: Pending
    /// - Priority: Low
    /// - Empty tags and subtasks
    /// - Version: 1
    ///
    /// # Example (within effect boundary)
    ///
    /// ```ignore
    /// let workflow = AsyncIO::new(|| async {
    ///     let task_id = TaskId::generate_v7();
    ///     let now = Timestamp::now();
    ///     Ok(Task::new(task_id, "My task", now))
    /// });
    /// ```
    #[must_use]
    pub fn new(task_id: TaskId, title: impl Into<String>, timestamp: Timestamp) -> Self {
        Self {
            task_id,
            title: title.into(),
            description: None,
            status: TaskStatus::Pending,
            priority: Priority::Low,
            tags: PersistentHashSet::new(),
            subtasks: PersistentList::new(),
            created_at: timestamp.clone(),
            updated_at: timestamp,
            version: 1,
        }
    }

    // -------------------------------------------------------------------------
    // Builder-style methods (pure immutable updates)
    // -------------------------------------------------------------------------

    /// Returns a new task with the updated timestamp.
    ///
    /// Use this method to update the `updated_at` field when modifying the task.
    /// This is a pure function - the timestamp should be obtained within an
    /// effect boundary.
    #[must_use]
    pub fn with_updated_at(self, timestamp: Timestamp) -> Self {
        Self {
            updated_at: timestamp,
            ..self
        }
    }

    /// Returns a new task with the given description.
    #[must_use]
    pub fn with_description(self, description: impl Into<String>) -> Self {
        Self {
            description: Some(description.into()),
            ..self
        }
    }

    /// Returns a new task with the given status.
    #[must_use]
    pub fn with_status(self, status: TaskStatus) -> Self {
        Self { status, ..self }
    }

    /// Returns a new task with the given priority.
    #[must_use]
    pub fn with_priority(self, priority: Priority) -> Self {
        Self { priority, ..self }
    }

    /// Returns a new task with the given tags (replacing existing tags).
    #[must_use]
    pub fn with_tags(self, tags: PersistentHashSet<Tag>) -> Self {
        Self { tags, ..self }
    }

    /// Returns a new task with the given subtasks (replacing existing subtasks).
    #[must_use]
    pub fn with_subtasks(self, subtasks: PersistentList<SubTask>) -> Self {
        Self { subtasks, ..self }
    }

    /// Returns a new task with an incremented version.
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
    // Tag operations (pure)
    // -------------------------------------------------------------------------

    /// Returns a new task with the given tag added.
    ///
    /// If the tag already exists, the task is returned unchanged.
    #[must_use]
    pub fn add_tag(self, tag: Tag) -> Self {
        Self {
            tags: self.tags.insert(tag),
            ..self
        }
    }

    /// Returns a new task with the given tag removed.
    ///
    /// If the tag doesn't exist, the task is returned unchanged.
    #[must_use]
    pub fn remove_tag(self, tag: &Tag) -> Self {
        Self {
            tags: self.tags.remove(tag),
            ..self
        }
    }

    /// Returns `true` if the task has the given tag.
    #[must_use]
    pub fn has_tag(&self, tag: &Tag) -> bool {
        self.tags.contains(tag)
    }

    // -------------------------------------------------------------------------
    // Subtask operations (pure)
    // -------------------------------------------------------------------------

    /// Returns a new task with the given subtask prepended to the front.
    ///
    /// Subtasks are stored in a persistent list. New subtasks are prepended
    /// to the front (O(1) operation), meaning the most recently added subtask
    /// will be at the head of the list.
    #[must_use]
    pub fn prepend_subtask(self, subtask: SubTask) -> Self {
        Self {
            subtasks: self.subtasks.cons(subtask),
            ..self
        }
    }

    /// Returns the number of subtasks.
    #[must_use]
    pub const fn subtask_count(&self) -> usize {
        self.subtasks.len()
    }

    /// Returns the number of completed subtasks.
    #[must_use]
    pub fn completed_subtask_count(&self) -> usize {
        self.subtasks
            .iter()
            .filter(|subtask| subtask.completed)
            .count()
    }

    /// Returns the progress of subtasks as a ratio (0.0 to 1.0).
    ///
    /// Returns 1.0 if there are no subtasks.
    #[must_use]
    pub fn subtask_progress(&self) -> f64 {
        let total = self.subtask_count();
        if total == 0 {
            return 1.0;
        }
        #[allow(clippy::cast_precision_loss)]
        let progress = self.completed_subtask_count() as f64 / total as f64;
        progress
    }

    // -------------------------------------------------------------------------
    // Status operations
    // -------------------------------------------------------------------------

    /// Returns a new task marked as in progress.
    #[must_use]
    pub fn start(self) -> Self {
        self.with_status(TaskStatus::InProgress)
    }

    /// Returns a new task marked as completed.
    #[must_use]
    pub fn complete(self) -> Self {
        self.with_status(TaskStatus::Completed)
    }

    /// Returns a new task marked as cancelled.
    #[must_use]
    pub fn cancel(self) -> Self {
        self.with_status(TaskStatus::Cancelled)
    }

    /// Returns `true` if the task is in a terminal state.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }

    /// Returns `true` if the task is active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.status.is_active()
    }
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.task_id == other.task_id
    }
}

impl Eq for Task {}

impl std::hash::Hash for Task {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.task_id.hash(state);
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
    // Helper function for tests
    // -------------------------------------------------------------------------

    fn test_task(title: &str) -> Task {
        Task::new(TaskId::generate(), title, Timestamp::now())
    }

    fn test_subtask(title: &str) -> SubTask {
        SubTask::new(SubTaskId::generate(), title)
    }

    // -------------------------------------------------------------------------
    // TaskId Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_task_id_generate_creates_unique_ids() {
        let id1 = TaskId::generate();
        let id2 = TaskId::generate();
        assert_ne!(id1, id2);
    }

    #[rstest]
    fn test_task_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let task_id = TaskId::from_uuid(uuid);
        assert_eq!(task_id.as_uuid(), &uuid);
    }

    #[rstest]
    fn test_task_id_display() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let task_id = TaskId::from_uuid(uuid);
        assert_eq!(format!("{task_id}"), "550e8400-e29b-41d4-a716-446655440000");
    }

    // -------------------------------------------------------------------------
    // Tag Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_tag_normalizes_input() {
        let tag = Tag::new("  Backend  ");
        assert_eq!(tag.as_str(), "backend");
    }

    #[rstest]
    fn test_tag_from_str() {
        let tag: Tag = "Feature".into();
        assert_eq!(tag.as_str(), "feature");
    }

    #[rstest]
    fn test_tag_equality() {
        let tag1 = Tag::new("backend");
        let tag2 = Tag::new("BACKEND");
        assert_eq!(tag1, tag2);
    }

    // -------------------------------------------------------------------------
    // TaskStatus Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_task_status_is_terminal() {
        assert!(!TaskStatus::Pending.is_terminal());
        assert!(!TaskStatus::InProgress.is_terminal());
        assert!(TaskStatus::Completed.is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
    }

    #[rstest]
    fn test_task_status_is_active() {
        assert!(TaskStatus::Pending.is_active());
        assert!(TaskStatus::InProgress.is_active());
        assert!(!TaskStatus::Completed.is_active());
        assert!(!TaskStatus::Cancelled.is_active());
    }

    // -------------------------------------------------------------------------
    // Priority Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_priority_values() {
        assert_eq!(Priority::Low.value(), 0);
        assert_eq!(Priority::Medium.value(), 1);
        assert_eq!(Priority::High.value(), 2);
        assert_eq!(Priority::Critical.value(), 3);
    }

    #[rstest]
    fn test_priority_ordering() {
        assert!(Priority::Low < Priority::Medium);
        assert!(Priority::Medium < Priority::High);
        assert!(Priority::High < Priority::Critical);
    }

    // -------------------------------------------------------------------------
    // SubTask Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_subtask_new() {
        let subtask = test_subtask("Test subtask");
        assert_eq!(subtask.title, "Test subtask");
        assert!(!subtask.completed);
    }

    #[rstest]
    fn test_subtask_complete() {
        let subtask = test_subtask("Test subtask").complete();
        assert!(subtask.completed);
    }

    // -------------------------------------------------------------------------
    // Task Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_task_new() {
        let task = test_task("Test task");
        assert_eq!(task.title, "Test task");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.priority, Priority::Low);
        assert!(task.tags.is_empty());
        assert!(task.subtasks.is_empty());
        assert_eq!(task.version, 1);
    }

    #[rstest]
    fn test_task_with_description() {
        let task = test_task("Test").with_description("Description");
        assert_eq!(task.description, Some("Description".to_string()));
    }

    #[rstest]
    fn test_task_with_priority() {
        let task = test_task("Test").with_priority(Priority::Critical);
        assert_eq!(task.priority, Priority::Critical);
    }

    #[rstest]
    fn test_task_add_tag() {
        let tag = Tag::new("backend");
        let task = test_task("Test").add_tag(tag.clone());
        assert!(task.has_tag(&tag));
        assert_eq!(task.tags.len(), 1);
    }

    #[rstest]
    fn test_task_remove_tag() {
        let tag = Tag::new("backend");
        let task = test_task("Test").add_tag(tag.clone()).remove_tag(&tag);
        assert!(!task.has_tag(&tag));
        assert!(task.tags.is_empty());
    }

    #[rstest]
    fn test_task_prepend_subtask() {
        let subtask = test_subtask("Subtask 1");
        let task = test_task("Test").prepend_subtask(subtask);
        assert_eq!(task.subtask_count(), 1);
    }

    #[rstest]
    fn test_task_subtask_progress() {
        let task = test_task("Test")
            .prepend_subtask(test_subtask("Subtask 1").complete())
            .prepend_subtask(test_subtask("Subtask 2"));
        assert!((task.subtask_progress() - 0.5).abs() < f64::EPSILON);
    }

    #[rstest]
    fn test_task_subtask_progress_empty() {
        let task = test_task("Test");
        assert!((task.subtask_progress() - 1.0).abs() < f64::EPSILON);
    }

    #[rstest]
    fn test_task_status_transitions() {
        let task = test_task("Test");
        assert_eq!(task.status, TaskStatus::Pending);

        let task = task.start();
        assert_eq!(task.status, TaskStatus::InProgress);

        let task = task.complete();
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.is_terminal());
    }

    #[rstest]
    fn test_task_cancel() {
        let task = test_task("Test").cancel();
        assert_eq!(task.status, TaskStatus::Cancelled);
        assert!(task.is_terminal());
    }

    #[rstest]
    fn test_task_increment_version() {
        let task = test_task("Test");
        assert_eq!(task.version, 1);
        let task = task.increment_version();
        assert_eq!(task.version, 2);
    }

    #[rstest]
    fn test_task_equality_by_id() {
        let id = TaskId::generate();
        let timestamp = Timestamp::now();
        let task1 = Task::new(id.clone(), "Task 1", timestamp.clone());
        let task2 = Task::new(id, "Task 2", timestamp);
        assert_eq!(task1, task2);
    }

    #[rstest]
    fn test_task_inequality_by_id() {
        let task1 = test_task("Task 1");
        let task2 = test_task("Task 2");
        assert_ne!(task1, task2);
    }

    // -------------------------------------------------------------------------
    // Serialization Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_task_serialization() {
        let task = test_task("Test task")
            .with_description("Test description")
            .with_priority(Priority::High)
            .add_tag(Tag::new("backend"));

        let json = serde_json::to_string(&task).expect("Failed to serialize");
        let deserialized: Task = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.title, task.title);
        assert_eq!(deserialized.description, task.description);
        assert_eq!(deserialized.priority, task.priority);
        assert_eq!(deserialized.tags.len(), task.tags.len());
    }

    #[rstest]
    fn test_subtask_serialization() {
        let subtask = test_subtask("Test subtask").complete();

        let json = serde_json::to_string(&subtask).expect("Failed to serialize");
        let deserialized: SubTask = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.title, subtask.title);
        assert_eq!(deserialized.completed, subtask.completed);
    }
}
