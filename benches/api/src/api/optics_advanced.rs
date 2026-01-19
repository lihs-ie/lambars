//! Advanced Optics operations for complex data manipulation.
//!
//! This module demonstrates advanced Optics patterns including:
//!
//! - **Traversal**: Focus on 0 or more elements (batch operations)
//! - **Optional**: Focus on values that may or may not exist
//! - **At**: Key-based access with insertion/deletion for maps
//! - **Filtered**: Conditional focus based on predicates
//!
//! # lambars Features
//!
//! - `VecTraversal`: Traversal for `Vec<A>`
//! - `FilteredTraversal`: Traversal with predicate filtering
//! - `At` trait: `HashMap` key access
//! - `Optional`: Lens + Prism composition
//!
//! # Optics Hierarchy
//!
//! - `Lens`: Focus on exactly 1 element
//! - `Prism`: Focus on 0 or 1 element (sum types)
//! - `Optional`: Focus on 0 or 1 element (Lens + Prism)
//! - `Traversal`: Focus on 0 or more elements
//! - `Fold`: Read-only access to 0 or more elements

use std::collections::HashMap;

use axum::Json;
use axum::extract::{Path, Query, State};
use lambars::lens;
use lambars::optics::at::At;
use lambars::optics::filtered::FilteredTraversal;
use lambars::optics::{Lens, Optional, Traversal, VecTraversal};
use serde::{Deserialize, Serialize};

use super::dto::TaskResponse;
use super::error::{ApiErrorResponse, FieldError};
use super::handlers::AppState;
use crate::domain::task::{Priority, Tag, Task, TaskId, TaskStatus};
use crate::infrastructure::Pagination;

// =============================================================================
// Type Definitions
// =============================================================================

/// Supported fields for batch update operations.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BatchUpdateField {
    Priority,
    Status,
}

/// Action for optional field updates.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OptionalAction {
    Set,
    Clear,
    Modify,
}

/// Filter conditions for tasks.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskFilterCondition {
    pub priority: Option<String>,
    pub status: Option<String>,
    #[serde(default)]
    pub has_description: Option<bool>,
}

/// Update specification for filtered updates.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskUpdateSpec {
    pub priority: Option<String>,
    pub status: Option<String>,
    pub add_tag: Option<String>,
}

/// Path segment for nested access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    Field(String),
    Traverse,
}

// =============================================================================
// DTOs
// =============================================================================

// -----------------------------------------------------------------------------
// PUT /tasks/batch-update-field
// -----------------------------------------------------------------------------

/// Request for batch-update-field endpoint.
#[derive(Debug, Deserialize)]
pub struct BatchUpdateFieldRequest {
    pub field: BatchUpdateField,
    pub value: serde_json::Value,
    pub filter: Option<TaskFilterCondition>,
}

/// Response for batch-update-field endpoint.
#[derive(Debug, Serialize)]
pub struct BatchUpdateFieldResponse {
    pub updated_count: usize,
    pub tasks: Vec<TaskResponse>,
    pub field_updated: String,
}

// -----------------------------------------------------------------------------
// PUT /tasks/{id}/update-optional
// -----------------------------------------------------------------------------

/// Request for update-optional endpoint.
#[derive(Debug, Deserialize)]
pub struct UpdateOptionalRequest {
    pub field: String,
    pub action: OptionalAction,
    pub value: Option<serde_json::Value>,
}

/// Response for update-optional endpoint.
#[derive(Debug, Serialize)]
pub struct UpdateOptionalResponse {
    pub task: TaskResponse,
    pub previous_value: Option<String>,
    pub new_value: Option<String>,
    pub action_performed: String,
}

// -----------------------------------------------------------------------------
// PUT /projects/{id}/metadata/{key}
// -----------------------------------------------------------------------------

/// Request for update-metadata-key endpoint.
#[derive(Debug, Deserialize)]
pub struct UpdateMetadataKeyRequest {
    pub value: Option<serde_json::Value>,
}

/// Response for update-metadata-key endpoint.
#[derive(Debug, Serialize)]
pub struct UpdateMetadataKeyResponse {
    pub project_id: String,
    pub key: String,
    pub previous_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub action: String,
}

// -----------------------------------------------------------------------------
// PUT /tasks/update-filtered
// -----------------------------------------------------------------------------

/// Request for update-filtered endpoint.
#[derive(Debug, Deserialize)]
pub struct UpdateFilteredRequest {
    pub filter: TaskFilterCondition,
    pub update: TaskUpdateSpec,
}

/// Response for update-filtered endpoint.
#[derive(Debug, Serialize)]
pub struct UpdateFilteredResponse {
    /// Number of tasks that matched the filter conditions.
    pub matched_count: usize,
    /// Number of tasks that were actually modified (matched and had changes).
    pub modified_count: usize,
    pub tasks: Vec<TaskResponse>,
}

// -----------------------------------------------------------------------------
// GET /tasks/nested-access
// -----------------------------------------------------------------------------

/// Query parameters for nested-access endpoint.
#[derive(Debug, Deserialize)]
pub struct NestedAccessQuery {
    pub access_path: String,
    pub filter: Option<String>,
}

/// Response for nested-access endpoint.
#[derive(Debug, Serialize)]
pub struct NestedAccessResponse {
    pub values: Vec<serde_json::Value>,
    pub count: usize,
    pub path_used: String,
}

// =============================================================================
// Pure Functions - Batch Update
// =============================================================================

