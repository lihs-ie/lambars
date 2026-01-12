use std::time::Duration;

use redis::AsyncCommands;
use roguelike_domain::game_session::GameIdentifier;
use roguelike_infrastructure::adapters::redis::{
    CachedGameSession, RedisConfig, RedisConnectionFactory, RedisSessionCache,
};
use roguelike_workflow::ports::SessionCache;
use rstest::rstest;
use uuid::Uuid;

const REDIS_URL: &str = "redis://localhost:6379";

// =============================================================================
// Connection Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_redis_connection() {
    let config = RedisConfig::with_url(REDIS_URL).with_key_prefix("test:");

    let connection =
        RedisConnectionFactory::create_client(&config).expect("Failed to create Redis client");

    // Get an async connection and test with PING
    let mut async_connection = connection
        .get_async_connection()
        .await
        .expect("Failed to get async connection");

    let pong: String = redis::cmd("PING")
        .query_async(&mut async_connection)
        .await
        .expect("Failed to execute PING");

    assert_eq!(pong, "PONG");
}

#[rstest]
#[tokio::test]
async fn test_redis_connection_key_prefix() {
    let config = RedisConfig::with_url(REDIS_URL).with_key_prefix("integration-test:");

    let connection =
        RedisConnectionFactory::create_client(&config).expect("Failed to create Redis client");

    // Verify the key formatting works correctly
    let formatted_key = connection.format_key("session:abc");
    assert_eq!(formatted_key, "integration-test:session:abc");
}

// =============================================================================
// Session Cache Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_session_cache_set_get_invalidate() {
    // Use a unique prefix for this test to avoid conflicts
    let test_prefix = format!("test:integration:{}:", Uuid::new_v4());
    let config = RedisConfig::with_url(REDIS_URL).with_key_prefix(&test_prefix);

    let connection =
        RedisConnectionFactory::create_client(&config).expect("Failed to create Redis client");
    let cache = RedisSessionCache::new(connection);

    let game_identifier = GameIdentifier::new();
    let session = CachedGameSession {
        game_identifier: game_identifier.to_string(),
        player_identifier: Uuid::new_v4().to_string(),
        current_floor_level: 5,
        turn_count: 150,
        status: "InProgress".to_string(),
        random_seed: 42,
        event_sequence: 75,
    };

    // Set the session in cache with a 60-second TTL
    cache
        .set(&game_identifier, &session, Duration::from_secs(60))
        .run_async()
        .await;

    // Get the session from cache
    let found = cache.get(&game_identifier).run_async().await;

    assert!(found.is_some());
    let cached_session = found.unwrap();
    assert_eq!(cached_session.game_identifier, session.game_identifier);
    assert_eq!(cached_session.player_identifier, session.player_identifier);
    assert_eq!(
        cached_session.current_floor_level,
        session.current_floor_level
    );
    assert_eq!(cached_session.turn_count, session.turn_count);
    assert_eq!(cached_session.status, session.status);
    assert_eq!(cached_session.random_seed, session.random_seed);
    assert_eq!(cached_session.event_sequence, session.event_sequence);

    // Invalidate the cache entry
    cache.invalidate(&game_identifier).run_async().await;

    // Verify the entry is gone
    let not_found = cache.get(&game_identifier).run_async().await;
    assert!(not_found.is_none());
}

#[rstest]
#[tokio::test]
async fn test_session_cache_get_not_found() {
    let test_prefix = format!("test:integration:{}:", Uuid::new_v4());
    let config = RedisConfig::with_url(REDIS_URL).with_key_prefix(&test_prefix);

    let connection =
        RedisConnectionFactory::create_client(&config).expect("Failed to create Redis client");
    let cache: RedisSessionCache<CachedGameSession> = RedisSessionCache::new(connection);

    let nonexistent_identifier = GameIdentifier::new();
    let result = cache.get(&nonexistent_identifier).run_async().await;

    assert!(result.is_none());
}

#[rstest]
#[tokio::test]
async fn test_session_cache_invalidate_nonexistent() {
    let test_prefix = format!("test:integration:{}:", Uuid::new_v4());
    let config = RedisConfig::with_url(REDIS_URL).with_key_prefix(&test_prefix);

    let connection =
        RedisConnectionFactory::create_client(&config).expect("Failed to create Redis client");
    let cache: RedisSessionCache<CachedGameSession> = RedisSessionCache::new(connection);

    let nonexistent_identifier = GameIdentifier::new();

    // This should not panic or cause an error
    cache.invalidate(&nonexistent_identifier).run_async().await;
}

