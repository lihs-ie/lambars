//! HTTP handlers for the Task Management API.
//!
//! This module demonstrates the use of lambars features in HTTP handlers:
//! - Functor: Mapping over `Either` for DTO validation
//! - Monad: Chaining validation results
//! - Either: Representing validation success/failure
//! - `AsyncIO`: Encapsulating side effects
//!
//! # Note on Send bounds
//!
//! lambars' persistent data structures (`PersistentHashSet`, `PersistentList`)
//! use `Rc` internally and are not `Send`. Therefore, `Task` cannot cross
//! await boundaries. We handle this by:
//! 1. Creating/processing `Task` synchronously in a block
//! 2. Executing async operations separately
//! 3. Converting `Task` to `TaskResponse` (which is `Send`) before returning

use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};

use super::dto::{
    CreateTaskRequest, TaskResponse, validate_description, validate_tags, validate_title,
};
use super::error::ApiErrorResponse;
use crate::domain::{Priority, Tag, Task, TaskId, Timestamp};
use crate::infrastructure::{EventStore, ProjectRepository, Repositories, TaskRepository};

// =============================================================================
// Application Configuration
// =============================================================================

/// Application configuration for runtime settings.
///
/// This struct is used with `Reader` monad to demonstrate dependency injection
/// patterns in functional programming.
///
/// # lambars Features
///
/// - `Reader`: Configuration is accessed via `Reader<AppConfig, A>` for
///   composable dependency injection
#[derive(Clone, Debug)]
pub struct AppConfig {
    /// Maximum number of tasks allowed per project.
    pub max_tasks_per_project: usize,
    /// Default page size for pagination.
    pub default_page_size: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            max_tasks_per_project: 100,
            default_page_size: 20,
        }
    }
}

// =============================================================================
// Application State
// =============================================================================

/// Shared application dependencies.
///
/// Uses trait objects (`dyn`) instead of generics to work seamlessly with
/// the `RepositoryFactory` which returns trait objects. This design allows
/// runtime selection of repository backends (in-memory, `PostgreSQL`, Redis).
#[derive(Clone)]
pub struct AppState {
    /// Task repository for persistence.
    pub task_repository: Arc<dyn TaskRepository + Send + Sync>,
    /// Project repository for project operations.
    pub project_repository: Arc<dyn ProjectRepository + Send + Sync>,
    /// Event store for event sourcing.
    pub event_store: Arc<dyn EventStore + Send + Sync>,
    /// Application configuration.
    pub config: AppConfig,
}

impl AppState {
    /// Creates a new `AppState` from initialized repositories.
    ///
    /// This constructor takes ownership of the `Repositories` struct returned
    /// by `RepositoryFactory::create()`.
    #[must_use]
    pub fn from_repositories(repositories: Repositories) -> Self {
        Self {
            task_repository: repositories.task_repository,
            project_repository: repositories.project_repository,
            event_store: repositories.event_store,
            config: AppConfig::default(),
        }
    }

    /// Creates a new `AppState` from repositories and custom configuration.
    #[must_use]
    pub fn with_config(repositories: Repositories, config: AppConfig) -> Self {
        Self {
            task_repository: repositories.task_repository,
            project_repository: repositories.project_repository,
            event_store: repositories.event_store,
            config,
        }
    }
}

// =============================================================================
// POST /tasks Handler
// =============================================================================

/// Creates a new task.
///
/// This handler demonstrates the use of:
/// - **Functor**: Mapping over `Either` for DTO validation
/// - **Monad**: Chaining validation results with `map_left`
/// - **Either**: Representing validation success/failure
/// - **`AsyncIO`**: Encapsulating repository side effects
///
/// # Request Body
///
/// ```json
/// {
///   "title": "Task title",
///   "description": "Optional description",
///   "priority": "low|medium|high|critical",
///   "tags": ["tag1", "tag2"]
/// }
/// ```
///
/// # Response
///
/// - **201 Created**: Task created successfully
/// - **400 Bad Request**: Validation error
/// - **500 Internal Server Error**: Database error
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] in the following cases:
/// - Validation error (400 Bad Request): Invalid title, description, or tags
/// - Database error (500 Internal Server Error): Repository operation failed
///
/// # Note on Send bounds
///
/// `Task` is not `Send` because it contains `PersistentHashSet` and `PersistentList`
/// which use `Rc`. We handle this by:
/// 1. Creating the task synchronously
/// 2. Converting to `TaskResponse` before the async boundary
/// 3. Executing the repository save in a separate async block
#[allow(clippy::future_not_send)]
pub async fn create_task(
    State(state): State<AppState>,
    Json(request): Json<CreateTaskRequest>,
) -> Result<(StatusCode, Json<TaskResponse>), ApiErrorResponse> {
    // Step 1: Validate using Either (demonstrates Monad chaining)
    let validated = validate_create_request(&request)?;

    // Step 2: Create task synchronously (Task is not Send)
    // Generate IDs and timestamp within this block (impure operations)
    let (task, response) = {
        let ids = generate_task_ids();
        let task = build_task(ids, validated);
        let response = TaskResponse::from(&task);
        (task, response)
    };

    // Step 3: Save to repository using AsyncIO
    // The task reference is consumed here before the await
    state
        .task_repository
        .save(&task)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    Ok((StatusCode::CREATED, Json(response)))
}

