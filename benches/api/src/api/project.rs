//! Project handlers demonstrating lambars functional programming features.
//!
//! This module contains HTTP handlers for project management operations,
//! showcasing the following lambars features:
//!
//! - **Semigroup/Monoid**: Error accumulation in validation, progress aggregation
//! - **Reader**: Configuration-based dependency injection
//! - **Validated (Applicative)**: Error-accumulating validation pattern
//!
//! # Handlers
//!
//! - `POST /projects`: Create project with error-accumulating validation
//! - `GET /projects/{id}`: Get project detail using Reader for config
//! - `GET /projects/{id}/progress`: Calculate progress using Monoid
//! - `GET /projects/{id}/stats`: Aggregate statistics using Semigroup

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use lambars::control::Trampoline;
use lambars::effect::Reader;
use lambars::typeclass::{Foldable, Monoid, Semigroup};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::cache_header::CacheSource;
use super::dto::{PriorityDto, TaskStatusDto};
use super::error::ApiErrorResponse;
use super::handlers::{AppConfig, AppState, build_cache_headers};
use crate::domain::{Priority, Project, ProjectId, TaskStatus, TaskSummary, Timestamp};

// =============================================================================
// Request/Response DTOs
// =============================================================================

/// Request body for creating a new project.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateProjectRequest {
    /// Name of the project (1-100 characters).
    pub name: String,
    /// Optional description (max 1000 characters).
    pub description: Option<String>,
}

/// Response body for project operations.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectResponse {
    /// Unique identifier.
    pub project_id: String,
    /// Project name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Number of tasks in the project.
    pub task_count: usize,
    /// ISO 8601 timestamp when created.
    pub created_at: String,
    /// ISO 8601 timestamp when last updated.
    pub updated_at: String,
    /// Version number.
    pub version: u64,
}

impl From<&Project> for ProjectResponse {
    fn from(project: &Project) -> Self {
        Self {
            project_id: project.project_id.to_string(),
            name: project.name.clone(),
            description: project.description.clone(),
            task_count: project.task_count(),
            created_at: project.created_at.to_string(),
            updated_at: project.updated_at.to_string(),
            version: project.version,
        }
    }
}

/// Response body for task summary within a project.
#[derive(Debug, Clone, Serialize)]
pub struct TaskSummaryResponse {
    /// Task unique identifier.
    pub task_id: String,
    /// Task title.
    pub title: String,
    /// Current status (serialized as `snake_case`).
    pub status: TaskStatusDto,
    /// Priority level (serialized as `snake_case`).
    pub priority: PriorityDto,
}

impl From<&TaskSummary> for TaskSummaryResponse {
    fn from(summary: &TaskSummary) -> Self {
        Self {
            task_id: summary.task_id.to_string(),
            title: summary.title.clone(),
            status: TaskStatusDto::from(summary.status),
            priority: PriorityDto::from(summary.priority),
        }
    }
}

/// Response body for project detail with tasks.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectDetailResponse {
    /// Unique identifier.
    pub project_id: String,
    /// Project name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// List of task summaries.
    pub tasks: Vec<TaskSummaryResponse>,
    /// Total number of tasks.
    pub task_count: usize,
    /// Maximum tasks allowed per project (from config).
    pub max_tasks: usize,
    /// Default page size (from config).
    pub default_page_size: u32,
    /// ISO 8601 timestamp when created.
    pub created_at: String,
    /// ISO 8601 timestamp when last updated.
    pub updated_at: String,
}

/// Response body for project progress.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectProgressResponse {
    /// Project unique identifier.
    pub project_id: String,
    /// Total number of tasks.
    pub total_tasks: usize,
    /// Number of pending tasks.
    pub pending_tasks: usize,
    /// Number of in-progress tasks.
    pub in_progress_tasks: usize,
    /// Number of completed tasks.
    pub completed_tasks: usize,
    /// Number of cancelled tasks.
    pub cancelled_tasks: usize,
    /// Completion percentage (0.0 - 100.0).
    pub completion_percentage: f64,
}

/// Response body for project statistics.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectStatsResponse {
    /// Project unique identifier.
    pub project_id: String,
    /// Total number of tasks.
    pub total_tasks: usize,
    /// Counts by status.
    pub status_counts: StatusCountsResponse,
    /// Counts by priority.
    pub priority_counts: PriorityCountsResponse,
}

/// Status counts breakdown.
#[derive(Debug, Clone, Serialize)]
pub struct StatusCountsResponse {
    /// Number of pending tasks.
    pub pending: usize,
    /// Number of in-progress tasks.
    pub in_progress: usize,
    /// Number of completed tasks.
    pub completed: usize,
    /// Number of cancelled tasks.
    pub cancelled: usize,
}

impl From<ProgressStats> for StatusCountsResponse {
    fn from(stats: ProgressStats) -> Self {
        Self {
            pending: stats.pending,
            in_progress: stats.in_progress,
            completed: stats.completed,
            cancelled: stats.cancelled,
        }
    }
}

/// Priority counts breakdown.
#[derive(Debug, Clone, Serialize)]
pub struct PriorityCountsResponse {
    /// Number of low priority tasks.
    pub low: usize,
    /// Number of medium priority tasks.
    pub medium: usize,
    /// Number of high priority tasks.
    pub high: usize,
    /// Number of critical priority tasks.
    pub critical: usize,
}

impl From<PriorityCounts> for PriorityCountsResponse {
    fn from(counts: PriorityCounts) -> Self {
        Self {
            low: counts.low,
            medium: counts.medium,
            high: counts.high,
            critical: counts.critical,
        }
    }
}

// =============================================================================
// Validation Types
// =============================================================================

/// Validated project data after passing all validations.
#[derive(Debug, Clone)]
struct ValidatedProject {
    name: String,
    description: Option<String>,
}

/// Validation type alias for collecting all errors using Applicative pattern.
///
/// Unlike `Result` which stops at the first error (fail-fast), `Validated` accumulates
/// all errors using `Vec<FieldError>`. This follows the Applicative Functor pattern
/// where independent validations can be combined and all errors collected.
///
/// # Applicative Properties
///
/// - **Independence**: Each field validation is computed independently
/// - **Error accumulation**: All validation errors are collected, not just the first
/// - **Combination**: Results are combined using a pure function via `map2`
type Validated<A> = Result<A, Vec<super::error::FieldError>>;

// =============================================================================
// Validation Functions (Pure)
// =============================================================================

/// Validates project name using the Validated (error-accumulating) pattern.
///
/// Name must be 1-100 characters (whitespace-only names are rejected).
///
/// # Returns
///
/// - `Ok(String)`: Validated and trimmed name
/// - `Err(Vec<FieldError>)`: List of validation errors for this field
fn validate_project_name_validated(name: &str) -> Validated<String> {
    use super::error::FieldError;
    let trimmed = name.trim();
    if trimmed.is_empty() {
        Err(vec![FieldError::new("name", "Name is required")])
    } else if trimmed.len() > 100 {
        Err(vec![FieldError::new(
            "name",
            "Name must be at most 100 characters",
        )])
    } else {
        Ok(trimmed.to_string())
    }
}

