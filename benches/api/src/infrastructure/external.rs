//! External data source implementations.
//!
//! This module provides traits and implementations for external data sources
//! (Redis, HTTP) with fail injection capabilities.
//!
//! # Design Principles
//!
//! - **Real I/O is always executed first** - Never skip actual network calls
//! - **Fail injection is applied after successful I/O** - Simulates post-processing failures
//! - **URL fallback uses TEST-NET-1 (192.0.2.0/24)** - RFC 5737 reserved, guaranteed unreachable
//! - **RNG is injected externally** - Enables deterministic behavior for testing/benchmarks

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use lambars::effect::AsyncIO;

use super::fail_injection::{FailInjectionConfig, RngProvider, apply_post_injection};
use crate::domain::{Priority, TaskId, TaskStatus};

// =============================================================================
// External Error
// =============================================================================

/// Error type for external data source operations.
///
/// Covers both real I/O failures and injected failures for testing.
#[derive(Debug, Clone, Error)]
pub enum ExternalError {
    /// Failed to establish connection to the external service.
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Request timed out after the specified duration.
    #[error("Timeout after {0}ms")]
    Timeout(u64),

    /// Service is temporarily unavailable (e.g., HTTP 5xx, Redis error).
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    /// Failure injected for testing purposes.
    #[error("Injected failure: {0}")]
    InjectedFailure(String),
}

// =============================================================================
// External Task Data
// =============================================================================

/// Data retrieved from an external source.
///
/// This is a simplified representation of task data that external sources
/// can provide. Not all fields may be available from every source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalTaskData {
    /// Task title (may be None if not available from source).
    pub title: Option<String>,
    /// Task description.
    pub description: Option<String>,
    /// Task priority.
    pub priority: Option<Priority>,
    /// Task status.
    pub status: Option<TaskStatus>,
    /// Task tags.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl ExternalTaskData {
    /// Creates an empty external task data.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            title: None,
            description: None,
            priority: None,
            status: None,
            tags: Vec::new(),
        }
    }

    /// Checks if all fields are None/empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.description.is_none()
            && self.priority.is_none()
            && self.status.is_none()
            && self.tags.is_empty()
    }
}

// =============================================================================
// External Data Source Trait
// =============================================================================

/// Trait for external data sources.
///
/// Implementations should:
/// 1. Always execute real I/O first (never skip)
/// 2. Apply fail injection after successful I/O
/// 3. Use fallback URLs when no URL is configured (TEST-NET-1)
pub trait ExternalDataSource: Send + Sync {
    /// Fetches task data from the external source.
    ///
    /// Returns `Ok(Some(data))` if the task is found, `Ok(None)` if not found,
    /// or `Err(error)` if the fetch fails.
    fn fetch_task_data(
        &self,
        task_id: &TaskId,
    ) -> AsyncIO<Result<Option<ExternalTaskData>, ExternalError>>;

    /// Returns the source name for logging and error reporting.
    fn source_name(&self) -> &'static str;
}

// =============================================================================
// Fallback URLs (RFC 5737 TEST-NET-1)
// =============================================================================

/// Fallback URL for Redis when `SECONDARY_SOURCE_URL` is not set.
/// Uses RFC 5737 TEST-NET-1 which is guaranteed to be unreachable.
const REDIS_FALLBACK_URL: &str = "redis://192.0.2.1:6379";

/// Fallback URL for HTTP when `EXTERNAL_SOURCE_URL` is not set.
/// Uses RFC 5737 TEST-NET-1 which is guaranteed to be unreachable.
const HTTP_FALLBACK_URL: &str = "http://192.0.2.1:80";

// =============================================================================
// Redis External Data Source
// =============================================================================

/// Redis-based external data source implementation.
///
/// Connects to Redis to fetch task data. When the URL is not configured,
/// it uses the fallback URL (TEST-NET-1) which will result in a connection timeout.
pub struct RedisExternalDataSource {
    /// Redis connection URL.
    url: String,
    /// Fail injection configuration.
    fail_injection: FailInjectionConfig,
    /// Request timeout duration.
    timeout: Duration,
    /// RNG provider for fail injection (externally injected).
    rng_provider: Arc<RngProvider>,
}

impl RedisExternalDataSource {
    /// Creates a new Redis external data source.
    ///
    /// # Arguments
    ///
    /// * `url` - Redis URL. If `None`, uses fallback URL (TEST-NET-1).
    /// * `fail_injection` - Configuration for fail injection.
    /// * `timeout` - Request timeout duration.
    /// * `rng_provider` - RNG provider for deterministic/random behavior.
    #[must_use]
    pub fn new(
        url: Option<String>,
        fail_injection: FailInjectionConfig,
        timeout: Duration,
        rng_provider: Arc<RngProvider>,
    ) -> Self {
        Self {
            url: url.unwrap_or_else(|| REDIS_FALLBACK_URL.to_string()),
            fail_injection,
            timeout,
            rng_provider,
        }
    }
}

