//! Repository factory for runtime backend selection.
//!
//! This module provides a factory pattern for creating repository instances
//! based on environment configuration. It supports switching between `InMemory`,
//! Redis, and `PostgreSQL` backends at runtime.
//!
//! # Environment Variables
//!
//! - `STORAGE_MODE`: `in_memory` (default) | `postgres`
//! - `CACHE_MODE`: `in_memory` (default) | `redis`
//! - `DATABASE_URL`: `PostgreSQL` connection URL (required when `STORAGE_MODE=postgres`)
//! - `REDIS_URL`: Redis connection URL (required when `CACHE_MODE=redis`)
//!
//! # Example
//!
//! ```ignore
//! use infrastructure::factory::{RepositoryConfig, RepositoryFactory};
//!
//! // Load configuration from environment
//! let config = RepositoryConfig::from_env()?;
//!
//! // Create factory and initialize repositories
//! let factory = RepositoryFactory::new(config);
//! let repositories = factory.create().await?;
//!
//! // Use the repositories
//! let task = repositories.task_repository.find_by_id(&task_id).run_async().await?;
//! ```

use std::env;
use std::str::FromStr;
use std::sync::Arc;

use sqlx::PgPool;
use thiserror::Error;

use super::{
    EventStore, InMemoryEventStore, InMemoryProjectRepository, InMemoryTaskRepository,
    PostgresEventStore, PostgresProjectRepository, PostgresTaskRepository, ProjectRepository,
    RedisProjectRepository, RedisTaskRepository, TaskRepository,
};

// =============================================================================
// Configuration Types
// =============================================================================

/// Storage mode for persistent data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StorageMode {
    /// In-memory storage using persistent data structures.
    /// Suitable for testing and development.
    #[default]
    InMemory,
    /// `PostgreSQL` storage for production use.
    Postgres,
}

impl FromStr for StorageMode {
    type Err = ConfigurationError;

    /// Parses a storage mode from a string.
    ///
    /// # Errors
    ///
    /// Returns `ConfigurationError::InvalidStorageMode` if the string is not recognized.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "in_memory" | "inmemory" | "memory" => Ok(Self::InMemory),
            "postgres" | "postgresql" | "pg" => Ok(Self::Postgres),
            _ => Err(ConfigurationError::InvalidStorageMode(value.to_string())),
        }
    }
}

/// Cache mode for read-optimized data access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CacheMode {
    /// In-memory cache using persistent data structures.
    /// Suitable for testing and development.
    #[default]
    InMemory,
    /// Redis cache for production use.
    Redis,
}

impl FromStr for CacheMode {
    type Err = ConfigurationError;

    /// Parses a cache mode from a string.
    ///
    /// # Errors
    ///
    /// Returns `ConfigurationError::InvalidCacheMode` if the string is not recognized.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "in_memory" | "inmemory" | "memory" => Ok(Self::InMemory),
            "redis" => Ok(Self::Redis),
            _ => Err(ConfigurationError::InvalidCacheMode(value.to_string())),
        }
    }
}

/// Configuration for repository factory.
///
/// This struct holds all configuration needed to create repositories.
/// Use `RepositoryConfigBuilder` for a fluent API to construct this.
#[derive(Debug, Clone, Default)]
pub struct RepositoryConfig {
    /// Storage mode for persistent data (Tasks, Projects, Events).
    pub storage_mode: StorageMode,
    /// Cache mode for read-optimized access.
    pub cache_mode: CacheMode,
    /// `PostgreSQL` connection URL (required when `storage_mode` is `Postgres`).
    pub database_url: Option<String>,
    /// Redis connection URL (required when `cache_mode` is `Redis`).
    pub redis_url: Option<String>,
}

impl RepositoryConfig {
    /// Creates a new configuration builder.
    pub fn builder() -> RepositoryConfigBuilder {
        RepositoryConfigBuilder::default()
    }

