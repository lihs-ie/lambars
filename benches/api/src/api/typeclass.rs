//! Type class handlers demonstrating lambars type class hierarchy.
//!
//! This module showcases:
//! - `ReaderT`, `StateT`, `WriterT` monad transformers
//! - `FunctorMut` for multi-element containers
//! - `Flatten` for nested type unwrapping
//! - `MonadError` for error handling
//! - `Identity` as the simplest functor
//!
//! # lambars Features Demonstrated
//!
//! - **Monad Transformers**: Composing effects with transformers
//! - **Type Classes**: Functor, Applicative, Monad hierarchy
//! - **Error Handling**: `MonadError` for functional error management

use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};

use lambars::effect::{MonadError, ReaderT, StateT, WriterT};
use lambars::typeclass::{Flatten, Functor, FunctorMut, Identity, Monad};

use super::error::ApiErrorResponse;
use super::handlers::AppState;

// =============================================================================
// Request/Response DTOs
// =============================================================================

/// Request body for monad transformer operations.
#[derive(Debug, Clone, Deserialize)]
pub struct MonadTransformerRequest {
    /// Base value to transform.
    pub base_value: i32,
    /// Environment value for `ReaderT`.
    pub environment: i32,
    /// Initial state for `StateT`.
    pub initial_state: i32,
    /// Log prefix for `WriterT`.
    pub log_prefix: String,
    /// Operations to perform.
    pub operations: Vec<TransformerOperation>,
}

/// Operation types for transformers.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TransformerOperation {
    /// Add environment value (`ReaderT`).
    AddEnvironment,
    /// Multiply by state (`StateT`).
    MultiplyByState,
    /// Log current value (`WriterT`).
    LogValue { message: String },
    /// Double the value.
    Double,
    /// Add a constant.
    Add { value: i32 },
}

/// Response from monad transformer operations.
#[derive(Debug, Clone, Serialize)]
pub struct MonadTransformerResponse {
    /// Final computed value.
    pub result: i32,
    /// Final state (from `StateT`).
    pub final_state: i32,
    /// Accumulated logs (from `WriterT`).
    pub logs: Vec<String>,
    /// Transformers used.
    pub transformers_used: Vec<String>,
}

/// Request body for `FunctorMut` operations.
#[derive(Debug, Clone, Deserialize)]
pub struct FunctorMutRequest {
    /// Values to transform.
    pub values: Vec<i32>,
    /// Transformation to apply.
    pub transformation: String,
}

/// Response from `FunctorMut` operations.
#[derive(Debug, Clone, Serialize)]
pub struct FunctorMutResponse {
    /// Original values.
    pub original: Vec<i32>,
    /// Transformed values.
    pub transformed: Vec<i32>,
    /// Transformation applied.
    pub transformation: String,
}

/// Request body for Flatten operations.
#[derive(Debug, Clone, Deserialize)]
pub struct FlattenRequest {
    /// Nested optional value (None represented as null).
    pub nested_option: Option<Option<i32>>,
    /// Nested result values.
    pub nested_results: Vec<NestedResult>,
}

/// Nested result for Flatten demo.
#[derive(Debug, Clone, Deserialize)]
pub struct NestedResult {
    /// Outer result success.
    pub outer_ok: bool,
    /// Inner result success.
    pub inner_ok: bool,
    /// Value if both succeed.
    pub value: i32,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Response from Flatten operations.
#[derive(Debug, Clone, Serialize)]
pub struct FlattenResponse {
    /// Flattened option result.
    pub flattened_option: Option<i32>,
    /// Flattened results.
    pub flattened_results: Vec<FlattenedResultItem>,
}

/// Single flattened result item.
#[derive(Debug, Clone, Serialize)]
pub struct FlattenedResultItem {
    /// Whether the flatten succeeded.
    pub success: bool,
    /// Value if success.
    pub value: Option<i32>,
    /// Error if failed.
    pub error: Option<String>,
}

/// Request body for `MonadError` operations.
#[derive(Debug, Clone, Deserialize)]
pub struct MonadErrorRequest {
    /// Operations that may fail.
    pub operations: Vec<FallibleOperation>,
    /// Whether to recover from errors.
    pub recover_on_error: bool,
    /// Default value for recovery.
    pub recovery_value: i32,
}

/// An operation that may fail.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FallibleOperation {
    /// Pure value (always succeeds).
    Pure { value: i32 },
    /// Division (fails on zero).
    Divide { dividend: i32, divisor: i32 },
    /// Parse (fails on invalid input).
    Parse { input: String },
    /// Throw error explicitly.
    ThrowError { message: String },
}

