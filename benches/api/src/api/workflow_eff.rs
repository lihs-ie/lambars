//! Effect-based workflow handlers using `ExceptT` + `AsyncIO` + `eff_async!` pattern.
//!
//! This module demonstrates lambars' effect system for HTTP handlers:
//! - **`ExceptT`**: Monad transformer for error handling
//! - **`AsyncIO`**: Asynchronous effect monad
//! - **`eff_async!`**: Do-notation macro for composing async effects
//!
//! # Pattern
//!
//! The `ExceptT<E, AsyncIO<Result<A, E>>>` pattern provides:
//! - Automatic error short-circuiting (like `?` operator but composable)
//! - Clean separation of pure computation and effects
//! - Monad laws compliance for predictable behavior
//!
//! # Example
//!
//! ```ignore
//! eff_async! {
//!     validated <= validate_request(request);
//!     task <= create_task(validated);
//!     _ <= save_task(&state, &task);
//!     ExceptT::pure_async_io(TaskResponse::from(task))
//! }
//! ```

use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};

use super::json_buffer::JsonResponse;
use lambars::eff_async;
use lambars::effect::{AsyncIO, ExceptT};

use super::dto::{
    CreateTaskRequest, TaskResponse, validate_description, validate_tags, validate_title,
};
use super::error::ApiErrorResponse;
use super::handlers::AppState;
use super::query::TaskChange;
use crate::domain::{Priority, Tag, Task, TaskId, Timestamp};
use crate::infrastructure::TaskRepository;

// =============================================================================
// Type Aliases for Effect Stack
// =============================================================================

/// Effect type for handlers using `ExceptT` + `AsyncIO`.
///
/// This type represents a computation that:
/// - May fail with `ApiErrorResponse`
/// - Executes asynchronously via `AsyncIO`
/// - Returns a value of type `A` on success
type HandlerEffect<A> = ExceptT<ApiErrorResponse, AsyncIO<Result<A, ApiErrorResponse>>>;

// =============================================================================
// POST /tasks-eff Handler
// =============================================================================

/// Creates a new task using the `ExceptT` + `AsyncIO` + `eff_async!` pattern.
///
/// This handler demonstrates functional programming patterns:
/// - **`ExceptT`**: Automatic error propagation without explicit `?`
/// - **`eff_async!`**: Do-notation for clean monadic composition
/// - **Separation of concerns**: Validation, construction, and persistence are distinct
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
#[allow(clippy::future_not_send)]
pub async fn create_task_eff(
    State(state): State<AppState>,
    Json(request): Json<CreateTaskRequest>,
) -> Result<(StatusCode, JsonResponse<TaskResponse>), ApiErrorResponse> {
    // Clone task_repository before the closure to avoid partial move of state
    let task_repository = state.task_repository.clone();

    // Build the effect pipeline using eff_async! macro
    // Returns (TaskResponse, Task) to enable search index update after success
    let effect: HandlerEffect<(TaskResponse, Task)> = eff_async! {
        // Step 1: Validate request (may fail with ApiErrorResponse)
        validated <= validate_request_eff(&request);

        // Step 2: Generate IDs (pure side effects wrapped in AsyncIO)
        ids <= generate_ids_eff();

        // Step 3: Build task (pure computation)
        let task = build_task_pure(ids, validated);

        // Step 4: Convert to response before async boundary (Task is not Send)
        let response = TaskResponse::from(&task);

        // Step 5: Save to repository (may fail with ApiErrorResponse)
        _ <= save_task_eff(task_repository.clone(), task.clone());

        // Return both response and task for search index update
        ExceptT::pure_async_io((response, task))
    };

    // Execute the effect and convert to HTTP response
    match effect.run_async().await {
        Ok((response, task)) => {
            // Step 6: Update search index with the new task (lock-free write via RCU)
            state.update_search_index(TaskChange::Add(task));
            Ok((StatusCode::CREATED, JsonResponse(response)))
        }
        Err(error) => Err(error),
    }
}

// =============================================================================
// Effect Combinators
// =============================================================================

/// Validates a create task request, returning an effect.
///
/// Uses `ExceptT::from_result` to lift validation results into the effect stack.
fn validate_request_eff(request: &CreateTaskRequest) -> HandlerEffect<ValidatedCreateTask> {
    // Perform validation (pure functions returning Either)
    let title_result = validate_title(&request.title);
    let desc_result = validate_description(request.description.as_deref());
    let tags_result = validate_tags(&request.tags);

    // Convert Either to Result for ExceptT
    let title: Result<String, ApiErrorResponse> =
        title_result.map_left(ApiErrorResponse::from).into();
    let description: Result<Option<String>, ApiErrorResponse> =
        desc_result.map_left(ApiErrorResponse::from).into();
    let tags: Result<Vec<Tag>, ApiErrorResponse> =
        tags_result.map_left(ApiErrorResponse::from).into();

    // Combine results
    match (title, description, tags) {
        (Ok(title), Ok(description), Ok(tags)) => ExceptT::pure_async_io(ValidatedCreateTask {
            title,
            description,
            priority: Priority::from(request.priority),
            tags,
        }),
        (Err(error), _, _) | (_, Err(error), _) | (_, _, Err(error)) => {
            ExceptT::throw_async_io(error)
        }
    }
}

