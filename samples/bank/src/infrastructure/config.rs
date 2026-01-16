//! Application configuration management.
//!
//! This module provides configuration loading from environment variables
//! using a functional approach with explicit error handling.
//!
//! # Design
//!
//! - Configuration is loaded once at startup
//! - Missing or invalid values result in clear error messages
//! - All values are validated before use
//!
//! # Example
//!
//! ```rust,ignore
//! use bank::infrastructure::AppConfig;
//!
//! let config = AppConfig::from_env()?;
//! println!("Database URL: {}", config.database_url);
//! ```

use std::env;
use std::num::ParseIntError;

/// Configuration error types.
///
/// Represents errors that can occur when loading configuration
/// from environment variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// A required environment variable is not set.
    MissingEnvVar(String),
    /// An environment variable has an invalid value.
    InvalidValue {
        /// The name of the environment variable.
        key: String,
        /// Description of why the value is invalid.
        message: String,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEnvVar(key) => {
                write!(formatter, "Missing environment variable: {key}")
            }
            Self::InvalidValue { key, message } => {
                write!(formatter, "Invalid value for {key}: {message}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<env::VarError> for ConfigError {
    fn from(error: env::VarError) -> Self {
        match error {
            env::VarError::NotPresent => Self::MissingEnvVar("unknown".to_string()),
            env::VarError::NotUnicode(_) => Self::InvalidValue {
                key: "unknown".to_string(),
                message: "value is not valid Unicode".to_string(),
            },
        }
    }
}

/// Application configuration.
///
/// Contains all configuration values needed to run the bank application.
/// Values are loaded from environment variables using [`AppConfig::from_env`].
///
/// # Fields
///
/// - `database_url`: Postgres connection string
/// - `redis_url`: Redis connection string
/// - `sqs_endpoint`: AWS SQS endpoint (can be `LocalStack` for development)
/// - `sqs_events_queue_url`: URL for the events queue
/// - `sqs_projections_queue_url`: URL for the projections queue
/// - `snapshot_threshold`: Number of events before creating a snapshot
/// - `app_host`: Host address for the HTTP server
/// - `app_port`: Port number for the HTTP server
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppConfig {
    /// Postgres database connection URL.
    pub database_url: String,
    /// Redis connection URL for caching.
    pub redis_url: String,
    /// AWS SQS endpoint URL.
    pub sqs_endpoint: String,
    /// SQS queue URL for domain events.
    pub sqs_events_queue_url: String,
    /// SQS queue URL for projection updates.
    pub sqs_projections_queue_url: String,
    /// Number of events after which to create a snapshot.
    pub snapshot_threshold: u64,
    /// HTTP server host address.
    pub app_host: String,
    /// HTTP server port.
    pub app_port: u16,
}

impl AppConfig {
    /// Loads configuration from environment variables.
    ///
    /// Reads all required environment variables and validates their values.
    /// Returns an error if any required variable is missing or has an invalid value.
    ///
    /// # Environment Variables
    ///
    /// - `DATABASE_URL`: Postgres connection string (required)
    /// - `REDIS_URL`: Redis connection string (required)
    /// - `SQS_ENDPOINT`: SQS endpoint URL (required)
    /// - `SQS_EVENTS_QUEUE_URL`: Events queue URL (required)
    /// - `SQS_PROJECTIONS_QUEUE_URL`: Projections queue URL (required)
    /// - `SNAPSHOT_THRESHOLD`: Events before snapshot (optional, default: 100)
    /// - `APP_HOST`: Server host (optional, default: "0.0.0.0")
    /// - `APP_PORT`: Server port (optional, default: 8081)
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::MissingEnvVar` if a required variable is not set.
    /// Returns `ConfigError::InvalidValue` if a variable has an invalid value.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = AppConfig::from_env()?;
    /// ```
    pub fn from_env() -> Result<Self, ConfigError> {
        // Load .env file if present (ignores errors if file doesn't exist)
        dotenvy::dotenv().ok();

        let database_url = get_required_env("DATABASE_URL")?;
        let redis_url = get_required_env("REDIS_URL")?;
        let sqs_endpoint = get_required_env("SQS_ENDPOINT")?;
        let sqs_events_queue_url = get_required_env("SQS_EVENTS_QUEUE_URL")?;
        let sqs_projections_queue_url = get_required_env("SQS_PROJECTIONS_QUEUE_URL")?;

        let snapshot_threshold = get_optional_env_parsed("SNAPSHOT_THRESHOLD", 100)?;
        let app_host = get_optional_env("APP_HOST", "0.0.0.0".to_string());
        let app_port = get_optional_env_parsed("APP_PORT", 8081)?;

        Ok(Self {
            database_url,
            redis_url,
            sqs_endpoint,
            sqs_events_queue_url,
            sqs_projections_queue_url,
            snapshot_threshold,
            app_host,
            app_port,
        })
    }

    /// Creates a new `AppConfig` with the given values.
    ///
    /// This is useful for testing or when configuration is provided programmatically.
    ///
    /// # Arguments
    ///
    /// All fields of the configuration struct.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        database_url: String,
        redis_url: String,
        sqs_endpoint: String,
        sqs_events_queue_url: String,
        sqs_projections_queue_url: String,
        snapshot_threshold: u64,
        app_host: String,
        app_port: u16,
    ) -> Self {
        Self {
            database_url,
            redis_url,
            sqs_endpoint,
            sqs_events_queue_url,
            sqs_projections_queue_url,
            snapshot_threshold,
            app_host,
            app_port,
        }
    }
}

/// Gets a required environment variable.
///
/// # Errors
///
/// Returns `ConfigError::MissingEnvVar` if the variable is not set.
fn get_required_env(key: &str) -> Result<String, ConfigError> {
    env::var(key).map_err(|_| ConfigError::MissingEnvVar(key.to_string()))
}

/// Gets an optional environment variable with a default value.
fn get_optional_env(key: &str, default: String) -> String {
    env::var(key).unwrap_or(default)
}

/// Gets an optional environment variable and parses it, with a default value.
///
/// # Errors
///
/// Returns `ConfigError::InvalidValue` if the variable is set but cannot be parsed.
fn get_optional_env_parsed<T>(key: &str, default: T) -> Result<T, ConfigError>
where
    T: std::str::FromStr<Err = ParseIntError>,
{
    env::var(key).map_or_else(
        |_| Ok(default),
        |value| {
            value
                .parse()
                .map_err(|error: ParseIntError| ConfigError::InvalidValue {
                    key: key.to_string(),
                    message: error.to_string(),
                })
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // ConfigError Tests
    // =========================================================================

    #[rstest]
    fn config_error_missing_env_var_display() {
        let error = ConfigError::MissingEnvVar("TEST_VAR".to_string());
        assert_eq!(format!("{error}"), "Missing environment variable: TEST_VAR");
    }

    #[rstest]
    fn config_error_invalid_value_display() {
        let error = ConfigError::InvalidValue {
            key: "TEST_VAR".to_string(),
            message: "must be a number".to_string(),
        };
        assert_eq!(
            format!("{error}"),
            "Invalid value for TEST_VAR: must be a number"
        );
    }

    #[rstest]
    fn config_error_equality() {
        let error1 = ConfigError::MissingEnvVar("VAR1".to_string());
        let error2 = ConfigError::MissingEnvVar("VAR1".to_string());
        let error3 = ConfigError::MissingEnvVar("VAR2".to_string());

        assert_eq!(error1, error2);
        assert_ne!(error1, error3);
    }

    #[rstest]
    fn config_error_clone() {
        let original = ConfigError::InvalidValue {
            key: "KEY".to_string(),
            message: "message".to_string(),
        };
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[rstest]
    fn config_error_debug() {
        let error = ConfigError::MissingEnvVar("TEST".to_string());
        let debug_str = format!("{error:?}");

        assert!(debug_str.contains("MissingEnvVar"));
        assert!(debug_str.contains("TEST"));
    }

    #[rstest]
    fn config_error_is_error_trait() {
        fn assert_error<E: std::error::Error>(_: &E) {}

        let error = ConfigError::MissingEnvVar("test".to_string());
        assert_error(&error);
    }

    // =========================================================================
    // AppConfig::new Tests
    // =========================================================================

    #[rstest]
    fn app_config_new_creates_config() {
        let config = AppConfig::new(
            "postgres://localhost/bank".to_string(),
            "redis://localhost".to_string(),
            "http://sqs:4566".to_string(),
            "http://sqs:4566/events".to_string(),
            "http://sqs:4566/projections".to_string(),
            50,
            "127.0.0.1".to_string(),
            3000,
        );

        assert_eq!(config.database_url, "postgres://localhost/bank");
        assert_eq!(config.redis_url, "redis://localhost");
        assert_eq!(config.sqs_endpoint, "http://sqs:4566");
        assert_eq!(config.sqs_events_queue_url, "http://sqs:4566/events");
        assert_eq!(
            config.sqs_projections_queue_url,
            "http://sqs:4566/projections"
        );
        assert_eq!(config.snapshot_threshold, 50);
        assert_eq!(config.app_host, "127.0.0.1");
        assert_eq!(config.app_port, 3000);
    }

    #[rstest]
    fn app_config_clone() {
        let original = AppConfig::new(
            "postgres://localhost/bank".to_string(),
            "redis://localhost".to_string(),
            "http://sqs:4566".to_string(),
            "http://sqs:4566/events".to_string(),
            "http://sqs:4566/projections".to_string(),
            100,
            "0.0.0.0".to_string(),
            8081,
        );
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[rstest]
    fn app_config_debug() {
        let config = AppConfig::new(
            "postgres://localhost/bank".to_string(),
            "redis://localhost".to_string(),
            "http://sqs:4566".to_string(),
            "http://sqs:4566/events".to_string(),
            "http://sqs:4566/projections".to_string(),
            100,
            "0.0.0.0".to_string(),
            8081,
        );
        let debug_str = format!("{config:?}");

        assert!(debug_str.contains("AppConfig"));
        assert!(debug_str.contains("database_url"));
        assert!(debug_str.contains("postgres://localhost/bank"));
    }

    #[rstest]
    fn app_config_equality() {
        let config1 = AppConfig::new(
            "postgres://localhost/bank".to_string(),
            "redis://localhost".to_string(),
            "http://sqs:4566".to_string(),
            "http://sqs:4566/events".to_string(),
            "http://sqs:4566/projections".to_string(),
            100,
            "0.0.0.0".to_string(),
            8081,
        );
        let config2 = AppConfig::new(
            "postgres://localhost/bank".to_string(),
            "redis://localhost".to_string(),
            "http://sqs:4566".to_string(),
            "http://sqs:4566/events".to_string(),
            "http://sqs:4566/projections".to_string(),
            100,
            "0.0.0.0".to_string(),
            8081,
        );
        let config3 = AppConfig::new(
            "postgres://localhost/other".to_string(),
            "redis://localhost".to_string(),
            "http://sqs:4566".to_string(),
            "http://sqs:4566/events".to_string(),
            "http://sqs:4566/projections".to_string(),
            100,
            "0.0.0.0".to_string(),
            8081,
        );

        assert_eq!(config1, config2);
        assert_ne!(config1, config3);
    }

    // =========================================================================
    // ConfigError::from Tests
    // =========================================================================

    #[rstest]
    fn config_error_from_var_error_not_present() {
        let var_error = env::VarError::NotPresent;
        let config_error: ConfigError = var_error.into();

        match config_error {
            ConfigError::MissingEnvVar(key) => {
                assert_eq!(key, "unknown");
            }
            ConfigError::InvalidValue { .. } => panic!("Expected MissingEnvVar"),
        }
    }

    // Note: AppConfig::from_env tests are removed because they require
    // unsafe env::set_var/remove_var in Rust 2024 edition.
    // Integration tests should be used for environment variable testing.
}
