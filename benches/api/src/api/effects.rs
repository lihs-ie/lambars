//! Effects and Optics handlers demonstrating lambars advanced features.
//!
//! This module showcases:
//! - `define_effect!` macro for creating custom algebraic effects
//! - `Eff` (Freer monad) for effectful computations
//! - `lens!`, `prism!`, `iso!` macros for optics
//! - `State`, `Writer`, `RWS` monads for stateful computations
//!
//! # lambars Features Demonstrated
//!
//! - **Algebraic Effects**: Custom effect definitions with handlers
//! - **Optics**: Type-safe, composable access to nested data structures
//! - **Monad Transformers**: State, Writer, and RWS for complex workflows

use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use lambars::define_effect;
use lambars::eff;
use lambars::effect::algebraic::{Eff, Handler, NoEffect, PureHandler};
use lambars::effect::{RWS, State as StateMonad, Writer};
use lambars::optics::{Iso, Lens, Prism};
use lambars::{iso, lens, prism};

use super::error::{ApiErrorResponse, FieldError};
use super::handlers::AppState;
use crate::domain::{Priority, Task, TaskId, TaskStatus};

// =============================================================================
// Custom Effect Definitions using define_effect! macro
// =============================================================================

define_effect! {
    /// Task validation effect for checking task constraints.
    effect TaskValidation {
        /// Validates that a title meets requirements.
        fn validate_title(title: String) -> bool;
        /// Validates that a priority is appropriate.
        fn validate_priority(priority: String) -> bool;
        /// Logs a validation message.
        fn log_validation(message: String) -> ();
    }
}

define_effect! {
    /// Task workflow effect for business operations.
    effect TaskWorkflow {
        /// Gets the current task state.
        fn get_task_state() -> String;
        /// Updates the task state.
        fn set_task_state(state: String) -> ();
        /// Records an audit entry.
        fn audit(action: String) -> ();
    }
}

// =============================================================================
// Request/Response DTOs
// =============================================================================

/// Request body for workflow execution.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowRequest {
    /// The title to validate.
    pub title: String,
    /// The priority to validate.
    pub priority: String,
    /// Whether to enable detailed logging.
    #[serde(default)]
    pub enable_logging: bool,
}

/// Response from workflow execution.
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowResponse {
    /// Whether the validation passed.
    pub valid: bool,
    /// Validation messages.
    pub messages: Vec<String>,
    /// Effect execution trace.
    pub trace: Vec<String>,
}

/// Request body for optics operations.
#[derive(Debug, Clone, Deserialize)]
pub struct OpticsRequest {
    /// Field to update using lens.
    pub field: String,
    /// New value for the field.
    pub value: serde_json::Value,
    /// Optional transformation to apply.
    pub transform: Option<String>,
}

/// Response from optics operations.
#[derive(Debug, Clone, Serialize)]
pub struct OpticsResponse {
    /// The task ID.
    pub task_id: String,
    /// Updated field name.
    pub updated_field: String,
    /// Previous value.
    pub previous_value: serde_json::Value,
    /// New value after update.
    pub new_value: serde_json::Value,
    /// Optics used for the operation.
    pub optics_used: String,
}

/// Request body for state workflow.
#[derive(Debug, Clone, Deserialize)]
pub struct StateWorkflowRequest {
    /// Initial counter value.
    pub initial_count: i32,
    /// Operations to perform.
    pub operations: Vec<StateOperation>,
    /// Whether to use RWS monad.
    #[serde(default)]
    pub use_rws: bool,
}

/// A state operation.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StateOperation {
    /// Increment the counter.
    Increment,
    /// Decrement the counter.
    Decrement,
    /// Multiply by a value.
    Multiply { factor: i32 },
    /// Add a value.
    Add { value: i32 },
    /// Log the current state.
    Log { message: String },
}

/// Response from state workflow.
#[derive(Debug, Clone, Serialize)]
pub struct StateWorkflowResponse {
    /// Final counter value.
    pub final_count: i32,
    /// Operation log.
    pub logs: Vec<String>,
    /// Monad type used.
    pub monad_type: String,
    /// Intermediate states.
    pub states: Vec<i32>,
}

// =============================================================================
// POST /tasks/workflow - Algebraic Effects with define_effect! and Eff
// =============================================================================

