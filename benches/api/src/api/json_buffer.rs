//! JSON buffer pool for efficient serialization.
//!
//! This module provides a thread-local buffer pool for JSON serialization,
//! eliminating intermediate `String` allocations by using `serde_json::to_writer`
//! directly into a reusable `BytesMut` buffer.
//!
//! # Design Principles
//!
//! - **Zero-copy**: Uses `split().freeze()` to convert `BytesMut` to `Bytes` without copying.
//! - **Thread-local isolation**: Each thread has its own buffer, eliminating contention.
//! - **Bounded memory**: Buffers exceeding 1MB are discarded and replaced with fresh 64KB buffers.
//! - **Re-entrancy safe**: Detects nested calls and falls back to temporary buffers.
//!
//! # Usage
//!
//! ```ignore
//! use crate::api::json_buffer::serialize_json_bytes;
//!
//! let data = MySerializableType { ... };
//! let bytes = serialize_json_bytes(&data)?;
//! ```

use axum::{
    body::Body,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use bytes::{BufMut, Bytes, BytesMut};
use serde::Serialize;
use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};

use super::error::ApiErrorResponse;

/// Maximum buffer capacity before discarding (1 MB).
///
/// Note: This constant is used internally by `with_buffer` and tests.
/// The `#[allow(dead_code)]` is temporary until later phases integrate this module.
#[allow(dead_code)]
const MAX_BUFFER_CAPACITY: usize = 1024 * 1024;

/// Initial buffer capacity (64 KB).
///
/// Note: This constant is used by `thread_local!` initialization and tests.
/// The `#[allow(dead_code)]` is temporary until later phases integrate this module.
#[allow(dead_code)]
const INITIAL_CAPACITY: usize = 64 * 1024;

/// Counter for fallback occurrences (for monitoring/debugging).
///
/// Note: This static is used internally by `with_buffer`.
/// The `#[allow(dead_code)]` is temporary until later phases integrate this module.
#[allow(dead_code)]
static FALLBACK_COUNT: AtomicU64 = AtomicU64::new(0);

thread_local! {
    /// Thread-local JSON serialization buffer.
    static JSON_BUFFER: RefCell<BytesMut> = RefCell::new(BytesMut::with_capacity(INITIAL_CAPACITY));
}

/// Guard that ensures buffer cleanup on drop (including panic scenarios).
///
/// This guard always performs cleanup when dropped, ensuring proper
/// resource management even if a panic occurs during buffer operations.
///
/// The `capacity_before_operation` field stores the buffer capacity measured
/// before the closure executes. This is necessary because operations like
/// `split()` can reduce the buffer's capacity, making post-operation capacity
/// checks unreliable.
///
/// Note: The `#[allow(dead_code)]` is temporary until later phases integrate this module.
#[allow(dead_code)]
struct BufferGuard<'a> {
    buffer: &'a mut BytesMut,
    /// Capacity recorded before closure execution for accurate overflow detection.
    capacity_before_operation: usize,
}

impl Drop for BufferGuard<'_> {
    fn drop(&mut self) {
        // Always perform cleanup on drop (normal exit or panic).
        // Check both the recorded capacity (for split() scenarios) and current capacity
        // (for panic scenarios where data was written but split() never called).
        let max_observed_capacity = self.capacity_before_operation.max(self.buffer.capacity());
        if max_observed_capacity > MAX_BUFFER_CAPACITY {
            *self.buffer = BytesMut::with_capacity(INITIAL_CAPACITY);
        } else {
            self.buffer.clear();
        }
    }
}

/// JSON buffer pool for efficient serialization.
///
/// This struct provides a namespace for buffer pool operations.
/// All methods are static and operate on thread-local storage.
///
/// Note: The `#[allow(dead_code)]` is temporary until later phases integrate this module.
#[allow(dead_code)]
pub struct JsonBufferPool;