/// Validates project description using the Validated (error-accumulating) pattern.
///
/// Description is optional but must be at most 1000 characters if provided.
///
/// # Returns
///
/// - `Ok(Option<String>)`: Validated description (None if not provided)
/// - `Err(Vec<FieldError>)`: List of validation errors for this field
fn validate_project_description_validated(description: Option<&str>) -> Validated<Option<String>> {
    use super::error::FieldError;
    match description {
        None => Ok(None),
        Some(d) if d.len() > 1000 => Err(vec![FieldError::new(
            "description",
            "Description must be at most 1000 characters",
        )]),
        Some(d) => Ok(Some(d.to_string())),
    }
}

/// Combines two Validated results using Applicative semantics.
///
/// This function implements the Applicative `map2` pattern for error accumulation.
/// Unlike `Result::map2` which is fail-fast (stops at first error), this function
/// collects ALL errors from both validations.
///
/// # Applicative Laws
///
/// - **Independence**: Both validations are evaluated regardless of each other's result
/// - **Combination**: When both succeed, the combiner function creates the final value
/// - **Error accumulation**: When either or both fail, all errors are concatenated
///
/// # Type Parameters
///
/// - `A`: Type of first validated value
/// - `B`: Type of second validated value
/// - `C`: Type of combined result
/// - `F`: Combiner function type
fn validated_map2<A, B, C, F>(
    first: Validated<A>,
    second: Validated<B>,
    combiner: F,
) -> Validated<C>
where
    F: FnOnce(A, B) -> C,
{
    match (first, second) {
        (Ok(a), Ok(b)) => Ok(combiner(a, b)),
        (Err(errors_a), Err(errors_b)) => {
            // Accumulate all errors from both validations
            let mut all_errors = errors_a;
            all_errors.extend(errors_b);
            Err(all_errors)
        }
        (Err(errors), Ok(_)) | (Ok(_), Err(errors)) => Err(errors),
    }
}

/// Validates create project request, accumulating all errors using Applicative pattern.
///
/// This function demonstrates the **Applicative Functor pattern** for validation:
///
/// 1. Each field is validated independently using `Validated<T>` (error-accumulating type)
/// 2. Results are combined using `validated_map2` which implements Applicative `map2` semantics
/// 3. All validation errors are collected, not just the first one
///
/// # lambars Features
///
/// - `Validated<A>`: Type alias for `Result<A, Vec<FieldError>>` enabling error accumulation
/// - `validated_map2`: Applicative combinator that collects all errors
/// - `Applicative::map2` semantics: Independent computations combined with a pure function
///
/// # Difference from Monad
///
/// - **Monad (`flat_map`)**: Dependent computations where later steps depend on earlier results
/// - **Applicative (`map2`)**: Independent computations that can be evaluated in parallel
///
/// For validation, Applicative is superior because:
/// - All fields can be validated independently
/// - All errors can be reported at once (better UX)
///
/// # Example
///
/// When both name and description are invalid, both errors are returned:
/// ```text
/// Input: { name: "", description: "x".repeat(1001) }
/// Output: Err([
///     FieldError { field: "name", message: "Name is required" },
///     FieldError { field: "description", message: "Description must be at most 1000 characters" }
/// ])
/// ```
fn validate_create_project(request: &CreateProjectRequest) -> Validated<ValidatedProject> {
    let name_result = validate_project_name_validated(&request.name);
    let description_result = validate_project_description_validated(request.description.as_deref());

    // Use Applicative map2 to combine validations with error accumulation
    validated_map2(name_result, description_result, |name, description| {
        ValidatedProject { name, description }
    })
}

// =============================================================================
// Pure Construction Functions
// =============================================================================

/// Builds a project from validated data.
///
/// This is a pure function - all impure data (ID, timestamp) is passed as arguments.
fn build_project_pure(
    project_id: ProjectId,
    validated: ValidatedProject,
    timestamp: Timestamp,
) -> Project {
    let base = Project::new(project_id, validated.name, timestamp);
    match validated.description {
        Some(desc) => base.with_description(desc),
        None => base,
    }
}

// =============================================================================
// Progress Statistics (Monoid Pattern)
// =============================================================================

/// Progress statistics as a record monoid.
///
/// Implements `Semigroup` and `Monoid` for functional aggregation.
#[derive(Clone, Default, Debug)]
pub struct ProgressStats {
    /// Total number of tasks.
    pub total: usize,
    /// Number of pending tasks.
    pub pending: usize,
    /// Number of in-progress tasks.
    pub in_progress: usize,
    /// Number of completed tasks.
    pub completed: usize,
    /// Number of cancelled tasks.
    pub cancelled: usize,
}

impl Semigroup for ProgressStats {
    fn combine(self, other: Self) -> Self {
        Self {
            total: self.total + other.total,
            pending: self.pending + other.pending,
            in_progress: self.in_progress + other.in_progress,
            completed: self.completed + other.completed,
            cancelled: self.cancelled + other.cancelled,
        }
    }
}

impl Monoid for ProgressStats {
    fn empty() -> Self {
        Self::default()
    }
}

/// Pure: Converts a task summary to progress stats.
///
/// Uses immutable pattern matching (no mutation).
fn task_to_stats(summary: &TaskSummary) -> ProgressStats {
    match summary.status {
        TaskStatus::Pending => ProgressStats {
            total: 1,
            pending: 1,
            ..Default::default()
        },
        TaskStatus::InProgress => ProgressStats {
            total: 1,
            in_progress: 1,
            ..Default::default()
        },
        TaskStatus::Completed => ProgressStats {
            total: 1,
            completed: 1,
            ..Default::default()
        },
        TaskStatus::Cancelled => ProgressStats {
            total: 1,
            cancelled: 1,
            ..Default::default()
        },
    }
}

/// Pure: Calculates completion percentage.
///
/// Matches domain `Project::progress` behavior - returns 100% for empty/all-cancelled.
#[allow(clippy::cast_precision_loss)]
fn calculate_completion(progress_stats: &ProgressStats) -> f64 {
    let active_total = progress_stats
        .total
        .saturating_sub(progress_stats.cancelled);
    if active_total == 0 {
        100.0 // Matches Project::progress() behavior
    } else {
        (progress_stats.completed as f64 / active_total as f64) * 100.0
    }
}

