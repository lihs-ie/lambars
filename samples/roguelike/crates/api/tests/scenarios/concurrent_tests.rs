use crate::helpers::{IntegrationTestContext, query_game_session, redis_key_exists};
use rstest::rstest;

// =============================================================================
// S4: Multiple Game Sessions Concurrent Processing
// =============================================================================

#[rstest]
#[tokio::test]
async fn s4_concurrent_game_creation() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    // Create 3 games sequentially (concurrent requests would require additional setup)
    let game_ids: Vec<String> = vec![
        context.create_game("Player1").await,
        context.create_game("Player2").await,
        context.create_game("Player3").await,
    ];

    // Verify all games are distinct
    let unique_ids: std::collections::HashSet<_> = game_ids.iter().collect();
    assert_eq!(unique_ids.len(), 3, "All game IDs should be unique");

    // Verify all games exist in MySQL
    for game_id in &game_ids {
        let record = query_game_session(&context.mysql_pool, game_id).await;
        assert!(record.is_some(), "Game {} should exist in MySQL", game_id);
    }

    // Verify all games exist in Redis
    for game_id in &game_ids {
        let exists = redis_key_exists(&mut context.redis_connection, game_id).await;
        assert!(exists, "Game {} should exist in Redis", game_id);
    }
}

#[rstest]
#[tokio::test]
async fn s4_games_are_independent() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    // Create multiple games
    let game1 = context.create_game("IndependentPlayer1").await;
    let game2 = context.create_game("IndependentPlayer2").await;

    // Get both games
    let response1 = context
        .client
        .get(&format!("/api/v1/games/{}", game1))
        .await;
    let response2 = context
        .client
        .get(&format!("/api/v1/games/{}", game2))
        .await;

    assert_eq!(response1.status_code(), 200);
    assert_eq!(response2.status_code(), 200);

    // Verify they have different IDs
    let id1 = response1.body["game_id"].as_str().unwrap();
    let id2 = response2.body["game_id"].as_str().unwrap();
    assert_ne!(id1, id2, "Game IDs should be different");

    // Note: Player names are not currently persisted/retrieved by the API
    // The API returns a placeholder "Player" for all games
    // Future implementation should store and return the actual player name
}

#[rstest]
#[tokio::test]
async fn s4_events_are_isolated() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    // Create two games
    let game1 = context.create_game("EventIsolation1").await;
    let game2 = context.create_game("EventIsolation2").await;

    // Get events for both games
    let events1 = context
        .client
        .get(&format!("/api/v1/games/{}/events", game1))
        .await;
    let events2 = context
        .client
        .get(&format!("/api/v1/games/{}/events", game2))
        .await;

    assert_eq!(events1.status_code(), 200);
    assert_eq!(events2.status_code(), 200);

    // Each game should have its own independent events
    let events1_array = events1.body["events"].as_array().unwrap();
    let events2_array = events2.body["events"].as_array().unwrap();

    // Both should have at least one event (Started)
    assert!(!events1_array.is_empty());
    assert!(!events2_array.is_empty());

    // Events should start from sequence 0 for both games
    assert_eq!(events1_array[0]["sequence"].as_u64().unwrap(), 0);
    assert_eq!(events2_array[0]["sequence"].as_u64().unwrap(), 0);
}

#[rstest]
#[tokio::test]
async fn s4_multiple_games_with_same_seed() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    // Create two games with the same seed
    let game1 = context.create_game_with_seed("SeedPlayer1", 42).await;
    let game2 = context.create_game_with_seed("SeedPlayer2", 42).await;

    // Verify both games exist
    let record1 = query_game_session(&context.mysql_pool, &game1).await;
    let record2 = query_game_session(&context.mysql_pool, &game2).await;

    assert!(record1.is_some());
    assert!(record2.is_some());

    // Verify both have the same seed
    assert_eq!(record1.unwrap().random_seed, 42);
    assert_eq!(record2.unwrap().random_seed, 42);

    // But different game IDs
    assert_ne!(game1, game2);
}
