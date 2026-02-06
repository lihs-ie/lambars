//! HTTP request/response types
//!
//! Defines abstract HTTP types used in the API layer.

// =============================================================================
// HttpRequest (REQ-087)
// =============================================================================

/// Abstract HTTP request type
///
/// A simple struct that holds the request body.
///
/// # Examples
///
/// ```
/// use order_taking_sample::api::HttpRequest;
///
/// let request = HttpRequest::new(r#"{"order_id": "order-001"}"#.to_string());
/// assert!(request.body().contains("order_id"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpRequest {
    /// Request body
    body: String,
}

impl HttpRequest {
    /// Creates a new `HttpRequest`
    ///
    /// # Arguments
    ///
    /// * `body` - Request body
    ///
    /// # Returns
    ///
    /// An `HttpRequest` instance
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::api::HttpRequest;
    ///
    /// let request = HttpRequest::new(r#"{"key": "value"}"#.to_string());
    /// ```
    #[must_use]
    pub const fn new(body: String) -> Self {
        Self { body }
    }

    /// Returns a reference to Request body
    ///
    /// # Returns
    ///
    /// A reference to the request body string
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::api::HttpRequest;
    ///
    /// let request = HttpRequest::new("test body".to_string());
    /// assert_eq!(request.body(), "test body");
    /// ```
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
}

// =============================================================================
// HttpResponse (REQ-088)
// =============================================================================

/// Abstract HTTP response type
///
/// A struct that holds a status code and response body.
///
/// # Examples
///
/// ```
/// use order_taking_sample::api::HttpResponse;
///
/// let response = HttpResponse::ok(r#"{"success": true}"#.to_string());
/// assert_eq!(response.status_code(), 200);
/// assert!(response.is_success());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpResponse {
    /// HTTP status code
    status_code: u16,
    /// Response body
    body: String,
}

impl HttpResponse {
    /// Creates a new `HttpResponse`
    ///
    /// # Arguments
    ///
    /// * `status_code` - HTTP status code
    /// * `body` - Response body
    ///
    /// # Returns
    ///
    /// An `HttpResponse` instance
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::api::HttpResponse;
    ///
    /// let response = HttpResponse::new(201, r#"{"id": 123}"#.to_string());
    /// assert_eq!(response.status_code(), 201);
    /// ```
    #[must_use]
    pub const fn new(status_code: u16, body: String) -> Self {
        Self { status_code, body }
    }

    /// Creates a 200 OK response
    ///
    /// # Arguments
    ///
    /// * `body` - Response body
    ///
    /// # Returns
    ///
    /// An `HttpResponse` with status code 200
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::api::HttpResponse;
    ///
    /// let response = HttpResponse::ok(r#"{"success": true}"#.to_string());
    /// assert_eq!(response.status_code(), 200);
    /// ```
    #[must_use]
    pub const fn ok(body: String) -> Self {
        Self::new(200, body)
    }

    /// Creates a 400 Bad Request response
    ///
    /// # Arguments
    ///
    /// * `body` - Response body (error message)
    ///
    /// # Returns
    ///
    /// An `HttpResponse` with status code 400
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::api::HttpResponse;
    ///
    /// let response = HttpResponse::bad_request(r#"{"error": "Invalid input"}"#.to_string());
    /// assert_eq!(response.status_code(), 400);
    /// ```
    #[must_use]
    pub const fn bad_request(body: String) -> Self {
        Self::new(400, body)
    }

    /// Creates a 500 Internal Server Error response
    ///
    /// # Arguments
    ///
    /// * `body` - Response body (error message)
    ///
    /// # Returns
    ///
    /// An `HttpResponse` with status code 500
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::api::HttpResponse;
    ///
    /// let response = HttpResponse::internal_server_error("Server error".to_string());
    /// assert_eq!(response.status_code(), 500);
    /// ```
    #[must_use]
    pub const fn internal_server_error(body: String) -> Self {
        Self::new(500, body)
    }

    /// Returns the HTTP status code
    ///
    /// # Returns
    ///
    /// HTTP status code
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::api::HttpResponse;
    ///
    /// let response = HttpResponse::new(404, "Not Found".to_string());
    /// assert_eq!(response.status_code(), 404);
    /// ```
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        self.status_code
    }

    /// Returns a reference to Response body
    ///
    /// # Returns
    ///
    /// A reference to the response body string
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::api::HttpResponse;
    ///
    /// let response = HttpResponse::ok("Success".to_string());
    /// assert_eq!(response.body(), "Success");
    /// ```
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Returns whether the response is a success (2xx)
    ///
    /// # Returns
    ///
    /// `true` if the status code is in the range 200-299
    ///
    /// # Examples
    ///
    /// ```
    /// use order_taking_sample::api::HttpResponse;
    ///
    /// let success = HttpResponse::ok("OK".to_string());
    /// assert!(success.is_success());
    ///
    /// let error = HttpResponse::bad_request("Error".to_string());
    /// assert!(!error.is_success());
    /// ```
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.status_code >= 200 && self.status_code < 300
    }
}
