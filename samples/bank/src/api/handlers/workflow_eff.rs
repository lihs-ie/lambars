//! ExceptT + eff_async! workflow utilities.
//!
//! This module provides utilities for composing async workflows using
//! the ExceptT monad transformer with eff_async! macro for do-notation style.
//!
//! # Design
//!
//! The key insight is that `ExceptT<E, AsyncIO<Result<A, E>>>` allows us to:
//! - Chain async operations declaratively using eff_async!
//! - Automatically propagate errors through the computation
//! - Maintain referential transparency
//!
//! # Example
//!
//! ```rust,ignore
//! use lambars::eff_async;
//! use bank::api::handlers::workflow_eff::*;
//!
//! let workflow = eff_async! {
//!     events <= load_events_eff(&event_store, &account_id);
//!     account <= from_result_eff(Account::from_events(&events).ok_or(not_found_error()));
//!     event <= from_result_eff(deposit(&command, &account, timestamp));
//!     _ <= persist_event_eff(&event_store, &account_id, version, event.clone());
//!     WorkflowResult::pure_async_io(event)
//! };
//!
//! let result = workflow.run_async_io().run_async().await;
//! ```

use lambars::effect::{AsyncIO, ExceptT};

use crate::api::middleware::error_handler::ApiErrorResponse;

/// Type alias for async workflow result with ExceptT.
///
/// This type represents an async computation that can fail with `ApiErrorResponse`.
/// It is compatible with the `eff_async!` macro for do-notation style composition.
pub type WorkflowResult<A> = ExceptT<ApiErrorResponse, AsyncIO<Result<A, ApiErrorResponse>>>;

/// Lifts a pure value into `WorkflowResult`.
///
/// # Examples
///
/// ```rust,ignore
/// let result: WorkflowResult<i32> = pure_async(42);
/// ```
#[inline]
pub fn pure_async<A>(value: A) -> WorkflowResult<A>
where
    A: Send + 'static,
{
    WorkflowResult::pure_async_io(value)
}

/// Lifts an error into `WorkflowResult`.
///
/// # Examples
///
/// ```rust,ignore
/// let result: WorkflowResult<i32> = throw_async(api_error);
/// ```
#[inline]
pub fn throw_async<A>(error: ApiErrorResponse) -> WorkflowResult<A>
where
    A: Send + 'static,
{
    WorkflowResult::throw_async_io(error)
}

/// Lifts a `Result` into `WorkflowResult`.
///
/// # Examples
///
/// ```rust,ignore
/// let result: WorkflowResult<i32> = from_result(Ok(42));
/// let error: WorkflowResult<i32> = from_result(Err(api_error));
/// ```
#[inline]
pub fn from_result<A>(result: Result<A, ApiErrorResponse>) -> WorkflowResult<A>
where
    A: Send + 'static,
{
    WorkflowResult::from_result(result)
}

/// Lifts an `AsyncIO` into `WorkflowResult`.
///
/// This is useful for wrapping existing AsyncIO operations.
///
/// # Examples
///
/// ```rust,ignore
/// let async_io: AsyncIO<i32> = AsyncIO::pure(42);
/// let result: WorkflowResult<i32> = lift_async_io(async_io);
/// ```
#[inline]
pub fn lift_async_io<A>(async_io: AsyncIO<A>) -> WorkflowResult<A>
where
    A: Send + 'static,
{
    WorkflowResult::lift_async_io(async_io)
}

/// Lifts an `AsyncIO<Result<A, E>>` into `WorkflowResult` with error mapping.
///
/// This is the most common use case for wrapping AsyncIO operations that
/// return Result.
///
/// # Examples
///
/// ```rust,ignore
/// let events = lift_async_result(
///     event_store.load_events(&account_id),
///     |e| event_store_error_response(&e)
/// );
/// ```
#[inline]
pub fn lift_async_result<A, E, F>(
    async_io: AsyncIO<Result<A, E>>,
    map_error: F,
) -> WorkflowResult<A>
where
    A: Send + 'static,
    E: Send + 'static,
    F: FnOnce(E) -> ApiErrorResponse + Send + 'static,
{
    WorkflowResult::new(async_io.fmap(move |result| result.map_err(map_error)))
}

