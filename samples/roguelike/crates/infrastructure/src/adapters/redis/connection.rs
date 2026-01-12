use std::sync::Arc;

use redis::aio::MultiplexedConnection;

use super::RedisConfig;
use crate::errors::InfraError;

// =============================================================================
// RedisConnection
// =============================================================================

#[derive(Clone)]
pub struct RedisConnection {
    client: Arc<redis::Client>,
    config: Arc<RedisConfig>,
}

// =============================================================================
// Constructors
// =============================================================================

impl RedisConnection {
    #[must_use]
    pub fn new(client: redis::Client, config: RedisConfig) -> Self {
        Self {
            client: Arc::new(client),
            config: Arc::new(config),
        }
    }
}

// =============================================================================
// Connection Operations
// =============================================================================

impl RedisConnection {
    pub async fn get_async_connection(&self) -> Result<MultiplexedConnection, InfraError> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(InfraError::from)
    }

    #[must_use]
    pub fn format_key(&self, suffix: &str) -> String {
        format!("{}{}", self.config.key_prefix, suffix)
    }

    #[must_use]
    pub fn as_client(&self) -> &redis::Client {
        &self.client
    }

    #[must_use]
    pub fn config(&self) -> &RedisConfig {
        &self.config
    }
}

// =============================================================================
// Debug Implementation
// =============================================================================

impl std::fmt::Debug for RedisConnection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RedisConnection")
            .field("url", &self.config.url)
            .field("key_prefix", &self.config.key_prefix)
            .finish_non_exhaustive()
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
    // Clone Tests
    // =========================================================================

    mod clone_tests {
        use super::*;

        #[rstest]
        fn redis_connection_is_clone() {
            fn assert_clone<T: Clone>() {}
            assert_clone::<RedisConnection>();
        }
    }

    // =========================================================================
    // Debug Tests
    // =========================================================================

    mod debug_tests {
        use super::*;

        #[rstest]
        fn redis_connection_is_debug() {
            fn assert_debug<T: std::fmt::Debug>() {}
            assert_debug::<RedisConnection>();
        }
    }

    // =========================================================================
    // Format Key Tests
    // =========================================================================

    mod format_key_tests {
        use super::*;

        fn create_test_connection(key_prefix: &str) -> RedisConnection {
            let client =
                redis::Client::open("redis://localhost:6379").expect("Failed to create client");
            let config =
                RedisConfig::with_url("redis://localhost:6379").with_key_prefix(key_prefix);
            RedisConnection::new(client, config)
        }

        #[rstest]
        fn format_key_prepends_prefix() {
            let connection = create_test_connection("dev:roguelike:");
            let key = connection.format_key("session:abc-123");
            assert_eq!(key, "dev:roguelike:session:abc-123");
        }

        #[rstest]
        fn format_key_with_empty_suffix() {
            let connection = create_test_connection("dev:roguelike:");
            let key = connection.format_key("");
            assert_eq!(key, "dev:roguelike:");
        }

        #[rstest]
        fn format_key_with_different_prefix() {
            let connection = create_test_connection("prod:app:");
            let key = connection.format_key("user:123");
            assert_eq!(key, "prod:app:user:123");
        }

        #[rstest]
        fn format_key_with_empty_prefix() {
            let connection = create_test_connection("");
            let key = connection.format_key("session:abc");
            assert_eq!(key, "session:abc");
        }
    }

    // =========================================================================
    // Config Access Tests
    // =========================================================================

    mod config_access_tests {
        use super::*;
        use std::time::Duration;

        #[rstest]
        fn config_returns_correct_values() {
            let client =
                redis::Client::open("redis://localhost:6379").expect("Failed to create client");
            let config = RedisConfig::with_url("redis://localhost:6379")
                .with_key_prefix("test:")
                .with_default_ttl(Duration::from_secs(7200));
            let connection = RedisConnection::new(client, config);

            assert_eq!(connection.config().key_prefix, "test:");
            assert_eq!(connection.config().default_ttl, Duration::from_secs(7200));
        }
    }

    // =========================================================================
    // Client Access Tests
    // =========================================================================

    mod client_access_tests {
        use super::*;

        #[rstest]
        fn as_client_returns_reference() {
            let client =
                redis::Client::open("redis://localhost:6379").expect("Failed to create client");
            let config = RedisConfig::with_url("redis://localhost:6379");
            let connection = RedisConnection::new(client, config);

            // Just verify we can call the method and it returns something
            let _client = connection.as_client();
        }
    }
}
