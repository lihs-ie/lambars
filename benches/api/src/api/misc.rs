//! Miscellaneous lambars features demonstration.
//!
//! This module demonstrates:
//! - **`partial!`**: Partial function application macro
//! - **`ConcurrentLazy`**: Thread-safe lazy evaluation
//! - **`PersistentDeque`**: Persistent double-ended queue
//! - **`Sum`/`Product`**: Numeric monoids for aggregation
//! - **`Freer`**: Freer monad for DSL construction
//!
//! # lambars Features Demonstrated
//!
//! - **`partial!` macro**: Partial application with placeholder support
//! - **`ConcurrentLazy<T>`**: Thread-safe memoized lazy evaluation
//! - **`PersistentDeque<T>`**: Finger tree based persistent deque
//! - **`Sum<T>`/`Product<T>`**: Additive/multiplicative monoid wrappers
//! - **`Freer<I, A>`**: Free monad without Functor constraint

use std::any::Any;

use axum::Json;
use axum::extract::{Query, State};

use super::json_buffer::JsonResponse;
use serde::{Deserialize, Serialize};

use lambars::control::{ConcurrentLazy, Freer};
use lambars::partial;
use lambars::persistent::PersistentDeque;
use lambars::typeclass::{Monoid, Product, Semigroup, Sum};

use super::dto::TaskResponse;
use super::error::{ApiErrorResponse, FieldError};
use super::handlers::AppState;
use crate::domain::{Priority, Task, TaskId, TaskStatus, Timestamp};
use crate::infrastructure::Pagination;

// =============================================================================
// DTOs
// =============================================================================

/// Request for partial application.
#[derive(Debug, Deserialize)]
pub struct PartialApplyRequest {
    /// List of task IDs to process (1-100 items).
    pub task_ids: Vec<String>,
    /// Processing configuration.
    pub config: ProcessConfig,
    /// Operation type: score, transform, filter.
    pub operation: String,
}

/// Processing configuration for partial application.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProcessConfig {
    /// Multiplier for calculations.
    pub multiplier: i32,
    /// Weights for scoring.
    pub weights: Weights,
    /// Processing flags.
    #[serde(default)]
    pub flags: Vec<String>,
}

/// Weights for score calculation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Weights {
    /// Weight for complexity.
    pub complexity_weight: f64,
    /// Weight for priority.
    pub priority_weight: f64,
    /// Weight for urgency.
    pub urgency_weight: f64,
}

/// Processed task result.
#[derive(Debug, Serialize)]
pub struct ProcessedTaskDto {
    pub id: String,
    pub title: String,
    pub score: Option<f64>,
    pub transformed: bool,
    pub multiplier_applied: i32,
}

/// Response for partial application.
#[derive(Debug, Serialize)]
pub struct PartialApplyResponse {
    pub results: Vec<ProcessedTaskDto>,
    pub config_applied: ProcessConfig,
    pub operation: String,
}

/// Request for concurrent lazy evaluation.
#[derive(Debug, Deserialize)]
pub struct ConcurrentLazyRequest {
    /// Number of subsequent `force()` calls after the initial computation.
    /// Used to demonstrate that memoized calls are near-instant.
    #[serde(default = "default_subsequent_calls")]
    pub subsequent_calls: usize,
}

/// Default number of subsequent `force()` calls.
const fn default_subsequent_calls() -> usize {
    3
}

/// Statistics from expensive calculation.
#[derive(Debug, Clone, Serialize)]
pub struct StatsDto {
    pub total_tasks: usize,
    pub average_priority: f64,
    pub completion_rate: f64,
    pub priority_distribution: PriorityDistribution,
}

/// Priority distribution statistics.
#[derive(Debug, Clone, Serialize)]
pub struct PriorityDistribution {
    pub low: usize,
    pub medium: usize,
    pub high: usize,
    pub critical: usize,
}

/// Response for concurrent lazy evaluation.
#[derive(Debug, Serialize)]
pub struct ConcurrentLazyResponse {
    pub stats: StatsDto,
    /// Number of subsequent `force()` calls (after initial computation).
    pub subsequent_calls: usize,
    /// Time for first `force()` call in microseconds (actual computation).
    pub first_force_time_us: u64,
    /// Time for subsequent `force()` calls in microseconds (memoized - should be near zero).
    pub subsequent_force_time_us: u64,
    /// Note explaining the memoization behavior.
    pub memoization_note: String,
}

/// Deque operation type.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DequeOperationDto {
    PushFront { task_id: String },
    PushBack { task_id: String },
    PopFront,
    PopBack,
    PeekFront,
    PeekBack,
}

/// Request for deque operations.
#[derive(Debug, Deserialize)]
pub struct DequeOperationsRequest {
    /// Operations to execute (1-50 items).
    pub operations: Vec<DequeOperationDto>,
    /// Initial task IDs in the deque.
    #[serde(default)]
    pub initial_state: Vec<String>,
}

/// Response for deque operations.
#[derive(Debug, Serialize)]
pub struct DequeOperationsResponse {
    pub final_state: Vec<String>,
    pub popped_items: Vec<Option<String>>,
    pub operation_count: usize,
    pub structural_sharing_note: String,
}

