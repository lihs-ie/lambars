//! Redis connection configuration.
//!
//! This module provides the [`RedisConfig`] struct for configuring
//! Redis connection settings.

use std::time::Duration;

// =============================================================================
// RedisConfig
// =============================================================================

/// Configuration for Redis connection.
///
/// This struct contains all the settings needed to configure a Redis connection,
/// including connection URL, key prefix, default TTL, and timeout settings.
///
/// # Examples
///
/// ```
/// use roguelike_infrastructure::adapters::redis::RedisConfig;
/// use std::time::Duration;
///
/// // Create with default settings and a URL
/// let config = RedisConfig::with_url("redis://localhost:6379");
///
/// // Create with custom settings
/// let config = RedisConfig::with_url("redis://localhost:6379")
///     .with_key_prefix("prod:roguelike:")
///     .with_default_ttl(Duration::from_secs(7200))
///     .with_connection_timeout(Duration::from_secs(60));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisConfig {
    /// The Redis connection URL.
    ///
    /// Format: `redis://[user[:password]@]host[:port][/database]`
    pub url: String,

    /// Key prefix for all Redis keys.
    ///
    /// Used for environment isolation (e.g., `dev:roguelike:`, `prod:roguelike:`).
    /// Defaults to `dev:roguelike:`.
    pub key_prefix: String,

    /// Default TTL (time-to-live) for cached entries.
    ///
    /// Defaults to 1 hour (3600 seconds).
    pub default_ttl: Duration,

    /// Maximum time to wait for a connection to be established.
    ///
    /// Defaults to 30 seconds.
    pub connection_timeout: Duration,
}

// =============================================================================
// Default Implementation
// =============================================================================

impl Default for RedisConfig {
    /// Creates a new `RedisConfig` with default values.
    ///
    /// # Default Values
    ///
    /// - `url`: `redis://localhost:6379`
    /// - `key_prefix`: `dev:roguelike:`
    /// - `default_ttl`: 1 hour (3600 seconds)
    /// - `connection_timeout`: 30 seconds
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_infrastructure::adapters::redis::RedisConfig;
    /// use std::time::Duration;
    ///
    /// let config = RedisConfig::default();
    /// assert_eq!(config.url, "redis://localhost:6379");
    /// assert_eq!(config.key_prefix, "dev:roguelike:");
    /// assert_eq!(config.default_ttl, Duration::from_secs(3600));
    /// assert_eq!(config.connection_timeout, Duration::from_secs(30));
    /// ```
    fn default() -> Self {
        Self {
            url: String::from("redis://localhost:6379"),
            key_prefix: String::from("dev:roguelike:"),
            default_ttl: Duration::from_secs(3600),
            connection_timeout: Duration::from_secs(30),
        }
    }
}

// =============================================================================
// Builder Methods
// =============================================================================

impl RedisConfig {
    /// Creates a new `RedisConfig` with the specified URL and default settings.
    ///
    /// # Arguments
    ///
    /// * `url` - The Redis connection URL.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_infrastructure::adapters::redis::RedisConfig;
    ///
    /// let config = RedisConfig::with_url("redis://localhost:6379");
    /// assert_eq!(config.url, "redis://localhost:6379");
    /// assert_eq!(config.key_prefix, "dev:roguelike:");
    /// ```
    #[must_use]
    pub fn with_url(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }

    /// Sets the key prefix for Redis keys.
    ///
    /// # Arguments
    ///
    /// * `key_prefix` - The prefix to prepend to all Redis keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_infrastructure::adapters::redis::RedisConfig;
    ///
    /// let config = RedisConfig::with_url("redis://localhost:6379")
    ///     .with_key_prefix("prod:roguelike:");
    /// assert_eq!(config.key_prefix, "prod:roguelike:");
    /// ```
    #[must_use]
    pub fn with_key_prefix(mut self, key_prefix: impl Into<String>) -> Self {
        self.key_prefix = key_prefix.into();
        self
    }

    /// Sets the default TTL for cached entries.
    ///
    /// # Arguments
    ///
    /// * `default_ttl` - The default time-to-live for cache entries.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_infrastructure::adapters::redis::RedisConfig;
    /// use std::time::Duration;
    ///
    /// let config = RedisConfig::with_url("redis://localhost:6379")
    ///     .with_default_ttl(Duration::from_secs(7200));
    /// assert_eq!(config.default_ttl, Duration::from_secs(7200));
    /// ```
    #[must_use]
    pub const fn with_default_ttl(mut self, default_ttl: Duration) -> Self {
        self.default_ttl = default_ttl;
        self
    }

