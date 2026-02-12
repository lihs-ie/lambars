//! Cache repository implementations.
//!
//! This module provides cache layer implementations that wrap primary repositories
//! (Postgres/InMemory) with Redis caching for improved read performance.
//!
//! # Cache Strategies
//!
//! - **`ReadThrough`**: Cache reads (`find_by_id` only), write-through on writes
//! - **`WriteThrough`**: Same as `ReadThrough` (explicit naming)
//! - **`WriteBehind`**: Async write batching (future extension)
//!
//! # Key Design (CACHE-REQ-011)
//!
//! - Data key: `cache:task:{task_id}:v{version}` -> JSON
//! - Latest key: `cache:task:{task_id}:latest` -> version number
//!
//! # Cache Scope (CACHE-REQ-010)
//!
//! - **Cached**: `find_by_id` (single entity retrieval)
//! - **Bypassed**: `list`, `list_filtered`, `count`, `search` (variable results, difficult invalidation)

use std::str::FromStr;
use std::sync::Arc;

use deadpool_redis::Pool;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use lambars::effect::AsyncIO;

use crate::domain::{Priority, Project, ProjectId, Task, TaskId, TaskStatus};
use crate::infrastructure::{
    PaginatedResult, Pagination, ProjectRepository, RepositoryError, SearchScope, TaskRepository,
};

// =============================================================================
// Cache Configuration Types
// =============================================================================

/// Cache strategy for controlling cache behavior.
///
/// # Variants
///
/// - `ReadThrough`: Read from cache first, fall back to primary storage on miss
/// - `WriteThrough`: Write to both cache and primary storage synchronously
/// - `WriteBehind`: Write to cache immediately, async batch write to primary (future)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheStrategy {
    /// Read-through caching: cache reads, write-through on writes.
    #[default]
    ReadThrough,
    /// Write-through caching: same as `ReadThrough` with explicit naming.
    WriteThrough,
    /// Write-behind caching: async write batching (future extension).
    WriteBehind,
}

impl FromStr for CacheStrategy {
    type Err = String;

    /// Parses a cache strategy from a string.
    ///
    /// Supports both hyphen and underscore separators for compatibility:
    /// - `"read-through"` or `"read_through"` -> `ReadThrough`
    /// - `"write-through"` or `"write_through"` -> `WriteThrough`
    /// - `"write-behind"` or `"write_behind"` -> `WriteBehind`
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // Normalize: lowercase and replace underscores with hyphens
        match value.to_lowercase().replace('_', "-").as_str() {
            "read-through" => Ok(Self::ReadThrough),
            "write-through" => Ok(Self::WriteThrough),
            "write-behind" => Ok(Self::WriteBehind),
            _ => Err(format!("Unknown cache strategy: {value}")),
        }
    }
}

impl std::fmt::Display for CacheStrategy {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadThrough => write!(formatter, "read-through"),
            Self::WriteThrough => write!(formatter, "write-through"),
            Self::WriteBehind => write!(formatter, "write-behind"),
        }
    }
}

/// Configuration for cache behavior.
///
/// This struct is typically constructed from environment variables via `from_env()`.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// The caching strategy to use.
    pub strategy: CacheStrategy,
    /// Time-to-live for cached entries in seconds.
    pub ttl_seconds: u64,
    /// Whether caching is enabled.
    pub enabled: bool,
    /// Buffer size for write-behind batching (future extension).
    pub write_behind_buffer_size: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            strategy: CacheStrategy::ReadThrough,
            ttl_seconds: 60,
            enabled: true,
            write_behind_buffer_size: 100,
        }
    }
}

impl CacheConfig {
    /// Creates a new `CacheConfig` with the given parameters.
    ///
    /// # TTL Validation
    ///
    /// If `ttl_seconds` is 0, it will be set to 1 to avoid Redis SETEX errors.
    #[must_use]
    pub const fn new(
        strategy: CacheStrategy,
        ttl_seconds: u64,
        enabled: bool,
        write_behind_buffer_size: usize,
    ) -> Self {
        // TTL must be at least 1 second to avoid Redis SETEX errors
        let validated_ttl = if ttl_seconds == 0 { 1 } else { ttl_seconds };
        Self {
            strategy,
            ttl_seconds: validated_ttl,
            enabled,
            write_behind_buffer_size,
        }
    }

    /// Creates a `CacheConfig` from environment variables.
    ///
    /// # I/O Notice
    ///
    /// This function reads environment variables and should be called only once
    /// at application startup. The resulting `CacheConfig` is immutable and
    /// should be shared across the application.
    ///
    /// # Environment Variables
    ///
    /// - `CACHE_STRATEGY`: Cache strategy (`read-through`, `write-through`, `write-behind`)
    /// - `CACHE_TTL_SECS`: TTL in seconds (default: 60, minimum: 1)
    /// - `CACHE_ENABLED`: Whether caching is enabled (`true`, `false`, `1`, `0`)
    ///
    /// # Defaults
    ///
    /// - Strategy: `ReadThrough`
    /// - TTL: 60 seconds (minimum: 1)
    /// - Enabled: true
    /// - Write-behind buffer size: 100
    #[must_use]
    pub fn from_env() -> Self {
        let strategy = std::env::var("CACHE_STRATEGY")
            .ok()
            .and_then(|value| CacheStrategy::from_str(&value).ok())
            .unwrap_or_default();

        let raw_ttl = std::env::var("CACHE_TTL_SECS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(60);

        // TTL must be at least 1 second to avoid Redis SETEX errors
        let ttl_seconds = if raw_ttl == 0 { 1 } else { raw_ttl };

        let enabled = std::env::var("CACHE_ENABLED")
            .map(|value| value == "true" || value == "1")
            .unwrap_or(true);

        Self {
            strategy,
            ttl_seconds,
            enabled,
            write_behind_buffer_size: 100,
        }
    }

    /// Returns the effective TTL for Redis operations.
    ///
    /// This ensures TTL is at least 1 second to avoid Redis SETEX errors.
    #[must_use]
    pub const fn effective_ttl(&self) -> u64 {
        if self.ttl_seconds == 0 {
            1
        } else {
            self.ttl_seconds
        }
    }
}

// =============================================================================
// Cache Result Types
// =============================================================================

/// Status of a cache operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStatus {
    /// Value was found in cache.
    Hit,
    /// Value was not found in cache (fetched from primary storage).
    Miss,
    /// Cache was bypassed (disabled or non-cacheable operation).
    Bypass,
    /// Cache operation failed (Redis error, but data fetched from primary storage).
    ///
    /// This status indicates fail-open behavior: the cache layer encountered an error
    /// but the request was still served from primary storage.
    Error,
}

impl std::fmt::Display for CacheStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hit => write!(formatter, "HIT"),
            Self::Miss => write!(formatter, "MISS"),
            Self::Bypass => write!(formatter, "BYPASS"),
            Self::Error => write!(formatter, "ERROR"),
        }
    }
}

/// Result of a cache operation containing both the value and cache status.
#[derive(Debug, Clone)]
pub struct CacheResult<T> {
    /// The retrieved value.
    pub value: T,
    /// The status of the cache operation.
    pub cache_status: CacheStatus,
}

impl<T> CacheResult<T> {
    #[must_use]
    pub const fn new(value: T, cache_status: CacheStatus) -> Self {
        Self {
            value,
            cache_status,
        }
    }

    #[must_use]
    pub const fn hit(value: T) -> Self {
        Self::new(value, CacheStatus::Hit)
    }

