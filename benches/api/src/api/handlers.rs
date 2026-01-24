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

use arc_swap::ArcSwap;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use lambars::control::Either;
use lambars::persistent::PersistentVector;
use uuid::Uuid;

use super::dto::{
    CreateTaskRequest, TaskResponse, validate_description, validate_tags, validate_title,
};
use super::error::ApiErrorResponse;
use super::query::{SearchCache, SearchIndex, TaskChange};
use crate::domain::{Priority, Tag, Task, TaskId, Timestamp};
use crate::infrastructure::{
    EventStore, Pagination, ProjectRepository, Repositories, TaskRepository,
};

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
///
/// # Search Index
///
/// The `search_index` field holds an immutable `SearchIndex` wrapped in `ArcSwap`.
/// This allows lock-free reads during search operations while supporting
/// atomic updates when tasks are created/updated/deleted.
///
/// - **Read**: `state.search_index.load()` returns a `Guard<Arc<SearchIndex>>`
/// - **Write**: `state.search_index.store(Arc::new(new_index))` atomically replaces the index
pub struct AppState {
    /// Task repository for persistence.
    pub task_repository: Arc<dyn TaskRepository + Send + Sync>,
    /// Project repository for project operations.
    pub project_repository: Arc<dyn ProjectRepository + Send + Sync>,
    /// Event store for event sourcing.
    pub event_store: Arc<dyn EventStore + Send + Sync>,
    /// Application configuration.
    pub config: AppConfig,
    /// Search index for task search (lock-free reads via `ArcSwap`).
    ///
    /// This index is built once at startup and updated incrementally
    /// when tasks are created, updated, or deleted.
    pub search_index: Arc<ArcSwap<SearchIndex>>,
    /// Search result cache (TTL 5s, LRU 2000 entries).
    ///
    /// Caches search results to improve performance for repeated queries.
    /// The cache key is `(normalized_query, scope, limit, offset)`.
    pub search_cache: Arc<SearchCache>,
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            task_repository: Arc::clone(&self.task_repository),
            project_repository: Arc::clone(&self.project_repository),
            event_store: Arc::clone(&self.event_store),
            config: self.config.clone(),
            search_index: Arc::clone(&self.search_index),
            search_cache: Arc::clone(&self.search_cache),
        }
    }
}

impl AppState {
    /// Creates a new `AppState` from initialized repositories.
    ///
    /// This constructor takes ownership of the `Repositories` struct returned
    /// by `RepositoryFactory::create()`.
    ///
    /// # Note
    ///
    /// This is an async function because it needs to fetch all tasks from the
    /// repository to build the initial search index. The index is built once
    /// at startup and updated incrementally thereafter.
    ///
    /// # Errors
    ///
    /// Returns an error if the task repository fails to list tasks.
    pub async fn from_repositories(
        repositories: Repositories,
    ) -> Result<Self, crate::infrastructure::RepositoryError> {
        Self::with_config(repositories, AppConfig::default()).await
    }

    /// Creates a new `AppState` from repositories and custom configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the task repository fails to list tasks.
    pub async fn with_config(
        repositories: Repositories,
        config: AppConfig,
    ) -> Result<Self, crate::infrastructure::RepositoryError> {
        // Fetch all tasks to build the initial search index
        let all_tasks = repositories
            .task_repository
            .list(Pagination::all())
            .run_async()
            .await?;

        // Build the search index from all tasks (pure function)
        let tasks: PersistentVector<Task> = all_tasks.items.into_iter().collect();
        let search_index = SearchIndex::build(&tasks);

        Ok(Self {
            task_repository: repositories.task_repository,
            project_repository: repositories.project_repository,
            event_store: repositories.event_store,
            config,
            search_index: Arc::new(ArcSwap::from_pointee(search_index)),
            search_cache: Arc::new(SearchCache::with_default_config()),
        })
    }