/// Generates task IDs, returning an effect.
///
/// ID generation is an impure operation (side effect), so we wrap it in `AsyncIO`
/// to defer execution until the effect is run. This preserves referential transparency.
fn generate_ids_eff() -> HandlerEffect<TaskIds> {
    ExceptT::new(AsyncIO::new(|| async {
        Ok(TaskIds {
            task_id: TaskId::generate_v7(),
            timestamp: Timestamp::now(),
        })
    }))
}

/// Saves a task to the repository, returning an effect.
///
/// Repository operations are async side effects that may fail.
fn save_task_eff(
    repository: Arc<dyn TaskRepository + Send + Sync>,
    task: Task,
) -> HandlerEffect<()> {
    ExceptT::new(AsyncIO::new(move || {
        let repository = repository.clone();
        async move {
            repository
                .save(&task)
                .run_async()
                .await
                .map_err(ApiErrorResponse::from)
        }
    }))
}

// =============================================================================
// Pure Computation Functions
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

/// Builds a task from validated data and generated IDs.
///
/// This is a pure function with no side effects.
/// Uses functional composition (fold, match) to avoid mutable state.
fn build_task_pure(ids: TaskIds, validated: ValidatedCreateTask) -> Task {
    let base_task = Task::new(ids.task_id, validated.title, ids.timestamp);

    // Apply description if present (functional pattern using match)
    let with_description = match validated.description {
        Some(desc) => base_task.with_description(desc),
        None => base_task,
    };

    // Apply priority
    let with_priority = with_description.with_priority(validated.priority);

    // Apply tags using fold (functional iteration)
    validated
        .tags
        .into_iter()
        .fold(with_priority, Task::add_tag)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    use crate::api::dto::PriorityDto;

    // -------------------------------------------------------------------------
    // Validation Effect Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_validate_request_eff_valid() {
        let request = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: Some("Description".to_string()),
            priority: PriorityDto::High,
            tags: vec!["backend".to_string()],
        };

        let result = validate_request_eff(&request).run_async().await;
        assert!(result.is_ok());

        let validated = result.unwrap();
        assert_eq!(validated.title, "Test Task");
        assert_eq!(validated.description, Some("Description".to_string()));
        assert_eq!(validated.priority, Priority::High);
        assert_eq!(validated.tags.len(), 1);
    }

    #[rstest]
    #[tokio::test]
    async fn test_validate_request_eff_empty_title() {
        let request = CreateTaskRequest {
            title: String::new(),
            description: None,
            priority: PriorityDto::Low,
            tags: vec![],
        };

        let result = validate_request_eff(&request).run_async().await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
    }

    #[rstest]
    #[tokio::test]
    async fn test_validate_request_eff_invalid_tags() {
        let request = CreateTaskRequest {
            title: "Valid Title".to_string(),
            description: None,
            priority: PriorityDto::Low,
            tags: vec![String::new()],
        };

        let result = validate_request_eff(&request).run_async().await;
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // ID Generation Effect Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_generate_ids_eff_produces_unique_ids() {
        let ids1 = generate_ids_eff().run_async().await.unwrap();
        let ids2 = generate_ids_eff().run_async().await.unwrap();

        assert_ne!(ids1.task_id, ids2.task_id);
    }

    // -------------------------------------------------------------------------
    // Pure Function Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_build_task_pure_with_all_fields() {
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

        let task = build_task_pure(ids, validated);

        assert_eq!(task.title, "Test Task");
        assert_eq!(task.description, Some("Description".to_string()));
        assert_eq!(task.priority, Priority::High);
        assert_eq!(task.tags.len(), 2);
    }

    #[rstest]
    fn test_build_task_pure_minimal() {
        let ids = TaskIds {
            task_id: TaskId::generate(),
            timestamp: Timestamp::now(),
        };

        let validated = ValidatedCreateTask {
            title: "Minimal Task".to_string(),
            description: None,
            priority: Priority::Low,
            tags: vec![],
        };

        let task = build_task_pure(ids, validated);

        assert_eq!(task.title, "Minimal Task");
        assert!(task.description.is_none());
        assert!(task.tags.is_empty());
    }

    // -------------------------------------------------------------------------
    // Effect Composition Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_eff_async_composition() {
        // Test that eff_async! properly chains effects
        let effect: HandlerEffect<i32> = eff_async! {
            x <= ExceptT::<ApiErrorResponse, AsyncIO<Result<i32, ApiErrorResponse>>>::pure_async_io(10);
            y <= ExceptT::<ApiErrorResponse, AsyncIO<Result<i32, ApiErrorResponse>>>::pure_async_io(20);
            let sum = x + y;
            ExceptT::<ApiErrorResponse, AsyncIO<Result<i32, ApiErrorResponse>>>::pure_async_io(sum)
        };

        let result = effect.run_async().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 30);
    }

    #[rstest]
    #[tokio::test]
    async fn test_eff_async_short_circuits_on_error() {
        let effect: HandlerEffect<i32> = eff_async! {
            x <= ExceptT::<ApiErrorResponse, AsyncIO<Result<i32, ApiErrorResponse>>>::pure_async_io(10);
            _ <= ExceptT::<ApiErrorResponse, AsyncIO<Result<i32, ApiErrorResponse>>>::throw_async_io(
                ApiErrorResponse::bad_request("TEST", "Test error")
            );
            // This line should not be executed
            ExceptT::<ApiErrorResponse, AsyncIO<Result<i32, ApiErrorResponse>>>::pure_async_io(x * 2)
        };

        let result = effect.run_async().await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().error.code, "TEST");
    }
}
