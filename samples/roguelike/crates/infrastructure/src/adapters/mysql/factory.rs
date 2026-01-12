//! MySQL connection pool factory.
//!
//! This module provides the [`MySqlPoolFactory`] for creating MySQL connection pools
//! with support for both synchronous and asynchronous creation patterns.

use lambars::effect::AsyncIO;
use sqlx::mysql::MySqlPoolOptions;

use crate::adapters::mysql::{MySqlPool, MySqlPoolConfig};
use crate::errors::InfraError;

// =============================================================================
// MySqlPoolFactory
// =============================================================================

/// Factory for creating MySQL connection pools.
///
/// This struct provides static methods for creating MySQL connection pools
/// using configurations defined in [`MySqlPoolConfig`]. It supports both
/// synchronous and asynchronous pool creation patterns.
///
/// # Examples
///
/// ```rust,ignore
/// use roguelike_infrastructure::adapters::mysql::{MySqlPoolConfig, MySqlPoolFactory};
///
/// // Create a pool synchronously (for use outside async context)
/// let config = MySqlPoolConfig::with_url("mysql://localhost/db");
/// let pool = MySqlPoolFactory::create_pool(&config)?;
///
/// // Create a pool asynchronously using AsyncIO
/// let async_pool = MySqlPoolFactory::create_pool_async(&config);
/// let pool = async_pool.run_async().await?;
/// ```
#[derive(Debug, Clone, Copy)]
pub struct MySqlPoolFactory;

// =============================================================================
// Factory Methods
// =============================================================================

impl MySqlPoolFactory {
    /// Creates a MySQL connection pool synchronously.
    ///
    /// This method blocks the current thread until the pool is created.
    /// It should be used outside of async contexts (e.g., during application startup).
    ///
    /// # Arguments
    ///
    /// * `config` - The pool configuration.
    ///
    /// # Returns
    ///
    /// A `Result` containing the created pool or an infrastructure error.
    ///
    /// # Errors
    ///
    /// Returns `InfraError::Connection` if the connection to the database fails.
    ///
    /// # Panics
    ///
    /// Panics if called from within an async runtime context, as this creates
    /// a nested runtime which is not allowed by tokio.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use roguelike_infrastructure::adapters::mysql::{MySqlPoolConfig, MySqlPoolFactory};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = MySqlPoolConfig::with_url("mysql://localhost/db");
    ///     let pool = MySqlPoolFactory::create_pool(&config)?;
    ///     Ok(())
    /// }
    /// ```
    pub fn create_pool(config: &MySqlPoolConfig) -> Result<MySqlPool, InfraError> {
        let runtime = tokio::runtime::Runtime::new().map_err(|error| {
            InfraError::database_connection(format!("failed to create tokio runtime: {}", error))
        })?;

        runtime.block_on(Self::create_pool_internal(config))
    }

    /// Creates an `AsyncIO` action that, when executed, creates a MySQL connection pool.
    ///
    /// This method returns an `AsyncIO` that describes the pool creation operation
    /// without executing it. The actual connection is established only when
    /// `run_async().await` is called.
    ///
    /// # Arguments
    ///
    /// * `config` - The pool configuration.
    ///
    /// # Returns
    ///
    /// An `AsyncIO<Result<MySqlPool, InfraError>>` that can be composed with other
    /// async operations before execution.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use roguelike_infrastructure::adapters::mysql::{MySqlPoolConfig, MySqlPoolFactory};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = MySqlPoolConfig::with_url("mysql://localhost/db");
    ///
    ///     // Create the AsyncIO action (no connection yet)
    ///     let create_pool_action = MySqlPoolFactory::create_pool_async(&config);
    ///
    ///     // Execute the action to create the pool
    ///     let pool = create_pool_action.run_async().await?;
    ///
    ///     Ok(())
    /// }
    /// ```
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

    /// Internal helper method to create a pool asynchronously.
    ///
    /// This is used by both `create_pool` and `create_pool_async`.
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

        /// Note: Actual connection error tests should be in integration tests
        /// with proper database setup. This test validates the error type.
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
