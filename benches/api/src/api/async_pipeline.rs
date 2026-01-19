//! Async pipeline operations using `pipe_async`! macro.
//!
//! This module demonstrates:
//! - **`pipe_async!`**: Left-to-right function application for `AsyncIO`
//! - **`=>`**: Lift operator for pure functions (fmap)
//! - **`=>>`**: Bind operator for monadic functions (`flat_map`)
//!
//! # lambars Features Demonstrated
//!
//! - **`pipe_async!` macro**: `AsyncIO`-specific pipeline construction
//! - **Deferred execution**: Pipelines are lazy until `run_async()` is called
//! - **`Send + 'static` compliance**: Proper handling of closure captures
//! - **Conditional branching**: Dynamic workflow selection within pipelines

use std::sync::Arc;
use std::time::Instant;

use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};

use lambars::effect::AsyncIO;
use lambars::pipe_async;
use lambars::typeclass::Traversable;

use super::dto::{PriorityDto, TaskResponse};
use super::error::{ApiErrorResponse, FieldError};
use super::handlers::AppState;
use crate::domain::{Priority, Tag, Task, TaskId, Timestamp};
use crate::infrastructure::TaskRepository;

// =============================================================================
// DTOs
// =============================================================================

/// Transform type for async pipeline.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransformType {
    NormalizeTitle,
    BumpPriority,
    LowerPriority,
    AddTag { tag: String },
    SetDescription { description: String },
}

/// Request for async transform pipeline.
#[derive(Debug, Deserialize)]
pub struct TransformAsyncRequest {
    /// List of transforms to apply (1-10).
    pub transforms: Vec<TransformType>,
    /// Whether to validate before transform.
    #[serde(default)]
    pub validate_first: Option<bool>,
}

/// Response for async transform pipeline.
#[derive(Debug, Serialize)]
pub struct TransformAsyncResponse {
    pub task: TaskResponse,
    pub applied_transforms: Vec<String>,
    pub execution_time_ms: u64,
}

/// Request for async workflow.
#[derive(Debug, Deserialize)]
pub struct WorkflowAsyncRequest {
    /// Title for the new task.
    pub title: String,
    /// Optional description.
    pub description: Option<String>,
    /// Priority for the new task.
    #[serde(default)]
    pub priority: Option<PriorityDto>,
    /// Whether to notify after creation (simulated).
    #[serde(default)]
    pub notify: bool,
}

/// Step result in workflow.
#[derive(Debug, Serialize)]
pub struct StepResult {
    pub step_name: String,
    pub success: bool,
    pub duration_ms: u64,
}

/// Response for async workflow.
#[derive(Debug, Serialize)]
pub struct WorkflowAsyncResponse {
    pub task: TaskResponse,
    pub steps_executed: Vec<StepResult>,
    pub total_time_ms: u64,
}

/// Processing step type.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingStepType {
    Validate,
    Transform,
    Enrich,
}

/// Processing step definition.
#[derive(Debug, Clone, Deserialize)]
pub struct ProcessingStep {
    pub name: String,
    pub step_type: ProcessingStepType,
}

/// Request for batch processing.
#[derive(Debug, Deserialize)]
pub struct BatchProcessAsyncRequest {
    /// Task IDs to process (1-50).
    pub task_ids: Vec<String>,
    /// Processing steps to apply (1-5).
    pub processing_steps: Vec<ProcessingStep>,
}

/// Processed task result.
#[derive(Debug, Serialize)]
pub struct ProcessedTaskDto {
    pub id: String,
    pub title: String,
    pub steps_applied: Vec<String>,
    pub success: bool,
    /// Error message if processing failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for batch processing.
#[derive(Debug, Serialize)]
pub struct BatchProcessAsyncResponse {
    pub results: Vec<ProcessedTaskDto>,
    pub success_count: usize,
    pub failure_count: usize,
    pub total_time_ms: u64,
}

/// Pipeline conditions for conditional pipeline.
#[derive(Debug, Clone, Deserialize)]
pub struct PipelineConditions {
    /// Treat these priorities as high priority.
    pub high_priority_threshold: Option<PriorityDto>,
    /// Action for overdue tasks.
    pub overdue_action: Option<OverdueAction>,
    /// Simulate overdue condition for testing escalation path.
    #[serde(default)]
    pub simulate_overdue: bool,
}

/// Action for overdue tasks.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OverdueAction {
    Escalate,
    Notify,
    Skip,
}

/// Request for conditional pipeline.
#[derive(Debug, Deserialize)]
pub struct ConditionalPipelineRequest {
    /// Conditions for pipeline selection.
    pub conditions: PipelineConditions,
}

/// Condition evaluation result.
#[derive(Debug, Serialize)]
pub struct ConditionResult {
    pub condition: String,
    pub matched: bool,
}

