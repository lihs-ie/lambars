//! Redis integration tests for `CachedTaskRepository`.
//!
//! These tests verify the cache behavior with an actual Redis instance.
//! All tests are marked with `#[ignore]` and require Redis to be running
//! on `localhost:6379`.
//!
//! # Running the tests
//!
//! ```bash
//! # Start Redis locally
//! docker run -d -p 6379:6379 redis:7-alpine
//!
//! # Run the ignored tests
//! cargo test --test cache_integration -- --ignored
//! ```
//!
//! # Requirements
//!
//! - Redis running on `localhost:6379`
//! - No authentication required
//!
//! # Test Coverage (CACHE-REQ-010, CACHE-REQ-011, CACHE-REQ-012)
//!
//! - Read-through caching: cache miss fetches from primary, populates cache
//! - Write-through caching: writes update both primary and cache
//! - Cache key versioning: old version keys are invalidated on update
//! - Redis failure fallback: fail-open behavior on Redis errors
//! - Cache disabled mode: `CACHE_ENABLED=false` bypasses cache reads

use std::sync::Arc;

use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;
use rstest::rstest;

use task_management_benchmark_api::domain::{Task, TaskId, Timestamp};
use task_management_benchmark_api::infrastructure::{
    CacheConfig, CacheStatus, CacheStrategy, CachedTaskRepository, InMemoryTaskRepository,
    TaskRepository, task_data_key, task_latest_key,
};

// =============================================================================
// Test Fixtures
// =============================================================================

/// Redis connection URL for tests.
const REDIS_URL: &str = "redis://localhost:6379";

/// Creates a Redis connection pool for testing.
fn create_test_pool() -> Pool {
    let config = Config::from_url(REDIS_URL);
    config
        .create_pool(Some(Runtime::Tokio1))
        .expect("Failed to create Redis pool")
}

/// Creates a test task with a unique ID.
fn create_test_task(title: &str) -> Task {
    Task::new(TaskId::generate_v7(), title, Timestamp::now())
}

/// Cleans up test keys from Redis.
///
/// This function removes all keys with the test prefix to ensure
/// test isolation.
async fn cleanup_test_keys(pool: &Pool, task_id: &TaskId) {
    let mut connection = pool.get().await.expect("Failed to get connection");

    // Delete the data key and latest key for this task
    let data_key_pattern = format!("cache:task:{task_id}:v*");
    let latest_key = task_latest_key(task_id);

    // Get all matching keys and delete them
    let keys: Vec<String> = connection.keys(&data_key_pattern).await.unwrap_or_default();

    for key in keys {
        let _: () = connection.del(&key).await.unwrap_or_default();
    }

    let _: () = connection.del(&latest_key).await.unwrap_or_default();
}

/// Verifies Redis is accessible, returns true if connected.
async fn is_redis_available(pool: &Pool) -> bool {
    match pool.get().await {
        Ok(mut connection) => {
            let result: Result<String, _> = redis::cmd("PING").query_async(&mut *connection).await;
            result.is_ok()
        }
        Err(_) => false,
    }
}

// =============================================================================
// Read-Through Tests (CACHE-REQ-010)
// =============================================================================

/// Test: Cache miss fetches from primary storage and populates cache.
///
/// Scenario:
/// 1. Save a task to primary storage only
/// 2. First read should be a cache miss, data fetched from primary
/// 3. Cache should be populated with the task data
/// 4. Second read should be a cache hit
#[rstest]
#[tokio::test]
#[ignore = "requires Redis running on localhost:6379"]
async fn test_cached_task_repository_read_through_miss() {
    // Setup
    let pool = create_test_pool();
    if !is_redis_available(&pool).await {
        eprintln!("Redis not available, skipping test");
        return;
    }

    let primary = Arc::new(InMemoryTaskRepository::new());
    let config = CacheConfig::new(CacheStrategy::ReadThrough, 60, true, 100);
    let cached_repository = CachedTaskRepository::new(primary.clone(), pool.clone(), config);

    // Create and save task to primary only
    let task = create_test_task("Read-through miss test");
    let task_id = task.task_id.clone();

    // Ensure cache is clean before test
    cleanup_test_keys(&pool, &task_id).await;

    // Save directly to primary (bypassing cache)
    primary.save(&task).await.unwrap();

    // First read - should be a cache miss
    let result = cached_repository
        .find_by_id_with_status(&task_id)
        .await
        .unwrap();

    assert_eq!(result.cache_status, CacheStatus::Miss);
    assert!(result.value.is_some());
    assert_eq!(
        result.value.as_ref().unwrap().title,
        "Read-through miss test"
    );

    // Verify cache was populated
    let mut connection = pool.get().await.unwrap();
    let latest_key = task_latest_key(&task_id);
    let cached_version: Option<u64> = connection.get(&latest_key).await.unwrap();
    assert_eq!(cached_version, Some(1), "Cache should have version 1");

    // Second read - should be a cache hit
    let result = cached_repository
        .find_by_id_with_status(&task_id)
        .await
        .unwrap();

    assert_eq!(result.cache_status, CacheStatus::Hit);
    assert!(result.value.is_some());
    assert_eq!(
        result.value.as_ref().unwrap().title,
        "Read-through miss test"
    );

    // Cleanup
    cleanup_test_keys(&pool, &task_id).await;
}