/// Response from `MonadError` operations.
#[derive(Debug, Clone, Serialize)]
pub struct MonadErrorResponse {
    /// Results of each operation.
    pub results: Vec<OperationResult>,
    /// Whether recovery was applied.
    pub recovery_applied: bool,
    /// Final aggregated value.
    pub final_value: Option<i32>,
}

/// Result of a single operation.
#[derive(Debug, Clone, Serialize)]
pub struct OperationResult {
    /// Operation description.
    pub operation: String,
    /// Whether it succeeded.
    pub success: bool,
    /// Value if success.
    pub value: Option<i32>,
    /// Error if failed.
    pub error: Option<String>,
}

/// Request body for Identity operations.
#[derive(Debug, Clone, Deserialize)]
pub struct IdentityRequest {
    /// Value to wrap.
    pub value: i32,
    /// Transformations to apply.
    pub transformations: Vec<String>,
}

/// Response from Identity operations.
#[derive(Debug, Clone, Serialize)]
pub struct IdentityResponse {
    /// Original value.
    pub original: i32,
    /// Wrapped value after transformations.
    pub result: i32,
    /// Transformations applied.
    pub transformations_applied: Vec<String>,
    /// Demonstrates Identity laws.
    pub law_demonstrations: Vec<LawDemonstration>,
}

/// Demonstration of a type class law.
#[derive(Debug, Clone, Serialize)]
pub struct LawDemonstration {
    /// Name of the law.
    pub law_name: String,
    /// Left-hand side of equality.
    pub lhs: String,
    /// Right-hand side of equality.
    pub rhs: String,
    /// Whether the law holds.
    pub holds: bool,
}

// =============================================================================
// POST /tasks/monad-transformers - ReaderT, StateT, WriterT
// =============================================================================