/// Query parameters for numeric aggregation.
#[derive(Debug, Deserialize)]
pub struct AggregateNumericQuery {
    /// Field to aggregate: `priority`, `tag_count`, `title_length`.
    pub field: String,
    /// Aggregation type: sum, product, average.
    pub aggregation: String,
    /// Optional grouping: status, priority.
    #[serde(default)]
    pub group_by: Option<String>,
}

/// Response for numeric aggregation.
#[derive(Debug, Serialize)]
pub struct AggregateNumericResponse {
    pub result: f64,
    pub field: String,
    pub aggregation: String,
    pub count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<std::collections::HashMap<String, f64>>,
}

/// Workflow step for Freer monad.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowStepDto {
    CreateTask { title: String },
    UpdatePriority { task_id: String, priority: String },
    AddTag { task_id: String, tag: String },
    GetTask { task_id: String },
}

/// Request for Freer workflow.
#[derive(Debug, Deserialize)]
pub struct FreerWorkflowRequest {
    /// Workflow steps to execute (1-20 items).
    pub steps: Vec<WorkflowStepDto>,
    /// Execution mode: production, `dry_run`, test.
    pub execution_mode: String,
}

/// Response for Freer workflow.
#[derive(Debug, Serialize)]
pub struct FreerWorkflowResponse {
    pub result: Option<TaskResponse>,
    pub operations_executed: Vec<String>,
    pub execution_mode: String,
}

// =============================================================================
// Pure Functions
// =============================================================================

/// Pure: Parses a task ID string into `TaskId`.
fn parse_task_id(s: &str) -> Result<TaskId, String> {
    uuid::Uuid::parse_str(s)
        .map(TaskId::from_uuid)
        .map_err(|_| format!("Invalid task ID format: {s}"))
}

/// Pure: Calculates a score for a task using weights.
#[allow(clippy::cast_precision_loss)]
fn calculate_task_score(
    complexity_weight: f64,
    priority_weight: f64,
    urgency_weight: f64,
    task: &Task,
) -> f64 {
    let complexity = task.title.len() as f64 / 10.0;
    let priority_value = priority_to_score(task.priority);
    let urgency = if task.status == TaskStatus::InProgress {
        1.5
    } else {
        1.0
    };

    complexity * complexity_weight + priority_value * priority_weight + urgency * urgency_weight
}

/// Pure: Converts priority to numeric score.
const fn priority_to_score(priority: Priority) -> f64 {
    match priority {
        Priority::Low => 1.0,
        Priority::Medium => 2.0,
        Priority::High => 3.0,
        Priority::Critical => 4.0,
    }
}

/// Pure: Transforms a task using multiplier.
fn transform_task_with_multiplier(multiplier: i32, task: Task) -> Task {
    let new_title = format!("[x{}] {}", multiplier, task.title);
    Task {
        title: new_title,
        ..task
    }
}

/// Pure: Creates a partial score calculator using `partial!` macro.
fn create_score_calculator(
    weights: &Weights,
) -> impl Fn(&Task) -> f64 + Clone + Send + Sync + 'static {
    let complexity_weight = weights.complexity_weight;
    let priority_weight = weights.priority_weight;
    let urgency_weight = weights.urgency_weight;

    partial!(
        calculate_task_score,
        complexity_weight,
        priority_weight,
        urgency_weight,
        __
    )
}

/// Pure: Creates a partial transformer using `partial!` macro.
fn create_task_transformer(
    multiplier: i32,
) -> impl Fn(Task) -> Task + Clone + Send + Sync + 'static {
    partial!(transform_task_with_multiplier, multiplier, __)
}

/// Pure: Calculates statistics from tasks.
#[allow(clippy::cast_precision_loss)]
fn calculate_stats(tasks: &[Task]) -> StatsDto {
    if tasks.is_empty() {
        return StatsDto {
            total_tasks: 0,
            average_priority: 0.0,
            completion_rate: 0.0,
            priority_distribution: PriorityDistribution {
                low: 0,
                medium: 0,
                high: 0,
                critical: 0,
            },
        };
    }

    let total = tasks.len();

    let priority_sum: f64 = tasks.iter().map(|t| priority_to_score(t.priority)).sum();
    let average_priority = priority_sum / total as f64;

    let completed = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Completed)
        .count();
    let completion_rate = completed as f64 / total as f64;

    let mut distribution = PriorityDistribution {
        low: 0,
        medium: 0,
        high: 0,
        critical: 0,
    };
    for task in tasks {
        match task.priority {
            Priority::Low => distribution.low += 1,
            Priority::Medium => distribution.medium += 1,
            Priority::High => distribution.high += 1,
            Priority::Critical => distribution.critical += 1,
        }
    }

    StatsDto {
        total_tasks: total,
        average_priority,
        completion_rate,
        priority_distribution: distribution,
    }
}

