//! Advanced feature handlers demonstrating lambars functional programming patterns.
//!
//! This module contains HTTP handlers showcasing advanced lambars features:
//!
//! - **Continuation**: CPS-based pagination for task history
//! - **`PersistentList`**: Efficient prepend for event history
//! - **`compose!/pipe!`**: Function composition for task transformations
//! - **`pipe_async!/for_async!`**: Async pipeline processing
//! - **Lazy**: Deferred computation patterns
//!
//! # Handlers
//!
//! - `GET /tasks/{id}/history`: Task event history with CPS pagination
//! - `POST /tasks/transform`: Apply transformation pipeline
//! - `POST /tasks/async-pipeline`: Async processing pipeline
//! - `POST /tasks/lazy-compute`: Deferred computation demo

use axum::{
    Json,
    extract::{Path, Query, State},
};

use super::json_buffer::JsonResponse;
use lambars::control::{Continuation, Lazy};
use lambars::effect::AsyncIO;
use lambars::for_async;
use lambars::{compose, pipe};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::dto::{PriorityDto, TaskResponse, TaskStatusDto};
use super::error::ApiErrorResponse;
use super::handlers::AppState;
use crate::domain::{Priority, Tag, Task, TaskEvent as DomainTaskEvent, TaskEventKind, TaskId};

// =============================================================================
// Constants
// =============================================================================

/// Maximum number of events per page for history endpoint.
const MAX_HISTORY_LIMIT: usize = 100;

/// Default number of events per page.
const DEFAULT_HISTORY_LIMIT: usize = 20;

/// Maximum batch size for async pipeline.
const MAX_PIPELINE_BATCH_SIZE: usize = 50;

// =============================================================================
// Task Event Types (for History)
// =============================================================================

/// Represents a task event in the history.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum TaskEvent {
    /// Task was created.
    Created { timestamp: String, title: String },
    /// Task title was updated.
    TitleUpdated {
        timestamp: String,
        old_title: String,
        new_title: String,
    },
    /// Task status was changed.
    StatusChanged {
        timestamp: String,
        old_status: TaskStatusDto,
        new_status: TaskStatusDto,
    },
    /// Task priority was changed.
    PriorityChanged {
        timestamp: String,
        old_priority: PriorityDto,
        new_priority: PriorityDto,
    },
    /// Tag was added to task.
    TagAdded { timestamp: String, tag: String },
}

impl TaskEvent {
    /// Converts a domain `TaskEvent` to an API `TaskEvent`.
    ///
    /// This function maps the domain event types to API DTOs for HTTP response.
    fn from_domain(event: &DomainTaskEvent) -> Option<Self> {
        let timestamp = event.timestamp.to_string();

        match &event.kind {
            TaskEventKind::Created(payload) => Some(Self::Created {
                timestamp,
                title: payload.title.clone(),
            }),
            TaskEventKind::TitleUpdated(payload) => Some(Self::TitleUpdated {
                timestamp,
                old_title: payload.old_title.clone(),
                new_title: payload.new_title.clone(),
            }),
            TaskEventKind::StatusChanged(payload) => Some(Self::StatusChanged {
                timestamp,
                old_status: TaskStatusDto::from(payload.old_status),
                new_status: TaskStatusDto::from(payload.new_status),
            }),
            TaskEventKind::PriorityChanged(payload) => Some(Self::PriorityChanged {
                timestamp,
                old_priority: PriorityDto::from(payload.old_priority),
                new_priority: PriorityDto::from(payload.new_priority),
            }),
            TaskEventKind::TagAdded(payload) => Some(Self::TagAdded {
                timestamp,
                tag: payload.tag.as_str().to_string(),
            }),
            // These event types are not exposed in the API response
            TaskEventKind::DescriptionUpdated(_)
            | TaskEventKind::TagRemoved(_)
            | TaskEventKind::SubTaskAdded(_)
            | TaskEventKind::SubTaskCompleted(_)
            | TaskEventKind::ProjectAssigned(_)
            | TaskEventKind::ProjectRemoved(_) => None,
        }
    }
}

// =============================================================================
// History DTOs
// =============================================================================

/// Query parameters for history endpoint.
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    /// Number of events to return (default: 20, max: 100).
    #[serde(default)]
    pub limit: Option<usize>,
    /// Cursor for pagination (event offset).
    #[serde(default)]
    pub cursor: Option<usize>,
}

/// Response for task history.
#[derive(Debug, Serialize)]
pub struct TaskHistoryResponse {
    /// Task ID.
    pub task_id: String,
    /// List of events.
    pub events: Vec<TaskEvent>,
    /// Next cursor for pagination (None if no more events).
    pub next_cursor: Option<usize>,
    /// Whether there are more events.
    pub has_more: bool,
    /// Total number of events.
    pub total_events: usize,
}

/// Internal paginated result type.
#[derive(Debug, Clone)]
struct PaginatedResult<T> {
    items: Vec<T>,
    next_cursor: Option<usize>,
    has_more: bool,
    total: usize,
}

// =============================================================================
// Transform DTOs
// =============================================================================

/// Request for task transformation.
#[derive(Debug, Deserialize)]
pub struct TransformRequest {
    /// Task ID to transform.
    pub task_id: String,
    /// List of transformations to apply (in order).
    pub transformations: Vec<String>,
}