    #[must_use]
    pub const fn miss(value: T) -> Self {
        Self::new(value, CacheStatus::Miss)
    }

    #[must_use]
    pub const fn bypass(value: T) -> Self {
        Self::new(value, CacheStatus::Bypass)
    }

    /// Creates a cache error result (fail-open: Redis failed but data fetched from primary).
    #[must_use]
    pub const fn error(value: T) -> Self {
        Self::new(value, CacheStatus::Error)
    }

    pub fn map<U, F>(self, function: F) -> CacheResult<U>
    where
        F: FnOnce(T) -> U,
    {
        CacheResult {
            value: function(self.value),
            cache_status: self.cache_status,
        }
    }
}

// =============================================================================
// Cache Key Generation
// =============================================================================

const TASK_CACHE_PREFIX: &str = "cache:task:";
const PROJECT_CACHE_PREFIX: &str = "cache:project:";

#[must_use]
pub fn task_data_key(task_id: &TaskId, version: u64) -> String {
    format!("{TASK_CACHE_PREFIX}{task_id}:v{version}")
}

#[must_use]
pub fn task_latest_key(task_id: &TaskId) -> String {
    format!("{TASK_CACHE_PREFIX}{task_id}:latest")
}

#[must_use]
pub fn project_data_key(project_id: &ProjectId, version: u64) -> String {
    format!("{PROJECT_CACHE_PREFIX}{project_id}:v{version}")
}

#[must_use]
pub fn project_latest_key(project_id: &ProjectId) -> String {
    format!("{PROJECT_CACHE_PREFIX}{project_id}:latest")
}

// =============================================================================
// Cached Task Repository
// =============================================================================

/// A caching wrapper around a `TaskRepository` implementation.
///
/// This repository implements read-through caching for `find_by_id` operations
/// and write-through for write operations (`save`, `save_bulk`, `delete`).
///
/// # Cache Scope
///
/// - **Cached**: `find_by_id` (single entity retrieval)
/// - **Bypassed**: `list`, `list_filtered`, `count`, `search` (variable results)
///
/// # Key Structure (CACHE-REQ-011)
///
/// - Data key: `cache:task:{task_id}:v{version}` -> JSON data
/// - Latest key: `cache:task:{task_id}:latest` -> version number pointer
#[derive(Clone)]
pub struct CachedTaskRepository<P: TaskRepository> {
    /// The primary storage repository.
    primary: Arc<P>,
    /// Redis connection pool for caching.
    pool: Pool,
    /// Cache configuration.
    config: CacheConfig,
}

impl<P: TaskRepository + 'static> CachedTaskRepository<P> {
    #[must_use]
    pub const fn new(primary: Arc<P>, pool: Pool, config: CacheConfig) -> Self {
        Self {
            primary,
            pool,
            config,
        }
    }

    #[must_use]
    pub const fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Finds a task by ID with cache status information.
    ///
    /// This method returns a `CacheResult` that includes both the value and
    /// the cache status (hit/miss/bypass/error).
    ///
    /// # Fail-Open Behavior
    ///
    /// On Redis errors, this method falls back to primary storage and returns
    /// `CacheStatus::Error` instead of returning an error. This indicates that
    /// the data was fetched from primary storage due to a Redis failure.
    ///
    /// When cache is disabled (via configuration), returns `CacheStatus::Bypass`
    /// to indicate that the cache layer was intentionally skipped.
    #[allow(clippy::too_many_lines)]
    pub fn find_by_id_with_status(
        &self,
        id: &TaskId,
    ) -> AsyncIO<Result<CacheResult<Option<Task>>, RepositoryError>> {
        let pool = self.pool.clone();
        let primary = self.primary.clone();
        let config = self.config.clone();
        let task_id = *id;

        AsyncIO::new(move || async move {
            // If cache is disabled, bypass cache entirely
            if !config.enabled {
                let result = primary.find_by_id(&task_id).await?;
                return Ok(CacheResult::bypass(result));
            }

            // Try to get from cache first (fail-open on Redis errors)
            let mut connection = match pool.get().await {
                Ok(connection) => connection,
                Err(error) => {
                    // Redis connection error: fail-open, fallback to primary storage
                    tracing::warn!(
                        task_id = %task_id,
                        error = %error,
                        "Redis connection failed, falling back to primary storage"
                    );
                    let result = primary.find_by_id(&task_id).await?;
                    return Ok(CacheResult::error(result));
                }
            };

            // 1. Get the latest version pointer (fail-open on Redis errors)
            let latest_key = task_latest_key(&task_id);
            let latest_version: Option<u64> = match connection.get(&latest_key).await {
                Ok(version) => version,
                Err(error) => {
                    tracing::warn!(
                        task_id = %task_id,
                        error = %error,
                        "Redis GET failed, falling back to primary storage"
                    );
                    let result = primary.find_by_id(&task_id).await?;
                    return Ok(CacheResult::error(result));
                }
            };

            if let Some(version) = latest_version {
                // 2. Get the data using the versioned key
                let data_key = task_data_key(&task_id, version);
                let cached_json: Option<String> = match connection.get(&data_key).await {
                    Ok(json) => json,
                    Err(error) => {
                        tracing::warn!(
                            task_id = %task_id,
                            error = %error,
                            "Redis GET failed, falling back to primary storage"
                        );
                        let result = primary.find_by_id(&task_id).await?;
                        return Ok(CacheResult::error(result));
                    }
                };

                if let Some(json) = cached_json {
                    // Cache hit: deserialize and return
                    match serde_json::from_str::<Task>(&json) {
                        Ok(task) => return Ok(CacheResult::hit(Some(task))),
                        Err(error) => {
                            // Fail-open: JSON corruption detected, invalidate cache and fetch from primary
                            tracing::warn!(
                                task_id = %task_id,
                                error = %error,
                                "Cache JSON deserialization failed, invalidating cache and fetching from primary"
                            );
                            // Best-effort cache invalidation
                            let script = redis::Script::new(CACHE_INVALIDATE_SCRIPT);
                            let data_key_prefix = format!("{TASK_CACHE_PREFIX}{task_id}:v");
                            let _ = script
                                .key(&latest_key)
                                .key(&data_key_prefix)
                                .invoke_async::<()>(&mut *connection)
                                .await;
                            // Fall through to fetch from primary storage (treated as cache miss)
                        }
                    }
                }
            }

            // Cache miss: fetch from primary storage
            let result = primary.find_by_id(&task_id).await?;

            // If found, populate the cache (best effort - log errors but don't fail)
            if let Some(ref task) = result {
                let json = match serde_json::to_string(task) {
                    Ok(json) => json,
                    Err(error) => {
                        tracing::warn!(
                            task_id = %task_id,
                            error = %error,
                            "Failed to serialize task for cache"
                        );
                        return Ok(CacheResult::miss(result));
                    }
                };

                // Use Lua script for atomic cache population
                let script = redis::Script::new(CACHE_SET_SCRIPT);
                let data_key = task_data_key(&task_id, task.version);
                let latest_key = task_latest_key(&task_id);

                if let Err(error) = script
                    .key(&data_key)
                    .key(&latest_key)
                    .arg(&json)
                    .arg(task.version)
                    .arg(config.effective_ttl())
                    .invoke_async::<()>(&mut *connection)
                    .await
                {
                    tracing::warn!(
                        task_id = %task_id,
                        error = %error,
                        "Failed to populate cache after primary read"
                    );
                }
            }

            Ok(CacheResult::miss(result))
        })
    }
}