/// Pure: Parses a priority value from JSON.
fn parse_priority(value: &serde_json::Value) -> Result<Priority, String> {
    match value {
        serde_json::Value::String(s) => match s.to_lowercase().as_str() {
            "low" => Ok(Priority::Low),
            "medium" => Ok(Priority::Medium),
            "high" => Ok(Priority::High),
            "critical" => Ok(Priority::Critical),
            _ => Err(format!("Invalid priority: {s}")),
        },
        serde_json::Value::Number(n) => match n.as_i64() {
            Some(1) => Ok(Priority::Low),
            Some(2) => Ok(Priority::Medium),
            Some(3) => Ok(Priority::High),
            Some(4) => Ok(Priority::Critical),
            _ => Err(format!("Invalid priority number: {n}")),
        },
        _ => Err("Priority must be a string or number".to_string()),
    }
}

/// Pure: Parses a status value from JSON.
fn parse_status(value: &serde_json::Value) -> Result<TaskStatus, String> {
    match value {
        serde_json::Value::String(s) => match s.to_lowercase().as_str() {
            "pending" => Ok(TaskStatus::Pending),
            "in_progress" | "inprogress" => Ok(TaskStatus::InProgress),
            "completed" => Ok(TaskStatus::Completed),
            "cancelled" => Ok(TaskStatus::Cancelled),
            _ => Err(format!("Invalid status: {s}")),
        },
        _ => Err("Status must be a string".to_string()),
    }
}

/// Pure: Updates all tasks' priority using `VecTraversal`.
///
/// Demonstrates: `VecTraversal` + `Lens` composition for batch updates.
fn batch_update_priority(tasks: Vec<Task>, new_priority: Priority) -> Vec<Task> {
    let traversal = VecTraversal::<Task>::new();
    let priority_lens = lens!(Task, priority);

    traversal.modify_all(tasks, |task| priority_lens.set(task, new_priority))
}

/// Pure: Updates all tasks' status using `VecTraversal`.
///
/// Demonstrates: `VecTraversal` + `Lens` composition for batch updates.
fn batch_update_status(tasks: Vec<Task>, new_status: TaskStatus) -> Vec<Task> {
    let traversal = VecTraversal::<Task>::new();
    let status_lens = lens!(Task, status);

    traversal.modify_all(tasks, |task| status_lens.set(task, new_status))
}

/// Pure: Applies batch update based on field type.
fn apply_batch_update(
    tasks: Vec<Task>,
    field: &BatchUpdateField,
    value: &serde_json::Value,
) -> Result<Vec<Task>, String> {
    match field {
        BatchUpdateField::Priority => {
            let priority = parse_priority(value)?;
            Ok(batch_update_priority(tasks, priority))
        }
        BatchUpdateField::Status => {
            let status = parse_status(value)?;
            Ok(batch_update_status(tasks, status))
        }
    }
}

// =============================================================================
// Pure Functions - Optional Update
// =============================================================================

/// Pure: Updates a task's description using Lens on Option field.
///
/// Demonstrates: Lens access to Option<A> field with Optional-like semantics.
fn update_task_description(task: Task, action: &OptionalAction, value: Option<String>) -> Task {
    let description_lens = lens!(Task, description);

    match action {
        OptionalAction::Set => description_lens.set(task, value),
        OptionalAction::Clear => description_lens.set(task, None),
        OptionalAction::Modify => {
            description_lens.modify(task, |opt| opt.map(|s| format!("{s} [modified]")))
        }
    }
}

/// Pure: Applies optional field update.
fn apply_optional_update(
    task: Task,
    field: &str,
    action: &OptionalAction,
    value: Option<serde_json::Value>,
) -> Result<(Task, Option<String>, Option<String>), String> {
    match field {
        "description" => {
            let previous = task.description.clone();
            let new_value = value.and_then(|v| v.as_str().map(String::from));
            let updated = update_task_description(task, action, new_value);
            let new_actual = updated.description.clone();
            Ok((updated, previous, new_actual))
        }
        _ => Err(format!("Unsupported optional field: {field}")),
    }
}

// =============================================================================
// Pure Functions - At (HashMap Access)
// =============================================================================

/// Pure: Updates a metadata value at a specific key using At.
///
/// Demonstrates: `At` trait for `HashMap` key access with insert/update/delete.
fn update_metadata_at_key(
    metadata: HashMap<String, serde_json::Value>,
    key: &str,
    value: Option<&serde_json::Value>,
) -> (
    HashMap<String, serde_json::Value>,
    Option<serde_json::Value>,
    String,
) {
    let previous = metadata.get(key).cloned();
    let optional = <HashMap<String, serde_json::Value> as At<String>>::at(key.to_string());

    match (&previous, value) {
        (None, Some(v)) => {
            let updated = optional.set(metadata, v.clone());
            (updated, previous, "created".to_string())
        }
        (Some(_), Some(v)) => {
            let updated = optional.set(metadata, v.clone());
            (updated, previous, "updated".to_string())
        }
        (Some(_), None) => {
            let mut updated = metadata;
            updated.remove(key);
            (updated, previous, "deleted".to_string())
        }
        (None, None) => (metadata, previous, "no_change".to_string()),
    }
}

// =============================================================================
// Pure Functions - Filtered Traversal
// =============================================================================

/// Pure: Converts Priority to canonical string representation.
const fn priority_to_string(priority: Priority) -> &'static str {
    match priority {
        Priority::Low => "low",
        Priority::Medium => "medium",
        Priority::High => "high",
        Priority::Critical => "critical",
    }
}

/// Pure: Converts `TaskStatus` to canonical string representation.
const fn status_to_string(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "pending",
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Completed => "completed",
        TaskStatus::Cancelled => "cancelled",
    }
}