/// Pure: Applies deque operation and returns result.
fn apply_deque_operation(
    deque: PersistentDeque<String>,
    operation: &DequeOperationDto,
) -> (PersistentDeque<String>, Option<String>) {
    match operation {
        DequeOperationDto::PushFront { task_id } => (deque.push_front(task_id.clone()), None),
        DequeOperationDto::PushBack { task_id } => (deque.push_back(task_id.clone()), None),
        DequeOperationDto::PopFront => deque
            .pop_front()
            .map_or((deque, None), |(new_deque, item)| (new_deque, Some(item))),
        DequeOperationDto::PopBack => deque
            .pop_back()
            .map_or((deque, None), |(new_deque, item)| (new_deque, Some(item))),
        DequeOperationDto::PeekFront => (deque.clone(), deque.front().cloned()),
        DequeOperationDto::PeekBack => (deque.clone(), deque.back().cloned()),
    }
}

/// Pure: Extracts numeric value from task for aggregation.
#[allow(clippy::cast_precision_loss)]
fn extract_numeric_value(task: &Task, field: &str) -> Option<f64> {
    match field {
        "priority" => Some(priority_to_score(task.priority)),
        "tag_count" => Some(task.tags.len() as f64),
        "title_length" => Some(task.title.len() as f64),
        _ => None,
    }
}

/// Pure: Aggregates values using Sum monoid.
fn aggregate_with_sum(values: &[f64]) -> f64 {
    values
        .iter()
        .map(|&v| Sum::new(v))
        .fold(Sum::empty(), Semigroup::combine)
        .into_inner()
}

/// Pure: Aggregates values using Product monoid.
fn aggregate_with_product(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 1.0;
    }
    values
        .iter()
        .map(|&v| Product::new(v))
        .fold(Product::new(1.0), Semigroup::combine)
        .into_inner()
}

/// Pure: Calculates average from values.
#[allow(clippy::cast_precision_loss)]
fn aggregate_with_average(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    aggregate_with_sum(values) / values.len() as f64
}

/// Pure: Parses priority from string.
fn parse_priority(priority: &str) -> Option<Priority> {
    match priority.to_lowercase().as_str() {
        "low" => Some(Priority::Low),
        "medium" => Some(Priority::Medium),
        "high" => Some(Priority::High),
        "critical" => Some(Priority::Critical),
        _ => None,
    }
}

// =============================================================================
// Freer Monad DSL
// =============================================================================

/// Commands for task workflow DSL.
#[derive(Debug)]
enum TaskCommand {
    Create { title: String },
    UpdatePriority { task_id: String, priority: Priority },
    AddTag { task_id: String, tag: String },
    GetTask { task_id: String },
}

/// Creates a task creation command.
///
/// # Panics
///
/// Panics if the interpreter returns a non-Task value.
fn create_task_command(title: String) -> Freer<TaskCommand, Task> {
    Freer::<TaskCommand, Task>::lift_instruction(TaskCommand::Create { title }, |result| {
        *result.downcast::<Task>().expect("Create must return Task")
    })
}

/// Creates a priority update command.
///
/// # Panics
///
/// Panics if the interpreter returns a non-Task value.
fn update_priority_command(task_id: String, priority: Priority) -> Freer<TaskCommand, Task> {
    Freer::<TaskCommand, Task>::lift_instruction(
        TaskCommand::UpdatePriority { task_id, priority },
        |result| {
            *result
                .downcast::<Task>()
                .expect("UpdatePriority must return Task")
        },
    )
}

/// Creates an add tag command.
///
/// # Panics
///
/// Panics if the interpreter returns a non-Task value.
fn add_tag_command(task_id: String, tag: String) -> Freer<TaskCommand, Task> {
    Freer::<TaskCommand, Task>::lift_instruction(TaskCommand::AddTag { task_id, tag }, |result| {
        *result.downcast::<Task>().expect("AddTag must return Task")
    })
}

/// Creates a get task command.
///
/// # Panics
///
/// Panics if the interpreter returns a non-Task value.
fn get_task_command(task_id: String) -> Freer<TaskCommand, Task> {
    Freer::<TaskCommand, Task>::lift_instruction(TaskCommand::GetTask { task_id }, |result| {
        *result.downcast::<Task>().expect("GetTask must return Task")
    })
}

/// Pure: Builds a Freer workflow from steps.
fn build_workflow(steps: &[WorkflowStepDto]) -> Option<Freer<TaskCommand, Task>> {
    if steps.is_empty() {
        return None;
    }

    let first = match &steps[0] {
        WorkflowStepDto::CreateTask { title } => create_task_command(title.clone()),
        WorkflowStepDto::UpdatePriority { task_id, priority } => {
            let priority = parse_priority(priority)?;
            update_priority_command(task_id.clone(), priority)
        }
        WorkflowStepDto::AddTag { task_id, tag } => add_tag_command(task_id.clone(), tag.clone()),
        WorkflowStepDto::GetTask { task_id } => get_task_command(task_id.clone()),
    };

    let workflow = steps[1..].iter().fold(first, |acc, step| match step {
        WorkflowStepDto::CreateTask { title } => {
            let title = title.clone();
            acc.then(create_task_command(title))
        }
        WorkflowStepDto::UpdatePriority { task_id, priority } => {
            let task_id = task_id.clone();
            if let Some(priority) = parse_priority(priority) {
                acc.then(update_priority_command(task_id, priority))
            } else {
                acc
            }
        }
        WorkflowStepDto::AddTag { task_id, tag } => {
            let task_id = task_id.clone();
            let tag = tag.clone();
            acc.then(add_tag_command(task_id, tag))
        }
        WorkflowStepDto::GetTask { task_id } => {
            let task_id = task_id.clone();
            acc.then(get_task_command(task_id))
        }
    });

    Some(workflow)
}

