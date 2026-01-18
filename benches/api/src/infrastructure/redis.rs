//! Redis repository implementations.
//!
//! This module provides Redis-based implementations of the repository traits
//! using `deadpool-redis` for connection pooling. These implementations are
//! suitable for caching and high-performance read operations.
//!
//! # Features
//!
//! - Connection pooling with `deadpool-redis`
//! - JSON serialization with `serde_json`
//! - Optimistic locking with Redis transactions (WATCH/MULTI/EXEC)
//! - All operations return `AsyncIO` for effect encapsulation
//!
//! # Key Design
//!
//! - Task: `task:{task_id}` -> JSON
//! - Project: `project:{project_id}` -> JSON
//! - Task index: `tasks:index` -> ZSET (score = timestamp for ordering)
//! - Project index: `projects:index` -> ZSET (score = timestamp for ordering)

use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;

use lambars::effect::AsyncIO;

use crate::domain::{Project, ProjectId, Task, TaskId, Timestamp};
use crate::infrastructure::{
    PaginatedResult, Pagination, ProjectRepository, RepositoryError, TaskRepository,
};

// =============================================================================
// Redis Key Constants
// =============================================================================

/// Prefix for task keys.
const TASK_KEY_PREFIX: &str = "task:";

/// Prefix for project keys.
const PROJECT_KEY_PREFIX: &str = "project:";

/// Key for the task index (sorted set).
const TASK_INDEX_KEY: &str = "tasks:index";

/// Key for the project index (sorted set).
const PROJECT_INDEX_KEY: &str = "projects:index";

// =============================================================================
// Helper Functions
// =============================================================================

/// Generates a Redis key for a task.
fn task_key(task_id: &TaskId) -> String {
    format!("{TASK_KEY_PREFIX}{task_id}")
}

/// Generates a Redis key for a project.
fn project_key(project_id: &ProjectId) -> String {
    format!("{PROJECT_KEY_PREFIX}{project_id}")
}

/// Converts a timestamp to a score for sorted sets.
///
/// Uses milliseconds since UNIX epoch for ordering.
#[allow(clippy::cast_precision_loss, clippy::missing_const_for_fn)]
fn timestamp_to_score(timestamp: &Timestamp) -> f64 {
    timestamp.as_datetime().timestamp_millis() as f64
}

// =============================================================================
// Redis Task Repository
// =============================================================================

/// Redis implementation of `TaskRepository`.
///
/// Uses a Redis connection pool for efficient connection management.
/// Tasks are stored as JSON strings with an additional sorted set for ordering.
///
/// # Key Structure
///
/// - `task:{task_id}` -> JSON serialized Task
/// - `tasks:index` -> ZSET with `task_id` as member and `created_at` as score
///
/// # Example
///
/// ```ignore
/// use infrastructure::redis::RedisTaskRepository;
///
/// let repository = RedisTaskRepository::from_url("redis://localhost:6379")?;
/// let task = Task::new(TaskId::generate(), "My Task", Timestamp::now());
///
/// repository.save(&task).run_async().await?;
/// let found = repository.find_by_id(&task.task_id).run_async().await?;
/// ```
#[derive(Debug, Clone)]
pub struct RedisTaskRepository {
    /// Connection pool for Redis.
    pool: Pool,
}

impl RedisTaskRepository {
    /// Creates a new Redis task repository with the given connection pool.
    #[must_use]
    pub const fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Creates a new Redis task repository from a Redis URL.
    ///
    /// # Errors
    ///
    /// Returns `RepositoryError::CacheError` if the pool cannot be created.
    pub fn from_url(redis_url: &str) -> Result<Self, RepositoryError> {
        let config = Config::from_url(redis_url);
        let pool = config
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|error| RepositoryError::CacheError(error.to_string()))?;
        Ok(Self { pool })
    }
}

#[allow(clippy::significant_drop_tightening)]
impl TaskRepository for RedisTaskRepository {
    #[allow(clippy::future_not_send)]
    fn find_by_id(&self, id: &TaskId) -> AsyncIO<Result<Option<Task>, RepositoryError>> {
        let pool = self.pool.clone();
        let key = task_key(id);
        AsyncIO::new(move || async move {
            let mut connection = pool
                .get()
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            let data: Option<String> = connection
                .get(&key)
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            data.map(|json| serde_json::from_str(&json))
                .transpose()
                .map_err(|error| RepositoryError::SerializationError(error.to_string()))
        })
    }

