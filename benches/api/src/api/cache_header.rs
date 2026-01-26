//! Cache header middleware for X-Cache header support.
//!
//! This module provides Tower middleware for adding cache status headers
//! to HTTP responses. It implements CACHE-REQ-020 and CACHE-REQ-021 from
//! the cache semantics requirements.
//!
//! # Headers
//!
//! - `X-Cache`: `HIT` | `MISS` (cache hit/miss status)
//! - `X-Cache-Status`: `hit` | `miss` | `bypass` | `error` (detailed status)
//! - `X-Cache-Source`: `redis` | `memory` | `none` (cache source type)
//!
//! # Current Implementation
//!
//! **Note**: The current implementation sets cache headers directly in handlers using
//! the [`build_cache_headers`](super::handlers::build_cache_headers) function rather
//! than using the middleware layer. This approach was chosen because:
//!
//! 1. Cache status is determined at the handler level (after repository operations)
//! 2. Direct header setting is simpler and avoids request extension overhead
//! 3. The handler has direct access to `CacheStatus` from repository operations
//!
//! **Note: The `CacheHeaderLayer` middleware is currently not used in production.**
//! The middleware reads request extensions before the inner service call, which means
//! handlers cannot set the extension for the middleware to read. This is retained for
//! future extensibility where:
//!
//! - Response extensions could be used instead of request extensions
//! - A pre-handler middleware could set the extension based on route configuration
//! - Automatic header injection might be preferred for routes that don't directly
//!   call cacheable repository methods
//!
//! # Header Attachment Policy
//!
//! Cache headers are only attached to successful responses (HTTP 200 OK):
//!
//! - **404 Not Found**: No cache headers are attached because the entity does not exist,
//!   and the response does not represent a cache operation result.
//! - **500 Internal Server Error**: No cache headers are attached because the server
//!   failed to process the request, and cache status is not meaningful.
//! - **200 OK**: Cache headers are attached to indicate whether the data was served
//!   from cache (HIT) or fetched from the primary data source (MISS).
//!
//! This policy ensures cache headers accurately reflect the cache operation outcome
//! for the requested resource.
//!
//! # Middleware Usage (Future Extension)
//!
//! ```ignore
//! use axum::Router;
//! use tower::ServiceBuilder;
//!
//! let app = Router::new()
//!     .route("/tasks/{id}", get(get_task))
//!     .layer(ServiceBuilder::new().layer(CacheHeaderLayer));
//! ```
//!
//! # Architecture
//!
//! The middleware follows a functional programming approach:
//! - `CacheHeaderExtension` is an immutable value object carrying cache metadata
//! - Handlers set headers directly (current implementation)
//! - The middleware layer is available for future use with response extensions

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::http::{HeaderValue, Request, Response};
use tower::{Layer, Service};

use crate::infrastructure::CacheStatus;

// =============================================================================
// Cache Source Type
// =============================================================================

/// Source of the cached data.
///
/// This enum indicates where the cached data was retrieved from,
/// enabling metrics collection to distinguish between different cache tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CacheSource {
    /// Data was retrieved from Redis cache.
    Redis,
    /// Data was retrieved from in-memory cache (e.g., `SearchCache`).
    Memory,
    /// No cache was used (cache-bypassed or non-cacheable endpoint).
    #[default]
    None,
}

impl fmt::Display for CacheSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Redis => write!(formatter, "redis"),
            Self::Memory => write!(formatter, "memory"),
            Self::None => write!(formatter, "none"),
        }
    }
}

// =============================================================================
// Cache Header Extension
// =============================================================================

/// Extension carrying cache status information for a request.
///
/// This struct provides cache status metadata that can be used for:
/// 1. Direct header construction via [`x_cache_value`](Self::x_cache_value),
///    [`x_cache_status_value`](Self::x_cache_status_value), and
///    [`x_cache_source_value`](Self::x_cache_source_value) methods
/// 2. Future middleware integration via request/response extensions
///
/// **Current Usage**: Handlers use the factory methods to create instances,
/// then call the header value methods to build response headers directly.
/// See [`build_cache_headers`](super::handlers::build_cache_headers).
///
/// # Functional Programming Notes
///
/// - Immutable value object (all fields are read-only after construction)
/// - Pure construction via factory methods (`redis_hit()`, `redis_miss()`, `bypass()`, etc.)
/// - No side effects in the struct itself
#[derive(Debug, Clone)]
pub struct CacheHeaderExtension {
    /// The cache operation status.
    pub status: CacheStatus,
    /// The source of the cached data.
    pub source: CacheSource,
}