/// Lua script for atomic cache set operation (read-through cache population).
///
/// Sets both the data key and the latest version pointer atomically.
/// Includes version comparison to prevent race conditions where concurrent
/// read-through operations could overwrite newer cached data with stale data.
///
/// # Version Comparison (CACHE-REQ-011 compliance)
///
/// This script compares the new version against the current version in the
/// `latest` key. If `new_version > current_version`, the cache is populated.
/// Otherwise, the operation is skipped to prevent stale data from being cached.
///
/// This is critical for concurrent scenarios where:
/// 1. Request A reads Task v1 from primary storage (cache miss)
/// 2. Request B updates Task to v2 and writes to cache
/// 3. Request A tries to populate cache with v1 (stale data)
/// 4. Without version comparison, v1 would overwrite v2 in the `latest` key
const CACHE_SET_SCRIPT: &str = r"
local data_key = KEYS[1]
local latest_key = KEYS[2]
local json_data = ARGV[1]
local version = tonumber(ARGV[2])
local ttl = tonumber(ARGV[3])

-- Get current version from latest key
local current_version = redis.call('GET', latest_key)
current_version = current_version and tonumber(current_version) or 0

-- Only update latest pointer if new version is greater
-- This prevents stale cache population from overwriting newer data
if version > current_version then
    redis.call('SET', latest_key, version)
end