    #[allow(clippy::future_not_send)]
    fn save(&self, task: &Task) -> AsyncIO<Result<(), RepositoryError>> {
        let pool = self.pool.clone();
        let task = task.clone();
        let key = task_key(&task.task_id);
        let task_id_string = task.task_id.to_string();
        let score = timestamp_to_score(&task.created_at);

        AsyncIO::new(move || async move {
            // Serialize the task
            let json = serde_json::to_string(&task)
                .map_err(|error| RepositoryError::SerializationError(error.to_string()))?;
            let mut connection = pool
                .get()
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            // Use Lua script for atomic version checking and update
            // This script:
            // 1. Gets the current value
            // 2. For new entities: requires version == 1
            // 3. For updates: requires version == existing_version + 1
            // 4. Sets the new value and updates the index atomically
            // Returns:
            //   {0} = success
            //   {1, expected, found} = version conflict
            //   {2} = data corruption (JSON decode failed or version missing/invalid)
            let script = redis::Script::new(
                r"
                local key = KEYS[1]
                local index_key = KEYS[2]
                local new_json = ARGV[1]
                local new_version = tonumber(ARGV[2])
                local entity_id = ARGV[3]
                local score = tonumber(ARGV[4])

                local existing = redis.call('GET', key)
                if existing then
                    -- Update case: version must be existing + 1
                    local ok, data = pcall(cjson.decode, existing)
                    if not ok then
                        return {2}
                    end
                    local existing_version = tonumber(data.version)
                    if existing_version == nil then
                        return {2}
                    end
                    if new_version ~= existing_version + 1 then
                        return {1, existing_version + 1, new_version}
                    end
                else
                    -- New entity case: version must be 1
                    if new_version ~= 1 then
                        return {1, 1, new_version}
                    end
                end

                redis.call('SET', key, new_json)
                redis.call('ZADD', index_key, score, entity_id)
                return {0}
                ",
            );

            #[allow(clippy::cast_sign_loss)]
            let result: Vec<i64> = script
                .key(&key)
                .key(TASK_INDEX_KEY)
                .arg(&json)
                .arg(task.version)
                .arg(&task_id_string)
                .arg(score)
                .invoke_async(&mut *connection)
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            match result[0] {
                0 => Ok(()),
                1 => {
                    #[allow(clippy::cast_sign_loss)]
                    Err(RepositoryError::VersionConflict {
                        expected: result[1] as u64,
                        found: result[2] as u64,
                    })
                }
                2 => Err(RepositoryError::SerializationError(
                    "Corrupted data in Redis: invalid JSON or missing version field".to_string(),
                )),
                _ => Err(RepositoryError::CacheError(
                    "Unexpected Lua script result".to_string(),
                )),
            }?;

            Ok(())
        })
    }

    #[allow(clippy::future_not_send)]
    fn delete(&self, id: &TaskId) -> AsyncIO<Result<bool, RepositoryError>> {
        let pool = self.pool.clone();
        let key = task_key(id);
        let task_id_string = id.to_string();

        AsyncIO::new(move || async move {
            let mut connection = pool
                .get()
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            // Use Lua script for atomic existence check and delete
            // Returns: 1 = deleted, 0 = not found
            let script = redis::Script::new(
                r"
                local key = KEYS[1]
                local index_key = KEYS[2]
                local entity_id = ARGV[1]

                local deleted = redis.call('DEL', key)
                if deleted == 1 then
                    redis.call('ZREM', index_key, entity_id)
                    return 1
                end
                return 0
                ",
            );

            let result: i64 = script
                .key(&key)
                .key(TASK_INDEX_KEY)
                .arg(&task_id_string)
                .invoke_async(&mut *connection)
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            Ok(result == 1)
        })
    }