/// Pure: Builds a predicate from filter conditions.
fn build_filter_predicate(filter: TaskFilterCondition) -> impl Fn(&Task) -> bool + Clone {
    move |task: &Task| {
        let priority_match = filter
            .priority
            .as_ref()
            .is_none_or(|p| priority_to_string(task.priority) == p.to_lowercase());

        let status_match = filter
            .status
            .as_ref()
            .is_none_or(|s| status_to_string(task.status) == s.to_lowercase());

        let description_match = filter.has_description.is_none_or(|has| {
            if has {
                task.description.is_some()
            } else {
                task.description.is_none()
            }
        });

        priority_match && status_match && description_match
    }
}

/// Pure: Applies update spec to a single task.
fn apply_update_spec(task: Task, spec: &TaskUpdateSpec) -> Task {
    let mut updated = task;

    if let Some(priority_str) = &spec.priority
        && let Ok(priority) = parse_priority(&serde_json::Value::String(priority_str.clone()))
    {
        let priority_lens = lens!(Task, priority);
        updated = priority_lens.set(updated, priority);
    }

    if let Some(status_str) = &spec.status
        && let Ok(status) = parse_status(&serde_json::Value::String(status_str.clone()))
    {
        let status_lens = lens!(Task, status);
        updated = status_lens.set(updated, status);
    }

    if let Some(tag) = &spec.add_tag {
        let tags_lens = lens!(Task, tags);
        let new_tag = Tag::from(tag.clone());
        updated = tags_lens.modify(updated, |tags| {
            if tags.contains(&new_tag) {
                tags
            } else {
                tags.insert(new_tag.clone())
            }
        });
    }

    updated
}

/// Pure: Checks if a task would be modified by the update spec.
fn would_be_modified(task: &Task, spec: &TaskUpdateSpec) -> bool {
    if let Some(priority_str) = &spec.priority
        && let Ok(priority) = parse_priority(&serde_json::Value::String(priority_str.clone()))
        && task.priority != priority
    {
        return true;
    }

    if let Some(status_str) = &spec.status
        && let Ok(status) = parse_status(&serde_json::Value::String(status_str.clone()))
        && task.status != status
    {
        return true;
    }

    if let Some(tag) = &spec.add_tag {
        let new_tag = Tag::from(tag.clone());
        if !task.tags.contains(&new_tag) {
            return true;
        }
    }

    false
}

/// Pure: Updates tasks matching filter using `FilteredTraversal`.
///
/// Demonstrates: `FilteredTraversal` for conditional batch updates.
/// Returns a tuple of (updated tasks, matched count, actually modified count).
fn update_filtered_tasks(
    tasks: Vec<Task>,
    filter: &TaskFilterCondition,
    update: &TaskUpdateSpec,
) -> (Vec<Task>, usize, usize) {
    let predicate = build_filter_predicate(filter.clone());
    let matched_count = tasks.iter().filter(|t| predicate(t)).count();
    let modified_count = tasks
        .iter()
        .filter(|t| predicate(t) && would_be_modified(t, update))
        .count();

    let filtered_traversal = FilteredTraversal::<Vec<Task>, Task, _>::new(predicate);
    let updated = filtered_traversal.modify_all(tasks, |task| apply_update_spec(task, update));

    (updated, matched_count, modified_count)
}

// =============================================================================
// Pure Functions - Nested Access
// =============================================================================

/// Pure: Parses an access path string into segments.
///
/// Supports paths like: "tasks.*.priority", "subtasks.*.title"
fn parse_access_path(path: &str) -> Vec<PathSegment> {
    path.split('.')
        .map(|segment| {
            if segment == "*" {
                PathSegment::Traverse
            } else {
                PathSegment::Field(segment.to_string())
            }
        })
        .collect()
}