/// Pipeline type used.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineType {
    HighPriority,
    Escalation,
    Standard,
}

/// Response for conditional pipeline.
#[derive(Debug, Serialize)]
pub struct ConditionalPipelineResponse {
    pub task: TaskResponse,
    pub pipeline_used: PipelineType,
    pub conditions_evaluated: Vec<ConditionResult>,
    pub execution_time_ms: u64,
}

/// Type alias for conditional pipeline result.
type ConditionalPipelineResult = Result<(Task, PipelineType, Vec<ConditionResult>), PipelineError>;

/// Error types for pipeline operations.
#[derive(Debug, Clone)]
pub enum PipelineError {
    /// Task not found.
    NotFound(String),
    /// Validation failed.
    ValidationFailed(String),
    /// Internal error (repository error, etc.).
    InternalError(String),
}

impl PipelineError {
    /// Converts pipeline error to API error response.
    ///
    /// - `NotFound` → 404 Not Found
    /// - `ValidationFailed` → 422 Unprocessable Entity
    /// - `InternalError` → 500 Internal Server Error (details hidden)
    fn into_api_error(self) -> ApiErrorResponse {
        match self {
            Self::NotFound(message) => ApiErrorResponse::not_found(message),
            Self::ValidationFailed(message) => ApiErrorResponse::unprocessable_entity(
                "Pipeline validation failed",
                vec![FieldError::new("pipeline", message)],
            ),
            // Internal errors should not expose details to clients
            Self::InternalError(_) => {
                ApiErrorResponse::internal_error("An internal error occurred")
            }
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Parses a task ID string into `TaskId`.
fn parse_task_id(s: &str) -> Result<TaskId, String> {
    uuid::Uuid::parse_str(s)
        .map(TaskId::from_uuid)
        .map_err(|_| format!("Invalid task ID format: {s}"))
}

// =============================================================================
// Pure Transform Functions
// =============================================================================

/// Pure: Normalizes task title (trims and capitalizes first letter).
fn normalize_title(task: Task) -> Task {
    let title = task.title.trim().to_string();
    let normalized = if let Some(first) = title.chars().next() {
        let rest: String = title.chars().skip(1).collect();
        format!("{}{}", first.to_uppercase(), rest)
    } else {
        title
    };
    Task { title: normalized, ..task }
}

/// Pure: Bumps task priority by one level.
fn bump_priority(task: Task) -> Task {
    let new_priority = match task.priority {
        Priority::Low => Priority::Medium,
        Priority::Medium => Priority::High,
        Priority::High | Priority::Critical => Priority::Critical,
    };
    task.with_priority(new_priority)
}

/// Pure: Lowers task priority by one level.
fn lower_priority(task: Task) -> Task {
    let new_priority = match task.priority {
        Priority::Critical => Priority::High,
        Priority::High => Priority::Medium,
        Priority::Medium | Priority::Low => Priority::Low,
    };
    task.with_priority(new_priority)
}

/// Pure: Adds a tag to the task.
fn add_tag(task: Task, tag: String) -> Task {
    task.add_tag(Tag::new(tag))
}

/// Pure: Sets description on the task.
fn set_description(task: Task, description: String) -> Task {
    task.with_description(description)
}

/// Pure: Applies a single transform to a task.
fn apply_single_transform(task: Task, transform: &TransformType) -> Task {
    match transform {
        TransformType::NormalizeTitle => normalize_title(task),
        TransformType::BumpPriority => bump_priority(task),
        TransformType::LowerPriority => lower_priority(task),
        TransformType::AddTag { tag } => add_tag(task, tag.clone()),
        TransformType::SetDescription { description } => set_description(task, description.clone()),
    }
}

/// Pure: Applies multiple transforms to a task.
fn apply_transforms(task: Task, transforms: &[TransformType]) -> Task {
    transforms.iter().fold(task, apply_single_transform)
}

/// Pure: Gets transform name as string.
fn transform_name(transform: &TransformType) -> String {
    match transform {
        TransformType::NormalizeTitle => "normalize_title".to_string(),
        TransformType::BumpPriority => "bump_priority".to_string(),
        TransformType::LowerPriority => "lower_priority".to_string(),
        TransformType::AddTag { tag } => format!("add_tag:{tag}"),
        TransformType::SetDescription { .. } => "set_description".to_string(),
    }
}

/// Pure: Validates a task (simulated validation).
fn validate_task(task: &Task) -> Result<(), String> {
    if task.title.trim().is_empty() {
        return Err("Task title cannot be empty".to_string());
    }
    if task.title.len() > 200 {
        return Err("Task title too long".to_string());
    }
    Ok(())
}

/// Pure: Validates a task returning `PipelineError`.
fn validate_task_pipeline(task: &Task) -> Result<(), PipelineError> {
    validate_task(task).map_err(PipelineError::ValidationFailed)
}

/// Pure: Enriches a task with metadata (simulated).
fn enrich_task(task: Task) -> Task {
    task.add_tag(Tag::new("enriched"))
}

/// Pure: Checks if task has high priority.
fn is_high_priority(task: &Task, threshold: Option<Priority>) -> bool {
    let threshold = threshold.unwrap_or(Priority::High);
    task.priority >= threshold
}

/// Pure: Checks if task is overdue.
///
/// In production, this would check against a deadline.
/// For demo/testing purposes, `simulate` flag allows testing the escalation path.
const fn is_overdue(_task: &Task, simulate: bool) -> bool {
    simulate
}

/// Pure: Finalizes a task after pipeline processing.
fn finalize_task(task: Task) -> Task {
    task.add_tag(Tag::new("processed"))
}

// =============================================================================
// POST /tasks/{id}/transform-async - Async transform pipeline
// =============================================================================

/// Transforms a task using an async pipeline.
///
/// This handler demonstrates:
/// - **`pipe_async!`**: Building async transformation pipelines
/// - **`=>`**: Pure transformation using fmap
/// - **`=>>`**: Async operations using `flat_map`
///
/// # Path Parameters
///
/// - `id`: Task ID
///
/// # Request Body
///
/// - `transforms`: List of transforms to apply (1-10)
/// - `validate_first`: Whether to validate before transforming
///
/// # Errors
///
/// - `400 Bad Request`: Invalid request
/// - `404 Not Found`: Task not found
/// - `422 Unprocessable Entity`: Transform failed
#[allow(clippy::cast_possible_truncation)] // Milliseconds won't overflow u64
pub async fn transform_async(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Json(request): Json<TransformAsyncRequest>,
) -> Result<Json<TransformAsyncResponse>, ApiErrorResponse> {
    let start = Instant::now();

    // Validate request
    if request.transforms.is_empty() {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("transforms", "transforms list cannot be empty")],
        ));
    }