/// Interprets workflow in dry-run mode (logs operations, returns dummy task).
///
/// Note: This is not a pure function as it generates new `TaskId` and `Timestamp`.
/// In a production system, these would be injected as dependencies.
fn interpret_dry_run(workflow: Freer<TaskCommand, Task>) -> (Task, Vec<String>) {
    let mut operations = Vec::new();
    let result = workflow.interpret(|command| {
        let op_description = match &command {
            TaskCommand::Create { title } => format!("CREATE: {title}"),
            TaskCommand::UpdatePriority { task_id, priority } => {
                format!("UPDATE_PRIORITY: {task_id} -> {priority:?}")
            }
            TaskCommand::AddTag { task_id, tag } => {
                format!("ADD_TAG: {task_id} + {tag}")
            }
            TaskCommand::GetTask { task_id } => format!("GET: {task_id}"),
        };
        operations.push(op_description);

        let dummy_task = Task::new(
            TaskId::generate(),
            "dry_run_task".to_string(),
            Timestamp::now(),
        );
        Box::new(dummy_task) as Box<dyn Any>
    });

    (result, operations)
}

// =============================================================================
// Handlers
// =============================================================================

/// POST /tasks/partial-apply
///
/// Demonstrates `partial!` macro for partial function application.
/// Creates specialized processors by fixing configuration parameters.
///
/// # Errors
///
/// Returns `ApiErrorResponse` when:
/// - `task_ids` is empty or has more than 100 items
/// - `operation` is not one of: score, transform, filter
/// - Repository operations fail
#[allow(clippy::future_not_send, clippy::too_many_lines)]
pub async fn partial_apply(
    State(state): State<AppState>,
    Json(request): Json<PartialApplyRequest>,
) -> Result<JsonResponse<PartialApplyResponse>, ApiErrorResponse> {
    // Validate request
    if request.task_ids.is_empty() || request.task_ids.len() > 100 {
        return Err(ApiErrorResponse::validation_error(
            "task_ids must have 1-100 items",
            vec![FieldError::new("task_ids", "Must have 1-100 items")],
        ));
    }

    let valid_operations = ["score", "transform", "filter"];
    if !valid_operations.contains(&request.operation.as_str()) {
        return Err(ApiErrorResponse::bad_request(
            "INVALID_OPERATION",
            format!(
                "Invalid operation: {}. Valid: {:?}",
                request.operation, valid_operations
            ),
        ));
    }

    // Parse and validate task IDs
    let mut task_ids = Vec::with_capacity(request.task_ids.len());
    let mut invalid_ids = Vec::new();

    for id in &request.task_ids {
        match parse_task_id(id) {
            Ok(task_id) => task_ids.push(task_id),
            Err(_) => invalid_ids.push(id.clone()),
        }
    }

    // Report invalid IDs as errors
    if !invalid_ids.is_empty() {
        return Err(ApiErrorResponse::validation_error(
            format!("Invalid task IDs: {invalid_ids:?}"),
            invalid_ids
                .iter()
                .map(|id| FieldError::new("task_ids", format!("Invalid UUID format: {id}")))
                .collect(),
        ));
    }

    // Fetch tasks from repository
    let all_tasks = state
        .task_repository
        .list(Pagination::default())
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    // Filter to only requested task IDs
    let tasks: Vec<Task> = all_tasks
        .items
        .into_iter()
        .filter(|t| task_ids.contains(&t.task_id))
        .collect();

    // Create partial functions using partial! macro
    let score_calculator = create_score_calculator(&request.config.weights);
    let transformer = create_task_transformer(request.config.multiplier);

    // Process tasks based on operation
    let results: Vec<ProcessedTaskDto> = match request.operation.as_str() {
        "score" => tasks
            .iter()
            .map(|task| ProcessedTaskDto {
                id: task.task_id.to_string(),
                title: task.title.clone(),
                score: Some(score_calculator(task)),
                transformed: false,
                multiplier_applied: 0,
            })
            .collect(),
        "transform" => tasks
            .into_iter()
            .map(|task| {
                let id = task.task_id.to_string();
                let transformed_task = transformer(task);
                ProcessedTaskDto {
                    id,
                    title: transformed_task.title,
                    score: None,
                    transformed: true,
                    multiplier_applied: request.config.multiplier,
                }
            })
            .collect(),
        "filter" => {
            let min_score = f64::from(request.config.multiplier);
            tasks
                .iter()
                .filter(|task| score_calculator(task) >= min_score)
                .map(|task| ProcessedTaskDto {
                    id: task.task_id.to_string(),
                    title: task.title.clone(),
                    score: Some(score_calculator(task)),
                    transformed: false,
                    multiplier_applied: 0,
                })
                .collect()
        }
        _ => Vec::new(),
    };

    Ok(JsonResponse(PartialApplyResponse {
        results,
        config_applied: request.config,
        operation: request.operation,
    }))
}

