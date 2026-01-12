//! Redis session cache implementation.
//!
//! This module provides a Redis-based implementation of the [`SessionCache`] port,
//! using JSON serialization for game session data.
//!
//! # Examples
//!
//! ```rust,ignore
//! use roguelike_infrastructure::adapters::redis::{RedisConfig, RedisConnectionFactory, RedisSessionCache};
//! use roguelike_workflow::ports::SessionCache;
//! use std::time::Duration;
//!
//! let config = RedisConfig::with_url("redis://localhost:6379")
//!     .with_key_prefix("dev:roguelike:");
//! let connection = RedisConnectionFactory::create_client(&config)?;
//! let cache = RedisSessionCache::new(connection);
//!
//! // Set a session with TTL
//! let session = CachedGameSession { /* ... */ };
//! cache.set(&identifier, &session, Duration::from_secs(300)).run_async().await;
//!
//! // Get a session
//! if let Some(session) = cache.get(&identifier).run_async().await {
//!     // Use the session
//! }
//!
//! // Invalidate a session
//! cache.invalidate(&identifier).run_async().await;
//! ```

use std::time::Duration;

use lambars::effect::AsyncIO;
use redis::AsyncCommands;
use roguelike_domain::game_session::GameIdentifier;
use roguelike_workflow::ports::SessionCache;
use serde::{Deserialize, Serialize};

use super::RedisConnection;

// =============================================================================
// CachedGameSession
// =============================================================================

/// A serializable representation of a game session for caching.
///
/// This structure contains the essential data needed to restore a game session
/// from cache. It is designed to be lightweight and serializable using JSON.
///
/// # Examples
///
/// ```rust
/// use roguelike_infrastructure::adapters::redis::CachedGameSession;
///
/// let session = CachedGameSession {
///     game_identifier: "550e8400-e29b-41d4-a716-446655440000".to_string(),
///     player_identifier: "player-123".to_string(),
///     current_floor_level: 5,
///     turn_count: 150,
///     status: "InProgress".to_string(),
///     random_seed: 42,
///     event_sequence: 75,
/// };
///
/// // Serialize to JSON
/// let json = serde_json::to_string(&session).unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedGameSession {
    /// The unique identifier of the game session.
    pub game_identifier: String,

    /// The unique identifier of the player.
    pub player_identifier: String,

    /// The current floor level the player is on.
    pub current_floor_level: u32,

    /// The total number of turns elapsed in the game.
    pub turn_count: u64,

    /// The current status of the game session (e.g., "InProgress", "Completed").
    pub status: String,

    /// The random seed used for deterministic game generation.
    pub random_seed: u64,

    /// The current event sequence number for event sourcing.
    pub event_sequence: u64,
}

// =============================================================================
// RedisSessionCache
// =============================================================================

/// A Redis-based implementation of the [`SessionCache`] port.
///
/// This cache stores game sessions in Redis using JSON serialization.
/// Keys are formatted as `{prefix}session:{game_identifier}`.
///
/// # Thread Safety
///
/// `RedisSessionCache` is `Clone`, `Send`, and `Sync`, allowing it to be shared
/// safely across multiple async tasks.
///
/// # Examples
///
/// ```rust,ignore
/// use roguelike_infrastructure::adapters::redis::{RedisConfig, RedisConnectionFactory, RedisSessionCache};
/// use roguelike_workflow::ports::SessionCache;
/// use std::time::Duration;
///
/// let config = RedisConfig::with_url("redis://localhost:6379");
/// let connection = RedisConnectionFactory::create_client(&config)?;
/// let cache = RedisSessionCache::new(connection);
///
/// // Use the cache
/// let identifier = GameIdentifier::new();
/// cache.invalidate(&identifier).run_async().await;
/// ```
#[derive(Clone, Debug)]
pub struct RedisSessionCache {
    /// The Redis connection used for cache operations.
    connection: RedisConnection,
}

// =============================================================================
// Constructors
// =============================================================================

impl RedisSessionCache {
    /// Creates a new `RedisSessionCache` with the given connection.
    ///
    /// # Arguments
    ///
    /// * `connection` - The Redis connection to use for cache operations.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use roguelike_infrastructure::adapters::redis::{RedisConnection, RedisSessionCache};
    ///
    /// let connection = /* create connection */;
    /// let cache = RedisSessionCache::new(connection);
    /// ```
    #[must_use]
    pub fn new(connection: RedisConnection) -> Self {
        Self { connection }
    }