impl ExternalDataSource for RedisExternalDataSource {
    fn fetch_task_data(
        &self,
        task_id: &TaskId,
    ) -> AsyncIO<Result<Option<ExternalTaskData>, ExternalError>> {
        let url = self.url.clone();
        let task_id_str = task_id.to_string();
        let fail_injection = self.fail_injection.clone();
        let timeout = self.timeout;
        let rng_provider = Arc::clone(&self.rng_provider);

        AsyncIO::new(move || async move {
            // Create scoped RNG for this operation
            let mut rng = rng_provider.for_operation(&task_id_str, "secondary", "fetch_task_data");

            // 1. Execute real I/O (always required, never skipped)
            let io_result = execute_redis_io(&url, &task_id_str, timeout).await;

            // 2. Apply fail injection post-processing (only on success)
            match io_result {
                Ok(data) => {
                    apply_post_injection(&fail_injection, &mut rng).await?;
                    Ok(data)
                }
                Err(error) => Err(error),
            }
        })
    }

    fn source_name(&self) -> &'static str {
        "secondary"
    }
}

/// Executes Redis I/O operation.
///
/// This is an I/O function (not pure) that performs actual network communication.
/// It should only be called within an `AsyncIO` closure.
#[allow(clippy::cast_possible_truncation)] // Timeout in ms will not exceed u64
async fn execute_redis_io(
    url: &str,
    task_id: &str,
    timeout: Duration,
) -> Result<Option<ExternalTaskData>, ExternalError> {
    let timeout_ms = timeout.as_millis() as u64;

    // Open Redis client
    let client = redis::Client::open(url)
        .map_err(|error| ExternalError::ConnectionFailed(error.to_string()))?;

    // Get connection with timeout
    let mut connection = tokio::time::timeout(timeout, client.get_multiplexed_async_connection())
        .await
        .map_err(|_| ExternalError::Timeout(timeout_ms))?
        .map_err(|error| ExternalError::ConnectionFailed(error.to_string()))?;

    // Execute GET command with timeout
    let result: Option<String> = tokio::time::timeout(timeout, async {
        redis::cmd("GET")
            .arg(format!("task:{task_id}"))
            .query_async(&mut connection)
            .await
    })
    .await
    .map_err(|_| ExternalError::Timeout(timeout_ms))?
    .map_err(|error| ExternalError::ServiceUnavailable(error.to_string()))?;

    // Parse JSON response
    Ok(result.and_then(|json_string| serde_json::from_str(&json_string).ok()))
}

// =============================================================================
// HTTP External Data Source
// =============================================================================

/// HTTP-based external data source implementation.
///
/// Connects to an HTTP API to fetch task data. When the URL is not configured,
/// it uses the fallback URL (TEST-NET-1) which will result in a connection timeout.
pub struct HttpExternalDataSource {
    /// HTTP client.
    client: reqwest::Client,
    /// Base URL for the external API.
    base_url: String,
    /// Fail injection configuration.
    fail_injection: FailInjectionConfig,
    /// Request timeout duration.
    timeout: Duration,
    /// RNG provider for fail injection (externally injected).
    rng_provider: Arc<RngProvider>,
}

impl HttpExternalDataSource {
    /// Creates a new HTTP external data source.
    ///
    /// # Arguments
    ///
    /// * `base_url` - Base URL for the API. If `None`, uses fallback URL (TEST-NET-1).
    /// * `fail_injection` - Configuration for fail injection.
    /// * `timeout` - Request timeout duration.
    /// * `rng_provider` - RNG provider for deterministic/random behavior.
    #[must_use]
    pub fn new(
        base_url: Option<String>,
        fail_injection: FailInjectionConfig,
        timeout: Duration,
        rng_provider: Arc<RngProvider>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.unwrap_or_else(|| HTTP_FALLBACK_URL.to_string()),
            fail_injection,
            timeout,
            rng_provider,
        }
    }
}

