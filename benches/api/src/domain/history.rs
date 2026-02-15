//! History domain model (Event Sourcing).
//!
//! This module contains the task event definitions for event sourcing,
//! demonstrating the use of `PersistentList` and `Continuation` monad.

use lambars::control::Continuation;
use lambars::persistent::PersistentList;
use serde::{Deserialize, Serialize};

use super::project::ProjectId;
use super::task::{Priority, SubTaskId, Tag, Task, TaskId, TaskStatus, Timestamp};

// =============================================================================
// Event ID
// =============================================================================

/// Unique identifier for an event.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(uuid::Uuid);

impl EventId {
    /// Creates an `EventId` from an existing UUID.
    ///
    /// This is a pure function.
    #[must_use]
    pub const fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the inner UUID.
    #[must_use]
    pub const fn as_uuid(&self) -> &uuid::Uuid {
        &self.0
    }

    /// Generates a new `EventId` with a randomly generated UUID (v4).
    ///
    /// **Note**: This is an impure function. Call within an effect boundary.
    #[must_use]
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Generates a new `EventId` with a time-ordered UUID (v7).
    ///
    /// **Note**: This is an impure function. Call within an effect boundary.
    #[must_use]
    pub fn generate_v7() -> Self {
        Self(uuid::Uuid::now_v7())
    }
}

impl std::fmt::Display for EventId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

// =============================================================================
// Event Payloads
// =============================================================================

/// Payload for a task creation event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCreated {
    pub title: String,
    pub description: Option<String>,
    pub priority: Priority,
    pub status: TaskStatus,
}

/// Payload for a task title update event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskTitleUpdated {
    pub old_title: String,
    pub new_title: String,
}

/// Payload for a task description update event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskDescriptionUpdated {
    pub old_description: Option<String>,
    pub new_description: Option<String>,
}

/// Payload for a status change event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusChanged {
    pub old_status: TaskStatus,
    pub new_status: TaskStatus,
}

/// Payload for a priority change event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriorityChanged {
    pub old_priority: Priority,
    pub new_priority: Priority,
}

/// Payload for a tag added event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagAdded {
    pub tag: Tag,
}

/// Payload for a tag removed event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagRemoved {
    pub tag: Tag,
}

/// Payload for a subtask added event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubTaskAdded {
    pub subtask_id: SubTaskId,
    pub title: String,
}

/// Payload for a subtask completed event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubTaskCompleted {
    pub subtask_id: SubTaskId,
}

/// Payload for a project assignment event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectAssigned {
    pub project_id: ProjectId,
}

/// Payload for a project removal event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRemoved {
    pub project_id: ProjectId,
}

// =============================================================================
// Task Event
// =============================================================================

/// Represents a change event for a task.
///
/// This is an algebraic data type (sum type) representing all possible
/// events that can occur on a task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskEventKind {
    /// Task was created.
    Created(TaskCreated),
    /// Task title was updated.
    TitleUpdated(TaskTitleUpdated),
    /// Task description was updated.
    DescriptionUpdated(TaskDescriptionUpdated),
    /// Task status was changed.
    StatusChanged(StatusChanged),
    /// Task priority was changed.
    PriorityChanged(PriorityChanged),
    /// A tag was added to the task.
    TagAdded(TagAdded),
    /// A tag was removed from the task.
    TagRemoved(TagRemoved),
    /// A subtask was added to the task.
    SubTaskAdded(SubTaskAdded),
    /// A subtask was marked as completed.
    SubTaskCompleted(SubTaskCompleted),
    /// Task was assigned to a project.
    ProjectAssigned(ProjectAssigned),
    /// Task was removed from a project.
    ProjectRemoved(ProjectRemoved),
}

/// A task event with metadata.
///
/// Contains the event payload along with metadata such as event ID,
/// task ID, timestamp, and version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskEvent {
    /// Unique identifier for this event.
    pub event_id: EventId,
    /// The task this event belongs to.
    pub task_id: TaskId,
    /// When the event occurred.
    pub timestamp: Timestamp,
    /// The version of the task after this event.
    pub version: u64,
    /// The event payload.
    pub kind: TaskEventKind,
}