-- Always set the data key (it's versioned, so safe to set)
redis.call('SETEX', data_key, ttl, json_data)

return 'OK'
";

/// Lua script for atomic cache update operation.
///
/// Updates cache with new version, deleting the old version key.
/// The old version is retrieved from the latest key to handle version jumps.
///
/// # Version Comparison (CACHE-REQ-011 compliance)
///
/// This script includes version comparison to prevent race conditions where
/// a stale update (lower version) arrives after a newer update has already
/// been applied. If `new_version <= current_version`, the update is skipped
/// to ensure the cache always reflects the latest data.
const CACHE_UPDATE_SCRIPT: &str = r"
local new_data_key = KEYS[1]
local latest_key = KEYS[2]
local key_prefix = KEYS[3]
local json_data = ARGV[1]
local new_version = tonumber(ARGV[2])
local ttl = tonumber(ARGV[3])

-- Get the current version from latest key (handles version jumps)
local current_version = redis.call('GET', latest_key)
if current_version then
    current_version = tonumber(current_version)
    -- Version comparison: skip update if new version is not greater
    -- This prevents race conditions where stale updates arrive late
    if new_version <= current_version then
        return 'SKIPPED'
    end
    local old_data_key = key_prefix .. current_version
    redis.call('DEL', old_data_key)
end

-- Set the new data key with TTL
redis.call('SETEX', new_data_key, ttl, json_data)

-- Update the latest version pointer
redis.call('SET', latest_key, new_version)

return 'OK'
";

/// Lua script for atomic cache invalidation operation.
///
/// Deletes both the versioned data key and the latest version pointer atomically.
///
/// # Key Arguments
///
/// - `KEYS[1]`: Latest key (e.g., `cache:task:{id}:latest`)
/// - `KEYS[2]`: Data key prefix (e.g., `cache:task:{id}:v`)
///
/// # Usage
///
/// This script is used in two scenarios:
/// 1. **Delete operation**: When an entity is deleted from primary storage
/// 2. **Cache disabled mode**: When `CACHE_ENABLED=false`, writes still invalidate
///    the cache to prevent stale data when cache is re-enabled
///
/// Both scenarios use the same invalidation logic, hence the unified script.
const CACHE_INVALIDATE_SCRIPT: &str = r"
local latest_key = KEYS[1]
local key_prefix = KEYS[2]

-- Get the current version to find the data key
local current_version = redis.call('GET', latest_key)

-- Delete the latest pointer
redis.call('DEL', latest_key)

-- Delete the versioned data key if version exists
if current_version then
    local data_key = key_prefix .. current_version
    redis.call('DEL', data_key)
end

return 'OK'
";

#[allow(clippy::significant_drop_tightening)]
impl<P: TaskRepository + 'static> TaskRepository for CachedTaskRepository<P> {
    #[allow(clippy::future_not_send)]
    fn find_by_id(&self, id: &TaskId) -> AsyncIO<Result<Option<Task>, RepositoryError>> {
        let pool = self.pool.clone();
        let primary = self.primary.clone();
        let config = self.config.clone();
        let task_id = *id;

        AsyncIO::new(move || async move {
            // If cache is disabled, bypass cache entirely
            if !config.enabled {
                return primary.find_by_id(&task_id).await;
            }

            // Try to get from cache first (fail-open on Redis errors)
            let mut connection = match pool.get().await {
                Ok(connection) => connection,
                Err(error) => {
                    // Redis connection error: fail-open, fallback to primary storage
                    tracing::warn!(
                        task_id = %task_id,
                        error = %error,
                        "Redis connection failed, falling back to primary storage"
                    );
                    return primary.find_by_id(&task_id).await;
                }
            };

            // 1. Get the latest version pointer (fail-open on Redis errors)
            let latest_key = task_latest_key(&task_id);
            let latest_version: Option<u64> = match connection.get(&latest_key).await {
                Ok(version) => version,
                Err(error) => {
                    tracing::warn!(
                        task_id = %task_id,
                        error = %error,
                        "Redis GET failed, falling back to primary storage"
                    );
                    return primary.find_by_id(&task_id).await;
                }
            };

            if let Some(version) = latest_version {
                // 2. Get the data using the versioned key
                let data_key = task_data_key(&task_id, version);
                let cached_json: Option<String> = match connection.get(&data_key).await {
                    Ok(json) => json,
                    Err(error) => {
                        tracing::warn!(
                            task_id = %task_id,
                            error = %error,
                            "Redis GET failed, falling back to primary storage"
                        );
                        return primary.find_by_id(&task_id).await;
                    }
                };

                if let Some(json) = cached_json {
                    // Cache hit: deserialize and return
                    match serde_json::from_str::<Task>(&json) {
                        Ok(task) => return Ok(Some(task)),
                        Err(error) => {
                            // Fail-open: JSON corruption detected, invalidate cache and fetch from primary
                            tracing::warn!(
                                task_id = %task_id,
                                error = %error,
                                "Cache JSON deserialization failed, invalidating cache and fetching from primary"
                            );
                            // Best-effort cache invalidation
                            let script = redis::Script::new(CACHE_INVALIDATE_SCRIPT);
                            let data_key_prefix = format!("{TASK_CACHE_PREFIX}{task_id}:v");
                            let _ = script
                                .key(&latest_key)
                                .key(&data_key_prefix)
                                .invoke_async::<()>(&mut *connection)
                                .await;
                            // Fall through to fetch from primary storage (treated as cache miss)
                        }
                    }
                }
            }

            // Cache miss: fetch from primary storage
            let result = primary.find_by_id(&task_id).await?;

            // If found, populate the cache (best effort - log errors but don't fail)
            if let Some(ref task) = result {
                let json = match serde_json::to_string(task) {
                    Ok(json) => json,
                    Err(error) => {
                        tracing::warn!(
                            task_id = %task_id,
                            error = %error,
                            "Failed to serialize task for cache"
                        );
                        return Ok(result);
                    }
                };

                // Use Lua script for atomic cache population
                let script = redis::Script::new(CACHE_SET_SCRIPT);
                let data_key = task_data_key(&task_id, task.version);
                let latest_key = task_latest_key(&task_id);

                if let Err(error) = script
                    .key(&data_key)
                    .key(&latest_key)
                    .arg(&json)
                    .arg(task.version)
                    .arg(config.effective_ttl())
                    .invoke_async::<()>(&mut *connection)
                    .await
                {
                    tracing::warn!(
                        task_id = %task_id,
                        error = %error,
                        "Failed to populate cache after primary read"
                    );
                }
            }

            Ok(result)
        })
    }

    #[allow(clippy::future_not_send)]
    fn save(&self, task: &Task) -> AsyncIO<Result<(), RepositoryError>> {
        let pool = self.pool.clone();
        let primary = self.primary.clone();
        let config = self.config.clone();
        let task = task.clone();

        AsyncIO::new(move || async move {
            // Handle based on cache strategy
            match config.strategy {
                CacheStrategy::WriteBehind => {
                    // Write-behind: async write batching is a future extension (CACHE-FUT-001)
                    // For Phase 1, fall back to write-through with a warning
                    tracing::warn!(
                        task_id = %task.task_id,
                        "WriteBehind strategy selected but not yet implemented; \
                         operating as WriteThrough (CACHE-FUT-001)"
                    );
                    // Primary storage first to ensure data consistency
                    primary.save(&task).await?;
                }
                CacheStrategy::ReadThrough | CacheStrategy::WriteThrough => {
                    // Write-through: save to primary storage first
                    primary.save(&task).await?;
                }
            }

            // Get Redis connection (best effort - don't fail if Redis is down)
            let mut connection = match pool.get().await {
                Ok(connection) => connection,
                Err(error) => {
                    // Redis connection error after primary success: log only (best effort)
                    tracing::warn!(
                        task_id = %task.task_id,
                        error = %error,
                        "Redis connection failed after primary save, cache may be stale"
                    );
                    return Ok(());
                }
            };

            // CACHE_ENABLED=false: invalidate only, don't write new cache
            if !config.enabled {
                let script = redis::Script::new(CACHE_INVALIDATE_SCRIPT);
                let latest_key = task_latest_key(&task.task_id);
                let key_prefix = format!("{TASK_CACHE_PREFIX}{}:v", task.task_id);

                if let Err(error) = script
                    .key(&latest_key)
                    .key(&key_prefix)
                    .invoke_async::<()>(&mut *connection)
                    .await
                {
                    tracing::warn!(
                        task_id = %task.task_id,
                        error = %error,
                        "Failed to invalidate cache (cache disabled mode)"
                    );
                }
                return Ok(());
            }

            // Cache enabled: update the cache
            let json = match serde_json::to_string(&task) {
                Ok(json) => json,
                Err(error) => {
                    tracing::warn!(
                        task_id = %task.task_id,
                        error = %error,
                        "Failed to serialize task for cache update"
                    );
                    return Ok(());
                }
            };

            // Use key prefix for Lua script to find old version
            let key_prefix = format!("{TASK_CACHE_PREFIX}{}:v", task.task_id);

            let script = redis::Script::new(CACHE_UPDATE_SCRIPT);
            let new_data_key = task_data_key(&task.task_id, task.version);
            let latest_key = task_latest_key(&task.task_id);

            // Best effort cache update - log errors but don't fail
            if let Err(error) = script
                .key(&new_data_key)
                .key(&latest_key)
                .key(&key_prefix)
                .arg(&json)
                .arg(task.version)
                .arg(config.effective_ttl())
                .invoke_async::<()>(&mut *connection)
                .await
            {
                tracing::warn!(
                    task_id = %task.task_id,
                    error = %error,
                    "Failed to update cache after primary save"
                );
            }

            Ok(())
        })
    }

    #[allow(clippy::future_not_send)]
    fn save_bulk(&self, tasks: &[Task]) -> AsyncIO<Vec<Result<(), RepositoryError>>> {
        let pool = self.pool.clone();
        let primary = self.primary.clone();
        let config = self.config.clone();
        let tasks: Vec<Task> = tasks.to_vec();

        AsyncIO::new(move || async move {
            if tasks.is_empty() {
                return Vec::new();
            }

            // Handle based on cache strategy (same as save() for consistency)
            match config.strategy {
                CacheStrategy::WriteBehind => {
                    // Write-behind: async write batching is a future extension (CACHE-FUT-001)
                    // For Phase 1, fall back to write-through with a warning
                    tracing::warn!(
                        task_count = tasks.len(),
                        "WriteBehind strategy selected but not yet implemented; \
                         operating as WriteThrough (CACHE-FUT-001)"
                    );
                }
                CacheStrategy::ReadThrough | CacheStrategy::WriteThrough => {
                    // Write-through: proceed with synchronous writes
                }
            }

            // Save all to primary storage first (write-through behavior)
            let primary_results = primary.save_bulk(&tasks).await;

            // Get Redis connection (best effort - don't fail if Redis is down)
            let mut connection = match pool.get().await {
                Ok(connection) => connection,
                Err(error) => {
                    // Redis connection error after primary success: log only (best effort)
                    // Return primary results as-is since primary storage is the source of truth
                    tracing::warn!(
                        error = %error,
                        task_count = tasks.len(),
                        "Redis connection failed after primary save_bulk, cache may be stale"
                    );
                    return primary_results;
                }
            };

            let mut results = Vec::with_capacity(tasks.len());

            for (task, primary_result) in tasks.iter().zip(primary_results.into_iter()) {
                if let Err(error) = primary_result {
                    results.push(Err(error));
                    continue;
                }

                // CACHE_ENABLED=false: invalidate only, don't write new cache
                if !config.enabled {
                    let script = redis::Script::new(CACHE_INVALIDATE_SCRIPT);
                    let latest_key = task_latest_key(&task.task_id);
                    let key_prefix = format!("{TASK_CACHE_PREFIX}{}:v", task.task_id);

                    if let Err(error) = script
                        .key(&latest_key)
                        .key(&key_prefix)
                        .invoke_async::<()>(&mut *connection)
                        .await
                    {
                        tracing::warn!(
                            task_id = %task.task_id,
                            error = %error,
                            "Failed to invalidate cache (cache disabled mode)"
                        );
                    }
                    results.push(Ok(()));
                    continue;
                }

                // Cache enabled: update the cache
                let json = match serde_json::to_string(task) {
                    Ok(json) => json,
                    Err(error) => {
                        // Serialization error: log and continue (primary succeeded)
                        tracing::warn!(
                            task_id = %task.task_id,
                            error = %error,
                            "Failed to serialize task for cache update"
                        );
                        results.push(Ok(()));
                        continue;
                    }
                };

                // Use key prefix for Lua script to find old version
                let key_prefix = format!("{TASK_CACHE_PREFIX}{}:v", task.task_id);

                let script = redis::Script::new(CACHE_UPDATE_SCRIPT);
                let new_data_key = task_data_key(&task.task_id, task.version);
                let latest_key = task_latest_key(&task.task_id);

                // Best effort cache update - log errors but don't fail
                if let Err(error) = script
                    .key(&new_data_key)
                    .key(&latest_key)
                    .key(&key_prefix)
                    .arg(&json)
                    .arg(task.version)
                    .arg(config.effective_ttl())
                    .invoke_async::<()>(&mut *connection)
                    .await
                {
                    tracing::warn!(
                        task_id = %task.task_id,
                        error = %error,
                        "Failed to update cache after primary save"
                    );
                }

                results.push(Ok(()));
            }

            results
        })
    }

    #[allow(clippy::future_not_send)]
    fn delete(&self, id: &TaskId) -> AsyncIO<Result<bool, RepositoryError>> {
        let pool = self.pool.clone();
        let primary = self.primary.clone();
        let task_id = *id;

        AsyncIO::new(move || async move {
            // First, delete from primary storage
            let deleted = primary.delete(&task_id).await?;

            // Get Redis connection (best effort - don't fail if Redis is down)
            let mut connection = match pool.get().await {
                Ok(connection) => connection,
                Err(error) => {
                    // Redis connection error after primary success: log only (best effort)
                    tracing::warn!(
                        task_id = %task_id,
                        error = %error,
                        "Redis connection failed after primary delete, cache may be stale"
                    );
                    return Ok(deleted);
                }
            };

            // Invalidate the cache (best effort)
            let script = redis::Script::new(CACHE_INVALIDATE_SCRIPT);
            let latest_key = task_latest_key(&task_id);
            let data_key_prefix = format!("{TASK_CACHE_PREFIX}{task_id}:v");

            if let Err(error) = script
                .key(&latest_key)
                .key(&data_key_prefix)
                .invoke_async::<()>(&mut *connection)
                .await
            {
                tracing::warn!(
                    task_id = %task_id,
                    error = %error,
                    "Failed to invalidate cache after primary delete"
                );
            }

            Ok(deleted)
        })
    }

    // List/search/count operations bypass cache (variable results, difficult invalidation)

    fn list(
        &self,
        pagination: Pagination,
    ) -> AsyncIO<Result<PaginatedResult<Task>, RepositoryError>> {
        self.primary.list(pagination)
    }

    fn list_filtered(
        &self,
        status: Option<TaskStatus>,
        priority: Option<Priority>,
        pagination: Pagination,
    ) -> AsyncIO<Result<PaginatedResult<Task>, RepositoryError>> {
        self.primary.list_filtered(status, priority, pagination)
    }

    fn search(
        &self,
        query: &str,
        scope: SearchScope,
        limit: u32,
        offset: u32,
    ) -> AsyncIO<Result<Vec<Task>, RepositoryError>> {
        self.primary.search(query, scope, limit, offset)
    }

    fn count(&self) -> AsyncIO<Result<u64, RepositoryError>> {
        self.primary.count()
    }
}