impl CacheHeaderExtension {
    /// Creates a new cache header extension.
    #[must_use]
    pub const fn new(status: CacheStatus, source: CacheSource) -> Self {
        Self { status, source }
    }

    /// Creates an extension for a Redis cache hit.
    #[must_use]
    pub const fn redis_hit() -> Self {
        Self::new(CacheStatus::Hit, CacheSource::Redis)
    }

    /// Creates an extension for a Redis cache miss.
    #[must_use]
    pub const fn redis_miss() -> Self {
        Self::new(CacheStatus::Miss, CacheSource::Redis)
    }

    /// Creates an extension for a cache bypass (no cache used).
    ///
    /// When cache is bypassed (disabled or non-cacheable operation), the cache
    /// source is `None` because no cache layer was consulted.
    #[must_use]
    pub const fn bypass() -> Self {
        Self::new(CacheStatus::Bypass, CacheSource::None)
    }

    /// Creates an extension for an in-memory cache hit.
    #[must_use]
    pub const fn memory_hit() -> Self {
        Self::new(CacheStatus::Hit, CacheSource::Memory)
    }

    /// Creates an extension for an in-memory cache miss.
    #[must_use]
    pub const fn memory_miss() -> Self {
        Self::new(CacheStatus::Miss, CacheSource::Memory)
    }

    /// Creates an extension for a Redis cache error.
    ///
    /// Used when Redis failed but data was fetched from primary storage (fail-open).
    #[must_use]
    pub const fn redis_error() -> Self {
        Self::new(CacheStatus::Error, CacheSource::Redis)
    }

    /// Returns the X-Cache header value.
    ///
    /// - `Hit` -> `"HIT"`
    /// - `Miss` -> `"MISS"`
    /// - `Bypass` -> `"MISS"`
    /// - `Error` -> `"MISS"` (data was fetched from primary, so treat as miss)
    #[must_use]
    pub const fn x_cache_value(&self) -> &'static str {
        match self.status {
            CacheStatus::Hit => "HIT",
            CacheStatus::Miss | CacheStatus::Bypass | CacheStatus::Error => "MISS",
        }
    }

    /// Returns the X-Cache-Status header value.
    ///
    /// - `Hit` -> `"hit"`
    /// - `Miss` -> `"miss"`
    /// - `Bypass` -> `"bypass"`
    /// - `Error` -> `"error"`
    #[must_use]
    pub const fn x_cache_status_value(&self) -> &'static str {
        match self.status {
            CacheStatus::Hit => "hit",
            CacheStatus::Miss => "miss",
            CacheStatus::Bypass => "bypass",
            CacheStatus::Error => "error",
        }
    }

    /// Returns the X-Cache-Source header value.
    #[must_use]
    pub const fn x_cache_source_value(&self) -> &'static str {
        match self.source {
            CacheSource::Redis => "redis",
            CacheSource::Memory => "memory",
            CacheSource::None => "none",
        }
    }
}

impl Default for CacheHeaderExtension {
    fn default() -> Self {
        Self::new(CacheStatus::Miss, CacheSource::None)
    }
}

// =============================================================================
// Cache Header Layer
// =============================================================================

/// Tower layer for adding cache status headers to responses.
///
/// **Note: This middleware is currently not used in production.**
///
/// The current implementation reads request extensions before calling the inner service,
/// which means handlers cannot set the extension after the request is processed.
/// This layer is retained for future extensibility where response extensions or
/// pre-handler configuration might be used.
///
/// For the current working implementation, see
/// [`build_cache_headers`](super::handlers::build_cache_headers) which handlers use
/// directly to set cache headers on responses.
///
/// # Headers Added (when extension is present)
///
/// Only adds headers when `CacheHeaderExtension` is present in request extensions:
/// - `X-Cache`: `HIT` | `MISS`
/// - `X-Cache-Status`: `hit` | `miss` | `bypass` | `error`
/// - `X-Cache-Source`: `redis` | `memory` | `none`
///
/// # Future Design Consideration
///
/// To make this middleware functional, one of these approaches could be used:
/// 1. Use response extensions instead of request extensions
/// 2. Have a pre-handler middleware set the extension based on route configuration
/// 3. Use a shared state that handlers can write to and the middleware can read from
#[derive(Debug, Clone, Copy, Default)]
pub struct CacheHeaderLayer;

