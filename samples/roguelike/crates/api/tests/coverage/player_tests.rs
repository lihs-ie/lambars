use crate::helpers::{IntegrationTestContext, assert_error_response};
use rstest::rstest;

// =============================================================================
// P1: Get Player for Non-Existent Game Returns 404
// =============================================================================

#[rstest]
#[tokio::test]
async fn p1_get_player_nonexistent_game_returns_404() {
    let context = IntegrationTestContext::new().await;

    let fake_id = uuid::Uuid::new_v4().to_string();
    let response = context
        .client
        .get(&format!("/api/v1/games/{}/player", fake_id))
        .await;

    assert_eq!(response.status_code(), 404);
    assert_error_response(&response.body, "GAMESESSION_NOT_FOUND");
}

// =============================================================================
// P2: Get Player Invalid UUID Returns 400
// =============================================================================

#[rstest]
#[tokio::test]
async fn p2_get_player_invalid_uuid_returns_400() {
    let context = IntegrationTestContext::new().await;

    let response = context
        .client
        .get("/api/v1/games/invalid-uuid/player")
        .await;

    assert_eq!(response.status_code(), 400);
    assert_error_response(&response.body, "VALIDATION_ERROR");
}

// =============================================================================
// P3: Get Inventory for Non-Existent Game Returns 404
// =============================================================================

#[rstest]
#[tokio::test]
async fn p3_get_inventory_nonexistent_game_returns_404() {
    let context = IntegrationTestContext::new().await;

    let fake_id = uuid::Uuid::new_v4().to_string();
    let response = context
        .client
        .get(&format!("/api/v1/games/{}/inventory", fake_id))
        .await;

    assert_eq!(response.status_code(), 404);
    assert_error_response(&response.body, "GAMESESSION_NOT_FOUND");
}

// =============================================================================
// P4: Get Inventory Invalid UUID Returns 400
// =============================================================================

#[rstest]
#[tokio::test]
async fn p4_get_inventory_invalid_uuid_returns_400() {
    let context = IntegrationTestContext::new().await;

    let response = context
        .client
        .get("/api/v1/games/invalid-uuid/inventory")
        .await;

    assert_eq!(response.status_code(), 400);
    assert_error_response(&response.body, "VALIDATION_ERROR");
}