/// Demonstrates monad transformers for effect handling.
///
/// This handler showcases each transformer **individually** (not composed):
/// - **`ReaderT`**: Environment reading transformer
/// - **`StateT`**: State threading transformer
/// - **`WriterT`**: Logging/output accumulation transformer
///
/// **Note**: For simplicity, this demo shows each transformer separately.
/// True transformer composition (e.g., `ReaderT<StateT<WriterT<...>>>`) requires
/// more complex setup and is typically used in larger applications.
///
/// # Request Body
///
/// ```json
/// {
///   "base_value": 10,
///   "environment": 5,
///   "initial_state": 2,
///   "log_prefix": "Step",
///   "operations": [
///     { "type": "add_environment" },
///     { "type": "multiply_by_state" },
///     { "type": "log_value", "message": "computed" }
///   ]
/// }
/// ```
///
/// # Response
///
/// - **200 OK**: Transformations applied successfully
///
/// # Errors
///
/// This handler always returns `Ok` as transformations are performed synchronously.
#[allow(clippy::unused_async)]
pub async fn monad_transformers(
    State(_state): State<AppState>,
    Json(request): Json<MonadTransformerRequest>,
) -> Result<Json<MonadTransformerResponse>, ApiErrorResponse> {
    // Demonstrate each transformer separately, then show composition
    let response = {
        let mut transformers_used = Vec::new();
        let mut logs = Vec::new();

        // ReaderT demonstration: Read environment value
        let reader_computation: ReaderT<i32, Option<i32>> =
            ReaderT::new(move |env| Some(request.base_value + env));
        let reader_result = reader_computation.run(request.environment);
        transformers_used.push("ReaderT<i32, Option<i32>>".to_string());

        let after_reader = reader_result.unwrap_or(request.base_value);
        logs.push(format!(
            "{}: After ReaderT (base + env): {}",
            request.log_prefix, after_reader
        ));

        // StateT demonstration: Thread state through computation
        let state_computation: StateT<i32, Option<(i32, i32)>> = StateT::new(move |state| {
            let new_value = after_reader * state;
            let new_state = state + 1;
            Some((new_value, new_state))
        });
        let state_result = state_computation.run(request.initial_state);
        transformers_used.push("StateT<i32, Option<(i32, i32)>>".to_string());

        let (after_state, final_state) =
            state_result.unwrap_or((after_reader, request.initial_state));
        logs.push(format!(
            "{}: After StateT (value * state): {}, state: {}",
            request.log_prefix, after_state, final_state
        ));

        // WriterT demonstration: Accumulate logs
        // WriterT<W, M> where M = Option<(A, W)>
        let writer_computation: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
            WriterT::new(Some((
                after_state,
                vec![format!("Final value computed: {after_state}")],
            )));
        let writer_result = writer_computation.run();
        transformers_used.push("WriterT<Vec<String>, Option<(i32, Vec<String>)>>".to_string());

        let (final_value, writer_logs) = writer_result.unwrap_or((after_state, vec![]));
        logs.extend(writer_logs);

        // Apply additional operations
        let mut result = final_value;
        for op in &request.operations {
            match op {
                TransformerOperation::AddEnvironment => {
                    result += request.environment;
                    logs.push(format!(
                        "{}: Added environment: {}",
                        request.log_prefix, result
                    ));
                }
                TransformerOperation::MultiplyByState => {
                    result *= request.initial_state;
                    logs.push(format!(
                        "{}: Multiplied by state: {}",
                        request.log_prefix, result
                    ));
                }
                TransformerOperation::LogValue { message } => {
                    logs.push(format!(
                        "{}: {} - value is {}",
                        request.log_prefix, message, result
                    ));
                }
                TransformerOperation::Double => {
                    result *= 2;
                    logs.push(format!("{}: Doubled: {}", request.log_prefix, result));
                }
                TransformerOperation::Add { value } => {
                    result += value;
                    logs.push(format!(
                        "{}: Added {}: {}",
                        request.log_prefix, value, result
                    ));
                }
            }
        }

        MonadTransformerResponse {
            result,
            final_state,
            logs,
            transformers_used,
        }
    };

    Ok(Json(response))
}

// =============================================================================
// GET /tasks/functor-mut - FunctorMut
// =============================================================================

/// Demonstrates `FunctorMut` for multi-element transformations.
///
/// This handler showcases:
/// - **`FunctorMut`**: Functor with mutable function support
/// - **`fmap_mut`**: Apply function to all elements
///
/// # Request Body
///
/// ```json
/// {
///   "values": [1, 2, 3, 4, 5],
///   "transformation": "double"
/// }
/// ```
///
/// # Response
///
/// - **200 OK**: Transformation applied successfully
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] if transformation is invalid.
#[allow(clippy::unused_async)]
pub async fn functor_mut_demo(
    State(_state): State<AppState>,
    Json(request): Json<FunctorMutRequest>,
) -> Result<Json<FunctorMutResponse>, ApiErrorResponse> {
    let transformed: Vec<i32> = match request.transformation.as_str() {
        "double" => request.values.clone().fmap_mut(|x| x * 2),
        "square" => request.values.clone().fmap_mut(|x| x * x),
        "negate" => request.values.clone().fmap_mut(|x| -x),
        "increment" => request.values.clone().fmap_mut(|x| x + 1),
        "abs" => request.values.clone().fmap_mut(i32::abs),
        _ => {
            return Err(ApiErrorResponse::bad_request(
                "INVALID_TRANSFORMATION",
                format!("Unknown transformation: {}", request.transformation),
            ));
        }
    };

    Ok(Json(FunctorMutResponse {
        original: request.values,
        transformed,
        transformation: request.transformation,
    }))
}