/// Converts an `Option` into `WorkflowResult` with a default error.
///
/// # Examples
///
/// ```rust,ignore
/// let account = from_option(
///     Account::from_events(&events),
///     || account_not_found_response(&account_id_string)
/// );
/// ```
#[inline]
pub fn from_option<A, F>(option: Option<A>, error_fn: F) -> WorkflowResult<A>
where
    A: Send + 'static,
    F: FnOnce() -> ApiErrorResponse,
{
    match option {
        Some(value) => pure_async(value),
        None => throw_async(error_fn()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use crate::api::middleware::error_handler::ApiError;
    use rstest::rstest;

    fn test_error() -> ApiErrorResponse {
        ApiErrorResponse::new(
            StatusCode::BAD_REQUEST,
            ApiError::new("TEST_ERROR", "Test error"),
        )
    }

    #[tokio::test]
    async fn pure_async_returns_value() {
        let result = pure_async(42);
        assert_eq!(result.run_async_io().run_async().await.unwrap(), 42);
    }

    #[tokio::test]
    async fn throw_async_returns_error() {
        let result: WorkflowResult<i32> = throw_async(test_error());
        let err = result.run_async_io().run_async().await.unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn from_result_ok_returns_value() {
        let result = from_result(Ok(42));
        assert_eq!(result.run_async_io().run_async().await.unwrap(), 42);
    }

    #[tokio::test]
    async fn from_result_err_returns_error() {
        let result: WorkflowResult<i32> = from_result(Err(test_error()));
        let err = result.run_async_io().run_async().await.unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn lift_async_io_wraps_async_io() {
        let async_io = AsyncIO::pure(42);
        let result = lift_async_io(async_io);
        assert_eq!(result.run_async_io().run_async().await.unwrap(), 42);
    }

    #[tokio::test]
    async fn lift_async_result_ok_returns_value() {
        let async_io = AsyncIO::pure(Ok::<i32, &str>(42));
        let result = lift_async_result(async_io, |e: &str| {
            ApiErrorResponse::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError::new("ERROR", e),
            )
        });
        assert_eq!(result.run_async_io().run_async().await.unwrap(), 42);
    }

    #[tokio::test]
    async fn lift_async_result_err_returns_mapped_error() {
        let async_io = AsyncIO::pure(Err::<i32, &str>("test error"));
        let result = lift_async_result(async_io, |e: &str| {
            ApiErrorResponse::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError::new("ERROR", e),
            )
        });
        let err = result.run_async_io().run_async().await.unwrap_err();
        assert_eq!(err.status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn from_option_some_returns_value() {
        let result = from_option(Some(42), test_error);
        assert_eq!(result.run_async_io().run_async().await.unwrap(), 42);
    }

    #[tokio::test]
    async fn from_option_none_returns_error() {
        let result: WorkflowResult<i32> = from_option(None, test_error);
        let err = result.run_async_io().run_async().await.unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
    }

    #[rstest]
    #[tokio::test]
    async fn eff_async_macro_chains_operations() {
        let result = lambars::eff_async! {
            x <= pure_async(5);
            y <= pure_async(10);
            pure_async(x + y)
        };
        assert_eq!(result.run_async_io().run_async().await.unwrap(), 15);
    }

    #[rstest]
    #[tokio::test]
    async fn eff_async_macro_short_circuits_on_error() {
        let result: WorkflowResult<i32> = lambars::eff_async! {
            x <= pure_async(5);
            _ <= throw_async::<i32>(test_error());
            pure_async(x * 2)
        };
        let err = result.run_async_io().run_async().await.unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
    }

    #[rstest]
    #[tokio::test]
    async fn eff_async_macro_with_from_result() {
        let result = lambars::eff_async! {
            x <= from_result(Ok(10));
            y <= from_result(Ok(20));
            pure_async(x + y)
        };
        assert_eq!(result.run_async_io().run_async().await.unwrap(), 30);
    }

    #[rstest]
    #[tokio::test]
    async fn eff_async_macro_with_from_option() {
        let result = lambars::eff_async! {
            x <= from_option(Some(10), test_error);
            y <= from_option(Some(20), test_error);
            pure_async(x + y)
        };
        assert_eq!(result.run_async_io().run_async().await.unwrap(), 30);
    }

    #[rstest]
    #[tokio::test]
    async fn eff_async_macro_from_option_none_short_circuits() {
        let result: WorkflowResult<i32> = lambars::eff_async! {
            x <= from_option(Some(10), test_error);
            _ <= from_option::<i32, _>(None, test_error);
            pure_async(x * 2)
        };
        let err = result.run_async_io().run_async().await.unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
    }
}