/// Executes a workflow using algebraic effects.
///
/// This handler demonstrates:
/// - **`define_effect!`**: Custom effect definition (see module-level definitions)
/// - **`Eff`** (Freer monad): Effectful computation representation
/// - **`PureHandler`**: Running pure computations
///
/// # Note on Effect Usage
///
/// The `TaskValidationEffect` and `TaskWorkflowEffect` defined at module level
/// demonstrate the `define_effect!` macro syntax. In this handler, we use
/// `Eff<NoEffect, bool>` with `PureHandler` for simplicity, as the validation
/// logic is pure. In a production system, you would:
/// 1. Define a custom handler implementing `TaskValidationHandler`
/// 2. Use `Eff<TaskValidationEffect, bool>` for the computation
/// 3. Run with your custom handler to interpret effects
///
/// # Request Body
///
/// ```json
/// {
///   "title": "My Task",
///   "priority": "high",
///   "enable_logging": true
/// }
/// ```
///
/// # Response
///
/// - **200 OK**: Workflow executed successfully
///
/// # Errors
///
/// This handler always returns `Ok` as validation is performed synchronously.
#[allow(clippy::unused_async)]
pub async fn execute_workflow(
    State(_state): State<AppState>,
    Json(request): Json<WorkflowRequest>,
) -> Result<Json<WorkflowResponse>, ApiErrorResponse> {
    // Build the effectful computation using Eff monad
    // Note: Eff uses Rc internally and is not Send, so we process synchronously
    let response = {
        // Create validation workflow using Eff monad with NoEffect
        // This demonstrates Eff's monadic operations (pure, flat_map, fmap)
        let computation = create_validation_workflow(&request);

        // Run with pure handler (demonstrates Handler trait usage)
        let validation_result = PureHandler.run(computation);

        // Build trace showing the workflow steps
        let mut trace = vec![
            "Created Eff<NoEffect, bool> computation".to_string(),
            "Used Eff::pure and flat_map for monadic composition".to_string(),
            format!("Validated title: '{}'", request.title),
            format!("Validated priority: '{}'", request.priority),
        ];

        if request.enable_logging {
            trace.push("Logging enabled - validation messages recorded".to_string());
        }

        let messages = if validation_result {
            vec![
                format!("Title '{}' meets requirements", request.title),
                format!("Priority '{}' is valid", request.priority),
            ]
        } else {
            let mut msgs = Vec::new();
            if request.title.trim().is_empty() {
                msgs.push("Title cannot be empty".to_string());
            }
            if request.title.len() > 200 {
                msgs.push("Title exceeds maximum length".to_string());
            }
            if !["low", "medium", "high", "critical"].contains(&request.priority.as_str()) {
                msgs.push(format!("Invalid priority: '{}'", request.priority));
            }
            msgs
        };

        WorkflowResponse {
            valid: validation_result,
            messages,
            trace,
        }
    };

    Ok(Json(response))
}

/// Creates a validation workflow as an Eff computation.
///
/// This demonstrates building effectful computations that can be
/// interpreted by different handlers.
fn create_validation_workflow(request: &WorkflowRequest) -> Eff<NoEffect, bool> {
    // For demonstration, we use NoEffect and pure computation
    // In a full implementation, we would use TaskValidationEffect
    // and provide a custom handler

    let title_valid = !request.title.trim().is_empty() && request.title.len() <= 200;
    let priority_valid =
        ["low", "medium", "high", "critical"].contains(&request.priority.to_lowercase().as_str());

    // Build computation using eff! macro for do-notation style
    eff! {
        t_valid <= Eff::pure(title_valid);
        p_valid <= Eff::pure(priority_valid);
        Eff::pure(t_valid && p_valid)
    }
}

// =============================================================================
// PUT /tasks/{id}/optics - Lens, Prism, Iso operations
// =============================================================================

/// Updates a task field using optics.
///
/// This handler demonstrates:
/// - **`lens!`**: Type-safe field access and modification
/// - **`prism!`**: Safe enum variant access
/// - **`iso!`**: Bidirectional type conversions
///
/// # Path Parameters
///
/// - `id`: Task ID
///
/// # Request Body
///
/// ```json
/// {
///   "field": "title",
///   "value": "Updated Title",
///   "transform": "uppercase"
/// }
/// ```
///
/// # Response
///
/// - **200 OK**: Field updated successfully
/// - **404 Not Found**: Task not found
/// - **400 Bad Request**: Invalid field or value
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] in the following cases:
/// - **400 Bad Request**: Invalid task ID, field, or value
/// - **404 Not Found**: Task not found
/// - **500 Internal Server Error**: Repository error
#[allow(clippy::unused_async)]
pub async fn update_with_optics(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<OpticsRequest>,
) -> Result<Json<OpticsResponse>, ApiErrorResponse> {
    let task_id = Uuid::parse_str(&id)
        .map(TaskId::from_uuid)
        .map_err(|_| ApiErrorResponse::bad_request("INVALID_TASK_ID", "Invalid task ID format"))?;

    let task = state
        .task_repository
        .find_by_id(&task_id)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?
        .ok_or_else(|| ApiErrorResponse::not_found(format!("Task {id} not found")))?;

    // Apply optics operation synchronously (Task is not Send)
    let (updated_task, previous_value, new_value, optics_used) =
        apply_optics_update(task, &request)?;

    // Save the updated task
    state
        .task_repository
        .save(&updated_task)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    Ok(Json(OpticsResponse {
        task_id: id,
        updated_field: request.field,
        previous_value,
        new_value,
        optics_used,
    }))
}