// =============================================================================
// GET /tasks/flatten - Flatten
// =============================================================================

/// Demonstrates Flatten for nested type unwrapping.
///
/// This handler showcases:
/// - **`Flatten`**: Remove one level of nesting
/// - Works with `Option<Option<T>>`, `Result<Result<T, E>, E>`
///
/// # Request Body
///
/// ```json
/// {
///   "nested_option": [[42]],
///   "nested_results": [
///     { "outer_ok": true, "inner_ok": true, "value": 100 }
///   ]
/// }
/// ```
///
/// # Response
///
/// - **200 OK**: Flatten applied successfully
///
/// # Errors
///
/// This handler always returns `Ok`.
#[allow(clippy::unused_async)]
pub async fn flatten_demo(
    State(_state): State<AppState>,
    Json(request): Json<FlattenRequest>,
) -> Result<Json<FlattenResponse>, ApiErrorResponse> {
    // Flatten Option<Option<T>>
    let flattened_option = request.nested_option.flatten();

    // Flatten nested results
    let flattened_results: Vec<FlattenedResultItem> = request
        .nested_results
        .into_iter()
        .map(|nested| {
            let outer: Result<Result<i32, String>, String> = if nested.outer_ok {
                if nested.inner_ok {
                    Ok(Ok(nested.value))
                } else {
                    Ok(Err(nested
                        .error
                        .unwrap_or_else(|| "Inner error".to_string())))
                }
            } else {
                Err(nested.error.unwrap_or_else(|| "Outer error".to_string()))
            };

            // Use Flatten trait
            let flattened: Result<i32, String> = outer.flatten();

            match flattened {
                Ok(v) => FlattenedResultItem {
                    success: true,
                    value: Some(v),
                    error: None,
                },
                Err(e) => FlattenedResultItem {
                    success: false,
                    value: None,
                    error: Some(e),
                },
            }
        })
        .collect();

    Ok(Json(FlattenResponse {
        flattened_option,
        flattened_results,
    }))
}

// =============================================================================
// POST /tasks/monad-error - MonadError
// =============================================================================

