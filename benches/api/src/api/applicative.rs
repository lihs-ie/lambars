//! Applicative operations for independent computation combining.
//!
//! This module demonstrates `Applicative` type class operations including:
//!
//! - **Validation pattern**: Collecting all errors instead of fail-fast
//! - **Independent data fetching**: Parallel fetching and combining
//! - **Building from parts**: Constructing values from optional components
//! - **Parallel computation**: Running independent computations concurrently
//!
//! # lambars Features
//!
//! - `Applicative::pure`: Lift values into context
//! - `Applicative::map2`, `map3`: Combine multiple values
//! - `Applicative::apply`: Apply functions within context
//! - `Applicative::product`: Create tuples from values
//!
//! # Key Differences
//!
//! - **Applicative vs Monad**: Applicative computations are independent;
//!   Monad computations can depend on previous results
//! - **Validation vs Result**: Validation collects all errors;
//!   Result stops at first error (fail-fast)

use std::collections::HashMap;
use std::time::Instant;

use axum::Json;
use axum::extract::{Query, State};

use super::json_buffer::JsonResponse;
use lambars::typeclass::Applicative;
use serde::{Deserialize, Serialize};

use super::dto::TaskResponse;
use super::error::{ApiErrorResponse, FieldError};
use super::handlers::AppState;
use crate::domain::task::{Priority, Task, TaskId, TaskStatus, Timestamp};
use crate::infrastructure::Pagination;

// =============================================================================
// Type Definitions
// =============================================================================

/// Validation type alias for collecting all errors.
///
/// Unlike `Result` which stops at the first error, `Validation` accumulates
/// all errors using a `Vec<E>`.
pub type Validation<E, A> = Result<A, Vec<E>>;

/// Validation error for field-level validation failures.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApplicativeValidationError {
    InvalidTitle { reason: String, field: String },
    InvalidPriority { value: i32, allowed_range: String },
    InvalidDeadline { reason: String },
    InvalidTag { tag: String, reason: String },
    InvalidDescription { reason: String },
}

/// Validated task data after successful validation.
#[derive(Debug, Clone, Serialize)]
pub struct ValidatedTaskDto {
    pub title: String,
    pub priority: Priority,
    pub deadline: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

/// Dashboard data component error.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "component", rename_all = "snake_case")]
pub enum DashboardComponentError {
    TasksFailed { message: String },
    ProjectsFailed { message: String },
    StatsFailed { message: String },
}

/// Computation type for parallel computation endpoint.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ComputationType {
    Complexity,
    Progress,
    Dependencies,
    Estimate,
}

/// Result of a single computation.
#[derive(Debug, Clone, Serialize)]
pub struct ComputationResult {
    pub computation_type: String,
    pub value: serde_json::Value,
    pub confidence: f64,
}

// =============================================================================
// DTOs
// =============================================================================

// -----------------------------------------------------------------------------
// POST /tasks/validate-collect-all
// -----------------------------------------------------------------------------

/// Request for validate-collect-all endpoint.
#[derive(Debug, Deserialize)]
pub struct ValidateCollectAllRequest {
    pub title: String,
    pub priority: i32,
    pub deadline: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Response for validate-collect-all endpoint.
#[derive(Debug, Serialize)]
pub struct ValidateCollectAllResponse {
    pub valid: bool,
    pub validated_task: Option<ValidatedTaskDto>,
    pub errors: Vec<ApplicativeValidationError>,
    pub validation_mode: String,
}

// -----------------------------------------------------------------------------
// GET /dashboard
// -----------------------------------------------------------------------------

/// Query parameters for dashboard endpoint.
#[derive(Debug, Deserialize)]
pub struct DashboardQuery {
    pub include: Option<String>,
}

/// Task summary for dashboard.
#[derive(Debug, Clone, Serialize)]
pub struct TaskSummaryDto {
    pub id: String,
    pub title: String,
    pub priority: String,
    pub status: String,
}

/// Project summary for dashboard.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectSummaryDto {
    pub id: String,
    pub name: String,
    pub task_count: usize,
    pub progress: f64,
}

/// Statistics summary for dashboard.
#[derive(Debug, Clone, Serialize)]
pub struct StatsDto {
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub active_projects: usize,
    pub completion_rate: f64,
}

/// Response for dashboard endpoint.
#[derive(Debug, Serialize)]
pub struct DashboardResponse {
    pub recent_tasks: Vec<TaskSummaryDto>,
    pub active_projects: Vec<ProjectSummaryDto>,
    pub statistics: StatsDto,
    pub partial_failure: bool,
    pub errors: Vec<DashboardComponentError>,
    pub fetch_time_ms: u64,
}

// -----------------------------------------------------------------------------
// POST /tasks/build-from-parts
// -----------------------------------------------------------------------------

/// Request for build-from-parts endpoint.
#[derive(Debug, Deserialize)]
pub struct BuildFromPartsRequest {
    pub title_template_id: Option<String>,
    pub priority_preset: Option<String>,
    pub project_id: Option<String>,
    #[serde(default)]
    pub use_defaults: bool,
}

/// Response for build-from-parts endpoint.
#[derive(Debug, Serialize)]
pub struct BuildFromPartsResponse {
    pub task: Option<TaskResponse>,
    pub project_id: Option<String>,
    pub missing_parts: Vec<String>,
    pub build_success: bool,
    pub build_mode: String,
}

// -----------------------------------------------------------------------------
// POST /tasks/compute-parallel
// -----------------------------------------------------------------------------

/// Request for compute-parallel endpoint.
#[derive(Debug, Deserialize)]
pub struct ComputeParallelRequest {
    pub task_id: String,
    pub computations: Vec<ComputationType>,
}