/// Applies an optics-based update to a task.
///
/// Demonstrates lens!, prism!, and iso! macro usage for type-safe updates.
#[allow(clippy::too_many_lines)]
fn apply_optics_update(
    task: Task,
    request: &OpticsRequest,
) -> Result<(Task, serde_json::Value, serde_json::Value, String), ApiErrorResponse> {
    match request.field.as_str() {
        "title" => {
            let new_title = request
                .value
                .as_str()
                .ok_or_else(|| {
                    ApiErrorResponse::validation_error(
                        "Validation failed",
                        vec![FieldError::new("value", "Title must be a string")],
                    )
                })?
                .to_string();

            // Apply optional transformation
            let transformed_title = match request.transform.as_deref() {
                Some("uppercase") => new_title.to_uppercase(),
                Some("lowercase") => new_title.to_lowercase(),
                Some("trim") => new_title.trim().to_string(),
                _ => new_title,
            };

            let previous = serde_json::json!(task.title);
            let new = serde_json::json!(&transformed_title);

            // Use lens for title update
            // lens!(Task, title) creates a lens focusing on the title field
            let title_lens = lens!(Task, title);
            let updated = title_lens.set(task, transformed_title);

            Ok((updated, previous, new, "lens!(Task, title)".to_string()))
        }

        "priority" => {
            let priority_str = request
                .value
                .as_str()
                .ok_or_else(|| {
                    ApiErrorResponse::validation_error(
                        "Validation failed",
                        vec![FieldError::new("value", "Priority must be a string")],
                    )
                })?
                .to_lowercase();

            let new_priority = match priority_str.as_str() {
                "low" => Priority::Low,
                "medium" => Priority::Medium,
                "high" => Priority::High,
                "critical" => Priority::Critical,
                _ => {
                    return Err(ApiErrorResponse::validation_error(
                        "Validation failed",
                        vec![FieldError::new(
                            "value",
                            format!("Invalid priority: {priority_str}"),
                        )],
                    ));
                }
            };

            let previous = serde_json::json!(format!("{:?}", task.priority).to_lowercase());
            let new = serde_json::json!(priority_str);

            // Use lens for priority update
            let priority_lens = lens!(Task, priority);
            let updated = priority_lens.set(task, new_priority);

            Ok((updated, previous, new, "lens!(Task, priority)".to_string()))
        }

        "description" => {
            let new_desc = if request.value.is_null() {
                None
            } else {
                Some(
                    request
                        .value
                        .as_str()
                        .ok_or_else(|| {
                            ApiErrorResponse::validation_error(
                                "Validation failed",
                                vec![FieldError::new(
                                    "value",
                                    "Description must be a string or null",
                                )],
                            )
                        })?
                        .to_string(),
                )
            };

            let previous = serde_json::json!(task.description);
            let new = serde_json::json!(&new_desc);

            // Use lens for description update
            let description_lens = lens!(Task, description);
            let updated = description_lens.set(task, new_desc);

            Ok((
                updated,
                previous,
                new,
                "lens!(Task, description)".to_string(),
            ))
        }

        "status" => {
            // Demonstrate iso! for TaskStatus <-> String conversion
            let status_str = request
                .value
                .as_str()
                .ok_or_else(|| {
                    ApiErrorResponse::validation_error(
                        "Validation failed",
                        vec![FieldError::new("value", "Status must be a string")],
                    )
                })?
                .to_lowercase();

            // Validate status before creating iso
            let new_status = match status_str.as_str() {
                "pending" => TaskStatus::Pending,
                "in_progress" => TaskStatus::InProgress,
                "completed" => TaskStatus::Completed,
                "cancelled" => TaskStatus::Cancelled,
                _ => {
                    return Err(ApiErrorResponse::validation_error(
                        "Validation failed",
                        vec![FieldError::new(
                            "value",
                            format!("Invalid status: {status_str}"),
                        )],
                    ));
                }
            };

            // Create an iso between TaskStatus and its string representation
            // Note: This iso is used for demonstration; in production, use direct conversion
            let status_iso = iso!(
                |status: TaskStatus| match status {
                    TaskStatus::Pending => "pending".to_string(),
                    TaskStatus::InProgress => "in_progress".to_string(),
                    TaskStatus::Completed => "completed".to_string(),
                    TaskStatus::Cancelled => "cancelled".to_string(),
                },
                |s: String| match s.as_str() {
                    "pending" => TaskStatus::Pending,
                    "in_progress" => TaskStatus::InProgress,
                    "completed" => TaskStatus::Completed,
                    "cancelled" => TaskStatus::Cancelled,
                    _ => unreachable!("Status already validated"),
                }
            );

            // Use iso for bidirectional conversion (demonstration of iso! usage)
            let previous_str = status_iso.get(task.status);

            let previous = serde_json::json!(previous_str);
            let new = serde_json::json!(status_iso.get(new_status));

            // Combine lens with iso
            let status_lens = lens!(Task, status);
            let updated = status_lens.set(task, new_status);

            Ok((
                updated,
                previous,
                new,
                "lens!(Task, status) + iso!(TaskStatus, String)".to_string(),
            ))
        }

        "description_prism" => {
            // Demonstrate prism! for conditional access to Option<String>
            // prism!(Option<String>, Some) creates a prism focusing on the Some variant
            let description_prism = prism!(Option<String>, Some);
            let description_lens = lens!(Task, description);

            let current_description = task.description.clone();
            let previous = serde_json::json!(&current_description);

            let (new_description, optics_info) = match request.transform.as_deref() {
                Some("uppercase") => {
                    // Transform existing value to uppercase using prism's modify_option
                    let transformed =
                        description_prism.modify_option(current_description, |s| s.to_uppercase());
                    transformed.map_or(
                        (
                            None,
                            "prism!(Option<String>, Some) -> None (no modification)",
                        ),
                        |desc| {
                            (
                                desc,
                                "prism!(Option<String>, Some) + modify_option -> uppercase",
                            )
                        },
                    )
                }
                Some("lowercase") => {
                    // Transform existing value to lowercase using prism's modify_option
                    let transformed =
                        description_prism.modify_option(current_description, |s| s.to_lowercase());
                    transformed.map_or(
                        (
                            None,
                            "prism!(Option<String>, Some) -> None (no modification)",
                        ),
                        |desc| {
                            (
                                desc,
                                "prism!(Option<String>, Some) + modify_option -> lowercase",
                            )
                        },
                    )
                }
                Some("set_if_exists") => {
                    // Set new value only if current value exists (using prism's preview)
                    // value must be a string or null; validate type first
                    let is_null = request.value.is_null();
                    let new_value_str = request.value.as_str().map(ToString::to_string);

                    // Type validation: value must be string or null
                    if !is_null && new_value_str.is_none() {
                        return Err(ApiErrorResponse::validation_error(
                            "Validation failed",
                            vec![FieldError::new(
                                "value",
                                "Value must be a string or null for set_if_exists transform",
                            )],
                        ));
                    }

                    let preview_result = description_prism.preview(&current_description);
                    if preview_result.is_none() {
                        // Current value is None, do not set (regardless of value)
                        (
                            current_description,
                            "prism!(Option<String>, Some) -> None (not set)",
                        )
                    } else if is_null {
                        // null means "no change" - keep current value
                        (
                            current_description,
                            "prism!(Option<String>, Some) + preview -> unchanged (null value)",
                        )
                    } else {
                        // Use prism's review to construct Some variant
                        let constructed =
                            description_prism.review(new_value_str.expect("validated above"));
                        (
                            constructed,
                            "prism!(Option<String>, Some) + preview + review",
                        )
                    }
                }
                _ => {
                    // Default behavior: set value directly using prism's review
                    if let Some(new_val) = request.value.as_str() {
                        (
                            description_prism.review(new_val.to_string()),
                            "prism!(Option<String>, Some) + review",
                        )
                    } else if request.value.is_null() {
                        (None, "prism!(Option<String>, Some) -> None")
                    } else {
                        return Err(ApiErrorResponse::validation_error(
                            "Validation failed",
                            vec![FieldError::new(
                                "value",
                                "Description must be a string or null",
                            )],
                        ));
                    }
                }
            };

            let new = serde_json::json!(&new_description);
            let updated = description_lens.set(task, new_description);

            Ok((updated, previous, new, optics_info.to_string()))
        }

        _ => Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new(
                "field",
                format!("Unknown field: {}", request.field),
            )],
        )),
    }
}