/// Test: Cache hit returns data from cache without primary access.
///
/// Scenario:
/// 1. Save a task through cached repository (populates cache)
/// 2. Read should be a cache hit
#[rstest]
#[tokio::test]
#[ignore = "requires Redis running on localhost:6379"]
async fn test_cached_task_repository_read_through_hit() {
    // Setup
    let pool = create_test_pool();
    if !is_redis_available(&pool).await {
        eprintln!("Redis not available, skipping test");
        return;
    }

    let primary = Arc::new(InMemoryTaskRepository::new());
    let config = CacheConfig::new(CacheStrategy::ReadThrough, 60, true, 100);
    let cached_repository = CachedTaskRepository::new(primary.clone(), pool.clone(), config);

    // Create and save task through cached repository
    let task = create_test_task("Read-through hit test");
    let task_id = task.task_id.clone();

    // Ensure cache is clean before test
    cleanup_test_keys(&pool, &task_id).await;

    // Save through cached repository (this writes to both primary and cache)
    cached_repository.save(&task).await.unwrap();

    // Read - should be a cache hit
    let result = cached_repository
        .find_by_id_with_status(&task_id)
        .await
        .unwrap();

    assert_eq!(result.cache_status, CacheStatus::Hit);
    assert!(result.value.is_some());
    assert_eq!(
        result.value.as_ref().unwrap().title,
        "Read-through hit test"
    );

    // Cleanup
    cleanup_test_keys(&pool, &task_id).await;
}

// =============================================================================
// Write-Through Tests (CACHE-REQ-010)
// =============================================================================

/// Test: Write-through updates both primary storage and cache.
///
/// Scenario:
/// 1. Save a new task through cached repository
/// 2. Verify task is in primary storage
/// 3. Verify task is in cache
/// 4. Update the task
/// 5. Verify both primary and cache have the updated version
#[rstest]
#[tokio::test]
#[ignore = "requires Redis running on localhost:6379"]
async fn test_cached_task_repository_write_through() {
    // Setup
    let pool = create_test_pool();
    if !is_redis_available(&pool).await {
        eprintln!("Redis not available, skipping test");
        return;
    }

    let primary = Arc::new(InMemoryTaskRepository::new());
    let config = CacheConfig::new(CacheStrategy::WriteThrough, 60, true, 100);
    let cached_repository = CachedTaskRepository::new(primary.clone(), pool.clone(), config);

    // Create and save task
    let task = create_test_task("Write-through test");
    let task_id = task.task_id.clone();

    // Ensure cache is clean before test
    cleanup_test_keys(&pool, &task_id).await;

    // Save through cached repository
    cached_repository.save(&task).await.unwrap();

    // Verify in primary
    let primary_result = primary.find_by_id(&task_id).await.unwrap();
    assert!(primary_result.is_some());
    assert_eq!(primary_result.as_ref().unwrap().version, 1);

    // Verify in cache
    let mut connection = pool.get().await.unwrap();
    let latest_key = task_latest_key(&task_id);
    let cached_version: Option<u64> = connection.get(&latest_key).await.unwrap();
    assert_eq!(cached_version, Some(1));

    // Update the task
    let updated_task = task
        .clone()
        .increment_version()
        .with_description("Updated description");

    cached_repository
        .save(&updated_task)
        .await
        .unwrap();

    // Verify primary has updated version
    let primary_result = primary.find_by_id(&task_id).await.unwrap();
    assert!(primary_result.is_some());
    assert_eq!(primary_result.as_ref().unwrap().version, 2);

    // Verify cache has updated version
    let cached_version: Option<u64> = connection.get(&latest_key).await.unwrap();
    assert_eq!(cached_version, Some(2));

    // Verify cache data is correct
    let data_key = task_data_key(&task_id, 2);
    let cached_json: Option<String> = connection.get(&data_key).await.unwrap();
    assert!(cached_json.is_some());

    let cached_task: Task = serde_json::from_str(&cached_json.unwrap()).unwrap();
    assert_eq!(cached_task.version, 2);
    assert_eq!(
        cached_task.description,
        Some("Updated description".to_string())
    );

    // Cleanup
    cleanup_test_keys(&pool, &task_id).await;
}