/// POST /tasks/concurrent-lazy
///
/// Demonstrates `ConcurrentLazy` for thread-safe lazy evaluation.
/// Calls `force()` multiple times to show that computation happens only once.
///
/// # Errors
///
/// Returns `ApiErrorResponse` when:
/// - `subsequent_calls` is greater than 100
/// - Repository operations fail
#[allow(clippy::future_not_send, clippy::cast_possible_truncation)]
pub async fn concurrent_lazy(
    State(state): State<AppState>,
    Json(request): Json<ConcurrentLazyRequest>,
) -> Result<JsonResponse<ConcurrentLazyResponse>, ApiErrorResponse> {
    // Validate request
    if request.subsequent_calls > 100 {
        return Err(ApiErrorResponse::validation_error(
            "subsequent_calls must be 0-100",
            vec![FieldError::new("subsequent_calls", "Must be 0-100")],
        ));
    }

    // Fetch all tasks from repository
    let all_tasks = state
        .task_repository
        .list(Pagination::default())
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    let tasks: Vec<Task> = all_tasks.items;

    // Create a ConcurrentLazy for the expensive calculation
    // The calculation will only happen once, regardless of how many times force() is called
    let lazy_stats = ConcurrentLazy::new(move || calculate_stats(&tasks));

    // First force() - this triggers the actual computation
    let first_start = std::time::Instant::now();
    let computed_stats = lazy_stats.force().clone();
    let first_force_time_us = first_start.elapsed().as_micros() as u64;

    // Subsequent force() calls - these return the memoized result (should be near-instant)
    let subsequent_start = std::time::Instant::now();
    for _ in 0..request.subsequent_calls {
        let _ = lazy_stats.force();
    }
    let subsequent_force_time_us = subsequent_start.elapsed().as_micros() as u64;

    Ok(JsonResponse(ConcurrentLazyResponse {
        stats: computed_stats,
        subsequent_calls: request.subsequent_calls,
        first_force_time_us,
        subsequent_force_time_us,
        memoization_note: format!(
            "First force() computed in {first_force_time_us}μs. \
             {subsequent_calls} subsequent calls took {subsequent_force_time_us}μs total (memoized).",
            subsequent_calls = request.subsequent_calls
        ),
    }))
}

/// POST /tasks/deque-operations
///
/// Demonstrates `PersistentDeque` for persistent double-ended queue operations.
/// All operations preserve previous versions through structural sharing.
///
/// # Errors
///
/// Returns `ApiErrorResponse` when `operations` is empty or has more than 50 items.
pub async fn deque_operations(
    Json(request): Json<DequeOperationsRequest>,
) -> Result<JsonResponse<DequeOperationsResponse>, ApiErrorResponse> {
    // Validate request
    if request.operations.is_empty() || request.operations.len() > 50 {
        return Err(ApiErrorResponse::validation_error(
            "operations must have 1-50 items",
            vec![FieldError::new("operations", "Must have 1-50 items")],
        ));
    }

    // Initialize deque from initial state
    let initial_deque: PersistentDeque<String> = request.initial_state.into_iter().collect();

    // Apply all operations
    let mut current_deque = initial_deque;
    let mut popped_items = Vec::new();

    for operation in &request.operations {
        let (new_deque, popped) = apply_deque_operation(current_deque, operation);
        current_deque = new_deque;
        if matches!(
            operation,
            DequeOperationDto::PopFront
                | DequeOperationDto::PopBack
                | DequeOperationDto::PeekFront
                | DequeOperationDto::PeekBack
        ) {
            popped_items.push(popped);
        }
    }

    // Collect final state
    let final_state: Vec<String> = current_deque.iter().cloned().collect();

    Ok(JsonResponse(DequeOperationsResponse {
        final_state,
        popped_items,
        operation_count: request.operations.len(),
        structural_sharing_note:
            "PersistentDeque uses structural sharing - original versions are preserved".to_string(),
    }))
}