    /// Updates the search index with a task change.
    ///
    /// This method atomically replaces the search index with a new version
    /// that reflects the given change using Read-Copy-Update (RCU) pattern.
    /// The RCU pattern ensures that concurrent updates are handled correctly
    /// through CAS (Compare-And-Swap) retry, preventing lost updates.
    ///
    /// # Arguments
    ///
    /// * `change` - The task change to apply (Add, Update, or Remove).
    ///   Takes ownership because `rcu` may retry the closure multiple times,
    ///   requiring the change to be cloned on each retry.
    ///
    /// # Concurrency
    ///
    /// The `rcu` method provides atomic updates:
    /// 1. Read current value
    /// 2. Apply transformation (copy with modification)
    /// 3. Attempt CAS to replace the old value
    /// 4. If CAS fails (another thread updated), retry from step 1
    ///
    /// This ensures no updates are lost even under concurrent modifications.
    #[allow(clippy::needless_pass_by_value)] // Ownership needed for rcu retry via clone
    pub fn update_search_index(&self, change: TaskChange) {
        self.search_index.rcu(|current| {
            let updated = current.apply_change(change.clone());
            Arc::new(updated)
        });
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

    // Step 4: Update search index with the new task (lock-free write)
    state.update_search_index(TaskChange::Add(task));

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
// GET /tasks/{id} Handler
// =============================================================================

/// Gets a task by its ID.
///
/// This handler demonstrates the use of:
/// - **Either**: Lifting `Option<Task>` to `Either<ApiErrorResponse, Task>`
/// - **Pattern matching**: Functional error handling without exceptions
/// - **`AsyncIO`**: Encapsulating repository side effects
///
/// # Path Parameters
///
/// * `id` - The UUID of the task to retrieve
///
/// # Response
///
/// - **200 OK**: Task found and returned
/// - **404 Not Found**: Task with the given ID does not exist
/// - **500 Internal Server Error**: Database error
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] in the following cases:
/// - Not found error (404 Not Found): Task does not exist
/// - Database error (500 Internal Server Error): Repository operation failed
///
/// # lambars Features
///
/// The handler uses `Either<ApiErrorResponse, Task>` to represent the result
/// of the lookup operation. `Option<Task>` from `find_by_id` is lifted to
/// `Either` using pattern matching:
/// - `Some(task)` becomes `Either::Right(task)` (success)
/// - `None` becomes `Either::Left(ApiErrorResponse::not_found(...))` (failure)
#[allow(clippy::future_not_send)]
pub async fn get_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<TaskResponse>, ApiErrorResponse> {
    // Step 1: Convert Uuid to TaskId (pure)
    let task_id = TaskId::from_uuid(task_id);

    // Step 2: Fetch task from repository using AsyncIO
    let maybe_task = state
        .task_repository
        .find_by_id(&task_id)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    // Step 3: Lift Option<Task> to Either<ApiErrorResponse, Task>
    // This demonstrates functional error handling using Either
    let task_result: Either<ApiErrorResponse, Task> = lift_option_to_either(maybe_task, || {
        ApiErrorResponse::not_found(format!("Task not found: {task_id}"))
    });

    // Step 4: Convert Either to Result and map to response
    // Task is not Send, so we convert to TaskResponse (which is Send) immediately
    let result: Result<Task, ApiErrorResponse> = task_result.into();
    let task = result?;
    let response = TaskResponse::from(&task);

    Ok(Json(response))
}

/// Lifts an `Option<T>` to `Either<L, T>`.
///
/// This is a pure function that converts `Option` to `Either`:
/// - `Some(value)` becomes `Either::Right(value)`
/// - `None` becomes `Either::Left(left_value())` where `left_value` is lazily evaluated
///
/// # Type Parameters
///
/// * `L` - The type for the Left case (typically an error type)
/// * `T` - The type for the Right case (the success value)
/// * `F` - A function that produces the Left value when None is encountered
///
/// # Examples
///
/// ```ignore
/// let some_value = Some(42);
/// let result = lift_option_to_either(some_value, || "not found");
/// assert_eq!(result, Either::Right(42));
///
/// let none_value: Option<i32> = None;
/// let result = lift_option_to_either(none_value, || "not found");
/// assert_eq!(result, Either::Left("not found"));
/// ```
fn lift_option_to_either<L, T, F>(option: Option<T>, left_value: F) -> Either<L, T>
where
    F: FnOnce() -> L,
{
    option.map_or_else(|| Either::Left(left_value()), Either::Right)
}

// =============================================================================
// DELETE /tasks/{id} Handler
// =============================================================================

/// Deletes a task by its ID.
///
/// This handler demonstrates the use of:
/// - **Either**: Lifting `Option<Task>` to `Either<ApiErrorResponse, Task>`
/// - **Pattern matching**: Functional error handling without exceptions
/// - **`AsyncIO`**: Encapsulating repository side effects
/// - **Search index update**: Incremental index maintenance via RCU
///
/// # Path Parameters
///
/// * `id` - The UUID of the task to delete
///
/// # Response
///
/// - **204 No Content**: Task deleted successfully
/// - **404 Not Found**: Task with the given ID does not exist
/// - **500 Internal Server Error**: Database error
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] in the following cases:
/// - Not found error (404 Not Found): Task does not exist
/// - Database error (500 Internal Server Error): Repository operation failed
///
/// # Search Index
///
/// On successful deletion, the search index is updated atomically using the
/// RCU (Read-Copy-Update) pattern to remove the deleted task. This ensures
/// that search results are consistent with the actual data store.
#[allow(clippy::future_not_send)]
pub async fn delete_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<StatusCode, ApiErrorResponse> {
    // Step 1: Convert Uuid to TaskId (pure)
    let task_id = TaskId::from_uuid(task_id);

    // Step 2: Delete from repository using AsyncIO
    // The delete operation returns true if the task was found and deleted
    let deleted = state
        .task_repository
        .delete(&task_id)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    // Step 3: Check if deletion was successful
    if !deleted {
        return Err(ApiErrorResponse::not_found(format!(
            "Task not found: {task_id}"
        )));
    }

    // Step 4: Update search index to remove the deleted task (lock-free write via RCU)
    state.update_search_index(TaskChange::Remove(task_id));

    Ok(StatusCode::NO_CONTENT)
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

    use lambars::effect::AsyncIO;

    use crate::infrastructure::{
        InMemoryEventStore, InMemoryProjectRepository, InMemoryTaskRepository, RepositoryError,
    };

    // -------------------------------------------------------------------------
    // Mock TaskRepository for Error Simulation
    // -------------------------------------------------------------------------

    /// A mock `TaskRepository` that can be configured to return errors.
    ///
    /// This mock is used to test error handling paths in handlers.
    struct MockTaskRepository {
        /// The error to return from `find_by_id`, if any.
        find_by_id_error: Option<RepositoryError>,
        /// The task to return from `find_by_id`, if no error is configured.
        find_by_id_result: Option<Task>,
    }

    impl MockTaskRepository {
        /// Creates a mock that returns `Some(task)` from `find_by_id`.
        fn with_task(task: Task) -> Self {
            Self {
                find_by_id_error: None,
                find_by_id_result: Some(task),
            }
        }

        /// Creates a mock that returns `None` from `find_by_id`.
        fn not_found() -> Self {
            Self {
                find_by_id_error: None,
                find_by_id_result: None,
            }
        }

        /// Creates a mock that returns an error from `find_by_id`.
        fn with_error(error: RepositoryError) -> Self {
            Self {
                find_by_id_error: Some(error),
                find_by_id_result: None,
            }
        }
    }

    impl TaskRepository for MockTaskRepository {
        fn find_by_id(&self, _id: &TaskId) -> AsyncIO<Result<Option<Task>, RepositoryError>> {
            let error = self.find_by_id_error.clone();
            let result = self.find_by_id_result.clone();
            AsyncIO::new(move || async move { error.map_or_else(|| Ok(result), Err) })
        }

        fn save(&self, _task: &Task) -> AsyncIO<Result<(), RepositoryError>> {
            AsyncIO::new(|| async { Ok(()) })
        }

        fn delete(&self, _id: &TaskId) -> AsyncIO<Result<bool, RepositoryError>> {
            AsyncIO::new(|| async { Ok(false) })
        }

        fn list(
            &self,
            pagination: crate::infrastructure::Pagination,
        ) -> AsyncIO<Result<crate::infrastructure::PaginatedResult<Task>, RepositoryError>>
        {
            AsyncIO::new(move || async move {
                Ok(crate::infrastructure::PaginatedResult::new(
                    vec![],
                    0,
                    pagination.page,
                    pagination.page_size,
                ))
            })
        }

        fn count(&self) -> AsyncIO<Result<u64, RepositoryError>> {
            AsyncIO::new(|| async { Ok(0) })
        }
    }

    // -------------------------------------------------------------------------
    // Helper Functions for AppState Creation
    // -------------------------------------------------------------------------

    /// Creates an `AppState` with the given mock task repository.
    fn create_app_state_with_mock_task_repository(
        task_repository: impl TaskRepository + 'static,
    ) -> AppState {
        use crate::api::query::{SearchCache, SearchIndex};
        use arc_swap::ArcSwap;
        use lambars::persistent::PersistentVector;

        AppState {
            task_repository: Arc::new(task_repository),
            project_repository: Arc::new(InMemoryProjectRepository::new()),
            event_store: Arc::new(InMemoryEventStore::new()),
            config: AppConfig::default(),
            search_index: Arc::new(ArcSwap::from_pointee(SearchIndex::build(
                &PersistentVector::new(),
            ))),
            search_cache: Arc::new(SearchCache::with_default_config()),
        }
    }

    /// Creates an `AppState` with the default in-memory repositories.
    fn create_default_app_state() -> AppState {
        use crate::api::query::{SearchCache, SearchIndex};
        use arc_swap::ArcSwap;
        use lambars::persistent::PersistentVector;

        AppState {
            task_repository: Arc::new(InMemoryTaskRepository::new()),
            project_repository: Arc::new(InMemoryProjectRepository::new()),
            event_store: Arc::new(InMemoryEventStore::new()),
            config: AppConfig::default(),
            search_index: Arc::new(ArcSwap::from_pointee(SearchIndex::build(
                &PersistentVector::new(),
            ))),
            search_cache: Arc::new(SearchCache::with_default_config()),
        }
    }

    // -------------------------------------------------------------------------
    // get_task Handler Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_get_task_returns_200_when_task_found() {
        // Arrange
        let task_id = TaskId::generate();
        let task = Task::new(task_id.clone(), "Test Task", Timestamp::now())
            .with_description("Test description")
            .with_priority(Priority::High);
        let state = create_app_state_with_mock_task_repository(MockTaskRepository::with_task(task));

        // Act
        let result = get_task(State(state), Path(*task_id.as_uuid())).await;

        // Assert
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert_eq!(response.title, "Test Task");
        assert_eq!(response.description, Some("Test description".to_string()));
        assert_eq!(response.priority, super::super::dto::PriorityDto::High);
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_task_returns_404_when_task_not_found() {
        // Arrange
        let task_id = TaskId::generate();
        let state = create_app_state_with_mock_task_repository(MockTaskRepository::not_found());

        // Act
        let result = get_task(State(state), Path(*task_id.as_uuid())).await;

        // Assert
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.status, StatusCode::NOT_FOUND);
        assert_eq!(error.error.code, "NOT_FOUND");
        assert!(error.error.message.contains("Task not found"));
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_task_returns_500_when_repository_error() {
        // Arrange
        let task_id = TaskId::generate();
        let state = create_app_state_with_mock_task_repository(MockTaskRepository::with_error(
            RepositoryError::DatabaseError("Connection failed".to_string()),
        ));

        // Act
        let result = get_task(State(state), Path(*task_id.as_uuid())).await;

        // Assert
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(error.error.code, "INTERNAL_ERROR");
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_task_converts_task_to_task_response_correctly() {
        // Arrange
        let task_id = TaskId::generate();
        let timestamp = Timestamp::now();
        let task = Task::new(task_id.clone(), "Complete Task", timestamp)
            .with_description("Detailed description")
            .with_priority(Priority::Critical)
            .add_tag(Tag::new("urgent"))
            .add_tag(Tag::new("backend"));
        let state = create_app_state_with_mock_task_repository(MockTaskRepository::with_task(task));

        // Act
        let result = get_task(State(state), Path(*task_id.as_uuid())).await;

        // Assert
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert_eq!(response.id, task_id.to_string());
        assert_eq!(response.title, "Complete Task");
        assert_eq!(
            response.description,
            Some("Detailed description".to_string())
        );
        assert_eq!(response.priority, super::super::dto::PriorityDto::Critical);
        assert_eq!(response.tags.len(), 2);
        assert!(response.tags.contains(&"urgent".to_string()));
        assert!(response.tags.contains(&"backend".to_string()));
        assert_eq!(response.version, 1);
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_task_with_real_repository_integration() {
        // Arrange
        let state = create_default_app_state();
        let task_id = TaskId::generate();
        let task = Task::new(task_id.clone(), "Integration Test Task", Timestamp::now());

        // Save the task first
        state
            .task_repository
            .save(&task)
            .run_async()
            .await
            .expect("Failed to save task");

        // Act
        let result = get_task(State(state), Path(*task_id.as_uuid())).await;

        // Assert
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert_eq!(response.title, "Integration Test Task");
    }

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

    // -------------------------------------------------------------------------
    // lift_option_to_either Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_lift_option_to_either_some_returns_right() {
        let some_value: Option<i32> = Some(42);
        let result = lift_option_to_either(some_value, || "error");

        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), 42);
    }

    #[rstest]
    fn test_lift_option_to_either_none_returns_left() {
        let none_value: Option<i32> = None;
        let result = lift_option_to_either(none_value, || "not found");

        assert!(result.is_left());
        assert_eq!(result.unwrap_left(), "not found");
    }

    #[rstest]
    fn test_lift_option_to_either_left_value_is_lazy() {
        use std::cell::Cell;

        let call_count = Cell::new(0);
        let some_value: Option<i32> = Some(42);

        let _result = lift_option_to_either(some_value, || {
            call_count.set(call_count.get() + 1);
            "error"
        });

        // Left value function should not be called for Some case
        assert_eq!(call_count.get(), 0);
    }

    #[rstest]
    fn test_lift_option_to_either_with_api_error_response() {
        let none_value: Option<Task> = None;
        let task_id = TaskId::generate();

        let result: Either<ApiErrorResponse, Task> = lift_option_to_either(none_value, || {
            ApiErrorResponse::not_found(format!("Task not found: {task_id}"))
        });

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert_eq!(error.status, StatusCode::NOT_FOUND);
        assert_eq!(error.error.code, "NOT_FOUND");
    }

    #[rstest]
    fn test_lift_option_to_either_with_task() {
        let task = Task::new(TaskId::generate(), "Test Task", Timestamp::now());
        let some_task: Option<Task> = Some(task);

        let result: Either<ApiErrorResponse, Task> =
            lift_option_to_either(some_task, || ApiErrorResponse::not_found("Not found"));

        assert!(result.is_right());
        let returned_task = result.unwrap_right();
        assert_eq!(returned_task.title, "Test Task");
    }

    // -------------------------------------------------------------------------
    // delete_task Handler Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_delete_task_returns_204_when_task_exists() {
        // Arrange
        let state = create_default_app_state();
        let task_id = TaskId::generate();
        let task = Task::new(task_id.clone(), "Task to Delete", Timestamp::now());

        // Save the task first
        state
            .task_repository
            .save(&task)
            .run_async()
            .await
            .expect("Failed to save task");

        // Act
        let result = delete_task(State(state), Path(*task_id.as_uuid())).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
    }

    #[rstest]
    #[tokio::test]
    async fn test_delete_task_returns_404_when_task_not_found() {
        // Arrange
        let state = create_default_app_state();
        let nonexistent_task_id = TaskId::generate();

        // Act
        let result = delete_task(State(state), Path(*nonexistent_task_id.as_uuid())).await;

        // Assert
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.status, StatusCode::NOT_FOUND);
        assert_eq!(error.error.code, "NOT_FOUND");
        assert!(error.error.message.contains("Task not found"));
    }

    #[rstest]
    #[tokio::test]
    async fn test_delete_task_removes_from_repository() {
        // Arrange
        let state = create_default_app_state();
        let task_id = TaskId::generate();
        let task = Task::new(task_id.clone(), "Task to Delete", Timestamp::now());

        // Save the task first
        state
            .task_repository
            .save(&task)
            .run_async()
            .await
            .expect("Failed to save task");

        // Act
        let result = delete_task(State(state.clone()), Path(*task_id.as_uuid())).await;
        assert!(result.is_ok());

        // Assert: Task should no longer exist in repository
        let find_result = state
            .task_repository
            .find_by_id(&task_id)
            .run_async()
            .await
            .expect("Failed to find task");
        assert!(find_result.is_none());
    }
}
