use std::sync::Arc;

// =============================================================================
// MySqlPool
// =============================================================================

#[derive(Clone)]
pub struct MySqlPool {
    inner: Arc<sqlx::MySqlPool>,
}

// =============================================================================
// Constructors
// =============================================================================

impl MySqlPool {
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
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }

    pub async fn close(&self) {
        self.inner.close().await;
    }

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