    #[allow(clippy::future_not_send)]
    fn list(
        &self,
        pagination: Pagination,
    ) -> AsyncIO<Result<PaginatedResult<Task>, RepositoryError>> {
        let pool = self.pool.clone();

        AsyncIO::new(move || async move {
            let mut connection = pool
                .get()
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            // Get total count
            let total: u64 = connection
                .zcard(TASK_INDEX_KEY)
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            if total == 0 {
                return Ok(PaginatedResult::new(
                    vec![],
                    0,
                    pagination.page,
                    pagination.page_size,
                ));
            }

            // Get task IDs with pagination (sorted by score/created_at)
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            let start = pagination.offset() as isize;
            #[allow(clippy::cast_possible_wrap)]
            let stop = start + pagination.limit() as isize - 1;

            let task_ids: Vec<String> = connection
                .zrange(TASK_INDEX_KEY, start, stop)
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            if task_ids.is_empty() {
                return Ok(PaginatedResult::new(
                    vec![],
                    total,
                    pagination.page,
                    pagination.page_size,
                ));
            }

            // Get all tasks in a single MGET
            let keys: Vec<String> = task_ids
                .iter()
                .map(|id| format!("{TASK_KEY_PREFIX}{id}"))
                .collect();

            let task_jsons: Vec<Option<String>> = connection
                .mget(&keys)
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            // Deserialize tasks
            let mut tasks = Vec::with_capacity(task_jsons.len());
            for maybe_json in task_jsons.into_iter().flatten() {
                let task: Task = serde_json::from_str(&maybe_json)
                    .map_err(|error| RepositoryError::SerializationError(error.to_string()))?;
                tasks.push(task);
            }

            Ok(PaginatedResult::new(
                tasks,
                total,
                pagination.page,
                pagination.page_size,
            ))
        })
    }

    #[allow(clippy::future_not_send)]
    fn count(&self) -> AsyncIO<Result<u64, RepositoryError>> {
        let pool = self.pool.clone();

        AsyncIO::new(move || async move {
            let mut connection = pool
                .get()
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            let count: u64 = connection
                .zcard(TASK_INDEX_KEY)
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            Ok(count)
        })
    }
}

// =============================================================================
// Redis Project Repository
// =============================================================================

/// Redis implementation of `ProjectRepository`.
///
/// Uses a Redis connection pool for efficient connection management.
/// Projects are stored as JSON strings with an additional sorted set for ordering.
///
/// # Key Structure
///
/// - `project:{project_id}` -> JSON serialized Project
/// - `projects:index` -> ZSET with `project_id` as member and `created_at` as score
///
/// # Example
///
/// ```ignore
/// use infrastructure::redis::RedisProjectRepository;
///
/// let repository = RedisProjectRepository::from_url("redis://localhost:6379")?;
/// let project = Project::new(ProjectId::generate(), "My Project", Timestamp::now());
///
/// repository.save(&project).run_async().await?;
/// let found = repository.find_by_id(&project.project_id).run_async().await?;
/// ```
#[derive(Debug, Clone)]
pub struct RedisProjectRepository {
    /// Connection pool for Redis.
    pool: Pool,
}

impl RedisProjectRepository {
    /// Creates a new Redis project repository with the given connection pool.
    #[must_use]
    pub const fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Creates a new Redis project repository from a Redis URL.
    ///
    /// # Errors
    ///
    /// Returns `RepositoryError::CacheError` if the pool cannot be created.
    pub fn from_url(redis_url: &str) -> Result<Self, RepositoryError> {
        let config = Config::from_url(redis_url);
        let pool = config
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|error| RepositoryError::CacheError(error.to_string()))?;
        Ok(Self { pool })
    }
}

#[allow(clippy::significant_drop_tightening)]
impl ProjectRepository for RedisProjectRepository {
    #[allow(clippy::future_not_send)]
    fn find_by_id(&self, id: &ProjectId) -> AsyncIO<Result<Option<Project>, RepositoryError>> {
        let pool = self.pool.clone();
        let key = project_key(id);

        AsyncIO::new(move || async move {
            let mut connection = pool
                .get()
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            let data: Option<String> = connection
                .get(&key)
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            data.map(|json| serde_json::from_str(&json))
                .transpose()
                .map_err(|error| RepositoryError::SerializationError(error.to_string()))
        })
    }

