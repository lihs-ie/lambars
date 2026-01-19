//! Bifunctor-based transformation operations.
//!
//! This module demonstrates:
//! - **`bimap`**: Apply functions to both type parameters simultaneously
//! - **`first`**: Apply function to the first type parameter (error side)
//! - **`second`**: Apply function to the second type parameter (success side)
//!
//! # lambars Features Demonstrated
//!
//! - **`Bifunctor` trait**: Mapping over two type parameters
//! - **`Result<T, E>` as `Bifunctor<E, T>`**: first = error, second = success
//! - **`Either<L, R>` as `Bifunctor<L, R>`**: first = Left, second = Right
//! - **Tuple `(A, B)` as `Bifunctor<A, B>`**: first = first element, second = second element
//! - **Reference variants**: `bimap_ref`, `first_ref`, `second_ref`

use std::collections::HashMap;
use std::time::Instant;

use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};

use lambars::typeclass::Bifunctor;

use super::dto::TaskResponse;
use super::error::{ApiErrorResponse, FieldError};
use super::handlers::AppState;
use crate::domain::{Tag, Task, TaskId, Timestamp};

// =============================================================================
// Error Types for Bifunctor Demonstrations
// =============================================================================

/// Internal processing error (domain layer).
#[derive(Debug, Clone)]
pub enum ProcessingError {
    NotFound(String),
    ValidationFailed(String),
    Conflict(String),
    InternalError(String),
}

/// User-friendly error for API responses.
#[derive(Debug, Clone, Serialize)]
pub struct UserFriendlyError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HashMap<String, String>>,
}

/// Error enriched with context information.
#[derive(Debug, Clone, Serialize)]
pub struct EnrichedError {
    pub original_message: String,
    pub error_code: String,
    pub request_id: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

/// Domain-level error for internal operations.
#[derive(Debug, Clone)]
pub enum DomainError {
    NotFound { id: String },
    ValidationFailed { message: String },
    Conflict { reason: String },
    Unauthorized { reason: String },
    Internal { message: String },
}

/// API-level error DTO.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BifunctorApiError {
    NotFound { resource: String, id: String },
    BadRequest { message: String, code: String },
    Conflict { message: String },
    Internal { message: String },
}

/// Task metadata for pair transformations.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskMetadata {
    pub source: String,
    #[serde(default)]
    pub attributes: HashMap<String, String>,
}

/// Processed metadata after transformation.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessedMetadata {
    pub processed_at: String,
    pub source: String,
    pub attributes: HashMap<String, String>,
    pub attribute_count: usize,
}

// =============================================================================
// DTOs
// =============================================================================

/// Options for processing tasks.
#[derive(Debug, Clone, Deserialize)]
pub struct ProcessingOptions {
    /// Whether to validate the task.
    #[serde(default)]
    pub validate: bool,
    /// Whether to simulate a failure.
    #[serde(default)]
    pub simulate_failure: bool,
    /// Failure reason if `simulate_failure` is true.
    #[serde(default)]
    pub failure_reason: Option<String>,
}

/// Request for process-with-error-transform endpoint.
#[derive(Debug, Deserialize)]
pub struct ProcessWithErrorTransformRequest {
    pub task_id: String,
    #[serde(default)]
    pub processing_options: Option<ProcessingOptions>,
}

/// Response for process-with-error-transform endpoint.
#[derive(Debug, Serialize)]
pub struct ProcessWithErrorTransformResponse {
    pub result: TransformResult,
    pub processing_time_ms: u64,
}

/// Transform result using Either for explicit success/failure.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum TransformResult {
    Success { task: TaskResponse },
    Failure { error: UserFriendlyError },
}

/// Transform options for pair endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PairTransformOption {
    Both,
    FirstOnly,
    SecondOnly,
}

/// Request for transform-pair endpoint.
#[derive(Debug, Deserialize)]
pub struct TransformPairRequest {
    pub task_id: String,
    pub metadata: TaskMetadata,
    #[serde(default = "default_transform_option")]
    pub transform_option: PairTransformOption,
}

const fn default_transform_option() -> PairTransformOption {
    PairTransformOption::Both
}

/// Response for transform-pair endpoint.
#[derive(Debug, Serialize)]
pub struct TransformPairResponse {
    pub task: TaskResponse,
    pub metadata: ProcessedMetadata,
    pub transform_applied: String,
}

/// Request context for error enrichment.
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub trace_id: Option<String>,
    pub timestamp: String,
}

/// Request for enrich-error endpoint.
#[derive(Debug, Deserialize)]
pub struct EnrichErrorRequest {
    pub task_id: String,
    #[serde(default)]
    pub include_trace: bool,
    #[serde(default)]
    pub simulate_failure: bool,
}

/// Response for enrich-error endpoint.
#[derive(Debug, Serialize)]
pub struct EnrichErrorResponse {
    pub result: EnrichResult,
    pub enrichment_applied: bool,
}

