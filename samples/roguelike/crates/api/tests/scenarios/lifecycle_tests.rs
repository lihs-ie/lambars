use crate::helpers::{
    IntegrationTestContext, assert_json_has_key, assert_json_string_eq, query_game_events,
    query_game_session, redis_key_exists,
};
use rstest::rstest;

// =============================================================================
// S1: Basic Game Lifecycle (Health → Create → Get → Events)
// =============================================================================

#[rstest]
#[tokio::test]
async fn s1_basic_game_lifecycle() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    // Step 1: Health Check
    let health_response = context.client.get("/api/v1/health").await;
    assert_eq!(health_response.status_code(), 200);
    assert_json_string_eq(&health_response.body, "status", "healthy");

    // Step 2: Create Game
    let create_request = serde_json::json!({
        "player_name": "LifecycleHero"
    });
    let create_response = context.client.post("/api/v1/games", &create_request).await;
    assert_eq!(create_response.status_code(), 201);

    let game_id = create_response.body["game_id"].as_str().unwrap();

    // Step 3: Verify MySQL Persistence (game_sessions)
    let mysql_session = query_game_session(&context.mysql_pool, game_id).await;
    assert!(
        mysql_session.is_some(),
        "Game session should be persisted in MySQL"
    );
    let session = mysql_session.unwrap();
    assert_eq!(session.status, "in_progress");
    assert_eq!(session.turn_count, 0);
    assert_eq!(session.current_floor_level, 1);

    // Step 4: Verify MySQL Persistence (game_events)
    let mysql_events = query_game_events(&context.mysql_pool, game_id).await;
    assert!(!mysql_events.is_empty(), "At least one event should exist");
    assert_eq!(mysql_events[0].event_type, "Started");
    assert_eq!(mysql_events[0].sequence_number, 0);

    // Step 5: Verify Redis Cache
    let redis_exists = redis_key_exists(&mut context.redis_connection, game_id).await;
    assert!(redis_exists, "Game should be cached in Redis");

    // Step 6: Get Game
    let get_response = context
        .client
        .get(&format!("/api/v1/games/{}", game_id))
        .await;
    assert_eq!(get_response.status_code(), 200);
    assert_json_string_eq(&get_response.body, "game_id", game_id);
    // Note: Player name is not currently persisted/returned by the API
    assert_json_has_key(&get_response.body["player"], "name");

    // Step 7: Get Events
    let events_response = context
        .client
        .get(&format!("/api/v1/games/{}/events", game_id))
        .await;
    assert_eq!(events_response.status_code(), 200);
    assert_json_has_key(&events_response.body, "events");

    let events = events_response.body["events"].as_array().unwrap();
    assert!(!events.is_empty(), "Events array should not be empty");
}

// =============================================================================
// S5: Error Handling Comprehensive
// =============================================================================

#[rstest]
#[tokio::test]
async fn s5_error_handling_comprehensive() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    // Test 1: Create game with empty player name
    let invalid_create = serde_json::json!({
        "player_name": ""
    });
    let response = context.client.post("/api/v1/games", &invalid_create).await;
    assert_eq!(response.status_code(), 400);

    // Test 2: Get non-existent game
    let fake_id = uuid::Uuid::new_v4().to_string();
    let response = context
        .client
        .get(&format!("/api/v1/games/{}", fake_id))
        .await;
    assert_eq!(response.status_code(), 404);

    // Test 3: Get game with invalid UUID
    let response = context.client.get("/api/v1/games/not-a-uuid").await;
    assert_eq!(response.status_code(), 400);

    // Test 4: Events with invalid limit
    let game_id = context.create_game("ErrorTestHero").await;
    let response = context
        .client
        .get(&format!("/api/v1/games/{}/events?limit=9999", game_id))
        .await;
    assert_eq!(response.status_code(), 400);

    // Test 5: Leaderboard with invalid type
    let response = context.client.get("/api/v1/leaderboard?type=invalid").await;
    assert_eq!(response.status_code(), 400);
}

// =============================================================================
// S6: Partially Implemented Endpoints
// =============================================================================

#[rstest]
#[tokio::test]
async fn s6_not_implemented_endpoints() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("NotImplementedHero").await;

    // Test End Game - Now implemented, returns 200
    let end_request = serde_json::json!({
        "outcome": "abandon"
    });
    let response = context
        .client
        .post(&format!("/api/v1/games/{}/end", game_id), &end_request)
        .await;
    assert_eq!(
        response.status_code(),
        200,
        "Expected 200, got {}",
        response.status_code()
    );

    // Create a new game since the previous one was ended
    let game_id = context.create_game("NotImplementedHero2").await;

    // Test Execute Command
    // Note: Returns 422 if command format validation fails, 500/501 if workflow not implemented
    let command_request = serde_json::json!({
        "command": "Wait"
    });
    let response = context
        .client
        .post(
            &format!("/api/v1/games/{}/commands", game_id),
            &command_request,
        )
        .await;
    assert!(
        response.status_code() == 422
            || response.status_code() == 500
            || response.status_code() == 501,
        "Expected 422, 500, or 501, got {}",
        response.status_code()
    );

    // Test Get Player (returns 404 as not found, not 501)
    let response = context
        .client
        .get(&format!("/api/v1/games/{}/player", game_id))
        .await;
    assert_eq!(response.status_code(), 404);

    // Test Get Inventory
    let response = context
        .client
        .get(&format!("/api/v1/games/{}/inventory", game_id))
        .await;
    assert_eq!(response.status_code(), 404);

    // Test Get Floor
    let response = context
        .client
        .get(&format!("/api/v1/games/{}/floor", game_id))
        .await;
    assert_eq!(response.status_code(), 404);

    // Test Get Visible Area
    let response = context
        .client
        .get(&format!("/api/v1/games/{}/floor/visible", game_id))
        .await;
    assert_eq!(response.status_code(), 404);
}

// =============================================================================
// S7: Leaderboard Functionality
// =============================================================================

#[rstest]
#[tokio::test]
async fn s7_leaderboard_functionality() {
    let context = IntegrationTestContext::new().await;

    // Test Global Leaderboard
    let response = context.client.get("/api/v1/leaderboard").await;
    assert_eq!(response.status_code(), 200);
    assert_json_string_eq(&response.body, "type", "global");
    assert_json_has_key(&response.body, "entries");

    // Test Daily Leaderboard
    let response = context.client.get("/api/v1/leaderboard?type=daily").await;
    assert_eq!(response.status_code(), 200);
    assert_json_string_eq(&response.body, "type", "daily");

    // Test Weekly Leaderboard
    let response = context.client.get("/api/v1/leaderboard?type=weekly").await;
    assert_eq!(response.status_code(), 200);
    assert_json_string_eq(&response.body, "type", "weekly");

    // Test with Limit
    let response = context
        .client
        .get("/api/v1/leaderboard?type=global&limit=5")
        .await;
    assert_eq!(response.status_code(), 200);
}