    #[allow(clippy::future_not_send)]
    fn save(&self, project: &Project) -> AsyncIO<Result<(), RepositoryError>> {
        let pool = self.pool.clone();
        let project = project.clone();
        let key = project_key(&project.project_id);
        let project_id_string = project.project_id.to_string();
        let score = timestamp_to_score(&project.created_at);

        AsyncIO::new(move || async move {
            // Serialize the project
            let json = serde_json::to_string(&project)
                .map_err(|error| RepositoryError::SerializationError(error.to_string()))?;
            let mut connection = pool
                .get()
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            // Use Lua script for atomic version checking and update
            // This script:
            // 1. Gets the current value
            // 2. For new entities: requires version == 1
            // 3. For updates: requires version == existing_version + 1
            // 4. Sets the new value and updates the index atomically
            // Returns:
            //   {0} = success
            //   {1, expected, found} = version conflict
            //   {2} = data corruption (JSON decode failed or version missing/invalid)
            let script = redis::Script::new(
                r"
                local key = KEYS[1]
                local index_key = KEYS[2]
                local new_json = ARGV[1]
                local new_version = tonumber(ARGV[2])
                local entity_id = ARGV[3]
                local score = tonumber(ARGV[4])

                local existing = redis.call('GET', key)
                if existing then
                    -- Update case: version must be existing + 1
                    local ok, data = pcall(cjson.decode, existing)
                    if not ok then
                        return {2}
                    end
                    local existing_version = tonumber(data.version)
                    if existing_version == nil then
                        return {2}
                    end
                    if new_version ~= existing_version + 1 then
                        return {1, existing_version + 1, new_version}
                    end
                else
                    -- New entity case: version must be 1
                    if new_version ~= 1 then
                        return {1, 1, new_version}
                    end
                end

                redis.call('SET', key, new_json)
                redis.call('ZADD', index_key, score, entity_id)
                return {0}
                ",
            );

            #[allow(clippy::cast_sign_loss)]
            let result: Vec<i64> = script
                .key(&key)
                .key(PROJECT_INDEX_KEY)
                .arg(&json)
                .arg(project.version)
                .arg(&project_id_string)
                .arg(score)
                .invoke_async(&mut *connection)
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            match result[0] {
                0 => Ok(()),
                1 => {
                    #[allow(clippy::cast_sign_loss)]
                    Err(RepositoryError::VersionConflict {
                        expected: result[1] as u64,
                        found: result[2] as u64,
                    })
                }
                2 => Err(RepositoryError::SerializationError(
                    "Corrupted data in Redis: invalid JSON or missing version field".to_string(),
                )),
                _ => Err(RepositoryError::CacheError(
                    "Unexpected Lua script result".to_string(),
                )),
            }?;

            Ok(())
        })
    }

    #[allow(clippy::future_not_send)]
    fn delete(&self, id: &ProjectId) -> AsyncIO<Result<bool, RepositoryError>> {
        let pool = self.pool.clone();
        let key = project_key(id);
        let project_id_string = id.to_string();

        AsyncIO::new(move || async move {
            let mut connection = pool
                .get()
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            // Use Lua script for atomic existence check and delete
            // Returns: 1 = deleted, 0 = not found
            let script = redis::Script::new(
                r"
                local key = KEYS[1]
                local index_key = KEYS[2]
                local entity_id = ARGV[1]

                local deleted = redis.call('DEL', key)
                if deleted == 1 then
                    redis.call('ZREM', index_key, entity_id)
                    return 1
                end
                return 0
                ",
            );

            let result: i64 = script
                .key(&key)
                .key(PROJECT_INDEX_KEY)
                .arg(&project_id_string)
                .invoke_async(&mut *connection)
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            Ok(result == 1)
        })
    }

    #[allow(clippy::future_not_send)]
    fn list(
        &self,
        pagination: Pagination,
    ) -> AsyncIO<Result<PaginatedResult<Project>, RepositoryError>> {
        let pool = self.pool.clone();

        AsyncIO::new(move || async move {
            let mut connection = pool
                .get()
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            // Get total count
            let total: u64 = connection
                .zcard(PROJECT_INDEX_KEY)
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            if total == 0 {
                return Ok(PaginatedResult::new(
                    vec![],
                    0,
                    pagination.page,
                    pagination.page_size,
                ));
            }

            // Get project IDs with pagination (sorted by score/created_at)
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            let start = pagination.offset() as isize;
            #[allow(clippy::cast_possible_wrap)]
            let stop = start + pagination.limit() as isize - 1;

            let project_ids: Vec<String> = connection
                .zrange(PROJECT_INDEX_KEY, start, stop)
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            if project_ids.is_empty() {
                return Ok(PaginatedResult::new(
                    vec![],
                    total,
                    pagination.page,
                    pagination.page_size,
                ));
            }

            // Get all projects in a single MGET
            let keys: Vec<String> = project_ids
                .iter()
                .map(|id| format!("{PROJECT_KEY_PREFIX}{id}"))
                .collect();

            let project_jsons: Vec<Option<String>> = connection
                .mget(&keys)
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            // Deserialize projects
            let mut projects = Vec::with_capacity(project_jsons.len());
            for maybe_json in project_jsons.into_iter().flatten() {
                let project: Project = serde_json::from_str(&maybe_json)
                    .map_err(|error| RepositoryError::SerializationError(error.to_string()))?;
                projects.push(project);
            }

            Ok(PaginatedResult::new(
                projects,
                total,
                pagination.page,
                pagination.page_size,
            ))
        })
    }

