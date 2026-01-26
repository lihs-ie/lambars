//! Domain module for task management.
//!
//! This module contains domain models, value objects, and domain services.

pub mod history;
pub mod project;
pub mod task;

pub use history::{
    EventId, PriorityChanged, ProjectAssigned, ProjectRemoved, StatusChanged, SubTaskAdded,
    SubTaskCompleted, TagAdded, TagRemoved, TaskCreated, TaskDescriptionUpdated, TaskEvent,
    TaskEventKind, TaskHistory, TaskHistoryExt, TaskTitleUpdated, combine_histories,
    create_description_updated_event, create_priority_changed_event, create_status_changed_event,
    create_tag_added_event, create_task_created_event, create_title_updated_event,
    load_history_continuation,
};
pub use project::{Project, ProjectId, TaskSummary};
pub use task::{Priority, SubTask, SubTaskId, Tag, Task, TaskId, TaskStatus, Timestamp};