impl TaskEvent {
    /// Creates a new task event.
    ///
    /// This is a pure function. Use `EventId::generate()` and `Timestamp::now()`
    /// within an effect boundary to create the required parameters.
    #[must_use]
    pub const fn new(
        event_id: EventId,
        task_id: TaskId,
        timestamp: Timestamp,
        version: u64,
        kind: TaskEventKind,
    ) -> Self {
        Self {
            event_id,
            task_id,
            timestamp,
            version,
            kind,
        }
    }

    /// Returns true if this is a creation event.
    #[must_use]
    pub const fn is_creation(&self) -> bool {
        matches!(self.kind, TaskEventKind::Created(_))
    }

    /// Returns true if this is a status change event.
    #[must_use]
    pub const fn is_status_change(&self) -> bool {
        matches!(self.kind, TaskEventKind::StatusChanged(_))
    }
}

// =============================================================================
// Task History
// =============================================================================

/// A type alias for task history using persistent list.
///
/// The history stores events in reverse chronological order (newest first)
/// because `PersistentList::cons` prepends elements.
pub type TaskHistory = PersistentList<TaskEvent>;

/// Extension trait for `TaskHistory` providing aggregation operations.
pub trait TaskHistoryExt {
    /// Returns the number of events in the history.
    fn event_count(&self) -> usize;

    /// Returns the latest event, if any.
    fn latest_event(&self) -> Option<&TaskEvent>;

    /// Returns the current version (version of the latest event, or 0 if empty).
    fn current_version(&self) -> u64;

    /// Filters events by a predicate.
    fn filter_events<F>(&self, predicate: F) -> Vec<&TaskEvent>
    where
        F: Fn(&TaskEvent) -> bool;

    /// Returns all status change events.
    fn status_changes(&self) -> Vec<&TaskEvent>;

    /// Returns all tag-related events.
    fn tag_events(&self) -> Vec<&TaskEvent>;
}

impl TaskHistoryExt for TaskHistory {
    fn event_count(&self) -> usize {
        self.len()
    }

    fn latest_event(&self) -> Option<&TaskEvent> {
        self.head()
    }

    fn current_version(&self) -> u64 {
        self.head().map_or(0, |event| event.version)
    }

    fn filter_events<F>(&self, predicate: F) -> Vec<&TaskEvent>
    where
        F: Fn(&TaskEvent) -> bool,
    {
        self.iter().filter(|event| predicate(event)).collect()
    }

    fn status_changes(&self) -> Vec<&TaskEvent> {
        self.filter_events(TaskEvent::is_status_change)
    }

    fn tag_events(&self) -> Vec<&TaskEvent> {
        self.filter_events(|event| {
            matches!(
                event.kind,
                TaskEventKind::TagAdded(_) | TaskEventKind::TagRemoved(_)
            )
        })
    }
}

// =============================================================================
// Continuation-based Lazy Loading
// =============================================================================

/// Creates a `Continuation` for lazy loading of task history.
///
/// This demonstrates the use of the Continuation monad for deferred
/// computation. The actual loading is performed when the continuation
/// is run with a callback.
///
/// # Type Signature
///
/// `Continuation<R, TaskHistory>` represents a computation that:
/// - Takes a callback `(TaskHistory -> R)` as input
/// - Produces a result of type `R`
///
/// # Example
///
/// ```ignore
/// // The loader function performs the actual I/O
/// let loader = |task_id: &TaskId| -> TaskHistory {
///     // Load from database
///     todo!()
/// };
///
/// // Create a continuation that will load the history when needed
/// let continuation = load_history_continuation(&task_id, loader);
///
/// // Run the continuation with a callback to process the history
/// let result = continuation.run(|history| {
///     format!("Loaded {} events", history.len())
/// });
/// ```
#[must_use]
pub fn load_history_continuation<R, F>(task_id: &TaskId, loader: F) -> Continuation<R, TaskHistory>
where
    R: 'static,
    F: FnOnce(&TaskId) -> TaskHistory + 'static,
{
    let task_id = *task_id;
    Continuation::new(move |k: Box<dyn FnOnce(TaskHistory) -> R>| {
        let history = loader(&task_id);
        k(history)
    })
}

