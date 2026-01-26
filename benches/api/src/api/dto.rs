//! Data Transfer Objects for API requests and responses.
//!
//! This module contains DTOs that are separate from domain models,
//! providing a clean API contract.

use serde::{Deserialize, Serialize};

use crate::domain::{Priority, SubTask, Tag, Task, TaskStatus};

// =============================================================================
// Task DTOs
// =============================================================================

/// Request DTO for creating a new task.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTaskRequest {
    /// Title of the task.
    pub title: String,
    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,
    /// Priority level (defaults to Low).
    #[serde(default)]
    pub priority: PriorityDto,
    /// Tags to add to the task.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Request DTO for updating a task.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateTaskRequest {
    /// New title for the task.
    #[serde(default)]
    pub title: Option<String>,
    /// New description for the task.
    #[serde(default)]
    pub description: Option<String>,
    /// New status for the task.
    #[serde(default)]
    pub status: Option<TaskStatusDto>,
    /// New priority for the task.
    #[serde(default)]
    pub priority: Option<PriorityDto>,
    /// Expected version for optimistic locking.
    pub version: u64,
}

/// Response DTO for a task.
#[derive(Debug, Clone, Serialize)]
pub struct TaskResponse {
    /// Task ID.
    pub id: String,
    /// Title of the task.
    pub title: String,
    /// Description of the task.
    pub description: Option<String>,
    /// Current status.
    pub status: TaskStatusDto,
    /// Priority level.
    pub priority: PriorityDto,
    /// Tags on the task.
    pub tags: Vec<String>,
    /// Subtasks.
    pub subtasks: Vec<SubTaskResponse>,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
    /// Current version.
    pub version: u64,
    /// Warnings (e.g., consistency issues) - only included if present.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

impl From<&Task> for TaskResponse {
    fn from(task: &Task) -> Self {
        Self {
            id: task.task_id.to_string(),
            title: task.title.clone(),
            description: task.description.clone(),
            status: TaskStatusDto::from(task.status),
            priority: PriorityDto::from(task.priority),
            tags: task.tags.iter().map(|t| t.as_str().to_string()).collect(),
            subtasks: task.subtasks.iter().map(SubTaskResponse::from).collect(),
            created_at: task.created_at.to_string(),
            updated_at: task.updated_at.to_string(),
            version: task.version,
            warnings: Vec::new(),
        }
    }
}

impl From<Task> for TaskResponse {
    fn from(task: Task) -> Self {
        Self::from(&task)
    }
}

/// Response DTO for a subtask.
#[derive(Debug, Clone, Serialize)]
pub struct SubTaskResponse {
    /// Subtask ID.
    pub id: String,
    /// Title of the subtask.
    pub title: String,
    /// Whether the subtask is completed.
    pub completed: bool,
}

impl From<&SubTask> for SubTaskResponse {
    fn from(subtask: &SubTask) -> Self {
        Self {
            id: subtask.subtask_id.to_string(),
            title: subtask.title.clone(),
            completed: subtask.completed,
        }
    }
}

// =============================================================================
// Enum DTOs
// =============================================================================

/// DTO for task status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatusDto {
    #[default]
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

impl From<TaskStatus> for TaskStatusDto {
    fn from(status: TaskStatus) -> Self {
        match status {
            TaskStatus::Pending => Self::Pending,
            TaskStatus::InProgress => Self::InProgress,
            TaskStatus::Completed => Self::Completed,
            TaskStatus::Cancelled => Self::Cancelled,
        }
    }
}

impl From<TaskStatusDto> for TaskStatus {
    fn from(dto: TaskStatusDto) -> Self {
        match dto {
            TaskStatusDto::Pending => Self::Pending,
            TaskStatusDto::InProgress => Self::InProgress,
            TaskStatusDto::Completed => Self::Completed,
            TaskStatusDto::Cancelled => Self::Cancelled,
        }
    }
}

/// DTO for priority level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PriorityDto {
    #[default]
    Low,
    Medium,
    High,
    Critical,
}

