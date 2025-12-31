//! HTTP リクエスト/レスポンス型
//!
//! API レイヤーで使用する HTTP の抽象型を定義する。

// =============================================================================
// HttpRequest (REQ-087)
// =============================================================================

/// HTTP リクエストの抽象型
///
/// リクエストボディを保持するシンプルな構造体。
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
    /// リクエストボディ
    body: String,
}

impl HttpRequest {
    /// 新しい `HttpRequest` を生成する
    ///
    /// # Arguments
    ///
    /// * `body` - リクエストボディ
    ///
    /// # Returns
    ///
    /// `HttpRequest` インスタンス
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

    /// リクエストボディへの参照を返す
    ///
    /// # Returns
    ///
    /// リクエストボディ文字列への参照
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

/// HTTP レスポンスの抽象型
///
/// ステータスコードとレスポンスボディを保持する構造体。
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
    /// HTTP ステータスコード
    status_code: u16,
    /// レスポンスボディ
    body: String,
}

impl HttpResponse {
    /// 新しい `HttpResponse` を生成する
    ///
    /// # Arguments
    ///
    /// * `status_code` - HTTP ステータスコード
    /// * `body` - レスポンスボディ
    ///
    /// # Returns
    ///
    /// `HttpResponse` インスタンス
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

    /// 200 OK レスポンスを生成する
    ///
    /// # Arguments
    ///
    /// * `body` - レスポンスボディ
    ///
    /// # Returns
    ///
    /// ステータスコード 200 の `HttpResponse`
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

    /// 400 Bad Request レスポンスを生成する
    ///
    /// # Arguments
    ///
    /// * `body` - レスポンスボディ（エラーメッセージ）
    ///
    /// # Returns
    ///
    /// ステータスコード 400 の `HttpResponse`
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

    /// 500 Internal Server Error レスポンスを生成する
    ///
    /// # Arguments
    ///
    /// * `body` - レスポンスボディ（エラーメッセージ）
    ///
    /// # Returns
    ///
    /// ステータスコード 500 の `HttpResponse`
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

    /// HTTP ステータスコードを返す
    ///
    /// # Returns
    ///
    /// HTTP ステータスコード
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

    /// レスポンスボディへの参照を返す
    ///
    /// # Returns
    ///
    /// レスポンスボディ文字列への参照
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

    /// 成功レスポンス（2xx）かどうかを返す
    ///
    /// # Returns
    ///
    /// ステータスコードが 200-299 の場合 `true`
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