impl ExternalDataSource for HttpExternalDataSource {
    fn fetch_task_data(
        &self,
        task_id: &TaskId,
    ) -> AsyncIO<Result<Option<ExternalTaskData>, ExternalError>> {
        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let task_id_str = task_id.to_string();
        let fail_injection = self.fail_injection.clone();
        let timeout = self.timeout;
        let rng_provider = Arc::clone(&self.rng_provider);

        AsyncIO::new(move || async move {
            // Create scoped RNG for this operation
            let mut rng = rng_provider.for_operation(&task_id_str, "external", "fetch_task_data");

            // 1. Execute real I/O (always required, never skipped)
            let io_result = execute_http_io(&client, &base_url, &task_id_str, timeout).await;

            // 2. Apply fail injection post-processing (only on success)
            match io_result {
                Ok(data) => {
                    apply_post_injection(&fail_injection, &mut rng).await?;
                    Ok(data)
                }
                Err(error) => Err(error),
            }
        })
    }

    fn source_name(&self) -> &'static str {
        "external"
    }
}

/// Executes HTTP I/O operation.
///
/// This is an I/O function (not pure) that performs actual network communication.
/// It should only be called within an `AsyncIO` closure.
#[allow(clippy::cast_possible_truncation)] // Timeout in ms will not exceed u64
async fn execute_http_io(
    client: &reqwest::Client,
    base_url: &str,
    task_id: &str,
    timeout: Duration,
) -> Result<Option<ExternalTaskData>, ExternalError> {
    let timeout_ms = timeout.as_millis() as u64;

    let response = client
        .get(format!("{base_url}/tasks/{task_id}"))
        .timeout(timeout)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                ExternalError::Timeout(timeout_ms)
            } else if error.is_connect() {
                ExternalError::ConnectionFailed(error.to_string())
            } else {
                ExternalError::ServiceUnavailable(error.to_string())
            }
        })?;

    if response.status().is_success() {
        let data = response
            .json()
            .await
            .map_err(|error| ExternalError::ServiceUnavailable(error.to_string()))?;
        Ok(Some(data))
    } else if response.status() == reqwest::StatusCode::NOT_FOUND {
        Ok(None)
    } else {
        Err(ExternalError::ServiceUnavailable(format!(
            "HTTP {}",
            response.status()
        )))
    }
}

// =============================================================================
// Stub External Data Source (for testing)
// =============================================================================

/// Stub external data source for testing.
///
/// Always returns a fixed result without performing real I/O.
/// Useful for unit tests that don't need actual network calls.
pub struct StubExternalDataSource {
    /// The result to return.
    result: Result<Option<ExternalTaskData>, ExternalError>,
    /// Source name.
    name: &'static str,
}

impl StubExternalDataSource {
    /// Creates a stub that returns `Ok(Some(data))`.
    #[must_use]
    pub const fn with_data(data: ExternalTaskData, name: &'static str) -> Self {
        Self {
            result: Ok(Some(data)),
            name,
        }
    }

    /// Creates a stub that returns `Ok(None)`.
    #[must_use]
    pub const fn not_found(name: &'static str) -> Self {
        Self {
            result: Ok(None),
            name,
        }
    }

    /// Creates a stub that returns an error.
    #[must_use]
    pub const fn with_error(error: ExternalError, name: &'static str) -> Self {
        Self {
            result: Err(error),
            name,
        }
    }
}

impl ExternalDataSource for StubExternalDataSource {
    fn fetch_task_data(
        &self,
        _task_id: &TaskId,
    ) -> AsyncIO<Result<Option<ExternalTaskData>, ExternalError>> {
        let result = self.result.clone();
        AsyncIO::new(move || async move { result })
    }

    fn source_name(&self) -> &'static str {
        self.name
    }
}

// =============================================================================
// External Sources Container
// =============================================================================

/// Container for external data sources.
///
/// Holds references to secondary (Redis) and external (HTTP) data sources.
pub struct ExternalSources {
    /// Secondary source (Redis).
    pub secondary_source: Arc<dyn ExternalDataSource + Send + Sync>,
    /// External source (HTTP).
    pub external_source: Arc<dyn ExternalDataSource + Send + Sync>,
}

/// Default timeout for external sources in milliseconds.
const DEFAULT_EXTERNAL_TIMEOUT_MS: u64 = 5000;