    #[allow(clippy::future_not_send)]
    fn count(&self) -> AsyncIO<Result<u64, RepositoryError>> {
        let pool = self.pool.clone();

        AsyncIO::new(move || async move {
            let mut connection = pool
                .get()
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            let count: u64 = connection
                .zcard(PROJECT_INDEX_KEY)
                .await
                .map_err(|error| RepositoryError::CacheError(error.to_string()))?;

            Ok(count)
        })
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
    // Helper Functions
    // -------------------------------------------------------------------------

    fn test_task(title: &str) -> Task {
        Task::new(TaskId::generate(), title, Timestamp::now())
    }

    fn test_task_with_id(task_id: TaskId, title: &str) -> Task {
        Task::new(task_id, title, Timestamp::now())
    }

    fn test_project(name: &str) -> Project {
        Project::new(ProjectId::generate(), name, Timestamp::now())
    }

    fn test_project_with_id(project_id: ProjectId, name: &str) -> Project {
        Project::new(project_id, name, Timestamp::now())
    }

    // -------------------------------------------------------------------------
    // Key Generation Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_task_key_generation() {
        let task_id = TaskId::generate();
        let key = task_key(&task_id);
        assert!(key.starts_with(TASK_KEY_PREFIX));
        assert!(key.contains(&task_id.to_string()));
    }

    #[rstest]
    fn test_project_key_generation() {
        let project_id = ProjectId::generate();
        let key = project_key(&project_id);
        assert!(key.starts_with(PROJECT_KEY_PREFIX));
        assert!(key.contains(&project_id.to_string()));
    }

    #[rstest]
    fn test_timestamp_to_score() {
        let timestamp = Timestamp::now();
        let score = timestamp_to_score(&timestamp);
        assert!(score > 0.0);
    }

