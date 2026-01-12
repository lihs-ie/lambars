use std::task::{Context, Poll};

use axum::extract::FromRequestParts;
use axum::http::header::HeaderName;
use axum::http::request::Parts;
use axum::http::{HeaderValue, Request, Response, StatusCode};
use futures::future::BoxFuture;
use tower::{Layer, Service};
use uuid::Uuid;

pub static REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

// =============================================================================
// RequestId
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RequestId(String);

impl RequestId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl From<String> for RequestId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<RequestId> for String {
    fn from(value: RequestId) -> Self {
        value.0
    }
}

// =============================================================================
// RequestId Extractor
// =============================================================================

impl<State> FromRequestParts<State> for RequestId
where
    State: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &State,
    ) -> Result<Self, Self::Rejection> {
        parts
            .headers
            .get(&REQUEST_ID_HEADER)
            .and_then(|value| value.to_str().ok())
            .map(RequestId::new)
            .or_else(|| parts.extensions.get::<RequestId>().cloned())
            .ok_or((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Request ID not found. Ensure RequestIdLayer is applied.",
            ))
    }
}

// =============================================================================
// RequestIdLayer
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct RequestIdLayer;

impl RequestIdLayer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl<Service> Layer<Service> for RequestIdLayer {
    type Service = RequestIdService<Service>;

    fn layer(&self, inner: Service) -> Self::Service {
        RequestIdService { inner }
    }
}

// =============================================================================
// RequestIdService
// =============================================================================

#[derive(Debug, Clone)]
pub struct RequestIdService<Service> {
    inner: Service,
}

impl<InnerService, RequestBody, ResponseBody> Service<Request<RequestBody>>
    for RequestIdService<InnerService>
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

    fn call(&mut self, mut request: Request<RequestBody>) -> Self::Future {
        // Extract or generate request ID
        let request_id = request
            .headers()
            .get(&REQUEST_ID_HEADER)
            .and_then(|value| value.to_str().ok())
            .map(RequestId::new)
            .unwrap_or_else(RequestId::generate);

        // Add request ID to extensions for handler extraction
        request.extensions_mut().insert(request_id.clone());

        let mut inner = self.inner.clone();

        Box::pin(async move {
            let mut response = inner.call(request).await?;

            // Add request ID to response headers
            if let Ok(header_value) = HeaderValue::from_str(request_id.as_str()) {
                response
                    .headers_mut()
                    .insert(REQUEST_ID_HEADER.clone(), header_value);
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

    mod request_id {
        use super::*;

        #[rstest]
        fn new_creates_request_id() {
            let id = RequestId::new("abc-123");
            assert_eq!(id.as_str(), "abc-123");
        }

        #[rstest]
        fn generate_creates_unique_ids() {
            let id1 = RequestId::generate();
            let id2 = RequestId::generate();
            assert_ne!(id1, id2);
        }

        #[rstest]
        fn generate_creates_valid_uuid() {
            let id = RequestId::generate();
            // Should be parseable as UUID
            assert!(Uuid::parse_str(id.as_str()).is_ok());
        }

        #[rstest]
        fn as_str_returns_inner() {
            let id = RequestId::new("test-id");
            assert_eq!(id.as_str(), "test-id");
        }

        #[rstest]
        fn into_string_consumes_and_returns() {
            let id = RequestId::new("test-id");
            let string = id.into_string();
            assert_eq!(string, "test-id");
        }

        #[rstest]
        fn display_format() {
            let id = RequestId::new("abc-123");
            assert_eq!(format!("{}", id), "abc-123");
        }

        #[rstest]
        fn from_string() {
            let id: RequestId = "abc-123".to_string().into();
            assert_eq!(id.as_str(), "abc-123");
        }

        #[rstest]
        fn into_string_trait() {
            let id = RequestId::new("abc-123");
            let s: String = id.into();
            assert_eq!(s, "abc-123");
        }

        #[rstest]
        fn equality() {
            let id1 = RequestId::new("abc-123");
            let id2 = RequestId::new("abc-123");
            let id3 = RequestId::new("xyz-789");

            assert_eq!(id1, id2);
            assert_ne!(id1, id3);
        }

        #[rstest]
        fn clone() {
            let id1 = RequestId::new("abc-123");
            let id2 = id1.clone();
            assert_eq!(id1, id2);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let id1 = RequestId::new("abc-123");
            let id2 = RequestId::new("abc-123");
            let id3 = RequestId::new("xyz-789");

            let mut set = HashSet::new();
            set.insert(id1.clone());

            assert!(set.contains(&id2));
            assert!(!set.contains(&id3));
        }

        #[rstest]
        fn debug_format() {
            let id = RequestId::new("abc-123");
            let debug = format!("{:?}", id);
            assert!(debug.contains("abc-123"));
        }
    }

    mod request_id_layer {
        use super::*;

        #[rstest]
        fn new_creates_layer() {
            let _layer = RequestIdLayer::new();
        }

        #[rstest]
        fn default_creates_layer() {
            let _layer: RequestIdLayer = Default::default();
        }

        #[rstest]
        fn clone_layer() {
            let layer = RequestIdLayer::new();
            let _cloned = layer.clone();
        }

        #[rstest]
        fn debug_format() {
            let layer = RequestIdLayer::new();
            let debug = format!("{:?}", layer);
            assert!(debug.contains("RequestIdLayer"));
        }
    }
}
