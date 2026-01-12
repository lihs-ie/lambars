use lambars::effect::AsyncIO;
use redis::Client;

use super::{RedisConfig, RedisConnection};
use crate::errors::InfraError;

// =============================================================================
// RedisConnectionFactory
// =============================================================================

#[derive(Debug, Clone, Copy)]
pub struct RedisConnectionFactory;

// =============================================================================
// Factory Methods
// =============================================================================

impl RedisConnectionFactory {
    pub fn create_client(config: &RedisConfig) -> Result<RedisConnection, InfraError> {
        let client = Client::open(config.url.as_str()).map_err(|error| {
            InfraError::cache_connection(format!("failed to create Redis client: {}", error))
        })?;

        Ok(RedisConnection::new(client, config.clone()))
    }

    #[must_use]
    pub fn create_client_async(
        config: &RedisConfig,
    ) -> AsyncIO<Result<RedisConnection, InfraError>> {
        let url = config.url.clone();
        let key_prefix = config.key_prefix.clone();
        let default_ttl = config.default_ttl;
        let connection_timeout = config.connection_timeout;

        AsyncIO::new(move || async move {
            let config = RedisConfig {
                url,
                key_prefix,
                default_ttl,
                connection_timeout,
            };
            Self::create_client(&config)
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
    // Debug Tests
    // =========================================================================

    mod debug_tests {
        use super::*;

        #[rstest]
        fn factory_implements_debug() {
            fn assert_debug<T: std::fmt::Debug>() {}
            assert_debug::<RedisConnectionFactory>();
        }

        #[rstest]
        fn factory_debug_output() {
            let debug_string = format!("{:?}", RedisConnectionFactory);
            assert!(debug_string.contains("RedisConnectionFactory"));
        }
    }

    // =========================================================================
    // Clone Tests
    // =========================================================================

    mod clone_tests {
        use super::*;

        #[rstest]
        fn factory_implements_clone() {
            fn assert_clone<T: Clone>() {}
            assert_clone::<RedisConnectionFactory>();
        }
    }

    // =========================================================================
    // Copy Tests
    // =========================================================================

    mod copy_tests {
        use super::*;

        #[rstest]
        fn factory_implements_copy() {
            fn assert_copy<T: Copy>() {}
            assert_copy::<RedisConnectionFactory>();
        }
    }

    // =========================================================================
    // Create Client Tests
    // =========================================================================

    mod create_client_tests {
        use super::*;

        #[rstest]
        fn create_client_with_valid_url_succeeds() {
            let config = RedisConfig::with_url("redis://localhost:6379");
            let result = RedisConnectionFactory::create_client(&config);
            assert!(result.is_ok());
        }

        #[rstest]
        fn create_client_returns_connection_with_correct_config() {
            let config =
                RedisConfig::with_url("redis://localhost:6379").with_key_prefix("test:prefix:");
            let connection = RedisConnectionFactory::create_client(&config)
                .expect("Failed to create connection");
            assert_eq!(connection.config().key_prefix, "test:prefix:");
        }

        #[rstest]
        fn create_client_with_invalid_url_fails() {
            let config = RedisConfig::with_url("invalid://not-a-valid-url");
            let result = RedisConnectionFactory::create_client(&config);
            assert!(result.is_err());
            if let Err(error) = result {
                assert!(error.is_connection());
            }
        }
    }

    // =========================================================================
    // AsyncIO Tests
    // =========================================================================

    mod async_io_tests {
        use super::*;

        #[rstest]
        fn create_client_async_returns_async_io() {
            let config = RedisConfig::with_url("redis://localhost:6379");
            let _async_io = RedisConnectionFactory::create_client_async(&config);
            // The test validates that the return type is correct
        }

        #[rstest]
        fn create_client_async_is_lazy() {
            // This test verifies that create_client_async does not attempt
            // to connect immediately - it just creates an AsyncIO description.
            // The client creation is lazy in redis-rs, so even with an
            // invalid URL, no error occurs until we try to use it.
            let config = RedisConfig::with_url("redis://nonexistent-host:6379");
            let _async_io = RedisConnectionFactory::create_client_async(&config);
            // If we reach here without error, the operation is lazy
        }
    }
}