    /// Creates a configuration from environment variables.
    ///
    /// # Environment Variables
    ///
    /// - `STORAGE_MODE`: `in_memory` (default) | `postgres`
    /// - `CACHE_MODE`: `in_memory` (default) | `redis`
    /// - `DATABASE_URL`: `PostgreSQL` connection URL
    /// - `REDIS_URL`: Redis connection URL
    ///
    /// # Errors
    ///
    /// Returns `ConfigurationError` if:
    /// - `STORAGE_MODE` or `CACHE_MODE` contains an invalid value
    /// - `DATABASE_URL` is missing when `STORAGE_MODE=postgres`
    /// - `REDIS_URL` is missing when `CACHE_MODE=redis`
    pub fn from_env() -> Result<Self, ConfigurationError> {
        let storage_mode = match env::var("STORAGE_MODE") {
            Ok(value) => value.parse()?,
            Err(env::VarError::NotPresent) => StorageMode::default(),
            Err(env::VarError::NotUnicode(_)) => {
                return Err(ConfigurationError::InvalidStorageMode(
                    "<non-UTF-8 value>".to_string(),
                ))
            }
        };

        let cache_mode = match env::var("CACHE_MODE") {
            Ok(value) => value.parse()?,
            Err(env::VarError::NotPresent) => CacheMode::default(),
            Err(env::VarError::NotUnicode(_)) => {
                return Err(ConfigurationError::InvalidCacheMode(
                    "<non-UTF-8 value>".to_string(),
                ))
            }
        };

        // Parse URLs, treating empty/whitespace-only as None
        let database_url = env::var("DATABASE_URL")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let redis_url = env::var("REDIS_URL")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let config = Self {
            storage_mode,
            cache_mode,
            database_url,
            redis_url,
        };

        config.validate()?;
        Ok(config)
    }

    /// Validates the configuration.
    ///
    /// # Errors
    ///
    /// Returns `ConfigurationError` if required URLs are missing for the selected modes.
    pub fn validate(&self) -> Result<(), ConfigurationError> {
        if self.storage_mode == StorageMode::Postgres && self.database_url.is_none() {
            return Err(ConfigurationError::MissingDatabaseUrl);
        }

        if self.cache_mode == CacheMode::Redis && self.redis_url.is_none() {
            return Err(ConfigurationError::MissingRedisUrl);
        }

        Ok(())
    }
}

/// Builder for `RepositoryConfig`.
///
/// Provides a fluent API for constructing configuration.
///
/// # Example
///
/// ```ignore
/// let config = RepositoryConfig::builder()
///     .storage_mode(StorageMode::Postgres)
///     .database_url("postgres://localhost/mydb")
///     .cache_mode(CacheMode::Redis)
///     .redis_url("redis://localhost:6379")
///     .build()?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct RepositoryConfigBuilder {
    storage_mode: StorageMode,
    cache_mode: CacheMode,
    database_url: Option<String>,
    redis_url: Option<String>,
}

impl RepositoryConfigBuilder {
    /// Sets the storage mode.
    #[must_use]
    pub const fn storage_mode(mut self, mode: StorageMode) -> Self {
        self.storage_mode = mode;
        self
    }

    /// Sets the cache mode.
    #[must_use]
    pub const fn cache_mode(mut self, mode: CacheMode) -> Self {
        self.cache_mode = mode;
        self
    }

    /// Sets the `PostgreSQL` database URL.
    #[must_use]
    pub fn database_url(mut self, url: impl Into<String>) -> Self {
        self.database_url = Some(url.into());
        self
    }

    /// Sets the Redis URL.
    #[must_use]
    pub fn redis_url(mut self, url: impl Into<String>) -> Self {
        self.redis_url = Some(url.into());
        self
    }

    /// Builds the configuration.
    ///
    /// # Errors
    ///
    /// Returns `ConfigurationError` if the configuration is invalid.
    pub fn build(self) -> Result<RepositoryConfig, ConfigurationError> {
        let config = RepositoryConfig {
            storage_mode: self.storage_mode,
            cache_mode: self.cache_mode,
            database_url: self.database_url,
            redis_url: self.redis_url,
        };

        config.validate()?;
        Ok(config)
    }
}

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during factory configuration and initialization.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ConfigurationError {
    /// Invalid storage mode value.
    #[error("Invalid storage mode: '{0}'. Expected 'in_memory' or 'postgres'")]
    InvalidStorageMode(String),

    /// Invalid cache mode value.
    #[error("Invalid cache mode: '{0}'. Expected 'in_memory' or 'redis'")]
    InvalidCacheMode(String),

    /// Missing `DATABASE_URL` when storage mode is Postgres.
    #[error("DATABASE_URL environment variable is required when STORAGE_MODE=postgres")]
    MissingDatabaseUrl,

    /// Missing `REDIS_URL` when cache mode is Redis.
    #[error("REDIS_URL environment variable is required when CACHE_MODE=redis")]
    MissingRedisUrl,
}