/// Creates a `Continuation` that combines two history loading operations.
///
/// This demonstrates how to compose Continuation monads using `flat_map`.
///
/// # Example
///
/// ```ignore
/// let combined = combine_histories(
///     &task_id1, loader1,
///     &task_id2, loader2,
/// );
///
/// let total_events = combined.run(|(h1, h2)| h1.len() + h2.len());
/// ```
#[must_use]
pub fn combine_histories<R, F1, F2>(
    task_id1: &TaskId,
    loader1: F1,
    task_id2: &TaskId,
    loader2: F2,
) -> Continuation<R, (TaskHistory, TaskHistory)>
where
    R: 'static,
    F1: FnOnce(&TaskId) -> TaskHistory + 'static,
    F2: FnOnce(&TaskId) -> TaskHistory + 'static,
{
    let id1 = *task_id1;
    let id2 = *task_id2;

    load_history_continuation(&id1, loader1)
        .flat_map(move |h1| load_history_continuation(&id2, loader2).map(move |h2| (h1, h2)))
}

// =============================================================================
// Event Generation Pure Functions
// =============================================================================

/// Creates a task created event (pure function).
///
/// This function generates the event payload from a `Task` reference.
/// The event ID, timestamp, and version are provided as parameters to
/// ensure purity - they should be generated within an I/O boundary.
///
/// # Arguments
///
/// * `task` - The task that was created
/// * `event_id` - Unique identifier for this event (generated in I/O boundary)
/// * `timestamp` - When the event occurred (generated in I/O boundary)
/// * `version` - This event's version (`expected_version` + 1, i.e., 1 for initial creation)
///
/// # Version Semantics
///
/// The `version` parameter represents the resulting version after this event is applied:
/// - For the first event: `expected_version = 0`, so `version = 1`
/// - For subsequent events: `version = current_version + 1`
///
/// This matches the `EventStore`'s optimistic locking semantics where `append(event, expected_version)`
/// verifies that `event.version == expected_version + 1`.
///
/// # Example
///
/// ```ignore
/// let event = create_task_created_event(
///     &task,
///     EventId::generate_v7(),  // Generated in I/O boundary
///     Timestamp::now(),        // Generated in I/O boundary
///     1,                       // First event for this task (expected_version=0 + 1)
/// );
/// ```
#[must_use]
pub fn create_task_created_event(
    task: &Task,
    event_id: EventId,
    timestamp: Timestamp,
    version: u64,
) -> TaskEvent {
    TaskEvent::new(
        event_id,
        task.task_id,
        timestamp,
        version,
        TaskEventKind::Created(TaskCreated {
            title: task.title.clone(),
            description: task.description.clone(),
            priority: task.priority,
            status: task.status,
        }),
    )
}

/// Creates a status changed event (pure function).
///
/// This function generates the event payload for a status change.
/// Use this when a task's status is being updated.
///
/// # Arguments
///
/// * `task_id` - The ID of the task whose status changed
/// * `old_status` - The previous status
/// * `new_status` - The new status
/// * `event_id` - Unique identifier for this event (generated in I/O boundary)
/// * `timestamp` - When the event occurred (generated in I/O boundary)
/// * `version` - This event's version (`current_version` + 1)
///
/// # Version Semantics
///
/// The `version` parameter represents the resulting version after this event is applied:
/// `version = current_version + 1` where `current_version` is obtained from
/// `EventStore::get_current_version()`.
///
/// # Example
///
/// ```ignore
/// let current_version = event_store.get_current_version(&task_id).await?;
/// let event = create_status_changed_event(
///     &task_id,
///     TaskStatus::Pending,
///     TaskStatus::InProgress,
///     EventId::generate_v7(),
///     Timestamp::now(),
///     current_version + 1,  // This event's version
/// );
/// event_store.append(&event, current_version).await?;  // expected_version = current_version
/// ```
#[must_use]
pub const fn create_status_changed_event(
    task_id: &TaskId,
    old_status: TaskStatus,
    new_status: TaskStatus,
    event_id: EventId,
    timestamp: Timestamp,
    version: u64,
) -> TaskEvent {
    TaskEvent::new(
        event_id,
        *task_id,
        timestamp,
        version,
        TaskEventKind::StatusChanged(StatusChanged {
            old_status,
            new_status,
        }),
    )
}