/// Enrich result.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum EnrichResult {
    Success { task: TaskResponse },
    Failure { error: EnrichedError },
}

/// Task operation type.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskOperation {
    Create,
    Update,
    Delete,
}

/// Task data for operations.
#[derive(Debug, Clone, Deserialize)]
pub struct TaskData {
    #[serde(default)]
    pub id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Request for convert-error-domain endpoint.
#[derive(Debug, Deserialize)]
pub struct ConvertErrorDomainRequest {
    pub operation: TaskOperation,
    pub data: TaskData,
    #[serde(default)]
    pub simulate_error: Option<String>,
}

/// Response for convert-error-domain endpoint.
#[derive(Debug, Serialize)]
pub struct ConvertErrorDomainResponse {
    pub result: DomainConvertResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_error_type: Option<String>,
}

/// Domain convert result.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum DomainConvertResult {
    Success { task: TaskResponse },
    Failure { error: BifunctorApiError },
}

/// Request for batch-transform-results endpoint.
#[derive(Debug, Deserialize)]
pub struct BatchTransformResultsRequest {
    pub task_ids: Vec<String>,
    #[serde(default)]
    pub fail_ids: Vec<String>,
}

/// Batch transform item result.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum BatchItemResult {
    Success { task: TaskResponse },
    Failure { error: BifunctorApiError },
}

/// Response for batch-transform-results endpoint.
#[derive(Debug, Serialize)]
pub struct BatchTransformResultsResponse {
    pub results: Vec<BatchItemResult>,
    pub success_count: usize,
    pub failure_count: usize,
}

// =============================================================================
// Pure Functions - Error Transformations
// =============================================================================

/// Pure: Converts processing error to user-friendly error.
fn to_user_friendly_error(error: &ProcessingError) -> UserFriendlyError {
    match error {
        ProcessingError::NotFound(id) => UserFriendlyError {
            code: "NOT_FOUND".to_string(),
            message: format!("The requested resource '{id}' was not found."),
            suggestion: Some("Please verify the ID and try again.".to_string()),
            details: None,
        },
        ProcessingError::ValidationFailed(msg) => UserFriendlyError {
            code: "VALIDATION_FAILED".to_string(),
            message: msg.clone(),
            suggestion: Some("Please check your input and correct any errors.".to_string()),
            details: None,
        },
        ProcessingError::Conflict(reason) => UserFriendlyError {
            code: "CONFLICT".to_string(),
            message: format!("A conflict occurred: {reason}"),
            suggestion: Some("Please refresh and try again.".to_string()),
            details: None,
        },
        ProcessingError::InternalError(_) => UserFriendlyError {
            code: "INTERNAL_ERROR".to_string(),
            message: "An unexpected error occurred.".to_string(),
            suggestion: Some("Please try again later or contact support.".to_string()),
            details: None,
        },
    }
}

/// Pure: Transforms processing result using bimap.
///
/// Demonstrates `Bifunctor::bimap` on `Result<T, E>`:
/// - first function transforms the error (E)
/// - second function transforms the success value (T)
fn transform_processing_result(
    result: Result<Task, ProcessingError>,
) -> Result<TaskResponse, UserFriendlyError> {
    result.bimap(
        |error| to_user_friendly_error(&error),
        |task| TaskResponse::from(&task),
    )
}

/// Pure: Enriches error with context using first.
///
/// Demonstrates `Bifunctor::first` on `Result<T, E>`:
/// - Only transforms the error side
/// - Success side passes through unchanged
fn enrich_error_with_context(
    result: Result<Task, ProcessingError>,
    context: &RequestContext,
) -> Result<Task, EnrichedError> {
    result.first(|error| {
        // Sanitize error message - don't expose internal details
        let (error_code, safe_message) = match &error {
            ProcessingError::NotFound(id) => ("NOT_FOUND", format!("Resource not found: {id}")),
            ProcessingError::ValidationFailed(msg) => ("VALIDATION_FAILED", msg.clone()),
            ProcessingError::Conflict(reason) => ("CONFLICT", format!("Conflict: {reason}")),
            ProcessingError::InternalError(_) => {
                ("INTERNAL_ERROR", "An internal error occurred".to_string())
            }
        };
        EnrichedError {
            original_message: safe_message,
            error_code: error_code.to_string(),
            request_id: context.request_id.clone(),
            timestamp: context.timestamp.clone(),
            trace_id: context.trace_id.clone(),
        }
    })
}

/// Pure: Converts domain error to API error using first.
///
/// Demonstrates `Bifunctor::first` for error type conversion.
fn domain_to_api_error(error: DomainError) -> BifunctorApiError {
    match error {
        DomainError::NotFound { id } => BifunctorApiError::NotFound {
            resource: "task".to_string(),
            id,
        },
        DomainError::ValidationFailed { message } => BifunctorApiError::BadRequest {
            message,
            code: "VALIDATION_ERROR".to_string(),
        },
        DomainError::Conflict { reason } => BifunctorApiError::Conflict { message: reason },
        DomainError::Unauthorized { reason } => BifunctorApiError::BadRequest {
            message: reason,
            code: "UNAUTHORIZED".to_string(),
        },
        DomainError::Internal { message: _ } => BifunctorApiError::Internal {
            message: "An internal error occurred".to_string(),
        },
    }
}