/// Errors that can occur during factory initialization.
#[derive(Debug, Error)]
pub enum FactoryError {
    /// Configuration error.
    #[error("Configuration error: {0}")]
    Configuration(#[from] ConfigurationError),

    /// Database connection error.
    #[error("Database connection error: {0}")]
    DatabaseConnection(String),

    /// Redis connection error.
    #[error("Redis connection error: {0}")]
    RedisConnection(String),
}

// =============================================================================
// Repository Factory
// =============================================================================

/// Collection of initialized repositories.
///
/// This struct holds all repository instances created by the factory.
/// All repositories are wrapped in `Arc` to allow sharing across threads.
/// The `Send + Sync` bounds ensure thread safety for multi-threaded runtimes.
#[derive(Clone)]
pub struct Repositories {
    /// Task repository for task CRUD operations.
    pub task_repository: Arc<dyn TaskRepository + Send + Sync>,
    /// Project repository for project CRUD operations.
    pub project_repository: Arc<dyn ProjectRepository + Send + Sync>,
    /// Event store for task event sourcing.
    pub event_store: Arc<dyn EventStore + Send + Sync>,
}

impl std::fmt::Debug for Repositories {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Repositories")
            .field("task_repository", &"Arc<dyn TaskRepository>")
            .field("project_repository", &"Arc<dyn ProjectRepository>")
            .field("event_store", &"Arc<dyn EventStore>")
            .finish()
    }
}

/// Factory for creating repository instances based on configuration.
///
/// The factory handles the initialization of database connections and
/// creates appropriate repository implementations based on the configured
/// storage and cache modes.
///
/// # Example
///
/// ```ignore
/// let config = RepositoryConfig::from_env()?;
/// let factory = RepositoryFactory::new(config);
/// let repositories = factory.create().await?;
/// ```
#[derive(Debug, Clone)]
pub struct RepositoryFactory {
    config: RepositoryConfig,
}

impl RepositoryFactory {
    /// Creates a new repository factory with the given configuration.
    #[must_use]
    pub const fn new(config: RepositoryConfig) -> Self {
        Self { config }
    }

    /// Creates a new repository factory from environment variables.
    ///
    /// # Errors
    ///
    /// Returns `FactoryError::Configuration` if environment configuration is invalid.
    pub fn from_env() -> Result<Self, FactoryError> {
        let config = RepositoryConfig::from_env()?;
        Ok(Self::new(config))
    }

    /// Returns the configuration used by this factory.
    #[must_use]
    pub const fn config(&self) -> &RepositoryConfig {
        &self.config
    }

    /// Creates all repositories based on the configuration.
    ///
    /// This method initializes database connections as needed and creates
    /// the appropriate repository implementations.
    ///
    /// # Errors
    ///
    /// Returns `FactoryError` if:
    /// - Database connection fails (when `storage_mode` is `Postgres`)
    /// - Redis connection fails (when `cache_mode` is `Redis`)
    pub async fn create(&self) -> Result<Repositories, FactoryError> {
        match (self.config.storage_mode, self.config.cache_mode) {
            // All in-memory: simple case, no external connections needed
            (StorageMode::InMemory, CacheMode::InMemory) => {
                Ok(Self::create_in_memory_repositories())
            }

            // PostgreSQL for storage, in-memory for cache
            (StorageMode::Postgres, CacheMode::InMemory) => {
                let pool = self.create_postgres_pool().await?;
                Ok(Self::create_postgres_repositories(pool))
            }

            // In-memory for storage, Redis for cache
            // This combination uses Redis as the primary repository since it has cache capabilities
            (StorageMode::InMemory, CacheMode::Redis) => {
                let redis_repositories = self.create_redis_repositories()?;
                // For this combination, we use Redis for Task and Project,
                // but keep EventStore in-memory since Redis doesn't implement it
                Ok(Repositories {
                    task_repository: redis_repositories.task_repository,
                    project_repository: redis_repositories.project_repository,
                    event_store: Arc::new(InMemoryEventStore::new()),
                })
            }

            // PostgreSQL for storage, Redis for cache
            // This is the full production configuration
            (StorageMode::Postgres, CacheMode::Redis) => {
                let pool = self.create_postgres_pool().await?;
                let postgres_repositories = Self::create_postgres_repositories(pool);
                let redis_repositories = self.create_redis_repositories()?;

                // Use Redis for Task and Project (with cache), PostgreSQL for EventStore
                Ok(Repositories {
                    task_repository: redis_repositories.task_repository,
                    project_repository: redis_repositories.project_repository,
                    event_store: postgres_repositories.event_store,
                })
            }
        }
    }