/// Response for transformation result.
#[derive(Debug, Serialize)]
pub struct TransformResponse {
    /// Whether transformation was successful.
    pub success: bool,
    /// The transformed task (if successful).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<TaskResponse>,
    /// List of applied transformations.
    pub applied_transformations: Vec<String>,
    /// Error message (if failed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// =============================================================================
// Async Pipeline DTOs
// =============================================================================

/// Request for async pipeline processing.
#[derive(Debug, Deserialize)]
pub struct AsyncPipelineRequest {
    /// Task IDs to process.
    pub task_ids: Vec<String>,
}

/// Result for a single task in the pipeline.
#[derive(Debug, Clone, Serialize)]
pub struct PipelineTaskResult {
    /// Task ID.
    pub task_id: String,
    /// Whether processing was successful.
    pub success: bool,
    /// Validation score (0-100).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_score: Option<u32>,
    /// Enrichment data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enrichment: Option<TaskEnrichment>,
    /// Error message (if failed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Enrichment data for a task.
#[derive(Debug, Clone, Serialize)]
pub struct TaskEnrichment {
    /// Estimated complexity (1-10).
    pub complexity: u8,
    /// Suggested tags based on content.
    pub suggested_tags: Vec<String>,
    /// Word count of title + description.
    pub word_count: usize,
}

/// Response for async pipeline.
#[derive(Debug, Serialize)]
pub struct AsyncPipelineResponse {
    /// Results for each task.
    pub results: Vec<PipelineTaskResult>,
    /// Total tasks processed.
    pub total_processed: usize,
    /// Number of successful tasks.
    pub successful: usize,
    /// Number of failed tasks.
    pub failed: usize,
}

// =============================================================================
// Lazy Compute DTOs
// =============================================================================

/// Request for lazy computation.
#[derive(Debug, Deserialize)]
pub struct LazyComputeRequest {
    /// Task ID.
    pub task_id: String,
    /// Type of computation to perform.
    pub computation: String,
    /// Whether to include the result (triggers evaluation).
    #[serde(default)]
    pub include_result: bool,
}

/// Response for lazy computation.
#[derive(Debug, Serialize)]
pub struct LazyComputeResponse {
    /// Task ID.
    pub task_id: String,
    /// Type of computation.
    pub computation_type: String,
    /// Computation result (only present if `include_result` was true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Whether the computation was evaluated.
    pub was_evaluated: bool,
}

// =============================================================================
// Pure Functions for History
// =============================================================================

/// Pure: Paginates items using Continuation monad.
///
/// This demonstrates CPS-based pagination where the continuation
/// represents "what to do with the paginated result".
fn paginate_with_continuation<T: Clone + 'static>(
    items: &[T],
    offset: usize,
    limit: usize,
) -> Continuation<PaginatedResult<T>, PaginatedResult<T>> {
    Continuation::pure(paginate_items(items, offset, limit))
}

/// Pure: Core pagination logic.
fn paginate_items<T: Clone>(items: &[T], offset: usize, limit: usize) -> PaginatedResult<T> {
    let total = items.len();
    let paginated: Vec<T> = items.iter().skip(offset).take(limit).cloned().collect();
    let next_offset = offset + paginated.len();
    let has_more = next_offset < total;

    PaginatedResult {
        items: paginated,
        next_cursor: if has_more { Some(next_offset) } else { None },
        has_more,
        total,
    }
}

// =============================================================================
// Pure Functions for Transform
// =============================================================================

/// Pure: Normalizes task title (trim whitespace, collapse multiple spaces).
fn normalize_title(task: Task) -> Task {
    let normalized = task.title.split_whitespace().collect::<Vec<_>>().join(" ");
    Task {
        title: normalized,
        ..task
    }
}

/// Pure: Converts title to title case.
fn title_case(task: Task) -> Task {
    let title_cased = task
        .title
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first
                    .to_uppercase()
                    .chain(chars.flat_map(char::to_lowercase))
                    .collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ");
    Task {
        title: title_cased,
        ..task
    }
}

/// Pure: Bumps priority one level up.
fn bump_priority(task: Task) -> Task {
    let new_priority = match task.priority {
        Priority::Low => Priority::Medium,
        Priority::Medium => Priority::High,
        Priority::High | Priority::Critical => Priority::Critical,
    };
    task.with_priority(new_priority)
}

/// Pure: Lowers priority one level down.
fn lower_priority(task: Task) -> Task {
    let new_priority = match task.priority {
        Priority::Critical => Priority::High,
        Priority::High => Priority::Medium,
        Priority::Medium | Priority::Low => Priority::Low,
    };
    task.with_priority(new_priority)
}

/// Pure: Adds "processed" tag to task.
fn add_processed_tag(task: Task) -> Task {
    task.add_tag(Tag::new("processed"))
}

/// Pure: Adds "reviewed" tag to task.
fn add_reviewed_tag(task: Task) -> Task {
    task.add_tag(Tag::new("reviewed"))
}

/// Pure: Resolves transformation name to function.
///
/// Returns Some(function) if known, None if unknown.
fn resolve_transformation(name: &str) -> Option<fn(Task) -> Task> {
    match name {
        "normalize_title" => Some(normalize_title),
        "title_case" => Some(title_case),
        "bump_priority" => Some(bump_priority),
        "lower_priority" => Some(lower_priority),
        "add_processed_tag" => Some(add_processed_tag),
        "add_reviewed_tag" => Some(add_reviewed_tag),
        _ => None,
    }
}

/// Pure: Applies a list of transformations to a task.
///
/// Returns `Ok(transformed_task)` or `Err(unknown_transformation_name)`.
///
/// For preset pipelines, uses `pipe!` or `compose!` macro directly for compile-time composition.
/// For custom transformation lists, resolves and applies functions sequentially.
///
/// # Preset Pipelines using `pipe!` (left-to-right)
///
/// - `standard_cleanup`: `pipe!(task, normalize_title, title_case)`
/// - `priority_bump_pipeline`: `pipe!(task, normalize_title, bump_priority, add_processed_tag)`
///
/// # Preset Pipelines using `compose!` (right-to-left)
///
/// - `compose_pipeline`: `compose!(add_processed_tag, bump_priority, normalize_title)(task)`
///   Executes: `normalize_title` → `bump_priority` → `add_processed_tag`
/// - `reverse_priority_pipeline`: `compose!(add_reviewed_tag, lower_priority, normalize_title)(task)`
///   Executes: `normalize_title` → `lower_priority` → `add_reviewed_tag`
fn apply_transformations(task: Task, transformations: &[String]) -> Result<Task, String> {
    // Check for preset pipelines that use pipe! or compose! macro
    if transformations.len() == 1 {
        match transformations[0].as_str() {
            // pipe! presets (left-to-right composition)
            "standard_cleanup" => {
                // Use pipe! macro for compile-time composition
                return Ok(pipe!(task, normalize_title, title_case));
            }
            "priority_bump_pipeline" => {
                // Use pipe! macro for compile-time composition
                return Ok(pipe!(
                    task,
                    normalize_title,
                    bump_priority,
                    add_processed_tag
                ));
            }
            // compose! presets (right-to-left composition)
            "compose_pipeline" => {
                // Use compose! macro for right-to-left composition
                // compose!(f, g, h)(x) = f(g(h(x)))
                // Execution order: normalize_title → bump_priority → add_processed_tag
                let composed = compose!(add_processed_tag, bump_priority, normalize_title);
                return Ok(composed(task));
            }
            "reverse_priority_pipeline" => {
                // Use compose! macro for right-to-left composition
                // Execution order: normalize_title → lower_priority → add_reviewed_tag
                let composed = compose!(add_reviewed_tag, lower_priority, normalize_title);
                return Ok(composed(task));
            }
            _ => {}
        }
    }

    // For custom transformation lists, resolve dynamically
    let mut current = task;

    for transformation_name in transformations {
        match resolve_transformation(transformation_name) {
            Some(transform_fn) => {
                current = transform_fn(current);
            }
            None => {
                return Err(format!("Unknown transformation: {transformation_name}"));
            }
        }
    }

    Ok(current)
}

/// Pure: Demonstrates compose! macro for right-to-left composition.
///
/// compose!(f, g, h)(x) = f(g(h(x)))
#[allow(dead_code)]
fn demonstrate_compose(task: Task) -> Task {
    let composed = compose!(add_processed_tag, bump_priority, normalize_title);
    composed(task)
}

/// Pure: Demonstrates pipe! macro for left-to-right composition.
///
/// pipe!(x, f, g, h) = h(g(f(x)))
#[allow(dead_code)]
fn demonstrate_pipe(task: Task) -> Task {
    pipe!(task, normalize_title, bump_priority, add_processed_tag)
}

// =============================================================================
// Pure Functions for Pipeline
// =============================================================================

/// Pure: Validates a task and returns a score (0-100).
#[allow(dead_code)]
fn validate_task_score(task: &Task) -> u32 {
    let mut score = 50u32;

    // Title length check
    if task.title.len() >= 5 {
        score += 10;
    }
    if task.title.len() >= 20 {
        score += 10;
    }

    // Description check
    if task.description.is_some() {
        score += 15;
    }

    // Tags check
    if !task.tags.is_empty() {
        score += 10;
    }
    if task.tags.len() >= 3 {
        score += 5;
    }

    score.min(100)
}

/// Pure: Enriches task with computed metadata.
fn enrich_task(task: &Task) -> TaskEnrichment {
    let word_count = task.title.split_whitespace().count()
        + task
            .description
            .as_ref()
            .map_or(0, |d| d.split_whitespace().count());

    let complexity = match word_count {
        0..=5 => 1,
        6..=15 => 3,
        16..=30 => 5,
        31..=50 => 7,
        _ => 9,
    };

    let suggested_tags = generate_suggested_tags(task);

    TaskEnrichment {
        complexity,
        suggested_tags,
        word_count,
    }
}

/// Pure: Generates suggested tags based on task content.
fn generate_suggested_tags(task: &Task) -> Vec<String> {
    let mut suggestions = Vec::new();
    let content = format!(
        "{} {}",
        task.title,
        task.description.as_deref().unwrap_or("")
    )
    .to_lowercase();

    // Simple keyword matching for demonstration
    if content.contains("bug") || content.contains("fix") {
        suggestions.push("bug".to_string());
    }
    if content.contains("feature") || content.contains("add") {
        suggestions.push("feature".to_string());
    }
    if content.contains("urgent") || content.contains("critical") {
        suggestions.push("urgent".to_string());
    }
    if content.contains("doc") || content.contains("readme") {
        suggestions.push("documentation".to_string());
    }
    if content.contains("test") {
        suggestions.push("testing".to_string());
    }

    suggestions
}

// =============================================================================
// Pure Functions for Lazy Compute
// =============================================================================

/// Pure: Calculates complexity score for a task.
#[allow(clippy::cast_possible_truncation)]
fn calculate_complexity_score(task: &Task) -> u32 {
    let title_complexity = task.title.len() as u32;
    let desc_complexity = task.description.as_ref().map_or(0, |d| d.len() as u32);
    let subtask_complexity = (task.subtasks.len() as u32) * 10;
    let tag_complexity = (task.tags.len() as u32) * 5;

    title_complexity + desc_complexity + subtask_complexity + tag_complexity
}

/// Pure: Calculates estimated duration in minutes.
#[allow(clippy::cast_possible_truncation)]
fn calculate_estimated_duration(task: &Task) -> u32 {
    let base_duration = match task.priority {
        Priority::Critical => 120,
        Priority::High => 60,
        Priority::Medium => 30,
        Priority::Low => 15,
    };

    let subtask_duration = (task.subtasks.len() as u32) * 15;
    let complexity_factor = calculate_complexity_score(task) / 50;

    base_duration + subtask_duration + complexity_factor
}

/// Pure: Generates a summary of the task.
fn generate_task_summary(task: &Task) -> String {
    let status_str = format!("{:?}", task.status).to_lowercase();
    let priority_str = format!("{:?}", task.priority).to_lowercase();
    let tag_count = task.tags.len();
    let subtask_count = task.subtasks.len();

    format!(
        "Task '{}' is {} with {} priority. {} tags, {} subtasks.",
        task.title, status_str, priority_str, tag_count, subtask_count
    )
}

// =============================================================================
// GET /tasks/{id}/history Handler
// =============================================================================

/// Returns task event history with CPS-based pagination.
///
/// This handler demonstrates:
/// - **`PersistentList`**: Efficient prepend for building event history
/// - **Continuation**: CPS-based pagination pattern
///
/// # Query Parameters
///
/// - `limit`: Number of events per page (default: 20, max: 100)
/// - `cursor`: Pagination cursor (event offset)
///
/// # Response
///
/// Returns paginated list of task events.
///
/// # Errors
///
/// Returns `ApiErrorResponse` in the following cases:
/// - Task not found (404)
/// - Invalid task ID format (400)
/// - Repository error (500)
pub async fn get_task_history(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Query(query): Query<HistoryQuery>,
) -> Result<JsonResponse<TaskHistoryResponse>, ApiErrorResponse> {
    // Parse and validate task ID
    let task_id = parse_task_id(&task_id)?;

    // Verify task exists
    let _task = state
        .task_repository
        .find_by_id(&task_id)
        .await
        .map_err(ApiErrorResponse::from)?
        .ok_or_else(|| ApiErrorResponse::not_found("Task not found"))?;

    // Load events from EventStore (real I/O)
    let history_list = state
        .event_store
        .load_events(&task_id)
        .await
        .map_err(ApiErrorResponse::from)?;

    // Compute pagination parameters
    // Use clamp(1, MAX) to ensure limit is at least 1 (prevents infinite pagination loops)
    let limit = query
        .limit
        .unwrap_or(DEFAULT_HISTORY_LIMIT)
        .clamp(1, MAX_HISTORY_LIMIT);
    let cursor = query.cursor.unwrap_or(0);

    // Build response synchronously (Continuation/PersistentList are not Send)
    let response = {
        // Convert domain events to API DTOs (filter out unsupported event types)
        // PersistentList stores events in reverse chronological order (newest first via cons),
        // but load_events returns in chronological order (oldest first).
        let history_vec: Vec<TaskEvent> = history_list
            .iter()
            .filter_map(TaskEvent::from_domain)
            .collect();

        // Paginate using Continuation monad
        let paginated =
            paginate_with_continuation(&history_vec, cursor, limit).run(|result| result);

        TaskHistoryResponse {
            task_id: task_id.to_string(),
            events: paginated.items,
            next_cursor: paginated.next_cursor,
            has_more: paginated.has_more,
            total_events: paginated.total,
        }
    };

    Ok(JsonResponse(response))
}

// =============================================================================
// POST /tasks/transform Handler
// =============================================================================

/// Applies a transformation pipeline to a task.
///
/// This handler demonstrates:
/// - **`pipe!`**: Compile-time function composition (for preset pipelines)
/// - Pure transformation functions (immutable task updates)
///
/// # Preset Pipelines (using `pipe!` macro)
///
/// - `standard_cleanup`: `pipe!(task, normalize_title, title_case)`
/// - `priority_bump_pipeline`: `pipe!(task, normalize_title, bump_priority, add_processed_tag)`
///
/// # Individual Transformations
///
/// - `normalize_title`: Trim and collapse whitespace
/// - `title_case`: Convert to Title Case
/// - `bump_priority`: Increase priority one level
/// - `lower_priority`: Decrease priority one level
/// - `add_processed_tag`: Add "processed" tag
/// - `add_reviewed_tag`: Add "reviewed" tag
///
/// # Request Body
///
/// ```json
/// {
///   "task_id": "uuid",
///   "transformations": ["normalize_title", "bump_priority"]
/// }
/// ```
///
/// # Response
///
/// Returns the transformed task or error details.
///
/// # Errors
///
/// Returns `ApiErrorResponse` in the following cases:
/// - Task not found (404)
/// - Invalid task ID format (400)
/// - Unknown transformation name (400)
/// - Repository error (500)
pub async fn transform_task(
    State(state): State<AppState>,
    Json(request): Json<TransformRequest>,
) -> Result<JsonResponse<TransformResponse>, ApiErrorResponse> {
    // Parse and validate task ID
    let task_id = parse_task_id(&request.task_id)?;

    // Fetch task from repository
    let task = state
        .task_repository
        .find_by_id(&task_id)
        .await
        .map_err(ApiErrorResponse::from)?
        .ok_or_else(|| ApiErrorResponse::not_found("Task not found"))?;

    // Apply transformations synchronously
    // Note: Task contains PersistentHashSet/PersistentList which use Rc internally,
    // so Task cannot cross await boundaries. We convert to TaskResponse (which is Send)
    // before the repository save operation.
    let (transformed, applied) = {
        match apply_transformations(task, &request.transformations) {
            Ok(transformed_task) => {
                let response = TaskResponse::from(&transformed_task);
                (
                    Ok((transformed_task, response)),
                    request.transformations.clone(),
                )
            }
            Err(error_msg) => (Err(error_msg), vec![]),
        }
    };

    match transformed {
        Ok((task_to_save, response)) => {
            // Save transformed task
            state
                .task_repository
                .save(&task_to_save)
                .await
                .map_err(ApiErrorResponse::from)?;

            Ok(JsonResponse(TransformResponse {
                success: true,
                task: Some(response),
                applied_transformations: applied,
                error: None,
            }))
        }
        Err(error_msg) => Ok(JsonResponse(TransformResponse {
            success: false,
            task: None,
            applied_transformations: applied,
            error: Some(error_msg),
        })),
    }
}

// =============================================================================
// POST /tasks/async-pipeline Handler
// =============================================================================

/// Processes tasks through an async validation and enrichment pipeline.
///
/// This handler demonstrates:
/// - **`for_async!`**: Async list comprehension for batch validation score computation
/// - Sequential async processing with pure validation/enrichment functions
/// - Batch processing with size limits for controlled resource usage
///
/// # Pipeline Steps
///
/// 1. Fetch all valid tasks from repository
/// 2. Use `for_async!` to compute batch validation scores
/// 3. Enrich each task with computed metadata
///
/// # Request Body
///
/// ```json
/// {
///   "task_ids": ["uuid1", "uuid2"]
/// }
/// ```
///
/// # Response
///
/// Returns processing results for each task.
///
/// # Errors
///
/// Returns `ApiErrorResponse` in the following cases:
/// - Too many tasks in batch (400)
/// - Repository error (500)
pub async fn async_pipeline(
    State(state): State<AppState>,
    Json(request): Json<AsyncPipelineRequest>,
) -> Result<JsonResponse<AsyncPipelineResponse>, ApiErrorResponse> {
    // Validate batch size
    if request.task_ids.len() > MAX_PIPELINE_BATCH_SIZE {
        return Err(ApiErrorResponse::bad_request(
            "BATCH_TOO_LARGE",
            format!(
                "Maximum batch size is {}. Received: {}",
                MAX_PIPELINE_BATCH_SIZE,
                request.task_ids.len()
            ),
        ));
    }

    // First, fetch all tasks and separate valid/invalid ones
    let mut valid_tasks: Vec<Task> = Vec::new();
    let mut failed_results: Vec<PipelineTaskResult> = Vec::new();

    for task_id_str in &request.task_ids {
        let task_id = if let Ok(uuid) = Uuid::parse_str(task_id_str) {
            TaskId::from_uuid(uuid)
        } else {
            failed_results.push(PipelineTaskResult {
                task_id: task_id_str.clone(),
                success: false,
                validation_score: None,
                enrichment: None,
                error: Some("Invalid task ID format".to_string()),
            });
            continue;
        };

        match state.task_repository.find_by_id(&task_id).await {
            Ok(Some(task)) => valid_tasks.push(task),
            Ok(None) => {
                failed_results.push(PipelineTaskResult {
                    task_id: task_id_str.clone(),
                    success: false,
                    validation_score: None,
                    enrichment: None,
                    error: Some("Task not found".to_string()),
                });
            }
            Err(e) => {
                failed_results.push(PipelineTaskResult {
                    task_id: task_id_str.clone(),
                    success: false,
                    validation_score: None,
                    enrichment: None,
                    error: Some(format!("Repository error: {e}")),
                });
            }
        }
    }

    // Extract Send-safe validation inputs from tasks (Task contains Rc-based types, not Send)
    let validation_inputs: Vec<TaskValidationInput> = valid_tasks
        .iter()
        .map(TaskValidationInput::from_task)
        .collect();

    // Use for_async! to compute batch validation scores (demonstrates async list comprehension)
    // for_async! generates AsyncIO<Vec<T>> which is a deferred computation
    // This demonstrates the declarative batch processing pattern with async comprehension
    let scores_async = compute_batch_validation_scores_async(validation_inputs);

    // Execute the AsyncIO computation
    // Note: Since we're using AsyncIO::pure internally, this is effectively synchronous,
    // but demonstrates the proper for_async! pattern that would work with real async operations
    let batch_scores: Vec<(String, u32)> = scores_async.await;

    // Build successful results from batch computation
    let mut results: Vec<PipelineTaskResult> = Vec::with_capacity(request.task_ids.len());

    // Create lookup maps for scores and enrichments
    let score_map: std::collections::HashMap<String, u32> = batch_scores.into_iter().collect();
    // Compute enrichments and build HashMap directly (avoids intermediate Vec)
    let enrichment_map: std::collections::HashMap<String, TaskEnrichment> = valid_tasks
        .iter()
        .map(|task| (task.task_id.to_string(), enrich_task(task)))
        .collect();

    // Preserve original order from request
    for task_id_str in &request.task_ids {
        if let (Some(&score), Some(enrichment)) = (
            score_map.get(task_id_str),
            enrichment_map.get(task_id_str).cloned(),
        ) {
            results.push(PipelineTaskResult {
                task_id: task_id_str.clone(),
                success: true,
                validation_score: Some(score),
                enrichment: Some(enrichment),
                error: None,
            });
        } else {
            // Find the failed result for this task_id
            if let Some(failed) = failed_results.iter().find(|r| &r.task_id == task_id_str) {
                results.push(failed.clone());
            }
        }
    }

    let successful = results.iter().filter(|r| r.success).count();
    let failed = results.len() - successful;

    Ok(JsonResponse(AsyncPipelineResponse {
        results,
        total_processed: request.task_ids.len(),
        successful,
        failed,
    }))
}

/// Processes a single task through the pipeline.
#[allow(dead_code)]
async fn process_single_task(state: &AppState, task_id_str: &str) -> PipelineTaskResult {
    // Parse task ID
    let task_id = match Uuid::parse_str(task_id_str) {
        Ok(uuid) => TaskId::from_uuid(uuid),
        Err(_) => {
            return PipelineTaskResult {
                task_id: task_id_str.to_string(),
                success: false,
                validation_score: None,
                enrichment: None,
                error: Some("Invalid task ID format".to_string()),
            };
        }
    };

    // Fetch task
    let task = match state.task_repository.find_by_id(&task_id).await {
        Ok(Some(task)) => task,
        Ok(None) => {
            return PipelineTaskResult {
                task_id: task_id_str.to_string(),
                success: false,
                validation_score: None,
                enrichment: None,
                error: Some("Task not found".to_string()),
            };
        }
        Err(e) => {
            return PipelineTaskResult {
                task_id: task_id_str.to_string(),
                success: false,
                validation_score: None,
                enrichment: None,
                error: Some(format!("Repository error: {e}")),
            };
        }
    };

    // Process synchronously (Task is not Send)
    let (validation_score, enrichment) = {
        let score = validate_task_score(&task);
        let enrichment = enrich_task(&task);
        (score, enrichment)
    };

    PipelineTaskResult {
        task_id: task_id_str.to_string(),
        success: true,
        validation_score: Some(validation_score),
        enrichment: Some(enrichment),
        error: None,
    }
}

/// Holds extracted task information for batch processing.
///
/// This struct contains only Send-safe data extracted from Task,
/// allowing it to be used across await boundaries with `for_async!`.
#[derive(Debug, Clone)]
struct TaskValidationInput {
    task_id: String,
    title_length: usize,
    has_description: bool,
    tags_count: usize,
}

impl TaskValidationInput {
    /// Extracts validation-relevant information from a Task.
    fn from_task(task: &Task) -> Self {
        Self {
            task_id: task.task_id.to_string(),
            title_length: task.title.len(),
            has_description: task.description.is_some(),
            tags_count: task.tags.len(),
        }
    }

