use lambars::effect::AsyncIO;
use sqlx::mysql::MySqlPoolOptions;

use crate::adapters::mysql::{MySqlPool, MySqlPoolConfig};
use crate::errors::InfraError;

// =============================================================================
// MySqlPoolFactory
// =============================================================================

#[derive(Debug, Clone, Copy)]
pub struct MySqlPoolFactory;

// =============================================================================
// Factory Methods
// =============================================================================

impl MySqlPoolFactory {
    pub fn create_pool(config: &MySqlPoolConfig) -> Result<MySqlPool, InfraError> {
        let runtime = tokio::runtime::Runtime::new().map_err(|error| {
            InfraError::database_connection(format!("failed to create tokio runtime: {}", error))
        })?;

        runtime.block_on(Self::create_pool_internal(config))
    }

    #[must_use]
    pub fn create_pool_async(config: &MySqlPoolConfig) -> AsyncIO<Result<MySqlPool, InfraError>> {
        let url = config.url.clone();
        let max_connections = config.max_connections;
        let min_connections = config.min_connections;
        let connect_timeout = config.connect_timeout;
        let idle_timeout = config.idle_timeout;

        AsyncIO::new(move || async move {
            let config = MySqlPoolConfig {
                url,
                max_connections,
                min_connections,
                connect_timeout,
                idle_timeout,
            };
            Self::create_pool_internal(&config).await
        })
    }

    async fn create_pool_internal(config: &MySqlPoolConfig) -> Result<MySqlPool, InfraError> {
        let mut pool_options = MySqlPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(config.connect_timeout);

        if let Some(idle_timeout) = config.idle_timeout {
            pool_options = pool_options.idle_timeout(idle_timeout);
        }

        let sqlx_pool = pool_options.connect(&config.url).await.map_err(|error| {
            InfraError::database_connection(format!(
                "failed to create MySQL connection pool: {}",
                error
            ))
        })?;

        Ok(MySqlPool::new(sqlx_pool))
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
            assert_debug::<MySqlPoolFactory>();
        }

        #[rstest]
        fn factory_debug_output() {
            let debug_string = format!("{:?}", MySqlPoolFactory);
            assert!(debug_string.contains("MySqlPoolFactory"));
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
            assert_clone::<MySqlPoolFactory>();
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
            assert_copy::<MySqlPoolFactory>();
        }
    }

    // =========================================================================
    // AsyncIO Tests
    // =========================================================================

    mod async_io_tests {
        use super::*;

        #[rstest]
        fn create_pool_async_returns_async_io() {
            let config = MySqlPoolConfig::with_url("mysql://localhost/db");
            let _async_io = MySqlPoolFactory::create_pool_async(&config);
            // The test validates that the return type is correct
            // Actual connection tests should be in integration tests
        }

        #[rstest]
        fn create_pool_async_is_lazy() {
            // This test verifies that create_pool_async does not attempt
            // to connect immediately - it just creates an AsyncIO description.
            // If it tried to connect, this test would hang or fail since
            // there's no actual database.
            let config = MySqlPoolConfig::with_url("mysql://nonexistent-host/db");
            let _async_io = MySqlPoolFactory::create_pool_async(&config);
            // If we reach here without hanging, the operation is lazy
        }
    }

    // =========================================================================
    // Error Handling Tests
    // =========================================================================

    mod error_handling_tests {
        use super::*;

        #[rstest]
        fn invalid_url_produces_connection_error() {
            let config = MySqlPoolConfig::with_url("mysql://invalid:host:port/db");
            let result = MySqlPoolFactory::create_pool(&config);
            assert!(result.is_err());
            if let Err(error) = result {
                assert!(error.is_connection());
            }
        }
    }
}