/// Stack-safe `fold_map` using `Trampoline`.
///
/// This function performs a stack-safe fold operation over an iterator,
/// mapping each element to a Monoid value and combining them.
///
/// # Type Parameters
///
/// * `I` - Iterator type
/// * `M` - Monoid type for accumulation
/// * `F` - Function to map elements to Monoid values
///
/// # Arguments
///
/// * `iterator` - Iterator over elements
/// * `map_function` - Function to convert each element to a Monoid value
///
/// # Returns
///
/// A `Trampoline<M>` that, when run, produces the combined result.
fn trampoline_fold_map<I, M, F>(iterator: I, map_function: F) -> Trampoline<M>
where
    I: IntoIterator,
    I::IntoIter: 'static,
    M: Monoid + 'static,
    F: Fn(I::Item) -> M + 'static,
{
    fn fold_step<Item, M, F>(
        mut iterator: impl Iterator<Item = Item> + 'static,
        accumulator: M,
        map_function: std::rc::Rc<F>,
    ) -> Trampoline<M>
    where
        M: Monoid + 'static,
        F: Fn(Item) -> M + 'static,
    {
        match iterator.next() {
            None => Trampoline::done(accumulator),
            Some(item) => {
                let mapped = map_function(item);
                let new_accumulator = accumulator.combine(mapped);
                Trampoline::suspend(move || fold_step(iterator, new_accumulator, map_function))
            }
        }
    }

    let iter = iterator.into_iter();
    let map_function_rc = std::rc::Rc::new(map_function);
    fold_step(iter, M::empty(), map_function_rc)
}

// =============================================================================
// Priority Statistics (Monoid Pattern)
// =============================================================================

/// Priority counts as a record monoid.
#[derive(Clone, Default, Debug)]
pub struct PriorityCounts {
    /// Number of low priority tasks.
    pub low: usize,
    /// Number of medium priority tasks.
    pub medium: usize,
    /// Number of high priority tasks.
    pub high: usize,
    /// Number of critical priority tasks.
    pub critical: usize,
}

impl Semigroup for PriorityCounts {
    fn combine(self, other: Self) -> Self {
        Self {
            low: self.low + other.low,
            medium: self.medium + other.medium,
            high: self.high + other.high,
            critical: self.critical + other.critical,
        }
    }
}

impl Monoid for PriorityCounts {
    fn empty() -> Self {
        Self::default()
    }
}

/// Pure: Converts a task summary to priority count.
fn task_to_priority(summary: &TaskSummary) -> PriorityCounts {
    match summary.priority {
        Priority::Low => PriorityCounts {
            low: 1,
            ..Default::default()
        },
        Priority::Medium => PriorityCounts {
            medium: 1,
            ..Default::default()
        },
        Priority::High => PriorityCounts {
            high: 1,
            ..Default::default()
        },
        Priority::Critical => PriorityCounts {
            critical: 1,
            ..Default::default()
        },
    }
}

// =============================================================================
// Combined Statistics (Monoid Pattern)
// =============================================================================

/// Combined project statistics for single-pass aggregation.
#[derive(Clone, Default, Debug)]
struct ProjectStats {
    /// Status counts.
    status: ProgressStats,
    /// Priority counts.
    priority: PriorityCounts,
}

impl Semigroup for ProjectStats {
    fn combine(self, other: Self) -> Self {
        Self {
            status: self.status.combine(other.status),
            priority: self.priority.combine(other.priority),
        }
    }
}

impl Monoid for ProjectStats {
    fn empty() -> Self {
        Self::default()
    }
}

/// Pure: Converts a task summary to all stats.
fn task_to_all_stats(summary: &TaskSummary) -> ProjectStats {
    ProjectStats {
        status: task_to_stats(summary),
        priority: task_to_priority(summary),
    }
}

// =============================================================================
// Reader Utilities
// =============================================================================

/// Reader that extracts max tasks per project from config.
fn ask_max_tasks() -> Reader<AppConfig, usize> {
    Reader::asks(|config: AppConfig| config.max_tasks_per_project)
}

/// Reader that extracts default page size from config.
fn ask_page_size() -> Reader<AppConfig, u32> {
    Reader::asks(|config: AppConfig| config.default_page_size)
}

/// Composes multiple Reader computations to build project detail response.
///
/// Demonstrates `Reader::map2` for composing config-dependent computations.
fn build_detail_response(project: Project) -> Reader<AppConfig, ProjectDetailResponse> {
    ask_max_tasks().map2(ask_page_size(), move |max_tasks, page_size| {
        let task_summaries: Vec<_> = project
            .tasks
            .iter()
            .map(|(_, summary)| TaskSummaryResponse::from(summary))
            .collect();

        ProjectDetailResponse {
            project_id: project.project_id.to_string(),
            name: project.name.clone(),
            description: project.description.clone(),
            tasks: task_summaries,
            task_count: project.tasks.len(),
            max_tasks,
            default_page_size: page_size,
            created_at: project.created_at.to_string(),
            updated_at: project.updated_at.to_string(),
        }
    })
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Parses a project ID from a string.
fn parse_project_id(id: &str) -> Result<ProjectId, ApiErrorResponse> {
    Uuid::parse_str(id).map(ProjectId::from_uuid).map_err(|_| {
        ApiErrorResponse::bad_request("INVALID_PROJECT_ID", "Invalid project ID format")
    })
}

// =============================================================================
// POST /projects Handler
// =============================================================================

/// Creates a new project.
///
/// This handler demonstrates error-accumulating validation using
/// the **Applicative + Validated** pattern.
///
/// # lambars Features
///
/// - `Validated<A>`: Type alias for `Result<A, Vec<FieldError>>` enabling error accumulation
/// - `validated_map2`: Applicative combinator implementing `map2` semantics
/// - `Applicative::map2`: Combining independent computations with error collection
///
/// # Applicative Pattern for Validation
///
/// Unlike `Monad` (which is fail-fast and stops at the first error), `Applicative`
/// allows independent computations to be evaluated and their errors accumulated.
/// This provides a better user experience by reporting ALL validation errors at once.
///
/// ```text
/// // Monad (fail-fast): only first error reported
/// name_error = validate_name("")       // Err("Name required")
/// desc_error = validate_desc("x"*1001) // Never evaluated!
///
/// // Applicative (error-accumulating): all errors reported
/// name_error = validate_name("")       // Err(["Name required"])
/// desc_error = validate_desc("x"*1001) // Err(["Description too long"])
/// combined = validated_map2(name_error, desc_error, build)
/// // Err(["Name required", "Description too long"])
/// ```
///
/// # Request Body
///
/// ```json
/// {
///   "name": "Project Name",
///   "description": "Optional description"
/// }
/// ```
///
/// # Response
///
/// - **201 Created**: Project created successfully
/// - **400 Bad Request**: Validation error (accumulates all field errors)
/// - **500 Internal Server Error**: Database error
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] in the following cases:
/// - Validation error (400 Bad Request): Invalid name or description
/// - Database error (500 Internal Server Error): Repository operation failed
#[allow(clippy::future_not_send)]
pub async fn create_project_handler(
    State(state): State<AppState>,
    Json(request): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<ProjectResponse>), ApiErrorResponse> {
    // Step 1: Validate using Applicative pattern (accumulates all errors)
    let validated = match validate_create_project(&request) {
        Ok(v) => v,
        Err(field_errors) => {
            return Err(ApiErrorResponse::validation_error(
                "Validation failed",
                field_errors,
            ));
        }
    };

    // Step 2: Impure - generate IDs and timestamp
    let (project, response) = {
        let project_id = ProjectId::generate_v7();
        let now = Timestamp::now();
        let project = build_project_pure(project_id, validated, now);
        let response = ProjectResponse::from(&project);
        (project, response)
    };

    // Step 3: Impure - save to repository
    state
        .project_repository
        .save(&project)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    Ok((StatusCode::CREATED, Json(response)))
}

