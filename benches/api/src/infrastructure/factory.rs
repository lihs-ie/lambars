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
//! let task = repositories.task_repository.find_by_id(&task_id).await?;
//! ```

use std::env;
use std::str::FromStr;
use std::sync::Arc;

use deadpool_redis::{Config as RedisConfig, Pool as RedisPool, Runtime};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use thiserror::Error;
use tracing::warn;

use super::{
    CacheConfig, CachedProjectRepository, CachedTaskRepository, EventStore, InMemoryEventStore,
    InMemoryProjectRepository, InMemoryTaskRepository, PostgresEventStore,
    PostgresProjectRepository, PostgresTaskRepository, ProjectRepository, RedisProjectRepository,
    RedisTaskRepository, TaskRepository,
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
///
/// # Cache Configuration (CACHE-REQ-030, CACHE-REQ-031)
///
/// The `cache_config` field controls cache behavior when `cache_mode` is `Redis`.
/// It includes strategy, TTL, and enable/disable settings that can be configured
/// via environment variables or programmatically.
///
/// # Pool Size Configuration (ENV-REQ-021)
///
/// The `database_pool_size` and `redis_pool_size` fields control connection pool sizes.
/// When `None`, each library's default value is used.
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
    /// Cache configuration for controlling cache behavior.
    /// This is used when `cache_mode` is `Redis` to configure
    /// `CachedTaskRepository` and `CachedProjectRepository`.
    pub cache_config: CacheConfig,
    /// `PostgreSQL` connection pool size.
    /// When `None`, sqlx's default value is used.
    pub database_pool_size: Option<u32>,
    /// Redis connection pool size.
    /// When `None`, deadpool-redis's default value is used.
    pub redis_pool_size: Option<u32>,
}

/// Parses a pool size value from a string.
///
/// This is a pure function that handles the parsing logic without side effects.
/// It is separated from environment variable reading for testability.
///
/// # Returns
///
/// - `Some(n)` if the value is a valid positive integer (`n > 0`)
/// - `None` if the value is empty, whitespace-only, zero, or unparseable
///
/// # Examples
///
/// ```ignore
/// assert_eq!(parse_pool_size("10"), Some(10));
/// assert_eq!(parse_pool_size("0"), None);
/// assert_eq!(parse_pool_size(""), None);
/// assert_eq!(parse_pool_size("  "), None);
/// assert_eq!(parse_pool_size("abc"), None);
/// ```
fn parse_pool_size(value: &str) -> Option<u32> {
    value.trim().parse::<u32>().ok().filter(|&n| n > 0)
}

/// Parses an optional u32 value from an environment variable.
///
/// Returns `None` if the variable is unset, empty, whitespace-only, zero, or unparseable.
/// Logs a warning if the value is present but invalid (non-UTF-8, zero, or unparseable).
///
/// # Arguments
///
/// * `variable_name` - The name of the environment variable to read
///
/// # I/O Notice
///
/// This function reads from environment variables and logs warnings.
/// It should only be called during application initialization.
fn parse_optional_u32(variable_name: &str) -> Option<u32> {
    match env::var(variable_name) {
        Ok(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return None;
            }
            parse_pool_size(trimmed).or_else(|| {
                warn!(
                    "Invalid {} value '{}', using default (library default will be applied)",
                    variable_name, value
                );
                None
            })
        }
        Err(env::VarError::NotPresent) => None,
        Err(env::VarError::NotUnicode(_)) => {
            warn!(
                "{} contains non-UTF-8 value, using default (library default will be applied)",
                variable_name
            );
            None
        }
    }
}

impl RepositoryConfig {
    /// Creates a new configuration builder.
    pub fn builder() -> RepositoryConfigBuilder {
        RepositoryConfigBuilder::default()
    }

