//! Redis connection factory.
//!
//! This module provides the [`RedisConnectionFactory`] for creating Redis connections
//! with support for both synchronous and asynchronous creation patterns.

use lambars::effect::AsyncIO;
use redis::Client;

use super::{RedisConfig, RedisConnection};
use crate::errors::InfraError;

// =============================================================================
// RedisConnectionFactory
// =============================================================================

/// Factory for creating Redis connections.
///
/// This struct provides static methods for creating Redis connections
/// using configurations defined in [`RedisConfig`]. It supports both
/// synchronous and asynchronous connection creation patterns.
///
/// # Examples
///
/// ```rust,ignore
/// use roguelike_infrastructure::adapters::redis::{RedisConfig, RedisConnectionFactory};
///
/// // Create a connection synchronously (for use outside async context)
/// let config = RedisConfig::with_url("redis://localhost:6379");
/// let connection = RedisConnectionFactory::create_client(&config)?;
///
/// // Create a connection asynchronously using AsyncIO
/// let async_connection = RedisConnectionFactory::create_client_async(&config);
/// let connection = async_connection.run_async().await?;
/// ```
#[derive(Debug, Clone, Copy)]
pub struct RedisConnectionFactory;

// =============================================================================
// Factory Methods
// =============================================================================

impl RedisConnectionFactory {
    /// Creates a Redis connection synchronously.
    ///
    /// This method creates a Redis client immediately. It should be used
    /// when you need the connection outside of an async context.
    ///
    /// Note: This method only creates the client object. The actual TCP
    /// connection is established lazily when operations are performed.
    ///
    /// # Arguments
    ///
    /// * `config` - The Redis configuration.
    ///
    /// # Returns
    ///
    /// A `Result` containing the created connection or an infrastructure error.
    ///
    /// # Errors
    ///
    /// Returns `InfraError::Connection` if the Redis URL is invalid.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use roguelike_infrastructure::adapters::redis::{RedisConfig, RedisConnectionFactory};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = RedisConfig::with_url("redis://localhost:6379");
    ///     let connection = RedisConnectionFactory::create_client(&config)?;
    ///     Ok(())
    /// }
    /// ```
    pub fn create_client(config: &RedisConfig) -> Result<RedisConnection, InfraError> {
        let client = Client::open(config.url.as_str()).map_err(|error| {
            InfraError::cache_connection(format!("failed to create Redis client: {}", error))
        })?;

        Ok(RedisConnection::new(client, config.clone()))
    }

    /// Creates an `AsyncIO` action that, when executed, creates a Redis connection.
    ///
    /// This method returns an `AsyncIO` that describes the connection creation
    /// without executing it. The actual client is created only when
    /// `run_async().await` is called.
    ///
    /// # Arguments
    ///
    /// * `config` - The Redis configuration.
    ///
    /// # Returns
    ///
    /// An `AsyncIO<Result<RedisConnection, InfraError>>` that can be composed
    /// with other async operations before execution.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use roguelike_infrastructure::adapters::redis::{RedisConfig, RedisConnectionFactory};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = RedisConfig::with_url("redis://localhost:6379");
    ///
    ///     // Create the AsyncIO action (no connection yet)
    ///     let create_connection_action = RedisConnectionFactory::create_client_async(&config);
    ///
    ///     // Execute the action to create the connection
    ///     let connection = create_connection_action.run_async().await?;
    ///
    ///     Ok(())
    /// }
    /// ```
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