// =============================================================================
// GET /projects/{id} Handler
// =============================================================================

/// Gets project details.
///
/// This handler demonstrates `Reader` monad for configuration access.
///
/// # lambars Features
///
/// - `Reader`: Configuration-based dependency injection
/// - `Reader::map2`: Composing multiple config readers
///
/// # Response
///
/// - **200 OK**: Project details with tasks
/// - **404 Not Found**: Project not found
/// - **500 Internal Server Error**: Database error
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] in the following cases:
/// - Invalid project ID format (400 Bad Request)
/// - Project not found (404 Not Found)
/// - Database error (500 Internal Server Error)
#[allow(clippy::future_not_send)]
pub async fn get_project_handler(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<(HeaderMap, Json<ProjectDetailResponse>), ApiErrorResponse> {
    let project_id = parse_project_id(&project_id)?;

    let cache_result = state
        .project_repository
        .find_by_id_with_status(&project_id)
        .run_async()
        .await?;

    let cache_status = cache_result.cache_status;
    state.record_cache_status(cache_status);

    let project = cache_result
        .value
        .ok_or_else(|| ApiErrorResponse::not_found("Project not found"))?;

    let response = build_detail_response(project).run(state.config.clone());
    let headers = build_cache_headers(cache_status, CacheSource::Redis);

    Ok((headers, Json(response)))
}

// =============================================================================
// GET /projects/{id}/progress Handler
// =============================================================================

/// Gets project progress statistics.
///
/// This handler demonstrates stack-safe aggregation using `Foldable`, `Trampoline`,
/// and `Monoid` for combining progress stats.
///
/// # lambars Features
///
/// - `Foldable`: Type class for folding over data structures (via `to_list`)
/// - `Trampoline`: Stack-safe recursion for arbitrary-depth fold operations
/// - `Monoid`: Identity element and associative combination for progress stats
/// - `Semigroup`: Combining progress stats via `combine`
///
/// # Response
///
/// - **200 OK**: Progress statistics
/// - **404 Not Found**: Project not found
/// - **500 Internal Server Error**: Database error
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] in the following cases:
/// - Invalid project ID format (400 Bad Request)
/// - Project not found (404 Not Found)
/// - Database error (500 Internal Server Error)
#[allow(clippy::future_not_send)]
pub async fn get_project_progress_handler(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<ProjectProgressResponse>, ApiErrorResponse> {
    let project_id = parse_project_id(&project_id)?;

    let project = state
        .project_repository
        .find_by_id(&project_id)
        .run_async()
        .await?
        .ok_or_else(|| ApiErrorResponse::not_found("Project not found"))?;

    // Convert PersistentHashMap to Vec using Foldable::to_list for explicit Foldable usage
    let task_summaries: Vec<TaskSummary> = project.tasks.to_list();

    // Stack-safe fold using Trampoline + Foldable + Monoid
    // The trampoline_fold_map function performs a stack-safe fold operation
    let progress = trampoline_fold_map(task_summaries, |summary: TaskSummary| {
        task_to_stats(&summary)
    })
    .run();

    let completion = calculate_completion(&progress);

    Ok(Json(ProjectProgressResponse {
        project_id: project_id.to_string(),
        total_tasks: progress.total,
        pending_tasks: progress.pending,
        in_progress_tasks: progress.in_progress,
        completed_tasks: progress.completed,
        cancelled_tasks: progress.cancelled,
        completion_percentage: completion,
    }))
}

// =============================================================================
// GET /projects/{id}/stats Handler
// =============================================================================

/// Gets project statistics.
///
/// This handler demonstrates single-pass aggregation using record monoids.
///
/// # lambars Features
///
/// - `Semigroup`: Combining stats
/// - `Monoid`: Record monoid for single-pass aggregation
/// - Nested monoids (`ProjectStats` contains `ProgressStats` and `PriorityCounts`)
///
/// # Response
///
/// - **200 OK**: Project statistics
/// - **404 Not Found**: Project not found
/// - **500 Internal Server Error**: Database error
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] in the following cases:
/// - Invalid project ID format (400 Bad Request)
/// - Project not found (404 Not Found)
/// - Database error (500 Internal Server Error)
#[allow(clippy::future_not_send)]
pub async fn get_project_stats_handler(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<ProjectStatsResponse>, ApiErrorResponse> {
    let project_id = parse_project_id(&project_id)?;

    let project = state
        .project_repository
        .find_by_id(&project_id)
        .run_async()
        .await?
        .ok_or_else(|| ApiErrorResponse::not_found("Project not found"))?;

    // Single-pass aggregation using record monoid
    let aggregated = project
        .tasks
        .iter()
        .map(|(_, summary)| task_to_all_stats(summary))
        .fold(ProjectStats::empty(), Semigroup::combine);

    Ok(Json(ProjectStatsResponse {
        project_id: project_id.to_string(),
        total_tasks: aggregated.status.total,
        status_counts: StatusCountsResponse::from(aggregated.status),
        priority_counts: PriorityCountsResponse::from(aggregated.priority),
    }))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    use crate::domain::TaskId;

    // -------------------------------------------------------------------------
    // Validated Pattern Tests - Name Validation
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_validate_project_name_validated_success() {
        let result = validate_project_name_validated("My Project");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "My Project");
    }

    #[rstest]
    fn test_validate_project_name_validated_trims_whitespace() {
        let result = validate_project_name_validated("  Trimmed Name  ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Trimmed Name");
    }

    #[rstest]
    fn test_validate_project_name_validated_empty() {
        let result = validate_project_name_validated("");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "name");
        assert!(errors[0].message.contains("required"));
    }

    #[rstest]
    fn test_validate_project_name_validated_whitespace_only() {
        let result = validate_project_name_validated("   ");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "name");
    }

    #[rstest]
    fn test_validate_project_name_validated_too_long() {
        let long_name = "a".repeat(101);
        let result = validate_project_name_validated(&long_name);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "name");
        assert!(errors[0].message.contains("100 characters"));
    }

    #[rstest]
    fn test_validate_project_name_validated_boundary_100_chars() {
        let name_100 = "a".repeat(100);
        let result = validate_project_name_validated(&name_100);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 100);
    }

    // -------------------------------------------------------------------------
    // Validated Pattern Tests - Description Validation
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_validate_project_description_validated_none() {
        let result = validate_project_description_validated(None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[rstest]
    fn test_validate_project_description_validated_some() {
        let result = validate_project_description_validated(Some("Description"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("Description".to_string()));
    }

    #[rstest]
    fn test_validate_project_description_validated_too_long() {
        let long_desc = "a".repeat(1001);
        let result = validate_project_description_validated(Some(&long_desc));
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "description");
        assert!(errors[0].message.contains("1000 characters"));
    }

    #[rstest]
    fn test_validate_project_description_validated_boundary_1000_chars() {
        let desc_1000 = "a".repeat(1000);
        let result = validate_project_description_validated(Some(&desc_1000));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_ref().map(String::len), Some(1000));
    }

    // -------------------------------------------------------------------------
    // validated_map2 Tests - Applicative Combinator
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_validated_map2_both_ok() {
        let first: Validated<i32> = Ok(1);
        let second: Validated<i32> = Ok(2);
        let result = validated_map2(first, second, |a, b| a + b);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3);
    }

    #[rstest]
    fn test_validated_map2_first_error() {
        use super::super::error::FieldError;
        let first: Validated<i32> = Err(vec![FieldError::new("field1", "error1")]);
        let second: Validated<i32> = Ok(2);
        let result = validated_map2(first, second, |a, b| a + b);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "field1");
    }

    #[rstest]
    fn test_validated_map2_second_error() {
        use super::super::error::FieldError;
        let first: Validated<i32> = Ok(1);
        let second: Validated<i32> = Err(vec![FieldError::new("field2", "error2")]);
        let result = validated_map2(first, second, |a, b| a + b);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "field2");
    }

    #[rstest]
    fn test_validated_map2_accumulates_errors_from_both() {
        use super::super::error::FieldError;
        let first: Validated<i32> = Err(vec![FieldError::new("field1", "error1")]);
        let second: Validated<i32> = Err(vec![FieldError::new("field2", "error2")]);
        let result = validated_map2(first, second, |a, b| a + b);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        // Key property of Applicative: both errors are accumulated
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].field, "field1");
        assert_eq!(errors[1].field, "field2");
    }

    #[rstest]
    fn test_validated_map2_accumulates_multiple_errors_per_field() {
        use super::super::error::FieldError;
        let first: Validated<i32> = Err(vec![
            FieldError::new("field1", "error1a"),
            FieldError::new("field1", "error1b"),
        ]);
        let second: Validated<i32> = Err(vec![FieldError::new("field2", "error2")]);
        let result = validated_map2(first, second, |a, b| a + b);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 3);
    }

    // -------------------------------------------------------------------------
    // validate_create_project Tests - Applicative Error Accumulation
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_validate_create_project_valid() {
        let request = CreateProjectRequest {
            name: "Project".to_string(),
            description: Some("Description".to_string()),
        };
        let result = validate_create_project(&request);
        assert!(result.is_ok());
        let validated = result.unwrap();
        assert_eq!(validated.name, "Project");
        assert_eq!(validated.description, Some("Description".to_string()));
    }

    #[rstest]
    fn test_validate_create_project_valid_no_description() {
        let request = CreateProjectRequest {
            name: "Project".to_string(),
            description: None,
        };
        let result = validate_create_project(&request);
        assert!(result.is_ok());
        let validated = result.unwrap();
        assert!(validated.description.is_none());
    }

    #[rstest]
    fn test_validate_create_project_name_only_error() {
        let request = CreateProjectRequest {
            name: String::new(),
            description: Some("Valid description".to_string()),
        };
        let result = validate_create_project(&request);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "name");
    }

    #[rstest]
    fn test_validate_create_project_description_only_error() {
        let request = CreateProjectRequest {
            name: "Valid Name".to_string(),
            description: Some("a".repeat(1001)),
        };
        let result = validate_create_project(&request);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "description");
    }

    #[rstest]
    fn test_validate_create_project_accumulates_all_errors() {
        let request = CreateProjectRequest {
            name: String::new(),
            description: Some("a".repeat(1001)),
        };
        let result = validate_create_project(&request);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        // Key test: Applicative pattern accumulates ALL errors
        assert_eq!(errors.len(), 2);

        // Verify both field errors are present
        let field_names: Vec<&str> = errors.iter().map(|e| e.field.as_str()).collect();
        assert!(field_names.contains(&"name"));
        assert!(field_names.contains(&"description"));
    }

    // -------------------------------------------------------------------------
    // Applicative vs Monad Comparison Test
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_applicative_vs_monad_error_behavior() {
        // This test demonstrates the key difference between Applicative and Monad:
        //
        // Monad (fail-fast with Either/Result):
        //   - Stops at first error
        //   - Cannot report multiple errors
        //
        // Applicative (error-accumulating with Validated):
        //   - Evaluates all independent validations
        //   - Reports ALL errors at once

        let request = CreateProjectRequest {
            name: String::new(),                 // Invalid: empty
            description: Some("a".repeat(1001)), // Invalid: too long
        };

        // Our Validated-based implementation reports ALL errors
        let result = validate_create_project(&request);
        assert!(result.is_err());
        let errors = result.unwrap_err();

        // Both errors are reported (Applicative behavior)
        assert_eq!(
            errors.len(),
            2,
            "Applicative pattern should accumulate all errors"
        );

        // Compare to hypothetical Monad behavior (fail-fast):
        // If this were using flat_map/and_then, we would only see ONE error
        // because the second validation would never be evaluated after the first fails.
        //
        // The Applicative pattern is superior for validation because:
        // 1. Better UX: users see all validation errors at once
        // 2. Independence: field validations don't depend on each other
        // 3. Parallelizable: independent computations can run concurrently
    }

    // -------------------------------------------------------------------------
    // ProgressStats Monoid Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_progress_stats_empty() {
        let empty = ProgressStats::empty();
        assert_eq!(empty.total, 0);
        assert_eq!(empty.pending, 0);
        assert_eq!(empty.in_progress, 0);
        assert_eq!(empty.completed, 0);
        assert_eq!(empty.cancelled, 0);
    }

    #[rstest]
    fn test_progress_stats_combine() {
        let a = ProgressStats {
            total: 2,
            pending: 1,
            in_progress: 1,
            ..Default::default()
        };
        let b = ProgressStats {
            total: 3,
            completed: 2,
            cancelled: 1,
            ..Default::default()
        };
        let combined = a.combine(b);

        assert_eq!(combined.total, 5);
        assert_eq!(combined.pending, 1);
        assert_eq!(combined.in_progress, 1);
        assert_eq!(combined.completed, 2);
        assert_eq!(combined.cancelled, 1);
    }

    #[rstest]
    fn test_progress_stats_monoid_identity() {
        let stats = ProgressStats {
            total: 1,
            pending: 1,
            ..Default::default()
        };

        // Left identity
        let left = ProgressStats::empty().combine(stats.clone());
        assert_eq!(left.total, stats.total);

        // Right identity
        let right = stats.combine(ProgressStats::empty());
        assert_eq!(right.total, 1);
    }

    // -------------------------------------------------------------------------
    // task_to_stats Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_task_to_stats_pending() {
        let summary = TaskSummary::new(
            TaskId::generate(),
            "Task",
            TaskStatus::Pending,
            Priority::Low,
        );
        let stats = task_to_stats(&summary);

        assert_eq!(stats.total, 1);
        assert_eq!(stats.pending, 1);
        assert_eq!(stats.in_progress, 0);
    }

    #[rstest]
    fn test_task_to_stats_completed() {
        let summary = TaskSummary::new(
            TaskId::generate(),
            "Task",
            TaskStatus::Completed,
            Priority::High,
        );
        let stats = task_to_stats(&summary);

        assert_eq!(stats.total, 1);
        assert_eq!(stats.completed, 1);
    }

    // -------------------------------------------------------------------------
    // calculate_completion Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_calculate_completion_empty() {
        let stats = ProgressStats::empty();
        assert!((calculate_completion(&stats) - 100.0).abs() < f64::EPSILON);
    }

    #[rstest]
    fn test_calculate_completion_all_cancelled() {
        let stats = ProgressStats {
            total: 3,
            cancelled: 3,
            ..Default::default()
        };
        assert!((calculate_completion(&stats) - 100.0).abs() < f64::EPSILON);
    }

    #[rstest]
    fn test_calculate_completion_half_done() {
        let stats = ProgressStats {
            total: 4,
            pending: 2,
            completed: 2,
            ..Default::default()
        };
        assert!((calculate_completion(&stats) - 50.0).abs() < f64::EPSILON);
    }

    #[rstest]
    fn test_calculate_completion_excludes_cancelled() {
        let stats = ProgressStats {
            total: 4,
            pending: 1,
            completed: 1,
            cancelled: 2,
            ..Default::default()
        };
        // 2 active (1 pending, 1 completed), 1/2 = 50%
        assert!((calculate_completion(&stats) - 50.0).abs() < f64::EPSILON);
    }

    // -------------------------------------------------------------------------
    // PriorityCounts Monoid Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_priority_counts_empty() {
        let empty = PriorityCounts::empty();
        assert_eq!(empty.low, 0);
        assert_eq!(empty.medium, 0);
        assert_eq!(empty.high, 0);
        assert_eq!(empty.critical, 0);
    }

    #[rstest]
    fn test_priority_counts_combine() {
        let a = PriorityCounts {
            low: 1,
            high: 2,
            ..Default::default()
        };
        let b = PriorityCounts {
            medium: 1,
            critical: 1,
            ..Default::default()
        };
        let combined = a.combine(b);

        assert_eq!(combined.low, 1);
        assert_eq!(combined.medium, 1);
        assert_eq!(combined.high, 2);
        assert_eq!(combined.critical, 1);
    }

    // -------------------------------------------------------------------------
    // ProjectStats Monoid Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_project_stats_combine() {
        let a = ProjectStats {
            status: ProgressStats {
                total: 1,
                pending: 1,
                ..Default::default()
            },
            priority: PriorityCounts {
                low: 1,
                ..Default::default()
            },
        };
        let b = ProjectStats {
            status: ProgressStats {
                total: 1,
                completed: 1,
                ..Default::default()
            },
            priority: PriorityCounts {
                high: 1,
                ..Default::default()
            },
        };

        let combined = a.combine(b);
        assert_eq!(combined.status.total, 2);
        assert_eq!(combined.priority.low, 1);
        assert_eq!(combined.priority.high, 1);
    }

    // -------------------------------------------------------------------------
    // Reader Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_ask_max_tasks() {
        let config = AppConfig {
            max_tasks_per_project: 50,
            default_page_size: 10,
        };
        let result = ask_max_tasks().run(config);
        assert_eq!(result, 50);
    }

    #[rstest]
    fn test_ask_page_size() {
        let config = AppConfig {
            max_tasks_per_project: 100,
            default_page_size: 25,
        };
        let result = ask_page_size().run(config);
        assert_eq!(result, 25);
    }

    #[rstest]
    fn test_reader_map2_composition() {
        // Compose two readers using map2
        let combined = ask_max_tasks().map2(ask_page_size(), |max, page| (max, page));
        let config = AppConfig {
            max_tasks_per_project: 100,
            default_page_size: 20,
        };
        let result = combined.run(config);
        assert_eq!(result, (100, 20));
    }

    // -------------------------------------------------------------------------
    // parse_project_id Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_parse_project_id_valid() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        let result = parse_project_id(id);
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_parse_project_id_invalid() {
        let id = "not-a-uuid";
        let result = parse_project_id(id);
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // build_project_pure Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_build_project_pure_with_description() {
        let project_id = ProjectId::generate();
        let timestamp = Timestamp::now();
        let validated = ValidatedProject {
            name: "Test".to_string(),
            description: Some("Desc".to_string()),
        };

        let project = build_project_pure(project_id, validated, timestamp);
        assert_eq!(project.name, "Test");
        assert_eq!(project.description, Some("Desc".to_string()));
    }

    #[rstest]
    fn test_build_project_pure_without_description() {
        let project_id = ProjectId::generate();
        let timestamp = Timestamp::now();
        let validated = ValidatedProject {
            name: "Test".to_string(),
            description: None,
        };

        let project = build_project_pure(project_id, validated, timestamp);
        assert!(project.description.is_none());
    }

    // -------------------------------------------------------------------------
    // trampoline_fold_map Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_trampoline_fold_map_empty() {
        let items: Vec<TaskSummary> = vec![];
        let result =
            trampoline_fold_map(items, |summary: TaskSummary| task_to_stats(&summary)).run();

        assert_eq!(result.total, 0);
        assert_eq!(result.pending, 0);
        assert_eq!(result.in_progress, 0);
        assert_eq!(result.completed, 0);
        assert_eq!(result.cancelled, 0);
    }

    #[rstest]
    fn test_trampoline_fold_map_single_pending() {
        let summary = TaskSummary::new(
            TaskId::generate(),
            "Task",
            TaskStatus::Pending,
            Priority::Low,
        );
        let items = vec![summary];
        let result = trampoline_fold_map(items, |s: TaskSummary| task_to_stats(&s)).run();

        assert_eq!(result.total, 1);
        assert_eq!(result.pending, 1);
        assert_eq!(result.in_progress, 0);
        assert_eq!(result.completed, 0);
        assert_eq!(result.cancelled, 0);
    }

    #[rstest]
    fn test_trampoline_fold_map_multiple_statuses() {
        let items = vec![
            TaskSummary::new(
                TaskId::generate(),
                "Task1",
                TaskStatus::Pending,
                Priority::Low,
            ),
            TaskSummary::new(
                TaskId::generate(),
                "Task2",
                TaskStatus::InProgress,
                Priority::Medium,
            ),
            TaskSummary::new(
                TaskId::generate(),
                "Task3",
                TaskStatus::Completed,
                Priority::High,
            ),
            TaskSummary::new(
                TaskId::generate(),
                "Task4",
                TaskStatus::Cancelled,
                Priority::Critical,
            ),
        ];
        let result = trampoline_fold_map(items, |s: TaskSummary| task_to_stats(&s)).run();

        assert_eq!(result.total, 4);
        assert_eq!(result.pending, 1);
        assert_eq!(result.in_progress, 1);
        assert_eq!(result.completed, 1);
        assert_eq!(result.cancelled, 1);
    }

    #[rstest]
    fn test_trampoline_fold_map_stack_safety() {
        // Test with a large number of items to ensure stack safety
        let items: Vec<TaskSummary> = (0..1000)
            .map(|i| {
                TaskSummary::new(
                    TaskId::generate(),
                    format!("Task {i}"),
                    TaskStatus::Pending,
                    Priority::Low,
                )
            })
            .collect();

        let result = trampoline_fold_map(items, |s: TaskSummary| task_to_stats(&s)).run();

        assert_eq!(result.total, 1000);
        assert_eq!(result.pending, 1000);
    }

    #[rstest]
    fn test_trampoline_fold_map_monoid_identity() {
        // Test that empty + x == x (left identity)
        let summary = TaskSummary::new(
            TaskId::generate(),
            "Task",
            TaskStatus::Completed,
            Priority::High,
        );
        let items = vec![summary.clone()];

        let result = trampoline_fold_map(items, |s: TaskSummary| task_to_stats(&s)).run();
        let expected = task_to_stats(&summary);

        assert_eq!(result.total, expected.total);
        assert_eq!(result.completed, expected.completed);
    }

    #[rstest]
    fn test_trampoline_fold_map_associativity() {
        // Test that (a + b) + c == a + (b + c) (associativity via fold)
        let items = vec![
            TaskSummary::new(TaskId::generate(), "A", TaskStatus::Pending, Priority::Low),
            TaskSummary::new(
                TaskId::generate(),
                "B",
                TaskStatus::InProgress,
                Priority::Medium,
            ),
            TaskSummary::new(
                TaskId::generate(),
                "C",
                TaskStatus::Completed,
                Priority::High,
            ),
        ];

        let a = task_to_stats(&items[0]);
        let b = task_to_stats(&items[1]);
        let c = task_to_stats(&items[2]);

        // Left associative: (a + b) + c
        let left = a.clone().combine(b.clone()).combine(c.clone());

        // Right associative: a + (b + c)
        let right = a.combine(b.combine(c));

        // Verify associativity: (a + b) + c == a + (b + c)
        assert_eq!(left.total, right.total);
        assert_eq!(left.pending, right.pending);
        assert_eq!(left.in_progress, right.in_progress);
        assert_eq!(left.completed, right.completed);
        assert_eq!(left.cancelled, right.cancelled);

        // Also verify that trampoline_fold_map produces the same result
        let result = trampoline_fold_map(items, |s: TaskSummary| task_to_stats(&s)).run();
        assert_eq!(result.total, left.total);
        assert_eq!(result.pending, left.pending);
        assert_eq!(result.in_progress, left.in_progress);
        assert_eq!(result.completed, left.completed);
        assert_eq!(result.cancelled, left.cancelled);
    }
}