/// Response for compute-parallel endpoint.
#[derive(Debug, Serialize)]
pub struct ComputeParallelResponse {
    pub task_id: String,
    pub results: HashMap<String, ComputationResult>,
    pub all_succeeded: bool,
    pub failed_computations: Vec<String>,
    pub total_compute_time_ms: u64,
}

// =============================================================================
// Pure Functions - Validation
// =============================================================================

/// Pure: Validates a title field.
fn validate_title(title: &str) -> Validation<ApplicativeValidationError, String> {
    let mut errors = Vec::new();

    let trimmed = title.trim();

    if trimmed.is_empty() {
        errors.push(ApplicativeValidationError::InvalidTitle {
            reason: "Title cannot be empty".to_string(),
            field: "title".to_string(),
        });
    }

    if trimmed.len() > 200 {
        errors.push(ApplicativeValidationError::InvalidTitle {
            reason: format!(
                "Title exceeds maximum length of 200 (got {})",
                trimmed.len()
            ),
            field: "title".to_string(),
        });
    }

    if errors.is_empty() {
        Ok(trimmed.to_string())
    } else {
        Err(errors)
    }
}

/// Pure: Validates a priority field.
fn validate_priority(priority: i32) -> Validation<ApplicativeValidationError, Priority> {
    match priority {
        1 => Ok(Priority::Low),
        2 => Ok(Priority::Medium),
        3 => Ok(Priority::High),
        4 => Ok(Priority::Critical),
        _ => Err(vec![ApplicativeValidationError::InvalidPriority {
            value: priority,
            allowed_range: "1-4 (Low, Medium, High, Critical)".to_string(),
        }]),
    }
}

/// Pure: Validates a deadline field.
fn validate_deadline(
    deadline: Option<&str>,
) -> Validation<ApplicativeValidationError, Option<String>> {
    match deadline {
        None | Some("") => Ok(None),
        Some(d) => {
            if chrono::DateTime::parse_from_rfc3339(d).is_ok() {
                Ok(Some(d.to_string()))
            } else {
                Err(vec![ApplicativeValidationError::InvalidDeadline {
                    reason: format!("Invalid RFC3339 date format: {d}"),
                }])
            }
        }
    }
}

/// Pure: Validates a description field.
fn validate_description(
    description: Option<&str>,
) -> Validation<ApplicativeValidationError, Option<String>> {
    match description {
        None | Some("") => Ok(None),
        Some(d) if d.len() > 2000 => Err(vec![ApplicativeValidationError::InvalidDescription {
            reason: format!(
                "Description exceeds maximum length of 2000 (got {})",
                d.len()
            ),
        }]),
        Some(d) => Ok(Some(d.to_string())),
    }
}

/// Pure: Validates tags.
fn validate_tags(tags: &[String]) -> Validation<ApplicativeValidationError, Vec<String>> {
    let mut errors = Vec::new();
    let mut valid_tags = Vec::new();

    for tag in tags {
        let trimmed = tag.trim();
        if trimmed.is_empty() {
            errors.push(ApplicativeValidationError::InvalidTag {
                tag: tag.clone(),
                reason: "Tag cannot be empty".to_string(),
            });
        } else if trimmed.len() > 50 {
            errors.push(ApplicativeValidationError::InvalidTag {
                tag: tag.clone(),
                reason: format!("Tag exceeds maximum length of 50 (got {})", trimmed.len()),
            });
        } else {
            valid_tags.push(trimmed.to_string());
        }
    }

    if errors.is_empty() {
        Ok(valid_tags)
    } else {
        Err(errors)
    }
}

/// Pure: Combines multiple validations using Applicative pattern.
///
/// This function demonstrates the Applicative pattern for error accumulation.
/// While lambars' `Result::map2`/`map3` is fail-fast (stops at first error),
/// the Validation pattern uses Applicative semantics to collect ALL errors.
///
/// # Applicative Laws Applied
///
/// - **Independence**: Each validation is computed independently
/// - **Combination**: Results are combined using a pure function
/// - **Error accumulation**: All errors are collected, not just the first
///
/// This pattern is equivalent to `Validation` in Haskell/Scala, which has
/// an Applicative instance that accumulates errors using `Semigroup`.
#[allow(clippy::many_single_char_names)]
fn combine_all_validations<A, B, C, D, E, R, Func>(
    title_result: Validation<ApplicativeValidationError, A>,
    priority_result: Validation<ApplicativeValidationError, B>,
    deadline_result: Validation<ApplicativeValidationError, C>,
    description_result: Validation<ApplicativeValidationError, D>,
    tags_result: Validation<ApplicativeValidationError, E>,
    combiner: Func,
) -> Validation<ApplicativeValidationError, R>
where
    Func: FnOnce(A, B, C, D, E) -> R,
{
    match (
        title_result,
        priority_result,
        deadline_result,
        description_result,
        tags_result,
    ) {
        (Ok(title), Ok(priority), Ok(deadline), Ok(description), Ok(tags)) => {
            Ok(combiner(title, priority, deadline, description, tags))
        }
        (title_result, priority_result, deadline_result, description_result, tags_result) => {
            let mut errors = Vec::new();
            if let Err(es) = title_result {
                errors.extend(es);
            }
            if let Err(es) = priority_result {
                errors.extend(es);
            }
            if let Err(es) = deadline_result {
                errors.extend(es);
            }
            if let Err(es) = description_result {
                errors.extend(es);
            }
            if let Err(es) = tags_result {
                errors.extend(es);
            }
            Err(errors)
        }
    }
}

