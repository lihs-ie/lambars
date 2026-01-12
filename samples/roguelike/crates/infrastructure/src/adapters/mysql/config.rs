use std::time::Duration;

// =============================================================================
// MySqlPoolConfig
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MySqlPoolConfig {
    pub url: String,

    pub max_connections: u32,

    pub min_connections: u32,

    pub connect_timeout: Duration,

    pub idle_timeout: Option<Duration>,
}

// =============================================================================
// Default Implementation
// =============================================================================

impl Default for MySqlPoolConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            max_connections: 10,
            min_connections: 1,
            connect_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
        }
    }
}

// =============================================================================
// Builder Methods
// =============================================================================

impl MySqlPoolConfig {
    #[must_use]
    pub fn with_url(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }

    #[must_use]
    pub const fn with_max_connections(mut self, max_connections: u32) -> Self {
        self.max_connections = max_connections;
        self
    }

    #[must_use]
    pub const fn with_min_connections(mut self, min_connections: u32) -> Self {
        self.min_connections = min_connections;
        self
    }

    #[must_use]
    pub const fn with_connect_timeout(mut self, connect_timeout: Duration) -> Self {
        self.connect_timeout = connect_timeout;
        self
    }

    #[must_use]
    pub const fn with_idle_timeout(mut self, idle_timeout: Option<Duration>) -> Self {
        self.idle_timeout = idle_timeout;
        self
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
    // Default Tests
    // =========================================================================

    mod default_tests {
        use super::*;

        #[rstest]
        fn default_url_is_empty() {
            let config = MySqlPoolConfig::default();
            assert!(config.url.is_empty());
        }

        #[rstest]
        fn default_max_connections_is_10() {
            let config = MySqlPoolConfig::default();
            assert_eq!(config.max_connections, 10);
        }

        #[rstest]
        fn default_min_connections_is_1() {
            let config = MySqlPoolConfig::default();
            assert_eq!(config.min_connections, 1);
        }

        #[rstest]
        fn default_connect_timeout_is_30_seconds() {
            let config = MySqlPoolConfig::default();
            assert_eq!(config.connect_timeout, Duration::from_secs(30));
        }

        #[rstest]
        fn default_idle_timeout_is_10_minutes() {
            let config = MySqlPoolConfig::default();
            assert_eq!(config.idle_timeout, Some(Duration::from_secs(600)));
        }
    }

    // =========================================================================
    // Builder Tests
    // =========================================================================

    mod builder_tests {
        use super::*;

        #[rstest]
        fn with_url_sets_url() {
            let url = "mysql://user:password@localhost:3306/database";
            let config = MySqlPoolConfig::with_url(url);
            assert_eq!(config.url, url);
        }

        #[rstest]
        fn with_url_uses_default_for_other_fields() {
            let config = MySqlPoolConfig::with_url("mysql://localhost/db");
            assert_eq!(config.max_connections, 10);
            assert_eq!(config.min_connections, 1);
            assert_eq!(config.connect_timeout, Duration::from_secs(30));
            assert_eq!(config.idle_timeout, Some(Duration::from_secs(600)));
        }

        #[rstest]
        fn with_max_connections_sets_value() {
            let config = MySqlPoolConfig::with_url("mysql://localhost/db").with_max_connections(20);
            assert_eq!(config.max_connections, 20);
        }

        #[rstest]
        fn with_min_connections_sets_value() {
            let config = MySqlPoolConfig::with_url("mysql://localhost/db").with_min_connections(5);
            assert_eq!(config.min_connections, 5);
        }

        #[rstest]
        fn with_connect_timeout_sets_value() {
            let config = MySqlPoolConfig::with_url("mysql://localhost/db")
                .with_connect_timeout(Duration::from_secs(60));
            assert_eq!(config.connect_timeout, Duration::from_secs(60));
        }

        #[rstest]
        fn with_idle_timeout_sets_some_value() {
            let config = MySqlPoolConfig::with_url("mysql://localhost/db")
                .with_idle_timeout(Some(Duration::from_secs(300)));
            assert_eq!(config.idle_timeout, Some(Duration::from_secs(300)));
        }

        #[rstest]
        fn with_idle_timeout_sets_none_value() {
            let config = MySqlPoolConfig::with_url("mysql://localhost/db").with_idle_timeout(None);
            assert_eq!(config.idle_timeout, None);
        }

        #[rstest]
        fn builder_chain() {
            let config = MySqlPoolConfig::with_url("mysql://localhost/db")
                .with_max_connections(20)
                .with_min_connections(5)
                .with_connect_timeout(Duration::from_secs(60))
                .with_idle_timeout(Some(Duration::from_secs(300)));

            assert_eq!(config.url, "mysql://localhost/db");
            assert_eq!(config.max_connections, 20);
            assert_eq!(config.min_connections, 5);
            assert_eq!(config.connect_timeout, Duration::from_secs(60));
            assert_eq!(config.idle_timeout, Some(Duration::from_secs(300)));
        }
    }

    // =========================================================================
    // Clone Tests
    // =========================================================================

    mod clone_tests {
        use super::*;

        #[rstest]
        fn clone_produces_equal_config() {
            let config = MySqlPoolConfig::with_url("mysql://localhost/db")
                .with_max_connections(20)
                .with_min_connections(5);
            let cloned = config.clone();
            assert_eq!(config, cloned);
        }
    }

    // =========================================================================
    // Debug Tests
    // =========================================================================

    mod debug_tests {
        use super::*;

        #[rstest]
        fn debug_contains_field_values() {
            let config = MySqlPoolConfig::with_url("mysql://localhost/db");
            let debug_string = format!("{:?}", config);
            assert!(debug_string.contains("MySqlPoolConfig"));
            assert!(debug_string.contains("mysql://localhost/db"));
            assert!(debug_string.contains("max_connections"));
            assert!(debug_string.contains("10"));
        }
    }

    // =========================================================================
    // PartialEq Tests
    // =========================================================================

    mod equality_tests {
        use super::*;

        #[rstest]
        fn equal_configs_are_equal() {
            let config1 = MySqlPoolConfig::with_url("mysql://localhost/db");
            let config2 = MySqlPoolConfig::with_url("mysql://localhost/db");
            assert_eq!(config1, config2);
        }

        #[rstest]
        fn different_url_configs_are_not_equal() {
            let config1 = MySqlPoolConfig::with_url("mysql://localhost/db1");
            let config2 = MySqlPoolConfig::with_url("mysql://localhost/db2");
            assert_ne!(config1, config2);
        }

        #[rstest]
        fn different_max_connections_configs_are_not_equal() {
            let config1 =
                MySqlPoolConfig::with_url("mysql://localhost/db").with_max_connections(10);
            let config2 =
                MySqlPoolConfig::with_url("mysql://localhost/db").with_max_connections(20);
            assert_ne!(config1, config2);
        }
    }
}