// =============================================================================
// Async Handler Tests
// =============================================================================

#[cfg(test)]
mod handler_tests {
    use super::*;
    use crate::api::query::{SearchCache, SearchIndex};
    use crate::domain::TaskId;
    use crate::infrastructure::{InMemoryEventStore, InMemoryProjectRepository};
    use arc_swap::ArcSwap;
    use lambars::persistent::PersistentVector;
    use rstest::rstest;
    use std::sync::Arc;

    fn create_test_app_state() -> AppState {
        use crate::api::bulk::BulkConfig;
        use crate::api::handlers::create_stub_external_sources;
        use crate::infrastructure::RngProvider;
        use std::sync::atomic::AtomicU64;

        let external_sources = create_stub_external_sources();

        AppState {
            task_repository: Arc::new(crate::infrastructure::InMemoryTaskRepository::new()),
            project_repository: Arc::new(InMemoryProjectRepository::new()),
            event_store: Arc::new(InMemoryEventStore::new()),
            config: AppConfig::default(),
            bulk_config: BulkConfig::default(),
            search_index: Arc::new(ArcSwap::from_pointee(SearchIndex::build(
                &PersistentVector::new(),
            ))),
            search_cache: Arc::new(SearchCache::with_default_config()),
            secondary_source: external_sources.secondary_source,
            external_source: external_sources.external_source,
            rng_provider: Arc::new(RngProvider::new_random()),
            cache_hits: Arc::new(AtomicU64::new(0)),
            cache_misses: Arc::new(AtomicU64::new(0)),
            cache_errors: Arc::new(AtomicU64::new(0)),
            cache_strategy: "read-through".to_string(),
            cache_ttl_seconds: 60,
        }
    }

