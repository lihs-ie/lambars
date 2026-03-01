//! Integration tests for GET /health endpoint.

use crate::common::*;
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn health_returns_200_with_status() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    let result = client.health().await;

    assert_success(&result);
    let response = result.unwrap();
    assert_eq!(response.status, "healthy");
}

#[rstest]
#[tokio::test]
async fn health_returns_version() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    let result = client.health().await;

    assert_success(&result);
    let response = result.unwrap();
    assert!(!response.version.is_empty(), "Version should not be empty");
    assert_eq!(response.version, "0.1.0");
}