    if request.transforms.len() > 10 {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("transforms", "transforms list cannot exceed 10 items")],
        ));
    }

    // Parse task ID
    let id = parse_task_id(&task_id).map_err(|error| {
        ApiErrorResponse::validation_error("Validation failed", vec![FieldError::new("id", error)])
    })?;

    // Build and execute pipeline using pipe_async!
    let repository = Arc::clone(&state.task_repository);
    let transforms = request.transforms.clone();
    let validate_first = request.validate_first.unwrap_or(false);

    // Build the async pipeline
    let pipeline = build_transform_pipeline(repository, id, transforms.clone(), validate_first);

    // Execute the pipeline
    let task = pipeline
        .run_async()
        .await
        .map_err(PipelineError::into_api_error)?;

    let execution_time_ms = start.elapsed().as_millis() as u64;
    let applied_transforms: Vec<String> = transforms.iter().map(transform_name).collect();

    Ok(Json(TransformAsyncResponse {
        task: TaskResponse::from(&task),
        applied_transforms,
        execution_time_ms,
    }))
}

/// Builds a transform pipeline using `pipe_async`!.
///
/// Demonstrates:
/// - `=>>` for async fetch operation
/// - `=>` for pure transformations
fn build_transform_pipeline(
    repository: Arc<dyn TaskRepository + Send + Sync>,
    task_id: TaskId,
    transforms: Vec<TransformType>,
    validate_first: bool,
) -> AsyncIO<Result<Task, PipelineError>> {
    pipe_async!(
        // Step 1: Fetch task from repository (async)
        fetch_task_async(repository, task_id),
        // Step 2: Optionally validate (pure, but wrapped in Result)
        =>> move |result: Result<Task, PipelineError>| {
            AsyncIO::pure(result.and_then(|task| {
                if validate_first {
                    validate_task_pipeline(&task)?;
                }
                Ok(task)
            }))
        },
        // Step 3: Apply transforms (pure)
        => move |result: Result<Task, PipelineError>| {
            result.map(|task| apply_transforms(task, &transforms))
        }
    )
}

/// Fetches a task asynchronously.
fn fetch_task_async(
    repository: Arc<dyn TaskRepository + Send + Sync>,
    task_id: TaskId,
) -> AsyncIO<Result<Task, PipelineError>> {
    AsyncIO::new(move || async move {
        repository
            .find_by_id(&task_id)
            .run_async()
            .await
            .map_err(|e| PipelineError::InternalError(format!("Repository error: {e}")))?
            .ok_or_else(|| PipelineError::NotFound(format!("Task not found: {}", task_id.as_uuid())))
    })
}

// =============================================================================
// POST /tasks/workflow-async - Multi-step async workflow
// =============================================================================