    /// Creates a configuration from environment variables.
    ///
    /// # I/O Notice
    ///
    /// This function reads environment variables and should be called **only once
    /// at application startup**. The resulting `RepositoryConfig` is immutable and
    /// should be shared across the application. This design isolates the side effect
    /// (environment variable reading) to the application's initialization phase.
    ///
    /// # Environment Variables
    ///
    /// - `STORAGE_MODE`: `in_memory` (default) | `postgres`
    /// - `CACHE_MODE`: `in_memory` (default) | `redis`
    /// - `DATABASE_URL`: `PostgreSQL` connection URL
    /// - `REDIS_URL`: Redis connection URL
    /// - `CACHE_STRATEGY`: Cache strategy (`read-through`, `write-through`, `write-behind`)
    /// - `CACHE_TTL_SECS`: TTL in seconds (default: 60)
    /// - `CACHE_ENABLED`: Whether caching is enabled (`true`, `false`, `1`, `0`)
    /// - `DATABASE_POOL_SIZE`: `PostgreSQL` connection pool size (optional)
    /// - `REDIS_POOL_SIZE`: Redis connection pool size (optional)
    ///
    /// # Errors
    ///
    /// Returns `ConfigurationError` if:
    /// - `STORAGE_MODE` or `CACHE_MODE` contains an invalid value
    /// - `DATABASE_URL` is missing when `STORAGE_MODE=postgres`
    /// - `REDIS_URL` is missing when `CACHE_MODE=redis`
    ///
    /// # Cache Configuration (CACHE-REQ-031)
    ///
    /// The cache configuration is loaded from environment variables via
    /// `CacheConfig::from_env()`. This allows scenarios to control cache
    /// behavior through environment variables.
    ///
    /// # Pool Size Configuration (ENV-REQ-021)
    ///
    /// Pool sizes are loaded from environment variables. When not set or empty,
    /// `None` is used which causes each library to use its default value.
    pub fn from_env() -> Result<Self, ConfigurationError> {
        let storage_mode = match env::var("STORAGE_MODE") {
            Ok(value) => value.parse()?,
            Err(env::VarError::NotPresent) => StorageMode::default(),
            Err(env::VarError::NotUnicode(_)) => {
                return Err(ConfigurationError::InvalidStorageMode(
                    "<non-UTF-8 value>".to_string(),
                ));
            }
        };

        let cache_mode = match env::var("CACHE_MODE") {
            Ok(value) => value.parse()?,
            Err(env::VarError::NotPresent) => CacheMode::default(),
            Err(env::VarError::NotUnicode(_)) => {
                return Err(ConfigurationError::InvalidCacheMode(
                    "<non-UTF-8 value>".to_string(),
                ));
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

        // Load cache configuration from environment variables (CACHE-REQ-031)
        let cache_config = CacheConfig::from_env();

        // Load pool size configuration from environment variables (ENV-REQ-021)
        let database_pool_size = parse_optional_u32("DATABASE_POOL_SIZE");
        let redis_pool_size = parse_optional_u32("REDIS_POOL_SIZE");

        let config = Self {
            storage_mode,
            cache_mode,
            database_url,
            redis_url,
            cache_config,
            database_pool_size,
            redis_pool_size,
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
///     .database_pool_size(10)
///     .cache_mode(CacheMode::Redis)
///     .redis_url("redis://localhost:6379")
///     .redis_pool_size(5)
///     .cache_config(CacheConfig::default())
///     .build()?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct RepositoryConfigBuilder {
    storage_mode: StorageMode,
    cache_mode: CacheMode,
    database_url: Option<String>,
    redis_url: Option<String>,
    cache_config: CacheConfig,
    database_pool_size: Option<u32>,
    redis_pool_size: Option<u32>,
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

    /// Sets the cache configuration.
    ///
    /// This configuration is used when `cache_mode` is `Redis` to control
    /// the behavior of `CachedTaskRepository` and `CachedProjectRepository`.
    ///
    /// # Arguments
    ///
    /// * `config` - Cache configuration including strategy, TTL, and enable/disable
    #[must_use]
    pub const fn cache_config(mut self, config: CacheConfig) -> Self {
        self.cache_config = config;
        self
    }

    /// Sets the `PostgreSQL` connection pool size.
    /// When not set, sqlx's default pool size is used.
    #[must_use]
    pub const fn database_pool_size(mut self, size: u32) -> Self {
        self.database_pool_size = Some(size);
        self
    }

    /// Sets the Redis connection pool size.
    /// When not set, deadpool-redis's default pool size is used.
    #[must_use]
    pub const fn redis_pool_size(mut self, size: u32) -> Self {
        self.redis_pool_size = Some(size);
        self
    }

    /// Builds the configuration.
    ///
    /// Pool sizes of `0` are treated as `None` (use library defaults),
    /// consistent with environment variable parsing behavior.
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
            cache_config: self.cache_config,
            database_pool_size: self.database_pool_size.filter(|&size| size > 0),
            redis_pool_size: self.redis_pool_size.filter(|&size| size > 0),
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
    /// # I/O Notice
    ///
    /// This function reads environment variables and should be called **only once
    /// at application startup**. See [`RepositoryConfig::from_env`] for details
    /// about the side effects involved.
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
    /// # I/O Notice
    ///
    /// This method performs I/O operations (database/Redis connections) and should
    /// be called **only once at application startup**. The returned `Repositories`
    /// struct is designed to be shared across the application via `Arc` wrappers.
    /// This design isolates the side effects to the initialization phase.
    ///
    /// # Repository Construction (CACHE-REQ-030)
    ///
    /// | `StorageMode` | `CacheMode` | Primary Storage | Cache Layer | `EventStore` |
    /// |-------------|-----------|--------------|--------------|------------|
    /// | `InMemory` | `InMemory` | `InMemory` | None | `InMemory` |
    /// | `Postgres` | `InMemory` | `Postgres` | None | `Postgres` |
    /// | `InMemory` | `Redis` | `InMemory` | `CachedTaskRepository(Redis)` | `InMemory` |
    /// | `Postgres` | `Redis` | `Postgres` | `CachedTaskRepository(Redis)` | `Postgres` |
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

            // In-memory for storage, Redis for cache (CACHE-REQ-030)
            // InMemory is the primary storage, Redis is the cache layer
            (StorageMode::InMemory, CacheMode::Redis) => {
                let redis_pool = self.create_redis_pool()?;

                // Create in-memory repositories once and reuse them
                let in_memory_task_repository = Arc::new(InMemoryTaskRepository::new());
                let in_memory_project_repository = Arc::new(InMemoryProjectRepository::new());
                let in_memory_event_store = Arc::new(InMemoryEventStore::new());

                // Wrap InMemory repositories with CachedRepository
                Ok(Repositories {
                    task_repository: Arc::new(CachedTaskRepository::new(
                        in_memory_task_repository,
                        redis_pool.clone(),
                        self.config.cache_config.clone(),
                    )),
                    project_repository: Arc::new(CachedProjectRepository::new(
                        in_memory_project_repository,
                        redis_pool,
                        self.config.cache_config.clone(),
                    )),
                    event_store: in_memory_event_store,
                })
            }

            // PostgreSQL for storage, Redis for cache (CACHE-REQ-030)
            // Postgres is the primary storage, Redis is the cache layer
            (StorageMode::Postgres, CacheMode::Redis) => {
                let pg_pool = self.create_postgres_pool().await?;
                let redis_pool = self.create_redis_pool()?;

                // Create Postgres repositories as primary storage
                let postgres_task_repository = PostgresTaskRepository::new(pg_pool.clone());
                let postgres_project_repository = PostgresProjectRepository::new(pg_pool.clone());
                let postgres_event_store = PostgresEventStore::new(pg_pool);

                // Wrap Postgres repositories with CachedRepository
                Ok(Repositories {
                    task_repository: Arc::new(CachedTaskRepository::new(
                        Arc::new(postgres_task_repository),
                        redis_pool.clone(),
                        self.config.cache_config.clone(),
                    )),
                    project_repository: Arc::new(CachedProjectRepository::new(
                        Arc::new(postgres_project_repository),
                        redis_pool,
                        self.config.cache_config.clone(),
                    )),
                    event_store: Arc::new(postgres_event_store),
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

        let mut pool_options = PgPoolOptions::new();
        if let Some(size) = self.config.database_pool_size {
            pool_options = pool_options.max_connections(size);
        }

        pool_options
            .connect(database_url)
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

    /// Creates a Redis connection pool for caching.
    fn create_redis_pool(&self) -> Result<RedisPool, FactoryError> {
        let redis_url = self
            .config
            .redis_url
            .as_ref()
            .ok_or(ConfigurationError::MissingRedisUrl)?;

        let mut redis_config = RedisConfig::from_url(redis_url);
        if let Some(size) = self.config.redis_pool_size {
            redis_config.pool = Some(deadpool_redis::PoolConfig::new(size as usize));
        }

        redis_config
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|error| FactoryError::RedisConnection(error.to_string()))
    }

    /// Creates Redis-backed repositories.
    ///
    /// # Deprecated
    ///
    /// This method creates Redis repositories directly without caching.
    /// For cache-enabled configurations, use `create_redis_pool()` with
    /// `CachedTaskRepository` and `CachedProjectRepository` instead.
    #[allow(dead_code)]
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
///
/// # Note
///
/// This struct is retained for potential future use or backward compatibility,
/// but is currently not used since `CacheMode::Redis` now uses `CachedRepository`
/// wrappers instead of direct Redis repositories.
#[allow(dead_code)]
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
    use crate::infrastructure::CacheStrategy;
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
            cache_config: CacheConfig::default(),
            database_pool_size: None,
            redis_pool_size: None,
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
            cache_config: CacheConfig::default(),
            database_pool_size: None,
            redis_pool_size: None,
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
            cache_config: CacheConfig::default(),
            database_pool_size: None,
            redis_pool_size: None,
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
            cache_config: CacheConfig::default(),
            database_pool_size: None,
            redis_pool_size: None,
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
            cache_config: CacheConfig::default(),
            database_pool_size: None,
            redis_pool_size: None,
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
            cache_config: CacheConfig::default(),
            database_pool_size: None,
            redis_pool_size: None,
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
        let count = repositories.task_repository.count().await;
        assert!(count.is_ok());
        assert_eq!(count.unwrap(), 0);

        let project_count = repositories.project_repository.count().await;
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

    // -------------------------------------------------------------------------
    // RepositoryConfig CacheConfig Tests (CACHE-REQ-030, CACHE-REQ-031)
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_repository_config_default_includes_cache_config() {
        let config = RepositoryConfig::default();
        assert_eq!(config.cache_config.strategy, CacheStrategy::ReadThrough);
        assert_eq!(config.cache_config.ttl_seconds, 60);
        assert!(config.cache_config.enabled);
    }

    #[rstest]
    fn test_repository_config_builder_with_cache_config() {
        let cache_config = CacheConfig::new(CacheStrategy::WriteThrough, 120, false, 200);
        let result = RepositoryConfig::builder()
            .cache_config(cache_config)
            .build();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.cache_config.strategy, CacheStrategy::WriteThrough);
        assert_eq!(config.cache_config.ttl_seconds, 120);
        assert!(!config.cache_config.enabled);
    }

    #[rstest]
    fn test_repository_config_builder_full_with_cache_config() {
        let cache_config = CacheConfig::new(CacheStrategy::WriteBehind, 90, true, 50);
        let result = RepositoryConfig::builder()
            .storage_mode(StorageMode::Postgres)
            .database_url("postgres://localhost/test")
            .cache_mode(CacheMode::Redis)
            .redis_url("redis://localhost:6379")
            .cache_config(cache_config)
            .build();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.storage_mode, StorageMode::Postgres);
        assert_eq!(config.cache_mode, CacheMode::Redis);
        assert_eq!(config.cache_config.strategy, CacheStrategy::WriteBehind);
        assert_eq!(config.cache_config.ttl_seconds, 90);
    }

    #[rstest]
    fn test_repository_config_cache_config_has_default_when_not_specified() {
        let result = RepositoryConfig::builder()
            .cache_mode(CacheMode::Redis)
            .redis_url("redis://localhost:6379")
            .build();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.cache_config.strategy, CacheStrategy::ReadThrough);
        assert_eq!(config.cache_config.ttl_seconds, 60);
        assert!(config.cache_config.enabled);
    }

    // -------------------------------------------------------------------------
    // RepositoryConfig::from_env() Integration Tests (CACHE-REQ-031)
    // -------------------------------------------------------------------------
    // Direct env var tests skipped: Rust 2024 requires unsafe for set_var,
    // and this project forbids unsafe code. Tested via builder pattern instead.

    #[rstest]
    fn test_repository_config_builder_propagates_cache_strategy_write_through() {
        let cache_config = CacheConfig::new(CacheStrategy::WriteThrough, 60, true, 100);
        let result = RepositoryConfig::builder()
            .cache_config(cache_config)
            .build();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.cache_config.strategy, CacheStrategy::WriteThrough);
    }

    #[rstest]
    fn test_repository_config_builder_propagates_cache_ttl() {
        let cache_config = CacheConfig::new(CacheStrategy::ReadThrough, 300, true, 100);
        let result = RepositoryConfig::builder()
            .cache_config(cache_config)
            .build();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.cache_config.ttl_seconds, 300);
    }

    #[rstest]
    fn test_repository_config_builder_propagates_cache_enabled_false() {
        let cache_config = CacheConfig::new(CacheStrategy::ReadThrough, 60, false, 100);
        let result = RepositoryConfig::builder()
            .cache_config(cache_config)
            .build();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(!config.cache_config.enabled);
    }

    #[rstest]
    fn test_repository_config_builder_propagates_all_cache_values() {
        let cache_config = CacheConfig::new(CacheStrategy::WriteBehind, 180, true, 50);
        let result = RepositoryConfig::builder()
            .cache_config(cache_config)
            .build();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.cache_config.strategy, CacheStrategy::WriteBehind);
        assert_eq!(config.cache_config.ttl_seconds, 180);
        assert!(config.cache_config.enabled);
        assert_eq!(config.cache_config.write_behind_buffer_size, 50);
    }

