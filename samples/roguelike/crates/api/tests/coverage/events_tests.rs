use crate::helpers::{
    IntegrationTestContext, assert_error_response, assert_json_array_len, assert_json_has_key,
    count_game_events,
};
use rstest::rstest;

// =============================================================================
// E1: Get Events Default Parameters Returns 200
// =============================================================================

#[rstest]
#[tokio::test]
async fn e1_get_events_default_returns_200() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let response = context
        .client
        .get(&format!("/api/v1/games/{}/events", game_id))
        .await;

    assert_eq!(response.status_code(), 200);
    assert_json_has_key(&response.body, "events");
    assert_json_has_key(&response.body, "next_sequence");
    assert_json_has_key(&response.body, "has_more");
}

#[rstest]
#[tokio::test]
async fn e1_get_events_returns_started_event() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let response = context
        .client
        .get(&format!("/api/v1/games/{}/events", game_id))
        .await;

    assert_eq!(response.status_code(), 200);

    let events = response.body["events"].as_array().unwrap();
    assert!(!events.is_empty(), "Expected at least one event");

    let first_event = &events[0];
    assert_json_has_key(first_event, "sequence");
    assert_json_has_key(first_event, "type");
    assert_json_has_key(first_event, "data");
    assert_json_has_key(first_event, "occurred_at");
}

// =============================================================================
// E2: Get Events with since Parameter
// =============================================================================

#[rstest]
#[tokio::test]
async fn e2_get_events_with_since_returns_200() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let response = context
        .client
        .get(&format!("/api/v1/games/{}/events?since=10", game_id))
        .await;

    assert_eq!(response.status_code(), 200);
}

// =============================================================================
// E3: Get Events with limit Parameter
// =============================================================================

#[rstest]
#[tokio::test]
async fn e3_get_events_with_limit_returns_200() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let response = context
        .client
        .get(&format!("/api/v1/games/{}/events?limit=50", game_id))
        .await;

    assert_eq!(response.status_code(), 200);
}

// =============================================================================
// E4: Get Events with since + limit
// =============================================================================

#[rstest]
#[tokio::test]
async fn e4_get_events_with_since_and_limit_returns_200() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let response = context
        .client
        .get(&format!(
            "/api/v1/games/{}/events?since=0&limit=20",
            game_id
        ))
        .await;

    assert_eq!(response.status_code(), 200);
}

// =============================================================================
// E5: Get Events with limit=1000 (Max Value)
// =============================================================================

#[rstest]
#[tokio::test]
async fn e5_get_events_limit_1000_returns_200() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let response = context
        .client
        .get(&format!("/api/v1/games/{}/events?limit=1000", game_id))
        .await;

    assert_eq!(response.status_code(), 200);
}

// =============================================================================
// E6: Get Events with limit=1001 (Exceeds Max) Returns 400
// =============================================================================

#[rstest]
#[tokio::test]
async fn e6_get_events_limit_exceeds_max_returns_400() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let response = context
        .client
        .get(&format!("/api/v1/games/{}/events?limit=1001", game_id))
        .await;

    assert_eq!(response.status_code(), 400);
    assert_error_response(&response.body, "VALIDATION_ERROR");
}

// =============================================================================
// E7: Get Events with limit=0 Returns Empty Array
// =============================================================================

#[rstest]
#[tokio::test]
async fn e7_get_events_limit_0_returns_empty() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let response = context
        .client
        .get(&format!("/api/v1/games/{}/events?limit=0", game_id))
        .await;

    assert_eq!(response.status_code(), 200);
    assert_json_array_len(&response.body, "events", 0);
}

// =============================================================================
// E8: Get Events Invalid UUID Returns 400
// =============================================================================

#[rstest]
#[tokio::test]
async fn e8_get_events_invalid_uuid_returns_400() {
    let context = IntegrationTestContext::new().await;

    let response = context
        .client
        .get("/api/v1/games/invalid-uuid/events")
        .await;

    assert_eq!(response.status_code(), 400);
    assert_error_response(&response.body, "VALIDATION_ERROR");
}

// =============================================================================
// E9: Get Events for Non-Existent Game Returns 200 with Empty Array
// =============================================================================

#[rstest]
#[tokio::test]
async fn e9_get_events_nonexistent_game_returns_200_empty() {
    let context = IntegrationTestContext::new().await;

    let fake_id = uuid::Uuid::new_v4().to_string();
    let response = context
        .client
        .get(&format!("/api/v1/games/{}/events", fake_id))
        .await;

    assert_eq!(response.status_code(), 200);
    assert_json_array_len(&response.body, "events", 0);
}

// =============================================================================
// E10: has_more Flag Verification
// =============================================================================

#[rstest]
#[tokio::test]
async fn e10_get_events_has_more_is_false_when_all_returned() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let response = context
        .client
        .get(&format!("/api/v1/games/{}/events", game_id))
        .await;

    assert_eq!(response.status_code(), 200);

    let has_more = response.body["has_more"].as_bool().unwrap();
    assert!(
        !has_more,
        "has_more should be false when all events are returned"
    );
}

// =============================================================================
// E11: next_sequence Verification
// =============================================================================

#[rstest]
#[tokio::test]
async fn e11_get_events_next_sequence_matches_event_count() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let response = context
        .client
        .get(&format!("/api/v1/games/{}/events", game_id))
        .await;

    assert_eq!(response.status_code(), 200);

    let events = response.body["events"].as_array().unwrap();
    let next_sequence = response.body["next_sequence"].as_u64().unwrap();

    let event_count = count_game_events(&context.mysql_pool, &game_id).await;

    assert_eq!(
        next_sequence, event_count as u64,
        "next_sequence should equal the total number of events"
    );
    assert_eq!(events.len(), event_count as usize);
}