impl JsonBufferPool {
    /// Executes a closure with access to the thread-local buffer.
    ///
    /// The buffer is guaranteed to be empty at the start of the closure.
    /// After the closure completes (normally or via panic), the buffer
    /// is cleared or replaced if it exceeds the maximum capacity.
    ///
    /// # Re-entrancy
    ///
    /// If called recursively (nested call), this function detects the
    /// re-entrancy and creates a temporary buffer for the nested call,
    /// ensuring the operation completes successfully (though without
    /// the benefit of buffer reuse).
    ///
    /// # Arguments
    ///
    /// * `f` - Closure that receives a mutable reference to the buffer.
    ///
    /// # Returns
    ///
    /// The return value of the closure.
    pub fn with_buffer<F, R>(f: F) -> R
    where
        F: FnOnce(&mut BytesMut) -> R,
    {
        JSON_BUFFER.with(|cell| {
            if let Ok(mut buffer) = cell.try_borrow_mut() {
                // Clear any residual data from previous use
                buffer.clear();

                // Record capacity before closure execution for accurate overflow detection.
                // This is necessary because operations like split() can reduce the buffer's
                // current capacity, making post-operation checks unreliable.
                let capacity_before = buffer.capacity();

                // Create guard for panic safety - it will handle cleanup on drop
                let guard = BufferGuard {
                    buffer: &mut buffer,
                    capacity_before_operation: capacity_before,
                };

                // Execute the closure using the guard's buffer reference
                let result = f(guard.buffer);

                // Normal exit: record capacity after operation for overflow check
                let capacity_after = guard.capacity_before_operation.max(guard.buffer.capacity());

                // Forget the guard to prevent double cleanup
                std::mem::forget(guard);

                // Perform manual cleanup with the correct capacity value
                if capacity_after > MAX_BUFFER_CAPACITY {
                    *buffer = BytesMut::with_capacity(INITIAL_CAPACITY);
                } else {
                    buffer.clear();
                }

                result
            } else {
                // Re-entrancy detected: use fallback path
                let count = FALLBACK_COUNT.fetch_add(1, Ordering::Relaxed);
                if count == 0 {
                    tracing::warn!(
                        "JsonBufferPool: nested borrow detected (further occurrences will be counted only)"
                    );
                }

                // Create temporary buffer for this nested call
                let mut temp_buffer = BytesMut::with_capacity(INITIAL_CAPACITY);
                f(&mut temp_buffer)
            }
        })
    }

    /// Returns the current fallback count for monitoring/debugging.
    ///
    /// This counter tracks how many times the fallback path was taken
    /// due to re-entrancy detection.
    #[cfg(test)]
    pub fn fallback_count() -> u64 {
        FALLBACK_COUNT.load(Ordering::Relaxed)
    }
}

/// Serializes a value to JSON bytes using the buffer pool.
///
/// This function uses `serde_json::to_writer` to serialize directly into
/// a reusable buffer, avoiding intermediate `String` allocations.
///
/// # Arguments
///
/// * `value` - The value to serialize (must implement `Serialize`).
///
/// # Returns
///
/// * `Ok(Bytes)` - The serialized JSON as immutable bytes.
/// * `Err(ApiErrorResponse)` - If serialization fails.
///
/// # Performance
///
/// - Uses thread-local buffer reuse to minimize allocations.
/// - Zero-copy conversion from `BytesMut` to `Bytes` via `split().freeze()`.
/// - Falls back to temporary buffer on re-entrancy (rare).
///
/// # Example
///
/// ```ignore
/// #[derive(Serialize)]
/// struct MyData { value: i32 }
///
/// let data = MyData { value: 42 };
/// let bytes = serialize_json_bytes(&data)?;
/// assert_eq!(&bytes[..], br#"{"value":42}"#);
/// ```
///
/// Note: The `#[allow(dead_code)]` is temporary until later phases integrate this module.
#[allow(dead_code)]
pub fn serialize_json_bytes<T: Serialize>(value: &T) -> Result<Bytes, ApiErrorResponse> {
    JsonBufferPool::with_buffer(|buffer| {
        // Use BufMut::writer() to get an impl Write adapter
        let mut writer = buffer.writer();
        serde_json::to_writer(&mut writer, value).map_err(|e| {
            // Log the detailed error for debugging, but return a generic message to clients
            tracing::error!("JSON serialization failed: {e}");
            ApiErrorResponse::internal_error("JSON serialization failed")
        })?;

        // Check if buffer exceeded max capacity BEFORE split()
        // (split() reduces capacity, so we must check beforehand)
        let exceeded = buffer.capacity() > MAX_BUFFER_CAPACITY;

        // Zero-copy conversion: split() detaches the written data, freeze() makes it immutable
        let bytes = buffer.split().freeze();

        // Reset buffer if it exceeded max capacity
        // Note: This works because split() leaves the original BytesMut empty
        // and we need to reset its internal allocation
        if exceeded {
            *buffer = BytesMut::with_capacity(INITIAL_CAPACITY);
        }

        Ok(bytes)
    })
}