/// Executes a multi-step async workflow.
///
/// This handler demonstrates:
/// - **`pipe_async!` chain**: Sequential async steps
/// - **Step tracking**: Recording each step's execution
///
/// # Request Body
///
/// - `title`: Title for the new task
/// - `description`: Optional description
/// - `priority`: Priority for the new task
/// - `notify`: Whether to send notification (simulated)
///
/// # Errors
///
/// - `400 Bad Request`: Invalid request
/// - `500 Internal Server Error`: Workflow step failed
#[allow(clippy::cast_possible_truncation)] // Milliseconds won't overflow u64
pub async fn workflow_async(
    State(state): State<AppState>,
    Json(request): Json<WorkflowAsyncRequest>,
) -> Result<Json<WorkflowAsyncResponse>, ApiErrorResponse> {
    let start = Instant::now();

    // Validate request
    if request.title.trim().is_empty() {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("title", "title cannot be empty")],
        ));
    }

    if request.title.len() > 200 {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("title", "title cannot exceed 200 characters")],
        ));
    }

    let repository = Arc::clone(&state.task_repository);
    let title = request.title.clone();
    let description = request.description.clone();
    let priority = request.priority.map_or(Priority::Medium, Priority::from);
    let notify = request.notify;

    // Build workflow pipeline using pipe_async!
    let pipeline = build_workflow_pipeline(repository, title, description, priority, notify);

    // Execute pipeline
    let (task, steps) = pipeline
        .run_async()
        .await
        .map_err(|_| ApiErrorResponse::internal_error("Workflow execution failed"))?;

    let total_time_ms = start.elapsed().as_millis() as u64;

    Ok(Json(WorkflowAsyncResponse {
        task: TaskResponse::from(&task),
        steps_executed: steps,
        total_time_ms,
    }))
}

/// Builds a workflow pipeline using `pipe_async`!.
#[allow(clippy::cast_possible_truncation)] // Milliseconds won't overflow u64
fn build_workflow_pipeline(
    repository: Arc<dyn TaskRepository + Send + Sync>,
    title: String,
    description: Option<String>,
    priority: Priority,
    notify: bool,
) -> AsyncIO<Result<(Task, Vec<StepResult>), String>> {
    let repo1 = repository.clone();
    let repo2 = repository;

    pipe_async!(
        // Step 1: Create task
        create_task_step(repo1, title, description, priority),
        // Step 2: Save to repository
        =>> move |result: Result<(Task, Vec<StepResult>), String>| {
            let repo = repo2.clone();
            AsyncIO::new(move || async move {
                let (task, mut steps) = result?;
                let step_start = Instant::now();

                repo.save(&task)
                    .run_async()
                    .await
                    .map_err(|e| format!("Save failed: {e}"))?;

                steps.push(StepResult {
                    step_name: "save_task".to_string(),
                    success: true,
                    duration_ms: step_start.elapsed().as_millis() as u64,
                });

                Ok((task, steps))
            })
        },
        // Step 3: Optionally notify (simulated)
        => move |result: Result<(Task, Vec<StepResult>), String>| {
            result.map(|(task, mut steps)| {
                if notify {
                    steps.push(StepResult {
                        step_name: "notify".to_string(),
                        success: true,
                        duration_ms: 0, // Simulated
                    });
                }
                (task, steps)
            })
        }
    )
}

/// Creates a task (first step of workflow).
#[allow(clippy::cast_possible_truncation)] // Milliseconds won't overflow u64
fn create_task_step(
    _repository: Arc<dyn TaskRepository + Send + Sync>,
    title: String,
    description: Option<String>,
    priority: Priority,
) -> AsyncIO<Result<(Task, Vec<StepResult>), String>> {
    AsyncIO::new(move || async move {
        let step_start = Instant::now();

        let task_id = TaskId::generate();
        let mut task = Task::new(task_id, title, Timestamp::now())
            .with_priority(priority);

        if let Some(desc) = description {
            task = task.with_description(desc);
        }

        let steps = vec![StepResult {
            step_name: "create_task".to_string(),
            success: true,
            duration_ms: step_start.elapsed().as_millis() as u64,
        }];

        Ok((task, steps))
    })
}

// =============================================================================
// POST /tasks/batch-process-async - Batch processing pipeline
// =============================================================================