/// Pure: Validates all task fields and collects all errors.
fn validate_task_all_errors(
    request: &ValidateCollectAllRequest,
) -> Validation<ApplicativeValidationError, ValidatedTaskDto> {
    let title_result = validate_title(&request.title);
    let priority_result = validate_priority(request.priority);
    let deadline_result = validate_deadline(request.deadline.as_deref());
    let description_result = validate_description(request.description.as_deref());
    let tags_result = validate_tags(&request.tags);

    combine_all_validations(
        title_result,
        priority_result,
        deadline_result,
        description_result,
        tags_result,
        |title, priority, deadline, description, tags| ValidatedTaskDto {
            title,
            priority,
            deadline,
            description,
            tags,
        },
    )
}

// =============================================================================
// Pure Functions - Dashboard
// =============================================================================

/// Pure: Creates task summaries from tasks.
fn create_task_summaries(tasks: &[Task]) -> Vec<TaskSummaryDto> {
    tasks
        .iter()
        .map(|task| TaskSummaryDto {
            id: task.task_id.as_uuid().to_string(),
            title: task.title.clone(),
            priority: format!("{:?}", task.priority),
            status: format!("{:?}", task.status),
        })
        .collect()
}

/// Pure: Creates project summaries (simulated).
fn create_project_summaries(task_count: usize) -> Vec<ProjectSummaryDto> {
    vec![
        ProjectSummaryDto {
            id: "proj-1".to_string(),
            name: "Main Project".to_string(),
            task_count,
            progress: if task_count > 0 { 0.5 } else { 0.0 },
        },
        ProjectSummaryDto {
            id: "proj-2".to_string(),
            name: "Secondary Project".to_string(),
            task_count: task_count / 2,
            progress: 0.3,
        },
    ]
}

/// Pure: Creates statistics from task data.
#[allow(clippy::cast_precision_loss)]
fn create_statistics(total: usize, completed: usize, project_count: usize) -> StatsDto {
    StatsDto {
        total_tasks: total,
        completed_tasks: completed,
        active_projects: project_count,
        completion_rate: if total > 0 {
            (completed as f64) / (total as f64)
        } else {
            0.0
        },
    }
}

/// Pure: Combines dashboard data using Applicative pattern.
///
/// All three data fetches are independent, so they can be combined
/// using Applicative semantics (which allows parallel execution).
fn combine_dashboard_data(
    tasks_result: Result<Vec<TaskSummaryDto>, DashboardComponentError>,
    projects_result: Result<Vec<ProjectSummaryDto>, DashboardComponentError>,
    stats_result: Result<StatsDto, DashboardComponentError>,
) -> (
    Vec<TaskSummaryDto>,
    Vec<ProjectSummaryDto>,
    StatsDto,
    Vec<DashboardComponentError>,
) {
    let mut errors = Vec::new();

    let tasks = match tasks_result {
        Ok(t) => t,
        Err(e) => {
            errors.push(e);
            Vec::new()
        }
    };

    let projects = match projects_result {
        Ok(p) => p,
        Err(e) => {
            errors.push(e);
            Vec::new()
        }
    };

    let stats = match stats_result {
        Ok(s) => s,
        Err(e) => {
            errors.push(e);
            StatsDto {
                total_tasks: 0,
                completed_tasks: 0,
                active_projects: 0,
                completion_rate: 0.0,
            }
        }
    };

    (tasks, projects, stats, errors)
}

// =============================================================================
// Pure Functions - Build from Parts
// =============================================================================

/// Pure: Resolves a title template to a title string.
fn resolve_title_template(template_id: Option<&str>, use_defaults: bool) -> Option<String> {
    match template_id {
        Some("task") => Some("New Task".to_string()),
        Some("bug") => Some("Bug: ".to_string()),
        Some("feature") => Some("Feature: ".to_string()),
        Some("docs") => Some("Documentation: ".to_string()),
        Some(custom) => Some(custom.to_string()),
        None if use_defaults => Some("Untitled Task".to_string()),
        None => None,
    }
}

/// Pure: Resolves a priority preset to a Priority.
fn resolve_priority_preset(preset: Option<&str>, use_defaults: bool) -> Option<Priority> {
    match preset {
        Some("low") => Some(Priority::Low),
        Some("medium") => Some(Priority::Medium),
        Some("high") => Some(Priority::High),
        Some("critical" | "urgent") => Some(Priority::Critical),
        None if use_defaults => Some(Priority::Medium),
        Some(_) | None => None,
    }
}

/// Pure: Resolves a project ID.
fn resolve_project_id(project_id: Option<&str>, use_defaults: bool) -> Option<String> {
    match project_id {
        Some(id) if !id.is_empty() => Some(id.to_string()),
        _ if use_defaults => Some("default-project".to_string()),
        _ => None,
    }
}

/// Pure: Builds a task from optional parts using Applicative pattern.
///
/// Uses `Option::map3` from Applicative to combine three optional values.
/// This is a pure function - all required values (including `TaskId`) are passed in.
///
/// Returns a tuple of:
/// - `Option<(Task, String)>`: The built task with its associated `project_id` if all parts present
/// - `Vec<String>`: List of missing required parts
fn build_task_from_parts(
    task_id: TaskId,
    title: Option<String>,
    priority: Option<Priority>,
    project_id: Option<String>,
    timestamp: Timestamp,
) -> (Option<(Task, String)>, Vec<String>) {
    let mut missing = Vec::new();

    if title.is_none() {
        missing.push("title".to_string());
    }
    if priority.is_none() {
        missing.push("priority".to_string());
    }
    if project_id.is_none() {
        missing.push("project_id".to_string());
    }

    // Use Applicative map3 to combine three optional values
    // map3 demonstrates Applicative's ability to combine independent computations
    // All three values (title, priority, project_id) are combined into a single result
    let result = title.map3(priority, project_id, move |t, p, proj| {
        (Task::new(task_id, t, timestamp).with_priority(p), proj)
    });

    (result, missing)
}