/// GET /tasks/aggregate-numeric
///
/// Demonstrates `Sum` and `Product` monoids for numeric aggregation.
/// Uses monoidal operations for composable aggregation.
///
/// # Errors
///
/// Returns `ApiErrorResponse` when:
/// - `field` is not one of: priority, `tag_count`, `title_length`
/// - `aggregation` is not one of: sum, product, average
/// - Repository operations fail
#[allow(clippy::future_not_send, clippy::too_many_lines)]
pub async fn aggregate_numeric(
    State(state): State<AppState>,
    Query(query): Query<AggregateNumericQuery>,
) -> Result<JsonResponse<AggregateNumericResponse>, ApiErrorResponse> {
    // Validate field
    let valid_fields = ["priority", "tag_count", "title_length"];
    if !valid_fields.contains(&query.field.as_str()) {
        return Err(ApiErrorResponse::bad_request(
            "INVALID_FIELD",
            format!("Invalid field: {}. Valid: {:?}", query.field, valid_fields),
        ));
    }

    // Validate aggregation
    let valid_aggregations = ["sum", "product", "average"];
    if !valid_aggregations.contains(&query.aggregation.as_str()) {
        return Err(ApiErrorResponse::bad_request(
            "INVALID_AGGREGATION",
            format!(
                "Invalid aggregation: {}. Valid: {:?}",
                query.aggregation, valid_aggregations
            ),
        ));
    }

    // Fetch all tasks
    let all_tasks = state
        .task_repository
        .list(Pagination::default())
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    let tasks: Vec<Task> = all_tasks.items;

    // Extract numeric values
    let values: Vec<f64> = tasks
        .iter()
        .filter_map(|task| extract_numeric_value(task, &query.field))
        .collect();

    // Aggregate using monoids
    let result = match query.aggregation.as_str() {
        "sum" => aggregate_with_sum(&values),
        "product" => aggregate_with_product(&values),
        "average" => aggregate_with_average(&values),
        _ => 0.0,
    };

    // Handle grouping
    let groups = query.group_by.as_ref().map(|group_field| {
        let mut grouped: std::collections::HashMap<String, Vec<f64>> =
            std::collections::HashMap::new();

        for task in &tasks {
            let group_key = match group_field.as_str() {
                "status" => format!("{:?}", task.status),
                "priority" => format!("{:?}", task.priority),
                _ => "unknown".to_string(),
            };

            if let Some(value) = extract_numeric_value(task, &query.field) {
                grouped.entry(group_key).or_default().push(value);
            }
        }

        grouped
            .into_iter()
            .map(|(key, vals)| {
                let agg = match query.aggregation.as_str() {
                    "sum" => aggregate_with_sum(&vals),
                    "product" => aggregate_with_product(&vals),
                    "average" => aggregate_with_average(&vals),
                    _ => 0.0,
                };
                (key, agg)
            })
            .collect()
    });

    Ok(JsonResponse(AggregateNumericResponse {
        result,
        field: query.field,
        aggregation: query.aggregation,
        count: values.len(),
        groups,
    }))
}