/// Creates a priority changed event (pure function).
///
/// This function generates the event payload for a priority change.
/// Use this when a task's priority is being updated.
///
/// # Arguments
///
/// * `task_id` - The ID of the task whose priority changed
/// * `old_priority` - The previous priority
/// * `new_priority` - The new priority
/// * `event_id` - Unique identifier for this event (generated in I/O boundary)
/// * `timestamp` - When the event occurred (generated in I/O boundary)
/// * `version` - This event's version (`current_version` + 1)
#[must_use]
pub const fn create_priority_changed_event(
    task_id: &TaskId,
    old_priority: Priority,
    new_priority: Priority,
    event_id: EventId,
    timestamp: Timestamp,
    version: u64,
) -> TaskEvent {
    TaskEvent::new(
        event_id,
        *task_id,
        timestamp,
        version,
        TaskEventKind::PriorityChanged(PriorityChanged {
            old_priority,
            new_priority,
        }),
    )
}

/// Creates a title updated event (pure function).
///
/// This function generates the event payload for a title change.
/// Use this when a task's title is being updated.
///
/// # Arguments
///
/// * `task_id` - The ID of the task whose title changed
/// * `old_title` - The previous title
/// * `new_title` - The new title
/// * `event_id` - Unique identifier for this event (generated in I/O boundary)
/// * `timestamp` - When the event occurred (generated in I/O boundary)
/// * `version` - This event's version (`current_version` + 1)
#[must_use]
pub fn create_title_updated_event(
    task_id: &TaskId,
    old_title: &str,
    new_title: &str,
    event_id: EventId,
    timestamp: Timestamp,
    version: u64,
) -> TaskEvent {
    TaskEvent::new(
        event_id,
        *task_id,
        timestamp,
        version,
        TaskEventKind::TitleUpdated(TaskTitleUpdated {
            old_title: old_title.to_string(),
            new_title: new_title.to_string(),
        }),
    )
}

/// Creates a description updated event (pure function).
///
/// This function generates the event payload for a description change.
/// Use this when a task's description is being updated.
///
/// # Arguments
///
/// * `task_id` - The ID of the task whose description changed
/// * `old_description` - The previous description (None if not set)
/// * `new_description` - The new description (None to clear)
/// * `event_id` - Unique identifier for this event (generated in I/O boundary)
/// * `timestamp` - When the event occurred (generated in I/O boundary)
/// * `version` - This event's version (`current_version` + 1)
#[must_use]
pub fn create_description_updated_event(
    task_id: &TaskId,
    old_description: Option<&str>,
    new_description: Option<&str>,
    event_id: EventId,
    timestamp: Timestamp,
    version: u64,
) -> TaskEvent {
    TaskEvent::new(
        event_id,
        *task_id,
        timestamp,
        version,
        TaskEventKind::DescriptionUpdated(TaskDescriptionUpdated {
            old_description: old_description.map(String::from),
            new_description: new_description.map(String::from),
        }),
    )
}