// =============================================================================
// Helper Types and Functions
// =============================================================================

/// Validated create task data.
#[derive(Debug)]
struct ValidatedCreateTask {
    title: String,
    description: Option<String>,
    priority: Priority,
    tags: Vec<Tag>,
}

/// Generated IDs for a new task.
struct TaskIds {
    task_id: TaskId,
    timestamp: Timestamp,
}

/// Validates a create task request.
///
/// Uses `Either` monad for validation, converting to `Result` at the boundary.
fn validate_create_request(
    request: &CreateTaskRequest,
) -> Result<ValidatedCreateTask, ApiErrorResponse> {
    // Chain validations using Either's monadic properties
    let title_result = validate_title(&request.title);
    let desc_result = validate_description(request.description.as_deref());
    let tags_result = validate_tags(&request.tags);

    // Combine validation results
    // Using map_left to convert ValidationError to ApiErrorResponse
    let title: Result<String, ApiErrorResponse> =
        title_result.map_left(ApiErrorResponse::from).into();
    let title = title?;

    let description: Result<Option<String>, ApiErrorResponse> =
        desc_result.map_left(ApiErrorResponse::from).into();
    let description = description?;

    let tags: Result<Vec<Tag>, ApiErrorResponse> =
        tags_result.map_left(ApiErrorResponse::from).into();
    let tags = tags?;

    Ok(ValidatedCreateTask {
        title,
        description,
        priority: Priority::from(request.priority),
        tags,
    })
}

/// Generates task IDs within an effect boundary.
///
/// Note: This function contains impure operations (UUID generation, timestamp).
fn generate_task_ids() -> TaskIds {
    TaskIds {
        task_id: TaskId::generate_v7(),
        timestamp: Timestamp::now(),
    }
}

/// Builds a task from validated data and generated IDs.
///
/// This is a pure function.
fn build_task(ids: TaskIds, validated: ValidatedCreateTask) -> Task {
    let mut task = Task::new(ids.task_id, validated.title, ids.timestamp);

    if let Some(desc) = validated.description {
        task = task.with_description(desc);
    }

    task = task.with_priority(validated.priority);

    for tag in validated.tags {
        task = task.add_tag(tag);
    }

    task
}

// =============================================================================
// GET /health Handler
// =============================================================================

/// Health check response body.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthResponse {
    /// Service status.
    pub status: &'static str,
    /// Service version.
    pub version: &'static str,
}

/// Health check endpoint.
///
/// Returns a simple JSON response indicating the service is running.
/// This endpoint can be used by load balancers and orchestration systems
/// to verify service availability.
///
/// # Response
///
/// - **200 OK**: Service is healthy
///
/// ```json
/// {
///   "status": "healthy",
///   "version": "0.1.0"
/// }
/// ```
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // Validation Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_validate_create_request_valid() {
        let request = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: Some("Description".to_string()),
            priority: super::super::dto::PriorityDto::High,
            tags: vec!["backend".to_string()],
        };

        let result = validate_create_request(&request);
        assert!(result.is_ok());

        let validated = result.unwrap();
        assert_eq!(validated.title, "Test Task");
        assert_eq!(validated.description, Some("Description".to_string()));
        assert_eq!(validated.priority, Priority::High);
        assert_eq!(validated.tags.len(), 1);
    }

    #[rstest]
    fn test_validate_create_request_empty_title() {
        let request = CreateTaskRequest {
            title: String::new(),
            description: None,
            priority: super::super::dto::PriorityDto::Low,
            tags: vec![],
        };

        let result = validate_create_request(&request);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
    }

    #[rstest]
    fn test_validate_create_request_invalid_tags() {
        let request = CreateTaskRequest {
            title: "Valid Title".to_string(),
            description: None,
            priority: super::super::dto::PriorityDto::Low,
            tags: vec![String::new()], // Empty tag
        };

        let result = validate_create_request(&request);
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // Build Task Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_build_task() {
        let ids = TaskIds {
            task_id: TaskId::generate(),
            timestamp: Timestamp::now(),
        };

        let validated = ValidatedCreateTask {
            title: "Test Task".to_string(),
            description: Some("Description".to_string()),
            priority: Priority::High,
            tags: vec![Tag::new("backend"), Tag::new("urgent")],
        };

        let task = build_task(ids, validated);

        assert_eq!(task.title, "Test Task");
        assert_eq!(task.description, Some("Description".to_string()));
        assert_eq!(task.priority, Priority::High);
        assert_eq!(task.tags.len(), 2);
        assert_eq!(task.version, 1);
    }

    #[rstest]
    fn test_build_task_no_description() {
        let ids = TaskIds {
            task_id: TaskId::generate(),
            timestamp: Timestamp::now(),
        };

        let validated = ValidatedCreateTask {
            title: "Test Task".to_string(),
            description: None,
            priority: Priority::Low,
            tags: vec![],
        };

        let task = build_task(ids, validated);

        assert!(task.description.is_none());
        assert!(task.tags.is_empty());
    }

    // -------------------------------------------------------------------------
    // Generate IDs Test
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_generate_task_ids_unique() {
        let ids1 = generate_task_ids();
        let ids2 = generate_task_ids();

        assert_ne!(ids1.task_id, ids2.task_id);
    }
}
