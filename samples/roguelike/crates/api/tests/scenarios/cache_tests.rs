use crate::helpers::{IntegrationTestContext, get_redis_ttl, invalidate_redis_cache, redis_key_exists};
use rstest::rstest;

// =============================================================================
// S3: Cache Hit/Miss Behavior
// =============================================================================

#[rstest]
#[tokio::test]
async fn s3_cache_populated_after_create() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("CacheTestHero").await;

    // Verify cache is populated after creation
    let exists = redis_key_exists(&mut context.redis_connection, &game_id).await;
    assert!(exists, "Cache should be populated after game creation");
}

#[rstest]
#[tokio::test]
async fn s3_cache_miss_repopulates() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("CacheMissHero").await;

    // Verify cache exists
    assert!(redis_key_exists(&mut context.redis_connection, &game_id).await);

    // Manually invalidate cache
    invalidate_redis_cache(&mut context.redis_connection, &game_id).await;

    // Verify cache is gone
    assert!(!redis_key_exists(&mut context.redis_connection, &game_id).await);

    // Get game (should miss cache, hit MySQL, repopulate cache)
    let response = context
        .client
        .get(&format!("/api/v1/games/{}", game_id))
        .await;
    assert_eq!(response.status_code(), 200);

    // Verify cache was repopulated
    assert!(
        redis_key_exists(&mut context.redis_connection, &game_id).await,
        "Cache should be repopulated after cache miss"
    );
}

#[rstest]
#[tokio::test]
async fn s3_cache_hit_returns_same_data() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("CacheHitHero").await;

    // First request
    let response1 = context
        .client
        .get(&format!("/api/v1/games/{}", game_id))
        .await;
    assert_eq!(response1.status_code(), 200);

    // Second request (should hit cache)
    let response2 = context
        .client
        .get(&format!("/api/v1/games/{}", game_id))
        .await;
    assert_eq!(response2.status_code(), 200);

    // Note: player_id is currently generated per request, so we compare other stable fields
    assert_eq!(
        response1.body["game_id"], response2.body["game_id"],
        "game_id should be consistent"
    );
    assert_eq!(
        response1.body["status"], response2.body["status"],
        "status should be consistent"
    );
    assert_eq!(
        response1.body["turn_count"], response2.body["turn_count"],
        "turn_count should be consistent"
    );
    assert_eq!(
        response1.body["floor"], response2.body["floor"],
        "floor should be consistent"
    );
}

#[rstest]
#[tokio::test]
async fn s3_cache_ttl_is_set() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("TTLTestHero").await;

    // Check TTL
    let ttl = get_redis_ttl(&mut context.redis_connection, &game_id).await;

    // TTL should be positive and within expected range (default is 300 seconds)
    assert!(ttl > 0, "TTL should be positive");
    assert!(ttl <= 300, "TTL should be at most 300 seconds");
}
