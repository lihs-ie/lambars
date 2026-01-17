//! Custom assertion helpers for integration tests.

use super::client::{ApiError, ApiResult};
use reqwest::StatusCode;

pub fn assert_api_error<T: std::fmt::Debug>(
    result: &ApiResult<T>,
    expected_code: &str,
    expected_status: StatusCode,
) {
    match result {
        Err(ApiError::Api { status, code, .. }) => {
            assert_eq!(
                *status, expected_status,
                "Expected status {expected_status}, got {status}"
            );
            assert_eq!(
                code, expected_code,
                "Expected error code '{expected_code}', got '{code}'"
            );
        }
        Err(ApiError::Http(e)) => {
            panic!("Expected API error '{expected_code}', got HTTP error: {e:?}");
        }
        Ok(v) => {
            panic!("Expected API error '{expected_code}', got success: {v:?}");
        }
    }
}

pub fn assert_success<T: std::fmt::Debug>(result: &ApiResult<T>) {
    assert!(result.is_ok(), "Expected success, got error: {result:?}");
}