/// Demonstrates `MonadError` for functional error handling.
///
/// This handler showcases:
/// - **`MonadError`**: `throw_error`, `catch_error` operations
/// - **`MonadErrorExt`**: `map_error` for error transformation
/// - Error recovery patterns
///
/// **Accumulation behavior**:
/// - With `recover_on_error: true`: All operations contribute to the sum,
///   with failed operations using `recovery_value` instead.
/// - With `recover_on_error: false`: The first error short-circuits the sum,
///   resulting in `final_value: null`. Individual results are still recorded.
///
/// # Request Body
///
/// ```json
/// {
///   "operations": [
///     { "type": "pure", "value": 10 },
///     { "type": "divide", "dividend": 100, "divisor": 5 },
///     { "type": "parse", "input": "42" }
///   ],
///   "recover_on_error": true,
///   "recovery_value": 0
/// }
/// ```
///
/// # Response
///
/// - **200 OK**: Operations executed
///
/// # Errors
///
/// This handler always returns `Ok` (errors are handled within the response).
#[allow(clippy::unused_async)]
pub async fn monad_error_demo(
    State(_state): State<AppState>,
    Json(request): Json<MonadErrorRequest>,
) -> Result<Json<MonadErrorResponse>, ApiErrorResponse> {
    let mut results = Vec::new();
    let mut sum: Result<i32, String> = Ok(0);
    let mut recovery_applied = false;
    let recovery_value = request.recovery_value;

    for op in &request.operations {
        let (op_result, was_error): (Result<i32, String>, bool) = match op {
            FallibleOperation::Pure { value } => {
                results.push(OperationResult {
                    operation: format!("pure({value})"),
                    success: true,
                    value: Some(*value),
                    error: None,
                });
                (Ok(*value), false)
            }
            FallibleOperation::Divide { dividend, divisor } => {
                if *divisor == 0 {
                    let error = "Division by zero".to_string();
                    results.push(OperationResult {
                        operation: format!("divide({dividend}, {divisor})"),
                        success: false,
                        value: None,
                        error: Some(error.clone()),
                    });
                    (Err(error), true)
                } else {
                    let value = dividend / divisor;
                    results.push(OperationResult {
                        operation: format!("divide({dividend}, {divisor})"),
                        success: true,
                        value: Some(value),
                        error: None,
                    });
                    (Ok(value), false)
                }
            }
            FallibleOperation::Parse { input } => {
                if let Ok(value) = input.parse::<i32>() {
                    results.push(OperationResult {
                        operation: format!("parse(\"{input}\")"),
                        success: true,
                        value: Some(value),
                        error: None,
                    });
                    (Ok(value), false)
                } else {
                    let error = format!("Cannot parse '{input}' as integer");
                    results.push(OperationResult {
                        operation: format!("parse(\"{input}\")"),
                        success: false,
                        value: None,
                        error: Some(error.clone()),
                    });
                    (Err(error), true)
                }
            }
            FallibleOperation::ThrowError { message } => {
                // Demonstrate throw_error
                let thrown: Result<i32, String> =
                    <Result<i32, String>>::throw_error(message.clone());
                results.push(OperationResult {
                    operation: format!("throw_error(\"{message}\")"),
                    success: false,
                    value: None,
                    error: Some(message.clone()),
                });
                (thrown, true)
            }
        };

        // Track if recovery was applied (error occurred and recovery enabled)
        if was_error && request.recover_on_error {
            recovery_applied = true;
        }

        // Use MonadError::catch_error for recovery
        let processed = if request.recover_on_error {
            <Result<i32, String>>::catch_error(op_result, move |_| Ok(recovery_value))
        } else {
            op_result
        };

        // Accumulate using flat_map
        sum = sum.flat_map(|acc| processed.fmap(move |v| acc + v));
    }

    let final_value = sum.ok();

    Ok(Json(MonadErrorResponse {
        results,
        recovery_applied,
        final_value,
    }))
}

// =============================================================================
// GET /tasks/identity-type - Identity
// =============================================================================

/// Demonstrates the Identity type as the simplest functor.
///
/// This handler showcases:
/// - **`Identity<A>`**: The trivial wrapper type
/// - Functor, Applicative, Monad instances for Identity
/// - Type class law demonstrations
///
/// # Request Body
///
/// ```json
/// {
///   "value": 42,
///   "transformations": ["double", "increment"]
/// }
/// ```
///
/// # Response
///
/// - **200 OK**: Transformations applied
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] if transformation is invalid.
#[allow(clippy::unused_async)]
pub async fn identity_demo(
    State(_state): State<AppState>,
    Json(request): Json<IdentityRequest>,
) -> Result<Json<IdentityResponse>, ApiErrorResponse> {
    let mut wrapped = Identity::new(request.value);
    let mut transformations_applied = Vec::new();

    for transformation in &request.transformations {
        wrapped = match transformation.as_str() {
            "double" => {
                transformations_applied.push("double (fmap |x| x * 2)".to_string());
                wrapped.fmap(|x| x * 2)
            }
            "increment" => {
                transformations_applied.push("increment (fmap |x| x + 1)".to_string());
                wrapped.fmap(|x| x + 1)
            }
            "square" => {
                transformations_applied.push("square (fmap |x| x * x)".to_string());
                wrapped.fmap(|x| x * x)
            }
            "negate" => {
                transformations_applied.push("negate (fmap |x| -x)".to_string());
                wrapped.fmap(|x| -x)
            }
            _ => {
                return Err(ApiErrorResponse::bad_request(
                    "INVALID_TRANSFORMATION",
                    format!("Unknown transformation: {transformation}"),
                ));
            }
        };
    }

    // Demonstrate type class laws
    let law_demonstrations = demonstrate_identity_laws(request.value);

    Ok(Json(IdentityResponse {
        original: request.value,
        result: wrapped.into_inner(),
        transformations_applied,
        law_demonstrations,
    }))
}