// =============================================================================
// POST /tasks/state-workflow - State, Writer, RWS monads
// =============================================================================

/// Executes a state workflow using State, Writer, or RWS monad.
///
/// This handler demonstrates:
/// - **`State`**: Stateful computation threading
/// - **`Writer`**: Accumulated logging/output
/// - **`RWS`**: Combined Reader + Writer + State
///
/// # Request Body
///
/// ```json
/// {
///   "initial_count": 10,
///   "operations": [
///     { "type": "increment" },
///     { "type": "multiply", "factor": 2 },
///     { "type": "log", "message": "checkpoint" }
///   ],
///   "use_rws": true
/// }
/// ```
///
/// # Response
///
/// - **200 OK**: Workflow executed successfully
///
/// # Errors
///
/// This handler always returns `Ok` as state operations are performed synchronously.
#[allow(clippy::unused_async)]
pub async fn execute_state_workflow(
    State(_state): State<AppState>,
    Json(request): Json<StateWorkflowRequest>,
) -> Result<Json<StateWorkflowResponse>, ApiErrorResponse> {
    // Execute workflow synchronously (State/Writer/RWS use Rc, not Send)
    let response = if request.use_rws {
        execute_with_rws(&request)
    } else {
        execute_with_state_and_writer(&request)
    };

    Ok(Json(response))
}

/// Workflow state for tracking.
#[derive(Clone)]
struct WorkflowState {
    count: i32,
    history: Vec<i32>,
}

/// Environment for RWS monad.
#[derive(Clone)]
struct WorkflowEnv {
    multiplier_limit: i32,
}