    /// Computes validation score from extracted data.
    fn compute_score(&self) -> u32 {
        let mut score = 50u32;

        // Title length check
        if self.title_length >= 5 {
            score += 10;
        }
        if self.title_length >= 20 {
            score += 10;
        }

        // Description check
        if self.has_description {
            score += 15;
        }

        // Tags check
        if self.tags_count > 0 {
            score += 10;
        }
        if self.tags_count >= 3 {
            score += 5;
        }

        score.min(100)
    }
}

/// Computes batch validation scores using `for_async!` macro.
///
/// This function demonstrates `for_async!` for async list comprehension,
/// processing a collection of task validation inputs and computing scores.
///
/// # Arguments
///
/// * `inputs` - Vector of `TaskValidationInput` (Send-safe task data)
///
/// # Returns
///
/// `AsyncIO<Vec<(String, u32)>>` - Task IDs with their validation scores
///
/// # Example
///
/// ```rust,ignore
/// let inputs = tasks.iter().map(TaskValidationInput::from_task).collect();
/// let scores_async = compute_batch_validation_scores_async(inputs);
/// let scores = scores_async.await;
/// ```
fn compute_batch_validation_scores_async(
    inputs: Vec<TaskValidationInput>,
) -> AsyncIO<Vec<(String, u32)>> {
    for_async! {
        input <= inputs;
        // Compute validation score from extracted data
        let score = input.compute_score();
        // Wrap in AsyncIO to demonstrate async comprehension pattern
        // In a real scenario, this could be an async validation service call
        validated_score <~ AsyncIO::pure(score);
        // Yield task ID with its validation score
        yield (input.task_id.clone(), validated_score)
    }
}

// =============================================================================
// POST /tasks/lazy-compute Handler
// =============================================================================

/// Performs lazy computation on a task.
///
/// This handler demonstrates:
/// - **Lazy**: Deferred evaluation with memoization
/// - Computation is only performed if `include_result` is true
///
/// # Available Computations
///
/// - `complexity_score`: Calculate task complexity
/// - `estimated_duration`: Estimate time to complete (minutes)
/// - `summary`: Generate task summary text
///
/// # Request Body
///
/// ```json
/// {
///   "task_id": "uuid",
///   "computation": "complexity_score",
///   "include_result": true
/// }
/// ```
///
/// # Response
///
/// Returns computation result if requested.
///
/// # Errors
///
/// Returns `ApiErrorResponse` in the following cases:
/// - Task not found (404)
/// - Invalid task ID format (400)
/// - Unknown computation type (400)
/// - Repository error (500)
pub async fn lazy_compute(
    State(state): State<AppState>,
    Json(request): Json<LazyComputeRequest>,
) -> Result<JsonResponse<LazyComputeResponse>, ApiErrorResponse> {
    // Parse and validate task ID
    let task_id = parse_task_id(&request.task_id)?;

    // Validate computation type
    if !["complexity_score", "estimated_duration", "summary"]
        .contains(&request.computation.as_str())
    {
        return Err(ApiErrorResponse::bad_request(
            "INVALID_COMPUTATION",
            "Unknown computation type. Valid: complexity_score, estimated_duration, summary",
        ));
    }

    // Fetch task from repository
    let task = state
        .task_repository
        .find_by_id(&task_id)
        .await
        .map_err(ApiErrorResponse::from)?
        .ok_or_else(|| ApiErrorResponse::not_found("Task not found"))?;

    // Create lazy computation (synchronous block - Lazy is not Send)
    // Demonstrates deferred evaluation: computation is defined but not executed until force()
    let (result, was_evaluated) = {
        match request.computation.as_str() {
            "complexity_score" => {
                // Create lazy computation (not yet evaluated)
                let lazy = Lazy::new(|| calculate_complexity_score(&task));
                if request.include_result {
                    // Force evaluation only when requested
                    let score = *lazy.force();
                    (Some(serde_json::json!(score)), true)
                } else {
                    // Lazy created but intentionally not evaluated - goes out of scope unused
                    // This demonstrates that Lazy only computes when force() is called
                    let _ = lazy;
                    (None, false)
                }
            }
            "estimated_duration" => {
                let lazy = Lazy::new(|| calculate_estimated_duration(&task));
                if request.include_result {
                    let duration = *lazy.force();
                    (Some(serde_json::json!(duration)), true)
                } else {
                    let _ = lazy;
                    (None, false)
                }
            }
            "summary" => {
                let lazy = Lazy::new(|| generate_task_summary(&task));
                if request.include_result {
                    let summary = lazy.force().clone();
                    (Some(serde_json::json!(summary)), true)
                } else {
                    let _ = lazy;
                    (None, false)
                }
            }
            _ => unreachable!("Validated above"),
        }
    };

    Ok(JsonResponse(LazyComputeResponse {
        task_id: request.task_id,
        computation_type: request.computation,
        result,
        was_evaluated,
    }))
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Parses a task ID from string.
fn parse_task_id(id: &str) -> Result<TaskId, ApiErrorResponse> {
    Uuid::parse_str(id)
        .map(TaskId::from_uuid)
        .map_err(|_| ApiErrorResponse::bad_request("INVALID_TASK_ID", "Invalid task ID format"))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{TaskStatus, Timestamp};
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // TaskEvent::from_domain Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_task_event_from_domain_created() {
        use crate::domain::{EventId, TaskCreated};

        let domain_event = DomainTaskEvent::new(
            EventId::generate(),
            TaskId::generate(),
            Timestamp::now(),
            1,
            TaskEventKind::Created(TaskCreated {
                title: "Test Task".to_string(),
                description: None,
                priority: Priority::Low,
                status: TaskStatus::Pending,
            }),
        );

        let api_event = TaskEvent::from_domain(&domain_event);
        assert!(api_event.is_some());

        if let Some(TaskEvent::Created { title, .. }) = api_event {
            assert_eq!(title, "Test Task");
        } else {
            panic!("Expected Created event");
        }
    }

    #[rstest]
    fn test_task_event_from_domain_status_changed() {
        use crate::domain::{EventId, StatusChanged};

        let domain_event = DomainTaskEvent::new(
            EventId::generate(),
            TaskId::generate(),
            Timestamp::now(),
            2,
            TaskEventKind::StatusChanged(StatusChanged {
                old_status: TaskStatus::Pending,
                new_status: TaskStatus::InProgress,
            }),
        );

        let api_event = TaskEvent::from_domain(&domain_event);
        assert!(api_event.is_some());

        if let Some(TaskEvent::StatusChanged {
            old_status,
            new_status,
            ..
        }) = api_event
        {
            assert_eq!(old_status, TaskStatusDto::Pending);
            assert_eq!(new_status, TaskStatusDto::InProgress);
        } else {
            panic!("Expected StatusChanged event");
        }
    }

    #[rstest]
    fn test_task_event_from_domain_unsupported_event() {
        use crate::domain::{EventId, TaskDescriptionUpdated};

        let domain_event = DomainTaskEvent::new(
            EventId::generate(),
            TaskId::generate(),
            Timestamp::now(),
            2,
            TaskEventKind::DescriptionUpdated(TaskDescriptionUpdated {
                old_description: None,
                new_description: Some("New description".to_string()),
            }),
        );

        // DescriptionUpdated is not exposed in the API
        let api_event = TaskEvent::from_domain(&domain_event);
        assert!(api_event.is_none());
    }

    // -------------------------------------------------------------------------
    // Pagination Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_paginate_items_first_page() {
        let items: Vec<i32> = (0..50).collect();
        let result = paginate_items(&items, 0, 10);

        assert_eq!(result.items.len(), 10);
        assert_eq!(result.items[0], 0);
        assert_eq!(result.items[9], 9);
        assert!(result.has_more);
        assert_eq!(result.next_cursor, Some(10));
        assert_eq!(result.total, 50);
    }