// =============================================================================
// Cache Key Version Invalidation Tests (CACHE-REQ-011)
// =============================================================================

/// Test: Old version key is deleted when task is updated.
///
/// Scenario:
/// 1. Save a task (creates v1 cache entry)
/// 2. Update the task (creates v2, should delete v1)
/// 3. Verify v1 key no longer exists
/// 4. Verify v2 key exists and latest pointer points to v2
#[rstest]
#[tokio::test]
#[ignore = "requires Redis running on localhost:6379"]
async fn test_cache_key_version_invalidation() {
    // Setup
    let pool = create_test_pool();
    if !is_redis_available(&pool).await {
        eprintln!("Redis not available, skipping test");
        return;
    }

    let primary = Arc::new(InMemoryTaskRepository::new());
    let config = CacheConfig::new(CacheStrategy::ReadThrough, 60, true, 100);
    let cached_repository = CachedTaskRepository::new(primary.clone(), pool.clone(), config);

    // Create and save task
    let task = create_test_task("Version invalidation test");
    let task_id = task.task_id.clone();

    // Ensure cache is clean before test
    cleanup_test_keys(&pool, &task_id).await;

    // Save initial version
    cached_repository.save(&task).await.unwrap();

    // Verify v1 exists
    let mut connection = pool.get().await.unwrap();
    let v1_key = task_data_key(&task_id, 1);
    let v1_exists: bool = connection.exists(&v1_key).await.unwrap();
    assert!(v1_exists, "v1 data key should exist after initial save");

    // Update to v2
    let updated_task = task.clone().increment_version();
    cached_repository
        .save(&updated_task)
        .await
        .unwrap();

    // Verify v1 is deleted
    let v1_exists: bool = connection.exists(&v1_key).await.unwrap();
    assert!(!v1_exists, "v1 data key should be deleted after update");

    // Verify v2 exists
    let v2_key = task_data_key(&task_id, 2);
    let v2_exists: bool = connection.exists(&v2_key).await.unwrap();
    assert!(v2_exists, "v2 data key should exist after update");

    // Verify latest pointer is updated
    let latest_key = task_latest_key(&task_id);
    let latest_version: Option<u64> = connection.get(&latest_key).await.unwrap();
    assert_eq!(latest_version, Some(2), "Latest pointer should be v2");

    // Cleanup
    cleanup_test_keys(&pool, &task_id).await;
}

// =============================================================================
// Redis Failure Fallback Tests (Fail-Open Behavior)
// =============================================================================

/// Test: Redis failure falls back to primary storage.
///
/// This test verifies fail-open behavior by using an invalid Redis URL.
/// The cached repository should fall back to primary storage when Redis
/// is unavailable.
///
/// Note: This test uses a separate pool with an invalid configuration
/// to simulate Redis failure.
#[rstest]
#[tokio::test]
#[ignore = "requires Redis running on localhost:6379"]
async fn test_redis_failure_fallback() {
    // Create a pool with an invalid Redis URL to simulate failure
    // Using a non-existent port to ensure connection failure
    let invalid_config = Config::from_url("redis://localhost:59999");
    let invalid_pool = invalid_config
        .create_pool(Some(Runtime::Tokio1))
        .expect("Failed to create pool (config)");

    let primary = Arc::new(InMemoryTaskRepository::new());
    let config = CacheConfig::new(CacheStrategy::ReadThrough, 60, true, 100);
    let cached_repository =
        CachedTaskRepository::new(primary.clone(), invalid_pool.clone(), config);

    // Create and save task to primary only
    let task = create_test_task("Fallback test");
    let task_id = task.task_id.clone();
    primary.save(&task).await.unwrap();

    // Read through cached repository - should fall back to primary
    // and return Bypass status due to Redis connection failure
    let result = cached_repository
        .find_by_id_with_status(&task_id)
        .await
        .unwrap();

    assert_eq!(
        result.cache_status,
        CacheStatus::Bypass,
        "Should bypass cache on Redis failure"
    );
    assert!(result.value.is_some());
    assert_eq!(result.value.as_ref().unwrap().title, "Fallback test");
}

// =============================================================================
// CACHE_ENABLED=false Tests (CACHE-REQ-012)
// =============================================================================