    /// Creates in-memory repositories.
    fn create_in_memory_repositories() -> Repositories {
        Repositories {
            task_repository: Arc::new(InMemoryTaskRepository::new()),
            project_repository: Arc::new(InMemoryProjectRepository::new()),
            event_store: Arc::new(InMemoryEventStore::new()),
        }
    }

    /// Creates a `PostgreSQL` connection pool.
    async fn create_postgres_pool(&self) -> Result<PgPool, FactoryError> {
        let database_url = self
            .config
            .database_url
            .as_ref()
            .ok_or(ConfigurationError::MissingDatabaseUrl)?;

        PgPool::connect(database_url)
            .await
            .map_err(|error| FactoryError::DatabaseConnection(error.to_string()))
    }

    /// Creates `PostgreSQL`-backed repositories.
    fn create_postgres_repositories(pool: PgPool) -> Repositories {
        Repositories {
            task_repository: Arc::new(PostgresTaskRepository::new(pool.clone())),
            project_repository: Arc::new(PostgresProjectRepository::new(pool.clone())),
            event_store: Arc::new(PostgresEventStore::new(pool)),
        }
    }

    /// Creates Redis-backed repositories.
    fn create_redis_repositories(&self) -> Result<RedisRepositories, FactoryError> {
        let redis_url = self
            .config
            .redis_url
            .as_ref()
            .ok_or(ConfigurationError::MissingRedisUrl)?;

        let task_repository = RedisTaskRepository::from_url(redis_url)
            .map_err(|error| FactoryError::RedisConnection(error.to_string()))?;

        let project_repository = RedisProjectRepository::from_url(redis_url)
            .map_err(|error| FactoryError::RedisConnection(error.to_string()))?;

        Ok(RedisRepositories {
            task_repository: Arc::new(task_repository),
            project_repository: Arc::new(project_repository),
        })
    }
}

/// Helper struct for Redis repositories (since Redis doesn't implement `EventStore`).
struct RedisRepositories {
    task_repository: Arc<dyn TaskRepository>,
    project_repository: Arc<dyn ProjectRepository>,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // StorageMode Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[case("in_memory", StorageMode::InMemory)]
    #[case("inmemory", StorageMode::InMemory)]
    #[case("memory", StorageMode::InMemory)]
    #[case("IN_MEMORY", StorageMode::InMemory)]
    #[case("postgres", StorageMode::Postgres)]
    #[case("postgresql", StorageMode::Postgres)]
    #[case("pg", StorageMode::Postgres)]
    #[case("POSTGRES", StorageMode::Postgres)]
    fn test_storage_mode_from_str_valid(#[case] input: &str, #[case] expected: StorageMode) {
        let result: Result<StorageMode, _> = input.parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }

    #[rstest]
    #[case("invalid")]
    #[case("mysql")]
    #[case("")]
    fn test_storage_mode_from_str_invalid(#[case] input: &str) {
        let result: Result<StorageMode, _> = input.parse();
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigurationError::InvalidStorageMode(value) => {
                assert_eq!(value, input);
            }
            _ => panic!("Expected InvalidStorageMode error"),
        }
    }

    #[rstest]
    fn test_storage_mode_default() {
        assert_eq!(StorageMode::default(), StorageMode::InMemory);
    }

    // -------------------------------------------------------------------------
    // CacheMode Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[case("in_memory", CacheMode::InMemory)]
    #[case("inmemory", CacheMode::InMemory)]
    #[case("memory", CacheMode::InMemory)]
    #[case("IN_MEMORY", CacheMode::InMemory)]
    #[case("redis", CacheMode::Redis)]
    #[case("REDIS", CacheMode::Redis)]
    fn test_cache_mode_from_str_valid(#[case] input: &str, #[case] expected: CacheMode) {
        let result: Result<CacheMode, _> = input.parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }

    #[rstest]
    #[case("invalid")]
    #[case("memcached")]
    #[case("")]
    fn test_cache_mode_from_str_invalid(#[case] input: &str) {
        let result: Result<CacheMode, _> = input.parse();
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigurationError::InvalidCacheMode(value) => {
                assert_eq!(value, input);
            }
            _ => panic!("Expected InvalidCacheMode error"),
        }
    }

    #[rstest]
    fn test_cache_mode_default() {
        assert_eq!(CacheMode::default(), CacheMode::InMemory);
    }

    // -------------------------------------------------------------------------
    // RepositoryConfig Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_repository_config_default() {
        let config = RepositoryConfig::default();
        assert_eq!(config.storage_mode, StorageMode::InMemory);
        assert_eq!(config.cache_mode, CacheMode::InMemory);
        assert!(config.database_url.is_none());
        assert!(config.redis_url.is_none());
    }

    #[rstest]
    fn test_repository_config_validate_in_memory() {
        let config = RepositoryConfig {
            storage_mode: StorageMode::InMemory,
            cache_mode: CacheMode::InMemory,
            database_url: None,
            redis_url: None,
        };
        assert!(config.validate().is_ok());
    }

    #[rstest]
    fn test_repository_config_validate_postgres_without_url() {
        let config = RepositoryConfig {
            storage_mode: StorageMode::Postgres,
            cache_mode: CacheMode::InMemory,
            database_url: None,
            redis_url: None,
        };
        let result = config.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ConfigurationError::MissingDatabaseUrl);
    }

    #[rstest]
    fn test_repository_config_validate_postgres_with_url() {
        let config = RepositoryConfig {
            storage_mode: StorageMode::Postgres,
            cache_mode: CacheMode::InMemory,
            database_url: Some("postgres://localhost/test".to_string()),
            redis_url: None,
        };
        assert!(config.validate().is_ok());
    }

    #[rstest]
    fn test_repository_config_validate_redis_without_url() {
        let config = RepositoryConfig {
            storage_mode: StorageMode::InMemory,
            cache_mode: CacheMode::Redis,
            database_url: None,
            redis_url: None,
        };
        let result = config.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ConfigurationError::MissingRedisUrl);
    }

    #[rstest]
    fn test_repository_config_validate_redis_with_url() {
        let config = RepositoryConfig {
            storage_mode: StorageMode::InMemory,
            cache_mode: CacheMode::Redis,
            database_url: None,
            redis_url: Some("redis://localhost:6379".to_string()),
        };
        assert!(config.validate().is_ok());
    }

    #[rstest]
    fn test_repository_config_validate_full_production() {
        let config = RepositoryConfig {
            storage_mode: StorageMode::Postgres,
            cache_mode: CacheMode::Redis,
            database_url: Some("postgres://localhost/test".to_string()),
            redis_url: Some("redis://localhost:6379".to_string()),
        };
        assert!(config.validate().is_ok());
    }

    // -------------------------------------------------------------------------
    // RepositoryConfigBuilder Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_builder_default() {
        let result = RepositoryConfig::builder().build();
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.storage_mode, StorageMode::InMemory);
        assert_eq!(config.cache_mode, CacheMode::InMemory);
    }

    #[rstest]
    fn test_builder_with_storage_mode() {
        let result = RepositoryConfig::builder()
            .storage_mode(StorageMode::Postgres)
            .database_url("postgres://localhost/test")
            .build();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().storage_mode, StorageMode::Postgres);
    }

    #[rstest]
    fn test_builder_with_cache_mode() {
        let result = RepositoryConfig::builder()
            .cache_mode(CacheMode::Redis)
            .redis_url("redis://localhost:6379")
            .build();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().cache_mode, CacheMode::Redis);
    }

    #[rstest]
    fn test_builder_full_configuration() {
        let result = RepositoryConfig::builder()
            .storage_mode(StorageMode::Postgres)
            .database_url("postgres://localhost/test")
            .cache_mode(CacheMode::Redis)
            .redis_url("redis://localhost:6379")
            .build();
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.storage_mode, StorageMode::Postgres);
        assert_eq!(config.cache_mode, CacheMode::Redis);
        assert_eq!(
            config.database_url,
            Some("postgres://localhost/test".to_string())
        );
        assert_eq!(config.redis_url, Some("redis://localhost:6379".to_string()));
    }

    #[rstest]
    fn test_builder_missing_database_url() {
        let result = RepositoryConfig::builder()
            .storage_mode(StorageMode::Postgres)
            .build();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ConfigurationError::MissingDatabaseUrl);
    }

    #[rstest]
    fn test_builder_missing_redis_url() {
        let result = RepositoryConfig::builder()
            .cache_mode(CacheMode::Redis)
            .build();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ConfigurationError::MissingRedisUrl);
    }

    // -------------------------------------------------------------------------
    // RepositoryFactory Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_repository_factory_new() {
        let config = RepositoryConfig::default();
        let factory = RepositoryFactory::new(config.clone());
        assert_eq!(factory.config().storage_mode, config.storage_mode);
        assert_eq!(factory.config().cache_mode, config.cache_mode);
    }

    #[rstest]
    #[tokio::test]
    async fn test_repository_factory_create_in_memory() {
        let config = RepositoryConfig::default();
        let factory = RepositoryFactory::new(config);
        let result = factory.create().await;
        assert!(result.is_ok());
    }

    #[rstest]
    #[tokio::test]
    async fn test_repository_factory_create_in_memory_repositories_are_functional() {
        let config = RepositoryConfig::default();
        let factory = RepositoryFactory::new(config);
        let repositories = factory.create().await.unwrap();

        // Verify we can call methods on the repositories
        let count = repositories.task_repository.count().run_async().await;
        assert!(count.is_ok());
        assert_eq!(count.unwrap(), 0);

        let project_count = repositories.project_repository.count().run_async().await;
        assert!(project_count.is_ok());
        assert_eq!(project_count.unwrap(), 0);
    }

    // -------------------------------------------------------------------------
    // Error Display Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_configuration_error_display() {
        let error = ConfigurationError::InvalidStorageMode("foo".to_string());
        assert!(error.to_string().contains("foo"));
        assert!(error.to_string().contains("storage mode"));

        let error = ConfigurationError::InvalidCacheMode("bar".to_string());
        assert!(error.to_string().contains("bar"));
        assert!(error.to_string().contains("cache mode"));

        let error = ConfigurationError::MissingDatabaseUrl;
        assert!(error.to_string().contains("DATABASE_URL"));

        let error = ConfigurationError::MissingRedisUrl;
        assert!(error.to_string().contains("REDIS_URL"));
    }

    #[rstest]
    fn test_factory_error_display() {
        let error = FactoryError::Configuration(ConfigurationError::MissingDatabaseUrl);
        assert!(error.to_string().contains("Configuration"));

        let error = FactoryError::DatabaseConnection("connection refused".to_string());
        assert!(error.to_string().contains("connection refused"));

        let error = FactoryError::RedisConnection("timeout".to_string());
        assert!(error.to_string().contains("timeout"));
    }

    // -------------------------------------------------------------------------
    // Repositories Debug Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_repositories_debug() {
        let config = RepositoryConfig::default();
        let factory = RepositoryFactory::new(config);
        let repositories = factory.create().await.unwrap();

        let debug_string = format!("{repositories:?}");
        assert!(debug_string.contains("Repositories"));
        assert!(debug_string.contains("task_repository"));
        assert!(debug_string.contains("project_repository"));
        assert!(debug_string.contains("event_store"));
    }

    // -------------------------------------------------------------------------
    // Integration Tests (require external services)
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires PostgreSQL instance"]
    async fn test_repository_factory_create_postgres() {
        let config = RepositoryConfig::builder()
            .storage_mode(StorageMode::Postgres)
            .database_url("postgres://localhost/test")
            .build()
            .unwrap();

        let factory = RepositoryFactory::new(config);
        let result = factory.create().await;
        assert!(result.is_ok());
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires Redis instance"]
    async fn test_repository_factory_create_redis() {
        let config = RepositoryConfig::builder()
            .cache_mode(CacheMode::Redis)
            .redis_url("redis://localhost:6379")
            .build()
            .unwrap();

        let factory = RepositoryFactory::new(config);
        let result = factory.create().await;
        assert!(result.is_ok());
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires PostgreSQL and Redis instances"]
    async fn test_repository_factory_create_full_production() {
        let config = RepositoryConfig::builder()
            .storage_mode(StorageMode::Postgres)
            .database_url("postgres://localhost/test")
            .cache_mode(CacheMode::Redis)
            .redis_url("redis://localhost:6379")
            .build()
            .unwrap();

        let factory = RepositoryFactory::new(config);
        let result = factory.create().await;
        assert!(result.is_ok());
    }
}