/// Executes workflow using separate State and Writer monads.
fn execute_with_state_and_writer(request: &StateWorkflowRequest) -> StateWorkflowResponse {
    // Build state computation
    let initial_state = WorkflowState {
        count: request.initial_count,
        history: vec![request.initial_count],
    };

    // Process operations using State monad
    let mut computation: StateMonad<WorkflowState, ()> = StateMonad::pure(());

    for op in &request.operations {
        computation = computation.then(create_state_operation(op.clone()));
    }

    // Run the state computation
    let ((), final_state) = computation.run(initial_state.clone());

    // Collect logs using Writer monad
    let log_computation: Writer<Vec<String>, ()> =
        create_log_computation(&request.operations, &initial_state, &final_state);

    let ((), logs) = log_computation.run();

    StateWorkflowResponse {
        final_count: final_state.count,
        logs,
        monad_type: "State + Writer".to_string(),
        states: final_state.history,
    }
}

/// Creates a state operation as a State monad computation.
fn create_state_operation(op: StateOperation) -> StateMonad<WorkflowState, ()> {
    StateMonad::modify(move |mut state: WorkflowState| {
        match &op {
            StateOperation::Increment => state.count += 1,
            StateOperation::Decrement => state.count -= 1,
            StateOperation::Multiply { factor } => state.count *= factor,
            StateOperation::Add { value } => state.count += value,
            StateOperation::Log { .. } => {} // Log operations don't change state
        }
        state.history.push(state.count);
        state
    })
}

/// Creates a log computation using Writer monad.
fn create_log_computation(
    operations: &[StateOperation],
    initial: &WorkflowState,
    final_state: &WorkflowState,
) -> Writer<Vec<String>, ()> {
    let mut writer = Writer::tell(vec![format!(
        "Starting workflow with count: {}",
        initial.count
    )]);

    for (index, op) in operations.iter().enumerate() {
        let message = match op {
            StateOperation::Increment => format!("Step {}: increment", index + 1),
            StateOperation::Decrement => format!("Step {}: decrement", index + 1),
            StateOperation::Multiply { factor } => {
                format!("Step {}: multiply by {}", index + 1, factor)
            }
            StateOperation::Add { value } => format!("Step {}: add {}", index + 1, value),
            StateOperation::Log { message } => format!("Step {}: log '{}'", index + 1, message),
        };
        writer = writer.then(Writer::tell(vec![message]));
    }

    writer.then(Writer::tell(vec![format!(
        "Workflow complete. Final count: {}",
        final_state.count
    )]))
}

/// Executes workflow using RWS monad (combined Reader + Writer + State).
fn execute_with_rws(request: &StateWorkflowRequest) -> StateWorkflowResponse {
    let env = WorkflowEnv {
        multiplier_limit: 100,
    };

    let initial_state = WorkflowState {
        count: request.initial_count,
        history: vec![request.initial_count],
    };

    // Build RWS computation
    let computation = create_rws_workflow(&request.operations);

    // Run the RWS computation
    let ((), final_state, logs) = computation.run(env, initial_state);

    StateWorkflowResponse {
        final_count: final_state.count,
        logs,
        monad_type: "RWS (Reader + Writer + State)".to_string(),
        states: final_state.history,
    }
}

/// Creates an RWS workflow computation.
fn create_rws_workflow(
    operations: &[StateOperation],
) -> RWS<WorkflowEnv, Vec<String>, WorkflowState, ()> {
    // Start with initial log
    let mut computation: RWS<WorkflowEnv, Vec<String>, WorkflowState, ()> =
        RWS::tell(vec!["RWS workflow started".to_string()]);

    for op in operations {
        computation = computation.then(create_rws_operation(op.clone()));
    }

    computation.then(RWS::tell(vec!["RWS workflow complete".to_string()]))
}

/// Creates an RWS operation.
fn create_rws_operation(op: StateOperation) -> RWS<WorkflowEnv, Vec<String>, WorkflowState, ()> {
    match op {
        StateOperation::Increment => RWS::modify(|mut s: WorkflowState| {
            s.count += 1;
            s.history.push(s.count);
            s
        })
        .then(RWS::tell(vec!["Incremented".to_string()])),
        StateOperation::Decrement => RWS::modify(|mut s: WorkflowState| {
            s.count -= 1;
            s.history.push(s.count);
            s
        })
        .then(RWS::tell(vec!["Decremented".to_string()])),
        StateOperation::Multiply { factor } => {
            // Demonstrate Reader: check environment limit
            RWS::ask().flat_map(move |env: WorkflowEnv| {
                let actual_factor = if factor > env.multiplier_limit {
                    env.multiplier_limit
                } else {
                    factor
                };
                RWS::modify(move |mut s: WorkflowState| {
                    s.count *= actual_factor;
                    s.history.push(s.count);
                    s
                })
                .then(RWS::tell(vec![format!("Multiplied by {}", actual_factor)]))
            })
        }
        StateOperation::Add { value } => RWS::modify(move |mut s: WorkflowState| {
            s.count += value;
            s.history.push(s.count);
            s
        })
        .then(RWS::tell(vec![format!("Added {}", value)])),
        StateOperation::Log { message } => RWS::tell(vec![format!("Log: {}", message)]),
    }
}