impl From<Priority> for PriorityDto {
    fn from(priority: Priority) -> Self {
        match priority {
            Priority::Low => Self::Low,
            Priority::Medium => Self::Medium,
            Priority::High => Self::High,
            Priority::Critical => Self::Critical,
        }
    }
}

impl From<PriorityDto> for Priority {
    fn from(dto: PriorityDto) -> Self {
        match dto {
            PriorityDto::Low => Self::Low,
            PriorityDto::Medium => Self::Medium,
            PriorityDto::High => Self::High,
            PriorityDto::Critical => Self::Critical,
        }
    }
}

// =============================================================================
// Validation
// =============================================================================

use lambars::control::Either;

use super::error::{FieldError, ValidationError};

/// Validates a task title.
///
/// Returns `Either::Right(title)` if valid, or `Either::Left(ValidationError)` if invalid.
///
/// # Validation Rules
///
/// - Title must not be empty
/// - Title must not exceed 200 characters
#[must_use]
pub fn validate_title(title: &str) -> Either<ValidationError, String> {
    let title = title.trim();

    if title.is_empty() {
        return Either::Left(ValidationError::single("title", "Title is required"));
    }

    if title.len() > 200 {
        return Either::Left(ValidationError::single(
            "title",
            "Title must not exceed 200 characters",
        ));
    }

    Either::Right(title.to_string())
}

/// Validates a task description.
///
/// Returns `Either::Right(description)` if valid, or `Either::Left(ValidationError)` if invalid.
///
/// # Validation Rules
///
/// - Description must not exceed 5000 characters
#[must_use]
pub fn validate_description(description: Option<&str>) -> Either<ValidationError, Option<String>> {
    description.map_or(Either::Right(None), |desc| {
        let desc = desc.trim();
        if desc.is_empty() {
            Either::Right(None)
        } else if desc.len() > 5000 {
            Either::Left(ValidationError::single(
                "description",
                "Description must not exceed 5000 characters",
            ))
        } else {
            Either::Right(Some(desc.to_string()))
        }
    })
}