    /// Sets the connection timeout.
    ///
    /// # Arguments
    ///
    /// * `connection_timeout` - The maximum time to wait for a connection.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_infrastructure::adapters::redis::RedisConfig;
    /// use std::time::Duration;
    ///
    /// let config = RedisConfig::with_url("redis://localhost:6379")
    ///     .with_connection_timeout(Duration::from_secs(60));
    /// assert_eq!(config.connection_timeout, Duration::from_secs(60));
    /// ```
    #[must_use]
    pub const fn with_connection_timeout(mut self, connection_timeout: Duration) -> Self {
        self.connection_timeout = connection_timeout;
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
        fn default_url_is_localhost() {
            let config = RedisConfig::default();
            assert_eq!(config.url, "redis://localhost:6379");
        }

        #[rstest]
        fn default_key_prefix_is_dev_roguelike() {
            let config = RedisConfig::default();
            assert_eq!(config.key_prefix, "dev:roguelike:");
        }

        #[rstest]
        fn default_ttl_is_one_hour() {
            let config = RedisConfig::default();
            assert_eq!(config.default_ttl, Duration::from_secs(3600));
        }

        #[rstest]
        fn default_connection_timeout_is_30_seconds() {
            let config = RedisConfig::default();
            assert_eq!(config.connection_timeout, Duration::from_secs(30));
        }
    }

    // =========================================================================
    // Builder Tests
    // =========================================================================

    mod builder_tests {
        use super::*;

        #[rstest]
        fn with_url_sets_url() {
            let url = "redis://user:password@localhost:6379/0";
            let config = RedisConfig::with_url(url);
            assert_eq!(config.url, url);
        }

        #[rstest]
        fn with_url_uses_default_for_other_fields() {
            let config = RedisConfig::with_url("redis://localhost:6379");
            assert_eq!(config.key_prefix, "dev:roguelike:");
            assert_eq!(config.default_ttl, Duration::from_secs(3600));
            assert_eq!(config.connection_timeout, Duration::from_secs(30));
        }

        #[rstest]
        fn with_key_prefix_sets_value() {
            let config =
                RedisConfig::with_url("redis://localhost:6379").with_key_prefix("prod:roguelike:");
            assert_eq!(config.key_prefix, "prod:roguelike:");
        }

        #[rstest]
        fn with_default_ttl_sets_value() {
            let config = RedisConfig::with_url("redis://localhost:6379")
                .with_default_ttl(Duration::from_secs(7200));
            assert_eq!(config.default_ttl, Duration::from_secs(7200));
        }

        #[rstest]
        fn with_connection_timeout_sets_value() {
            let config = RedisConfig::with_url("redis://localhost:6379")
                .with_connection_timeout(Duration::from_secs(60));
            assert_eq!(config.connection_timeout, Duration::from_secs(60));
        }

        #[rstest]
        fn builder_chain() {
            let config = RedisConfig::with_url("redis://localhost:6379")
                .with_key_prefix("prod:roguelike:")
                .with_default_ttl(Duration::from_secs(7200))
                .with_connection_timeout(Duration::from_secs(60));

            assert_eq!(config.url, "redis://localhost:6379");
            assert_eq!(config.key_prefix, "prod:roguelike:");
            assert_eq!(config.default_ttl, Duration::from_secs(7200));
            assert_eq!(config.connection_timeout, Duration::from_secs(60));
        }
    }

    // =========================================================================
    // Clone Tests
    // =========================================================================

    mod clone_tests {
        use super::*;

        #[rstest]
        fn clone_produces_equal_config() {
            let config = RedisConfig::with_url("redis://localhost:6379")
                .with_key_prefix("prod:roguelike:")
                .with_default_ttl(Duration::from_secs(7200));
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
            let config = RedisConfig::with_url("redis://localhost:6379");
            let debug_string = format!("{:?}", config);
            assert!(debug_string.contains("RedisConfig"));
            assert!(debug_string.contains("redis://localhost:6379"));
            assert!(debug_string.contains("key_prefix"));
            assert!(debug_string.contains("dev:roguelike:"));
        }
    }

    // =========================================================================
    // PartialEq Tests
    // =========================================================================

    mod equality_tests {
        use super::*;

        #[rstest]
        fn equal_configs_are_equal() {
            let config1 = RedisConfig::with_url("redis://localhost:6379");
            let config2 = RedisConfig::with_url("redis://localhost:6379");
            assert_eq!(config1, config2);
        }

        #[rstest]
        fn different_url_configs_are_not_equal() {
            let config1 = RedisConfig::with_url("redis://localhost:6379");
            let config2 = RedisConfig::with_url("redis://localhost:6380");
            assert_ne!(config1, config2);
        }

        #[rstest]
        fn different_key_prefix_configs_are_not_equal() {
            let config1 =
                RedisConfig::with_url("redis://localhost:6379").with_key_prefix("dev:roguelike:");
            let config2 =
                RedisConfig::with_url("redis://localhost:6379").with_key_prefix("prod:roguelike:");
            assert_ne!(config1, config2);
        }
    }
}