/// Demonstrates that Identity satisfies type class laws.
fn demonstrate_identity_laws(value: i32) -> Vec<LawDemonstration> {
    let mut laws = Vec::new();

    // Functor Identity Law: fmap id == id
    let id_wrapped = Identity::new(value);
    let fmapped: Identity<i32> = id_wrapped.fmap(|x| x);
    let identity_holds = fmapped.into_inner() == value;
    laws.push(LawDemonstration {
        law_name: "Functor Identity".to_string(),
        lhs: "Identity(x).fmap(|x| x)".to_string(),
        rhs: "Identity(x)".to_string(),
        holds: identity_holds,
    });

    // Functor Composition Law: fmap (f . g) == fmap f . fmap g
    let f = |x: i32| x * 2;
    let g = |x: i32| x + 1;
    let composed_first = Identity::new(value).fmap(move |x| f(g(x)));
    let chained = Identity::new(value).fmap(g).fmap(f);
    let composition_holds = composed_first.into_inner() == chained.into_inner();
    laws.push(LawDemonstration {
        law_name: "Functor Composition".to_string(),
        lhs: "Identity(x).fmap(|x| f(g(x)))".to_string(),
        rhs: "Identity(x).fmap(g).fmap(f)".to_string(),
        holds: composition_holds,
    });

    // Monad Left Identity: pure(a).flat_map(f) == f(a)
    let monad_f = |x: i32| Identity::new(x * 3);
    let left_identity_lhs = Identity::new(value).flat_map(monad_f);
    let left_identity_rhs = monad_f(value);
    let left_identity_holds = left_identity_lhs.into_inner() == left_identity_rhs.into_inner();
    laws.push(LawDemonstration {
        law_name: "Monad Left Identity".to_string(),
        lhs: "Identity::new(a).flat_map(f)".to_string(),
        rhs: "f(a)".to_string(),
        holds: left_identity_holds,
    });

    // Monad Right Identity: m.flat_map(pure) == m
    let right_identity_lhs = Identity::new(value).flat_map(Identity::new);
    let right_identity_rhs = Identity::new(value);
    let right_identity_holds = right_identity_lhs.into_inner() == right_identity_rhs.into_inner();
    laws.push(LawDemonstration {
        law_name: "Monad Right Identity".to_string(),
        lhs: "m.flat_map(Identity::new)".to_string(),
        rhs: "m".to_string(),
        holds: right_identity_holds,
    });

    // Flatten Law: Identity(Identity(x)).flatten() == Identity(x)
    let nested = Identity::new(Identity::new(value));
    let flattened: Identity<i32> = nested.flatten();
    let flatten_holds = flattened.into_inner() == value;
    laws.push(LawDemonstration {
        law_name: "Flatten".to_string(),
        lhs: "Identity(Identity(x)).flatten()".to_string(),
        rhs: "Identity(x)".to_string(),
        holds: flatten_holds,
    });

    laws
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lambars::effect::MonadErrorExt;
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // Identity Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_identity_fmap() {
        let id = Identity::new(10);
        let doubled: Identity<i32> = id.fmap(|x| x * 2);
        assert_eq!(doubled.into_inner(), 20);
    }

    #[rstest]
    fn test_identity_flat_map() {
        let id = Identity::new(5);
        let result = id.flat_map(|x| Identity::new(x + 10));
        assert_eq!(result.into_inner(), 15);
    }

    #[rstest]
    fn test_identity_flatten() {
        let nested = Identity::new(Identity::new(42));
        let flat: Identity<i32> = nested.flatten();
        assert_eq!(flat.into_inner(), 42);
    }

    #[rstest]
    fn test_identity_laws() {
        let laws = demonstrate_identity_laws(42);
        for law in laws {
            assert!(law.holds, "Law '{}' should hold", law.law_name);
        }
    }

    // -------------------------------------------------------------------------
    // FunctorMut Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_functor_mut_vec() {
        let values = vec![1, 2, 3];
        let doubled: Vec<i32> = values.fmap_mut(|x| x * 2);
        assert_eq!(doubled, vec![2, 4, 6]);
    }

    #[rstest]
    fn test_functor_mut_ref() {
        let values = vec![1, 2, 3];
        let doubled: Vec<i32> = values.fmap_ref_mut(|x| x * 2);
        assert_eq!(doubled, vec![2, 4, 6]);
    }

    // -------------------------------------------------------------------------
    // Flatten Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_flatten_option_some_some() {
        let nested: Option<Option<i32>> = Some(Some(42));
        let flat = nested.flatten();
        assert_eq!(flat, Some(42));
    }

    #[rstest]
    fn test_flatten_option_some_none() {
        let nested: Option<Option<i32>> = Some(None);
        let flat = nested.flatten();
        assert_eq!(flat, None);
    }

    #[rstest]
    fn test_flatten_option_none() {
        let nested: Option<Option<i32>> = None;
        let flat = nested.flatten();
        assert_eq!(flat, None);
    }

    #[rstest]
    fn test_flatten_result_ok_ok() {
        let nested: Result<Result<i32, String>, String> = Ok(Ok(42));
        let flat = nested.flatten();
        assert_eq!(flat, Ok(42));
    }

    #[rstest]
    fn test_flatten_result_ok_err() {
        let nested: Result<Result<i32, String>, String> = Ok(Err("inner".to_string()));
        let flat = nested.flatten();
        assert_eq!(flat, Err("inner".to_string()));
    }

    #[rstest]
    fn test_flatten_result_err() {
        let nested: Result<Result<i32, String>, String> = Err("outer".to_string());
        let flat = nested.flatten();
        assert_eq!(flat, Err("outer".to_string()));
    }

    // -------------------------------------------------------------------------
    // MonadError Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_monad_error_throw() {
        let thrown: Result<i32, String> = <Result<i32, String>>::throw_error("error".to_string());
        assert_eq!(thrown, Err("error".to_string()));
    }

    #[rstest]
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    fn test_monad_error_catch() {
        let failing: Result<i32, String> = Err("error".to_string());
        let recovered = <Result<i32, String>>::catch_error(failing, |e| Ok(e.len() as i32));
        assert_eq!(recovered, Ok(5));
    }

    #[rstest]
    fn test_monad_error_catch_success() {
        let success: Result<i32, String> = Ok(42);
        let result = <Result<i32, String>>::catch_error(success, |_| Ok(0));
        assert_eq!(result, Ok(42));
    }

    #[rstest]
    fn test_monad_error_ext_map_error() {
        let err: Result<i32, i32> = Err(404);
        let mapped: Result<i32, String> = err.map_error(|code| format!("Error {code}"));
        assert_eq!(mapped, Err("Error 404".to_string()));
    }

    // -------------------------------------------------------------------------
    // ReaderT Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_reader_t_basic() {
        let reader: ReaderT<i32, Option<i32>> = ReaderT::new(|env| Some(env * 2));
        assert_eq!(reader.run(21), Some(42));
    }

    #[rstest]
    fn test_reader_t_with_result() {
        let reader: ReaderT<i32, Result<i32, String>> = ReaderT::new(|env| Ok(env + 10));
        assert_eq!(reader.run(32), Ok(42));
    }

    // -------------------------------------------------------------------------
    // StateT Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_state_t_basic() {
        let state_t: StateT<i32, Option<(i32, i32)>> = StateT::new(|s| Some((s * 2, s + 1)));
        let result = state_t.run(10);
        assert_eq!(result, Some((20, 11)));
    }

    // -------------------------------------------------------------------------
    // WriterT Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_writer_t_basic() {
        let writer: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
            WriterT::new(Some((42, vec!["computed".to_string()])));
        let result = writer.run();
        assert_eq!(result, Some((42, vec!["computed".to_string()])));
    }

    // -------------------------------------------------------------------------
    // FunctorMut Handler Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[case("double", vec![1, 2, 3], vec![2, 4, 6])]
    #[case("square", vec![2, 3, 4], vec![4, 9, 16])]
    #[case("negate", vec![1, -2, 3], vec![-1, 2, -3])]
    #[case("increment", vec![0, 1, 2], vec![1, 2, 3])]
    #[case("abs", vec![-1, 2, -3], vec![1, 2, 3])]
    fn test_functor_mut_transformations(
        #[case] transformation: &str,
        #[case] input: Vec<i32>,
        #[case] expected: Vec<i32>,
    ) {
        let transformed: Vec<i32> = match transformation {
            "double" => input.fmap_mut(|x| x * 2),
            "square" => input.fmap_mut(|x| x * x),
            "negate" => input.fmap_mut(|x| -x),
            "increment" => input.fmap_mut(|x| x + 1),
            "abs" => input.fmap_mut(i32::abs),
            _ => panic!("Unknown transformation"),
        };
        assert_eq!(transformed, expected);
    }

    // -------------------------------------------------------------------------
    // MonadError Handler Logic Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_monad_error_accumulation_with_recovery() {
        // Simulate the accumulation logic with recovery enabled
        let operations: Vec<Result<i32, String>> = vec![Ok(10), Err("error".to_string()), Ok(20)];
        let recovery_value = 5;

        let mut sum: Result<i32, String> = Ok(0);
        for op in operations {
            let processed = <Result<i32, String>>::catch_error(op, move |_| Ok(recovery_value));
            sum = sum.flat_map(|acc| processed.fmap(move |v| acc + v));
        }

        // 10 + 5 (recovered) + 20 = 35
        assert_eq!(sum, Ok(35));
    }

    #[rstest]
    fn test_monad_error_accumulation_without_recovery() {
        // Simulate the accumulation logic without recovery
        let operations: Vec<Result<i32, String>> = vec![Ok(10), Err("error".to_string()), Ok(20)];

        let mut sum: Result<i32, String> = Ok(0);
        for op in operations {
            let op_clone = op.clone();
            sum = sum.flat_map(move |acc| op_clone.fmap(move |v| acc + v));
        }

        // First error short-circuits
        assert!(sum.is_err());
    }

    #[rstest]
    fn test_monad_error_all_success() {
        let operations: Vec<Result<i32, String>> = vec![Ok(10), Ok(20), Ok(30)];

        let mut sum: Result<i32, String> = Ok(0);
        for op in operations {
            sum = sum.flat_map(|acc| op.fmap(move |v| acc + v));
        }

        assert_eq!(sum, Ok(60));
    }

    // -------------------------------------------------------------------------
    // Identity Handler Logic Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_identity_chain_transformations() {
        let mut wrapped = Identity::new(10);
        wrapped = wrapped.fmap(|x| x * 2); // 20
        wrapped = wrapped.fmap(|x| x + 1); // 21
        wrapped = wrapped.fmap(|x| x * x); // 441
        assert_eq!(wrapped.into_inner(), 441);
    }

    #[rstest]
    fn test_identity_monad_laws_hold() {
        let value = 100;
        let laws = demonstrate_identity_laws(value);

        // All laws should hold
        for law in &laws {
            assert!(law.holds, "Law '{}' failed", law.law_name);
        }
        // Should have 5 laws tested
        assert_eq!(laws.len(), 5);
    }
}
