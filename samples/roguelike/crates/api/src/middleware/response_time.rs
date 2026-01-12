//! Response time logging middleware.
//!
//! This module provides middleware to measure and log response times
//! for all HTTP requests.

use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use axum::http::header::HeaderName;
use axum::http::{HeaderValue, Request, Response};
use futures::future::BoxFuture;
use tower::{Layer, Service};

/// The header name for response time (in milliseconds).
pub static RESPONSE_TIME_HEADER: HeaderName = HeaderName::from_static("x-response-time");

// =============================================================================
// ResponseTimeLayer
// =============================================================================

/// Layer that measures and logs response times.
///
/// This layer will:
/// 1. Record the start time of each request
/// 2. Measure the total processing time
/// 3. Add an `X-Response-Time` header to the response
/// 4. Log the response time using tracing
///
/// # Examples
///
/// ```ignore
/// use roguelike_api::middleware::ResponseTimeLayer;
/// use axum::Router;
///
/// let app = Router::new()
///     .route("/api/games", get(list_games))
///     .layer(ResponseTimeLayer::new());
/// ```
#[derive(Debug, Clone, Default)]
pub struct ResponseTimeLayer {
    /// Minimum duration to log (for filtering out fast requests).
    min_duration_to_log: Option<Duration>,
}

impl ResponseTimeLayer {
    /// Creates a new `ResponseTimeLayer` that logs all requests.
    #[must_use]
    pub fn new() -> Self {
        Self {
            min_duration_to_log: None,
        }
    }

    /// Creates a new `ResponseTimeLayer` that only logs requests
    /// taking longer than the specified duration.
    ///
    /// # Arguments
    ///
    /// * `min_duration` - Minimum duration for a request to be logged
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use std::time::Duration;
    /// use roguelike_api::middleware::ResponseTimeLayer;
    ///
    /// // Only log requests taking more than 100ms
    /// let layer = ResponseTimeLayer::with_min_duration(Duration::from_millis(100));
    /// ```
    #[must_use]
    pub fn with_min_duration(min_duration: Duration) -> Self {
        Self {
            min_duration_to_log: Some(min_duration),
        }
    }

    /// Returns the minimum duration configured for logging.
    #[must_use]
    pub fn min_duration_to_log(&self) -> Option<Duration> {
        self.min_duration_to_log
    }
}

impl<InnerService> Layer<InnerService> for ResponseTimeLayer {
    type Service = ResponseTimeService<InnerService>;

    fn layer(&self, inner: InnerService) -> Self::Service {
        ResponseTimeService {
            inner,
            min_duration_to_log: self.min_duration_to_log,
        }
    }
}

// =============================================================================
// ResponseTimeService
// =============================================================================

/// Service that handles response time measurement.
#[derive(Debug, Clone)]
pub struct ResponseTimeService<InnerService> {
    inner: InnerService,
    min_duration_to_log: Option<Duration>,
}

impl<InnerService, RequestBody, ResponseBody> Service<Request<RequestBody>>
    for ResponseTimeService<InnerService>
where
    InnerService:
        Service<Request<RequestBody>, Response = Response<ResponseBody>> + Clone + Send + 'static,
    InnerService::Future: Send,
    RequestBody: Send + 'static,
    ResponseBody: Send + 'static,
{
    type Response = Response<ResponseBody>;
    type Error = InnerService::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(context)
    }

    fn call(&mut self, request: Request<RequestBody>) -> Self::Future {
        let start = Instant::now();
        let method = request.method().clone();
        let uri = request.uri().clone();
        let min_duration = self.min_duration_to_log;
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let mut response = inner.call(request).await?;

            let elapsed = start.elapsed();
            let elapsed_ms = elapsed.as_secs_f64() * 1000.0;

            // Add response time header
            if let Ok(header_value) = HeaderValue::from_str(&format!("{:.2}ms", elapsed_ms)) {
                response
                    .headers_mut()
                    .insert(RESPONSE_TIME_HEADER.clone(), header_value);
            }

            // Log the response time
            let should_log = min_duration.is_none() || elapsed >= min_duration.unwrap();
            if should_log {
                tracing::info!(
                    method = %method,
                    uri = %uri,
                    status = %response.status(),
                    response_time_ms = elapsed_ms,
                    "Request completed"
                );
            }

            Ok(response)
        })
    }
}

// =============================================================================
// ResponseTime Value Type
// =============================================================================

/// Response time measurement.
///
/// This type can be used to represent and format response times.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResponseTime(Duration);

impl ResponseTime {
    /// Creates a new `ResponseTime` from a `Duration`.
    #[must_use]
    pub fn new(duration: Duration) -> Self {
        Self(duration)
    }

    /// Returns the duration.
    #[must_use]
    pub fn duration(&self) -> Duration {
        self.0
    }

    /// Returns the duration in milliseconds.
    #[must_use]
    pub fn as_millis(&self) -> f64 {
        self.0.as_secs_f64() * 1000.0
    }

    /// Returns the duration in seconds.
    #[must_use]
    pub fn as_secs(&self) -> f64 {
        self.0.as_secs_f64()
    }

