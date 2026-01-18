//! Project handlers demonstrating lambars functional programming features.
//!
//! This module contains HTTP handlers for project management operations,
//! showcasing the following lambars features:
//!
//! - **Semigroup/Monoid**: Error accumulation in validation, progress aggregation
//! - **Reader**: Configuration-based dependency injection
//! - **Either**: Validation result representation
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
    http::StatusCode,
};
use lambars::control::Either::{self, Left, Right};
use lambars::effect::Reader;
use lambars::typeclass::{Monoid, Semigroup};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::dto::{PriorityDto, TaskStatusDto};
use super::error::{ApiErrorResponse, ValidationError};
use super::handlers::{AppConfig, AppState};
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

/// Result type for field validation using Either.
type FieldResult<T> = Either<ValidationError, T>;

// =============================================================================
// Validation Functions (Pure)
// =============================================================================

/// Validates project name.
///
/// Name must be 1-100 characters (whitespace-only names are rejected).
fn validate_project_name(name: &str) -> FieldResult<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        Left(ValidationError::single("name", "Name is required"))
    } else if trimmed.len() > 100 {
        Left(ValidationError::single(
            "name",
            "Name must be at most 100 characters",
        ))
    } else {
        Right(trimmed.to_string())
    }
}

/// Validates project description.
///
/// Description is optional but must be at most 1000 characters if provided.
fn validate_project_description(desc: Option<&str>) -> FieldResult<Option<String>> {
    match desc {
        None => Right(None),
        Some(d) if d.len() > 1000 => Left(ValidationError::single(
            "description",
            "Description must be at most 1000 characters",
        )),
        Some(d) => Right(Some(d.to_string())),
    }
}

/// Validates create project request, accumulating all errors.
///
/// Uses `Semigroup::combine` to accumulate multiple validation errors.
fn validate_create_project(request: &CreateProjectRequest) -> FieldResult<ValidatedProject> {
    let name_result = validate_project_name(&request.name);
    let desc_result = validate_project_description(request.description.as_deref());

    match (name_result, desc_result) {
        (Right(name), Right(description)) => Right(ValidatedProject { name, description }),
        (Left(e1), Left(e2)) => Left(e1.combine(e2)),
        (Left(e), _) | (_, Left(e)) => Left(e),
    }
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
/// `Semigroup::combine` on `ValidationError`.
///
/// # lambars Features
///
/// - `Either`: Validation result representation
/// - `Semigroup`: Accumulating multiple validation errors
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
    // Step 1: Validate (accumulates all errors)
    let validated = match validate_create_project(&request) {
        Right(v) => v,
        Left(errors) => return Err(errors.into()),
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
) -> Result<Json<ProjectDetailResponse>, ApiErrorResponse> {
    let project_id = parse_project_id(&project_id)?;

    let project = state
        .project_repository
        .find_by_id(&project_id)
        .run_async()
        .await?
        .ok_or_else(|| ApiErrorResponse::not_found("Project not found"))?;

    // Use Reader to compose config-dependent response building
    let response = build_detail_response(project).run(state.config.clone());
    Ok(Json(response))
}

// =============================================================================
// GET /projects/{id}/progress Handler
// =============================================================================

/// Gets project progress statistics.
///
/// This handler demonstrates `Monoid` for aggregating progress stats.
///
/// # lambars Features
///
/// - `Semigroup`: Combining progress stats
/// - `Monoid`: Empty value for fold operations
/// - Iterator-based fold (no Vec allocation)
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

    // Iterator-based fold using Semigroup::combine (no Vec allocation)
    let progress = project
        .tasks
        .iter()
        .map(|(_, summary)| task_to_stats(summary))
        .fold(ProgressStats::empty(), Semigroup::combine);

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
    // Validation Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_validate_project_name_valid() {
        let result = validate_project_name("My Project");
        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), "My Project");
    }

    #[rstest]
    fn test_validate_project_name_empty() {
        let result = validate_project_name("");
        assert!(result.is_left());
        let error = result.unwrap_left();
        assert_eq!(error.errors[0].field, "name");
    }

    #[rstest]
    fn test_validate_project_name_too_long() {
        let long_name = "a".repeat(101);
        let result = validate_project_name(&long_name);
        assert!(result.is_left());
    }

    #[rstest]
    fn test_validate_project_description_none() {
        let result = validate_project_description(None);
        assert!(result.is_right());
        assert!(result.unwrap_right().is_none());
    }

    #[rstest]
    fn test_validate_project_description_valid() {
        let result = validate_project_description(Some("Description"));
        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), Some("Description".to_string()));
    }

    #[rstest]
    fn test_validate_project_description_too_long() {
        let long_desc = "a".repeat(1001);
        let result = validate_project_description(Some(&long_desc));
        assert!(result.is_left());
    }

    #[rstest]
    fn test_validate_create_project_valid() {
        let request = CreateProjectRequest {
            name: "Project".to_string(),
            description: Some("Description".to_string()),
        };
        let result = validate_create_project(&request);
        assert!(result.is_right());
    }

    #[rstest]
    fn test_validate_create_project_accumulates_errors() {
        let request = CreateProjectRequest {
            name: String::new(),
            description: Some("a".repeat(1001)),
        };
        let result = validate_create_project(&request);
        assert!(result.is_left());

        let errors = result.unwrap_left();
        assert_eq!(errors.errors.len(), 2);
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
}