/// Pure: Converts result with domain error to result with API error.
fn convert_result_error<T>(result: Result<T, DomainError>) -> Result<T, BifunctorApiError> {
    result.first(domain_to_api_error)
}

// =============================================================================
// Pure Functions - Pair Transformations
// =============================================================================

/// Pure: Processes raw metadata into processed form.
///
/// Takes timestamp as parameter to maintain referential transparency.
fn process_metadata(metadata: &TaskMetadata, processed_at: &str) -> ProcessedMetadata {
    ProcessedMetadata {
        processed_at: processed_at.to_string(),
        source: metadata.source.clone(),
        attributes: metadata.attributes.clone(),
        attribute_count: metadata.attributes.len(),
    }
}

/// Pure: Transforms both elements of a pair using bimap.
///
/// Demonstrates `Bifunctor::bimap` on tuples.
fn transform_pair_both(
    pair: (Task, TaskMetadata),
    processed_at: &str,
) -> (TaskResponse, ProcessedMetadata) {
    let timestamp = processed_at.to_string();
    pair.bimap(
        |task| TaskResponse::from(&task),
        |meta| process_metadata(&meta, &timestamp),
    )
}

/// Pure: Transforms only the first element using first.
///
/// Demonstrates `Bifunctor::first` on tuples.
fn transform_pair_first(pair: (Task, TaskMetadata)) -> (TaskResponse, TaskMetadata) {
    pair.first(|task| TaskResponse::from(&task))
}

/// Pure: Transforms only the second element using second.
///
/// Demonstrates `Bifunctor::second` on tuples.
fn transform_pair_second(pair: (Task, ProcessedMetadata)) -> (Task, ProcessedMetadata) {
    // Already processed metadata, just pass through
    pair.second(|meta| meta)
}

// =============================================================================
// Pure Functions - Batch Transformations
// =============================================================================

/// Pure: Transforms a single result using bimap.
fn transform_single_result(
    result: Result<Task, ProcessingError>,
) -> Result<TaskResponse, BifunctorApiError> {
    result.bimap(
        |error| match error {
            ProcessingError::NotFound(id) => BifunctorApiError::NotFound {
                resource: "task".to_string(),
                id,
            },
            ProcessingError::ValidationFailed(msg) => BifunctorApiError::BadRequest {
                message: msg,
                code: "VALIDATION_FAILED".to_string(),
            },
            ProcessingError::Conflict(reason) => BifunctorApiError::Conflict { message: reason },
            ProcessingError::InternalError(_) => BifunctorApiError::Internal {
                message: "An internal error occurred".to_string(),
            },
        },
        |task| TaskResponse::from(&task),
    )
}