// =============================================================================
// Pure Functions - Parallel Computation
// =============================================================================

/// Pure: Computes task complexity.
#[allow(clippy::cast_precision_loss)]
fn compute_complexity(task: &Task) -> ComputationResult {
    let base_score = task.title.len() as f64 / 10.0;
    let description_score = task
        .description
        .as_ref()
        .map_or(0.0, |d| d.len() as f64 / 100.0);
    let tag_score = task.tags.len() as f64 * 0.5;
    let priority_score = match task.priority {
        Priority::Low => 1.0,
        Priority::Medium => 2.0,
        Priority::High => 3.0,
        Priority::Critical => 4.0,
    };

    let complexity = base_score + description_score + tag_score + priority_score;

    ComputationResult {
        computation_type: "complexity".to_string(),
        value: serde_json::json!({
            "score": complexity,
            "factors": {
                "title_length": task.title.len(),
                "has_description": task.description.is_some(),
                "tag_count": task.tags.len(),
                "priority": format!("{:?}", task.priority),
            }
        }),
        confidence: 0.85,
    }
}

/// Pure: Computes task progress.
fn compute_progress(task: &Task) -> ComputationResult {
    let progress = match task.status {
        TaskStatus::Pending | TaskStatus::Cancelled => 0.0,
        TaskStatus::InProgress => 0.5,
        TaskStatus::Completed => 1.0,
    };

    ComputationResult {
        computation_type: "progress".to_string(),
        value: serde_json::json!({
            "percentage": progress * 100.0,
            "status": format!("{:?}", task.status),
        }),
        confidence: 1.0,
    }
}

/// Pure: Computes task dependencies (simulated).
fn compute_dependencies(task: &Task) -> ComputationResult {
    let dependency_count = task.subtasks.len();

    ComputationResult {
        computation_type: "dependencies".to_string(),
        value: serde_json::json!({
            "subtask_count": dependency_count,
            "has_dependencies": dependency_count > 0,
            "dependency_depth": i32::from(dependency_count > 0),
        }),
        confidence: 0.9,
    }
}

/// Pure: Computes task estimate (simulated).
#[allow(clippy::cast_precision_loss)]
fn compute_estimate(task: &Task) -> ComputationResult {
    let base_hours = match task.priority {
        Priority::Low => 2.0,
        Priority::Medium => 4.0,
        Priority::High => 8.0,
        Priority::Critical => 16.0,
    };

    let complexity_factor = 1.0 + (task.title.len() as f64 / 100.0);
    let estimate_hours = base_hours * complexity_factor;

    ComputationResult {
        computation_type: "estimate".to_string(),
        value: serde_json::json!({
            "hours": estimate_hours,
            "confidence_interval": {
                "low": estimate_hours * 0.8,
                "high": estimate_hours * 1.5,
            },
            "basis": "priority_and_complexity",
        }),
        confidence: 0.7,
    }
}

/// Pure: Runs a computation on a task.
fn run_computation(task: &Task, computation_type: &ComputationType) -> ComputationResult {
    match computation_type {
        ComputationType::Complexity => compute_complexity(task),
        ComputationType::Progress => compute_progress(task),
        ComputationType::Dependencies => compute_dependencies(task),
        ComputationType::Estimate => compute_estimate(task),
    }
}