/// Validates a list of tags.
///
/// Returns `Either::Right(tags)` if valid, or `Either::Left(ValidationError)` if invalid.
///
/// # Validation Rules
///
/// - Each tag must not be empty
/// - Each tag must not exceed 50 characters
/// - Maximum 20 tags allowed
#[must_use]
pub fn validate_tags(tags: &[String]) -> Either<ValidationError, Vec<Tag>> {
    if tags.len() > 20 {
        return Either::Left(ValidationError::single("tags", "Maximum 20 tags allowed"));
    }

    // Validate each tag and partition into valid tags and errors
    let (validated_tags, errors): (Vec<_>, Vec<_>) = tags
        .iter()
        .enumerate()
        .map(|(index, tag)| {
            let tag = tag.trim();
            if tag.is_empty() {
                Err(FieldError::new(
                    format!("tags[{index}]"),
                    "Tag must not be empty",
                ))
            } else if tag.len() > 50 {
                Err(FieldError::new(
                    format!("tags[{index}]"),
                    "Tag must not exceed 50 characters",
                ))
            } else {
                Ok(Tag::new(tag))
            }
        })
        .partition(Result::is_ok);

    if errors.is_empty() {
        Either::Right(validated_tags.into_iter().filter_map(Result::ok).collect())
    } else {
        Either::Left(ValidationError::new(
            errors.into_iter().filter_map(Result::err).collect(),
        ))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    use crate::domain::{SubTaskId, TaskId, Timestamp};

    // -------------------------------------------------------------------------
    // TaskResponse Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_task_response_from_task() {
        let task_id = TaskId::generate();
        let timestamp = Timestamp::now();
        let task = Task::new(task_id, "Test Task", timestamp)
            .with_description("Test description")
            .with_priority(Priority::High)
            .add_tag(Tag::new("backend"));

        let response = TaskResponse::from(&task);

        assert_eq!(response.title, "Test Task");
        assert_eq!(response.description, Some("Test description".to_string()));
        assert_eq!(response.priority, PriorityDto::High);
        assert_eq!(response.tags.len(), 1);
        assert_eq!(response.version, 1);
    }

    #[rstest]
    fn test_subtask_response_from_subtask() {
        let subtask_id = SubTaskId::generate();
        let subtask = SubTask::new(subtask_id, "Test Subtask").complete();

        let response = SubTaskResponse::from(&subtask);

        assert_eq!(response.title, "Test Subtask");
        assert!(response.completed);
    }

    // -------------------------------------------------------------------------
    // Status/Priority DTO Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_task_status_dto_conversion() {
        assert_eq!(
            TaskStatus::from(TaskStatusDto::Pending),
            TaskStatus::Pending
        );
        assert_eq!(
            TaskStatus::from(TaskStatusDto::InProgress),
            TaskStatus::InProgress
        );
        assert_eq!(
            TaskStatus::from(TaskStatusDto::Completed),
            TaskStatus::Completed
        );
        assert_eq!(
            TaskStatus::from(TaskStatusDto::Cancelled),
            TaskStatus::Cancelled
        );
    }

    #[rstest]
    fn test_priority_dto_conversion() {
        assert_eq!(Priority::from(PriorityDto::Low), Priority::Low);
        assert_eq!(Priority::from(PriorityDto::Medium), Priority::Medium);
        assert_eq!(Priority::from(PriorityDto::High), Priority::High);
        assert_eq!(Priority::from(PriorityDto::Critical), Priority::Critical);
    }

    // -------------------------------------------------------------------------
    // Validation Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_validate_title_valid() {
        let result = validate_title("Valid Title");
        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), "Valid Title");
    }

    #[rstest]
    fn test_validate_title_trims_whitespace() {
        let result = validate_title("  Trimmed Title  ");
        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), "Trimmed Title");
    }

    #[rstest]
    fn test_validate_title_empty() {
        let result = validate_title("");
        assert!(result.is_left());
    }

    #[rstest]
    fn test_validate_title_whitespace_only() {
        let result = validate_title("   ");
        assert!(result.is_left());
    }

    #[rstest]
    fn test_validate_title_too_long() {
        let long_title = "a".repeat(201);
        let result = validate_title(&long_title);
        assert!(result.is_left());
    }

    #[rstest]
    fn test_validate_description_valid() {
        let result = validate_description(Some("Valid description"));
        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), Some("Valid description".to_string()));
    }

    #[rstest]
    fn test_validate_description_none() {
        let result = validate_description(None);
        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), None);
    }

    #[rstest]
    fn test_validate_description_empty() {
        let result = validate_description(Some(""));
        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), None);
    }

    #[rstest]
    fn test_validate_description_too_long() {
        let long_desc = "a".repeat(5001);
        let result = validate_description(Some(&long_desc));
        assert!(result.is_left());
    }

    #[rstest]
    fn test_validate_tags_valid() {
        let tags = vec!["backend".to_string(), "urgent".to_string()];
        let result = validate_tags(&tags);
        assert!(result.is_right());
        assert_eq!(result.unwrap_right().len(), 2);
    }

    #[rstest]
    fn test_validate_tags_empty_list() {
        let tags: Vec<String> = vec![];
        let result = validate_tags(&tags);
        assert!(result.is_right());
        assert!(result.unwrap_right().is_empty());
    }

    #[rstest]
    fn test_validate_tags_too_many() {
        let tags: Vec<String> = (0..21).map(|i| format!("tag{i}")).collect();
        let result = validate_tags(&tags);
        assert!(result.is_left());
    }

    #[rstest]
    fn test_validate_tags_empty_tag() {
        let tags = vec!["valid".to_string(), String::new()];
        let result = validate_tags(&tags);
        assert!(result.is_left());
    }

    #[rstest]
    fn test_validate_tags_tag_too_long() {
        let tags = vec!["valid".to_string(), "a".repeat(51)];
        let result = validate_tags(&tags);
        assert!(result.is_left());
    }
}