    /// Formats a Redis key for the given game identifier.
    ///
    /// The key format is `{prefix}session:{game_identifier}`.
    ///
    /// # Arguments
    ///
    /// * `identifier` - The game session identifier.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let key = cache.session_key(&identifier);
    /// // e.g., "dev:roguelike:session:550e8400-e29b-41d4-a716-446655440000"
    /// ```
    #[must_use]
    fn session_key(&self, identifier: &GameIdentifier) -> String {
        self.connection.format_key(&format!("session:{}", identifier))
    }
}

// =============================================================================
// SessionCache Implementation
// =============================================================================

impl SessionCache for RedisSessionCache {
    type GameSession = CachedGameSession;

    fn get(&self, identifier: &GameIdentifier) -> AsyncIO<Option<Self::GameSession>> {
        let connection = self.connection.clone();
        let key = self.session_key(identifier);

        AsyncIO::new(move || async move {
            let mut async_connection = match connection.get_async_connection().await {
                Ok(connection) => connection,
                Err(error) => {
                    tracing::warn!("Failed to get Redis connection for cache get: {}", error);
                    return None;
                }
            };

            let result: Result<Option<String>, redis::RedisError> = async_connection.get(&key).await;

            match result {
                Ok(Some(json)) => match serde_json::from_str::<CachedGameSession>(&json) {
                    Ok(session) => Some(session),
                    Err(error) => {
                        tracing::warn!(
                            "Failed to deserialize cached session for key '{}': {}",
                            key,
                            error
                        );
                        None
                    }
                },
                Ok(None) => None,
                Err(error) => {
                    tracing::warn!("Failed to get cached session for key '{}': {}", key, error);
                    None
                }
            }
        })
    }

    fn set(
        &self,
        identifier: &GameIdentifier,
        session: &Self::GameSession,
        time_to_live: Duration,
    ) -> AsyncIO<()> {
        let connection = self.connection.clone();
        let key = self.session_key(identifier);
        let session = session.clone();

        AsyncIO::new(move || async move {
            let json = match serde_json::to_string(&session) {
                Ok(json) => json,
                Err(error) => {
                    tracing::warn!("Failed to serialize session for cache: {}", error);
                    return;
                }
            };

            let mut async_connection = match connection.get_async_connection().await {
                Ok(connection) => connection,
                Err(error) => {
                    tracing::warn!("Failed to get Redis connection for cache set: {}", error);
                    return;
                }
            };

            let ttl_seconds = time_to_live.as_secs() as i64;

            let result: Result<(), redis::RedisError> = async_connection
                .set_ex(&key, json, ttl_seconds as u64)
                .await;

            if let Err(error) = result {
                tracing::warn!("Failed to set cached session for key '{}': {}", key, error);
            }
        })
    }