    // -------------------------------------------------------------------------
    // RepositoryFactory CachedRepository Tests (CACHE-REQ-030)
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires Redis instance"]
    async fn test_repository_factory_creates_cached_repository_for_inmemory_redis() {
        // (InMemory, Redis) should use CachedTaskRepository with InMemory as primary
        let cache_config = CacheConfig::new(CacheStrategy::ReadThrough, 60, true, 100);
        let config = RepositoryConfig::builder()
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::Redis)
            .redis_url("redis://localhost:6379")
            .cache_config(cache_config)
            .build()
            .unwrap();

        let factory = RepositoryFactory::new(config);
        let result = factory.create().await;
        assert!(result.is_ok());

        // Verify the repositories are functional
        let repositories = result.unwrap();
        let count = repositories.task_repository.count().await;
        assert!(count.is_ok());
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires PostgreSQL and Redis instances"]
    async fn test_repository_factory_creates_cached_repository_for_postgres_redis() {
        // (Postgres, Redis) should use CachedTaskRepository with Postgres as primary
        let cache_config = CacheConfig::new(CacheStrategy::WriteThrough, 120, true, 100);
        let config = RepositoryConfig::builder()
            .storage_mode(StorageMode::Postgres)
            .database_url("postgres://localhost/test")
            .cache_mode(CacheMode::Redis)
            .redis_url("redis://localhost:6379")
            .cache_config(cache_config)
            .build()
            .unwrap();

        let factory = RepositoryFactory::new(config);
        let result = factory.create().await;
        assert!(result.is_ok());
    }