/// POST /tasks/freer-workflow
///
/// Demonstrates `Freer` monad for DSL construction.
/// Builds workflows as data structures, then interprets them.
///
/// The Freer monad allows defining a DSL of commands that can be interpreted
/// in different ways:
/// - `dry_run`/`test`: Logs operations without persisting
/// - `production`: Executes commands (demo mode - uses in-memory state)
///
/// Note: This demo uses `Box<dyn Any>` for simplicity. In production,
/// a typed interpreter with proper error handling would be preferred.
///
/// # Errors
///
/// Returns `ApiErrorResponse` when:
/// - `steps` is empty or has more than 20 items
/// - `execution_mode` is not one of: production, `dry_run`, test
/// - Workflow build fails (e.g., invalid priority string)
#[allow(clippy::future_not_send, clippy::too_many_lines)]
pub async fn freer_workflow(
    State(_state): State<AppState>,
    Json(request): Json<FreerWorkflowRequest>,
) -> Result<JsonResponse<FreerWorkflowResponse>, ApiErrorResponse> {
    // Validate request
    if request.steps.is_empty() || request.steps.len() > 20 {
        return Err(ApiErrorResponse::validation_error(
            "steps must have 1-20 items",
            vec![FieldError::new("steps", "Must have 1-20 items")],
        ));
    }

    let valid_modes = ["production", "dry_run", "test"];
    if !valid_modes.contains(&request.execution_mode.as_str()) {
        return Err(ApiErrorResponse::bad_request(
            "INVALID_MODE",
            format!(
                "Invalid execution_mode: {}. Valid: {:?}",
                request.execution_mode, valid_modes
            ),
        ));
    }

    // Build workflow
    let Some(workflow) = build_workflow(&request.steps) else {
        return Err(ApiErrorResponse::bad_request(
            "WORKFLOW_ERROR",
            "Failed to build workflow from steps",
        ));
    };

    // Interpret workflow based on mode
    let (result_task, operations) = match request.execution_mode.as_str() {
        "dry_run" | "test" => {
            let (task, ops) = interpret_dry_run(workflow);
            (Some(task), ops)
        }
        "production" => {
            // Production mode: execute against repository
            let mut last_task: Option<Task> = None;
            let mut operations = Vec::new();

            let result = workflow.interpret(|command| match &command {
                TaskCommand::Create { title } => {
                    let task = Task::new(TaskId::generate(), title.clone(), Timestamp::now());
                    last_task = Some(task.clone());
                    operations.push(format!("CREATE: {title}"));
                    Box::new(task) as Box<dyn Any>
                }
                TaskCommand::UpdatePriority { task_id, priority } => {
                    operations.push(format!("UPDATE_PRIORITY: {task_id} -> {priority:?}"));
                    let task = last_task
                        .clone()
                        .unwrap_or_else(|| {
                            Task::new(TaskId::generate(), "updated".to_string(), Timestamp::now())
                        })
                        .with_priority(*priority);
                    last_task = Some(task.clone());
                    Box::new(task) as Box<dyn Any>
                }
                TaskCommand::AddTag { task_id, tag } => {
                    operations.push(format!("ADD_TAG: {task_id} + {tag}"));
                    let task = last_task.clone().unwrap_or_else(|| {
                        Task::new(TaskId::generate(), "tagged".to_string(), Timestamp::now())
                    });
                    last_task = Some(task.clone());
                    Box::new(task) as Box<dyn Any>
                }
                TaskCommand::GetTask { task_id } => {
                    operations.push(format!("GET: {task_id}"));
                    let task = last_task.clone().unwrap_or_else(|| {
                        Task::new(TaskId::generate(), "fetched".to_string(), Timestamp::now())
                    });
                    Box::new(task) as Box<dyn Any>
                }
            });

            (Some(result), operations)
        }
        _ => (None, Vec::new()),
    };

    let response_task = result_task.map(|task| TaskResponse::from(&task));

    Ok(JsonResponse(FreerWorkflowResponse {
        result: response_task,
        operations_executed: operations,
        execution_mode: request.execution_mode,
    }))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    mod partial_macro_tests {
        use super::*;

        #[rstest]
        fn test_create_score_calculator() {
            let weights = Weights {
                complexity_weight: 0.3,
                priority_weight: 0.5,
                urgency_weight: 0.2,
            };
            let calculator = create_score_calculator(&weights);
            let task = Task::new(
                TaskId::generate(),
                "Test Task".to_string(),
                Timestamp::now(),
            )
            .with_priority(Priority::High);

            let score = calculator(&task);
            assert!(score > 0.0);
        }

        #[rstest]
        fn test_create_task_transformer() {
            let transformer = create_task_transformer(3);
            let task = Task::new(
                TaskId::generate(),
                "Original Title".to_string(),
                Timestamp::now(),
            );

            let transformed = transformer(task);
            assert!(transformed.title.starts_with("[x3]"));
        }

        #[rstest]
        fn test_partial_calculator_is_reusable() {
            let weights = Weights {
                complexity_weight: 1.0,
                priority_weight: 1.0,
                urgency_weight: 1.0,
            };
            let calculator = create_score_calculator(&weights);

            let task1 = Task::new(TaskId::generate(), "Task 1".to_string(), Timestamp::now());
            let task2 = Task::new(TaskId::generate(), "Task 2".to_string(), Timestamp::now());

            let score1 = calculator(&task1);
            let score2 = calculator(&task2);

            assert!(score1 > 0.0);
            assert!(score2 > 0.0);
        }
    }

    mod concurrent_lazy_tests {
        use super::*;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        #[rstest]
        fn test_calculate_stats_empty() {
            let stats = calculate_stats(&[]);
            assert_eq!(stats.total_tasks, 0);
            assert!(stats.average_priority.abs() < f64::EPSILON);
            assert!(stats.completion_rate.abs() < f64::EPSILON);
        }

        #[rstest]
        fn test_calculate_stats_single_task() {
            let task = Task::new(
                TaskId::generate(),
                "Test Task".to_string(),
                Timestamp::now(),
            )
            .with_priority(Priority::High);
            let stats = calculate_stats(&[task]);

            assert_eq!(stats.total_tasks, 1);
            assert!((stats.average_priority - 3.0).abs() < f64::EPSILON); // High = 3.0
            assert_eq!(stats.priority_distribution.high, 1);
        }

        #[rstest]
        fn test_concurrent_lazy_evaluates_once() {
            let counter = Arc::new(AtomicUsize::new(0));
            let counter_clone = counter.clone();
            let lazy = ConcurrentLazy::new(move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                42
            });

            assert_eq!(*lazy.force(), 42);
            assert_eq!(*lazy.force(), 42);
            assert_eq!(counter.load(Ordering::SeqCst), 1);
        }
    }

    mod deque_tests {
        use super::*;

        #[rstest]
        fn test_apply_deque_operation_push_front() {
            let deque = PersistentDeque::new();
            let op = DequeOperationDto::PushFront {
                task_id: "task-1".to_string(),
            };
            let (new_deque, popped) = apply_deque_operation(deque, &op);

            assert_eq!(new_deque.front(), Some(&"task-1".to_string()));
            assert!(popped.is_none());
        }

        #[rstest]
        fn test_apply_deque_operation_push_back() {
            let deque = PersistentDeque::singleton("task-1".to_string());
            let op = DequeOperationDto::PushBack {
                task_id: "task-2".to_string(),
            };
            let (new_deque, _) = apply_deque_operation(deque, &op);

            assert_eq!(new_deque.back(), Some(&"task-2".to_string()));
        }

        #[rstest]
        fn test_apply_deque_operation_pop_front() {
            let deque = PersistentDeque::singleton("task-1".to_string());
            let op = DequeOperationDto::PopFront;
            let (new_deque, popped) = apply_deque_operation(deque, &op);

            assert!(new_deque.is_empty());
            assert_eq!(popped, Some("task-1".to_string()));
        }

        #[rstest]
        fn test_deque_persistence() {
            let original = PersistentDeque::singleton("task-1".to_string());
            let op = DequeOperationDto::PushBack {
                task_id: "task-2".to_string(),
            };
            let (new_deque, _) = apply_deque_operation(original.clone(), &op);

            assert_eq!(original.len(), 1);
            assert_eq!(new_deque.len(), 2);
        }
    }

    mod monoid_aggregation_tests {
        use super::*;

        #[rstest]
        fn test_aggregate_with_sum() {
            let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
            let result = aggregate_with_sum(&values);
            assert!((result - 15.0).abs() < f64::EPSILON);
        }

        #[rstest]
        fn test_aggregate_with_sum_empty() {
            let values: Vec<f64> = vec![];
            let result = aggregate_with_sum(&values);
            assert!((result - 0.0).abs() < f64::EPSILON);
        }

        #[rstest]
        fn test_aggregate_with_product() {
            let values = vec![2.0, 3.0, 4.0];
            let result = aggregate_with_product(&values);
            assert!((result - 24.0).abs() < f64::EPSILON);
        }

        #[rstest]
        fn test_aggregate_with_product_empty() {
            let values: Vec<f64> = vec![];
            let result = aggregate_with_product(&values);
            assert!((result - 1.0).abs() < f64::EPSILON); // Identity for product
        }

        #[rstest]
        fn test_aggregate_with_average() {
            let values = vec![2.0, 4.0, 6.0];
            let result = aggregate_with_average(&values);
            assert!((result - 4.0).abs() < f64::EPSILON);
        }

        #[rstest]
        fn test_sum_monoid_associativity() {
            let a: Sum<f64> = Sum::new(1.0);
            let b: Sum<f64> = Sum::new(2.0);
            let c: Sum<f64> = Sum::new(3.0);

            let left: f64 = a.combine(b).combine(c).into_inner();
            let right: f64 = Sum::new(1.0)
                .combine(Sum::new(2.0).combine(Sum::new(3.0)))
                .into_inner();

            assert!((left - right).abs() < f64::EPSILON);
        }

        #[rstest]
        fn test_sum_monoid_identity() {
            let a = Sum::new(42.0);
            let empty: Sum<f64> = Sum::empty();

            let left = empty.combine(a);
            assert!((left.into_inner() - 42.0).abs() < f64::EPSILON);
        }

        #[rstest]
        fn test_product_monoid_associativity() {
            let a: Product<f64> = Product::new(2.0);
            let b: Product<f64> = Product::new(3.0);
            let c: Product<f64> = Product::new(4.0);

            let left: f64 = a.combine(b).combine(c).into_inner();
            let right: f64 = Product::new(2.0)
                .combine(Product::new(3.0).combine(Product::new(4.0)))
                .into_inner();

            assert!((left - right).abs() < f64::EPSILON);
        }
    }

    mod freer_tests {
        use super::*;

        #[rstest]
        fn test_build_workflow_single_step() {
            let steps = vec![WorkflowStepDto::CreateTask {
                title: "Test".to_string(),
            }];
            let workflow = build_workflow(&steps);
            assert!(workflow.is_some());
        }

        #[rstest]
        fn test_build_workflow_empty() {
            let steps: Vec<WorkflowStepDto> = vec![];
            let workflow = build_workflow(&steps);
            assert!(workflow.is_none());
        }

        #[rstest]
        fn test_interpret_dry_run() {
            let steps = vec![
                WorkflowStepDto::CreateTask {
                    title: "New Task".to_string(),
                },
                WorkflowStepDto::UpdatePriority {
                    task_id: "task-1".to_string(),
                    priority: "high".to_string(),
                },
            ];
            let workflow = build_workflow(&steps).unwrap();
            let (_, operations) = interpret_dry_run(workflow);

            assert_eq!(operations.len(), 2);
            assert!(operations[0].starts_with("CREATE:"));
            assert!(operations[1].starts_with("UPDATE_PRIORITY:"));
        }

        #[rstest]
        fn test_freer_monad_left_identity() {
            let value = 42i32;
            let f = |x: i32| Freer::<(), i32>::pure(x * 2);

            let left = Freer::<(), i32>::pure(value).flat_map(f);
            let right = f(value);

            assert_eq!(
                left.interpret(|()| Box::new(())),
                right.interpret(|()| Box::new(()))
            );
        }

        #[rstest]
        fn test_freer_monad_right_identity() {
            let value = 42i32;
            let result = Freer::<(), i32>::pure(value).flat_map(Freer::pure);
            assert_eq!(result.interpret(|()| Box::new(())), value);
        }
    }

    mod extract_value_tests {
        use super::*;

        #[rstest]
        fn test_extract_priority_value() {
            let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now())
                .with_priority(Priority::Critical);
            let value = extract_numeric_value(&task, "priority");
            assert_eq!(value, Some(4.0));
        }

        #[rstest]
        fn test_extract_tag_count() {
            let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());
            let value = extract_numeric_value(&task, "tag_count");
            assert_eq!(value, Some(0.0));
        }

        #[rstest]
        fn test_extract_title_length() {
            let task = Task::new(
                TaskId::generate(),
                "Hello World".to_string(),
                Timestamp::now(),
            );
            let value = extract_numeric_value(&task, "title_length");
            assert_eq!(value, Some(11.0));
        }

        #[rstest]
        fn test_extract_invalid_field() {
            let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());
            let value = extract_numeric_value(&task, "unknown_field");
            assert_eq!(value, None);
        }
    }
}