// =============================================================================
// Cached Project Repository
// =============================================================================

/// A caching wrapper around a `ProjectRepository` implementation.
///
/// This repository implements read-through caching for `find_by_id` operations
/// and write-through for write operations (`save`, `delete`).
///
/// # Cache Scope
///
/// - **Cached**: `find_by_id` (single entity retrieval)
/// - **Bypassed**: `list`, `count` (variable results)
///
/// # Key Structure (CACHE-REQ-011)
///
/// - Data key: `cache:project:{project_id}:v{version}` -> JSON data
/// - Latest key: `cache:project:{project_id}:latest` -> version number pointer
#[derive(Clone)]
pub struct CachedProjectRepository<P: ProjectRepository> {
    /// The primary storage repository.
    primary: Arc<P>,
    /// Redis connection pool for caching.
    pool: Pool,
    /// Cache configuration.
    config: CacheConfig,
}

impl<P: ProjectRepository + 'static> CachedProjectRepository<P> {
    #[must_use]
    pub const fn new(primary: Arc<P>, pool: Pool, config: CacheConfig) -> Self {
        Self {
            primary,
            pool,
            config,
        }
    }

    #[must_use]
    pub const fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Finds a project by ID with cache status information.
    ///
    /// This method returns a `CacheResult` that includes both the value and
    /// the cache status (hit/miss/bypass/error).
    ///
    /// # Fail-Open Behavior
    ///
    /// On Redis errors, this method falls back to primary storage and returns
    /// `CacheStatus::Error` instead of returning an error. This indicates that
    /// the data was fetched from primary storage due to a Redis failure.
    ///
    /// When cache is disabled (via configuration), returns `CacheStatus::Bypass`
    /// to indicate that the cache layer was intentionally skipped.
    #[allow(clippy::too_many_lines)]
    pub fn find_by_id_with_status(
        &self,
        id: &ProjectId,
    ) -> AsyncIO<Result<CacheResult<Option<Project>>, RepositoryError>> {
        let pool = self.pool.clone();
        let primary = self.primary.clone();
        let config = self.config.clone();
        let project_id = id.clone();

        AsyncIO::new(move || async move {
            // If cache is disabled, bypass cache entirely
            if !config.enabled {
                let result = primary.find_by_id(&project_id).await?;
                return Ok(CacheResult::bypass(result));
            }

            // Try to get from cache first (fail-open on Redis errors)
            let mut connection = match pool.get().await {
                Ok(connection) => connection,
                Err(error) => {
                    // Redis connection error: fail-open, fallback to primary storage
                    tracing::warn!(
                        project_id = %project_id,
                        error = %error,
                        "Redis connection failed, falling back to primary storage"
                    );
                    let result = primary.find_by_id(&project_id).await?;
                    return Ok(CacheResult::error(result));
                }
            };

            // 1. Get the latest version pointer (fail-open on Redis errors)
            let latest_key = project_latest_key(&project_id);
            let latest_version: Option<u64> = match connection.get(&latest_key).await {
                Ok(version) => version,
                Err(error) => {
                    tracing::warn!(
                        project_id = %project_id,
                        error = %error,
                        "Redis GET failed, falling back to primary storage"
                    );
                    let result = primary.find_by_id(&project_id).await?;
                    return Ok(CacheResult::error(result));
                }
            };

            if let Some(version) = latest_version {
                // 2. Get the data using the versioned key
                let data_key = project_data_key(&project_id, version);
                let cached_json: Option<String> = match connection.get(&data_key).await {
                    Ok(json) => json,
                    Err(error) => {
                        tracing::warn!(
                            project_id = %project_id,
                            error = %error,
                            "Redis GET failed, falling back to primary storage"
                        );
                        let result = primary.find_by_id(&project_id).await?;
                        return Ok(CacheResult::error(result));
                    }
                };

                if let Some(json) = cached_json {
                    // Cache hit: deserialize and return
                    match serde_json::from_str::<Project>(&json) {
                        Ok(project) => return Ok(CacheResult::hit(Some(project))),
                        Err(error) => {
                            // Fail-open: JSON corruption detected, invalidate cache and fetch from primary
                            tracing::warn!(
                                project_id = %project_id,
                                error = %error,
                                "Cache JSON deserialization failed, invalidating cache and fetching from primary"
                            );
                            // Best-effort cache invalidation
                            let script = redis::Script::new(CACHE_INVALIDATE_SCRIPT);
                            let data_key_prefix = format!("{PROJECT_CACHE_PREFIX}{project_id}:v");
                            let _ = script
                                .key(&latest_key)
                                .key(&data_key_prefix)
                                .invoke_async::<()>(&mut *connection)
                                .await;
                            // Fall through to fetch from primary storage (treated as cache miss)
                        }
                    }
                }
            }

            // Cache miss: fetch from primary storage
            let result = primary.find_by_id(&project_id).await?;

            // If found, populate the cache (best effort - log errors but don't fail)
            if let Some(ref project) = result {
                let json = match serde_json::to_string(project) {
                    Ok(json) => json,
                    Err(error) => {
                        tracing::warn!(
                            project_id = %project_id,
                            error = %error,
                            "Failed to serialize project for cache"
                        );
                        return Ok(CacheResult::miss(result));
                    }
                };

                // Use Lua script for atomic cache population
                let script = redis::Script::new(CACHE_SET_SCRIPT);
                let data_key = project_data_key(&project_id, project.version);
                let latest_key = project_latest_key(&project_id);

                if let Err(error) = script
                    .key(&data_key)
                    .key(&latest_key)
                    .arg(&json)
                    .arg(project.version)
                    .arg(config.effective_ttl())
                    .invoke_async::<()>(&mut *connection)
                    .await
                {
                    tracing::warn!(
                        project_id = %project_id,
                        error = %error,
                        "Failed to populate cache after primary read"
                    );
                }
            }

            Ok(CacheResult::miss(result))
        })
    }
}