impl CacheHeaderLayer {
    /// Creates a new cache header layer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for CacheHeaderLayer {
    type Service = CacheHeaderService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CacheHeaderService { inner }
    }
}

// =============================================================================
// Cache Header Service
// =============================================================================

/// Service that adds cache headers to responses.
///
/// This service is created by `CacheHeaderLayer` and wraps the inner service.
#[derive(Debug, Clone)]
pub struct CacheHeaderService<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for CacheHeaderService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(context)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        // Extract the cache header extension from request extensions
        let cache_extension = request.extensions().get::<CacheHeaderExtension>().cloned();

        // Clone the inner service for the async block
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Call the inner service
            let mut response = inner.call(request).await?;

            // Add cache headers if extension is present
            if let Some(extension) = cache_extension {
                let headers = response.headers_mut();

                // X-Cache: HIT | MISS
                headers.insert(
                    "X-Cache",
                    HeaderValue::from_static(extension.x_cache_value()),
                );

                // X-Cache-Status: hit | miss | bypass | error
                headers.insert(
                    "X-Cache-Status",
                    HeaderValue::from_static(extension.x_cache_status_value()),
                );

                // X-Cache-Source: redis | memory | none
                headers.insert(
                    "X-Cache-Source",
                    HeaderValue::from_static(extension.x_cache_source_value()),
                );
            }

            Ok(response)
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // CacheSource Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_cache_source_display_redis() {
        let source = CacheSource::Redis;
        assert_eq!(source.to_string(), "redis");
    }

    #[rstest]
    fn test_cache_source_display_memory() {
        let source = CacheSource::Memory;
        assert_eq!(source.to_string(), "memory");
    }

    #[rstest]
    fn test_cache_source_display_none() {
        let source = CacheSource::None;
        assert_eq!(source.to_string(), "none");
    }

    #[rstest]
    fn test_cache_source_default() {
        let source = CacheSource::default();
        assert_eq!(source, CacheSource::None);
    }

    // -------------------------------------------------------------------------
    // CacheHeaderExtension Construction Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_cache_header_extension_new() {
        let extension = CacheHeaderExtension::new(CacheStatus::Hit, CacheSource::Redis);
        assert_eq!(extension.status, CacheStatus::Hit);
        assert_eq!(extension.source, CacheSource::Redis);
    }

    #[rstest]
    fn test_cache_header_extension_redis_hit() {
        let extension = CacheHeaderExtension::redis_hit();
        assert_eq!(extension.status, CacheStatus::Hit);
        assert_eq!(extension.source, CacheSource::Redis);
    }

    #[rstest]
    fn test_cache_header_extension_redis_miss() {
        let extension = CacheHeaderExtension::redis_miss();
        assert_eq!(extension.status, CacheStatus::Miss);
        assert_eq!(extension.source, CacheSource::Redis);
    }

    #[rstest]
    fn test_cache_header_extension_bypass() {
        let extension = CacheHeaderExtension::bypass();
        assert_eq!(extension.status, CacheStatus::Bypass);
        assert_eq!(extension.source, CacheSource::None);
    }

    #[rstest]
    fn test_cache_header_extension_memory_hit() {
        let extension = CacheHeaderExtension::memory_hit();
        assert_eq!(extension.status, CacheStatus::Hit);
        assert_eq!(extension.source, CacheSource::Memory);
    }

    #[rstest]
    fn test_cache_header_extension_memory_miss() {
        let extension = CacheHeaderExtension::memory_miss();
        assert_eq!(extension.status, CacheStatus::Miss);
        assert_eq!(extension.source, CacheSource::Memory);
    }

    #[rstest]
    fn test_cache_header_extension_redis_error() {
        let extension = CacheHeaderExtension::redis_error();
        assert_eq!(extension.status, CacheStatus::Error);
        assert_eq!(extension.source, CacheSource::Redis);
    }

    #[rstest]
    fn test_cache_header_extension_default() {
        let extension = CacheHeaderExtension::default();
        assert_eq!(extension.status, CacheStatus::Miss);
        assert_eq!(extension.source, CacheSource::None);
    }

    // -------------------------------------------------------------------------
    // X-Cache Header Value Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_x_cache_value_hit() {
        let extension = CacheHeaderExtension::redis_hit();
        assert_eq!(extension.x_cache_value(), "HIT");
    }

    #[rstest]
    fn test_x_cache_value_miss() {
        let extension = CacheHeaderExtension::redis_miss();
        assert_eq!(extension.x_cache_value(), "MISS");
    }

    #[rstest]
    fn test_x_cache_value_bypass() {
        let extension = CacheHeaderExtension::bypass();
        assert_eq!(extension.x_cache_value(), "MISS");
    }

    #[rstest]
    fn test_x_cache_value_error() {
        let extension = CacheHeaderExtension::redis_error();
        assert_eq!(extension.x_cache_value(), "MISS");
    }

    // -------------------------------------------------------------------------
    // X-Cache-Status Header Value Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_x_cache_status_value_hit() {
        let extension = CacheHeaderExtension::redis_hit();
        assert_eq!(extension.x_cache_status_value(), "hit");
    }

    #[rstest]
    fn test_x_cache_status_value_miss() {
        let extension = CacheHeaderExtension::redis_miss();
        assert_eq!(extension.x_cache_status_value(), "miss");
    }

    #[rstest]
    fn test_x_cache_status_value_bypass() {
        let extension = CacheHeaderExtension::bypass();
        assert_eq!(extension.x_cache_status_value(), "bypass");
    }

    #[rstest]
    fn test_x_cache_status_value_error() {
        let extension = CacheHeaderExtension::redis_error();
        assert_eq!(extension.x_cache_status_value(), "error");
    }

    // -------------------------------------------------------------------------
    // X-Cache-Source Header Value Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_x_cache_source_value_redis() {
        let extension = CacheHeaderExtension::redis_hit();
        assert_eq!(extension.x_cache_source_value(), "redis");
    }

    #[rstest]
    fn test_x_cache_source_value_memory() {
        let extension = CacheHeaderExtension::memory_hit();
        assert_eq!(extension.x_cache_source_value(), "memory");
    }

    #[rstest]
    fn test_x_cache_source_value_none() {
        let extension = CacheHeaderExtension::default();
        assert_eq!(extension.x_cache_source_value(), "none");
    }

    // -------------------------------------------------------------------------
    // Combined Header Values Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[case(CacheStatus::Hit, CacheSource::Redis, "HIT", "hit", "redis")]
    #[case(CacheStatus::Miss, CacheSource::Redis, "MISS", "miss", "redis")]
    #[case(CacheStatus::Bypass, CacheSource::None, "MISS", "bypass", "none")]
    #[case(CacheStatus::Error, CacheSource::Redis, "MISS", "error", "redis")]
    #[case(CacheStatus::Hit, CacheSource::Memory, "HIT", "hit", "memory")]
    #[case(CacheStatus::Miss, CacheSource::Memory, "MISS", "miss", "memory")]
    #[case(CacheStatus::Miss, CacheSource::None, "MISS", "miss", "none")]
    fn test_header_values_combinations(
        #[case] status: CacheStatus,
        #[case] source: CacheSource,
        #[case] expected_x_cache: &str,
        #[case] expected_x_cache_status: &str,
        #[case] expected_x_cache_source: &str,
    ) {
        let extension = CacheHeaderExtension::new(status, source);
        assert_eq!(extension.x_cache_value(), expected_x_cache);
        assert_eq!(extension.x_cache_status_value(), expected_x_cache_status);
        assert_eq!(extension.x_cache_source_value(), expected_x_cache_source);
    }

    // -------------------------------------------------------------------------
    // CacheHeaderLayer Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_cache_header_layer_new() {
        let layer = CacheHeaderLayer::new();
        // Just verify it can be created
        let _ = layer;
    }

    #[rstest]
    fn test_cache_header_layer_default() {
        let layer = CacheHeaderLayer;
        // Just verify Default implementation works (unit struct)
        let _ = layer;
    }
}