/// Pure: Extracts values from tasks based on access path.
///
/// Demonstrates: `VecTraversal` for extracting values from nested structures.
fn execute_nested_access(tasks: &Vec<Task>, segments: &[PathSegment]) -> Vec<serde_json::Value> {
    if segments.is_empty() {
        return Vec::new();
    }

    let traversal = VecTraversal::<Task>::new();

    match segments.first() {
        Some(PathSegment::Traverse) if segments.len() >= 2 => {
            if let Some(PathSegment::Field(field_name)) = segments.get(1) {
                match field_name.as_str() {
                    "priority" => traversal
                        .get_all(tasks)
                        .map(|task| serde_json::json!(format!("{:?}", task.priority)))
                        .collect(),
                    "status" => traversal
                        .get_all(tasks)
                        .map(|task| serde_json::json!(format!("{:?}", task.status)))
                        .collect(),
                    "title" => traversal
                        .get_all(tasks)
                        .map(|task| serde_json::json!(task.title.clone()))
                        .collect(),
                    "created_at" => traversal
                        .get_all(tasks)
                        .map(|task| serde_json::json!(task.created_at.to_string()))
                        .collect(),
                    "description" => traversal
                        .get_all(tasks)
                        .map(|task| {
                            task.description
                                .as_ref()
                                .map_or(serde_json::Value::Null, |d| serde_json::json!(d.clone()))
                        })
                        .collect(),
                    "tags" => traversal
                        .get_all(tasks)
                        .map(|task| {
                            let tag_strings: Vec<String> =
                                task.tags.iter().map(ToString::to_string).collect();
                            serde_json::json!(tag_strings)
                        })
                        .collect(),
                    _ => Vec::new(),
                }
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Pure: Parses a task ID string into a `TaskId`.
fn parse_task_id(s: &str) -> Result<TaskId, String> {
    uuid::Uuid::parse_str(s)
        .map(TaskId::from_uuid)
        .map_err(|e| format!("Invalid task ID: {e}"))
}

// =============================================================================
// Handlers
// =============================================================================

// -----------------------------------------------------------------------------
// PUT /tasks/batch-update-field
// -----------------------------------------------------------------------------

/// Updates a specific field across all tasks using Traversal.
///
/// This handler demonstrates `VecTraversal` for batch operations.
/// All tasks have the specified field updated to the provided value.
///
/// # lambars Features
///
/// - `VecTraversal<Task>`: Traverses all elements in `Vec<Task>`
/// - `Lens<Task, Priority>`: Focuses on priority field
/// - `Traversal::modify_all`: Applies function to all elements
///
/// # Request Body
///
/// - `field`: Field to update ("priority" | "status")
/// - `value`: New value for the field
/// - `filter`: Optional filter conditions
///
/// # Response
///
/// Returns updated task list with count of modified tasks.
///
/// # Errors
///
/// - `400 Bad Request`: Invalid field or value
#[allow(clippy::cast_possible_truncation)]
pub async fn batch_update_field(
    State(state): State<AppState>,
    Json(request): Json<BatchUpdateFieldRequest>,
) -> Result<Json<BatchUpdateFieldResponse>, ApiErrorResponse> {
    let tasks = state
        .task_repository
        .list(Pagination::new(0, 1000))
        .run_async()
        .await
        .map_err(|_| ApiErrorResponse::internal_error("Repository error"))?
        .items;

    let filtered_tasks = if let Some(filter) = request.filter.clone() {
        let predicate = build_filter_predicate(filter);
        tasks.into_iter().filter(|t| predicate(t)).collect()
    } else {
        tasks
    };

    let updated_tasks = apply_batch_update(filtered_tasks, &request.field, &request.value)
        .map_err(|error| {
            ApiErrorResponse::validation_error(
                "Invalid value",
                vec![FieldError::new("value", error)],
            )
        })?;

    let updated_count = updated_tasks.len();
    let field_name = format!("{:?}", request.field).to_lowercase();

    Ok(Json(BatchUpdateFieldResponse {
        updated_count,
        tasks: updated_tasks.iter().map(TaskResponse::from).collect(),
        field_updated: field_name,
    }))
}

// -----------------------------------------------------------------------------
// PUT /tasks/{id}/update-optional
// -----------------------------------------------------------------------------

/// Updates an optional field on a task.
///
/// This handler demonstrates **Lens on Option<A>** for optional field access.
/// Supports set, clear, and modify operations on fields that may be None.
///
/// # lambars Features
///
/// - `Lens<Task, Option<String>>`: Focuses on optional description field
/// - Lens operations work with Option<A> directly
///
/// # Path Parameters
///
/// - `id`: Task ID
///
/// # Request Body
///
/// - `field`: Field to update ("description")
/// - `action`: Action to perform ("set" | "clear" | "modify")
/// - `value`: Optional new value
///
/// # Response
///
/// Returns updated task with previous and new values.
///
/// # Errors
///
/// - `404 Not Found`: Task not found
/// - `400 Bad Request`: Invalid field or action
pub async fn update_optional(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Json(request): Json<UpdateOptionalRequest>,
) -> Result<Json<UpdateOptionalResponse>, ApiErrorResponse> {
    let task_id = parse_task_id(&task_id).map_err(|error| {
        ApiErrorResponse::validation_error("Invalid task ID", vec![FieldError::new("id", error)])
    })?;

    let task = state
        .task_repository
        .find_by_id(&task_id)
        .run_async()
        .await
        .map_err(|_| ApiErrorResponse::internal_error("Repository error"))?
        .ok_or_else(|| {
            ApiErrorResponse::not_found(format!("Task not found: {}", task_id.as_uuid()))
        })?;

    let (updated_task, previous_value, new_value) = apply_optional_update(
        task,
        &request.field,
        &request.action,
        request.value,
    )
    .map_err(|error| {
        ApiErrorResponse::validation_error("Invalid field", vec![FieldError::new("field", error)])
    })?;

    let action_performed = format!("{:?}", request.action).to_lowercase();

    Ok(Json(UpdateOptionalResponse {
        task: TaskResponse::from(&updated_task),
        previous_value,
        new_value,
        action_performed,
    }))
}

// -----------------------------------------------------------------------------
// PUT /projects/{id}/metadata/{key}
// -----------------------------------------------------------------------------

/// Updates a metadata value at a specific key using At.
///
/// This handler demonstrates **At trait** for `HashMap` key access.
/// Supports creating, updating, and deleting metadata entries.
///
/// # lambars Features
///
/// - `At<String>` trait: Provides Optional for `HashMap` key access
/// - `HashMapAt`: Optional implementation for `HashMap`
/// - Supports insert (set on missing key) and update (set on existing key)
///
/// # Path Parameters
///
/// - `id`: Project ID
/// - `key`: Metadata key
///
/// # Request Body
///
/// - `value`: Optional new value (None to delete)
///
/// # Response
///
/// Returns updated metadata with action performed.
pub async fn update_metadata_key(
    Path((project_id, key)): Path<(String, String)>,
    Json(request): Json<UpdateMetadataKeyRequest>,
) -> Json<UpdateMetadataKeyResponse> {
    let metadata: HashMap<String, serde_json::Value> = HashMap::new();

    let (updated_metadata, previous_value, action) =
        update_metadata_at_key(metadata, &key, request.value.as_ref());

    let new_value = updated_metadata.get(&key).cloned();

    Json(UpdateMetadataKeyResponse {
        project_id,
        key,
        previous_value,
        new_value,
        action,
    })
}

// -----------------------------------------------------------------------------
// PUT /tasks/update-filtered
// -----------------------------------------------------------------------------

/// Updates tasks matching filter conditions using `FilteredTraversal`.
///
/// This handler demonstrates `FilteredTraversal` for conditional updates.
/// Only tasks matching the filter predicate are modified.
///
/// # lambars Features
///
/// - `FilteredTraversal<Vec<Task>, Task, P>`: Traversal with predicate
/// - `modify_all`: Only modifies elements where predicate returns true
/// - Non-matching elements are left unchanged
///
/// # Request Body
///
/// - `filter`: Conditions to match tasks
/// - `update`: Updates to apply to matching tasks
///
/// # Response
///
/// Returns matched count, updated count, and all tasks.
///
/// # Errors
///
/// - `400 Bad Request`: Invalid filter or update
#[allow(clippy::cast_possible_truncation)]
pub async fn update_filtered(
    State(state): State<AppState>,
    Json(request): Json<UpdateFilteredRequest>,
) -> Result<Json<UpdateFilteredResponse>, ApiErrorResponse> {
    let tasks = state
        .task_repository
        .list(Pagination::new(0, 1000))
        .run_async()
        .await
        .map_err(|_| ApiErrorResponse::internal_error("Repository error"))?
        .items;

    let (updated_tasks, matched_count, modified_count) =
        update_filtered_tasks(tasks, &request.filter, &request.update);

    Ok(Json(UpdateFilteredResponse {
        matched_count,
        modified_count,
        tasks: updated_tasks.iter().map(TaskResponse::from).collect(),
    }))
}

// -----------------------------------------------------------------------------
// GET /tasks/nested-access
// -----------------------------------------------------------------------------

/// Accesses nested values using composed Optics.
///
/// This handler demonstrates **Optics composition** for nested access.
/// Uses path notation to traverse nested structures and extract values.
///
/// # lambars Features
///
/// - `VecTraversal`: Traverses collection elements
/// - Path parsing: Converts string path to Optics operations
/// - `get_all`: Extracts values from all elements
///
/// # Query Parameters
///
/// - `access_path`: Path like "*.priority" or "*.deadline"
/// - `filter`: Optional filter expression
///
/// # Response
///
/// Returns extracted values with count and path used.
///
/// # Errors
///
/// - `400 Bad Request`: Invalid access path
pub async fn nested_access(
    State(state): State<AppState>,
    Query(query): Query<NestedAccessQuery>,
) -> Result<Json<NestedAccessResponse>, ApiErrorResponse> {
    let segments = parse_access_path(&query.access_path);

    if segments.is_empty() {
        return Err(ApiErrorResponse::validation_error(
            "Invalid path",
            vec![FieldError::new("access_path", "Path cannot be empty")],
        ));
    }

    let tasks = state
        .task_repository
        .list(Pagination::new(0, 1000))
        .run_async()
        .await
        .map_err(|_| ApiErrorResponse::internal_error("Repository error"))?
        .items;

    let values = execute_nested_access(&tasks, &segments);

    Ok(Json(NestedAccessResponse {
        values: values.clone(),
        count: values.len(),
        path_used: query.access_path,
    }))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::Timestamp;
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // Batch Update Pure Function Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_parse_priority_string() {
        assert_eq!(
            parse_priority(&serde_json::json!("low")).unwrap(),
            Priority::Low
        );
        assert_eq!(
            parse_priority(&serde_json::json!("high")).unwrap(),
            Priority::High
        );
        assert_eq!(
            parse_priority(&serde_json::json!("CRITICAL")).unwrap(),
            Priority::Critical
        );
    }

    #[rstest]
    fn test_parse_priority_number() {
        assert_eq!(
            parse_priority(&serde_json::json!(1)).unwrap(),
            Priority::Low
        );
        assert_eq!(
            parse_priority(&serde_json::json!(4)).unwrap(),
            Priority::Critical
        );
    }

    #[rstest]
    fn test_parse_priority_invalid() {
        assert!(parse_priority(&serde_json::json!("invalid")).is_err());
        assert!(parse_priority(&serde_json::json!(5)).is_err());
        assert!(parse_priority(&serde_json::json!(null)).is_err());
    }

    #[rstest]
    fn test_parse_status() {
        assert_eq!(
            parse_status(&serde_json::json!("pending")).unwrap(),
            TaskStatus::Pending
        );
        assert_eq!(
            parse_status(&serde_json::json!("in_progress")).unwrap(),
            TaskStatus::InProgress
        );
        assert_eq!(
            parse_status(&serde_json::json!("completed")).unwrap(),
            TaskStatus::Completed
        );
    }

    #[rstest]
    fn test_parse_status_invalid() {
        assert!(parse_status(&serde_json::json!("invalid")).is_err());
        assert!(parse_status(&serde_json::json!(123)).is_err());
    }

    #[rstest]
    fn test_batch_update_priority() {
        let tasks = vec![
            Task::new(TaskId::generate(), "Task 1".to_string(), Timestamp::now()),
            Task::new(TaskId::generate(), "Task 2".to_string(), Timestamp::now()),
        ];

        let updated = batch_update_priority(tasks, Priority::Critical);

        assert!(updated.iter().all(|t| t.priority == Priority::Critical));
    }

    #[rstest]
    fn test_batch_update_status() {
        let tasks = vec![
            Task::new(TaskId::generate(), "Task 1".to_string(), Timestamp::now()),
            Task::new(TaskId::generate(), "Task 2".to_string(), Timestamp::now()),
        ];

        let updated = batch_update_status(tasks, TaskStatus::Completed);

        assert!(updated.iter().all(|t| t.status == TaskStatus::Completed));
    }

    #[rstest]
    fn test_apply_batch_update() {
        let tasks = vec![Task::new(
            TaskId::generate(),
            "Task".to_string(),
            Timestamp::now(),
        )];

        let result = apply_batch_update(
            tasks.clone(),
            &BatchUpdateField::Priority,
            &serde_json::json!("high"),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].priority, Priority::High);

        let result = apply_batch_update(
            tasks,
            &BatchUpdateField::Status,
            &serde_json::json!("completed"),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].status, TaskStatus::Completed);
    }

    // -------------------------------------------------------------------------
    // Optional Update Pure Function Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_update_task_description_set() {
        let task = Task::new(TaskId::generate(), "Task".to_string(), Timestamp::now());

        let updated = update_task_description(
            task,
            &OptionalAction::Set,
            Some("New description".to_string()),
        );

        assert_eq!(updated.description, Some("New description".to_string()));
    }

    #[rstest]
    fn test_update_task_description_clear() {
        let task = Task::new(TaskId::generate(), "Task".to_string(), Timestamp::now())
            .with_description("Existing".to_string());

        let updated = update_task_description(task, &OptionalAction::Clear, None);

        assert_eq!(updated.description, None);
    }

    #[rstest]
    fn test_update_task_description_modify() {
        let task = Task::new(TaskId::generate(), "Task".to_string(), Timestamp::now())
            .with_description("Original".to_string());

        let updated = update_task_description(task, &OptionalAction::Modify, None);

        assert_eq!(updated.description, Some("Original [modified]".to_string()));
    }

    #[rstest]
    fn test_apply_optional_update() {
        let task = Task::new(TaskId::generate(), "Task".to_string(), Timestamp::now());

        let result = apply_optional_update(
            task,
            "description",
            &OptionalAction::Set,
            Some(serde_json::json!("New desc")),
        );

        assert!(result.is_ok());
        let (updated, previous, new_value) = result.unwrap();
        assert_eq!(previous, None);
        assert_eq!(new_value, Some("New desc".to_string()));
        assert_eq!(updated.description, Some("New desc".to_string()));
    }

    #[rstest]
    fn test_apply_optional_update_invalid_field() {
        let task = Task::new(TaskId::generate(), "Task".to_string(), Timestamp::now());

        let result = apply_optional_update(task, "invalid_field", &OptionalAction::Set, None);

        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // At (HashMap Access) Pure Function Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_update_metadata_at_key_create() {
        let metadata: HashMap<String, serde_json::Value> = HashMap::new();
        let value = serde_json::json!("value");

        let (updated, previous, action) = update_metadata_at_key(metadata, "new_key", Some(&value));

        assert!(previous.is_none());
        assert_eq!(action, "created");
        assert_eq!(updated.get("new_key"), Some(&serde_json::json!("value")));
    }

    #[rstest]
    fn test_update_metadata_at_key_update() {
        let mut metadata: HashMap<String, serde_json::Value> = HashMap::new();
        metadata.insert("key".to_string(), serde_json::json!("old"));
        let new_value = serde_json::json!("new");

        let (updated, previous, action) = update_metadata_at_key(metadata, "key", Some(&new_value));

        assert_eq!(previous, Some(serde_json::json!("old")));
        assert_eq!(action, "updated");
        assert_eq!(updated.get("key"), Some(&serde_json::json!("new")));
    }

    #[rstest]
    fn test_update_metadata_at_key_delete() {
        let mut metadata: HashMap<String, serde_json::Value> = HashMap::new();
        metadata.insert("key".to_string(), serde_json::json!("value"));

        let (updated, previous, action) = update_metadata_at_key(metadata, "key", None);

        assert_eq!(previous, Some(serde_json::json!("value")));
        assert_eq!(action, "deleted");
        assert!(!updated.contains_key("key"));
    }

    #[rstest]
    fn test_update_metadata_at_key_no_change() {
        let metadata: HashMap<String, serde_json::Value> = HashMap::new();

        let (updated, previous, action) = update_metadata_at_key(metadata, "missing", None);

        assert!(previous.is_none());
        assert_eq!(action, "no_change");
        assert!(updated.is_empty());
    }

    // -------------------------------------------------------------------------
    // Filtered Traversal Pure Function Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_build_filter_predicate_priority() {
        let filter = TaskFilterCondition {
            priority: Some("high".to_string()),
            status: None,
            has_description: None,
        };

        let predicate = build_filter_predicate(filter);

        let high_task = Task::new(TaskId::generate(), "Task".to_string(), Timestamp::now())
            .with_priority(Priority::High);
        let low_task = Task::new(TaskId::generate(), "Task".to_string(), Timestamp::now())
            .with_priority(Priority::Low);

        assert!(predicate(&high_task));
        assert!(!predicate(&low_task));
    }

    #[rstest]
    fn test_build_filter_predicate_status() {
        let filter = TaskFilterCondition {
            priority: None,
            status: Some("completed".to_string()),
            has_description: None,
        };

        let predicate = build_filter_predicate(filter);

        let completed = Task::new(TaskId::generate(), "Task".to_string(), Timestamp::now())
            .with_status(TaskStatus::Completed);
        let pending = Task::new(TaskId::generate(), "Task".to_string(), Timestamp::now())
            .with_status(TaskStatus::Pending);

        assert!(predicate(&completed));
        assert!(!predicate(&pending));
    }

    #[rstest]
    fn test_build_filter_predicate_combined() {
        let filter = TaskFilterCondition {
            priority: Some("high".to_string()),
            status: Some("pending".to_string()),
            has_description: None,
        };

        let predicate = build_filter_predicate(filter);

        let matching = Task::new(TaskId::generate(), "Task".to_string(), Timestamp::now())
            .with_priority(Priority::High)
            .with_status(TaskStatus::Pending);
        let not_matching = Task::new(TaskId::generate(), "Task".to_string(), Timestamp::now())
            .with_priority(Priority::High)
            .with_status(TaskStatus::Completed);

        assert!(predicate(&matching));
        assert!(!predicate(&not_matching));
    }

    #[rstest]
    fn test_apply_update_spec() {
        let task = Task::new(TaskId::generate(), "Task".to_string(), Timestamp::now());

        let spec = TaskUpdateSpec {
            priority: Some("critical".to_string()),
            status: Some("in_progress".to_string()),
            add_tag: Some("urgent".to_string()),
        };

        let updated = apply_update_spec(task, &spec);

        assert_eq!(updated.priority, Priority::Critical);
        assert_eq!(updated.status, TaskStatus::InProgress);
        assert!(updated.tags.contains(&Tag::from("urgent")));
    }

    #[rstest]
    fn test_update_filtered_tasks() {
        let tasks = vec![
            Task::new(TaskId::generate(), "Task 1".to_string(), Timestamp::now())
                .with_priority(Priority::High),
            Task::new(TaskId::generate(), "Task 2".to_string(), Timestamp::now())
                .with_priority(Priority::Low),
            Task::new(TaskId::generate(), "Task 3".to_string(), Timestamp::now())
                .with_priority(Priority::High),
        ];

        let filter = TaskFilterCondition {
            priority: Some("high".to_string()),
            status: None,
            has_description: None,
        };

        let update = TaskUpdateSpec {
            priority: None,
            status: Some("completed".to_string()),
            add_tag: None,
        };

        let (updated, matched_count, modified_count) =
            update_filtered_tasks(tasks, &filter, &update);

        assert_eq!(matched_count, 2);
        assert_eq!(modified_count, 2); // Both high-priority tasks were Pending, now Completed
        assert_eq!(updated[0].status, TaskStatus::Completed);
        assert_eq!(updated[1].status, TaskStatus::Pending);
        assert_eq!(updated[2].status, TaskStatus::Completed);
    }

    // -------------------------------------------------------------------------
    // Nested Access Pure Function Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_parse_access_path() {
        let segments = parse_access_path("*.priority");

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], PathSegment::Traverse);
        assert_eq!(segments[1], PathSegment::Field("priority".to_string()));
    }

    #[rstest]
    fn test_parse_access_path_complex() {
        let segments = parse_access_path("tasks.*.deadline");

        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], PathSegment::Field("tasks".to_string()));
        assert_eq!(segments[1], PathSegment::Traverse);
        assert_eq!(segments[2], PathSegment::Field("deadline".to_string()));
    }

    #[rstest]
    fn test_execute_nested_access_priority() {
        let tasks = vec![
            Task::new(TaskId::generate(), "Task 1".to_string(), Timestamp::now())
                .with_priority(Priority::High),
            Task::new(TaskId::generate(), "Task 2".to_string(), Timestamp::now())
                .with_priority(Priority::Low),
        ];

        let segments = parse_access_path("*.priority");
        let values = execute_nested_access(&tasks, &segments);

        assert_eq!(values.len(), 2);
        assert_eq!(values[0], serde_json::json!("High"));
        assert_eq!(values[1], serde_json::json!("Low"));
    }

    #[rstest]
    fn test_execute_nested_access_title() {
        let tasks = vec![
            Task::new(TaskId::generate(), "Task A".to_string(), Timestamp::now()),
            Task::new(TaskId::generate(), "Task B".to_string(), Timestamp::now()),
        ];

        let segments = parse_access_path("*.title");
        let values = execute_nested_access(&tasks, &segments);

        assert_eq!(values.len(), 2);
        assert_eq!(values[0], serde_json::json!("Task A"));
        assert_eq!(values[1], serde_json::json!("Task B"));
    }

    #[rstest]
    fn test_execute_nested_access_empty() {
        let tasks: Vec<Task> = vec![];
        let segments = parse_access_path("*.priority");

        let values = execute_nested_access(&tasks, &segments);

        assert!(values.is_empty());
    }

    // -------------------------------------------------------------------------
    // Traversal Law Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_traversal_identity_law() {
        let tasks = vec![
            Task::new(TaskId::generate(), "Task 1".to_string(), Timestamp::now()),
            Task::new(TaskId::generate(), "Task 2".to_string(), Timestamp::now()),
        ];

        let traversal = VecTraversal::<Task>::new();
        let result = traversal.modify_all(tasks.clone(), |t| t);

        assert_eq!(result.len(), tasks.len());
        for (original, modified) in tasks.iter().zip(result.iter()) {
            assert_eq!(original.task_id, modified.task_id);
            assert_eq!(original.title, modified.title);
        }
    }

    #[rstest]
    fn test_traversal_composition_law() {
        let tasks = vec![
            Task::new(TaskId::generate(), "Task".to_string(), Timestamp::now())
                .with_priority(Priority::Low),
        ];

        let traversal = VecTraversal::<Task>::new();
        let priority_lens = lens!(Task, priority);

        let function_f = |t: Task| priority_lens.set(t, Priority::Medium);
        let function_g = |t: Task| priority_lens.set(t, Priority::High);

        let sequential =
            traversal.modify_all(traversal.modify_all(tasks.clone(), function_f), function_g);
        let composed = traversal.modify_all(tasks, |t| function_g(function_f(t)));

        assert_eq!(sequential[0].priority, composed[0].priority);
    }

    #[rstest]
    fn test_filtered_traversal_only_modifies_matching() {
        let tasks = vec![
            Task::new(TaskId::generate(), "Task 1".to_string(), Timestamp::now())
                .with_priority(Priority::High),
            Task::new(TaskId::generate(), "Task 2".to_string(), Timestamp::now())
                .with_priority(Priority::Low),
        ];

        let filtered =
            FilteredTraversal::<Vec<Task>, Task, _>::new(|t: &Task| t.priority == Priority::High);
        let status_lens = lens!(Task, status);

        let updated = filtered.modify_all(tasks, |t| status_lens.set(t, TaskStatus::Completed));

        assert_eq!(updated[0].status, TaskStatus::Completed);
        assert_eq!(updated[1].status, TaskStatus::Pending);
    }

    // -------------------------------------------------------------------------
    // At Law Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_at_get_set_law() {
        let mut map: HashMap<String, i32> = HashMap::new();
        map.insert("key".to_string(), 42);

        let optional = <HashMap<String, i32> as At<String>>::at("key".to_string());

        if let Some(&value) = optional.get_option(&map) {
            let reconstructed = optional.set(map.clone(), value);
            assert_eq!(reconstructed.get("key"), map.get("key"));
        }
    }

    #[rstest]
    fn test_at_set_get_law() {
        let map: HashMap<String, i32> = HashMap::new();
        let optional = <HashMap<String, i32> as At<String>>::at("key".to_string());

        let updated = optional.set(map, 42);
        assert_eq!(optional.get_option(&updated), Some(&42));
    }

    #[rstest]
    fn test_at_set_set_law() {
        let map: HashMap<String, i32> = HashMap::new();
        let optional = <HashMap<String, i32> as At<String>>::at("key".to_string());

        let set_twice = optional.set(optional.set(map.clone(), 42), 100);
        let set_once = optional.set(map, 100);

        assert_eq!(set_twice.get("key"), set_once.get("key"));
    }

    // -------------------------------------------------------------------------
    // Lens Law Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_lens_get_put_law() {
        // GetPut: lens.set(source, lens.get(&source).clone()) == source
        let task = Task::new(TaskId::generate(), "Task".to_string(), Timestamp::now())
            .with_priority(Priority::High);

        let priority_lens = lens!(Task, priority);
        let value = *priority_lens.get(&task);
        let reconstructed = priority_lens.set(task.clone(), value);

        assert_eq!(reconstructed.priority, task.priority);
    }

    #[rstest]
    fn test_lens_put_get_law() {
        // PutGet: lens.get(&lens.set(source, value)) == &value
        let task = Task::new(TaskId::generate(), "Task".to_string(), Timestamp::now());

        let priority_lens = lens!(Task, priority);
        let updated = priority_lens.set(task, Priority::Critical);

        assert_eq!(*priority_lens.get(&updated), Priority::Critical);
    }

    #[rstest]
    fn test_lens_put_put_law() {
        // PutPut: lens.set(lens.set(source, v1), v2) == lens.set(source, v2)
        let task = Task::new(TaskId::generate(), "Task".to_string(), Timestamp::now());

        let priority_lens = lens!(Task, priority);
        let set_twice = priority_lens.set(
            priority_lens.set(task.clone(), Priority::Medium),
            Priority::High,
        );
        let set_once = priority_lens.set(task, Priority::High);

        assert_eq!(set_twice.priority, set_once.priority);
    }

    // -------------------------------------------------------------------------
    // FilteredTraversal Law Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_filtered_traversal_identity_law() {
        // Identity: modify_all(source, id) == source
        let tasks = vec![
            Task::new(TaskId::generate(), "Task 1".to_string(), Timestamp::now())
                .with_priority(Priority::High),
            Task::new(TaskId::generate(), "Task 2".to_string(), Timestamp::now())
                .with_priority(Priority::Low),
        ];

        let filtered =
            FilteredTraversal::<Vec<Task>, Task, _>::new(|t: &Task| t.priority == Priority::High);
        let result = filtered.modify_all(tasks.clone(), |t| t);

        // High priority tasks should be unchanged
        assert_eq!(result[0].priority, tasks[0].priority);
        assert_eq!(result[0].status, tasks[0].status);
        // Low priority tasks should also be unchanged
        assert_eq!(result[1].priority, tasks[1].priority);
    }

    #[rstest]
    fn test_filtered_traversal_composition_law() {
        // Composition: modify_all(modify_all(s, f), g) == modify_all(s, g . f) for matching elements
        let tasks = vec![
            Task::new(TaskId::generate(), "Task".to_string(), Timestamp::now())
                .with_priority(Priority::High),
        ];

        let filtered =
            FilteredTraversal::<Vec<Task>, Task, _>::new(|t: &Task| t.priority == Priority::High);
        let status_lens = lens!(Task, status);

        let function_f = |t: Task| status_lens.set(t, TaskStatus::InProgress);
        let function_g = |t: Task| status_lens.set(t, TaskStatus::Completed);

        let sequential =
            filtered.modify_all(filtered.modify_all(tasks.clone(), function_f), function_g);
        let composed = filtered.modify_all(tasks, |t| function_g(function_f(t)));

        assert_eq!(sequential[0].status, composed[0].status);
    }
}