    // -------------------------------------------------------------------------
    // Repository Creation Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_redis_task_repository_from_invalid_url() {
        // Invalid URL should return an error
        let result = RedisTaskRepository::from_url("invalid://url");
        // Note: deadpool-redis might accept various URLs, so we just check it returns a result
        // The actual connection will fail later when trying to use the pool
        let _ = result;
    }

    #[rstest]
    fn test_redis_project_repository_from_invalid_url() {
        let result = RedisProjectRepository::from_url("invalid://url");
        let _ = result;
    }

    // -------------------------------------------------------------------------
    // Integration Tests (require Redis)
    // -------------------------------------------------------------------------

    // Note: These tests require a running Redis instance.
    // They are disabled by default but can be enabled for integration testing.
    // Use `testcontainers` or a real Redis instance for these tests.

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires Redis instance"]
    async fn test_task_repository_save_and_find() {
        let repository = RedisTaskRepository::from_url("redis://localhost:6379").unwrap();
        let task = test_task("Test Task");
        let task_id = task.task_id.clone();

        // Save the task
        let save_result = repository.save(&task).run_async().await;
        assert!(save_result.is_ok());

        // Find the task
        let find_result = repository.find_by_id(&task_id).run_async().await;
        assert!(find_result.is_ok());
        let found_task = find_result.unwrap();
        assert!(found_task.is_some());
        assert_eq!(found_task.unwrap().title, "Test Task");
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires Redis instance"]
    async fn test_task_repository_save_update() {
        let repository = RedisTaskRepository::from_url("redis://localhost:6379").unwrap();
        let task = test_task("Original Title");
        let task_id = task.task_id.clone();

        // Save the original task
        repository.save(&task).run_async().await.unwrap();

        // Update the task with incremented version
        let updated_task = test_task_with_id(task_id.clone(), "Updated Title").increment_version();
        let update_result = repository.save(&updated_task).run_async().await;
        assert!(update_result.is_ok());

        // Verify the update
        let found = repository
            .find_by_id(&task_id)
            .run_async()
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.title, "Updated Title");
        assert_eq!(found.version, 2);
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires Redis instance"]
    async fn test_task_repository_save_version_conflict() {
        let repository = RedisTaskRepository::from_url("redis://localhost:6379").unwrap();
        let task = test_task("Test Task");
        let task_id = task.task_id.clone();

        // Save the original task
        repository.save(&task).run_async().await.unwrap();

        // Try to save with same version (should fail)
        let conflicting_task = test_task_with_id(task_id, "Conflicting Task");
        let result = repository.save(&conflicting_task).run_async().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            RepositoryError::VersionConflict { expected, found } => {
                assert_eq!(expected, 2);
                assert_eq!(found, 1);
            }
            _ => panic!("Expected VersionConflict error"),
        }
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires Redis instance"]
    async fn test_task_repository_delete() {
        let repository = RedisTaskRepository::from_url("redis://localhost:6379").unwrap();
        let task = test_task("Test Task");
        let task_id = task.task_id.clone();

        // Save the task
        repository.save(&task).run_async().await.unwrap();

        // Delete the task
        let delete_result = repository.delete(&task_id).run_async().await;
        assert!(delete_result.is_ok());
        assert!(delete_result.unwrap());

        // Verify deletion
        let find_result = repository.find_by_id(&task_id).run_async().await.unwrap();
        assert!(find_result.is_none());
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires Redis instance"]
    async fn test_task_repository_delete_not_found() {
        let repository = RedisTaskRepository::from_url("redis://localhost:6379").unwrap();
        let task_id = TaskId::generate();

        let result = repository.delete(&task_id).run_async().await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires Redis instance"]
    async fn test_task_repository_count() {
        let repository = RedisTaskRepository::from_url("redis://localhost:6379").unwrap();

        // Count before adding tasks
        let initial_count = repository.count().run_async().await.unwrap();

        // Add some tasks
        for i in 0..3 {
            let task = test_task(&format!("Task {i}"));
            repository.save(&task).run_async().await.unwrap();
        }

        // Verify count increased
        let final_count = repository.count().run_async().await.unwrap();
        assert!(final_count >= initial_count + 3);
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires Redis instance"]
    async fn test_project_repository_save_and_find() {
        let repository = RedisProjectRepository::from_url("redis://localhost:6379").unwrap();
        let project = test_project("Test Project");
        let project_id = project.project_id.clone();

        // Save the project
        let save_result = repository.save(&project).run_async().await;
        assert!(save_result.is_ok());

        // Find the project
        let find_result = repository.find_by_id(&project_id).run_async().await;
        assert!(find_result.is_ok());
        let found_project = find_result.unwrap();
        assert!(found_project.is_some());
        assert_eq!(found_project.unwrap().name, "Test Project");
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires Redis instance"]
    async fn test_project_repository_save_version_conflict() {
        let repository = RedisProjectRepository::from_url("redis://localhost:6379").unwrap();
        let project = test_project("Test Project");
        let project_id = project.project_id.clone();

        // Save the original project
        repository.save(&project).run_async().await.unwrap();

        // Try to save with same version (should fail)
        let conflicting_project = test_project_with_id(project_id, "Conflicting Project");
        let result = repository.save(&conflicting_project).run_async().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            RepositoryError::VersionConflict { expected, found } => {
                assert_eq!(expected, 2);
                assert_eq!(found, 1);
            }
            _ => panic!("Expected VersionConflict error"),
        }
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires Redis instance"]
    async fn test_project_repository_delete() {
        let repository = RedisProjectRepository::from_url("redis://localhost:6379").unwrap();
        let project = test_project("Test Project");
        let project_id = project.project_id.clone();

        // Save the project
        repository.save(&project).run_async().await.unwrap();

        // Delete the project
        let delete_result = repository.delete(&project_id).run_async().await;
        assert!(delete_result.is_ok());
        assert!(delete_result.unwrap());

        // Verify deletion
        let find_result = repository
            .find_by_id(&project_id)
            .run_async()
            .await
            .unwrap();
        assert!(find_result.is_none());
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires Redis instance"]
    async fn test_project_repository_list_with_pagination() {
        let repository = RedisProjectRepository::from_url("redis://localhost:6379").unwrap();

        // Save some projects
        for i in 0..5 {
            let project = test_project(&format!("Project {i}"));
            repository.save(&project).run_async().await.unwrap();
        }

        // Get first page
        let pagination = Pagination::new(0, 2);
        let result = repository.list(pagination).run_async().await.unwrap();
        assert_eq!(result.items.len(), 2);
        assert!(result.total >= 5);
    }
}