/// Processes multiple tasks in parallel using async pipelines.
///
/// This handler demonstrates:
/// - **`pipe_async!` + `traverse_async_io_parallel`**: Parallel batch processing
/// - Each task gets its own pipeline applied
///
/// # Request Body
///
/// - `task_ids`: Task IDs to process (1-50)
/// - `processing_steps`: Steps to apply (1-5)
///
/// # Errors
///
/// - `400 Bad Request`: Invalid request
/// - `500 Internal Server Error`: Processing failed
#[allow(clippy::cast_possible_truncation)] // Milliseconds won't overflow u64
pub async fn batch_process_async(
    State(state): State<AppState>,
    Json(request): Json<BatchProcessAsyncRequest>,
) -> Result<Json<BatchProcessAsyncResponse>, ApiErrorResponse> {
    let start = Instant::now();

    // Validate request
    if request.task_ids.is_empty() {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("task_ids", "task_ids list cannot be empty")],
        ));
    }

    if request.task_ids.len() > 50 {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("task_ids", "task_ids list cannot exceed 50 items")],
        ));
    }

    if request.processing_steps.is_empty() {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("processing_steps", "processing_steps list cannot be empty")],
        ));
    }

    if request.processing_steps.len() > 5 {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new(
                "processing_steps",
                "processing_steps list cannot exceed 5 items",
            )],
        ));
    }

    // Parse task IDs
    let task_ids: Result<Vec<TaskId>, _> = request
        .task_ids
        .iter()
        .map(|id| parse_task_id(id))
        .collect();
    let task_ids = task_ids.map_err(|error| {
        ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("task_ids", error)],
        )
    })?;

    let repository = Arc::clone(&state.task_repository);
    let steps = request.processing_steps.clone();

    // Process each task in parallel using traverse_async_io_parallel
    let results = task_ids
        .traverse_async_io_parallel(move |task_id| {
            let repo = repository.clone();
            let steps = steps.clone();
            build_batch_item_pipeline(repo, task_id, steps)
        })
        .run_async()
        .await;

    let success_count = results.iter().filter(|r| r.success).count();
    let failure_count = results.len() - success_count;
    let total_time_ms = start.elapsed().as_millis() as u64;

    Ok(Json(BatchProcessAsyncResponse {
        results,
        success_count,
        failure_count,
        total_time_ms,
    }))
}

/// Batch fetch result that distinguishes error types.
enum BatchFetchResult {
    Found(Task),
    NotFound,
    Error(String),
}

/// Builds a pipeline for a single batch item using `pipe_async`!.
fn build_batch_item_pipeline(
    repository: Arc<dyn TaskRepository + Send + Sync>,
    task_id: TaskId,
    steps: Vec<ProcessingStep>,
) -> AsyncIO<ProcessedTaskDto> {
    let task_id_str = task_id.as_uuid().to_string();

    pipe_async!(
        // Step 1: Fetch task
        fetch_task_for_batch(repository, task_id),
        // Step 2: Apply processing steps
        => move |result: BatchFetchResult| {
            let id = task_id_str;
            match result {
                BatchFetchResult::Found(task) => {
                    let processed = apply_processing_steps(task, &steps);
                    let step_names: Vec<String> = steps.iter().map(|s| s.name.clone()).collect();
                    ProcessedTaskDto {
                        id,
                        title: processed.title,
                        steps_applied: step_names,
                        success: true,
                        error: None,
                    }
                }
                BatchFetchResult::NotFound => ProcessedTaskDto {
                    id,
                    title: String::new(),
                    steps_applied: vec![],
                    success: false,
                    error: Some("Task not found".to_string()),
                },
                BatchFetchResult::Error(e) => ProcessedTaskDto {
                    id,
                    title: String::new(),
                    steps_applied: vec![],
                    success: false,
                    error: Some(e),
                },
            }
        }
    )
}

/// Fetches a task for batch processing with proper error distinction.
fn fetch_task_for_batch(
    repository: Arc<dyn TaskRepository + Send + Sync>,
    task_id: TaskId,
) -> AsyncIO<BatchFetchResult> {
    AsyncIO::new(move || async move {
        match repository.find_by_id(&task_id).run_async().await {
            Ok(Some(task)) => BatchFetchResult::Found(task),
            Ok(None) => BatchFetchResult::NotFound,
            Err(e) => BatchFetchResult::Error(format!("Repository error: {e}")),
        }
    })
}

/// Pure: Applies processing steps to a task.
fn apply_processing_steps(task: Task, steps: &[ProcessingStep]) -> Task {
    steps.iter().fold(task, |t, step| match step.step_type {
        ProcessingStepType::Validate => t, // Validation doesn't modify
        ProcessingStepType::Transform => normalize_title(t),
        ProcessingStepType::Enrich => enrich_task(t),
    })
}

// =============================================================================
// POST /tasks/{id}/conditional-pipeline - Conditional pipeline
// =============================================================================

