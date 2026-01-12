use thiserror::Error;

// =============================================================================
// ConnectionTarget
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionTarget {
    Database,
    Cache,
}

impl std::fmt::Display for ConnectionTarget {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database => write!(formatter, "database"),
            Self::Cache => write!(formatter, "cache"),
        }
    }
}

// =============================================================================
// InfraError
// =============================================================================

#[derive(Debug, Clone, Error)]
pub enum InfraError {
    #[error("Database error: {message}")]
    Database { message: String },

    #[error("Cache error: {message}")]
    Cache { message: String },

    #[error("Serialization error: {message}")]
    Serialization { message: String },

    #[error("{entity_type} with identifier '{identifier}' not found")]
    NotFound {
        entity_type: String,
        identifier: String,
    },

    #[error("Connection to {target} failed: {message}")]
    Connection {
        target: ConnectionTarget,
        message: String,
    },

    #[error("Operation timed out: {message}")]
    Timeout { message: String },

    #[error("Configuration error: {message}")]
    Configuration { message: String },
}

// =============================================================================
// Factory Methods
// =============================================================================

impl InfraError {
    #[must_use]
    pub fn database(message: impl Into<String>) -> Self {
        Self::Database {
            message: message.into(),
        }
    }

    #[must_use]
    pub fn cache(message: impl Into<String>) -> Self {
        Self::Cache {
            message: message.into(),
        }
    }

    #[must_use]
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization {
            message: message.into(),
        }
    }

    #[must_use]
    pub fn not_found(entity_type: impl Into<String>, identifier: impl Into<String>) -> Self {
        Self::NotFound {
            entity_type: entity_type.into(),
            identifier: identifier.into(),
        }
    }

    #[must_use]
    pub fn database_connection(message: impl Into<String>) -> Self {
        Self::Connection {
            target: ConnectionTarget::Database,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn cache_connection(message: impl Into<String>) -> Self {
        Self::Connection {
            target: ConnectionTarget::Cache,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::Timeout {
            message: message.into(),
        }
    }

    #[must_use]
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }
}

// =============================================================================
// Query Methods
// =============================================================================

impl InfraError {
    #[must_use]
    pub const fn is_database(&self) -> bool {
        matches!(self, Self::Database { .. })
    }

    #[must_use]
    pub const fn is_cache(&self) -> bool {
        matches!(self, Self::Cache { .. })
    }

    #[must_use]
    pub const fn is_serialization(&self) -> bool {
        matches!(self, Self::Serialization { .. })
    }

    #[must_use]
    pub const fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound { .. })
    }

    #[must_use]
    pub const fn is_connection(&self) -> bool {
        matches!(self, Self::Connection { .. })
    }

    #[must_use]
    pub const fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout { .. })
    }

    #[must_use]
    pub const fn is_configuration(&self) -> bool {
        matches!(self, Self::Configuration { .. })
    }

    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::Timeout { .. } | Self::Cache { .. } | Self::NotFound { .. }
        )
    }

    #[must_use]
    pub const fn connection_target(&self) -> Option<ConnectionTarget> {
        match self {
            Self::Connection { target, .. } => Some(*target),
            _ => None,
        }
    }
}

// =============================================================================
// From Implementations
// =============================================================================

impl From<sqlx::Error> for InfraError {
    fn from(error: sqlx::Error) -> Self {
        // Check for specific error types and convert appropriately
        match &error {
            sqlx::Error::RowNotFound => Self::NotFound {
                entity_type: "Entity".to_string(),
                identifier: "unknown".to_string(),
            },
            sqlx::Error::PoolTimedOut => Self::Timeout {
                message: "database pool connection timed out".to_string(),
            },
            sqlx::Error::Io(_) => Self::Connection {
                target: ConnectionTarget::Database,
                message: error.to_string(),
            },
            _ => Self::Database {
                message: error.to_string(),
            },
        }
    }
}