    #[rstest]
    fn test_paginate_items_last_page() {
        let items: Vec<i32> = (0..25).collect();
        let result = paginate_items(&items, 20, 10);

        assert_eq!(result.items.len(), 5);
        assert!(!result.has_more);
        assert_eq!(result.next_cursor, None);
    }

    #[rstest]
    fn test_paginate_with_continuation() {
        let items: Vec<i32> = (0..30).collect();
        let continuation = paginate_with_continuation(&items, 0, 10);
        let result = continuation.run(|r| r);

        assert_eq!(result.items.len(), 10);
        assert!(result.has_more);
    }

    // -------------------------------------------------------------------------
    // Transform Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_normalize_title() {
        let task = Task::new(
            TaskId::generate(),
            "  Test   Task  ".to_string(),
            Timestamp::now(),
        );
        let normalized = normalize_title(task);

        assert_eq!(normalized.title, "Test Task");
    }

    #[rstest]
    fn test_title_case() {
        let task = Task::new(
            TaskId::generate(),
            "hello world".to_string(),
            Timestamp::now(),
        );
        let title_cased = title_case(task);

        assert_eq!(title_cased.title, "Hello World");
    }

    #[rstest]
    fn test_bump_priority() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now())
            .with_priority(Priority::Low);
        let bumped = bump_priority(task);

        assert_eq!(bumped.priority, Priority::Medium);
    }

    #[rstest]
    fn test_bump_priority_at_max() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now())
            .with_priority(Priority::Critical);
        let bumped = bump_priority(task);

        assert_eq!(bumped.priority, Priority::Critical);
    }

    #[rstest]
    fn test_lower_priority() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now())
            .with_priority(Priority::High);
        let lowered = lower_priority(task);

        assert_eq!(lowered.priority, Priority::Medium);
    }

    #[rstest]
    fn test_add_processed_tag() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());
        let tagged = add_processed_tag(task);

        assert!(tagged.tags.contains(&Tag::new("processed")));
    }

    #[rstest]
    fn test_resolve_transformation_valid() {
        assert!(resolve_transformation("normalize_title").is_some());
        assert!(resolve_transformation("title_case").is_some());
        assert!(resolve_transformation("bump_priority").is_some());
    }

    #[rstest]
    fn test_resolve_transformation_invalid() {
        assert!(resolve_transformation("unknown").is_none());
        assert!(resolve_transformation("").is_none());
    }

    #[rstest]
    fn test_apply_transformations_success() {
        let task = Task::new(
            TaskId::generate(),
            "  test task  ".to_string(),
            Timestamp::now(),
        )
        .with_priority(Priority::Low);

        let result = apply_transformations(
            task,
            &["normalize_title".to_string(), "bump_priority".to_string()],
        );

        assert!(result.is_ok());
        let transformed = result.unwrap();
        assert_eq!(transformed.title, "test task");
        assert_eq!(transformed.priority, Priority::Medium);
    }

    #[rstest]
    fn test_apply_transformations_unknown() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());
        let result = apply_transformations(task, &["unknown_transform".to_string()]);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown transformation"));
    }

    #[rstest]
    fn test_compose_macro() {
        let task = Task::new(
            TaskId::generate(),
            "  test  task  ".to_string(),
            Timestamp::now(),
        )
        .with_priority(Priority::Low);

        let result = demonstrate_compose(task);

        assert_eq!(result.title, "test task");
        assert_eq!(result.priority, Priority::Medium);
        assert!(result.tags.contains(&Tag::new("processed")));
    }

    #[rstest]
    fn test_pipe_macro() {
        let task = Task::new(
            TaskId::generate(),
            "  test  task  ".to_string(),
            Timestamp::now(),
        )
        .with_priority(Priority::Low);

        let result = demonstrate_pipe(task);

        assert_eq!(result.title, "test task");
        assert_eq!(result.priority, Priority::Medium);
        assert!(result.tags.contains(&Tag::new("processed")));
    }

    #[rstest]
    fn test_apply_transformations_compose_pipeline() {
        let task = Task::new(
            TaskId::generate(),
            "  test  task  ".to_string(),
            Timestamp::now(),
        )
        .with_priority(Priority::Low);

        let result = apply_transformations(task, &["compose_pipeline".to_string()]);

        assert!(result.is_ok());
        let transformed = result.unwrap();
        // compose!(add_processed_tag, bump_priority, normalize_title)
        // Execution order: normalize_title → bump_priority → add_processed_tag
        assert_eq!(transformed.title, "test task");
        assert_eq!(transformed.priority, Priority::Medium);
        assert!(transformed.tags.contains(&Tag::new("processed")));
    }

    #[rstest]
    fn test_apply_transformations_reverse_priority_pipeline() {
        let task = Task::new(
            TaskId::generate(),
            "  test  task  ".to_string(),
            Timestamp::now(),
        )
        .with_priority(Priority::High);

        let result = apply_transformations(task, &["reverse_priority_pipeline".to_string()]);

        assert!(result.is_ok());
        let transformed = result.unwrap();
        // compose!(add_reviewed_tag, lower_priority, normalize_title)
        // Execution order: normalize_title → lower_priority → add_reviewed_tag
        assert_eq!(transformed.title, "test task");
        assert_eq!(transformed.priority, Priority::Medium);
        assert!(transformed.tags.contains(&Tag::new("reviewed")));
    }

    // -------------------------------------------------------------------------
    // for_async! Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_task_validation_input_from_task() {
        let task = Task::new(
            TaskId::generate(),
            "This is a test task title".to_string(),
            Timestamp::now(),
        )
        .with_description("A description".to_string())
        .add_tag(Tag::new("tag1"))
        .add_tag(Tag::new("tag2"));

        let input = TaskValidationInput::from_task(&task);

        assert_eq!(input.title_length, 25);
        assert!(input.has_description);
        assert_eq!(input.tags_count, 2);
    }

    #[rstest]
    fn test_task_validation_input_compute_score() {
        // Minimal task: score should be 50
        let minimal_input = TaskValidationInput {
            task_id: "test-id".to_string(),
            title_length: 3,
            has_description: false,
            tags_count: 0,
        };
        assert_eq!(minimal_input.compute_score(), 50);

        // Complete task: score should be high
        let complete_input = TaskValidationInput {
            task_id: "test-id".to_string(),
            title_length: 25,
            has_description: true,
            tags_count: 3,
        };
        // 50 base + 10 (title >= 5) + 10 (title >= 20) + 15 (description) + 10 (tags > 0) + 5 (tags >= 3)
        assert_eq!(complete_input.compute_score(), 100);
    }

    #[rstest]
    #[tokio::test]
    async fn test_compute_batch_validation_scores_async() {
        let inputs = vec![
            TaskValidationInput {
                task_id: "task-1".to_string(),
                title_length: 10,
                has_description: true,
                tags_count: 2,
            },
            TaskValidationInput {
                task_id: "task-2".to_string(),
                title_length: 3,
                has_description: false,
                tags_count: 0,
            },
        ];

        let scores_async = compute_batch_validation_scores_async(inputs);
        let scores: Vec<(String, u32)> = scores_async.await;

        assert_eq!(scores.len(), 2);
        assert_eq!(scores[0].0, "task-1");
        assert!(scores[0].1 > 70); // Should have high score
        assert_eq!(scores[1].0, "task-2");
        assert_eq!(scores[1].1, 50); // Minimal score
    }

    // -------------------------------------------------------------------------
    // Pipeline Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_validate_task_score_basic() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());
        let score = validate_task_score(&task);

        assert!(score >= 50);
        assert!(score <= 100);
    }

    #[rstest]
    fn test_validate_task_score_complete() {
        let task = Task::new(
            TaskId::generate(),
            "This is a properly described task".to_string(),
            Timestamp::now(),
        )
        .with_description("A detailed description of what needs to be done".to_string())
        .add_tag(Tag::new("tag1"))
        .add_tag(Tag::new("tag2"))
        .add_tag(Tag::new("tag3"));

        let score = validate_task_score(&task);
        assert!(score >= 90);
    }

    #[rstest]
    fn test_enrich_task() {
        let task = Task::new(
            TaskId::generate(),
            "Fix bug in login".to_string(),
            Timestamp::now(),
        )
        .with_description("Users cannot log in with email".to_string());

        let enrichment = enrich_task(&task);

        assert!(enrichment.complexity >= 1);
        assert!(enrichment.word_count > 0);
        assert!(enrichment.suggested_tags.contains(&"bug".to_string()));
    }

    #[rstest]
    fn test_generate_suggested_tags() {
        let task = Task::new(
            TaskId::generate(),
            "Add new feature for testing".to_string(),
            Timestamp::now(),
        );

        let tags = generate_suggested_tags(&task);

        assert!(tags.contains(&"feature".to_string()));
        assert!(tags.contains(&"testing".to_string()));
    }

    // -------------------------------------------------------------------------
    // Lazy Compute Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_calculate_complexity_score() {
        let task = Task::new(TaskId::generate(), "Simple".to_string(), Timestamp::now());
        let score = calculate_complexity_score(&task);

        assert!(score > 0);
    }

    #[rstest]
    fn test_calculate_complexity_score_complex() {
        let task = Task::new(
            TaskId::generate(),
            "Complex task with many details".to_string(),
            Timestamp::now(),
        )
        .with_description(
            "This is a very detailed description of what needs to be done".to_string(),
        )
        .add_tag(Tag::new("tag1"))
        .add_tag(Tag::new("tag2"));

        let score = calculate_complexity_score(&task);
        assert!(score > 50);
    }

    #[rstest]
    fn test_calculate_estimated_duration() {
        let low_task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now())
            .with_priority(Priority::Low);
        let critical_task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now())
            .with_priority(Priority::Critical);

        let low_duration = calculate_estimated_duration(&low_task);
        let critical_duration = calculate_estimated_duration(&critical_task);

        assert!(critical_duration > low_duration);
    }

    #[rstest]
    fn test_generate_task_summary() {
        let task = Task::new(
            TaskId::generate(),
            "Test Task".to_string(),
            Timestamp::now(),
        )
        .with_priority(Priority::High)
        .add_tag(Tag::new("important"));

        let summary = generate_task_summary(&task);

        assert!(summary.contains("Test Task"));
        assert!(summary.contains("high"));
        assert!(summary.contains("1 tags"));
    }

    #[rstest]
    fn test_lazy_evaluation() {
        use std::cell::Cell;

        let call_count = Cell::new(0);
        let lazy = Lazy::new(|| {
            call_count.set(call_count.get() + 1);
            42
        });

        // Not evaluated yet
        assert_eq!(call_count.get(), 0);

        // First force
        let value1 = *lazy.force();
        assert_eq!(value1, 42);
        assert_eq!(call_count.get(), 1);

        // Second force - memoized, no re-evaluation
        let value2 = *lazy.force();
        assert_eq!(value2, 42);
        assert_eq!(call_count.get(), 1);
    }

    // -------------------------------------------------------------------------
    // EventStore Integration Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_task_event_from_domain_title_updated() {
        use crate::domain::{EventId, TaskTitleUpdated};

        let domain_event = DomainTaskEvent::new(
            EventId::generate(),
            TaskId::generate(),
            Timestamp::now(),
            2,
            TaskEventKind::TitleUpdated(TaskTitleUpdated {
                old_title: "Old Title".to_string(),
                new_title: "New Title".to_string(),
            }),
        );

        let api_event = TaskEvent::from_domain(&domain_event);
        assert!(api_event.is_some());

        if let Some(TaskEvent::TitleUpdated {
            old_title,
            new_title,
            ..
        }) = api_event
        {
            assert_eq!(old_title, "Old Title");
            assert_eq!(new_title, "New Title");
        } else {
            panic!("Expected TitleUpdated event");
        }
    }

    #[rstest]
    fn test_task_event_from_domain_priority_changed() {
        use crate::domain::{EventId, PriorityChanged};

        let domain_event = DomainTaskEvent::new(
            EventId::generate(),
            TaskId::generate(),
            Timestamp::now(),
            2,
            TaskEventKind::PriorityChanged(PriorityChanged {
                old_priority: Priority::Low,
                new_priority: Priority::High,
            }),
        );

        let api_event = TaskEvent::from_domain(&domain_event);
        assert!(api_event.is_some());

        if let Some(TaskEvent::PriorityChanged {
            old_priority,
            new_priority,
            ..
        }) = api_event
        {
            assert_eq!(old_priority, PriorityDto::Low);
            assert_eq!(new_priority, PriorityDto::High);
        } else {
            panic!("Expected PriorityChanged event");
        }
    }

    #[rstest]
    fn test_task_event_from_domain_tag_added() {
        use crate::domain::{EventId, Tag, TagAdded};

        let domain_event = DomainTaskEvent::new(
            EventId::generate(),
            TaskId::generate(),
            Timestamp::now(),
            2,
            TaskEventKind::TagAdded(TagAdded {
                tag: Tag::new("important"),
            }),
        );

        let api_event = TaskEvent::from_domain(&domain_event);
        assert!(api_event.is_some());

        if let Some(TaskEvent::TagAdded { tag, .. }) = api_event {
            assert_eq!(tag, "important");
        } else {
            panic!("Expected TagAdded event");
        }
    }

    // -------------------------------------------------------------------------
    // Pagination Limit Clamping Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_get_task_history_limit_zero_clamped_to_one() {
        // This test verifies that limit=0 is clamped to 1 to prevent infinite pagination loops.
        // When limit=0, paginate_items would return 0 items but has_more=true if total > 0,
        // causing next_cursor to never advance and creating an infinite loop.

        let items: Vec<i32> = (1..=10).collect();

        // Simulate the limit calculation logic from get_task_history
        // Using clamp(1, MAX_HISTORY_LIMIT) ensures limit is at least 1
        let query_limit: usize = 0;
        let limit = query_limit.clamp(1, MAX_HISTORY_LIMIT);

        assert_eq!(limit, 1, "limit=0 should be clamped to 1");

        // Verify pagination works correctly with clamped limit
        let result = paginate_items(&items, 0, limit);
        assert_eq!(result.items.len(), 1, "Should return exactly 1 item");
        assert_eq!(result.next_cursor, Some(1), "Cursor should advance to 1");
        assert!(result.has_more, "Should have more items");
        assert_eq!(result.total, 10);

        // Continue pagination to verify cursor advances correctly
        let result2 = paginate_items(&items, 1, limit);
        assert_eq!(result2.items.len(), 1);
        assert_eq!(result2.next_cursor, Some(2));
    }

    #[rstest]
    fn test_pagination_without_limit_clamp_would_cause_infinite_loop() {
        // This test demonstrates the problem that the fix addresses.
        // With limit=0 (without clamping), pagination would get stuck in an infinite loop.

        let items: Vec<i32> = (1..=5).collect();

        // Simulate what happens WITHOUT the clamp fix
        let unclamped_limit: usize = 0;
        let result = paginate_items(&items, 0, unclamped_limit);

        // With limit=0: no items returned, but has_more=true and next_cursor=Some(0)
        // This creates an infinite loop: cursor never advances, but has_more stays true
        assert!(result.items.is_empty(), "No items returned with limit=0");
        assert!(
            result.has_more,
            "has_more is true because next_offset(0) < total(5)"
        );
        assert_eq!(
            result.next_cursor,
            Some(0),
            "next_cursor stays at 0, causing infinite loop"
        );

        // The infinite loop scenario:
        // 1. Client requests page with cursor=0, limit=0
        // 2. Server returns: items=[], has_more=true, next_cursor=Some(0)
        // 3. Client requests next page with cursor=0, limit=0
        // 4. Same result - infinite loop!
        //
        // The clamp(1, MAX) fix prevents this by ensuring limit >= 1
    }
}