#[allow(clippy::significant_drop_tightening)]
impl<P: ProjectRepository + 'static> ProjectRepository for CachedProjectRepository<P> {
    #[allow(clippy::future_not_send)]
    fn find_by_id(&self, id: &ProjectId) -> AsyncIO<Result<Option<Project>, RepositoryError>> {
        let pool = self.pool.clone();
        let primary = self.primary.clone();
        let config = self.config.clone();
        let project_id = id.clone();

        AsyncIO::new(move || async move {
            // If cache is disabled, bypass cache entirely
            if !config.enabled {
                return primary.find_by_id(&project_id).await;
            }

            // Try to get from cache first (fail-open on Redis errors)
            let mut connection = match pool.get().await {
                Ok(connection) => connection,
                Err(error) => {
                    // Redis connection error: fail-open, fallback to primary storage
                    tracing::warn!(
                        project_id = %project_id,
                        error = %error,
                        "Redis connection failed, falling back to primary storage"
                    );
                    return primary.find_by_id(&project_id).await;
                }
            };

            // 1. Get the latest version pointer (fail-open on Redis errors)
            let latest_key = project_latest_key(&project_id);
            let latest_version: Option<u64> = match connection.get(&latest_key).await {
                Ok(version) => version,
                Err(error) => {
                    tracing::warn!(
                        project_id = %project_id,
                        error = %error,
                        "Redis GET failed, falling back to primary storage"
                    );
                    return primary.find_by_id(&project_id).await;
                }
            };

            if let Some(version) = latest_version {
                // 2. Get the data using the versioned key
                let data_key = project_data_key(&project_id, version);
                let cached_json: Option<String> = match connection.get(&data_key).await {
                    Ok(json) => json,
                    Err(error) => {
                        tracing::warn!(
                            project_id = %project_id,
                            error = %error,
                            "Redis GET failed, falling back to primary storage"
                        );
                        return primary.find_by_id(&project_id).await;
                    }
                };

                if let Some(json) = cached_json {
                    // Cache hit: deserialize and return
                    match serde_json::from_str::<Project>(&json) {
                        Ok(project) => return Ok(Some(project)),
                        Err(error) => {
                            // Fail-open: JSON corruption detected, invalidate cache and fetch from primary
                            tracing::warn!(
                                project_id = %project_id,
                                error = %error,
                                "Cache JSON deserialization failed, invalidating cache and fetching from primary"
                            );
                            // Best-effort cache invalidation
                            let script = redis::Script::new(CACHE_INVALIDATE_SCRIPT);
                            let data_key_prefix = format!("{PROJECT_CACHE_PREFIX}{project_id}:v");
                            let _ = script
                                .key(&latest_key)
                                .key(&data_key_prefix)
                                .invoke_async::<()>(&mut *connection)
                                .await;
                            // Fall through to fetch from primary storage (treated as cache miss)
                        }
                    }
                }
            }

            // Cache miss: fetch from primary storage
            let result = primary.find_by_id(&project_id).await?;

            // If found, populate the cache (best effort - log errors but don't fail)
            if let Some(ref project) = result {
                let json = match serde_json::to_string(project) {
                    Ok(json) => json,
                    Err(error) => {
                        tracing::warn!(
                            project_id = %project_id,
                            error = %error,
                            "Failed to serialize project for cache"
                        );
                        return Ok(result);
                    }
                };

                // Use Lua script for atomic cache population
                let script = redis::Script::new(CACHE_SET_SCRIPT);
                let data_key = project_data_key(&project_id, project.version);
                let latest_key = project_latest_key(&project_id);

                if let Err(error) = script
                    .key(&data_key)
                    .key(&latest_key)
                    .arg(&json)
                    .arg(project.version)
                    .arg(config.effective_ttl())
                    .invoke_async::<()>(&mut *connection)
                    .await
                {
                    tracing::warn!(
                        project_id = %project_id,
                        error = %error,
                        "Failed to populate cache after primary read"
                    );
                }
            }

            Ok(result)
        })
    }

    #[allow(clippy::future_not_send)]
    fn save(&self, project: &Project) -> AsyncIO<Result<(), RepositoryError>> {
        let pool = self.pool.clone();
        let primary = self.primary.clone();
        let config = self.config.clone();
        let project = project.clone();

        AsyncIO::new(move || async move {
            // Handle based on cache strategy
            match config.strategy {
                CacheStrategy::WriteBehind => {
                    // Write-behind: async write batching is a future extension (CACHE-FUT-001)
                    // For Phase 1, fall back to write-through with a warning
                    tracing::warn!(
                        project_id = %project.project_id,
                        "WriteBehind strategy selected but not yet implemented; \
                         operating as WriteThrough (CACHE-FUT-001)"
                    );
                    // Primary storage first to ensure data consistency
                    primary.save(&project).await?;
                }
                CacheStrategy::ReadThrough | CacheStrategy::WriteThrough => {
                    // Write-through: save to primary storage first
                    primary.save(&project).await?;
                }
            }

            // Get Redis connection (best effort - don't fail if Redis is down)
            let mut connection = match pool.get().await {
                Ok(connection) => connection,
                Err(error) => {
                    // Redis connection error after primary success: log only (best effort)
                    tracing::warn!(
                        project_id = %project.project_id,
                        error = %error,
                        "Redis connection failed after primary save, cache may be stale"
                    );
                    return Ok(());
                }
            };

            // CACHE_ENABLED=false: invalidate only, don't write new cache
            if !config.enabled {
                let script = redis::Script::new(CACHE_INVALIDATE_SCRIPT);
                let latest_key = project_latest_key(&project.project_id);
                let key_prefix = format!("{PROJECT_CACHE_PREFIX}{}:v", project.project_id);

                if let Err(error) = script
                    .key(&latest_key)
                    .key(&key_prefix)
                    .invoke_async::<()>(&mut *connection)
                    .await
                {
                    tracing::warn!(
                        project_id = %project.project_id,
                        error = %error,
                        "Failed to invalidate cache (cache disabled mode)"
                    );
                }
                return Ok(());
            }

            // Cache enabled: update the cache
            let json = match serde_json::to_string(&project) {
                Ok(json) => json,
                Err(error) => {
                    tracing::warn!(
                        project_id = %project.project_id,
                        error = %error,
                        "Failed to serialize project for cache update"
                    );
                    return Ok(());
                }
            };

            // Use key prefix for Lua script to find old version
            let key_prefix = format!("{PROJECT_CACHE_PREFIX}{}:v", project.project_id);

            let script = redis::Script::new(CACHE_UPDATE_SCRIPT);
            let new_data_key = project_data_key(&project.project_id, project.version);
            let latest_key = project_latest_key(&project.project_id);

            // Best effort cache update - log errors but don't fail
            if let Err(error) = script
                .key(&new_data_key)
                .key(&latest_key)
                .key(&key_prefix)
                .arg(&json)
                .arg(project.version)
                .arg(config.effective_ttl())
                .invoke_async::<()>(&mut *connection)
                .await
            {
                tracing::warn!(
                    project_id = %project.project_id,
                    error = %error,
                    "Failed to update cache after primary save"
                );
            }

            Ok(())
        })
    }

    #[allow(clippy::future_not_send)]
    fn delete(&self, id: &ProjectId) -> AsyncIO<Result<bool, RepositoryError>> {
        let pool = self.pool.clone();
        let primary = self.primary.clone();
        let project_id = id.clone();

        AsyncIO::new(move || async move {
            // First, delete from primary storage
            let deleted = primary.delete(&project_id).await?;

            // Get Redis connection (best effort - don't fail if Redis is down)
            let mut connection = match pool.get().await {
                Ok(connection) => connection,
                Err(error) => {
                    // Redis connection error after primary success: log only (best effort)
                    tracing::warn!(
                        project_id = %project_id,
                        error = %error,
                        "Redis connection failed after primary delete, cache may be stale"
                    );
                    return Ok(deleted);
                }
            };

            // Invalidate the cache (best effort)
            let script = redis::Script::new(CACHE_INVALIDATE_SCRIPT);
            let latest_key = project_latest_key(&project_id);
            let data_key_prefix = format!("{PROJECT_CACHE_PREFIX}{project_id}:v");

            if let Err(error) = script
                .key(&latest_key)
                .key(&data_key_prefix)
                .invoke_async::<()>(&mut *connection)
                .await
            {
                tracing::warn!(
                    project_id = %project_id,
                    error = %error,
                    "Failed to invalidate cache after primary delete"
                );
            }

            Ok(deleted)
        })
    }

    // List/count operations bypass cache (variable results, difficult invalidation)

    fn list(
        &self,
        pagination: Pagination,
    ) -> AsyncIO<Result<PaginatedResult<Project>, RepositoryError>> {
        self.primary.list(pagination)
    }

    fn count(&self) -> AsyncIO<Result<u64, RepositoryError>> {
        self.primary.count()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // CacheStrategy Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[case("read-through", CacheStrategy::ReadThrough)]
    #[case("read_through", CacheStrategy::ReadThrough)]
    #[case("READ-THROUGH", CacheStrategy::ReadThrough)]
    #[case("READ_THROUGH", CacheStrategy::ReadThrough)]
    #[case("write-through", CacheStrategy::WriteThrough)]
    #[case("write_through", CacheStrategy::WriteThrough)]
    #[case("WRITE-THROUGH", CacheStrategy::WriteThrough)]
    #[case("write-behind", CacheStrategy::WriteBehind)]
    #[case("write_behind", CacheStrategy::WriteBehind)]
    #[case("WRITE_BEHIND", CacheStrategy::WriteBehind)]
    fn test_cache_strategy_from_str_valid(#[case] input: &str, #[case] expected: CacheStrategy) {
        let result = CacheStrategy::from_str(input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }

    #[rstest]
    #[case("invalid")]
    #[case("cache")]
    #[case("")]
    fn test_cache_strategy_from_str_invalid(#[case] input: &str) {
        let result = CacheStrategy::from_str(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown cache strategy"));
    }

    #[rstest]
    fn test_cache_strategy_default() {
        let strategy = CacheStrategy::default();
        assert_eq!(strategy, CacheStrategy::ReadThrough);
    }

    #[rstest]
    fn test_cache_strategy_display() {
        assert_eq!(format!("{}", CacheStrategy::ReadThrough), "read-through");
        assert_eq!(format!("{}", CacheStrategy::WriteThrough), "write-through");
        assert_eq!(format!("{}", CacheStrategy::WriteBehind), "write-behind");
    }

    // -------------------------------------------------------------------------
    // CacheConfig Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.strategy, CacheStrategy::ReadThrough);
        assert_eq!(config.ttl_seconds, 60);
        assert!(config.enabled);
        assert_eq!(config.write_behind_buffer_size, 100);
    }

    #[rstest]
    fn test_cache_config_new() {
        let config = CacheConfig::new(CacheStrategy::WriteBehind, 120, false, 200);
        assert_eq!(config.strategy, CacheStrategy::WriteBehind);
        assert_eq!(config.ttl_seconds, 120);
        assert!(!config.enabled);
        assert_eq!(config.write_behind_buffer_size, 200);
    }

    #[rstest]
    fn test_cache_config_new_with_zero_ttl() {
        // TTL=0 should be normalized to 1 to avoid Redis SETEX errors
        let config = CacheConfig::new(CacheStrategy::ReadThrough, 0, true, 100);
        assert_eq!(config.ttl_seconds, 1);
        assert_eq!(config.effective_ttl(), 1);
    }

    #[rstest]
    fn test_cache_config_effective_ttl() {
        // Normal TTL
        let config = CacheConfig::new(CacheStrategy::ReadThrough, 60, true, 100);
        assert_eq!(config.effective_ttl(), 60);

        // TTL=1 (minimum)
        let config_min = CacheConfig::new(CacheStrategy::ReadThrough, 1, true, 100);
        assert_eq!(config_min.effective_ttl(), 1);
    }

    // Note: Environment variable tests are removed because Rust 2024 edition
    // requires unsafe blocks for env::set_var/remove_var, and parallel test
    // execution makes environment variable tests unreliable.
    // CacheConfig::from_env() is tested implicitly through integration tests.

    // -------------------------------------------------------------------------
    // CacheStatus Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_cache_status_display() {
        assert_eq!(format!("{}", CacheStatus::Hit), "HIT");
        assert_eq!(format!("{}", CacheStatus::Miss), "MISS");
        assert_eq!(format!("{}", CacheStatus::Bypass), "BYPASS");
        assert_eq!(format!("{}", CacheStatus::Error), "ERROR");
    }

    // -------------------------------------------------------------------------
    // CacheResult Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[case::hit(CacheStatus::Hit)]
    #[case::miss(CacheStatus::Miss)]
    #[case::bypass(CacheStatus::Bypass)]
    #[case::error(CacheStatus::Error)]
    fn test_cache_result_constructors(#[case] expected_status: CacheStatus) {
        let result = match expected_status {
            CacheStatus::Hit => CacheResult::hit(42),
            CacheStatus::Miss => CacheResult::miss(42),
            CacheStatus::Bypass => CacheResult::bypass(42),
            CacheStatus::Error => CacheResult::error(42),
        };
        assert_eq!(result.value, 42);
        assert_eq!(result.cache_status, expected_status);
    }

    #[rstest]
    fn test_cache_result_map() {
        let result = CacheResult::hit(10);
        let mapped = result.map(|value| value * 2);
        assert_eq!(mapped.value, 20);
        assert_eq!(mapped.cache_status, CacheStatus::Hit);
    }

    // -------------------------------------------------------------------------
    // Cache Key Generation Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_task_data_key() {
        let task_id = TaskId::from_uuid(uuid::Uuid::nil());
        let key = task_data_key(&task_id, 5);
        assert_eq!(key, "cache:task:00000000-0000-0000-0000-000000000000:v5");
    }

    #[rstest]
    fn test_task_latest_key() {
        let task_id = TaskId::from_uuid(uuid::Uuid::nil());
        let key = task_latest_key(&task_id);
        assert_eq!(
            key,
            "cache:task:00000000-0000-0000-0000-000000000000:latest"
        );
    }

    #[rstest]
    fn test_project_data_key() {
        let project_id = ProjectId::from_uuid(uuid::Uuid::nil());
        let key = project_data_key(&project_id, 3);
        assert_eq!(key, "cache:project:00000000-0000-0000-0000-000000000000:v3");
    }

    #[rstest]
    fn test_project_latest_key() {
        let project_id = ProjectId::from_uuid(uuid::Uuid::nil());
        let key = project_latest_key(&project_id);
        assert_eq!(
            key,
            "cache:project:00000000-0000-0000-0000-000000000000:latest"
        );
    }

    #[rstest]
    fn test_cache_keys_uniqueness() {
        let task_id = TaskId::generate();
        let key_v1 = task_data_key(&task_id, 1);
        let key_v2 = task_data_key(&task_id, 2);
        let key_latest = task_latest_key(&task_id);

        // All keys should be different
        assert_ne!(key_v1, key_v2);
        assert_ne!(key_v1, key_latest);
        assert_ne!(key_v2, key_latest);
    }

    // -------------------------------------------------------------------------
    // Lua Script Tests (Version Comparison Logic)
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_cache_update_script_contains_version_comparison() {
        // Verify that the CACHE_UPDATE_SCRIPT includes version comparison logic
        // to prevent race conditions where stale updates arrive late
        assert!(
            CACHE_UPDATE_SCRIPT.contains("new_version <= current_version"),
            "CACHE_UPDATE_SCRIPT should include version comparison to prevent stale updates"
        );
        assert!(
            CACHE_UPDATE_SCRIPT.contains("SKIPPED"),
            "CACHE_UPDATE_SCRIPT should return SKIPPED when version is not greater"
        );
    }

    #[rstest]
    fn test_cache_set_script_sets_both_keys() {
        // Verify that the CACHE_SET_SCRIPT sets both data key and latest pointer
        assert!(
            CACHE_SET_SCRIPT.contains("SETEX"),
            "CACHE_SET_SCRIPT should use SETEX for TTL"
        );
        assert!(
            CACHE_SET_SCRIPT.contains("SET"),
            "CACHE_SET_SCRIPT should SET the latest pointer"
        );
    }

    #[rstest]
    fn test_cache_invalidate_script_deletes_both_keys() {
        // Verify that the CACHE_INVALIDATE_SCRIPT deletes both keys
        assert!(
            CACHE_INVALIDATE_SCRIPT.contains("DEL"),
            "CACHE_INVALIDATE_SCRIPT should delete keys"
        );
    }

    // -------------------------------------------------------------------------
    // CacheConfig WriteBehind Warning Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_write_behind_is_not_yet_implemented() {
        // WriteBehind is a future extension (CACHE-FUT-001)
        // Verify that the code documents this behavior
        let config = CacheConfig::new(CacheStrategy::WriteBehind, 60, true, 100);
        assert_eq!(config.strategy, CacheStrategy::WriteBehind);
        // The actual warning is emitted at runtime in save() and save_bulk()
        // This test verifies the configuration is accepted
    }

    // -------------------------------------------------------------------------
    // CACHE_ENABLED=false Behavior Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_cache_config_disabled() {
        let config = CacheConfig::new(CacheStrategy::ReadThrough, 60, false, 100);
        assert!(!config.enabled);
        // When disabled:
        // - Reads bypass Redis (go directly to primary storage)
        // - Writes still invalidate Redis (to prevent stale data on re-enable)
    }

    // -------------------------------------------------------------------------
    // Version Comparison Edge Cases
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_version_comparison_prevents_rollback() {
        // Scenario: Update v3 arrives before v2 due to network delay
        // Expected: v2 should be skipped, v3 remains
        //
        // This is tested via the Lua script logic:
        // - If new_version (2) <= current_version (3), return SKIPPED
        //
        // The script comparison is:
        // `if new_version <= current_version then return 'SKIPPED' end`
        //
        // This ensures cache always reflects the latest version.
        assert!(CACHE_UPDATE_SCRIPT.contains("new_version <= current_version"));
    }

    #[rstest]
    fn test_version_comparison_allows_forward_update() {
        // Scenario: Update v4 arrives after v3
        // Expected: v4 should replace v3
        //
        // The script only skips when new_version <= current_version
        // So new_version (4) > current_version (3) proceeds with update
        assert!(CACHE_UPDATE_SCRIPT.contains("SETEX"));
        assert!(CACHE_UPDATE_SCRIPT.contains("SET"));
    }

    // -------------------------------------------------------------------------
    // CACHE_SET_SCRIPT Version Comparison Tests (Critical Fix)
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_cache_set_script_contains_version_comparison() {
        // CRITICAL: Verify that CACHE_SET_SCRIPT includes version comparison
        // to prevent stale cache population during concurrent operations.
        //
        // Scenario:
        // 1. Request A reads Task v1 from primary storage (cache miss)
        // 2. Request B updates Task to v2 and writes to cache
        // 3. Request A tries to populate cache with v1 (stale data)
        // 4. Without version comparison, v1 would overwrite v2 in `latest` key
        //
        // The fix ensures `version > current_version` check before updating `latest`.
        assert!(
            CACHE_SET_SCRIPT.contains("version > current_version"),
            "CACHE_SET_SCRIPT should include version comparison to prevent stale cache population"
        );
    }

    #[rstest]
    fn test_cache_set_script_always_sets_versioned_data_key() {
        // Verify that CACHE_SET_SCRIPT always sets the versioned data key.
        // The data key is safe to set unconditionally because it includes the version.
        // Stale data in a versioned key won't be read (only latest pointer matters).
        assert!(
            CACHE_SET_SCRIPT.contains("SETEX"),
            "CACHE_SET_SCRIPT should always SETEX the versioned data key"
        );
    }

    #[rstest]
    fn test_cache_set_script_conditionally_updates_latest() {
        // Verify that CACHE_SET_SCRIPT only updates `latest` when version is greater.
        // This is the core fix for the concurrent update race condition.
        assert!(
            CACHE_SET_SCRIPT.contains("if version > current_version then"),
            "CACHE_SET_SCRIPT should conditionally update latest based on version comparison"
        );
        assert!(
            CACHE_SET_SCRIPT.contains("redis.call('SET', latest_key, version)"),
            "CACHE_SET_SCRIPT should SET latest_key inside the version check"
        );
    }

    #[rstest]
    fn test_cache_set_script_handles_missing_latest_key() {
        // Verify that CACHE_SET_SCRIPT handles the case where `latest` key doesn't exist.
        // In this case, current_version should default to 0, allowing any version > 0.
        assert!(
            CACHE_SET_SCRIPT.contains("current_version and tonumber(current_version) or 0"),
            "CACHE_SET_SCRIPT should default current_version to 0 when latest key is missing"
        );
    }

    // -------------------------------------------------------------------------
    // Lua Script Consistency Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_cache_scripts_use_consistent_version_comparison() {
        // Both CACHE_SET_SCRIPT and CACHE_UPDATE_SCRIPT should use version comparison.
        // CACHE_SET_SCRIPT: `version > current_version` (strict greater)
        // CACHE_UPDATE_SCRIPT: `new_version <= current_version` (skip if not greater)
        //
        // Both ensure the same invariant: cache always reflects the newest version.
        assert!(
            CACHE_SET_SCRIPT.contains("version > current_version"),
            "CACHE_SET_SCRIPT should check version > current_version"
        );
        assert!(
            CACHE_UPDATE_SCRIPT.contains("new_version <= current_version"),
            "CACHE_UPDATE_SCRIPT should check new_version <= current_version"
        );
    }
}