// =============================================================================
// Prism demonstration helper (for Option types)
// =============================================================================

/// Demonstrates prism usage with Option type.
///
/// This is a helper function showing how prism! works with enum variants.
#[allow(dead_code)]
fn demonstrate_prism_usage() {
    // prism! for Option<String>
    let some_prism = prism!(Option<String>, Some);

    let some_value: Option<String> = Some("hello".to_string());
    let none_value: Option<String> = None;

    // Preview: extract value if variant matches
    assert_eq!(some_prism.preview(&some_value), Some(&"hello".to_string()));
    assert_eq!(some_prism.preview(&none_value), None);

    // Review: construct variant from value
    let constructed = some_prism.review("world".to_string());
    assert_eq!(constructed, Some("world".to_string()));

    // Modify: transform value if variant matches
    let modified = some_prism.modify_option(some_value, |s| s.to_uppercase());
    assert_eq!(modified, Some(Some("HELLO".to_string())));
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // Workflow Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_create_validation_workflow_valid() {
        let request = WorkflowRequest {
            title: "Valid Title".to_string(),
            priority: "high".to_string(),
            enable_logging: false,
        };

        let computation = create_validation_workflow(&request);
        let result = PureHandler.run(computation);

        assert!(result);
    }

    #[rstest]
    fn test_create_validation_workflow_invalid_title() {
        let request = WorkflowRequest {
            title: String::new(),
            priority: "high".to_string(),
            enable_logging: false,
        };

        let computation = create_validation_workflow(&request);
        let result = PureHandler.run(computation);

        assert!(!result);
    }

    #[rstest]
    fn test_create_validation_workflow_invalid_priority() {
        let request = WorkflowRequest {
            title: "Valid Title".to_string(),
            priority: "invalid".to_string(),
            enable_logging: false,
        };

        let computation = create_validation_workflow(&request);
        let result = PureHandler.run(computation);

        assert!(!result);
    }

    // -------------------------------------------------------------------------
    // State Workflow Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_execute_with_state_and_writer() {
        let request = StateWorkflowRequest {
            initial_count: 10,
            operations: vec![
                StateOperation::Increment,
                StateOperation::Increment,
                StateOperation::Multiply { factor: 2 },
            ],
            use_rws: false,
        };

        let response = execute_with_state_and_writer(&request);

        assert_eq!(response.final_count, 24); // (10 + 1 + 1) * 2 = 24
        assert_eq!(response.monad_type, "State + Writer");
        assert!(!response.logs.is_empty());
    }

    #[rstest]
    fn test_execute_with_rws() {
        let request = StateWorkflowRequest {
            initial_count: 5,
            operations: vec![
                StateOperation::Add { value: 5 },
                StateOperation::Multiply { factor: 3 },
            ],
            use_rws: true,
        };

        let response = execute_with_rws(&request);

        assert_eq!(response.final_count, 30); // (5 + 5) * 3 = 30
        assert_eq!(response.monad_type, "RWS (Reader + Writer + State)");
    }

    #[rstest]
    fn test_rws_respects_multiplier_limit() {
        let request = StateWorkflowRequest {
            initial_count: 10,
            operations: vec![StateOperation::Multiply { factor: 1000 }], // Exceeds limit of 100
            use_rws: true,
        };

        let response = execute_with_rws(&request);

        // Should use limit of 100 instead of 1000
        assert_eq!(response.final_count, 1000); // 10 * 100 = 1000
    }

    // -------------------------------------------------------------------------
    // Prism Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_prism_preview_some() {
        let some_prism = prism!(Option<i32>, Some);
        let value: Option<i32> = Some(42);

        assert_eq!(some_prism.preview(&value), Some(&42));
    }

    #[rstest]
    fn test_prism_preview_none() {
        let some_prism = prism!(Option<i32>, Some);
        let value: Option<i32> = None;

        assert_eq!(some_prism.preview(&value), None);
    }

    #[rstest]
    fn test_prism_review() {
        let some_prism = prism!(Option<i32>, Some);
        let constructed = some_prism.review(42);

        assert_eq!(constructed, Some(42));
    }

    // -------------------------------------------------------------------------
    // Iso Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_iso_bidirectional() {
        let int_string_iso = iso!(|n: i32| n.to_string(), |s: String| s
            .parse::<i32>()
            .unwrap());

        let original = 42;
        let converted = int_string_iso.get(original);
        let back = int_string_iso.reverse_get(converted);

        assert_eq!(back, original);
    }

    #[rstest]
    fn test_iso_modify() {
        let int_string_iso = iso!(|n: i32| n.to_string(), |s: String| s
            .parse::<i32>()
            .unwrap());

        let result = int_string_iso.modify(42, |s| format!("{s}0")); // "42" -> "420"

        assert_eq!(result, 420);
    }

    // -------------------------------------------------------------------------
    // Optics Update Tests
    // -------------------------------------------------------------------------

    fn create_test_task() -> Task {
        use crate::domain::Timestamp;
        Task::new(
            TaskId::generate(),
            "Test Task".to_string(),
            Timestamp::now(),
        )
    }

    #[rstest]
    fn test_apply_optics_update_title() {
        let task = create_test_task();
        let request = OpticsRequest {
            field: "title".to_string(),
            value: serde_json::json!("New Title"),
            transform: None,
        };

        let result = apply_optics_update(task, &request);
        assert!(result.is_ok());

        let (updated, _, new_value, optics_used) = result.unwrap();
        assert_eq!(updated.title, "New Title");
        assert_eq!(new_value, serde_json::json!("New Title"));
        assert!(optics_used.contains("lens!"));
    }

    #[rstest]
    fn test_apply_optics_update_title_with_transform() {
        let task = create_test_task();
        let request = OpticsRequest {
            field: "title".to_string(),
            value: serde_json::json!("hello world"),
            transform: Some("uppercase".to_string()),
        };

        let result = apply_optics_update(task, &request);
        assert!(result.is_ok());

        let (updated, _, _, _) = result.unwrap();
        assert_eq!(updated.title, "HELLO WORLD");
    }

    #[rstest]
    fn test_apply_optics_update_priority() {
        let task = create_test_task();
        let request = OpticsRequest {
            field: "priority".to_string(),
            value: serde_json::json!("high"),
            transform: None,
        };

        let result = apply_optics_update(task, &request);
        assert!(result.is_ok());

        let (updated, _, _, _) = result.unwrap();
        assert_eq!(updated.priority, Priority::High);
    }

    #[rstest]
    fn test_apply_optics_update_invalid_priority() {
        let task = create_test_task();
        let request = OpticsRequest {
            field: "priority".to_string(),
            value: serde_json::json!("invalid_priority"),
            transform: None,
        };

        let result = apply_optics_update(task, &request);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_apply_optics_update_status() {
        let task = create_test_task();
        let request = OpticsRequest {
            field: "status".to_string(),
            value: serde_json::json!("in_progress"),
            transform: None,
        };

        let result = apply_optics_update(task, &request);
        assert!(result.is_ok());

        let (updated, _, new_value, optics_used) = result.unwrap();
        assert_eq!(updated.status, TaskStatus::InProgress);
        assert_eq!(new_value, serde_json::json!("in_progress"));
        assert!(optics_used.contains("iso!"));
    }

    #[rstest]
    fn test_apply_optics_update_invalid_status() {
        let task = create_test_task();
        let request = OpticsRequest {
            field: "status".to_string(),
            value: serde_json::json!("invalid_status"),
            transform: None,
        };

        let result = apply_optics_update(task, &request);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_apply_optics_update_description() {
        let task = create_test_task();
        let request = OpticsRequest {
            field: "description".to_string(),
            value: serde_json::json!("New description"),
            transform: None,
        };

        let result = apply_optics_update(task, &request);
        assert!(result.is_ok());

        let (updated, _, _, _) = result.unwrap();
        assert_eq!(updated.description, Some("New description".to_string()));
    }

    #[rstest]
    fn test_apply_optics_update_description_null() {
        let mut task = create_test_task();
        task = task.with_description("Old description".to_string());

        let request = OpticsRequest {
            field: "description".to_string(),
            value: serde_json::Value::Null,
            transform: None,
        };

        let result = apply_optics_update(task, &request);
        assert!(result.is_ok());

        let (updated, _, _, _) = result.unwrap();
        assert_eq!(updated.description, None);
    }

    #[rstest]
    fn test_apply_optics_update_unknown_field() {
        let task = create_test_task();
        let request = OpticsRequest {
            field: "unknown_field".to_string(),
            value: serde_json::json!("value"),
            transform: None,
        };

        let result = apply_optics_update(task, &request);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_apply_optics_update_wrong_value_type() {
        let task = create_test_task();
        let request = OpticsRequest {
            field: "title".to_string(),
            value: serde_json::json!(123), // Number instead of string
            transform: None,
        };

        let result = apply_optics_update(task, &request);
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // Description Prism Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_apply_optics_update_description_prism_uppercase() {
        let mut task = create_test_task();
        task = task.with_description("hello world".to_string());

        let request = OpticsRequest {
            field: "description_prism".to_string(),
            value: serde_json::Value::Null,
            transform: Some("uppercase".to_string()),
        };

        let result = apply_optics_update(task, &request);
        assert!(result.is_ok());

        let (updated, _, new_value, optics_used) = result.unwrap();
        assert_eq!(updated.description, Some("HELLO WORLD".to_string()));
        assert_eq!(new_value, serde_json::json!("HELLO WORLD"));
        assert!(optics_used.contains("prism!"));
        assert!(optics_used.contains("uppercase"));
    }

    #[rstest]
    fn test_apply_optics_update_description_prism_uppercase_none() {
        let task = create_test_task(); // description is None

        let request = OpticsRequest {
            field: "description_prism".to_string(),
            value: serde_json::Value::Null,
            transform: Some("uppercase".to_string()),
        };

        let result = apply_optics_update(task, &request);
        assert!(result.is_ok());

        let (updated, _, _, optics_used) = result.unwrap();
        assert_eq!(updated.description, None);
        assert!(optics_used.contains("None"));
    }

    #[rstest]
    fn test_apply_optics_update_description_prism_lowercase() {
        let mut task = create_test_task();
        task = task.with_description("HELLO WORLD".to_string());

        let request = OpticsRequest {
            field: "description_prism".to_string(),
            value: serde_json::Value::Null,
            transform: Some("lowercase".to_string()),
        };

        let result = apply_optics_update(task, &request);
        assert!(result.is_ok());

        let (updated, _, new_value, optics_used) = result.unwrap();
        assert_eq!(updated.description, Some("hello world".to_string()));
        assert_eq!(new_value, serde_json::json!("hello world"));
        assert!(optics_used.contains("prism!"));
        assert!(optics_used.contains("lowercase"));
    }

    #[rstest]
    fn test_apply_optics_update_description_prism_set_if_exists() {
        let mut task = create_test_task();
        task = task.with_description("old description".to_string());

        let request = OpticsRequest {
            field: "description_prism".to_string(),
            value: serde_json::json!("new description"),
            transform: Some("set_if_exists".to_string()),
        };

        let result = apply_optics_update(task, &request);
        assert!(result.is_ok());

        let (updated, _, new_value, optics_used) = result.unwrap();
        assert_eq!(updated.description, Some("new description".to_string()));
        assert_eq!(new_value, serde_json::json!("new description"));
        assert!(optics_used.contains("prism!"));
        assert!(optics_used.contains("preview"));
        assert!(optics_used.contains("review"));
    }

    #[rstest]
    fn test_apply_optics_update_description_prism_set_if_exists_none() {
        let task = create_test_task(); // description is None

        let request = OpticsRequest {
            field: "description_prism".to_string(),
            value: serde_json::json!("new description"),
            transform: Some("set_if_exists".to_string()),
        };

        let result = apply_optics_update(task, &request);
        assert!(result.is_ok());

        let (updated, _, _, optics_used) = result.unwrap();
        assert_eq!(updated.description, None); // Should not set because original is None
        assert!(optics_used.contains("not set"));
    }

    #[rstest]
    fn test_apply_optics_update_description_prism_default() {
        let task = create_test_task();

        let request = OpticsRequest {
            field: "description_prism".to_string(),
            value: serde_json::json!("new value"),
            transform: None,
        };

        let result = apply_optics_update(task, &request);
        assert!(result.is_ok());

        let (updated, _, new_value, optics_used) = result.unwrap();
        assert_eq!(updated.description, Some("new value".to_string()));
        assert_eq!(new_value, serde_json::json!("new value"));
        assert!(optics_used.contains("prism!"));
        assert!(optics_used.contains("review"));
    }

    #[rstest]
    fn test_apply_optics_update_description_prism_default_null() {
        let mut task = create_test_task();
        task = task.with_description("old description".to_string());

        let request = OpticsRequest {
            field: "description_prism".to_string(),
            value: serde_json::Value::Null,
            transform: None,
        };

        let result = apply_optics_update(task, &request);
        assert!(result.is_ok());

        let (updated, _, _, optics_used) = result.unwrap();
        assert_eq!(updated.description, None);
        assert!(optics_used.contains("None"));
    }

    #[rstest]
    fn test_apply_optics_update_description_prism_set_if_exists_null_keeps_current() {
        let mut task = create_test_task();
        task = task.with_description("keep this".to_string());

        let request = OpticsRequest {
            field: "description_prism".to_string(),
            value: serde_json::Value::Null,
            transform: Some("set_if_exists".to_string()),
        };

        let result = apply_optics_update(task, &request);
        assert!(result.is_ok());

        let (updated, _, _, optics_used) = result.unwrap();
        // null value means "no change" - should keep current value
        assert_eq!(updated.description, Some("keep this".to_string()));
        assert!(optics_used.contains("unchanged"));
    }

    #[rstest]
    fn test_apply_optics_update_description_prism_set_if_exists_invalid_type() {
        let mut task = create_test_task();
        task = task.with_description("old description".to_string());

        let request = OpticsRequest {
            field: "description_prism".to_string(),
            value: serde_json::json!(123), // Number instead of string
            transform: Some("set_if_exists".to_string()),
        };

        let result = apply_optics_update(task, &request);
        assert!(result.is_err()); // Should fail validation
    }

    #[rstest]
    fn test_apply_optics_update_description_prism_set_if_exists_invalid_type_none_description() {
        let task = create_test_task(); // description is None

        let request = OpticsRequest {
            field: "description_prism".to_string(),
            value: serde_json::json!(123), // Number instead of string
            transform: Some("set_if_exists".to_string()),
        };

        let result = apply_optics_update(task, &request);
        // Type validation happens before preview check, so should fail even when description is None
        assert!(result.is_err());
    }
}