#[rstest]
#[tokio::test]
async fn test_session_cache_update() {
    let test_prefix = format!("test:integration:{}:", Uuid::new_v4());
    let config = RedisConfig::with_url(REDIS_URL).with_key_prefix(&test_prefix);

    let connection =
        RedisConnectionFactory::create_client(&config).expect("Failed to create Redis client");
    let cache = RedisSessionCache::new(connection);

    let game_identifier = GameIdentifier::new();

    // Set initial session
    let initial_session = CachedGameSession {
        game_identifier: game_identifier.to_string(),
        player_identifier: Uuid::new_v4().to_string(),
        current_floor_level: 1,
        turn_count: 10,
        status: "InProgress".to_string(),
        random_seed: 42,
        event_sequence: 5,
    };

    cache
        .set(&game_identifier, &initial_session, Duration::from_secs(60))
        .run_async()
        .await;

    // Update the session
    let updated_session = CachedGameSession {
        game_identifier: game_identifier.to_string(),
        player_identifier: initial_session.player_identifier.clone(),
        current_floor_level: 5,
        turn_count: 100,
        status: "InProgress".to_string(),
        random_seed: 42,
        event_sequence: 50,
    };

    cache
        .set(&game_identifier, &updated_session, Duration::from_secs(60))
        .run_async()
        .await;

    // Verify the update
    let found = cache.get(&game_identifier).run_async().await;
    assert!(found.is_some());
    let cached_session = found.unwrap();
    assert_eq!(cached_session.current_floor_level, 5);
    assert_eq!(cached_session.turn_count, 100);
    assert_eq!(cached_session.event_sequence, 50);

    // Cleanup
    cache.invalidate(&game_identifier).run_async().await;
}

#[rstest]
#[tokio::test]
async fn test_session_cache_ttl() {
    let test_prefix = format!("test:integration:{}:", Uuid::new_v4());
    let config = RedisConfig::with_url(REDIS_URL).with_key_prefix(&test_prefix);

    let connection =
        RedisConnectionFactory::create_client(&config).expect("Failed to create Redis client");
    let cache = RedisSessionCache::new(connection.clone());

    let game_identifier = GameIdentifier::new();
    let session = CachedGameSession {
        game_identifier: game_identifier.to_string(),
        player_identifier: Uuid::new_v4().to_string(),
        current_floor_level: 1,
        turn_count: 10,
        status: "InProgress".to_string(),
        random_seed: 42,
        event_sequence: 5,
    };

    // Set with a 60-second TTL
    cache
        .set(&game_identifier, &session, Duration::from_secs(60))
        .run_async()
        .await;

    // Check that the key has a TTL set
    let mut async_connection = connection
        .get_async_connection()
        .await
        .expect("Failed to get async connection");

    let key = connection.format_key(&format!("session:{}", game_identifier));
    let ttl: i64 = async_connection.ttl(&key).await.expect("Failed to get TTL");

    // TTL should be greater than 0 and less than or equal to 60
    assert!(ttl > 0, "TTL should be positive");
    assert!(ttl <= 60, "TTL should be at most 60 seconds");

    // Cleanup
    cache.invalidate(&game_identifier).run_async().await;
}

#[rstest]
#[tokio::test]
async fn test_session_cache_multiple_entries() {
    let test_prefix = format!("test:integration:{}:", Uuid::new_v4());
    let config = RedisConfig::with_url(REDIS_URL).with_key_prefix(&test_prefix);

    let connection =
        RedisConnectionFactory::create_client(&config).expect("Failed to create Redis client");
    let cache = RedisSessionCache::new(connection);

    // Create multiple sessions
    let game_identifier1 = GameIdentifier::new();
    let game_identifier2 = GameIdentifier::new();
    let game_identifier3 = GameIdentifier::new();

    let session1 = CachedGameSession {
        game_identifier: game_identifier1.to_string(),
        player_identifier: Uuid::new_v4().to_string(),
        current_floor_level: 1,
        turn_count: 10,
        status: "InProgress".to_string(),
        random_seed: 111,
        event_sequence: 5,
    };

    let session2 = CachedGameSession {
        game_identifier: game_identifier2.to_string(),
        player_identifier: Uuid::new_v4().to_string(),
        current_floor_level: 3,
        turn_count: 50,
        status: "InProgress".to_string(),
        random_seed: 222,
        event_sequence: 25,
    };

    let session3 = CachedGameSession {
        game_identifier: game_identifier3.to_string(),
        player_identifier: Uuid::new_v4().to_string(),
        current_floor_level: 10,
        turn_count: 200,
        status: "Victory".to_string(),
        random_seed: 333,
        event_sequence: 100,
    };

    // Set all sessions
    cache
        .set(&game_identifier1, &session1, Duration::from_secs(60))
        .run_async()
        .await;
    cache
        .set(&game_identifier2, &session2, Duration::from_secs(60))
        .run_async()
        .await;
    cache
        .set(&game_identifier3, &session3, Duration::from_secs(60))
        .run_async()
        .await;

    // Verify all can be retrieved independently
    let found1 = cache.get(&game_identifier1).run_async().await;
    let found2 = cache.get(&game_identifier2).run_async().await;
    let found3 = cache.get(&game_identifier3).run_async().await;

    assert!(found1.is_some());
    assert!(found2.is_some());
    assert!(found3.is_some());

    assert_eq!(found1.unwrap().random_seed, 111);
    assert_eq!(found2.unwrap().random_seed, 222);
    assert_eq!(found3.unwrap().random_seed, 333);

    // Cleanup
    cache.invalidate(&game_identifier1).run_async().await;
    cache.invalidate(&game_identifier2).run_async().await;
    cache.invalidate(&game_identifier3).run_async().await;
}
