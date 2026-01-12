use crate::helpers::{
    IntegrationTestContext, assert_error_response, assert_json_has_key, assert_json_string_eq,
};
use rstest::rstest;

// =============================================================================
// L1: Get Leaderboard Default Returns 200
// =============================================================================

#[rstest]
#[tokio::test]
async fn l1_get_leaderboard_default_returns_200() {
    let context = IntegrationTestContext::new().await;

    let response = context.client.get("/api/v1/leaderboard").await;

    assert_eq!(response.status_code(), 200);
    assert_json_has_key(&response.body, "type");
    assert_json_has_key(&response.body, "entries");
}

#[rstest]
#[tokio::test]
async fn l1_get_leaderboard_default_type_is_global() {
    let context = IntegrationTestContext::new().await;

    let response = context.client.get("/api/v1/leaderboard").await;

    assert_eq!(response.status_code(), 200);
    assert_json_string_eq(&response.body, "type", "global");
}

// =============================================================================
// L2: Get Leaderboard type=global
// =============================================================================

#[rstest]
#[tokio::test]
async fn l2_get_leaderboard_type_global_returns_200() {
    let context = IntegrationTestContext::new().await;

    let response = context.client.get("/api/v1/leaderboard?type=global").await;

    assert_eq!(response.status_code(), 200);
    assert_json_string_eq(&response.body, "type", "global");
}

// =============================================================================
// L3: Get Leaderboard type=daily
// =============================================================================

#[rstest]
#[tokio::test]
async fn l3_get_leaderboard_type_daily_returns_200() {
    let context = IntegrationTestContext::new().await;

    let response = context.client.get("/api/v1/leaderboard?type=daily").await;

    assert_eq!(response.status_code(), 200);
    assert_json_string_eq(&response.body, "type", "daily");
}

// =============================================================================
// L4: Get Leaderboard type=weekly
// =============================================================================

#[rstest]
#[tokio::test]
async fn l4_get_leaderboard_type_weekly_returns_200() {
    let context = IntegrationTestContext::new().await;

    let response = context.client.get("/api/v1/leaderboard?type=weekly").await;

    assert_eq!(response.status_code(), 200);
    assert_json_string_eq(&response.body, "type", "weekly");
}

// =============================================================================
// L5: Get Leaderboard with limit
// =============================================================================

#[rstest]
#[tokio::test]
async fn l5_get_leaderboard_with_limit_returns_200() {
    let context = IntegrationTestContext::new().await;

    let response = context.client.get("/api/v1/leaderboard?limit=50").await;

    assert_eq!(response.status_code(), 200);
}

// =============================================================================
// L6: Get Leaderboard limit=100 (Max Value)
// =============================================================================

#[rstest]
#[tokio::test]
async fn l6_get_leaderboard_limit_100_returns_200() {
    let context = IntegrationTestContext::new().await;

    let response = context.client.get("/api/v1/leaderboard?limit=100").await;

    assert_eq!(response.status_code(), 200);
}

// =============================================================================
// L7: Get Leaderboard limit=101 (Exceeds Max) Returns 400
// =============================================================================

#[rstest]
#[tokio::test]
async fn l7_get_leaderboard_limit_exceeds_max_returns_400() {
    let context = IntegrationTestContext::new().await;

    let response = context.client.get("/api/v1/leaderboard?limit=101").await;

    assert_eq!(response.status_code(), 400);
    assert_error_response(&response.body, "VALIDATION_ERROR");
}

// =============================================================================
// L8: Get Leaderboard limit=0 Returns Empty Array
// =============================================================================

#[rstest]
#[tokio::test]
async fn l8_get_leaderboard_limit_0_returns_200() {
    let context = IntegrationTestContext::new().await;

    let response = context.client.get("/api/v1/leaderboard?limit=0").await;

    assert_eq!(response.status_code(), 200);

    let entries = response.body["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 0);
}

// =============================================================================
// L9: Get Leaderboard type + limit Combination
// =============================================================================

#[rstest]
#[tokio::test]
async fn l9_get_leaderboard_type_and_limit_returns_200() {
    let context = IntegrationTestContext::new().await;

    let response = context
        .client
        .get("/api/v1/leaderboard?type=daily&limit=20")
        .await;

    assert_eq!(response.status_code(), 200);
    assert_json_string_eq(&response.body, "type", "daily");
}

// =============================================================================
// L10: Get Leaderboard Invalid Type Returns 400
// =============================================================================

#[rstest]
#[tokio::test]
async fn l10_get_leaderboard_invalid_type_returns_400() {
    let context = IntegrationTestContext::new().await;

    let response = context.client.get("/api/v1/leaderboard?type=invalid").await;

    assert_eq!(response.status_code(), 400);
}