// =============================================================================
// JsonResponse
// =============================================================================

/// JSON response wrapper that uses buffer pool for efficient serialization.
///
/// This is a drop-in replacement for `axum::Json<T>` that uses
/// `serialize_json_bytes` for efficient serialization.
///
/// # Example
///
/// ```ignore
/// use crate::api::json_buffer::JsonResponse;
///
/// async fn handler() -> JsonResponse<MyData> {
///     JsonResponse(MyData { value: 42 })
/// }
/// ```
///
/// # Performance
///
/// Unlike `axum::Json<T>` which uses `serde_json::to_string()`, this type
/// uses `serde_json::to_writer()` with a reusable buffer pool, avoiding
/// intermediate `String` allocations.
#[allow(dead_code)]
#[derive(Debug)]
pub struct JsonResponse<T>(pub T);

impl<T: Serialize> IntoResponse for JsonResponse<T> {
    fn into_response(self) -> Response {
        match serialize_json_bytes(&self.0) {
            Ok(bytes) => {
                let mut headers = HeaderMap::new();
                headers.insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                );
                (StatusCode::OK, headers, Body::from(bytes)).into_response()
            }
            Err(error_response) => {
                // Delegate to ApiErrorResponse's IntoResponse implementation
                error_response.into_response()
            }
        }
    }
}

/// JSON response with custom status code.
///
/// This struct allows specifying a custom HTTP status code for the response.
///
/// # Example
///
/// ```ignore
/// use axum::http::StatusCode;
/// use crate::api::json_buffer::JsonResponseWithStatus;
///
/// async fn create_handler() -> JsonResponseWithStatus<MyData> {
///     JsonResponseWithStatus::new(StatusCode::CREATED, MyData { value: 42 })
/// }
/// ```
#[allow(dead_code)]
pub struct JsonResponseWithStatus<T> {
    status: StatusCode,
    body: T,
}

impl<T> JsonResponseWithStatus<T> {
    /// Creates a new `JsonResponseWithStatus` with the given status code and body.
    #[allow(dead_code)]
    pub const fn new(status: StatusCode, body: T) -> Self {
        Self { status, body }
    }
}

impl<T: Serialize> IntoResponse for JsonResponseWithStatus<T> {
    fn into_response(self) -> Response {
        match serialize_json_bytes(&self.body) {
            Ok(bytes) => {
                let mut headers = HeaderMap::new();
                headers.insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                );
                (self.status, headers, Body::from(bytes)).into_response()
            }
            Err(error_response) => error_response.into_response(),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use serde::Deserialize;

    /// Test data structure for serialization tests.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        content: String,
    }

    impl TestData {
        fn new(content: impl Into<String>) -> Self {
            Self {
                content: content.into(),
            }
        }
    }

