//! Redis adapters.
//!
//! This module provides Redis connection management and cache implementations
//! for the infrastructure layer.
//!
//! # Components
//!
//! - [`RedisConfig`]: Configuration for Redis connection settings
//! - [`RedisConnection`]: A wrapper around `redis::Client` with Arc-based sharing
//! - [`RedisConnectionFactory`]: Factory for creating Redis connections
//! - [`RedisSessionCache`]: Redis-based implementation of the SessionCache port
//! - [`CachedGameSession`]: Serializable game session data for caching
//!
//! # Examples
//!
//! ## Connection Management
//!
//! ```rust,ignore
//! use roguelike_infrastructure::adapters::redis::{RedisConfig, RedisConnectionFactory};
//!
//! // Create a connection configuration
//! let config = RedisConfig::with_url("redis://localhost:6379")
//!     .with_key_prefix("prod:roguelike:")
//!     .with_default_ttl(std::time::Duration::from_secs(7200));
//!
//! // Create a connection synchronously
//! let connection = RedisConnectionFactory::create_client(&config)?;
//!
//! // Format keys with the configured prefix
//! let session_key = connection.format_key("session:abc-123");
//! // session_key = "prod:roguelike:session:abc-123"
//!
//! // Or create a connection asynchronously using AsyncIO
//! let async_connection = RedisConnectionFactory::create_client_async(&config);
//! let connection = async_connection.run_async().await?;
//!
//! // Get an async connection for Redis operations
//! let mut async_conn = connection.get_async_connection().await?;
//! ```
//!
//! ## Session Caching
//!
//! ```rust,ignore
//! use roguelike_infrastructure::adapters::redis::{
//!     RedisConfig, RedisConnectionFactory, RedisSessionCache, CachedGameSession
//! };
//! use roguelike_workflow::ports::SessionCache;
//! use std::time::Duration;
//!
//! let config = RedisConfig::with_url("redis://localhost:6379");
//! let connection = RedisConnectionFactory::create_client(&config)?;
//! let cache = RedisSessionCache::new(connection);
//!
//! // Cache a game session
//! let session = CachedGameSession {
//!     game_identifier: "game-123".to_string(),
//!     player_identifier: "player-456".to_string(),
//!     current_floor_level: 5,
//!     turn_count: 150,
//!     status: "InProgress".to_string(),
//!     random_seed: 42,
//!     event_sequence: 75,
//! };
//! cache.set(&identifier, &session, Duration::from_secs(300)).run_async().await;
//!
//! // Retrieve from cache
//! if let Some(cached) = cache.get(&identifier).run_async().await {
//!     println!("Found session on floor {}", cached.current_floor_level);
//! }
//! ```

mod cache;
mod config;
mod connection;
mod factory;

pub use cache::{CachedGameSession, RedisSessionCache};
pub use config::RedisConfig;
pub use connection::RedisConnection;
pub use factory::RedisConnectionFactory;
