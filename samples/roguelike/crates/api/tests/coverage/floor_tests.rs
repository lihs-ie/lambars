use crate::helpers::{IntegrationTestContext, assert_error_response};
use rstest::rstest;

// =============================================================================
// F1: Get Floor Default Returns 404 (Not Implemented)
// =============================================================================

#[rstest]
#[tokio::test]
async fn f1_get_floor_default_returns_404() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let response = context
        .client
        .get(&format!("/api/v1/games/{}/floor", game_id))
        .await;

    assert_eq!(response.status_code(), 404);
    assert_error_response(&response.body, "GAMESESSION_NOT_FOUND");
}

// =============================================================================
// F2: Get Floor with include_fog=true Returns 404
// =============================================================================

#[rstest]
#[tokio::test]
async fn f2_get_floor_include_fog_true_returns_404() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let response = context
        .client
        .get(&format!("/api/v1/games/{}/floor?include_fog=true", game_id))
        .await;

    assert_eq!(response.status_code(), 404);
}

// =============================================================================
// F3: Get Floor with include_fog=false Returns 404
// =============================================================================

#[rstest]
#[tokio::test]
async fn f3_get_floor_include_fog_false_returns_404() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let response = context
        .client
        .get(&format!(
            "/api/v1/games/{}/floor?include_fog=false",
            game_id
        ))
        .await;

    assert_eq!(response.status_code(), 404);
}

// =============================================================================
// F4: Get Floor Invalid UUID Returns 400
// =============================================================================

#[rstest]
#[tokio::test]
async fn f4_get_floor_invalid_uuid_returns_400() {
    let context = IntegrationTestContext::new().await;

    let response = context.client.get("/api/v1/games/invalid-uuid/floor").await;

    assert_eq!(response.status_code(), 400);
    assert_error_response(&response.body, "VALIDATION_ERROR");
}

// =============================================================================
// F5: Get Visible Area Returns 404 (Not Implemented)
// =============================================================================

#[rstest]
#[tokio::test]
async fn f5_get_visible_area_returns_404() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let response = context
        .client
        .get(&format!("/api/v1/games/{}/floor/visible", game_id))
        .await;

    assert_eq!(response.status_code(), 404);
    assert_error_response(&response.body, "GAMESESSION_NOT_FOUND");
}

// =============================================================================
// F6: Get Visible Area Invalid UUID Returns 400
// =============================================================================

#[rstest]
#[tokio::test]
async fn f6_get_visible_area_invalid_uuid_returns_400() {
    let context = IntegrationTestContext::new().await;

    let response = context
        .client
        .get("/api/v1/games/invalid-uuid/floor/visible")
        .await;

    assert_eq!(response.status_code(), 400);
    assert_error_response(&response.body, "VALIDATION_ERROR");
}