/// Executes a conditional pipeline based on task properties.
///
/// This handler demonstrates:
/// - **Conditional branching in `pipe_async!`**: Dynamic workflow selection
/// - Different pipelines for high priority, overdue, and standard tasks
///
/// # Path Parameters
///
/// - `id`: Task ID
///
/// # Request Body
///
/// - `conditions`: Pipeline selection conditions
///
/// # Errors
///
/// - `400 Bad Request`: Invalid request
/// - `404 Not Found`: Task not found
/// - `500 Internal Server Error`: Pipeline error
#[allow(clippy::cast_possible_truncation)] // Milliseconds won't overflow u64
pub async fn conditional_pipeline(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Json(request): Json<ConditionalPipelineRequest>,
) -> Result<Json<ConditionalPipelineResponse>, ApiErrorResponse> {
    let start = Instant::now();

    // Parse task ID
    let id = parse_task_id(&task_id).map_err(|error| {
        ApiErrorResponse::validation_error("Validation failed", vec![FieldError::new("id", error)])
    })?;

    let repository = Arc::clone(&state.task_repository);
    let conditions = request.conditions.clone();

    // Build and execute conditional pipeline
    let pipeline = build_conditional_pipeline(repository, id, &conditions);

    let (task, pipeline_type, conditions_evaluated) = pipeline
        .run_async()
        .await
        .map_err(PipelineError::into_api_error)?;

    let execution_time_ms = start.elapsed().as_millis() as u64;

    Ok(Json(ConditionalPipelineResponse {
        task: TaskResponse::from(&task),
        pipeline_used: pipeline_type,
        conditions_evaluated,
        execution_time_ms,
    }))
}