/// Pure: Transforms all results in a batch using bimap.
fn transform_all_results(
    results: Vec<Result<Task, ProcessingError>>,
) -> Vec<Result<TaskResponse, BifunctorApiError>> {
    results.into_iter().map(transform_single_result).collect()
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

/// Generates a unique request ID.
fn generate_request_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Simulates task processing that may fail.
fn simulate_processing(task: Task, options: &ProcessingOptions) -> Result<Task, ProcessingError> {
    if options.simulate_failure {
        let reason = options
            .failure_reason
            .clone()
            .unwrap_or_else(|| "Simulated failure".to_string());
        return Err(ProcessingError::ValidationFailed(reason));
    }

    if options.validate && task.title.trim().is_empty() {
        return Err(ProcessingError::ValidationFailed(
            "Task title cannot be empty".to_string(),
        ));
    }

    // Add a "processed" tag to indicate successful processing
    Ok(task.add_tag(Tag::new("processed")))
}

/// Simulates a domain operation.
///
/// Takes timestamp as parameter to maintain referential transparency.
fn simulate_domain_operation(
    operation: &TaskOperation,
    data: &TaskData,
    simulate_error: Option<&String>,
    timestamp: Timestamp,
) -> Result<Task, DomainError> {
    if let Some(error_type) = simulate_error {
        return match error_type.as_str() {
            "not_found" => Err(DomainError::NotFound {
                id: data.id.clone().unwrap_or_else(|| "unknown".to_string()),
            }),
            "validation" => Err(DomainError::ValidationFailed {
                message: "Validation failed".to_string(),
            }),
            "conflict" => Err(DomainError::Conflict {
                reason: "Resource conflict".to_string(),
            }),
            "unauthorized" => Err(DomainError::Unauthorized {
                reason: "Not authorized".to_string(),
            }),
            _ => Err(DomainError::Internal {
                message: "Internal error".to_string(),
            }),
        };
    }

    match operation {
        TaskOperation::Create | TaskOperation::Update => {
            let task_id = data
                .id
                .as_ref()
                .and_then(|id| parse_task_id(id).ok())
                .unwrap_or_else(TaskId::generate);
            let mut task = Task::new(task_id, data.title.clone(), timestamp);
            if let Some(desc) = &data.description {
                task = task.with_description(desc.clone());
            }
            Ok(task)
        }
        TaskOperation::Delete => {
            let id = data.id.clone().unwrap_or_else(|| "unknown".to_string());
            Err(DomainError::NotFound { id })
        }
    }
}

// =============================================================================
// POST /tasks/process-with-error-transform - Error transformation using bimap
// =============================================================================

/// Processes a task and transforms both success and error using bimap.
///
/// This handler demonstrates:
/// - **`Bifunctor::bimap`**: Simultaneously transform success and error types
/// - Error transformation to user-friendly format
///
/// # Request Body
///
/// - `task_id`: ID of the task to process
/// - `processing_options`: Options for processing
///
/// # Errors
///
/// - `404 Not Found`: Task not found
/// - `422 Unprocessable Entity`: Processing failed
#[allow(clippy::cast_possible_truncation)]
pub async fn process_with_error_transform(
    State(state): State<AppState>,
    Json(request): Json<ProcessWithErrorTransformRequest>,
) -> Result<Json<ProcessWithErrorTransformResponse>, ApiErrorResponse> {
    let start = Instant::now();

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
        .ok_or_else(|| ApiErrorResponse::not_found(format!("Task not found: {task_id}")))?;

    // Process task (may fail)
    let options = request.processing_options.unwrap_or(ProcessingOptions {
        validate: false,
        simulate_failure: false,
        failure_reason: None,
    });
    let processing_result = simulate_processing(task, &options);

    // Use bimap to transform both success and error
    let transformed = transform_processing_result(processing_result);

    let result = match transformed {
        Ok(task_response) => TransformResult::Success {
            task: task_response,
        },
        Err(user_error) => TransformResult::Failure { error: user_error },
    };

    let processing_time_ms = start.elapsed().as_millis() as u64;

    Ok(Json(ProcessWithErrorTransformResponse {
        result,
        processing_time_ms,
    }))
}

// =============================================================================
// POST /tasks/transform-pair - Tuple transformation using bimap/first/second
// =============================================================================

/// Transforms a task-metadata pair using Bifunctor operations.
///
/// This handler demonstrates:
/// - **`bimap`**: Transform both elements simultaneously
/// - **`first`**: Transform only the first element (task)
/// - **`second`**: Transform only the second element (metadata)
///
/// # Request Body
///
/// - `task_id`: ID of the task
/// - `metadata`: Metadata to transform
/// - `transform_option`: Which transformation to apply (both, `first_only`, `second_only`)
///
/// # Errors
///
/// - `404 Not Found`: Task not found
#[allow(clippy::cast_possible_truncation)]
pub async fn transform_pair(
    State(state): State<AppState>,
    Json(request): Json<TransformPairRequest>,
) -> Result<Json<TransformPairResponse>, ApiErrorResponse> {
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
        .ok_or_else(|| ApiErrorResponse::not_found(format!("Task not found: {task_id}")))?;

    // Get current timestamp for metadata processing (effectful, done at boundary)
    let now = chrono::Utc::now().to_rfc3339();

    // Create pair and apply transformation based on option
    let (task_response, metadata_response, transform_applied) = match request.transform_option {
        PairTransformOption::Both => {
            let pair = (task, request.metadata);
            let (task_dto, processed_meta) = transform_pair_both(pair, &now);
            (task_dto, processed_meta, "both")
        }
        PairTransformOption::FirstOnly => {
            let pair = (task, request.metadata.clone());
            let (task_dto, raw_meta) = transform_pair_first(pair);
            let processed_meta = process_metadata(&raw_meta, &now);
            (task_dto, processed_meta, "first_only")
        }
        PairTransformOption::SecondOnly => {
            let processed_meta = process_metadata(&request.metadata, &now);
            let pair = (task, processed_meta);
            let (raw_task, meta) = transform_pair_second(pair);
            (TaskResponse::from(&raw_task), meta, "second_only")
        }
    };

    Ok(Json(TransformPairResponse {
        task: task_response,
        metadata: metadata_response,
        transform_applied: transform_applied.to_string(),
    }))
}

// =============================================================================
// POST /tasks/enrich-error - Error enrichment using first
// =============================================================================

/// Enriches processing errors with context information.
///
/// This handler demonstrates:
/// - **`Bifunctor::first`**: Transform only the error side
/// - Context enrichment (request ID, timestamp, trace ID)
///
/// # Request Body
///
/// - `task_id`: ID of the task to process
/// - `include_trace`: Whether to include trace ID
/// - `simulate_failure`: Whether to simulate a failure
///
/// # Errors
///
/// - `404 Not Found`: Task not found
#[allow(clippy::cast_possible_truncation)]
pub async fn enrich_error(
    State(state): State<AppState>,
    Json(request): Json<EnrichErrorRequest>,
) -> Result<Json<EnrichErrorResponse>, ApiErrorResponse> {
    // Parse task ID
    let task_id = parse_task_id(&request.task_id).map_err(|error| {
        ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("task_id", error)],
        )
    })?;

    // Create request context
    let context = RequestContext {
        request_id: generate_request_id(),
        trace_id: if request.include_trace {
            Some(uuid::Uuid::new_v4().to_string())
        } else {
            None
        },
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    // Fetch task
    let task = state
        .task_repository
        .find_by_id(&task_id)
        .run_async()
        .await
        .map_err(|_| ApiErrorResponse::internal_error("Repository error"))?
        .ok_or_else(|| ApiErrorResponse::not_found(format!("Task not found: {task_id}")))?;

    // Process (may fail if simulate_failure)
    let processing_result = if request.simulate_failure {
        Err(ProcessingError::ValidationFailed(
            "Simulated failure for enrichment demo".to_string(),
        ))
    } else {
        Ok(task)
    };

    // Use first to enrich errors with context
    let enriched_result = enrich_error_with_context(processing_result, &context);

    let (result, enrichment_applied) = match enriched_result {
        Ok(task) => (
            EnrichResult::Success {
                task: TaskResponse::from(&task),
            },
            false,
        ),
        Err(enriched_error) => (
            EnrichResult::Failure {
                error: enriched_error,
            },
            true,
        ),
    };

    Ok(Json(EnrichErrorResponse {
        result,
        enrichment_applied,
    }))
}