impl From<redis::RedisError> for InfraError {
    fn from(error: redis::RedisError) -> Self {
        // Check for specific error types
        if error.is_connection_refusal() || error.is_io_error() {
            Self::Connection {
                target: ConnectionTarget::Cache,
                message: error.to_string(),
            }
        } else if error.is_timeout() {
            Self::Timeout {
                message: format!("Redis operation timed out: {}", error),
            }
        } else {
            Self::Cache {
                message: error.to_string(),
            }
        }
    }
}

impl From<bincode::Error> for InfraError {
    fn from(error: bincode::Error) -> Self {
        Self::Serialization {
            message: format!("bincode: {}", error),
        }
    }
}

impl From<serde_json::Error> for InfraError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization {
            message: format!("JSON: {}", error),
        }
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
    // ConnectionTarget Tests
    // =========================================================================

    mod connection_target {
        use super::*;

        #[rstest]
        fn database_display() {
            let target = ConnectionTarget::Database;
            assert_eq!(format!("{}", target), "database");
        }

        #[rstest]
        fn cache_display() {
            let target = ConnectionTarget::Cache;
            assert_eq!(format!("{}", target), "cache");
        }

        #[rstest]
        fn equality() {
            assert_eq!(ConnectionTarget::Database, ConnectionTarget::Database);
            assert_eq!(ConnectionTarget::Cache, ConnectionTarget::Cache);
            assert_ne!(ConnectionTarget::Database, ConnectionTarget::Cache);
        }

        #[rstest]
        fn copy() {
            let target = ConnectionTarget::Database;
            let copied = target;
            assert_eq!(target, copied);
        }

        #[rstest]
        fn debug() {
            let debug_string = format!("{:?}", ConnectionTarget::Database);
            assert!(debug_string.contains("Database"));
        }
    }

    // =========================================================================
    // Factory Method Tests
    // =========================================================================

    mod factory_methods {
        use super::*;

        #[rstest]
        fn database_creates_error() {
            let error = InfraError::database("connection refused");

            match error {
                InfraError::Database { message } => {
                    assert_eq!(message, "connection refused");
                }
                _ => panic!("Expected Database variant"),
            }
        }

        #[rstest]
        fn cache_creates_error() {
            let error = InfraError::cache("cache miss");

            match error {
                InfraError::Cache { message } => {
                    assert_eq!(message, "cache miss");
                }
                _ => panic!("Expected Cache variant"),
            }
        }

        #[rstest]
        fn serialization_creates_error() {
            let error = InfraError::serialization("invalid JSON format");

            match error {
                InfraError::Serialization { message } => {
                    assert_eq!(message, "invalid JSON format");
                }
                _ => panic!("Expected Serialization variant"),
            }
        }

        #[rstest]
        fn not_found_creates_error() {
            let error = InfraError::not_found("GameSession", "abc-123");

            match error {
                InfraError::NotFound {
                    entity_type,
                    identifier,
                } => {
                    assert_eq!(entity_type, "GameSession");
                    assert_eq!(identifier, "abc-123");
                }
                _ => panic!("Expected NotFound variant"),
            }
        }

        #[rstest]
        fn database_connection_creates_error() {
            let error = InfraError::database_connection("connection refused");

            match error {
                InfraError::Connection { target, message } => {
                    assert_eq!(target, ConnectionTarget::Database);
                    assert_eq!(message, "connection refused");
                }
                _ => panic!("Expected Connection variant"),
            }
        }

        #[rstest]
        fn cache_connection_creates_error() {
            let error = InfraError::cache_connection("connection refused");

            match error {
                InfraError::Connection { target, message } => {
                    assert_eq!(target, ConnectionTarget::Cache);
                    assert_eq!(message, "connection refused");
                }
                _ => panic!("Expected Connection variant"),
            }
        }

        #[rstest]
        fn timeout_creates_error() {
            let error = InfraError::timeout("query exceeded 30 seconds");

            match error {
                InfraError::Timeout { message } => {
                    assert_eq!(message, "query exceeded 30 seconds");
                }
                _ => panic!("Expected Timeout variant"),
            }
        }

        #[rstest]
        fn configuration_creates_error() {
            let error = InfraError::configuration("missing DATABASE_URL");

            match error {
                InfraError::Configuration { message } => {
                    assert_eq!(message, "missing DATABASE_URL");
                }
                _ => panic!("Expected Configuration variant"),
            }
        }
    }

    // =========================================================================
    // Query Method Tests
    // =========================================================================

    mod query_methods {
        use super::*;

        #[rstest]
        fn is_database_returns_true_for_database() {
            let error = InfraError::database("error");
            assert!(error.is_database());
        }

        #[rstest]
        fn is_database_returns_false_for_others() {
            let error = InfraError::cache("error");
            assert!(!error.is_database());
        }

        #[rstest]
        fn is_cache_returns_true_for_cache() {
            let error = InfraError::cache("error");
            assert!(error.is_cache());
        }

        #[rstest]
        fn is_cache_returns_false_for_others() {
            let error = InfraError::database("error");
            assert!(!error.is_cache());
        }

        #[rstest]
        fn is_serialization_returns_true_for_serialization() {
            let error = InfraError::serialization("error");
            assert!(error.is_serialization());
        }

        #[rstest]
        fn is_serialization_returns_false_for_others() {
            let error = InfraError::database("error");
            assert!(!error.is_serialization());
        }

        #[rstest]
        fn is_not_found_returns_true_for_not_found() {
            let error = InfraError::not_found("Entity", "id");
            assert!(error.is_not_found());
        }

        #[rstest]
        fn is_not_found_returns_false_for_others() {
            let error = InfraError::database("error");
            assert!(!error.is_not_found());
        }

        #[rstest]
        fn is_connection_returns_true_for_connection() {
            let error = InfraError::database_connection("error");
            assert!(error.is_connection());
        }

        #[rstest]
        fn is_connection_returns_false_for_others() {
            let error = InfraError::database("error");
            assert!(!error.is_connection());
        }

        #[rstest]
        fn is_timeout_returns_true_for_timeout() {
            let error = InfraError::timeout("error");
            assert!(error.is_timeout());
        }

        #[rstest]
        fn is_timeout_returns_false_for_others() {
            let error = InfraError::database("error");
            assert!(!error.is_timeout());
        }

        #[rstest]
        fn is_configuration_returns_true_for_configuration() {
            let error = InfraError::configuration("error");
            assert!(error.is_configuration());
        }

        #[rstest]
        fn is_configuration_returns_false_for_others() {
            let error = InfraError::database("error");
            assert!(!error.is_configuration());
        }

        #[rstest]
        fn connection_target_returns_database_for_database_connection() {
            let error = InfraError::database_connection("error");
            assert_eq!(error.connection_target(), Some(ConnectionTarget::Database));
        }

        #[rstest]
        fn connection_target_returns_cache_for_cache_connection() {
            let error = InfraError::cache_connection("error");
            assert_eq!(error.connection_target(), Some(ConnectionTarget::Cache));
        }

        #[rstest]
        fn connection_target_returns_none_for_non_connection_errors() {
            let error = InfraError::database("error");
            assert_eq!(error.connection_target(), None);
        }
    }

    // =========================================================================
    // Recoverability Tests
    // =========================================================================

    mod recoverability {
        use super::*;

        #[rstest]
        fn timeout_is_recoverable() {
            let error = InfraError::timeout("query exceeded 30 seconds");
            assert!(error.is_recoverable());
        }

        #[rstest]
        fn cache_is_recoverable() {
            let error = InfraError::cache("cache miss");
            assert!(error.is_recoverable());
        }

        #[rstest]
        fn not_found_is_recoverable() {
            let error = InfraError::not_found("GameSession", "abc");
            assert!(error.is_recoverable());
        }

        #[rstest]
        fn database_is_not_recoverable() {
            let error = InfraError::database("query failed");
            assert!(!error.is_recoverable());
        }

        #[rstest]
        fn serialization_is_not_recoverable() {
            let error = InfraError::serialization("invalid JSON");
            assert!(!error.is_recoverable());
        }

        #[rstest]
        fn connection_is_not_recoverable() {
            let error = InfraError::database_connection("connection refused");
            assert!(!error.is_recoverable());
        }

        #[rstest]
        fn configuration_is_not_recoverable() {
            let error = InfraError::configuration("missing DATABASE_URL");
            assert!(!error.is_recoverable());
        }
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    mod display {
        use super::*;

        #[rstest]
        fn database_display() {
            let error = InfraError::database("connection refused");
            let display = format!("{}", error);
            assert_eq!(display, "Database error: connection refused");
        }

        #[rstest]
        fn cache_display() {
            let error = InfraError::cache("cache miss");
            let display = format!("{}", error);
            assert_eq!(display, "Cache error: cache miss");
        }

        #[rstest]
        fn serialization_display() {
            let error = InfraError::serialization("invalid JSON format");
            let display = format!("{}", error);
            assert_eq!(display, "Serialization error: invalid JSON format");
        }

        #[rstest]
        fn not_found_display() {
            let error = InfraError::not_found("GameSession", "abc-123");
            let display = format!("{}", error);
            assert_eq!(display, "GameSession with identifier 'abc-123' not found");
        }

        #[rstest]
        fn database_connection_display() {
            let error = InfraError::database_connection("connection refused");
            let display = format!("{}", error);
            assert_eq!(display, "Connection to database failed: connection refused");
        }

        #[rstest]
        fn cache_connection_display() {
            let error = InfraError::cache_connection("connection refused");
            let display = format!("{}", error);
            assert_eq!(display, "Connection to cache failed: connection refused");
        }

        #[rstest]
        fn timeout_display() {
            let error = InfraError::timeout("query exceeded 30 seconds");
            let display = format!("{}", error);
            assert_eq!(display, "Operation timed out: query exceeded 30 seconds");
        }

        #[rstest]
        fn configuration_display() {
            let error = InfraError::configuration("missing DATABASE_URL");
            let display = format!("{}", error);
            assert_eq!(display, "Configuration error: missing DATABASE_URL");
        }
    }

    // =========================================================================
    // Clone Tests
    // =========================================================================

    mod clone_tests {
        use super::*;

        #[rstest]
        fn database_clone() {
            let error = InfraError::database("error");
            let cloned = error.clone();
            assert_eq!(format!("{}", error), format!("{}", cloned));
        }

        #[rstest]
        fn cache_clone() {
            let error = InfraError::cache("error");
            let cloned = error.clone();
            assert_eq!(format!("{}", error), format!("{}", cloned));
        }

        #[rstest]
        fn serialization_clone() {
            let error = InfraError::serialization("error");
            let cloned = error.clone();
            assert_eq!(format!("{}", error), format!("{}", cloned));
        }

        #[rstest]
        fn not_found_clone() {
            let error = InfraError::not_found("Entity", "id");
            let cloned = error.clone();
            assert_eq!(format!("{}", error), format!("{}", cloned));
        }

        #[rstest]
        fn connection_clone() {
            let error = InfraError::database_connection("error");
            let cloned = error.clone();
            assert_eq!(format!("{}", error), format!("{}", cloned));
        }

        #[rstest]
        fn timeout_clone() {
            let error = InfraError::timeout("error");
            let cloned = error.clone();
            assert_eq!(format!("{}", error), format!("{}", cloned));
        }

        #[rstest]
        fn configuration_clone() {
            let error = InfraError::configuration("error");
            let cloned = error.clone();
            assert_eq!(format!("{}", error), format!("{}", cloned));
        }
    }

    // =========================================================================
    // Debug Tests
    // =========================================================================

    mod debug_tests {
        use super::*;

        #[rstest]
        fn database_debug() {
            let error = InfraError::database("connection refused");
            let debug_string = format!("{:?}", error);
            assert!(debug_string.contains("Database"));
            assert!(debug_string.contains("connection refused"));
        }

        #[rstest]
        fn cache_debug() {
            let error = InfraError::cache("cache miss");
            let debug_string = format!("{:?}", error);
            assert!(debug_string.contains("Cache"));
            assert!(debug_string.contains("cache miss"));
        }

        #[rstest]
        fn serialization_debug() {
            let error = InfraError::serialization("invalid JSON");
            let debug_string = format!("{:?}", error);
            assert!(debug_string.contains("Serialization"));
            assert!(debug_string.contains("invalid JSON"));
        }

        #[rstest]
        fn not_found_debug() {
            let error = InfraError::not_found("GameSession", "abc-123");
            let debug_string = format!("{:?}", error);
            assert!(debug_string.contains("NotFound"));
            assert!(debug_string.contains("GameSession"));
            assert!(debug_string.contains("abc-123"));
        }

        #[rstest]
        fn connection_debug() {
            let error = InfraError::database_connection("connection refused");
            let debug_string = format!("{:?}", error);
            assert!(debug_string.contains("Connection"));
            assert!(debug_string.contains("Database"));
        }

        #[rstest]
        fn timeout_debug() {
            let error = InfraError::timeout("query exceeded 30 seconds");
            let debug_string = format!("{:?}", error);
            assert!(debug_string.contains("Timeout"));
            assert!(debug_string.contains("query exceeded 30 seconds"));
        }

        #[rstest]
        fn configuration_debug() {
            let error = InfraError::configuration("missing DATABASE_URL");
            let debug_string = format!("{:?}", error);
            assert!(debug_string.contains("Configuration"));
            assert!(debug_string.contains("missing DATABASE_URL"));
        }
    }

    // =========================================================================
    // From Trait Tests
    // =========================================================================

    mod from_trait {
        use super::*;

        #[rstest]
        fn from_serde_json_error() {
            let json_error: serde_json::Error =
                serde_json::from_str::<String>("invalid").unwrap_err();
            let infra_error: InfraError = json_error.into();

            assert!(infra_error.is_serialization());
            let display = format!("{}", infra_error);
            assert!(display.contains("JSON"));
        }

        #[rstest]
        fn from_sqlx_row_not_found() {
            let sqlx_error = sqlx::Error::RowNotFound;
            let infra_error: InfraError = sqlx_error.into();

            assert!(infra_error.is_not_found());
        }

        #[rstest]
        fn from_sqlx_pool_timed_out() {
            let sqlx_error = sqlx::Error::PoolTimedOut;
            let infra_error: InfraError = sqlx_error.into();

            assert!(infra_error.is_timeout());
        }
    }

    // =========================================================================
    // Error Trait Tests
    // =========================================================================

    mod error_trait {
        use super::*;
        use std::error::Error;

        #[rstest]
        fn database_implements_error() {
            let error = InfraError::database("test");
            let _: &dyn Error = &error;
        }

        #[rstest]
        fn cache_implements_error() {
            let error = InfraError::cache("test");
            let _: &dyn Error = &error;
        }

        #[rstest]
        fn serialization_implements_error() {
            let error = InfraError::serialization("test");
            let _: &dyn Error = &error;
        }

        #[rstest]
        fn not_found_implements_error() {
            let error = InfraError::not_found("Entity", "id");
            let _: &dyn Error = &error;
        }

        #[rstest]
        fn connection_implements_error() {
            let error = InfraError::database_connection("test");
            let _: &dyn Error = &error;
        }

        #[rstest]
        fn timeout_implements_error() {
            let error = InfraError::timeout("test");
            let _: &dyn Error = &error;
        }

        #[rstest]
        fn configuration_implements_error() {
            let error = InfraError::configuration("test");
            let _: &dyn Error = &error;
        }
    }
}
