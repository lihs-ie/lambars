use reqwest::{Client, Response, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value as JsonValue;

pub struct TestClient {
    base_url: String,
    http_client: Client,
}

impl TestClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            http_client: Client::new(),
        }
    }

    pub async fn get(&self, path: &str) -> TestResponse {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .expect("Failed to send GET request");
        TestResponse::from_response(response).await
    }

    pub async fn post<T: Serialize>(&self, path: &str, body: &T) -> TestResponse {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .http_client
            .post(&url)
            .json(body)
            .send()
            .await
            .expect("Failed to send POST request");
        TestResponse::from_response(response).await
    }

    pub async fn post_raw(&self, path: &str, body: &str) -> TestResponse {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .await
            .expect("Failed to send POST request");
        TestResponse::from_response(response).await
    }
}

pub struct TestResponse {
    pub status: StatusCode,
    pub body: JsonValue,
    pub headers: reqwest::header::HeaderMap,
}

impl TestResponse {
    async fn from_response(response: Response) -> Self {
        let status = response.status();
        let headers = response.headers().clone();
        let body = response
            .json::<JsonValue>()
            .await
            .unwrap_or(JsonValue::Null);
        Self {
            status,
            body,
            headers,
        }
    }

    pub fn status_code(&self) -> u16 {
        self.status.as_u16()
    }

    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    pub fn json<T: DeserializeOwned>(&self) -> T {
        serde_json::from_value(self.body.clone()).expect("Failed to deserialize response body")
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    pub fn has_header(&self, name: &str) -> bool {
        self.headers.contains_key(name)
    }
}