    /// Returns true if the response time exceeds the given threshold.
    #[must_use]
    pub fn is_slow(&self, threshold: Duration) -> bool {
        self.0 >= threshold
    }
}

impl std::fmt::Display for ResponseTime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let millis = self.as_millis();
        if millis < 1000.0 {
            write!(formatter, "{:.2}ms", millis)
        } else {
            write!(formatter, "{:.2}s", self.as_secs())
        }
    }
}

impl From<Duration> for ResponseTime {
    fn from(duration: Duration) -> Self {
        Self(duration)
    }
}

impl From<ResponseTime> for Duration {
    fn from(response_time: ResponseTime) -> Self {
        response_time.0
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    mod response_time_layer {
        use super::*;

        #[rstest]
        fn new_creates_layer() {
            let layer = ResponseTimeLayer::new();
            assert!(layer.min_duration_to_log().is_none());
        }

        #[rstest]
        fn with_min_duration_sets_threshold() {
            let layer = ResponseTimeLayer::with_min_duration(Duration::from_millis(100));
            assert_eq!(
                layer.min_duration_to_log(),
                Some(Duration::from_millis(100))
            );
        }

        #[rstest]
        fn default_creates_layer() {
            let layer = ResponseTimeLayer::default();
            assert!(layer.min_duration_to_log().is_none());
        }

        #[rstest]
        fn clone_layer() {
            let layer = ResponseTimeLayer::with_min_duration(Duration::from_millis(50));
            let cloned = layer.clone();
            assert_eq!(layer.min_duration_to_log(), cloned.min_duration_to_log());
        }

        #[rstest]
        fn debug_format() {
            let layer = ResponseTimeLayer::new();
            let debug = format!("{:?}", layer);
            assert!(debug.contains("ResponseTimeLayer"));
        }
    }

    mod response_time {
        use super::*;

        #[rstest]
        fn new_creates_response_time() {
            let rt = ResponseTime::new(Duration::from_millis(150));
            assert_eq!(rt.duration(), Duration::from_millis(150));
        }

        #[rstest]
        fn as_millis_converts_correctly() {
            let rt = ResponseTime::new(Duration::from_millis(150));
            assert!((rt.as_millis() - 150.0).abs() < 0.001);
        }

        #[rstest]
        fn as_millis_handles_fractional() {
            let rt = ResponseTime::new(Duration::from_micros(1500));
            assert!((rt.as_millis() - 1.5).abs() < 0.001);
        }

        #[rstest]
        fn as_secs_converts_correctly() {
            let rt = ResponseTime::new(Duration::from_secs(2));
            assert!((rt.as_secs() - 2.0).abs() < 0.001);
        }

        #[rstest]
        fn is_slow_returns_true_when_exceeds_threshold() {
            let rt = ResponseTime::new(Duration::from_millis(150));
            assert!(rt.is_slow(Duration::from_millis(100)));
        }

        #[rstest]
        fn is_slow_returns_false_when_below_threshold() {
            let rt = ResponseTime::new(Duration::from_millis(50));
            assert!(!rt.is_slow(Duration::from_millis(100)));
        }

        #[rstest]
        fn is_slow_returns_true_when_equal_to_threshold() {
            let rt = ResponseTime::new(Duration::from_millis(100));
            assert!(rt.is_slow(Duration::from_millis(100)));
        }

        #[rstest]
        fn display_format_milliseconds() {
            let rt = ResponseTime::new(Duration::from_millis(150));
            let display = format!("{}", rt);
            assert!(display.contains("ms"));
            assert!(display.contains("150"));
        }

        #[rstest]
        fn display_format_seconds() {
            let rt = ResponseTime::new(Duration::from_secs(2));
            let display = format!("{}", rt);
            assert!(display.contains("s"));
            assert!(display.contains("2.00"));
        }

        #[rstest]
        fn display_format_sub_millisecond() {
            let rt = ResponseTime::new(Duration::from_micros(500));
            let display = format!("{}", rt);
            assert!(display.contains("ms"));
            assert!(display.contains("0.50"));
        }

        #[rstest]
        fn from_duration() {
            let duration = Duration::from_millis(100);
            let rt: ResponseTime = duration.into();
            assert_eq!(rt.duration(), duration);
        }

        #[rstest]
        fn into_duration() {
            let rt = ResponseTime::new(Duration::from_millis(100));
            let duration: Duration = rt.into();
            assert_eq!(duration, Duration::from_millis(100));
        }

        #[rstest]
        fn equality() {
            let rt1 = ResponseTime::new(Duration::from_millis(100));
            let rt2 = ResponseTime::new(Duration::from_millis(100));
            let rt3 = ResponseTime::new(Duration::from_millis(200));

            assert_eq!(rt1, rt2);
            assert_ne!(rt1, rt3);
        }

        #[rstest]
        fn clone() {
            let rt1 = ResponseTime::new(Duration::from_millis(100));
            let rt2 = rt1;
            assert_eq!(rt1, rt2);
        }

        #[rstest]
        fn debug_format() {
            let rt = ResponseTime::new(Duration::from_millis(100));
            let debug = format!("{:?}", rt);
            assert!(debug.contains("ResponseTime"));
        }
    }
}