// =============================================================================
// POST /tasks/convert-error-domain - Domain error conversion using first
// =============================================================================

/// Converts domain errors to API errors.
///
/// This handler demonstrates:
/// - **`Bifunctor::first`**: Convert error types between layers
/// - Domain → API error mapping
///
/// # Request Body
///
/// - `operation`: Type of operation (create, update, delete)
/// - `data`: Task data
/// - `simulate_error`: Optional error type to simulate
///
/// # Errors
///
/// - Appropriate error based on simulated error type
#[allow(clippy::cast_possible_truncation)]
pub async fn convert_error_domain(
    State(_state): State<AppState>,
    Json(request): Json<ConvertErrorDomainRequest>,
) -> Result<Json<ConvertErrorDomainResponse>, ApiErrorResponse> {
    // Get timestamp at system boundary
    let timestamp = Timestamp::now();

    // Perform domain operation
    let domain_result = simulate_domain_operation(
        &request.operation,
        &request.data,
        request.simulate_error.as_ref(),
        timestamp,
    );

    // Extract original error type for debugging
    let original_error_type = domain_result
        .as_ref()
        .err()
        .map(|e| match e {
            DomainError::NotFound { .. } => "NotFound",
            DomainError::ValidationFailed { .. } => "ValidationFailed",
            DomainError::Conflict { .. } => "Conflict",
            DomainError::Unauthorized { .. } => "Unauthorized",
            DomainError::Internal { .. } => "Internal",
        })
        .map(String::from);

    // Use first to convert domain error to API error
    let api_result = convert_result_error(domain_result);

    let result = match api_result {
        Ok(task) => DomainConvertResult::Success {
            task: TaskResponse::from(&task),
        },
        Err(api_error) => DomainConvertResult::Failure { error: api_error },
    };

    Ok(Json(ConvertErrorDomainResponse {
        result,
        original_error_type,
    }))
}

// =============================================================================
// POST /tasks/batch-transform-results - Batch transformation using bimap
// =============================================================================