    // -------------------------------------------------------------------------
    // Pool Size Configuration Tests (ENV-REQ-021)
    // -------------------------------------------------------------------------
    // Direct env var tests skipped: Rust 2024 requires unsafe for set_var.
    // Pool size behavior tested via builder pattern instead.

    #[rstest]
    fn test_repository_config_default_pool_sizes_are_none() {
        let config = RepositoryConfig::default();
        assert!(config.database_pool_size.is_none());
        assert!(config.redis_pool_size.is_none());
    }

    #[rstest]
    fn test_repository_config_builder_without_pool_sizes() {
        let result = RepositoryConfig::builder().build();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(config.database_pool_size.is_none());
        assert!(config.redis_pool_size.is_none());
    }

    #[rstest]
    fn test_repository_config_builder_with_database_pool_size() {
        let result = RepositoryConfig::builder().database_pool_size(10).build();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.database_pool_size, Some(10));
        assert!(config.redis_pool_size.is_none());
    }

    #[rstest]
    fn test_repository_config_builder_with_redis_pool_size() {
        let result = RepositoryConfig::builder().redis_pool_size(5).build();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(config.database_pool_size.is_none());
        assert_eq!(config.redis_pool_size, Some(5));
    }

    #[rstest]
    fn test_repository_config_builder_with_both_pool_sizes() {
        let result = RepositoryConfig::builder()
            .database_pool_size(20)
            .redis_pool_size(15)
            .build();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.database_pool_size, Some(20));
        assert_eq!(config.redis_pool_size, Some(15));
    }

    #[rstest]
    fn test_repository_config_builder_full_with_pool_sizes() {
        let cache_config = CacheConfig::new(CacheStrategy::ReadThrough, 60, true, 100);
        let result = RepositoryConfig::builder()
            .storage_mode(StorageMode::Postgres)
            .database_url("postgres://localhost/test")
            .database_pool_size(25)
            .cache_mode(CacheMode::Redis)
            .redis_url("redis://localhost:6379")
            .redis_pool_size(10)
            .cache_config(cache_config)
            .build();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.storage_mode, StorageMode::Postgres);
        assert_eq!(config.cache_mode, CacheMode::Redis);
        assert_eq!(config.database_pool_size, Some(25));
        assert_eq!(config.redis_pool_size, Some(10));
    }

    #[rstest]
    fn test_repository_config_builder_pool_size_zero_is_filtered_to_none() {
        // Zero pool sizes are filtered to None in build(),
        // consistent with from_env() behavior.
        let result = RepositoryConfig::builder()
            .database_pool_size(0)
            .redis_pool_size(0)
            .build();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.database_pool_size, None);
        assert_eq!(config.redis_pool_size, None);
    }

    #[rstest]
    fn test_repository_config_builder_large_pool_sizes() {
        let result = RepositoryConfig::builder()
            .database_pool_size(1000)
            .redis_pool_size(500)
            .build();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.database_pool_size, Some(1000));
        assert_eq!(config.redis_pool_size, Some(500));
    }

    // -------------------------------------------------------------------------
    // parse_pool_size Pure Function Tests (ENV-REQ-021)
    // -------------------------------------------------------------------------

    #[rstest]
    #[case("10", Some(10))]
    #[case("1", Some(1))]
    #[case("100", Some(100))]
    #[case("999999", Some(999_999))]
    fn test_parse_pool_size_valid_positive_values(
        #[case] input: &str,
        #[case] expected: Option<u32>,
    ) {
        assert_eq!(super::parse_pool_size(input), expected);
    }

    #[rstest]
    #[case("0")]
    fn test_parse_pool_size_zero_returns_none(#[case] input: &str) {
        assert_eq!(super::parse_pool_size(input), None);
    }

    #[rstest]
    #[case("")]
    fn test_parse_pool_size_empty_string_returns_none(#[case] input: &str) {
        assert_eq!(super::parse_pool_size(input), None);
    }

    #[rstest]
    #[case(" ")]
    #[case("  ")]
    #[case("\t")]
    #[case("\n")]
    #[case("   \t\n   ")]
    fn test_parse_pool_size_whitespace_only_returns_none(#[case] input: &str) {
        assert_eq!(super::parse_pool_size(input), None);
    }

    #[rstest]
    #[case("abc")]
    #[case("12abc")]
    #[case("abc12")]
    #[case("-1")]
    #[case("-10")]
    #[case("1.5")]
    #[case("10.0")]
    #[case("0x10")]
    #[case("")]
    fn test_parse_pool_size_invalid_values_return_none(#[case] input: &str) {
        assert_eq!(super::parse_pool_size(input), None);
    }

    #[rstest]
    #[case(" 10 ", Some(10))]
    #[case("  5  ", Some(5))]
    #[case("\t20\t", Some(20))]
    #[case("\n15\n", Some(15))]
    fn test_parse_pool_size_trims_whitespace(#[case] input: &str, #[case] expected: Option<u32>) {
        assert_eq!(super::parse_pool_size(input), expected);
    }

    #[rstest]
    #[case(" 0 ")]
    fn test_parse_pool_size_trimmed_zero_returns_none(#[case] input: &str) {
        assert_eq!(super::parse_pool_size(input), None);
    }

    // -------------------------------------------------------------------------
    // parse_optional_u32 Behavior Tests (ENV-REQ-021)
    // -------------------------------------------------------------------------
    // Tested indirectly via builder pattern since parse_optional_u32 reads from env.

    #[rstest]
    fn test_repository_config_builder_preserves_none_when_not_set() {
        let result = RepositoryConfig::builder()
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .build();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(config.database_pool_size.is_none());
        assert!(config.redis_pool_size.is_none());
    }

    #[rstest]
    #[tokio::test]
    async fn test_repository_factory_creates_in_memory_without_pool_sizes() {
        let config = RepositoryConfig::builder()
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .build()
            .unwrap();

        assert!(config.database_pool_size.is_none());
        assert!(config.redis_pool_size.is_none());

        let factory = RepositoryFactory::new(config);
        let result = factory.create().await;
        assert!(result.is_ok());
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires PostgreSQL instance"]
    async fn test_repository_factory_creates_postgres_with_custom_pool_size() {
        let config = RepositoryConfig::builder()
            .storage_mode(StorageMode::Postgres)
            .database_url("postgres://localhost/test")
            .database_pool_size(5)
            .build()
            .unwrap();

        assert_eq!(config.database_pool_size, Some(5));

        let factory = RepositoryFactory::new(config);
        let result = factory.create().await;
        assert!(result.is_ok());
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires Redis instance"]
    async fn test_repository_factory_creates_redis_with_custom_pool_size() {
        let config = RepositoryConfig::builder()
            .cache_mode(CacheMode::Redis)
            .redis_url("redis://localhost:6379")
            .redis_pool_size(3)
            .build()
            .unwrap();

        assert_eq!(config.redis_pool_size, Some(3));

        let factory = RepositoryFactory::new(config);
        let result = factory.create().await;
        assert!(result.is_ok());
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires PostgreSQL and Redis instances"]
    async fn test_repository_factory_creates_full_production_with_custom_pool_sizes() {
        let config = RepositoryConfig::builder()
            .storage_mode(StorageMode::Postgres)
            .database_url("postgres://localhost/test")
            .database_pool_size(20)
            .cache_mode(CacheMode::Redis)
            .redis_url("redis://localhost:6379")
            .redis_pool_size(10)
            .build()
            .unwrap();

        assert_eq!(config.database_pool_size, Some(20));
        assert_eq!(config.redis_pool_size, Some(10));

        let factory = RepositoryFactory::new(config);
        let result = factory.create().await;
        assert!(result.is_ok());
    }
}