/// Test: Cache disabled mode bypasses cache for reads.
///
/// Scenario:
/// 1. Create cached repository with `CACHE_ENABLED=false`
/// 2. Save a task (should write to primary, invalidate cache)
/// 3. Read should bypass cache and return from primary
#[rstest]
#[tokio::test]
#[ignore = "requires Redis running on localhost:6379"]
async fn test_cache_disabled_bypass() {
    // Setup
    let pool = create_test_pool();
    if !is_redis_available(&pool).await {
        eprintln!("Redis not available, skipping test");
        return;
    }

    let primary = Arc::new(InMemoryTaskRepository::new());
    // Cache disabled
    let config = CacheConfig::new(CacheStrategy::ReadThrough, 60, false, 100);
    let cached_repository = CachedTaskRepository::new(primary.clone(), pool.clone(), config);

    // Create and save task
    let task = create_test_task("Cache disabled test");
    let task_id = task.task_id.clone();

    // Ensure cache is clean before test
    cleanup_test_keys(&pool, &task_id).await;

    // Save through cached repository (should write to primary only)
    cached_repository.save(&task).await.unwrap();

    // Verify task is in primary
    let primary_result = primary.find_by_id(&task_id).await.unwrap();
    assert!(primary_result.is_some());

    // Read through cached repository - should bypass cache
    let result = cached_repository
        .find_by_id_with_status(&task_id)
        .await
        .unwrap();

    assert_eq!(
        result.cache_status,
        CacheStatus::Bypass,
        "Should bypass cache when disabled"
    );
    assert!(result.value.is_some());
    assert_eq!(result.value.as_ref().unwrap().title, "Cache disabled test");

    // Verify cache was not populated (cache is disabled)
    let mut connection = pool.get().await.unwrap();
    let latest_key = task_latest_key(&task_id);
    let cached_version: Option<u64> = connection.get(&latest_key).await.unwrap();
    assert!(
        cached_version.is_none(),
        "Cache should not be populated when disabled"
    );

    // Cleanup
    cleanup_test_keys(&pool, &task_id).await;
}

/// Test: Cache disabled mode still invalidates cache on writes.
///
/// This ensures that when cache is re-enabled, stale data is not served.
///
/// Scenario:
/// 1. Enable cache and save a task (populates cache)
/// 2. Disable cache and update the task (should invalidate cache)
/// 3. Re-enable cache and read (should be a cache miss, fetch from primary)
#[rstest]
#[tokio::test]
#[ignore = "requires Redis running on localhost:6379"]
async fn test_cache_disabled_invalidates_on_write() {
    // Setup
    let pool = create_test_pool();
    if !is_redis_available(&pool).await {
        eprintln!("Redis not available, skipping test");
        return;
    }

    let primary = Arc::new(InMemoryTaskRepository::new());

    // Create and save task with cache enabled
    let enabled_config = CacheConfig::new(CacheStrategy::ReadThrough, 60, true, 100);
    let enabled_repository =
        CachedTaskRepository::new(primary.clone(), pool.clone(), enabled_config);

    let task = create_test_task("Invalidation on disabled write test");
    let task_id = task.task_id.clone();

    // Ensure cache is clean before test
    cleanup_test_keys(&pool, &task_id).await;

    // Save with cache enabled (populates cache)
    enabled_repository.save(&task).await.unwrap();

    // Verify cache is populated
    let mut connection = pool.get().await.unwrap();
    let latest_key = task_latest_key(&task_id);
    let cached_version: Option<u64> = connection.get(&latest_key).await.unwrap();
    assert_eq!(cached_version, Some(1), "Cache should be populated");

    // Create disabled repository and update task
    let disabled_config = CacheConfig::new(CacheStrategy::ReadThrough, 60, false, 100);
    let disabled_repository =
        CachedTaskRepository::new(primary.clone(), pool.clone(), disabled_config);

    let updated_task = task.clone().increment_version();
    disabled_repository
        .save(&updated_task)
        .await
        .unwrap();

    // Verify cache was invalidated (latest key should be deleted)
    let cached_version: Option<u64> = connection.get(&latest_key).await.unwrap();
    assert!(
        cached_version.is_none(),
        "Cache should be invalidated after disabled write"
    );

    // Re-enable cache and read - should be a cache miss
    let result = enabled_repository
        .find_by_id_with_status(&task_id)
        .await
        .unwrap();

    assert_eq!(
        result.cache_status,
        CacheStatus::Miss,
        "Should be cache miss after invalidation"
    );
    assert!(result.value.is_some());
    assert_eq!(result.value.as_ref().unwrap().version, 2);

    // Cleanup
    cleanup_test_keys(&pool, &task_id).await;
}