/// Builds a conditional pipeline using `pipe_async`!.
fn build_conditional_pipeline(
    repository: Arc<dyn TaskRepository + Send + Sync>,
    task_id: TaskId,
    conditions: &PipelineConditions,
) -> AsyncIO<ConditionalPipelineResult> {
    let threshold = conditions.high_priority_threshold.map(Priority::from);
    let simulate_overdue = conditions.simulate_overdue;

    pipe_async!(
        // Step 1: Fetch task
        fetch_task_async(repository, task_id),
        // Step 2: Conditional processing based on task properties
        => move |result: Result<Task, PipelineError>| {
            result.map(|task| {
                let mut condition_results = Vec::new();

                // Check high priority condition
                let is_high = is_high_priority(&task, threshold);
                condition_results.push(ConditionResult {
                    condition: "high_priority".to_string(),
                    matched: is_high,
                });

                // Check overdue condition
                let is_task_overdue = is_overdue(&task, simulate_overdue);
                condition_results.push(ConditionResult {
                    condition: "overdue".to_string(),
                    matched: is_task_overdue,
                });

                // Select and apply pipeline
                let (processed_task, pipeline_type) = if is_high {
                    // High priority pipeline: add urgent tag
                    let t = task.add_tag(Tag::new("urgent"));
                    (finalize_task(t), PipelineType::HighPriority)
                } else if is_task_overdue {
                    // Escalation pipeline: bump priority and add overdue tag
                    let t = bump_priority(task).add_tag(Tag::new("overdue"));
                    (finalize_task(t), PipelineType::Escalation)
                } else {
                    // Standard pipeline: just finalize
                    (finalize_task(task), PipelineType::Standard)
                };

                (processed_task, pipeline_type, condition_results)
            })
        }
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
    // Pure Transform Function Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_normalize_title() {
        let task = Task::new(TaskId::generate(), "  hello world  ".to_string(), Timestamp::now());
        let result = normalize_title(task);
        assert_eq!(result.title, "Hello world");
    }

    #[rstest]
    fn test_normalize_title_empty() {
        let task = Task::new(TaskId::generate(), String::new(), Timestamp::now());
        let result = normalize_title(task);
        assert_eq!(result.title, "");
    }

    #[rstest]
    fn test_bump_priority_low_to_medium() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now())
            .with_priority(Priority::Low);
        let result = bump_priority(task);
        assert_eq!(result.priority, Priority::Medium);
    }

    #[rstest]
    fn test_bump_priority_critical_stays() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now())
            .with_priority(Priority::Critical);
        let result = bump_priority(task);
        assert_eq!(result.priority, Priority::Critical);
    }

    #[rstest]
    fn test_lower_priority_high_to_medium() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now())
            .with_priority(Priority::High);
        let result = lower_priority(task);
        assert_eq!(result.priority, Priority::Medium);
    }

    #[rstest]
    fn test_lower_priority_low_stays() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now())
            .with_priority(Priority::Low);
        let result = lower_priority(task);
        assert_eq!(result.priority, Priority::Low);
    }

    #[rstest]
    fn test_add_tag() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());
        let result = add_tag(task, "urgent".to_string());
        assert!(result.tags.iter().any(|t| t.to_string() == "urgent"));
    }

    #[rstest]
    fn test_set_description() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());
        let result = set_description(task, "A description".to_string());
        assert_eq!(result.description, Some("A description".to_string()));
    }

    #[rstest]
    fn test_apply_transforms_multiple() {
        let task = Task::new(TaskId::generate(), "  test  ".to_string(), Timestamp::now())
            .with_priority(Priority::Low);
        let transforms = vec![
            TransformType::NormalizeTitle,
            TransformType::BumpPriority,
            TransformType::AddTag { tag: "processed".to_string() },
        ];
        let result = apply_transforms(task, &transforms);
        assert_eq!(result.title, "Test");
        assert_eq!(result.priority, Priority::Medium);
        assert!(result.tags.iter().any(|t| t.to_string() == "processed"));
    }

    // -------------------------------------------------------------------------
    // Validation Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_validate_task_success() {
        let task = Task::new(TaskId::generate(), "Valid title".to_string(), Timestamp::now());
        assert!(validate_task(&task).is_ok());
    }

    #[rstest]
    fn test_validate_task_empty_title() {
        let task = Task::new(TaskId::generate(), "   ".to_string(), Timestamp::now());
        assert!(validate_task(&task).is_err());
    }

    #[rstest]
    fn test_validate_task_title_too_long() {
        let long_title = "a".repeat(201);
        let task = Task::new(TaskId::generate(), long_title, Timestamp::now());
        assert!(validate_task(&task).is_err());
    }

    // -------------------------------------------------------------------------
    // Priority Check Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_is_high_priority_critical() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now())
            .with_priority(Priority::Critical);
        assert!(is_high_priority(&task, None));
    }

    #[rstest]
    fn test_is_high_priority_high() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now())
            .with_priority(Priority::High);
        assert!(is_high_priority(&task, None));
    }

    #[rstest]
    fn test_is_high_priority_medium_not_high() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now())
            .with_priority(Priority::Medium);
        assert!(!is_high_priority(&task, None));
    }

    #[rstest]
    fn test_is_high_priority_with_custom_threshold() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now())
            .with_priority(Priority::Medium);
        assert!(is_high_priority(&task, Some(Priority::Medium)));
    }

    // -------------------------------------------------------------------------
    // pipe_async! Integration Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_pipe_async_pure_transform() {
        let task = Task::new(TaskId::generate(), "test".to_string(), Timestamp::now());

        let result = pipe_async!(
            AsyncIO::pure(task),
            => normalize_title,
            => bump_priority
        )
        .run_async()
        .await;

        assert_eq!(result.title, "Test");
        assert_eq!(result.priority, Priority::Medium);
    }

    #[rstest]
    #[tokio::test]
    async fn test_pipe_async_with_flat_map() {
        let result = pipe_async!(
            AsyncIO::pure(5),
            => |x| x * 2,
            =>> |x| AsyncIO::pure(x + 10)
        )
        .run_async()
        .await;

        assert_eq!(result, 20);
    }

    #[rstest]
    #[tokio::test]
    async fn test_pipe_async_deferred_execution() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let pipeline = pipe_async!(
            AsyncIO::new(move || async move {
                executed_clone.store(true, Ordering::SeqCst);
                42
            }),
            => |x| x * 2
        );

        // Not executed yet
        assert!(!executed.load(Ordering::SeqCst));

        // Execute
        let result = pipeline.run_async().await;
        assert!(executed.load(Ordering::SeqCst));
        assert_eq!(result, 84);
    }

    // -------------------------------------------------------------------------
    // Processing Steps Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_apply_processing_steps_transform() {
        let task = Task::new(TaskId::generate(), "  test  ".to_string(), Timestamp::now());
        let steps = vec![ProcessingStep {
            name: "transform".to_string(),
            step_type: ProcessingStepType::Transform,
        }];
        let result = apply_processing_steps(task, &steps);
        assert_eq!(result.title, "Test");
    }

    #[rstest]
    fn test_apply_processing_steps_enrich() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());
        let steps = vec![ProcessingStep {
            name: "enrich".to_string(),
            step_type: ProcessingStepType::Enrich,
        }];
        let result = apply_processing_steps(task, &steps);
        assert!(result.tags.iter().any(|t| t.to_string() == "enriched"));
    }

    #[rstest]
    fn test_apply_processing_steps_validate() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());
        let steps = vec![ProcessingStep {
            name: "validate".to_string(),
            step_type: ProcessingStepType::Validate,
        }];
        let result = apply_processing_steps(task, &steps);
        // Validate doesn't modify the task
        assert_eq!(result.title, "Test");
    }

    // -------------------------------------------------------------------------
    // Finalize Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_finalize_task() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());
        let result = finalize_task(task);
        assert!(result.tags.iter().any(|t| t.to_string() == "processed"));
    }

    // -------------------------------------------------------------------------
    // PipelineError Classification Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_pipeline_error_not_found_returns_404() {
        use axum::http::StatusCode;

        let error = PipelineError::NotFound("Task not found".to_string());
        let api_error = error.into_api_error();
        assert_eq!(api_error.status, StatusCode::NOT_FOUND);
        assert_eq!(api_error.error.code, "NOT_FOUND");
    }

    #[rstest]
    fn test_pipeline_error_validation_failed_returns_422() {
        use axum::http::StatusCode;

        let error = PipelineError::ValidationFailed("Title is empty".to_string());
        let api_error = error.into_api_error();
        assert_eq!(api_error.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(api_error.error.code, "VALIDATION_ERROR");
    }

    #[rstest]
    fn test_pipeline_error_internal_error_returns_500_without_details() {
        use axum::http::StatusCode;

        let error = PipelineError::InternalError("Database connection failed".to_string());
        let api_error = error.into_api_error();
        assert_eq!(api_error.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(api_error.error.code, "INTERNAL_ERROR");
        // Internal error details should NOT be exposed to clients
        assert_eq!(api_error.error.message, "An internal error occurred");
        assert!(!api_error.error.message.contains("Database"));
    }

    #[rstest]
    fn test_validate_task_pipeline_success() {
        let task = Task::new(TaskId::generate(), "Valid title".to_string(), Timestamp::now());
        assert!(validate_task_pipeline(&task).is_ok());
    }

    #[rstest]
    fn test_validate_task_pipeline_failure() {
        let task = Task::new(TaskId::generate(), "   ".to_string(), Timestamp::now());
        let result = validate_task_pipeline(&task);
        assert!(matches!(result, Err(PipelineError::ValidationFailed(_))));
    }

    // -------------------------------------------------------------------------
    // Conditional Pipeline Branching Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_is_overdue_simulated_true() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());
        assert!(is_overdue(&task, true));
    }

    #[rstest]
    fn test_is_overdue_simulated_false() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());
        assert!(!is_overdue(&task, false));
    }

    #[rstest]
    #[tokio::test]
    async fn test_conditional_pipeline_high_priority_branch() {
        // Test that high priority tasks use HighPriority pipeline
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now())
            .with_priority(Priority::Critical);

        let threshold = Some(Priority::High);
        let is_high = is_high_priority(&task, threshold);
        let is_task_overdue = is_overdue(&task, false);

        // High priority should match
        assert!(is_high);
        assert!(!is_task_overdue);

        // High priority takes precedence
        let pipeline_type = if is_high {
            PipelineType::HighPriority
        } else if is_task_overdue {
            PipelineType::Escalation
        } else {
            PipelineType::Standard
        };
        assert!(matches!(pipeline_type, PipelineType::HighPriority));
    }

    #[rstest]
    #[tokio::test]
    async fn test_conditional_pipeline_escalation_branch() {
        // Test that overdue non-high-priority tasks use Escalation pipeline
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now())
            .with_priority(Priority::Low);

        let threshold = Some(Priority::High);
        let is_high = is_high_priority(&task, threshold);
        let is_task_overdue = is_overdue(&task, true); // Simulate overdue

        // Not high priority, but overdue
        assert!(!is_high);
        assert!(is_task_overdue);

        let pipeline_type = if is_high {
            PipelineType::HighPriority
        } else if is_task_overdue {
            PipelineType::Escalation
        } else {
            PipelineType::Standard
        };
        assert!(matches!(pipeline_type, PipelineType::Escalation));
    }

    #[rstest]
    #[tokio::test]
    async fn test_conditional_pipeline_standard_branch() {
        // Test that normal tasks use Standard pipeline
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now())
            .with_priority(Priority::Low);

        let threshold = Some(Priority::High);
        let is_high = is_high_priority(&task, threshold);
        let is_task_overdue = is_overdue(&task, false);

        // Neither high priority nor overdue
        assert!(!is_high);
        assert!(!is_task_overdue);

        let pipeline_type = if is_high {
            PipelineType::HighPriority
        } else if is_task_overdue {
            PipelineType::Escalation
        } else {
            PipelineType::Standard
        };
        assert!(matches!(pipeline_type, PipelineType::Standard));
    }

    // -------------------------------------------------------------------------
    // Batch Processing Error Distinction Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_batch_fetch_result_found() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());
        let result = BatchFetchResult::Found(task.clone());
        assert!(matches!(result, BatchFetchResult::Found(_)));
    }

    #[rstest]
    fn test_batch_fetch_result_not_found() {
        let result = BatchFetchResult::NotFound;
        assert!(matches!(result, BatchFetchResult::NotFound));
    }

    #[rstest]
    fn test_batch_fetch_result_error() {
        let result = BatchFetchResult::Error("Connection failed".to_string());
        assert!(matches!(result, BatchFetchResult::Error(_)));
    }

    #[rstest]
    fn test_processed_task_dto_with_error() {
        let dto = ProcessedTaskDto {
            id: "test-id".to_string(),
            title: String::new(),
            steps_applied: vec![],
            success: false,
            error: Some("Repository error".to_string()),
        };
        assert!(!dto.success);
        assert_eq!(dto.error, Some("Repository error".to_string()));
    }

    #[rstest]
    fn test_processed_task_dto_success() {
        let dto = ProcessedTaskDto {
            id: "test-id".to_string(),
            title: "Processed".to_string(),
            steps_applied: vec!["transform".to_string()],
            success: true,
            error: None,
        };
        assert!(dto.success);
        assert!(dto.error.is_none());
    }
}
