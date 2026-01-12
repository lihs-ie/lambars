//! MySQL connection pool wrapper.
//!
//! This module provides the [`MySqlPool`] struct, a wrapper around
//! `sqlx::MySqlPool` that provides Arc-based sharing.

use std::sync::Arc;

// =============================================================================
// MySqlPool
// =============================================================================

/// A MySQL connection pool wrapper with Arc-based sharing.
///
/// This struct wraps `sqlx::MySqlPool` in an `Arc`, enabling cheap cloning
/// and sharing across multiple tasks or threads. The underlying pool is
/// reference-counted, so cloning a `MySqlPool` does not create a new pool.
///
/// # Examples
///
/// ```rust,ignore
/// use roguelike_infrastructure::adapters::mysql::{MySqlPoolConfig, MySqlPoolFactory};
///
/// let config = MySqlPoolConfig::with_url("mysql://localhost/db");
/// let pool = MySqlPoolFactory::create_pool(&config)?;
///
/// // Clone is cheap - shares the same underlying pool
/// let pool_clone = pool.clone();
///
/// // Check if the pool is closed
/// assert!(!pool.is_closed());
///
/// // Close the pool
/// pool.close().await;
/// ```
#[derive(Clone)]
pub struct MySqlPool {
    /// The underlying sqlx pool wrapped in Arc.
    inner: Arc<sqlx::MySqlPool>,
}

// =============================================================================
// Constructors
// =============================================================================

impl MySqlPool {
    /// Creates a new `MySqlPool` from an existing `sqlx::MySqlPool`.
    ///
    /// # Arguments
    ///
    /// * `pool` - The sqlx pool to wrap.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use sqlx::mysql::MySqlPoolOptions;
    ///
    /// let sqlx_pool = MySqlPoolOptions::new()
    ///     .connect("mysql://localhost/db")
    ///     .await?;
    /// let pool = MySqlPool::new(sqlx_pool);
    /// ```
    #[must_use]
    pub fn new(pool: sqlx::MySqlPool) -> Self {
        Self {
            inner: Arc::new(pool),
        }
    }
}

// =============================================================================
// Pool Operations
// =============================================================================

impl MySqlPool {
    /// Returns whether the pool has been explicitly closed.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let pool = MySqlPoolFactory::create_pool(&config)?;
    /// assert!(!pool.is_closed());
    ///
    /// pool.close().await;
    /// assert!(pool.is_closed());
    /// ```
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }

    /// Closes the pool.
    ///
    /// This prevents new connections from being created and waits for all
    /// existing connections to be returned to the pool before completing.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let pool = MySqlPoolFactory::create_pool(&config)?;
    /// pool.close().await;
    /// assert!(pool.is_closed());
    /// ```
    pub async fn close(&self) {
        self.inner.close().await;
    }

    /// Returns a reference to the underlying `sqlx::MySqlPool`.
    ///
    /// This is useful when you need to pass the pool to sqlx functions
    /// that expect a `&MySqlPool`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use sqlx::query;
    ///
    /// let pool = MySqlPoolFactory::create_pool(&config)?;
    /// let rows = query("SELECT 1").fetch_all(pool.as_inner()).await?;
    /// ```
    #[must_use]
    pub fn as_inner(&self) -> &sqlx::MySqlPool {
        &self.inner
    }
}

// =============================================================================
// From Implementation
// =============================================================================

impl From<sqlx::MySqlPool> for MySqlPool {
    fn from(pool: sqlx::MySqlPool) -> Self {
        Self::new(pool)
    }
}

// =============================================================================
// AsRef Implementation
// =============================================================================

impl AsRef<sqlx::MySqlPool> for MySqlPool {
    fn as_ref(&self) -> &sqlx::MySqlPool {
        &self.inner
    }
}

// =============================================================================
// Debug Implementation
// =============================================================================

impl std::fmt::Debug for MySqlPool {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MySqlPool")
            .field("is_closed", &self.is_closed())
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

        /// Note: This test validates that Clone is implemented correctly
        /// by checking that the trait bound is satisfied.
        #[rstest]
        fn mysql_pool_is_clone() {
            fn assert_clone<T: Clone>() {}
            assert_clone::<MySqlPool>();
        }
    }

    // =========================================================================
    // Debug Tests
    // =========================================================================

    mod debug_tests {
        use super::*;

        #[rstest]
        fn mysql_pool_is_debug() {
            fn assert_debug<T: std::fmt::Debug>() {}
            assert_debug::<MySqlPool>();
        }
    }

    // =========================================================================
    // From Tests
    // =========================================================================

    mod from_tests {
        use super::*;

        #[rstest]
        fn mysql_pool_implements_from_sqlx_pool() {
            fn assert_from<T: From<sqlx::MySqlPool>>() {}
            assert_from::<MySqlPool>();
        }
    }

    // =========================================================================
    // AsRef Tests
    // =========================================================================

    mod as_ref_tests {
        use super::*;

        #[rstest]
        fn mysql_pool_implements_as_ref() {
            fn assert_as_ref<T: AsRef<sqlx::MySqlPool>>() {}
            assert_as_ref::<MySqlPool>();
        }
    }
}