    // -------------------------------------------------------------------------
    // with_buffer basic tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_with_buffer_basic_operation() {
        let result = JsonBufferPool::with_buffer(|buffer| {
            buffer.extend_from_slice(b"hello");
            buffer.len()
        });
        assert_eq!(result, 5);
    }

    #[rstest]
    fn test_with_buffer_buffer_is_empty_at_start() {
        // First call to populate the buffer
        JsonBufferPool::with_buffer(|buffer| {
            buffer.extend_from_slice(b"some data");
        });

        // Second call should see empty buffer
        let is_empty = JsonBufferPool::with_buffer(|buffer| buffer.is_empty());
        assert!(is_empty, "Buffer should be empty at start of with_buffer");
    }

    #[rstest]
    fn test_with_buffer_no_residual_data_contamination() {
        // Run 100 iterations to ensure no residual data leaks between calls
        for i in 0..100 {
            let content = format!("iteration_{i}");
            let result = JsonBufferPool::with_buffer(|buffer| {
                // Check buffer is empty at start
                assert!(buffer.is_empty(), "Buffer should be empty at iteration {i}");

                // Write unique content
                buffer.extend_from_slice(content.as_bytes());
                buffer_to_bytes(buffer)
            });

            assert_eq!(result.as_ref(), content.as_bytes());
        }
    }

    #[rstest]
    fn test_with_buffer_large_data_buffer_reset() {
        // Write more than MAX_BUFFER_CAPACITY (1MB)
        let large_data = vec![b'X'; MAX_BUFFER_CAPACITY + 1];

        JsonBufferPool::with_buffer(|buffer| {
            buffer.extend_from_slice(&large_data);
            assert!(buffer.capacity() > MAX_BUFFER_CAPACITY);
        });

        // Next call should have a fresh buffer with initial capacity
        let capacity = JsonBufferPool::with_buffer(|buffer| {
            assert!(buffer.is_empty());
            buffer.capacity()
        });

        assert!(
            capacity <= INITIAL_CAPACITY,
            "Buffer should be reset to initial capacity after exceeding max. Got: {capacity}"
        );
    }

    // -------------------------------------------------------------------------
    // serialize_json_bytes tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_serialize_json_bytes_basic() {
        let data = TestData::new("hello");
        let bytes = serialize_json_bytes(&data).expect("Serialization should succeed");

        let expected: serde_json::Value =
            serde_json::from_slice(&bytes).expect("Should be valid JSON");
        assert_eq!(expected["content"], "hello");
    }

    #[rstest]
    fn test_serialize_json_bytes_output_matches_reference() {
        let data = TestData::new("test content");
        let our_bytes = serialize_json_bytes(&data).expect("Serialization should succeed");

        // Reference implementation using to_writer + BytesMut
        let reference = reference_serialize(&data);

        assert_eq!(
            our_bytes.as_ref(),
            reference.as_ref(),
            "Output should match reference serialization"
        );
    }

    #[rstest]
    fn test_serialize_json_bytes_empty_struct() {
        #[derive(Serialize)]
        struct Empty {}

        let data = Empty {};
        let bytes = serialize_json_bytes(&data).expect("Serialization should succeed");
        assert_eq!(&bytes[..], b"{}");
    }

    #[rstest]
    fn test_serialize_json_bytes_nested_structure() {
        #[derive(Serialize)]
        struct Nested {
            name: String,
            inner: Inner,
        }

        #[derive(Serialize)]
        struct Inner {
            value: i32,
        }

        let data = Nested {
            name: "outer".to_string(),
            inner: Inner { value: 42 },
        };

        let bytes = serialize_json_bytes(&data).expect("Serialization should succeed");
        let reference = reference_serialize(&data);
        assert_eq!(bytes.as_ref(), reference.as_ref());
    }

    #[rstest]
    fn test_serialize_json_bytes_array() {
        let data = vec![1, 2, 3, 4, 5];
        let bytes = serialize_json_bytes(&data).expect("Serialization should succeed");
        assert_eq!(&bytes[..], b"[1,2,3,4,5]");
    }

    #[rstest]
    fn test_serialize_json_bytes_unicode() {
        let data = TestData::new("Unicode \u{1F600} test \u{00E9}\u{00F1}");
        let bytes = serialize_json_bytes(&data).expect("Serialization should succeed");
        let reference = reference_serialize(&data);
        assert_eq!(bytes.as_ref(), reference.as_ref());
    }

    #[rstest]
    fn test_serialize_json_bytes_special_characters() {
        let data = TestData::new("line1\nline2\ttab\"quoted\"");
        let bytes = serialize_json_bytes(&data).expect("Serialization should succeed");
        let reference = reference_serialize(&data);
        assert_eq!(bytes.as_ref(), reference.as_ref());
    }

    #[rstest]
    fn test_serialize_json_bytes_no_residual_data() {
        // First: serialize large data
        let large_data = TestData::new("A".repeat(100_000));
        let _ = serialize_json_bytes(&large_data).expect("Large serialization should succeed");

        // Second: serialize small data and verify no contamination
        let small_data = TestData::new("B");
        let bytes = serialize_json_bytes(&small_data).expect("Small serialization should succeed");
        let json = std::str::from_utf8(&bytes).expect("Should be valid UTF-8");

        assert!(
            !json.contains("AAA"),
            "Previous data should not contaminate current output"
        );
        assert!(json.contains("\"B\""), "Current data should be present");
    }

    #[rstest]
    fn test_serialize_json_bytes_consecutive_calls() {
        // Test 100 consecutive serializations
        for i in 0..100 {
            let data = TestData::new(format!("item_{i}"));
            let bytes = serialize_json_bytes(&data).expect("Serialization should succeed");
            let reference = reference_serialize(&data);
            assert_eq!(
                bytes.as_ref(),
                reference.as_ref(),
                "Mismatch at iteration {i}"
            );
        }
    }

    #[rstest]
    fn test_serialize_large_data_resets_buffer() {
        // Serialize data larger than MAX_BUFFER_CAPACITY
        let large_content = "X".repeat(MAX_BUFFER_CAPACITY + 1000);
        let data = TestData::new(large_content);
        let _ = serialize_json_bytes(&data).expect("Large serialization should succeed");

        // Next call should have a fresh buffer with initial capacity
        let capacity = JsonBufferPool::with_buffer(|buffer| {
            assert!(buffer.is_empty());
            buffer.capacity()
        });

        assert!(
            capacity <= INITIAL_CAPACITY,
            "Buffer should be reset after large serialization. Got: {capacity}"
        );
    }

    // -------------------------------------------------------------------------
    // Fallback path tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_fallback_path_on_reentry() {
        // Get initial count (don't reset - may be affected by parallel tests)
        let initial_count = JsonBufferPool::fallback_count();

        // Simulate re-entrancy by calling with_buffer from within with_buffer
        let (outer_result, inner_result) = JsonBufferPool::with_buffer(|outer_buffer| {
            outer_buffer.extend_from_slice(b"outer");

            // Nested call should trigger fallback
            let inner = JsonBufferPool::with_buffer(|inner_buffer| {
                inner_buffer.extend_from_slice(b"inner");
                buffer_to_bytes(inner_buffer)
            });

            (buffer_to_bytes(outer_buffer), inner)
        });

        // Both should have correct data
        assert_eq!(outer_result.as_ref(), b"outer");
        assert_eq!(inner_result.as_ref(), b"inner");

        // Verify fallback was triggered (count increased)
        let final_count = JsonBufferPool::fallback_count();
        assert!(
            final_count > initial_count,
            "Fallback count should increase on re-entry. Initial: {initial_count}, Final: {final_count}"
        );
    }

    #[rstest]
    fn test_serialize_json_bytes_reentry() {
        // Get initial count (don't reset - may be affected by parallel tests)
        let initial_count = JsonBufferPool::fallback_count();

        // Nested serialization via with_buffer
        let result = JsonBufferPool::with_buffer(|_outer| {
            // This nested call should use fallback
            let data = TestData::new("nested");
            serialize_json_bytes(&data)
        });

        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(std::str::from_utf8(&bytes).unwrap().contains("\"nested\""));

        // Fallback should have been used (count increased)
        let final_count = JsonBufferPool::fallback_count();
        assert!(
            final_count > initial_count,
            "Fallback should be triggered for nested serialize_json_bytes. Initial: {initial_count}, Final: {final_count}"
        );
    }

    // -------------------------------------------------------------------------
    // Panic and error handling tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_panic_cleanup() {
        use std::panic::catch_unwind;

        // Trigger a panic inside with_buffer after writing data exceeding MAX_BUFFER_CAPACITY
        let result = catch_unwind(|| {
            JsonBufferPool::with_buffer(|buffer| {
                buffer.extend_from_slice(&vec![b'X'; MAX_BUFFER_CAPACITY + 1]);
                panic!("intentional panic for testing cleanup");
            });
        });

        // Verify the panic was caught
        assert!(result.is_err(), "Panic should have been caught");

        // Verify buffer was reset after the panic (BufferGuard should have cleaned up)
        let capacity = JsonBufferPool::with_buffer(|buffer| {
            assert!(
                buffer.is_empty(),
                "Buffer should be empty after panic cleanup"
            );
            buffer.capacity()
        });

        assert!(
            capacity <= INITIAL_CAPACITY,
            "Buffer should be reset to initial capacity after panic. Got: {capacity}"
        );
    }

    #[rstest]
    fn test_serialize_error() {
        /// A type that always fails to serialize.
        struct FailingSerializer;

        impl Serialize for FailingSerializer {
            fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                Err(serde::ser::Error::custom(
                    "intentional serialization failure",
                ))
            }
        }

        let result = serialize_json_bytes(&FailingSerializer);
        assert!(
            result.is_err(),
            "Serialization should fail for FailingSerializer"
        );

        // Verify the error message does not expose internal details
        let error = result.unwrap_err();
        assert_eq!(
            error.error.message, "JSON serialization failed",
            "Error message should be generic, not exposing internal details"
        );
    }

    // -------------------------------------------------------------------------
    // Helper functions
    // -------------------------------------------------------------------------

    /// Reference implementation using `to_writer` + `BytesMut` for testing.
    /// Returns `Bytes` to avoid `to_vec()` usage entirely.
    fn reference_serialize<T: Serialize>(value: &T) -> Bytes {
        let mut buffer = BytesMut::new();
        serde_json::to_writer((&mut buffer).writer(), value).unwrap();
        buffer.freeze()
    }

    /// Helper to extract buffer contents as `Bytes` for testing.
    /// Uses `split().freeze()` to avoid `to_vec()`.
    fn buffer_to_bytes(buffer: &mut BytesMut) -> Bytes {
        buffer.split().freeze()
    }

    // -------------------------------------------------------------------------
    // JsonResponse tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_json_response_basic() {
        use axum::http::header::CONTENT_TYPE;

        let data = TestData::new("hello");
        let response = JsonResponse(data).into_response();

        // Verify Content-Type header
        let content_type = response.headers().get(CONTENT_TYPE);
        assert!(content_type.is_some(), "Content-Type header should be set");
        assert_eq!(
            content_type.unwrap(),
            "application/json",
            "Content-Type should be application/json"
        );
    }

    #[rstest]
    fn test_json_response_status_code_default() {
        let data = TestData::new("test");
        let response = JsonResponse(data).into_response();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Default status code should be 200 OK"
        );
    }

    #[rstest]
    fn test_json_response_with_status_code() {
        let data = TestData::new("created");
        let response = JsonResponseWithStatus::new(StatusCode::CREATED, data).into_response();

        assert_eq!(
            response.status(),
            StatusCode::CREATED,
            "Status code should be 201 CREATED"
        );

        // Verify Content-Type header is still set
        let content_type = response.headers().get(header::CONTENT_TYPE);
        assert!(content_type.is_some(), "Content-Type header should be set");
        assert_eq!(content_type.unwrap(), "application/json");
    }

    #[rstest]
    #[tokio::test]
    async fn test_json_response_body_matches_reference() {
        use http_body_util::BodyExt;

        let data = TestData::new("test content");
        let response = JsonResponse(data.clone()).into_response();

        // Extract body bytes
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .expect("Failed to collect body")
            .to_bytes();

        // Reference implementation
        let reference = reference_serialize(&data);

        assert_eq!(
            body_bytes.as_ref(),
            reference.as_ref(),
            "Response body should match reference serialization"
        );
    }

    #[rstest]
    fn test_json_response_error_handling() {
        /// A type that always fails to serialize.
        struct FailingSerializer;

        impl Serialize for FailingSerializer {
            fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                Err(serde::ser::Error::custom(
                    "intentional serialization failure",
                ))
            }
        }

        let response = JsonResponse(FailingSerializer).into_response();

        // Should return an error response (500 Internal Server Error)
        assert_eq!(
            response.status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "Serialization failure should result in 500 Internal Server Error"
        );

        // Content-Type should still be application/json
        let content_type = response.headers().get(header::CONTENT_TYPE);
        assert!(content_type.is_some(), "Content-Type header should be set");
        assert_eq!(content_type.unwrap(), "application/json");
    }

    #[rstest]
    fn test_json_response_with_status_code_error_handling() {
        /// A type that always fails to serialize.
        struct FailingSerializer;

        impl Serialize for FailingSerializer {
            fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                Err(serde::ser::Error::custom(
                    "intentional serialization failure",
                ))
            }
        }

        // Even with a custom status code, serialization failure should return error
        let response =
            JsonResponseWithStatus::new(StatusCode::CREATED, FailingSerializer).into_response();

        // Should return an error response (500 Internal Server Error from ApiErrorResponse)
        assert_eq!(
            response.status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "Serialization failure should result in 500 Internal Server Error"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_json_response_complex_structure() {
        use http_body_util::BodyExt;

        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
        struct ComplexData {
            id: u64,
            name: String,
            tags: Vec<String>,
            metadata: Option<Metadata>,
        }

        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
        struct Metadata {
            created_at: String,
            updated_at: String,
        }

        let data = ComplexData {
            id: 123,
            name: "Test Item".to_string(),
            tags: vec!["tag1".to_string(), "tag2".to_string()],
            metadata: Some(Metadata {
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-02T00:00:00Z".to_string(),
            }),
        };

        let response = JsonResponse(data.clone()).into_response();

        // Verify status and headers
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );

        // Verify body
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .expect("Failed to collect body")
            .to_bytes();

        let reference = reference_serialize(&data);
        assert_eq!(body_bytes.as_ref(), reference.as_ref());
    }
}