    fn create_test_project_with_tasks(task_statuses: &[TaskStatus]) -> (Project, Vec<TaskSummary>) {
        let project_id = ProjectId::generate_v7();
        let timestamp = Timestamp::now();
        let mut project = Project::new(project_id, "Test Project", timestamp);

        let mut summaries = Vec::new();
        for (index, status) in task_statuses.iter().enumerate() {
            let summary = TaskSummary::new(
                TaskId::generate(),
                format!("Task {index}"),
                *status,
                Priority::Medium,
            );
            summaries.push(summary.clone());
            project = project.add_task(summary);
        }

        (project, summaries)
    }

    // -------------------------------------------------------------------------
    // GET /projects/{id} Handler Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_get_project_handler_returns_200_with_headers() {
        let state = create_test_app_state();
        let (project, _) = create_test_project_with_tasks(&[TaskStatus::Pending]);

        // Save the project
        state
            .project_repository
            .save(&project)
            .run_async()
            .await
            .expect("Failed to save project");

        // Call the handler
        let result = get_project_handler(
            axum::extract::State(state),
            axum::extract::Path(project.project_id.to_string()),
        )
        .await;

        assert!(result.is_ok());
        let (headers, response) = result.unwrap();

        // Verify response data
        assert_eq!(response.project_id, project.project_id.to_string());
        assert_eq!(response.name, "Test Project");
        assert_eq!(response.task_count, 1);