    fn invalidate(&self, identifier: &GameIdentifier) -> AsyncIO<()> {
        let connection = self.connection.clone();
        let key = self.session_key(identifier);

        AsyncIO::new(move || async move {
            let mut async_connection = match connection.get_async_connection().await {
                Ok(connection) => connection,
                Err(error) => {
                    tracing::warn!(
                        "Failed to get Redis connection for cache invalidate: {}",
                        error
                    );
                    return;
                }
            };

            let result: Result<i32, redis::RedisError> = async_connection.del(&key).await;

            if let Err(error) = result {
                tracing::warn!(
                    "Failed to invalidate cached session for key '{}': {}",
                    key,
                    error
                );
            }
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // CachedGameSession Tests
    // =========================================================================

    mod cached_game_session {
        use super::*;

        #[rstest]
        fn serialization_roundtrip() {
            let session = CachedGameSession {
                game_identifier: "550e8400-e29b-41d4-a716-446655440000".to_string(),
                player_identifier: "player-123".to_string(),
                current_floor_level: 5,
                turn_count: 150,
                status: "InProgress".to_string(),
                random_seed: 42,
                event_sequence: 75,
            };

            let json = serde_json::to_string(&session).expect("serialization should succeed");
            let deserialized: CachedGameSession =
                serde_json::from_str(&json).expect("deserialization should succeed");

            assert_eq!(session, deserialized);
        }

        #[rstest]
        fn serialization_includes_all_fields() {
            let session = CachedGameSession {
                game_identifier: "test-game-id".to_string(),
                player_identifier: "test-player-id".to_string(),
                current_floor_level: 10,
                turn_count: 500,
                status: "Completed".to_string(),
                random_seed: 12345,
                event_sequence: 250,
            };

            let json = serde_json::to_string(&session).expect("serialization should succeed");

            assert!(json.contains("test-game-id"));
            assert!(json.contains("test-player-id"));
            assert!(json.contains("10"));
            assert!(json.contains("500"));
            assert!(json.contains("Completed"));
            assert!(json.contains("12345"));
            assert!(json.contains("250"));
        }

        #[rstest]
        fn deserialization_from_json_string() {
            let json = r#"{
                "game_identifier": "game-abc",
                "player_identifier": "player-xyz",
                "current_floor_level": 3,
                "turn_count": 100,
                "status": "InProgress",
                "random_seed": 999,
                "event_sequence": 50
            }"#;

            let session: CachedGameSession =
                serde_json::from_str(json).expect("deserialization should succeed");

            assert_eq!(session.game_identifier, "game-abc");
            assert_eq!(session.player_identifier, "player-xyz");
            assert_eq!(session.current_floor_level, 3);
            assert_eq!(session.turn_count, 100);
            assert_eq!(session.status, "InProgress");
            assert_eq!(session.random_seed, 999);
            assert_eq!(session.event_sequence, 50);
        }

        #[rstest]
        fn clone_creates_equal_copy() {
            let session = CachedGameSession {
                game_identifier: "original-id".to_string(),
                player_identifier: "player-id".to_string(),
                current_floor_level: 7,
                turn_count: 300,
                status: "InProgress".to_string(),
                random_seed: 777,
                event_sequence: 150,
            };

            let cloned = session.clone();

            assert_eq!(session, cloned);
        }

        #[rstest]
        fn debug_format() {
            let session = CachedGameSession {
                game_identifier: "debug-test".to_string(),
                player_identifier: "player".to_string(),
                current_floor_level: 1,
                turn_count: 0,
                status: "New".to_string(),
                random_seed: 1,
                event_sequence: 0,
            };

            let debug_string = format!("{:?}", session);

            assert!(debug_string.contains("CachedGameSession"));
            assert!(debug_string.contains("debug-test"));
        }

        #[rstest]
        fn equality() {
            let session1 = CachedGameSession {
                game_identifier: "same-id".to_string(),
                player_identifier: "player".to_string(),
                current_floor_level: 1,
                turn_count: 10,
                status: "InProgress".to_string(),
                random_seed: 42,
                event_sequence: 5,
            };

            let session2 = CachedGameSession {
                game_identifier: "same-id".to_string(),
                player_identifier: "player".to_string(),
                current_floor_level: 1,
                turn_count: 10,
                status: "InProgress".to_string(),
                random_seed: 42,
                event_sequence: 5,
            };

            let session3 = CachedGameSession {
                game_identifier: "different-id".to_string(),
                player_identifier: "player".to_string(),
                current_floor_level: 1,
                turn_count: 10,
                status: "InProgress".to_string(),
                random_seed: 42,
                event_sequence: 5,
            };

            assert_eq!(session1, session2);
            assert_ne!(session1, session3);
        }
    }

    // =========================================================================
    // RedisSessionCache Tests
    // =========================================================================

    mod redis_session_cache {
        use super::*;
        use crate::adapters::redis::RedisConfig;

        fn create_test_cache(key_prefix: &str) -> RedisSessionCache {
            let client =
                redis::Client::open("redis://localhost:6379").expect("Failed to create client");
            let config =
                RedisConfig::with_url("redis://localhost:6379").with_key_prefix(key_prefix);
            let connection = RedisConnection::new(client, config);
            RedisSessionCache::new(connection)
        }

        #[rstest]
        fn session_key_format() {
            let cache = create_test_cache("dev:roguelike:");
            let identifier = GameIdentifier::new();

            let key = cache.session_key(&identifier);

            assert!(key.starts_with("dev:roguelike:session:"));
            assert!(key.contains(&identifier.to_string()));
        }

        #[rstest]
        fn session_key_with_empty_prefix() {
            let cache = create_test_cache("");
            let identifier = GameIdentifier::new();

            let key = cache.session_key(&identifier);

            assert!(key.starts_with("session:"));
            assert!(key.contains(&identifier.to_string()));
        }

        #[rstest]
        fn session_key_with_different_prefix() {
            let cache = create_test_cache("prod:app:");
            let identifier = GameIdentifier::new();

            let key = cache.session_key(&identifier);

            assert!(key.starts_with("prod:app:session:"));
        }

        #[rstest]
        fn cache_is_clone() {
            fn assert_clone<T: Clone>() {}
            assert_clone::<RedisSessionCache>();
        }

        #[rstest]
        fn cache_is_debug() {
            fn assert_debug<T: std::fmt::Debug>() {}
            assert_debug::<RedisSessionCache>();
        }

        #[rstest]
        fn cache_is_send_and_sync() {
            fn assert_send<T: Send>() {}
            fn assert_sync<T: Sync>() {}
            assert_send::<RedisSessionCache>();
            assert_sync::<RedisSessionCache>();
        }
    }
}