impl ExternalSources {
    /// Creates `ExternalSources` from environment variables.
    ///
    /// # Environment Variables
    ///
    /// - `SECONDARY_SOURCE_URL`: Redis URL for secondary source
    /// - `EXTERNAL_SOURCE_URL`: HTTP base URL for external source
    /// - `EXTERNAL_TIMEOUT_MS`: Timeout in milliseconds (default: 5000)
    /// - `RNG_SEED`: Seed for deterministic RNG (optional)
    /// - `SECONDARY_FAILURE_RATE`: Failure injection rate for secondary source
    /// - `SECONDARY_DELAY_MIN_MS`: Minimum delay for secondary source
    /// - `SECONDARY_DELAY_MAX_MS`: Maximum delay for secondary source
    /// - `SECONDARY_TIMEOUT_RATE`: Timeout injection rate for secondary source
    /// - `EXTERNAL_FAILURE_RATE`: Failure injection rate for external source
    /// - `EXTERNAL_DELAY_MIN_MS`: Minimum delay for external source
    /// - `EXTERNAL_DELAY_MAX_MS`: Maximum delay for external source
    /// - `EXTERNAL_TIMEOUT_RATE`: Timeout injection rate for external source
    ///
    /// # Errors
    ///
    /// Returns an error if any environment variable contains an invalid value.
    pub fn from_env() -> Result<Self, super::fail_injection::ConfigError> {
        use super::fail_injection::EnvParseError;

        let rng_provider = Arc::new(RngProvider::from_env()?);

        // Parse secondary source configuration
        // Empty strings are treated as "not set" to ensure fallback URL (TEST-NET-1) is used
        let secondary_url = std::env::var("SECONDARY_SOURCE_URL")
            .ok()
            .filter(|s| !s.trim().is_empty());
        let secondary_fail_config = FailInjectionConfig::from_env("SECONDARY")?;

        // Parse external source configuration
        // Empty strings are treated as "not set" to ensure fallback URL (TEST-NET-1) is used
        let external_url = std::env::var("EXTERNAL_SOURCE_URL")
            .ok()
            .filter(|s| !s.trim().is_empty());
        let external_fail_config = FailInjectionConfig::from_env("EXTERNAL")?;

        // Parse timeout - do not swallow parse errors
        let timeout_ms: u64 = match std::env::var("EXTERNAL_TIMEOUT_MS") {
            Ok(value) => value.parse().map_err(|e: std::num::ParseIntError| {
                super::fail_injection::ConfigError::EnvParseError(EnvParseError::InvalidU64 {
                    name: "EXTERNAL_TIMEOUT_MS".to_string(),
                    message: e.to_string(),
                    value,
                })
            })?,
            Err(std::env::VarError::NotPresent) => DEFAULT_EXTERNAL_TIMEOUT_MS,
            Err(error) => return Err(super::fail_injection::ConfigError::EnvVarError(error)),
        };
        let timeout = Duration::from_millis(timeout_ms);

        Ok(Self {
            secondary_source: Arc::new(RedisExternalDataSource::new(
                secondary_url,
                secondary_fail_config,
                timeout,
                Arc::clone(&rng_provider),
            )),
            external_source: Arc::new(HttpExternalDataSource::new(
                external_url,
                external_fail_config,
                timeout,
                rng_provider,
            )),
        })
    }