/// Transforms multiple results using bimap.
///
/// This handler demonstrates:
/// - **`bimap` + iteration**: Apply bimap to each result in a batch
/// - Batch error transformation
///
/// # Request Body
///
/// - `task_ids`: List of task IDs to process (1-50)
/// - `fail_ids`: IDs to simulate failure for
///
/// # Errors
///
/// - `400 Bad Request`: Invalid request
#[allow(clippy::cast_possible_truncation)]
pub async fn batch_transform_results(
    State(state): State<AppState>,
    Json(request): Json<BatchTransformResultsRequest>,
) -> Result<Json<BatchTransformResultsResponse>, ApiErrorResponse> {
    // Validate request
    if request.task_ids.is_empty() {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("task_ids", "task_ids cannot be empty")],
        ));
    }

    if request.task_ids.len() > 50 {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new(
                "task_ids",
                "task_ids cannot exceed 50 items",
            )],
        ));
    }

    // Build results for each task
    let mut processing_results: Vec<Result<Task, ProcessingError>> = Vec::new();

    for task_id_str in &request.task_ids {
        let result = match parse_task_id(task_id_str) {
            Ok(task_id) => {
                // Check if this ID should fail
                if request.fail_ids.contains(task_id_str) {
                    Err(ProcessingError::ValidationFailed(format!(
                        "Simulated failure for {task_id_str}"
                    )))
                } else {
                    // Fetch task
                    match state.task_repository.find_by_id(&task_id).run_async().await {
                        Ok(Some(task)) => Ok(task),
                        Ok(None) => Err(ProcessingError::NotFound(task_id_str.clone())),
                        Err(_) => Err(ProcessingError::InternalError(
                            "Repository error".to_string(),
                        )),
                    }
                }
            }
            Err(_) => Err(ProcessingError::ValidationFailed(format!(
                "Invalid task ID: {task_id_str}"
            ))),
        };
        processing_results.push(result);
    }

    // Use bimap to transform all results
    let transformed_results = transform_all_results(processing_results);

    // Convert to response format
    let results: Vec<BatchItemResult> = transformed_results
        .into_iter()
        .map(|r| match r {
            Ok(task) => BatchItemResult::Success { task },
            Err(error) => BatchItemResult::Failure { error },
        })
        .collect();

    let success_count = results
        .iter()
        .filter(|r| matches!(r, BatchItemResult::Success { .. }))
        .count();
    let failure_count = results.len() - success_count;

    Ok(Json(BatchTransformResultsResponse {
        results,
        success_count,
        failure_count,
    }))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lambars::control::Either;
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // Bifunctor Law Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_bimap_identity_law_result() {
        let result: Result<i32, String> = Ok(42);
        let mapped = result.clone().bimap(|e| e, |v| v);
        assert_eq!(result, mapped);

        let err_result: Result<i32, String> = Err("error".to_string());
        let err_mapped = err_result.clone().bimap(|e| e, |v| v);
        assert_eq!(err_result, err_mapped);
    }

    #[rstest]
    fn test_bimap_identity_law_either() {
        let left: Either<i32, String> = Either::Left(42);
        let left_mapped = left.clone().bimap(|l| l, |r: String| r);
        assert_eq!(left, left_mapped);

        let right: Either<i32, String> = Either::Right("hello".to_string());
        let right_mapped = right.clone().bimap(|l: i32| l, |r| r);
        assert_eq!(right, right_mapped);
    }

    #[rstest]
    fn test_bimap_identity_law_tuple() {
        let tuple = (42, "hello".to_string());
        let mapped = tuple.clone().bimap(|a| a, |b| b);
        assert_eq!(tuple, mapped);
    }

    #[rstest]
    fn test_first_second_consistency() {
        // bimap(f, g) == first(f).second(g)
        let result: Result<i32, String> = Ok(42);

        let by_bimap = result.clone().bimap(|e| e.len(), |v| v * 2);
        let by_first_second = result.first(|e: String| e.len()).second(|v| v * 2);

        assert_eq!(by_bimap, by_first_second);
    }

    // -------------------------------------------------------------------------
    // Result bimap Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_result_bimap_ok() {
        let result: Result<i32, String> = Ok(42);
        let mapped = result.bimap(|e| e.len(), |v| v * 2);
        assert_eq!(mapped, Ok(84));
    }

    #[rstest]
    fn test_result_bimap_err() {
        let result: Result<i32, String> = Err("error".to_string());
        let mapped = result.bimap(|e| e.len(), |v: i32| v * 2);
        assert_eq!(mapped, Err(5));
    }

    #[rstest]
    fn test_result_first_transforms_error() {
        let result: Result<i32, String> = Err("hello".to_string());
        let mapped = result.first(|e| e.len());
        assert_eq!(mapped, Err(5));
    }

    #[rstest]
    fn test_result_second_transforms_ok() {
        let result: Result<i32, String> = Ok(42);
        let mapped = result.second(|v| v * 2);
        assert_eq!(mapped, Ok(84));
    }

    // -------------------------------------------------------------------------
    // Either bimap Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_either_bimap_left() {
        let either: Either<i32, String> = Either::Left(42);
        let mapped = either.bimap(|l| l * 2, |r: String| r.len());
        assert_eq!(mapped, Either::Left(84));
    }

    #[rstest]
    fn test_either_bimap_right() {
        let either: Either<i32, String> = Either::Right("hello".to_string());
        let mapped = either.bimap(|l: i32| l * 2, |r| r.len());
        assert_eq!(mapped, Either::Right(5));
    }

    // -------------------------------------------------------------------------
    // Tuple bimap Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_tuple_bimap() {
        let tuple = (42, "hello".to_string());
        let mapped = tuple.bimap(|a| a * 2, |b| b.len());
        assert_eq!(mapped, (84, 5));
    }

    #[rstest]
    fn test_tuple_first() {
        let tuple = (42, "hello".to_string());
        let mapped = tuple.first(|a| a * 2);
        assert_eq!(mapped, (84, "hello".to_string()));
    }

    #[rstest]
    fn test_tuple_second() {
        let tuple = (42, "hello".to_string());
        let mapped = tuple.second(|b| b.len());
        assert_eq!(mapped, (42, 5));
    }

    // -------------------------------------------------------------------------
    // Pure Function Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_to_user_friendly_error_not_found() {
        let error = ProcessingError::NotFound("task-123".to_string());
        let friendly = to_user_friendly_error(&error);
        assert_eq!(friendly.code, "NOT_FOUND");
        assert!(friendly.message.contains("task-123"));
    }

    #[rstest]
    fn test_to_user_friendly_error_validation() {
        let error = ProcessingError::ValidationFailed("Title is required".to_string());
        let friendly = to_user_friendly_error(&error);
        assert_eq!(friendly.code, "VALIDATION_FAILED");
        assert_eq!(friendly.message, "Title is required");
    }

    #[rstest]
    fn test_to_user_friendly_error_internal() {
        let error = ProcessingError::InternalError("DB connection failed".to_string());
        let friendly = to_user_friendly_error(&error);
        assert_eq!(friendly.code, "INTERNAL_ERROR");
        // Internal details should not be exposed
        assert!(!friendly.message.contains("DB"));
    }

    #[rstest]
    fn test_transform_processing_result_success() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());
        let result: Result<Task, ProcessingError> = Ok(task);
        let transformed = transform_processing_result(result);
        assert!(transformed.is_ok());
    }

    #[rstest]
    fn test_transform_processing_result_failure() {
        let result: Result<Task, ProcessingError> =
            Err(ProcessingError::NotFound("task-123".to_string()));
        let transformed = transform_processing_result(result);
        assert!(transformed.is_err());
        let error = transformed.unwrap_err();
        assert_eq!(error.code, "NOT_FOUND");
    }

    #[rstest]
    fn test_domain_to_api_error_not_found() {
        let error = DomainError::NotFound {
            id: "task-123".to_string(),
        };
        let api_error = domain_to_api_error(error);
        assert!(matches!(api_error, BifunctorApiError::NotFound { .. }));
    }

    #[rstest]
    fn test_domain_to_api_error_validation() {
        let error = DomainError::ValidationFailed {
            message: "Invalid input".to_string(),
        };
        let api_error = domain_to_api_error(error);
        assert!(matches!(api_error, BifunctorApiError::BadRequest { .. }));
    }

    #[rstest]
    fn test_domain_to_api_error_internal_hides_details() {
        let error = DomainError::Internal {
            message: "Database connection failed".to_string(),
        };
        let api_error = domain_to_api_error(error);
        if let BifunctorApiError::Internal { message } = api_error {
            assert!(!message.contains("Database"));
        } else {
            panic!("Expected Internal error");
        }
    }

    #[rstest]
    fn test_process_metadata() {
        let metadata = TaskMetadata {
            source: "api".to_string(),
            attributes: HashMap::from([("key".to_string(), "value".to_string())]),
        };
        let processed = process_metadata(&metadata, "2024-01-01T00:00:00Z");
        assert_eq!(processed.source, "api");
        assert_eq!(processed.attribute_count, 1);
        assert_eq!(processed.processed_at, "2024-01-01T00:00:00Z");
    }

    #[rstest]
    fn test_transform_pair_both() {
        let task = Task::new(TaskId::generate(), "Test".to_string(), Timestamp::now());
        let metadata = TaskMetadata {
            source: "test".to_string(),
            attributes: HashMap::new(),
        };
        let (task_dto, processed_meta) =
            transform_pair_both((task, metadata), "2024-01-01T12:00:00Z");
        assert_eq!(task_dto.title, "Test");
        assert_eq!(processed_meta.source, "test");
        assert_eq!(processed_meta.processed_at, "2024-01-01T12:00:00Z");
    }

    #[rstest]
    fn test_transform_all_results() {
        let task1 = Task::new(TaskId::generate(), "Task 1".to_string(), Timestamp::now());
        let task2 = Task::new(TaskId::generate(), "Task 2".to_string(), Timestamp::now());

        let results = vec![
            Ok(task1),
            Err(ProcessingError::NotFound("missing".to_string())),
            Ok(task2),
        ];

        let transformed = transform_all_results(results);
        assert_eq!(transformed.len(), 3);
        assert!(transformed[0].is_ok());
        assert!(transformed[1].is_err());
        assert!(transformed[2].is_ok());
    }

    // -------------------------------------------------------------------------
    // bimap_ref Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_bimap_ref_preserves_original() {
        let tuple = (42, "hello".to_string());
        let mapped = tuple.bimap_ref(|a| a * 2, String::len);

        // Original is still available
        assert_eq!(tuple.0, 42);
        assert_eq!(tuple.1, "hello");

        // Mapped values are correct
        assert_eq!(mapped, (84, 5));
    }

    #[rstest]
    fn test_first_ref_preserves_original() {
        let either: Either<String, i32> = Either::Left("hello".to_string());
        let mapped = either.first_ref(String::len);

        // Original is still available
        assert!(either.is_left());

        // Mapped value is correct
        assert_eq!(mapped, Either::Left(5));
    }

    #[rstest]
    fn test_second_ref_preserves_original() {
        let either: Either<String, i32> = Either::Right(42);
        let mapped = either.second_ref(|n| n * 2);

        // Original is still available
        assert!(either.is_right());

        // Mapped value is correct
        assert_eq!(mapped, Either::Right(84));
    }

    // -------------------------------------------------------------------------
    // Bifunctor Composition Law Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_bifunctor_composition_law_tuple() {
        // Composition law: bimap (g . f) (i . h) = (bimap g i) . (bimap f h)
        // Standard form: bimap (f . g) (h . i) = (bimap f h) . (bimap g i)
        // These are equivalent by variable renaming.

        let tuple = (10, 20);

        // Functions for first element
        let f = |x: i32| x * 2; // 10 -> 20
        let g = |x: i32| x + 5; // 20 -> 25

        // Functions for second element
        let h = |x: i32| x.to_string(); // 20 -> "20"
        let i = |s: String| s.len(); // "20" -> 2

        // LHS: bimap (g . f) (i . h) where (g . f)(x) = g(f(x))
        let lhs = tuple.bimap(|x| g(f(x)), |x| i(h(x)));

        // RHS: (bimap g i) . (bimap f h) = bimap g i (bimap f h tuple)
        let rhs = tuple.bimap(f, h).bimap(g, i);

        assert_eq!(lhs, rhs);
        assert_eq!(lhs, (25, 2));
    }

    #[rstest]
    fn test_bifunctor_composition_law_result() {
        // Composition law for Result<T, E> as Bifunctor<E, T>
        // bimap (g . f) (i . h) = (bimap g i) . (bimap f h)
        // Remember: first transforms E, second transforms T

        let ok_result: Result<i32, String> = Ok(10);
        let err_result: Result<i32, String> = Err("error".to_string());

        // Functions for error (first): f_err then g_err
        let f_err = |s: String| s.to_uppercase();
        let g_err = |s: String| format!("[{s}]");

        // Functions for success (second): f_ok then g_ok
        let f_ok = |x: i32| x * 2;
        let g_ok = |x: i32| x + 100;

        // Test Ok case
        // LHS: bimap (g_err . f_err) (g_ok . f_ok)
        let lhs_ok = ok_result
            .clone()
            .bimap(|e| g_err(f_err(e)), |t| g_ok(f_ok(t)));
        // RHS: bimap g_err g_ok . bimap f_err f_ok
        let rhs_ok = ok_result.bimap(f_err, f_ok).bimap(g_err, g_ok);
        assert_eq!(lhs_ok, rhs_ok);
        assert_eq!(lhs_ok, Ok(120)); // 10 * 2 + 100 = 120

        // Test Err case
        let lhs_err = err_result
            .clone()
            .bimap(|e| g_err(f_err(e)), |t| g_ok(f_ok(t)));
        let rhs_err = err_result.bimap(f_err, f_ok).bimap(g_err, g_ok);
        assert_eq!(lhs_err, rhs_err);
        assert_eq!(lhs_err, Err("[ERROR]".to_string()));
    }

    #[rstest]
    fn test_bifunctor_composition_law_either() {
        // Composition law for Either<L, R> as Bifunctor<L, R>
        // bimap (g . f) (i . h) = (bimap g i) . (bimap f h)

        let left: Either<i32, String> = Either::Left(5);
        let right: Either<i32, String> = Either::Right("hello".to_string());

        // Functions for Left: f_left then g_left
        let f_left = |x: i32| x * 3;
        let g_left = |x: i32| x - 1;

        // Functions for Right: f_right then g_right
        let f_right = |s: String| s.len();
        let g_right = |n: usize| n * 10;

        // Test Left case
        // LHS: bimap (g_left . f_left) (g_right . f_right)
        let lhs_left = left
            .clone()
            .bimap(|l| g_left(f_left(l)), |r| g_right(f_right(r)));
        // RHS: bimap g_left g_right . bimap f_left f_right
        let rhs_left = left.bimap(f_left, f_right).bimap(g_left, g_right);
        assert_eq!(lhs_left, rhs_left);
        assert_eq!(lhs_left, Either::Left(14)); // 5 * 3 - 1 = 14

        // Test Right case
        let lhs_right = right
            .clone()
            .bimap(|l| g_left(f_left(l)), |r| g_right(f_right(r)));
        let rhs_right = right.bimap(f_left, f_right).bimap(g_left, g_right);
        assert_eq!(lhs_right, rhs_right);
        assert_eq!(lhs_right, Either::Right(50)); // "hello".len() = 5, 5 * 10 = 50
    }
}