/// Creates a tag added event (pure function).
///
/// This function generates the event payload for adding a tag.
/// Use this when a tag is being added to a task.
///
/// # Arguments
///
/// * `task_id` - The ID of the task to which the tag was added
/// * `tag` - The tag that was added
/// * `event_id` - Unique identifier for this event (generated in I/O boundary)
/// * `timestamp` - When the event occurred (generated in I/O boundary)
/// * `version` - This event's version (`current_version` + 1)
#[must_use]
pub fn create_tag_added_event(
    task_id: &TaskId,
    tag: &Tag,
    event_id: EventId,
    timestamp: Timestamp,
    version: u64,
) -> TaskEvent {
    TaskEvent::new(
        event_id,
        *task_id,
        timestamp,
        version,
        TaskEventKind::TagAdded(TagAdded { tag: tag.clone() }),
    )
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

    fn test_event(kind: TaskEventKind, version: u64) -> TaskEvent {
        TaskEvent::new(
            EventId::generate(),
            TaskId::generate(),
            Timestamp::now(),
            version,
            kind,
        )
    }

    fn test_created_event(version: u64) -> TaskEvent {
        test_event(
            TaskEventKind::Created(TaskCreated {
                title: "Test Task".to_string(),
                description: None,
                priority: Priority::Low,
                status: TaskStatus::Pending,
            }),
            version,
        )
    }

    fn test_status_changed_event(
        old_status: TaskStatus,
        new_status: TaskStatus,
        version: u64,
    ) -> TaskEvent {
        test_event(
            TaskEventKind::StatusChanged(StatusChanged {
                old_status,
                new_status,
            }),
            version,
        )
    }

    fn test_tag_added_event(tag: &str, version: u64) -> TaskEvent {
        test_event(
            TaskEventKind::TagAdded(TagAdded { tag: Tag::new(tag) }),
            version,
        )
    }

    // -------------------------------------------------------------------------
    // EventId Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_event_id_generate_creates_unique_ids() {
        let id1 = EventId::generate();
        let id2 = EventId::generate();
        assert_ne!(id1, id2);
    }

    #[rstest]
    fn test_event_id_from_uuid() {
        let uuid = uuid::Uuid::new_v4();
        let event_id = EventId::from_uuid(uuid);
        assert_eq!(event_id.as_uuid(), &uuid);
    }

    // -------------------------------------------------------------------------
    // TaskEvent Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_task_event_new() {
        let event = test_created_event(1);
        assert_eq!(event.version, 1);
        assert!(event.is_creation());
    }

    #[rstest]
    fn test_task_event_is_status_change() {
        let event = test_status_changed_event(TaskStatus::Pending, TaskStatus::InProgress, 2);
        assert!(event.is_status_change());
        assert!(!event.is_creation());
    }

    // -------------------------------------------------------------------------
    // TaskHistory Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_task_history_empty() {
        let history: TaskHistory = PersistentList::new();
        assert_eq!(history.event_count(), 0);
        assert!(history.latest_event().is_none());
        assert_eq!(history.current_version(), 0);
    }

    #[rstest]
    fn test_task_history_with_events() {
        let history: TaskHistory = PersistentList::new()
            .cons(test_created_event(1))
            .cons(test_status_changed_event(
                TaskStatus::Pending,
                TaskStatus::InProgress,
                2,
            ))
            .cons(test_tag_added_event("important", 3));

        assert_eq!(history.event_count(), 3);
        assert_eq!(history.current_version(), 3);
    }

    #[rstest]
    fn test_task_history_latest_event() {
        let event1 = test_created_event(1);
        let event2 = test_status_changed_event(TaskStatus::Pending, TaskStatus::InProgress, 2);

        let history: TaskHistory = PersistentList::new().cons(event1).cons(event2);

        // cons prepends, so event2 is the latest
        assert_eq!(history.latest_event().map(|e| e.version), Some(2));
    }

    #[rstest]
    fn test_task_history_status_changes() {
        let history: TaskHistory = PersistentList::new()
            .cons(test_created_event(1))
            .cons(test_status_changed_event(
                TaskStatus::Pending,
                TaskStatus::InProgress,
                2,
            ))
            .cons(test_tag_added_event("important", 3))
            .cons(test_status_changed_event(
                TaskStatus::InProgress,
                TaskStatus::Completed,
                4,
            ));

        let status_changes = history.status_changes();
        assert_eq!(status_changes.len(), 2);
    }

    #[rstest]
    fn test_task_history_tag_events() {
        let history: TaskHistory = PersistentList::new()
            .cons(test_created_event(1))
            .cons(test_tag_added_event("important", 2))
            .cons(test_tag_added_event("urgent", 3))
            .cons(test_event(
                TaskEventKind::TagRemoved(TagRemoved {
                    tag: Tag::new("important"),
                }),
                4,
            ));

        let tag_events = history.tag_events();
        assert_eq!(tag_events.len(), 3);
    }

    // -------------------------------------------------------------------------
    // Continuation Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_load_history_continuation() {
        let task_id = TaskId::generate();

        let loader = |_: &TaskId| -> TaskHistory {
            PersistentList::new()
                .cons(test_created_event(1))
                .cons(test_status_changed_event(
                    TaskStatus::Pending,
                    TaskStatus::InProgress,
                    2,
                ))
        };

        let continuation = load_history_continuation(&task_id, loader);

        let result = continuation.run(|history| history.event_count());
        assert_eq!(result, 2);
    }

    #[rstest]
    fn test_load_history_continuation_with_transform() {
        let task_id = TaskId::generate();

        let loader = |_: &TaskId| -> TaskHistory {
            PersistentList::new()
                .cons(test_created_event(1))
                .cons(test_status_changed_event(
                    TaskStatus::Pending,
                    TaskStatus::InProgress,
                    2,
                ))
                .cons(test_status_changed_event(
                    TaskStatus::InProgress,
                    TaskStatus::Completed,
                    3,
                ))
        };

        let continuation = load_history_continuation(&task_id, loader);

        let status_change_count = continuation.run(|history| history.status_changes().len());
        assert_eq!(status_change_count, 2);
    }

    #[rstest]
    fn test_combine_histories() {
        let task_id1 = TaskId::generate();
        let task_id2 = TaskId::generate();

        let loader1 = |_: &TaskId| -> TaskHistory {
            PersistentList::new()
                .cons(test_created_event(1))
                .cons(test_tag_added_event("a", 2))
        };

        let loader2 = |_: &TaskId| -> TaskHistory {
            PersistentList::new()
                .cons(test_created_event(1))
                .cons(test_tag_added_event("b", 2))
                .cons(test_tag_added_event("c", 3))
        };

        let combined = combine_histories(&task_id1, loader1, &task_id2, loader2);

        let (count1, count2) = combined.run(|(h1, h2)| (h1.event_count(), h2.event_count()));
        assert_eq!(count1, 2);
        assert_eq!(count2, 3);
    }

    // -------------------------------------------------------------------------
    // Serialization Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_task_event_serialization() {
        let event = test_created_event(1);

        let json = serde_json::to_string(&event).expect("Failed to serialize");
        let deserialized: TaskEvent = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.version, event.version);
        assert!(deserialized.is_creation());
    }

    #[rstest]
    fn test_status_changed_event_serialization() {
        let event = test_status_changed_event(TaskStatus::Pending, TaskStatus::InProgress, 2);

        let json = serde_json::to_string(&event).expect("Failed to serialize");
        let deserialized: TaskEvent = serde_json::from_str(&json).expect("Failed to deserialize");

        assert!(deserialized.is_status_change());

        if let TaskEventKind::StatusChanged(payload) = &deserialized.kind {
            assert_eq!(payload.old_status, TaskStatus::Pending);
            assert_eq!(payload.new_status, TaskStatus::InProgress);
        } else {
            panic!("Expected StatusChanged event");
        }
    }

    #[rstest]
    fn test_tag_added_event_serialization() {
        let event = test_tag_added_event("important", 2);

        let json = serde_json::to_string(&event).expect("Failed to serialize");
        let deserialized: TaskEvent = serde_json::from_str(&json).expect("Failed to deserialize");

        if let TaskEventKind::TagAdded(payload) = &deserialized.kind {
            assert_eq!(payload.tag.as_str(), "important");
        } else {
            panic!("Expected TagAdded event");
        }
    }

    // -------------------------------------------------------------------------
    // Event Generation Pure Function Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_create_task_created_event_generates_correct_payload() {
        let task_id = TaskId::generate();
        let timestamp = Timestamp::now();
        let task = Task::new(task_id, "Test Task", timestamp.clone())
            .with_description("Test description")
            .with_priority(Priority::High);

        // Initial event version is 1 (expected_version=0 + 1)
        let event = create_task_created_event(
            &task,
            EventId::generate_v7(),
            timestamp,
            1, // Initial event version (expected_version=0 + 1)
        );

        // Verify event metadata
        assert_eq!(event.task_id, task_id);
        assert_eq!(event.version, 1);

        // Verify payload
        if let TaskEventKind::Created(payload) = &event.kind {
            assert_eq!(payload.title, "Test Task");
            assert_eq!(payload.description, Some("Test description".to_string()));
            assert_eq!(payload.priority, Priority::High);
            assert_eq!(payload.status, TaskStatus::Pending);
        } else {
            panic!("Expected Created event");
        }
    }

    #[rstest]
    fn test_create_task_created_event_with_different_status() {
        let task = Task::new(TaskId::generate(), "Task", Timestamp::now())
            .with_status(TaskStatus::InProgress);

        // Initial event version is 1 (expected_version=0 + 1)
        let event = create_task_created_event(&task, EventId::generate_v7(), Timestamp::now(), 1);

        if let TaskEventKind::Created(payload) = &event.kind {
            assert_eq!(payload.status, TaskStatus::InProgress);
        } else {
            panic!("Expected Created event");
        }
    }

    #[rstest]
    fn test_create_status_changed_event_generates_correct_payload() {
        let task_id = TaskId::generate();
        let event = create_status_changed_event(
            &task_id,
            TaskStatus::Pending,
            TaskStatus::InProgress,
            EventId::generate_v7(),
            Timestamp::now(),
            5, // Current version
        );

        // Verify event metadata
        assert_eq!(event.task_id, task_id);
        assert_eq!(event.version, 5);

        // Verify payload
        if let TaskEventKind::StatusChanged(payload) = &event.kind {
            assert_eq!(payload.old_status, TaskStatus::Pending);
            assert_eq!(payload.new_status, TaskStatus::InProgress);
        } else {
            panic!("Expected StatusChanged event");
        }
    }

    #[rstest]
    fn test_create_title_updated_event_generates_correct_payload() {
        let task_id = TaskId::generate();
        let event = create_title_updated_event(
            &task_id,
            "Old Title",
            "New Title",
            EventId::generate_v7(),
            Timestamp::now(),
            3,
        );

        assert_eq!(event.task_id, task_id);
        assert_eq!(event.version, 3);

        if let TaskEventKind::TitleUpdated(payload) = &event.kind {
            assert_eq!(payload.old_title, "Old Title");
            assert_eq!(payload.new_title, "New Title");
        } else {
            panic!("Expected TitleUpdated event");
        }
    }

    #[rstest]
    fn test_create_priority_changed_event_generates_correct_payload() {
        let task_id = TaskId::generate();
        let event = create_priority_changed_event(
            &task_id,
            Priority::Low,
            Priority::Critical,
            EventId::generate_v7(),
            Timestamp::now(),
            4,
        );

        assert_eq!(event.task_id, task_id);
        assert_eq!(event.version, 4);

        if let TaskEventKind::PriorityChanged(payload) = &event.kind {
            assert_eq!(payload.old_priority, Priority::Low);
            assert_eq!(payload.new_priority, Priority::Critical);
        } else {
            panic!("Expected PriorityChanged event");
        }
    }

    #[rstest]
    fn test_create_tag_added_event_generates_correct_payload() {
        let task_id = TaskId::generate();
        let tag = Tag::new("important");
        let event =
            create_tag_added_event(&task_id, &tag, EventId::generate_v7(), Timestamp::now(), 2);

        assert_eq!(event.task_id, task_id);
        assert_eq!(event.version, 2);

        if let TaskEventKind::TagAdded(payload) = &event.kind {
            assert_eq!(payload.tag.as_str(), "important");
        } else {
            panic!("Expected TagAdded event");
        }
    }

    #[rstest]
    fn test_event_generation_functions_are_pure() {
        // Test that same inputs produce same output structure
        // (except for the event_id which is provided as input)
        let task_id = TaskId::generate();
        let event_id = EventId::generate_v7();
        let timestamp = Timestamp::now();

        let event1 = create_status_changed_event(
            &task_id,
            TaskStatus::Pending,
            TaskStatus::Completed,
            event_id.clone(),
            timestamp.clone(),
            10,
        );

        let event2 = create_status_changed_event(
            &task_id,
            TaskStatus::Pending,
            TaskStatus::Completed,
            event_id,
            timestamp,
            10,
        );

        // Same inputs should produce same output
        assert_eq!(event1.task_id, event2.task_id);
        assert_eq!(event1.version, event2.version);
        assert_eq!(event1.kind, event2.kind);
    }
}