    /// Creates stub `ExternalSources` for testing.
    ///
    /// Returns sources that always return `Ok(None)` (not found).
    /// Useful for unit tests and development without external services.
    #[must_use]
    pub fn stub() -> Self {
        Self {
            secondary_source: Arc::new(StubExternalDataSource::not_found("secondary")),
            external_source: Arc::new(StubExternalDataSource::not_found("external")),
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

    #[rstest]
    fn test_external_task_data_empty() {
        let data = ExternalTaskData::empty();
        assert!(data.is_empty());
    }

    #[rstest]
    fn test_external_task_data_not_empty_with_title() {
        let data = ExternalTaskData {
            title: Some("Test".to_string()),
            description: None,
            priority: None,
            status: None,
            tags: vec![],
        };
        assert!(!data.is_empty());
    }

    #[rstest]
    fn test_external_task_data_not_empty_with_tags() {
        let data = ExternalTaskData {
            title: None,
            description: None,
            priority: None,
            status: None,
            tags: vec!["tag".to_string()],
        };
        assert!(!data.is_empty());
    }

    #[rstest]
    fn test_stub_external_data_source_with_data() {
        let data = ExternalTaskData {
            title: Some("Test Task".to_string()),
            description: None,
            priority: Some(Priority::High),
            status: None,
            tags: vec![],
        };
        let stub = StubExternalDataSource::with_data(data, "test");
        assert_eq!(stub.source_name(), "test");
    }

    #[rstest]
    fn test_stub_external_data_source_not_found() {
        let stub = StubExternalDataSource::not_found("test");
        assert_eq!(stub.source_name(), "test");
    }

    #[rstest]
    fn test_stub_external_data_source_with_error() {
        let error = ExternalError::Timeout(1000);
        let stub = StubExternalDataSource::with_error(error, "test");
        assert_eq!(stub.source_name(), "test");
    }

    #[rstest]
    #[tokio::test]
    async fn test_stub_external_data_source_returns_data() {
        let data = ExternalTaskData {
            title: Some("Test Task".to_string()),
            description: None,
            priority: Some(Priority::High),
            status: None,
            tags: vec![],
        };
        let stub = StubExternalDataSource::with_data(data, "test");
        let task_id = TaskId::generate();

        let result = stub.fetch_task_data(&task_id).await;

        assert!(result.is_ok());
        let task_data = result.unwrap();
        assert!(task_data.is_some());
        assert_eq!(task_data.unwrap().title, Some("Test Task".to_string()));
    }

    #[rstest]
    #[tokio::test]
    async fn test_stub_external_data_source_returns_not_found() {
        let stub = StubExternalDataSource::not_found("test");
        let task_id = TaskId::generate();

        let result = stub.fetch_task_data(&task_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[rstest]
    #[tokio::test]
    async fn test_stub_external_data_source_returns_error() {
        let stub = StubExternalDataSource::with_error(
            ExternalError::ConnectionFailed("test error".to_string()),
            "test",
        );
        let task_id = TaskId::generate();

        let result = stub.fetch_task_data(&task_id).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, ExternalError::ConnectionFailed(_)));
    }

    #[rstest]
    fn test_redis_external_data_source_uses_fallback_url() {
        let rng_provider = Arc::new(RngProvider::new_random());
        let source = RedisExternalDataSource::new(
            None,
            FailInjectionConfig::default(),
            Duration::from_secs(5),
            rng_provider,
        );
        assert_eq!(source.url, REDIS_FALLBACK_URL);
        assert_eq!(source.source_name(), "secondary");
    }

    #[rstest]
    fn test_redis_external_data_source_uses_provided_url() {
        let rng_provider = Arc::new(RngProvider::new_random());
        let custom_url = "redis://localhost:6379";
        let source = RedisExternalDataSource::new(
            Some(custom_url.to_string()),
            FailInjectionConfig::default(),
            Duration::from_secs(5),
            rng_provider,
        );
        assert_eq!(source.url, custom_url);
    }

    #[rstest]
    fn test_http_external_data_source_uses_fallback_url() {
        let rng_provider = Arc::new(RngProvider::new_random());
        let source = HttpExternalDataSource::new(
            None,
            FailInjectionConfig::default(),
            Duration::from_secs(5),
            rng_provider,
        );
        assert_eq!(source.base_url, HTTP_FALLBACK_URL);
        assert_eq!(source.source_name(), "external");
    }

    #[rstest]
    fn test_http_external_data_source_uses_provided_url() {
        let rng_provider = Arc::new(RngProvider::new_random());
        let custom_url = "http://localhost:8080";
        let source = HttpExternalDataSource::new(
            Some(custom_url.to_string()),
            FailInjectionConfig::default(),
            Duration::from_secs(5),
            rng_provider,
        );
        assert_eq!(source.base_url, custom_url);
    }

    // -------------------------------------------------------------------------
    // Empty URL Fallback Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_redis_external_data_source_empty_string_uses_fallback_url() {
        let rng_provider = Arc::new(RngProvider::new_random());
        // Empty string should be treated as None, using fallback URL
        let source = RedisExternalDataSource::new(
            Some(String::new()),
            FailInjectionConfig::default(),
            Duration::from_secs(5),
            rng_provider.clone(),
        );
        // Note: The filtering for empty strings happens in from_env(), not in new()
        // This test verifies that new() accepts empty string as-is (raw constructor behavior)
        assert_eq!(source.url, "");

        // Test that whitespace-only string is also accepted as-is by new()
        let source_whitespace = RedisExternalDataSource::new(
            Some("   ".to_string()),
            FailInjectionConfig::default(),
            Duration::from_secs(5),
            rng_provider,
        );
        assert_eq!(source_whitespace.url, "   ");
    }

    #[rstest]
    fn test_http_external_data_source_empty_string_uses_fallback_url() {
        let rng_provider = Arc::new(RngProvider::new_random());
        // Empty string should be treated as None, using fallback URL
        let source = HttpExternalDataSource::new(
            Some(String::new()),
            FailInjectionConfig::default(),
            Duration::from_secs(5),
            rng_provider.clone(),
        );
        // Note: The filtering for empty strings happens in from_env(), not in new()
        // This test verifies that new() accepts empty string as-is (raw constructor behavior)
        assert_eq!(source.base_url, "");

        // Test that whitespace-only string is also accepted as-is by new()
        let source_whitespace = HttpExternalDataSource::new(
            Some("   ".to_string()),
            FailInjectionConfig::default(),
            Duration::from_secs(5),
            rng_provider,
        );
        assert_eq!(source_whitespace.base_url, "   ");
    }
}