/// Pure: Combines computation results using Applicative pattern.
fn combine_computation_results(
    results: Vec<(ComputationType, ComputationResult)>,
) -> HashMap<String, ComputationResult> {
    results
        .into_iter()
        .map(|(ct, result)| (format!("{ct:?}").to_lowercase(), result))
        .collect()
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
// POST /tasks/validate-collect-all
// -----------------------------------------------------------------------------

/// Validates task data and collects all errors.
///
/// This handler demonstrates the **Validation pattern** which uses
/// Applicative semantics to collect all validation errors instead of
/// stopping at the first one (fail-fast behavior of Result).
///
/// # Request Body
///
/// - `title`: Task title
/// - `priority`: Priority (1-4)
/// - `deadline`: Optional RFC3339 deadline
/// - `description`: Optional description
/// - `tags`: List of tags
///
/// # Response
///
/// Returns validation result with either:
/// - `valid: true` with `validated_task`
/// - `valid: false` with list of all `errors`
pub async fn validate_collect_all(
    Json(request): Json<ValidateCollectAllRequest>,
) -> JsonResponse<ValidateCollectAllResponse> {
    let validation_result = validate_task_all_errors(&request);

    match validation_result {
        Ok(validated) => JsonResponse(ValidateCollectAllResponse {
            valid: true,
            validated_task: Some(validated),
            errors: Vec::new(),
            validation_mode: "all_errors_collected".to_string(),
        }),
        Err(errors) => JsonResponse(ValidateCollectAllResponse {
            valid: false,
            validated_task: None,
            errors,
            validation_mode: "all_errors_collected".to_string(),
        }),
    }
}

// -----------------------------------------------------------------------------
// GET /dashboard
// -----------------------------------------------------------------------------

/// Fetches dashboard data from multiple independent sources.
///
/// This handler demonstrates **Applicative for independent data fetching**.
///
/// Two primary data sources (tasks, projects) are fetched independently and in
/// parallel using Applicative semantics. Statistics are derived from these
/// sources using pure computation.
///
/// # Applicative Pattern
///
/// - **Independent fetching**: Tasks and projects are fetched in parallel
/// - **Pure derivation**: Statistics are computed purely from fetched data
/// - **Error accumulation**: Partial failures are tracked without blocking
///
/// # Query Parameters
///
/// - `include`: Optional comma-separated list of components to include
///
/// # Response
///
/// Returns dashboard data with:
/// - Recent tasks
/// - Active projects
/// - Statistics (derived from tasks and projects)
/// - Partial failure info if some fetches failed
///
/// # Errors
///
/// Returns `ApiErrorResponse` if an internal error occurs during data fetching.
#[allow(clippy::cast_possible_truncation)]
pub async fn dashboard(
    State(state): State<AppState>,
    Query(query): Query<DashboardQuery>,
) -> Result<JsonResponse<DashboardResponse>, ApiErrorResponse> {
    let start = Instant::now();

    let include_all = query.include.is_none();
    let include_components: Vec<&str> = query
        .include
        .as_deref()
        .map(|s| s.split(',').map(str::trim).collect())
        .unwrap_or_default();

    // Determine which components to fetch
    // Note: stats requires tasks and projects, so if stats is requested, we fetch all
    let wants_stats = include_all || include_components.contains(&"stats");
    let fetch_tasks = include_all || include_components.contains(&"tasks") || wants_stats;
    let fetch_projects = include_all || include_components.contains(&"projects") || wants_stats;

    // Fetch tasks and projects independently and in parallel using tokio::join!
    // This demonstrates Applicative's key property: independent computations
    // can be executed in parallel and combined afterward.
    let (tasks_result, projects_result) = tokio::join!(
        async {
            if fetch_tasks {
                state
                    .task_repository
                    .list(Pagination::new(0, 100))
                    .run_async()
                    .await
                    .map(|paginated| create_task_summaries(&paginated.items))
                    .map_err(|_| DashboardComponentError::TasksFailed {
                        message: "Failed to fetch tasks".to_string(),
                    })
            } else {
                Ok(Vec::new())
            }
        },
        async {
            if fetch_projects {
                // Simulated independent project fetch
                // In a real app, this would fetch from a project repository
                Ok(create_project_summaries(5)) // Fixed count for demo
            } else {
                Ok(Vec::new())
            }
        }
    );

    // Stats computation is pure - it derives values from the fetched data
    // This demonstrates pure function composition: combine results without side effects
    let stats_result: Result<StatsDto, DashboardComponentError> = if wants_stats {
        // Pure derivation from independently fetched data
        let total = tasks_result.as_ref().map_or(0, Vec::len);
        let completed = tasks_result.as_ref().map_or(0, |tasks| {
            tasks.iter().filter(|t| t.status == "Completed").count()
        });
        let project_count = projects_result.as_ref().map_or(0, Vec::len);
        Ok(create_statistics(total, completed, project_count))
    } else {
        Ok(StatsDto {
            total_tasks: 0,
            completed_tasks: 0,
            active_projects: 0,
            completion_rate: 0.0,
        })
    };

    // Combine using Applicative pattern
    let (tasks, projects, statistics, errors) =
        combine_dashboard_data(tasks_result, projects_result, stats_result);

    let fetch_time_ms = start.elapsed().as_millis() as u64;

    Ok(JsonResponse(DashboardResponse {
        recent_tasks: tasks,
        active_projects: projects,
        statistics,
        partial_failure: !errors.is_empty(),
        errors,
        fetch_time_ms,
    }))
}

// -----------------------------------------------------------------------------
// POST /tasks/build-from-parts
// -----------------------------------------------------------------------------

/// Builds a task from optional parts.
///
/// This handler demonstrates **Applicative for building values from parts**.
/// Each part (title, priority, project) is optional, and we use Applicative's
/// `map2`/`map3` to combine them when all are present.
///
/// # Request Body
///
/// - `title_template_id`: Optional title template
/// - `priority_preset`: Optional priority preset
/// - `project_id`: Optional project ID
/// - `use_defaults`: Whether to use defaults for missing parts
///
/// # Response
///
/// Returns build result with:
/// - `task`: The built task if successful
/// - `missing_parts`: List of missing required parts
/// - `build_success`: Whether the build succeeded
#[allow(clippy::cast_possible_truncation)]
pub async fn build_from_parts(
    Json(request): Json<BuildFromPartsRequest>,
) -> JsonResponse<BuildFromPartsResponse> {
    let title = resolve_title_template(request.title_template_id.as_deref(), request.use_defaults);
    let priority =
        resolve_priority_preset(request.priority_preset.as_deref(), request.use_defaults);
    let project_id = resolve_project_id(request.project_id.as_deref(), request.use_defaults);

    // Generate ID and timestamp at system boundary (effect boundary)
    let task_id = TaskId::generate();
    let timestamp = Timestamp::now();

    // Call pure function with all values using Applicative map3
    let (result, missing_parts) =
        build_task_from_parts(task_id, title, priority, project_id, timestamp);

    let build_mode = if request.use_defaults {
        "with_defaults"
    } else {
        "strict"
    };

    // Destructure result which contains both task and project_id (combined by map3)
    let (task, resolved_project_id) = match result {
        Some((t, p)) => (Some(t), Some(p)),
        None => (None, None),
    };

    JsonResponse(BuildFromPartsResponse {
        task: task.as_ref().map(TaskResponse::from),
        project_id: resolved_project_id,
        missing_parts,
        build_success: task.is_some(),
        build_mode: build_mode.to_string(),
    })
}

// -----------------------------------------------------------------------------
// POST /tasks/compute-parallel
// -----------------------------------------------------------------------------

/// Computes multiple metrics for a task in parallel.
///
/// This handler demonstrates **Applicative for parallel computation**.
/// Each computation (complexity, progress, dependencies, estimate) is
/// independent and can be run in parallel, then combined.
///
/// # Request Body
///
/// - `task_id`: Task ID to compute metrics for
/// - `computations`: List of computation types to run
///
/// # Response
///
/// Returns computation results with:
/// - `results`: Map of computation type to result
/// - `all_succeeded`: Whether all computations succeeded
/// - `total_compute_time_ms`: Total computation time
///
/// # Errors
///
/// - `404 Not Found`: Task not found
/// - `400 Bad Request`: Invalid request
#[allow(clippy::cast_possible_truncation)]
pub async fn compute_parallel(
    State(state): State<AppState>,
    Json(request): Json<ComputeParallelRequest>,
) -> Result<JsonResponse<ComputeParallelResponse>, ApiErrorResponse> {
    let start = Instant::now();

    // Validate request
    if request.computations.is_empty() {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new(
                "computations",
                "computations list cannot be empty",
            )],
        ));
    }

    if request.computations.len() > 10 {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new(
                "computations",
                "computations list cannot exceed 10 items",
            )],
        ));
    }

    // Parse task ID
    let task_id = parse_task_id(&request.task_id).map_err(|error| {
        ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("task_id", error)],
        )
    })?;

    // Fetch task
    let task = state
        .task_repository
        .find_by_id(&task_id)
        .run_async()
        .await
        .map_err(|_| ApiErrorResponse::internal_error("Repository error"))?
        .ok_or_else(|| {
            ApiErrorResponse::not_found(format!("Task not found: {}", request.task_id))
        })?;

    // Run all computations - each computation is independent (Applicative property)
    // Since computations are pure functions with no I/O, they don't benefit from
    // async parallelism. For CPU-bound work, `rayon` would be appropriate.
    // The key Applicative insight: each computation is independent and can be
    // combined without any dependency on other computation results.
    let results: Vec<(ComputationType, ComputationResult)> = request
        .computations
        .iter()
        .map(|ct| (ct.clone(), run_computation(&task, ct)))
        .collect();

    // Combine independent results using Applicative pattern (similar to sequence/traverse)
    let result_map = combine_computation_results(results);
    let all_succeeded = true; // All computations are pure and cannot fail

    let total_compute_time_ms = start.elapsed().as_millis() as u64;

    Ok(JsonResponse(ComputeParallelResponse {
        task_id: request.task_id,
        results: result_map,
        all_succeeded,
        failed_computations: Vec::new(),
        total_compute_time_ms,
    }))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::{SubTask, SubTaskId};
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // Validation Pure Function Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_validate_title_valid() {
        let result = validate_title("Valid Title");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Valid Title");
    }

    #[rstest]
    fn test_validate_title_empty() {
        let result = validate_title("");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().len(), 1);
    }

    #[rstest]
    fn test_validate_title_whitespace() {
        let result = validate_title("   ");
        assert!(result.is_err());
    }

    #[rstest]
    fn test_validate_title_too_long() {
        let long_title = "a".repeat(201);
        let result = validate_title(&long_title);
        assert!(result.is_err());
        assert!(matches!(
            &result.unwrap_err()[0],
            ApplicativeValidationError::InvalidTitle { .. }
        ));
    }

    #[rstest]
    fn test_validate_priority_valid() {
        assert_eq!(validate_priority(1).unwrap(), Priority::Low);
        assert_eq!(validate_priority(2).unwrap(), Priority::Medium);
        assert_eq!(validate_priority(3).unwrap(), Priority::High);
        assert_eq!(validate_priority(4).unwrap(), Priority::Critical);
    }

    #[rstest]
    fn test_validate_priority_invalid() {
        assert!(validate_priority(0).is_err());
        assert!(validate_priority(5).is_err());
        assert!(validate_priority(-1).is_err());
    }

    #[rstest]
    fn test_validate_deadline_valid() {
        let result = validate_deadline(Some("2024-12-31T23:59:59Z"));
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_validate_deadline_none() {
        let result = validate_deadline(None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[rstest]
    fn test_validate_deadline_invalid() {
        let result = validate_deadline(Some("not-a-date"));
        assert!(result.is_err());
    }

    #[rstest]
    fn test_validate_tags_valid() {
        let tags = vec!["rust".to_string(), "api".to_string()];
        let result = validate_tags(&tags);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[rstest]
    fn test_validate_tags_empty_tag() {
        let tags = vec!["valid".to_string(), String::new()];
        let result = validate_tags(&tags);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_validate_task_all_errors_valid() {
        let request = ValidateCollectAllRequest {
            title: "Valid Title".to_string(),
            priority: 2,
            deadline: None,
            description: None,
            tags: vec![],
        };
        let result = validate_task_all_errors(&request);
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_validate_task_all_errors_collects_all() {
        let request = ValidateCollectAllRequest {
            title: String::new(),
            priority: 99,
            deadline: Some("invalid".to_string()),
            description: None,
            tags: vec![String::new()],
        };
        let result = validate_task_all_errors(&request);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        // Should have at least 3 errors: title, priority, deadline, tag
        assert!(errors.len() >= 3);
    }

    // -------------------------------------------------------------------------
    // Validation vs Fail-Fast Comparison Test
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_validation_vs_fail_fast() {
        // Validation pattern: collects all errors
        let request = ValidateCollectAllRequest {
            title: String::new(),
            priority: 99,
            deadline: None,
            description: None,
            tags: vec![],
        };
        let validation_result = validate_task_all_errors(&request);
        let validation_errors = validation_result.unwrap_err();

        // Fail-fast pattern: stops at first error
        let fail_fast_result: Result<(), &str> = Err("First error");
        let fail_fast_errors = fail_fast_result.err().map(|e| vec![e]).unwrap_or_default();

        // Validation collected multiple errors, fail-fast only one
        assert!(validation_errors.len() > fail_fast_errors.len());
    }

    // -------------------------------------------------------------------------
    // Dashboard Pure Function Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_create_task_summaries() {
        let tasks = vec![Task::new(
            TaskId::generate(),
            "Test Task".to_string(),
            Timestamp::now(),
        )];
        let summaries = create_task_summaries(&tasks);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].title, "Test Task");
    }

    #[rstest]
    fn test_create_statistics() {
        let stats = create_statistics(10, 5, 2);
        assert_eq!(stats.total_tasks, 10);
        assert_eq!(stats.completed_tasks, 5);
        assert_eq!(stats.active_projects, 2);
        assert!((stats.completion_rate - 0.5).abs() < f64::EPSILON);
    }

    #[rstest]
    fn test_create_statistics_zero_total() {
        let stats = create_statistics(0, 0, 0);
        assert!(stats.completion_rate.abs() < f64::EPSILON);
    }

    #[rstest]
    fn test_combine_dashboard_data_all_success() {
        let tasks = Ok(vec![TaskSummaryDto {
            id: "1".to_string(),
            title: "Task".to_string(),
            priority: "Medium".to_string(),
            status: "Todo".to_string(),
        }]);
        let projects = Ok(vec![]);
        let stats = Ok(StatsDto {
            total_tasks: 1,
            completed_tasks: 0,
            active_projects: 0,
            completion_rate: 0.0,
        });

        let (returned_tasks, _projects, _statistics, errors) =
            combine_dashboard_data(tasks, projects, stats);
        assert_eq!(returned_tasks.len(), 1);
        assert!(errors.is_empty());
    }

    #[rstest]
    fn test_combine_dashboard_data_partial_failure() {
        let tasks = Err(DashboardComponentError::TasksFailed {
            message: "Error".to_string(),
        });
        let projects = Ok(vec![]);
        let stats = Ok(StatsDto {
            total_tasks: 0,
            completed_tasks: 0,
            active_projects: 0,
            completion_rate: 0.0,
        });

        let (returned_tasks, _projects, _statistics, errors) =
            combine_dashboard_data(tasks, projects, stats);
        assert!(returned_tasks.is_empty());
        assert_eq!(errors.len(), 1);
    }

    // -------------------------------------------------------------------------
    // Build from Parts Pure Function Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_resolve_title_template() {
        assert_eq!(
            resolve_title_template(Some("task"), false),
            Some("New Task".to_string())
        );
        assert_eq!(
            resolve_title_template(Some("bug"), false),
            Some("Bug: ".to_string())
        );
        assert_eq!(resolve_title_template(None, false), None);
        assert_eq!(
            resolve_title_template(None, true),
            Some("Untitled Task".to_string())
        );
    }

    #[rstest]
    fn test_resolve_priority_preset() {
        assert_eq!(
            resolve_priority_preset(Some("low"), false),
            Some(Priority::Low)
        );
        assert_eq!(
            resolve_priority_preset(Some("critical"), false),
            Some(Priority::Critical)
        );
        assert_eq!(resolve_priority_preset(None, false), None);
        assert_eq!(resolve_priority_preset(None, true), Some(Priority::Medium));
    }

    #[rstest]
    fn test_build_task_from_parts_all_present() {
        let task_id = TaskId::generate();
        let title = Some("Test".to_string());
        let priority = Some(Priority::High);
        let project_id = Some("proj-1".to_string());
        let timestamp = Timestamp::now();

        let (result, missing) =
            build_task_from_parts(task_id, title, priority, project_id, timestamp);
        assert!(result.is_some());
        assert!(missing.is_empty());

        let (task, proj_id) = result.unwrap();
        assert_eq!(task.priority, Priority::High);
        assert_eq!(proj_id, "proj-1");
    }

    #[rstest]
    fn test_build_task_from_parts_missing_title() {
        let task_id = TaskId::generate();
        let title = None;
        let priority = Some(Priority::Medium);
        let project_id = Some("proj-1".to_string());
        let timestamp = Timestamp::now();

        let (result, missing) =
            build_task_from_parts(task_id, title, priority, project_id, timestamp);
        assert!(result.is_none());
        assert!(missing.contains(&"title".to_string()));
    }

    #[rstest]
    fn test_build_task_from_parts_missing_multiple() {
        let task_id = TaskId::generate();
        let title = None;
        let priority = None;
        let project_id = None;
        let timestamp = Timestamp::now();

        let (result, missing) =
            build_task_from_parts(task_id, title, priority, project_id, timestamp);
        assert!(result.is_none());
        assert!(missing.contains(&"title".to_string()));
        assert!(missing.contains(&"priority".to_string()));
        assert!(missing.contains(&"project_id".to_string()));
    }

    // -------------------------------------------------------------------------
    // Computation Pure Function Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_compute_complexity() {
        let task = Task::new(
            TaskId::generate(),
            "Test Task".to_string(),
            Timestamp::now(),
        )
        .with_priority(Priority::High);
        let result = compute_complexity(&task);
        assert_eq!(result.computation_type, "complexity");
        assert!(result.confidence > 0.0);
    }

    #[rstest]
    fn test_compute_progress() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());
        let result = compute_progress(&task);
        assert_eq!(result.computation_type, "progress");
        assert!((result.confidence - 1.0).abs() < f64::EPSILON);
    }

    #[rstest]
    fn test_compute_dependencies() {
        let task = Task::new(TaskId::generate(), "Parent".to_string(), Timestamp::now());
        let task = task.prepend_subtask(SubTask::new(SubTaskId::generate(), "Subtask".to_string()));
        let result = compute_dependencies(&task);
        assert_eq!(result.computation_type, "dependencies");
    }

    #[rstest]
    fn test_compute_estimate() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now())
            .with_priority(Priority::Critical);
        let result = compute_estimate(&task);
        assert_eq!(result.computation_type, "estimate");
    }

    #[rstest]
    fn test_run_computation_all_types() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());

        let complexity = run_computation(&task, &ComputationType::Complexity);
        assert_eq!(complexity.computation_type, "complexity");

        let progress = run_computation(&task, &ComputationType::Progress);
        assert_eq!(progress.computation_type, "progress");

        let deps = run_computation(&task, &ComputationType::Dependencies);
        assert_eq!(deps.computation_type, "dependencies");

        let estimate = run_computation(&task, &ComputationType::Estimate);
        assert_eq!(estimate.computation_type, "estimate");
    }

    #[rstest]
    fn test_combine_computation_results() {
        let results = vec![
            (
                ComputationType::Complexity,
                ComputationResult {
                    computation_type: "complexity".to_string(),
                    value: serde_json::json!({"score": 1.0}),
                    confidence: 0.9,
                },
            ),
            (
                ComputationType::Progress,
                ComputationResult {
                    computation_type: "progress".to_string(),
                    value: serde_json::json!({"percentage": 50.0}),
                    confidence: 1.0,
                },
            ),
        ];

        let map = combine_computation_results(results);
        assert_eq!(map.len(), 2);
        assert!(map.contains_key("complexity"));
        assert!(map.contains_key("progress"));
    }

    // -------------------------------------------------------------------------
    // Applicative Law Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[allow(clippy::map_identity)]
    fn test_applicative_identity_law_option() {
        // pure(id).apply(v) == v
        // Approximated with fmap since Rust doesn't have point-free id
        let v: Option<i32> = Some(42);
        let result = v.map(|x| x);
        assert_eq!(result, Some(42));
    }

    #[rstest]
    fn test_applicative_homomorphism_law_option() {
        // pure(f).apply(pure(x)) == pure(f(x))
        let f = |x: i32| x + 1;
        let x = 5;

        let left: Option<i32> = <Option<()>>::pure(f).apply(<Option<()>>::pure(x));
        let right: Option<i32> = <Option<()>>::pure(f(x));

        assert_eq!(left, right);
        assert_eq!(left, Some(6));
    }

    #[rstest]
    fn test_applicative_map2_option() {
        let a = Some(1);
        let b = Some(2);
        let result = a.map2(b, |x, y| x + y);
        assert_eq!(result, Some(3));
    }

    #[rstest]
    fn test_applicative_map2_option_with_none() {
        let a = Some(1);
        let b: Option<i32> = None;
        let result = a.map2(b, |x, y| x + y);
        assert_eq!(result, None);
    }

    #[rstest]
    fn test_applicative_map3_option() {
        let a = Some(1);
        let b = Some(2);
        let c = Some(3);
        let result = a.map3(b, c, |x, y, z| x + y + z);
        assert_eq!(result, Some(6));
    }

    #[rstest]
    fn test_applicative_product_option() {
        let a = Some(1);
        let b = Some("hello");
        let result = a.product(b);
        assert_eq!(result, Some((1, "hello")));
    }

    #[rstest]
    #[allow(clippy::map_identity)]
    fn test_applicative_identity_law_result() {
        let v: Result<i32, &str> = Ok(42);
        let result = v.map(|x| x);
        assert_eq!(result, Ok(42));
    }

    #[rstest]
    fn test_applicative_map2_result() {
        let a: Result<i32, &str> = Ok(1);
        let b: Result<i32, &str> = Ok(2);
        let result = a.map2(b, |x, y| x + y);
        assert_eq!(result, Ok(3));
    }

    #[rstest]
    fn test_applicative_map2_result_fail_fast() {
        // Result's Applicative is fail-fast, returns first error
        let a: Result<i32, &str> = Err("first");
        let b: Result<i32, &str> = Err("second");
        let result = a.map2(b, |x, y| x + y);
        assert_eq!(result, Err("first"));
    }

    // -------------------------------------------------------------------------
    // Validation vs Result Comparison Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_validation_collects_all_result_fails_fast() {
        // Result: stops at first error
        let r1: Result<i32, &str> = Err("error1");
        let r2: Result<i32, &str> = Err("error2");
        let result_combined = r1.map2(r2, |a, b| a + b);
        // Only gets first error
        assert_eq!(result_combined, Err("error1"));

        // Validation: collects all errors
        let v1: Validation<&str, i32> = Err(vec!["error1"]);
        let v2: Validation<&str, i32> = Err(vec!["error2"]);

        let validation_combined = match (v1, v2) {
            (Ok(a), Ok(b)) => Ok(a + b),
            (va, vb) => {
                let mut errors = Vec::new();
                if let Err(es) = va {
                    errors.extend(es);
                }
                if let Err(es) = vb {
                    errors.extend(es);
                }
                Err(errors)
            }
        };

        // Gets all errors
        assert_eq!(validation_combined, Err(vec!["error1", "error2"]));
    }
}
