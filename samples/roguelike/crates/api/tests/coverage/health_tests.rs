use crate::helpers::{IntegrationTestContext, assert_json_has_key, assert_json_string_eq};
use rstest::rstest;

// =============================================================================
// H1: Health Check Success
// =============================================================================

#[rstest]
#[tokio::test]
async fn h1_health_check_returns_200_ok() {
    let context = IntegrationTestContext::new().await;

    let response = context.client.get("/api/v1/health").await;

    assert_eq!(response.status_code(), 200);
    assert_json_has_key(&response.body, "status");
    assert_json_has_key(&response.body, "version");
    assert_json_has_key(&response.body, "components");
}

#[rstest]
#[tokio::test]
async fn h1_health_check_returns_healthy_status() {
    let context = IntegrationTestContext::new().await;

    let response = context.client.get("/api/v1/health").await;

    assert_eq!(response.status_code(), 200);
    assert_json_string_eq(&response.body, "status", "healthy");
}

#[rstest]
#[tokio::test]
async fn h1_health_check_returns_component_statuses() {
    let context = IntegrationTestContext::new().await;

    let response = context.client.get("/api/v1/health").await;

    assert_eq!(response.status_code(), 200);

    let components = &response.body["components"];
    assert_json_has_key(components, "database");
    assert_json_has_key(components, "cache");
    assert_json_string_eq(components, "database", "up");
    assert_json_string_eq(components, "cache", "up");
}

// =============================================================================
// H2: Request ID Header Exists
// =============================================================================

#[rstest]
#[tokio::test]
async fn h2_health_check_returns_request_id_header() {
    let context = IntegrationTestContext::new().await;

    let response = context.client.get("/api/v1/health").await;

    assert!(
        response.has_header("x-request-id"),
        "Expected x-request-id header to be present"
    );

    let request_id = response.header("x-request-id").unwrap();
    assert!(
        !request_id.is_empty(),
        "Expected x-request-id header to have a value"
    );
}

// =============================================================================
// H3: Response Time Header Exists
// =============================================================================

#[rstest]
#[tokio::test]
async fn h3_health_check_returns_response_time_header() {
    let context = IntegrationTestContext::new().await;

    let response = context.client.get("/api/v1/health").await;

    assert!(
        response.has_header("x-response-time"),
        "Expected x-response-time header to be present"
    );

    let response_time = response.header("x-response-time").unwrap();
    assert!(
        !response_time.is_empty(),
        "Expected x-response-time header to have a value"
    );
}