        // Verify cache headers are present with correct values
        assert!(headers.contains_key("X-Cache"));
        assert!(headers.contains_key("X-Cache-Status"));
        assert!(headers.contains_key("X-Cache-Source"));

        // Verify header values (mock returns CacheStatus::Bypass since no Redis layer)
        assert_eq!(headers.get("X-Cache").unwrap(), "MISS");
        assert_eq!(headers.get("X-Cache-Status").unwrap(), "bypass");
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_project_handler_returns_404_without_headers() {
        let state = create_test_app_state();
        let non_existent_id = ProjectId::generate_v7();

        // Call the handler
        let result = get_project_handler(
            axum::extract::State(state),
            axum::extract::Path(non_existent_id.to_string()),
        )
        .await;

        // Verify 404 error (no headers for error responses)
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.status, axum::http::StatusCode::NOT_FOUND);
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_project_handler_returns_400_for_invalid_id() {
        let state = create_test_app_state();

        // Call the handler with invalid UUID
        let result = get_project_handler(
            axum::extract::State(state),
            axum::extract::Path("not-a-valid-uuid".to_string()),
        )
        .await;

        // Verify 400 error (no headers for error responses)
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.status, axum::http::StatusCode::BAD_REQUEST);
    }

    // -------------------------------------------------------------------------
    // GET /projects/{id}/progress Handler Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_get_project_progress_handler_empty_project() {
        let state = create_test_app_state();
        let (project, _) = create_test_project_with_tasks(&[]);

        // Save the project
        state
            .project_repository
            .save(&project)
            .run_async()
            .await
            .expect("Failed to save project");

        // Call the handler
        let result = get_project_progress_handler(
            axum::extract::State(state),
            axum::extract::Path(project.project_id.to_string()),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert_eq!(response.total_tasks, 0);
        assert_eq!(response.pending_tasks, 0);
        assert_eq!(response.in_progress_tasks, 0);
        assert_eq!(response.completed_tasks, 0);
        assert_eq!(response.cancelled_tasks, 0);
        assert!((response.completion_percentage - 100.0).abs() < f64::EPSILON);
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_project_progress_handler_mixed_statuses() {
        let state = create_test_app_state();
        let (project, _) = create_test_project_with_tasks(&[
            TaskStatus::Pending,
            TaskStatus::InProgress,
            TaskStatus::Completed,
            TaskStatus::Cancelled,
        ]);

        // Save the project
        state
            .project_repository
            .save(&project)
            .run_async()
            .await
            .expect("Failed to save project");

        // Call the handler
        let result = get_project_progress_handler(
            axum::extract::State(state),
            axum::extract::Path(project.project_id.to_string()),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert_eq!(response.total_tasks, 4);
        assert_eq!(response.pending_tasks, 1);
        assert_eq!(response.in_progress_tasks, 1);
        assert_eq!(response.completed_tasks, 1);
        assert_eq!(response.cancelled_tasks, 1);
        // 3 active tasks (excluding cancelled), 1 completed = 33.33...%
        let expected_completion = (1.0 / 3.0) * 100.0;
        assert!((response.completion_percentage - expected_completion).abs() < 0.01);
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_project_progress_handler_all_completed() {
        let state = create_test_app_state();
        let (project, _) = create_test_project_with_tasks(&[
            TaskStatus::Completed,
            TaskStatus::Completed,
            TaskStatus::Completed,
        ]);

        // Save the project
        state
            .project_repository
            .save(&project)
            .run_async()
            .await
            .expect("Failed to save project");

        // Call the handler
        let result = get_project_progress_handler(
            axum::extract::State(state),
            axum::extract::Path(project.project_id.to_string()),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert_eq!(response.total_tasks, 3);
        assert_eq!(response.completed_tasks, 3);
        assert!((response.completion_percentage - 100.0).abs() < f64::EPSILON);
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_project_progress_handler_all_cancelled() {
        let state = create_test_app_state();
        let (project, _) =
            create_test_project_with_tasks(&[TaskStatus::Cancelled, TaskStatus::Cancelled]);

        // Save the project
        state
            .project_repository
            .save(&project)
            .run_async()
            .await
            .expect("Failed to save project");

        // Call the handler
        let result = get_project_progress_handler(
            axum::extract::State(state),
            axum::extract::Path(project.project_id.to_string()),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert_eq!(response.total_tasks, 2);
        assert_eq!(response.cancelled_tasks, 2);
        // All cancelled = 100% completion (no active tasks)
        assert!((response.completion_percentage - 100.0).abs() < f64::EPSILON);
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_project_progress_handler_not_found() {
        let state = create_test_app_state();
        let non_existent_id = ProjectId::generate_v7();

        // Call the handler
        let result = get_project_progress_handler(
            axum::extract::State(state),
            axum::extract::Path(non_existent_id.to_string()),
        )
        .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.status, axum::http::StatusCode::NOT_FOUND);
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_project_progress_handler_invalid_id() {
        let state = create_test_app_state();

        // Call the handler with invalid UUID
        let result = get_project_progress_handler(
            axum::extract::State(state),
            axum::extract::Path("not-a-valid-uuid".to_string()),
        )
        .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.status, axum::http::StatusCode::BAD_REQUEST);
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_project_progress_handler_half_completed() {
        let state = create_test_app_state();
        let (project, _) =
            create_test_project_with_tasks(&[TaskStatus::Completed, TaskStatus::Pending]);

        // Save the project
        state
            .project_repository
            .save(&project)
            .run_async()
            .await
            .expect("Failed to save project");

        // Call the handler
        let result = get_project_progress_handler(
            axum::extract::State(state),
            axum::extract::Path(project.project_id.to_string()),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert_eq!(response.total_tasks, 2);
        assert_eq!(response.completed_tasks, 1);
        assert_eq!(response.pending_tasks, 1);
        // 2 active tasks, 1 completed = 50%
        assert!((response.completion_percentage - 50.0).abs() < f64::EPSILON);
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_project_progress_handler_large_project() {
        let state = create_test_app_state();

        // Create a project with many tasks to test stack safety
        let project_id = ProjectId::generate_v7();
        let timestamp = Timestamp::now();
        let mut project = Project::new(project_id, "Large Project", timestamp);

        for i in 0..500 {
            let status = match i % 4 {
                0 => TaskStatus::Pending,
                1 => TaskStatus::InProgress,
                2 => TaskStatus::Completed,
                _ => TaskStatus::Cancelled,
            };
            let summary = TaskSummary::new(
                TaskId::generate(),
                format!("Task {i}"),
                status,
                Priority::Medium,
            );
            project = project.add_task(summary);
        }

        // Save the project
        state
            .project_repository
            .save(&project)
            .run_async()
            .await
            .expect("Failed to save project");

        // Call the handler
        let result = get_project_progress_handler(
            axum::extract::State(state),
            axum::extract::Path(project.project_id.to_string()),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert_eq!(response.total_tasks, 500);
        assert_eq!(response.pending_tasks, 125);
        assert_eq!(response.in_progress_tasks, 125);
        assert_eq!(response.completed_tasks, 125);
        assert_eq!(response.cancelled_tasks, 125);
    }
}
