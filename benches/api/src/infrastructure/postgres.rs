//! `PostgreSQL` repository implementations.
//!
//! This module provides `PostgreSQL`-based implementations of the repository traits
//! using `sqlx` for database operations. These implementations are suitable for
//! production use with full ACID transaction support.
//!
//! # Features
//!
//! - Connection pooling with `sqlx::PgPool`
//! - JSONB storage for flexible schema evolution
//! - Optimistic locking with version checking
//! - All operations return `AsyncIO` for effect encapsulation
//!
//! # Table Schema
//!
//! ```sql
//! -- tasks table
//! CREATE TABLE tasks (
//!     id UUID PRIMARY KEY,
//!     data JSONB NOT NULL,
//!     version BIGINT NOT NULL DEFAULT 1,
//!     created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
//!     updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
//! );
//!
//! -- projects table
//! CREATE TABLE projects (
//!     id UUID PRIMARY KEY,
//!     data JSONB NOT NULL,
//!     version BIGINT NOT NULL DEFAULT 1,
//!     created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
//!     updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
//! );
//!
//! -- task_events table
//! CREATE TABLE task_events (
//!     id UUID PRIMARY KEY,
//!     task_id UUID NOT NULL,
//!     event_type VARCHAR(100) NOT NULL,
//!     event_data JSONB NOT NULL,
//!     version BIGINT NOT NULL,
//!     occurred_at TIMESTAMPTZ NOT NULL,
//!     CONSTRAINT unique_task_version UNIQUE (task_id, version)
//! );
//! CREATE INDEX idx_task_events_task_id ON task_events(task_id);
//! ```

use sqlx::PgPool;

use lambars::effect::AsyncIO;

use crate::domain::{
    EventId, Priority, Project, ProjectId, Task, TaskEvent, TaskEventKind, TaskHistory, TaskId,
    TaskStatus,
};
use crate::infrastructure::{
    EventStore, PaginatedResult, Pagination, ProjectRepository, RepositoryError, SearchScope,
    TaskRepository,
};

// =============================================================================
// Helper Functions for DB String Conversion
// =============================================================================

/// Converts `TaskStatus` to its database string representation.
///
/// This ensures consistency with serde's `#[serde(rename_all = "snake_case")]`
/// attribute used in the `TaskStatus` enum, where variants are stored as
/// lowercase `snake_case` in JSONB (e.g., `in_progress` instead of `In Progress`).
const fn status_to_database_string(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "pending",
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Completed => "completed",
        TaskStatus::Cancelled => "cancelled",
    }
}

/// Converts `Priority` to its database string representation.
///
/// This ensures consistency with serde's `#[serde(rename_all = "snake_case")]`
/// attribute used in the `Priority` enum, where variants are stored as
/// lowercase `snake_case` in JSONB (e.g., `low` instead of `Low`).
const fn priority_to_database_string(priority: Priority) -> &'static str {
    match priority {
        Priority::Low => "low",
        Priority::Medium => "medium",
        Priority::High => "high",
        Priority::Critical => "critical",
    }
}

// =============================================================================
// PostgreSQL Task Repository
// =============================================================================

/// `PostgreSQL` implementation of `TaskRepository`.
///
/// Uses a `PostgreSQL` connection pool for efficient connection management.
/// Tasks are stored as JSONB with optimistic locking via version field.
///
/// # Example
///
/// ```ignore
/// use infrastructure::postgres::PostgresTaskRepository;
///
/// let pool = PgPool::connect("postgres://localhost/mydb").await?;
/// let repository = PostgresTaskRepository::new(pool);
/// let task = Task::new(TaskId::generate(), "My Task", Timestamp::now());
///
/// repository.save(&task).run_async().await?;
/// let found = repository.find_by_id(&task.task_id).run_async().await?;
/// ```
#[derive(Debug, Clone)]
pub struct PostgresTaskRepository {
    /// Connection pool for `PostgreSQL`.
    pool: PgPool,
}

impl PostgresTaskRepository {
    /// Creates a new `PostgreSQL` task repository with the given connection pool.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Returns the underlying connection pool.
    ///
    /// Useful for running custom queries or sharing the pool with other repositories.
    #[must_use]
    pub const fn pool(&self) -> &PgPool {
        &self.pool
    }
}

// =============================================================================
// Bulk Insert Helper Types and Functions
// =============================================================================

/// Serialized task data ready for database operations.
struct SerializedTask {
    /// Original index in the input slice for result ordering.
    original_index: usize,
    /// Task ID as UUID.
    task_id: uuid::Uuid,
    /// Serialized task data as JSON.
    data: serde_json::Value,
    /// Task version.
    version: i64,
}

/// Classified tasks for bulk operations.
struct ClassifiedTasks {
    /// New tasks (version == 1) for bulk INSERT.
    new_tasks: Vec<SerializedTask>,
    /// Existing tasks (version > 1) for individual UPDATE.
    update_tasks: Vec<SerializedTask>,
}

/// Result of detecting duplicate IDs in the input.
struct DuplicateDetectionResult {
    /// Indices of tasks that should be skipped due to duplicate IDs.
    /// These indices should be marked as `VersionConflict`.
    duplicate_indices: Vec<usize>,
}

/// Detects duplicate IDs in the input task list.
///
/// This is a pure function that identifies tasks with duplicate IDs.
/// Only the first occurrence of each ID is considered valid;
/// subsequent occurrences are marked as duplicates.
///
/// # Arguments
///
/// * `tasks` - Slice of tasks to check for duplicates
///
/// # Returns
///
/// `DuplicateDetectionResult` containing indices of duplicate tasks
fn detect_duplicate_ids(tasks: &[Task]) -> DuplicateDetectionResult {
    let mut seen_ids: std::collections::HashSet<uuid::Uuid> =
        std::collections::HashSet::with_capacity(tasks.len());
    let mut duplicate_indices = Vec::new();

    for (index, task) in tasks.iter().enumerate() {
        let task_id = *task.task_id.as_uuid();
        if !seen_ids.insert(task_id) {
            // ID was already seen, this is a duplicate
            duplicate_indices.push(index);
        }
    }

    DuplicateDetectionResult { duplicate_indices }
}

/// Classifies tasks into new/update categories and serializes them.
///
/// This is a pure function that separates tasks based on version:
/// - version == 1: New task for bulk INSERT
/// - version > 1: Existing task for individual UPDATE
///
/// Duplicate IDs (same ID appearing multiple times in input) are detected
/// and only the first occurrence is processed; subsequent occurrences are
/// marked as `VersionConflict` in the results.
///
/// Returns classified tasks and a pre-allocated results vector.
fn classify_and_serialize_tasks(
    tasks: &[Task],
) -> (ClassifiedTasks, Vec<Result<(), RepositoryError>>) {
    let mut results: Vec<Result<(), RepositoryError>> = vec![Ok(()); tasks.len()];
    let mut new_tasks = Vec::new();
    let mut update_tasks = Vec::new();

    // First, detect duplicates
    let duplicate_result = detect_duplicate_ids(tasks);
    let duplicate_set: std::collections::HashSet<usize> =
        duplicate_result.duplicate_indices.into_iter().collect();

    for (index, task) in tasks.iter().enumerate() {
        // Skip duplicates - mark them as VersionConflict
        // This matches InMemory behavior: after the first task (version=1) is saved,
        // subsequent tasks with the same ID would need version=2 to succeed.
        if duplicate_set.contains(&index) {
            results[index] = Err(RepositoryError::VersionConflict {
                expected: 2,
                found: task.version,
            });
            continue;
        }

        // Serialize task data
        let serialized_data = match serde_json::to_value(task) {
            Ok(data) => data,
            Err(error) => {
                results[index] = Err(RepositoryError::SerializationError(error.to_string()));
                continue;
            }
        };

        #[allow(clippy::cast_possible_wrap)]
        let version = task.version as i64;

        let serialized = SerializedTask {
            original_index: index,
            task_id: *task.task_id.as_uuid(),
            data: serialized_data,
            version,
        };

        match task.version {
            1 => new_tasks.push(serialized),
            v if v > 1 => update_tasks.push(serialized),
            _ => {
                // version == 0 is invalid for save
                results[index] = Err(RepositoryError::VersionConflict {
                    expected: 1,
                    found: task.version,
                });
            }
        }
    }

    (
        ClassifiedTasks {
            new_tasks,
            update_tasks,
        },
        results,
    )
}

/// Executes bulk INSERT for new tasks using UNNEST with RETURNING.
///
/// Uses `PostgreSQL`'s UNNEST for efficient multi-row INSERT.
/// The RETURNING clause ensures atomic determination of which rows were inserted.
///
/// # Atomicity
///
/// This function uses `INSERT ... ON CONFLICT DO NOTHING RETURNING id` to
/// atomically determine which tasks were successfully inserted in a single query.
/// Tasks whose IDs are not returned are considered to have conflicted with
/// existing records and are marked as `VersionConflict`.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `new_tasks` - Tasks to insert (all should have version == 1)
///
/// # Returns
///
/// A vector of results for each task in the same order as input:
/// - `Ok(())` if the task was successfully inserted
/// - `Err(VersionConflict)` if the task already existed
/// - `Err(DatabaseError)` if a database error occurred
async fn execute_bulk_insert(
    pool: &PgPool,
    new_tasks: &[SerializedTask],
) -> Vec<Result<(), RepositoryError>> {
    if new_tasks.is_empty() {
        return Vec::new();
    }

    tracing::info!(
        task_count = new_tasks.len(),
        "Executing UNNEST bulk INSERT for new tasks"
    );

    // Prepare arrays for UNNEST
    let ids: Vec<uuid::Uuid> = new_tasks.iter().map(|task| task.task_id).collect();
    let data_values: Vec<serde_json::Value> =
        new_tasks.iter().map(|task| task.data.clone()).collect();
    let versions: Vec<i64> = new_tasks.iter().map(|task| task.version).collect();

    // Execute bulk INSERT with ON CONFLICT DO NOTHING RETURNING id
    // RETURNING gives us the IDs of successfully inserted rows atomically
    let insert_result: Result<Vec<(uuid::Uuid,)>, _> = sqlx::query_as(
        "INSERT INTO tasks (id, data, version, created_at, updated_at) \
         SELECT id, data, version, NOW(), NOW() \
         FROM UNNEST($1::uuid[], $2::jsonb[], $3::bigint[]) AS t(id, data, version) \
         ON CONFLICT (id) DO NOTHING \
         RETURNING id",
    )
    .bind(&ids)
    .bind(&data_values)
    .bind(&versions)
    .fetch_all(pool)
    .await;

    match insert_result {
        Ok(inserted_rows) => {
            // Build a set of successfully inserted IDs
            let inserted_ids: std::collections::HashSet<uuid::Uuid> =
                inserted_rows.into_iter().map(|(id,)| id).collect();

            // Map results based on whether the ID was returned
            new_tasks
                .iter()
                .map(|task| {
                    if inserted_ids.contains(&task.task_id) {
                        // ID was returned by RETURNING - successfully inserted
                        Ok(())
                    } else {
                        // ID was not returned - task already existed (conflict)
                        // INSERT conflict means: we tried to insert a new task (version=1)
                        // but a record with this ID already exists.
                        // Use expected=0 to indicate "new insert failed (expected no existing record)"
                        // and found=1 to indicate "a record was found to exist".
                        Err(RepositoryError::VersionConflict {
                            expected: 0,
                            found: 1,
                        })
                    }
                })
                .collect()
        }
        Err(error) => {
            // Bulk INSERT failed - mark all new tasks as failed
            vec![Err(RepositoryError::DatabaseError(error.to_string())); new_tasks.len()]
        }
    }
}

/// Executes individual UPDATE operations for existing tasks.
///
/// Each update uses optimistic locking to ensure version consistency.
/// Returns a vector of results for each update task.
async fn execute_individual_updates(
    pool: &PgPool,
    update_tasks: &[SerializedTask],
) -> Vec<Result<(), RepositoryError>> {
    let mut results = Vec::with_capacity(update_tasks.len());

    for task in update_tasks {
        let result = execute_single_update(pool, task).await;
        results.push(result);
    }

    results
}

/// Executes a single UPDATE with optimistic locking.
async fn execute_single_update(
    pool: &PgPool,
    task: &SerializedTask,
) -> Result<(), RepositoryError> {
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

    // Check current version for optimistic locking
    let existing_row: Option<(i64,)> =
        sqlx::query_as("SELECT version FROM tasks WHERE id = $1 FOR UPDATE")
            .bind(task.task_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

    match existing_row {
        Some((existing_version,)) => {
            let expected_version = existing_version + 1;
            if task.version == expected_version {
                sqlx::query(
                    "UPDATE tasks SET data = $1, version = $2, updated_at = NOW() WHERE id = $3",
                )
                .bind(&task.data)
                .bind(task.version)
                .bind(task.task_id)
                .execute(&mut *transaction)
                .await
                .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

                transaction
                    .commit()
                    .await
                    .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

                Ok(())
            } else {
                #[allow(clippy::cast_sign_loss)]
                Err(RepositoryError::VersionConflict {
                    expected: expected_version as u64,
                    found: task.version as u64,
                })
            }
        }
        None => {
            // Task doesn't exist - version conflict (expected to exist for update)
            Err(RepositoryError::VersionConflict {
                expected: 1,
                #[allow(clippy::cast_sign_loss)]
                found: task.version as u64,
            })
        }
    }
}

/// Merges operation results back into the original order.
///
/// This is a pure function that combines results from bulk INSERT
/// and individual UPDATE operations into the results vector.
fn merge_results_into_original_order(
    results: &mut [Result<(), RepositoryError>],
    classified: &ClassifiedTasks,
    new_task_results: Vec<Result<(), RepositoryError>>,
    update_results: Vec<Result<(), RepositoryError>>,
) {
    // Merge new task results
    for (task, result) in classified.new_tasks.iter().zip(new_task_results) {
        results[task.original_index] = result;
    }

    // Merge update results
    for (task, result) in classified.update_tasks.iter().zip(update_results) {
        results[task.original_index] = result;
    }

    // Invalid version indices are already set in classify_and_serialize_tasks
}

impl TaskRepository for PostgresTaskRepository {
    fn find_by_id(&self, id: &TaskId) -> AsyncIO<Result<Option<Task>, RepositoryError>> {
        let pool = self.pool.clone();
        let task_id = id.clone();

        AsyncIO::new(move || async move {
            let row: Option<(serde_json::Value,)> =
                sqlx::query_as("SELECT data FROM tasks WHERE id = $1")
                    .bind(task_id.as_uuid())
                    .fetch_optional(&pool)
                    .await
                    .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            row.map(|(data,)| serde_json::from_value(data))
                .transpose()
                .map_err(|error| RepositoryError::SerializationError(error.to_string()))
        })
    }

    fn save(&self, task: &Task) -> AsyncIO<Result<(), RepositoryError>> {
        let pool = self.pool.clone();
        let task = task.clone();

        AsyncIO::new(move || async move {
            let task_data = serde_json::to_value(&task)
                .map_err(|error| RepositoryError::SerializationError(error.to_string()))?;

            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            // Check current version for optimistic locking
            let existing_row: Option<(i64,)> =
                sqlx::query_as("SELECT version FROM tasks WHERE id = $1 FOR UPDATE")
                    .bind(task.task_id.as_uuid())
                    .fetch_optional(&mut *transaction)
                    .await
                    .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            #[allow(clippy::cast_possible_wrap)]
            let new_version = task.version as i64;

            if let Some((existing_version,)) = existing_row {
                // Update case: version must be existing + 1
                let expected_version = existing_version + 1;
                if new_version != expected_version {
                    return Err(RepositoryError::VersionConflict {
                        #[allow(clippy::cast_sign_loss)]
                        expected: expected_version as u64,
                        found: task.version,
                    });
                }

                sqlx::query(
                    "UPDATE tasks SET data = $1, version = $2, updated_at = NOW() WHERE id = $3",
                )
                .bind(&task_data)
                .bind(new_version)
                .bind(task.task_id.as_uuid())
                .execute(&mut *transaction)
                .await
                .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;
            } else {
                // New entity case: version must be 1
                if task.version != 1 {
                    return Err(RepositoryError::VersionConflict {
                        expected: 1,
                        found: task.version,
                    });
                }

                sqlx::query(
                    "INSERT INTO tasks (id, data, version, created_at, updated_at) \
                     VALUES ($1, $2, $3, NOW(), NOW())",
                )
                .bind(task.task_id.as_uuid())
                .bind(&task_data)
                .bind(new_version)
                .execute(&mut *transaction)
                .await
                .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;
            }

            transaction
                .commit()
                .await
                .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            Ok(())
        })
    }

    fn save_bulk(&self, tasks: &[Task]) -> AsyncIO<Vec<Result<(), RepositoryError>>> {
        let pool = self.pool.clone();
        let tasks_to_save: Vec<Task> = tasks.to_vec();

        AsyncIO::new(move || async move {
            if tasks_to_save.is_empty() {
                return Vec::new();
            }

            // Phase 1: Classify tasks and serialize (pure functions)
            let (classified_tasks, mut results) = classify_and_serialize_tasks(&tasks_to_save);

            // Phase 2: Execute bulk INSERT for new tasks
            let new_task_results = execute_bulk_insert(&pool, &classified_tasks.new_tasks).await;

            // Phase 3: Execute individual UPDATEs for existing tasks (optimistic locking)
            let update_results =
                execute_individual_updates(&pool, &classified_tasks.update_tasks).await;

            // Phase 4: Merge results back to original order (pure function)
            merge_results_into_original_order(
                &mut results,
                &classified_tasks,
                new_task_results,
                update_results,
            );

            results
        })
    }

    fn delete(&self, id: &TaskId) -> AsyncIO<Result<bool, RepositoryError>> {
        let pool = self.pool.clone();
        let task_id = id.clone();

        AsyncIO::new(move || async move {
            let result = sqlx::query("DELETE FROM tasks WHERE id = $1")
                .bind(task_id.as_uuid())
                .execute(&pool)
                .await
                .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            Ok(result.rows_affected() > 0)
        })
    }

    fn list(
        &self,
        pagination: Pagination,
    ) -> AsyncIO<Result<PaginatedResult<Task>, RepositoryError>> {
        let pool = self.pool.clone();

        AsyncIO::new(move || async move {
            // Get total count
            let count_row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks")
                .fetch_one(&pool)
                .await
                .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            #[allow(clippy::cast_sign_loss)]
            let total = count_row.0 as u64;

            if total == 0 {
                return Ok(PaginatedResult::new(
                    vec![],
                    0,
                    pagination.page,
                    pagination.page_size,
                ));
            }

            // Get paginated tasks ordered by created_at
            #[allow(clippy::cast_possible_wrap)]
            let offset = pagination.offset() as i64;
            #[allow(clippy::cast_possible_wrap)]
            let limit = i64::from(pagination.limit());

            let rows: Vec<(serde_json::Value,)> =
                sqlx::query_as("SELECT data FROM tasks ORDER BY created_at ASC LIMIT $1 OFFSET $2")
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&pool)
                    .await
                    .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            let tasks: Result<Vec<Task>, _> = rows
                .into_iter()
                .map(|(data,)| {
                    serde_json::from_value(data)
                        .map_err(|error| RepositoryError::SerializationError(error.to_string()))
                })
                .collect();

            Ok(PaginatedResult::new(
                tasks?,
                total,
                pagination.page,
                pagination.page_size,
            ))
        })
    }

    fn list_filtered(
        &self,
        status: Option<TaskStatus>,
        priority: Option<Priority>,
        pagination: Pagination,
    ) -> AsyncIO<Result<PaginatedResult<Task>, RepositoryError>> {
        let pool = self.pool.clone();

        AsyncIO::new(move || async move {
            // Build dynamic WHERE clause conditions (pure function logic)
            let mut conditions = vec!["1=1".to_string()];
            let mut bind_index = 1;

            if status.is_some() {
                conditions.push(format!("data->>'status' = ${bind_index}"));
                bind_index += 1;
            }
            if priority.is_some() {
                conditions.push(format!("data->>'priority' = ${bind_index}"));
                bind_index += 1;
            }

            let where_clause = conditions.join(" AND ");

            // Count query
            let count_sql = format!("SELECT COUNT(*) FROM tasks WHERE {where_clause}");
            let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql);

            if let Some(status_value) = status {
                count_query = count_query.bind(status_to_database_string(status_value));
            }
            if let Some(priority_value) = priority {
                count_query = count_query.bind(priority_to_database_string(priority_value));
            }

            let count_row = count_query
                .fetch_one(&pool)
                .await
                .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            #[allow(clippy::cast_sign_loss)]
            let total = count_row.0 as u64;

            if total == 0 {
                return Ok(PaginatedResult::new(
                    vec![],
                    0,
                    pagination.page,
                    pagination.page_size,
                ));
            }

            // Data query with pagination
            let data_sql = format!(
                "SELECT data FROM tasks WHERE {} ORDER BY created_at ASC LIMIT ${} OFFSET ${}",
                where_clause,
                bind_index,
                bind_index + 1
            );

            #[allow(clippy::cast_possible_wrap)]
            let offset = pagination.offset() as i64;
            #[allow(clippy::cast_possible_wrap)]
            let limit = i64::from(pagination.limit());

            let mut data_query = sqlx::query_as::<_, (serde_json::Value,)>(&data_sql);

            if let Some(status_value) = status {
                data_query = data_query.bind(status_to_database_string(status_value));
            }
            if let Some(priority_value) = priority {
                data_query = data_query.bind(priority_to_database_string(priority_value));
            }

            data_query = data_query.bind(limit).bind(offset);

            let rows = data_query
                .fetch_all(&pool)
                .await
                .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            // Transform JSON to Task (pure function with filter_map for robustness)
            let tasks: Result<Vec<Task>, _> = rows
                .into_iter()
                .map(|(data,)| {
                    serde_json::from_value(data)
                        .map_err(|error| RepositoryError::SerializationError(error.to_string()))
                })
                .collect();

            Ok(PaginatedResult::new(
                tasks?,
                total,
                pagination.page,
                pagination.page_size,
            ))
        })
    }

    fn search(
        &self,
        query: &str,
        scope: SearchScope,
        limit: u32,
        offset: u32,
    ) -> AsyncIO<Result<Vec<Task>, RepositoryError>> {
        let pool = self.pool.clone();
        let search_pattern = format!("%{}%", query.to_lowercase());
        let search_tag = query.to_lowercase();

        AsyncIO::new(move || async move {
            #[allow(clippy::cast_possible_wrap)]
            let limit_i64 = i64::from(limit);
            #[allow(clippy::cast_possible_wrap)]
            let offset_i64 = i64::from(offset);

            // Execute search based on scope
            // - Title: LIKE search with pg_trgm index
            // - Tags: Containment operator (@>) with GIN index
            // - All: OR combination with separate parameters for title (LIKE) and tag (exact)
            let rows: Vec<(serde_json::Value,)> = match scope {
                SearchScope::Title => {
                    let sql = "SELECT data FROM tasks \
                               WHERE LOWER(data->>'title') LIKE $1 \
                               ORDER BY created_at DESC \
                               LIMIT $2 OFFSET $3";
                    sqlx::query_as(sql)
                        .bind(&search_pattern)
                        .bind(limit_i64)
                        .bind(offset_i64)
                        .fetch_all(&pool)
                        .await
                }
                SearchScope::Tags => {
                    let sql = "SELECT data FROM tasks \
                               WHERE data->'tags' @> to_jsonb(ARRAY[$1::text]) \
                               ORDER BY created_at DESC \
                               LIMIT $2 OFFSET $3";
                    sqlx::query_as(sql)
                        .bind(&search_tag)
                        .bind(limit_i64)
                        .bind(offset_i64)
                        .fetch_all(&pool)
                        .await
                }
                SearchScope::All => {
                    // Title: LIKE pattern ($1)
                    // Tag: exact match ($2)
                    let sql = "SELECT data FROM tasks \
                               WHERE LOWER(data->>'title') LIKE $1 \
                                  OR data->'tags' @> to_jsonb(ARRAY[$2::text]) \
                               ORDER BY created_at DESC \
                               LIMIT $3 OFFSET $4";
                    sqlx::query_as(sql)
                        .bind(&search_pattern)
                        .bind(&search_tag)
                        .bind(limit_i64)
                        .bind(offset_i64)
                        .fetch_all(&pool)
                        .await
                }
            }
            .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            // Transform JSON to Task (pure function - filter_map to handle invalid data gracefully)
            let tasks: Vec<Task> = rows
                .into_iter()
                .filter_map(|(data,)| serde_json::from_value(data).ok())
                .collect();

            Ok(tasks)
        })
    }

    fn count(&self) -> AsyncIO<Result<u64, RepositoryError>> {
        let pool = self.pool.clone();

        AsyncIO::new(move || async move {
            let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks")
                .fetch_one(&pool)
                .await
                .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            #[allow(clippy::cast_sign_loss)]
            Ok(row.0 as u64)
        })
    }
}

// =============================================================================
// PostgreSQL Project Repository
// =============================================================================

/// `PostgreSQL` implementation of `ProjectRepository`.
///
/// Uses a `PostgreSQL` connection pool for efficient connection management.
/// Projects are stored as JSONB with optimistic locking via version field.
///
/// # Example
///
/// ```ignore
/// use infrastructure::postgres::PostgresProjectRepository;
///
/// let pool = PgPool::connect("postgres://localhost/mydb").await?;
/// let repository = PostgresProjectRepository::new(pool);
/// let project = Project::new(ProjectId::generate(), "My Project", Timestamp::now());
///
/// repository.save(&project).run_async().await?;
/// let found = repository.find_by_id(&project.project_id).run_async().await?;
/// ```
#[derive(Debug, Clone)]
pub struct PostgresProjectRepository {
    /// Connection pool for `PostgreSQL`.
    pool: PgPool,
}

impl PostgresProjectRepository {
    /// Creates a new `PostgreSQL` project repository with the given connection pool.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Returns the underlying connection pool.
    ///
    /// Useful for running custom queries or sharing the pool with other repositories.
    #[must_use]
    pub const fn pool(&self) -> &PgPool {
        &self.pool
    }
}

impl ProjectRepository for PostgresProjectRepository {
    fn find_by_id(&self, id: &ProjectId) -> AsyncIO<Result<Option<Project>, RepositoryError>> {
        let pool = self.pool.clone();
        let project_id = id.clone();

        AsyncIO::new(move || async move {
            let row: Option<(serde_json::Value,)> =
                sqlx::query_as("SELECT data FROM projects WHERE id = $1")
                    .bind(project_id.as_uuid())
                    .fetch_optional(&pool)
                    .await
                    .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            row.map(|(data,)| serde_json::from_value(data))
                .transpose()
                .map_err(|error| RepositoryError::SerializationError(error.to_string()))
        })
    }

    fn save(&self, project: &Project) -> AsyncIO<Result<(), RepositoryError>> {
        let pool = self.pool.clone();
        let project = project.clone();

        AsyncIO::new(move || async move {
            let project_data = serde_json::to_value(&project)
                .map_err(|error| RepositoryError::SerializationError(error.to_string()))?;

            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            // Check current version for optimistic locking
            let existing_row: Option<(i64,)> =
                sqlx::query_as("SELECT version FROM projects WHERE id = $1 FOR UPDATE")
                    .bind(project.project_id.as_uuid())
                    .fetch_optional(&mut *transaction)
                    .await
                    .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            #[allow(clippy::cast_possible_wrap)]
            let new_version = project.version as i64;

            if let Some((existing_version,)) = existing_row {
                // Update case: version must be existing + 1
                let expected_version = existing_version + 1;
                if new_version != expected_version {
                    return Err(RepositoryError::VersionConflict {
                        #[allow(clippy::cast_sign_loss)]
                        expected: expected_version as u64,
                        found: project.version,
                    });
                }

                sqlx::query(
                    "UPDATE projects SET data = $1, version = $2, updated_at = NOW() WHERE id = $3",
                )
                .bind(&project_data)
                .bind(new_version)
                .bind(project.project_id.as_uuid())
                .execute(&mut *transaction)
                .await
                .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;
            } else {
                // New entity case: version must be 1
                if project.version != 1 {
                    return Err(RepositoryError::VersionConflict {
                        expected: 1,
                        found: project.version,
                    });
                }

                sqlx::query(
                    "INSERT INTO projects (id, data, version, created_at, updated_at) \
                     VALUES ($1, $2, $3, NOW(), NOW())",
                )
                .bind(project.project_id.as_uuid())
                .bind(&project_data)
                .bind(new_version)
                .execute(&mut *transaction)
                .await
                .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;
            }

            transaction
                .commit()
                .await
                .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            Ok(())
        })
    }

    fn delete(&self, id: &ProjectId) -> AsyncIO<Result<bool, RepositoryError>> {
        let pool = self.pool.clone();
        let project_id = id.clone();

        AsyncIO::new(move || async move {
            let result = sqlx::query("DELETE FROM projects WHERE id = $1")
                .bind(project_id.as_uuid())
                .execute(&pool)
                .await
                .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            Ok(result.rows_affected() > 0)
        })
    }

    fn list(
        &self,
        pagination: Pagination,
    ) -> AsyncIO<Result<PaginatedResult<Project>, RepositoryError>> {
        let pool = self.pool.clone();

        AsyncIO::new(move || async move {
            // Get total count
            let count_row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM projects")
                .fetch_one(&pool)
                .await
                .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            #[allow(clippy::cast_sign_loss)]
            let total = count_row.0 as u64;

            if total == 0 {
                return Ok(PaginatedResult::new(
                    vec![],
                    0,
                    pagination.page,
                    pagination.page_size,
                ));
            }

            // Get paginated projects ordered by created_at
            #[allow(clippy::cast_possible_wrap)]
            let offset = pagination.offset() as i64;
            #[allow(clippy::cast_possible_wrap)]
            let limit = i64::from(pagination.limit());

            let rows: Vec<(serde_json::Value,)> = sqlx::query_as(
                "SELECT data FROM projects ORDER BY created_at ASC LIMIT $1 OFFSET $2",
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&pool)
            .await
            .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            let projects: Result<Vec<Project>, _> = rows
                .into_iter()
                .map(|(data,)| {
                    serde_json::from_value(data)
                        .map_err(|error| RepositoryError::SerializationError(error.to_string()))
                })
                .collect();

            Ok(PaginatedResult::new(
                projects?,
                total,
                pagination.page,
                pagination.page_size,
            ))
        })
    }

    fn count(&self) -> AsyncIO<Result<u64, RepositoryError>> {
        let pool = self.pool.clone();

        AsyncIO::new(move || async move {
            let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM projects")
                .fetch_one(&pool)
                .await
                .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            #[allow(clippy::cast_sign_loss)]
            Ok(row.0 as u64)
        })
    }
}

// =============================================================================
// PostgreSQL Event Store
// =============================================================================

/// Helper function to derive event type string from `TaskEventKind`.
const fn event_type_from_kind(kind: &TaskEventKind) -> &'static str {
    match kind {
        TaskEventKind::Created(_) => "created",
        TaskEventKind::TitleUpdated(_) => "title_updated",
        TaskEventKind::DescriptionUpdated(_) => "description_updated",
        TaskEventKind::StatusChanged(_) => "status_changed",
        TaskEventKind::PriorityChanged(_) => "priority_changed",
        TaskEventKind::TagAdded(_) => "tag_added",
        TaskEventKind::TagRemoved(_) => "tag_removed",
        TaskEventKind::SubTaskAdded(_) => "subtask_added",
        TaskEventKind::SubTaskCompleted(_) => "subtask_completed",
        TaskEventKind::ProjectAssigned(_) => "project_assigned",
        TaskEventKind::ProjectRemoved(_) => "project_removed",
    }
}

/// `PostgreSQL` implementation of `EventStore`.
///
/// Provides event sourcing capabilities with optimistic locking.
/// Events are stored with a unique constraint on (`task_id`, version) to prevent
/// duplicate versions and ensure consistency.
///
/// # Example
///
/// ```ignore
/// use infrastructure::postgres::PostgresEventStore;
///
/// let pool = PgPool::connect("postgres://localhost/mydb").await?;
/// let event_store = PostgresEventStore::new(pool);
///
/// // Append an event
/// event_store.append(&event, 0).run_async().await?;
///
/// // Load events for a task
/// let history = event_store.load_events(&task_id).run_async().await?;
/// ```
#[derive(Debug, Clone)]
pub struct PostgresEventStore {
    /// Connection pool for `PostgreSQL`.
    pool: PgPool,
}

impl PostgresEventStore {
    /// Creates a new `PostgreSQL` event store with the given connection pool.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Returns the underlying connection pool.
    ///
    /// Useful for running custom queries or sharing the pool with other stores.
    #[must_use]
    pub const fn pool(&self) -> &PgPool {
        &self.pool
    }
}

impl EventStore for PostgresEventStore {
    fn append(
        &self,
        event: &TaskEvent,
        expected_version: u64,
    ) -> AsyncIO<Result<(), RepositoryError>> {
        let pool = self.pool.clone();
        let event = event.clone();

        AsyncIO::new(move || async move {
            let event_data = serde_json::to_value(&event.kind)
                .map_err(|error| RepositoryError::SerializationError(error.to_string()))?;

            let event_type = event_type_from_kind(&event.kind);

            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            // Check current version (optimistic lock) using FOR UPDATE to prevent race conditions
            let current_version_row: (i64,) = sqlx::query_as(
                "SELECT COALESCE(MAX(version), 0) FROM task_events WHERE task_id = $1 FOR UPDATE",
            )
            .bind(event.task_id.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            #[allow(clippy::cast_sign_loss)]
            let actual_version = current_version_row.0 as u64;

            if actual_version != expected_version {
                return Err(RepositoryError::VersionConflict {
                    expected: expected_version,
                    found: actual_version,
                });
            }

            // Verify the event version is exactly expected_version + 1
            let required_event_version = expected_version + 1;
            if event.version != required_event_version {
                return Err(RepositoryError::VersionConflict {
                    expected: required_event_version,
                    found: event.version,
                });
            }

            // Insert the event
            #[allow(clippy::cast_possible_wrap)]
            let version_i64 = event.version as i64;

            sqlx::query(
                "INSERT INTO task_events (id, task_id, event_type, event_data, version, occurred_at) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(event.event_id.as_uuid())
            .bind(event.task_id.as_uuid())
            .bind(event_type)
            .bind(&event_data)
            .bind(version_i64)
            .bind(event.timestamp.as_datetime())
            .execute(&mut *transaction)
            .await
            .map_err(|error| {
                // Check if this is a unique constraint violation (duplicate version)
                let error_string = error.to_string();
                if error_string.contains("unique_task_version")
                    || error_string.contains("duplicate key")
                {
                    RepositoryError::VersionConflict {
                        expected: expected_version,
                        found: actual_version,
                    }
                } else {
                    RepositoryError::DatabaseError(error_string)
                }
            })?;

            transaction
                .commit()
                .await
                .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            Ok(())
        })
    }

    fn load_events(&self, task_id: &TaskId) -> AsyncIO<Result<TaskHistory, RepositoryError>> {
        let pool = self.pool.clone();
        let task_id = task_id.clone();

        AsyncIO::new(move || async move {
            let rows: Vec<(
                uuid::Uuid,
                serde_json::Value,
                i64,
                chrono::DateTime<chrono::Utc>,
            )> = sqlx::query_as(
                "SELECT id, event_data, version, occurred_at \
                     FROM task_events \
                     WHERE task_id = $1 \
                     ORDER BY version ASC",
            )
            .bind(task_id.as_uuid())
            .fetch_all(&pool)
            .await
            .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            let events: Result<Vec<TaskEvent>, _> = rows
                .into_iter()
                .map(|(event_id, event_data, version, occurred_at)| {
                    let kind: TaskEventKind = serde_json::from_value(event_data)
                        .map_err(|error| RepositoryError::SerializationError(error.to_string()))?;

                    #[allow(clippy::cast_sign_loss)]
                    Ok(TaskEvent::new(
                        EventId::from_uuid(event_id),
                        task_id.clone(),
                        crate::domain::Timestamp::from_datetime(occurred_at),
                        version as u64,
                        kind,
                    ))
                })
                .collect();

            Ok(events?.into_iter().collect())
        })
    }

    fn load_events_from_version(
        &self,
        task_id: &TaskId,
        from_version: u64,
    ) -> AsyncIO<Result<TaskHistory, RepositoryError>> {
        let pool = self.pool.clone();
        let task_id = task_id.clone();

        AsyncIO::new(move || async move {
            #[allow(clippy::cast_possible_wrap)]
            let from_version_i64 = from_version as i64;

            let rows: Vec<(
                uuid::Uuid,
                serde_json::Value,
                i64,
                chrono::DateTime<chrono::Utc>,
            )> = sqlx::query_as(
                "SELECT id, event_data, version, occurred_at \
                     FROM task_events \
                     WHERE task_id = $1 AND version > $2 \
                     ORDER BY version ASC",
            )
            .bind(task_id.as_uuid())
            .bind(from_version_i64)
            .fetch_all(&pool)
            .await
            .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            let events: Result<Vec<TaskEvent>, _> = rows
                .into_iter()
                .map(|(event_id, event_data, version, occurred_at)| {
                    let kind: TaskEventKind = serde_json::from_value(event_data)
                        .map_err(|error| RepositoryError::SerializationError(error.to_string()))?;

                    #[allow(clippy::cast_sign_loss)]
                    Ok(TaskEvent::new(
                        EventId::from_uuid(event_id),
                        task_id.clone(),
                        crate::domain::Timestamp::from_datetime(occurred_at),
                        version as u64,
                        kind,
                    ))
                })
                .collect();

            Ok(events?.into_iter().collect())
        })
    }

    fn get_current_version(&self, task_id: &TaskId) -> AsyncIO<Result<u64, RepositoryError>> {
        let pool = self.pool.clone();
        let task_id = task_id.clone();

        AsyncIO::new(move || async move {
            let row: (Option<i64>,) =
                sqlx::query_as("SELECT MAX(version) FROM task_events WHERE task_id = $1")
                    .bind(task_id.as_uuid())
                    .fetch_one(&pool)
                    .await
                    .map_err(|error| RepositoryError::DatabaseError(error.to_string()))?;

            #[allow(clippy::cast_sign_loss)]
            Ok(row.0.unwrap_or(0) as u64)
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Priority, TaskCreated, TaskStatus, Timestamp};
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // Helper Functions for Tests
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

    fn test_task_event(task_id: TaskId, version: u64) -> TaskEvent {
        TaskEvent::new(
            EventId::generate(),
            task_id,
            Timestamp::now(),
            version,
            TaskEventKind::Created(TaskCreated {
                title: "Test Task".to_string(),
                description: None,
                priority: Priority::Low,
                status: TaskStatus::Pending,
            }),
        )
    }

    // -------------------------------------------------------------------------
    // Structure Tests (no DB connection required)
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_event_type_from_kind_created() {
        let kind = TaskEventKind::Created(TaskCreated {
            title: "Test".to_string(),
            description: None,
            priority: Priority::Low,
            status: TaskStatus::Pending,
        });
        assert_eq!(event_type_from_kind(&kind), "created");
    }

    #[rstest]
    fn test_event_type_from_kind_status_changed() {
        let kind = TaskEventKind::StatusChanged(crate::domain::StatusChanged {
            old_status: TaskStatus::Pending,
            new_status: TaskStatus::InProgress,
        });
        assert_eq!(event_type_from_kind(&kind), "status_changed");
    }

    #[rstest]
    fn test_event_type_from_kind_priority_changed() {
        let kind = TaskEventKind::PriorityChanged(crate::domain::PriorityChanged {
            old_priority: Priority::Low,
            new_priority: Priority::High,
        });
        assert_eq!(event_type_from_kind(&kind), "priority_changed");
    }

    #[rstest]
    fn test_event_type_from_kind_tag_added() {
        let kind = TaskEventKind::TagAdded(crate::domain::TagAdded {
            tag: crate::domain::Tag::new("test"),
        });
        assert_eq!(event_type_from_kind(&kind), "tag_added");
    }

    #[rstest]
    fn test_event_type_from_kind_tag_removed() {
        let kind = TaskEventKind::TagRemoved(crate::domain::TagRemoved {
            tag: crate::domain::Tag::new("test"),
        });
        assert_eq!(event_type_from_kind(&kind), "tag_removed");
    }

    // -------------------------------------------------------------------------
    // Duplicate ID Detection Tests (no DB connection required)
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_detect_duplicate_ids_no_duplicates() {
        let tasks = vec![
            test_task("Task 1"),
            test_task("Task 2"),
            test_task("Task 3"),
        ];

        let result = detect_duplicate_ids(&tasks);
        assert!(result.duplicate_indices.is_empty());
    }

    #[rstest]
    fn test_detect_duplicate_ids_with_duplicates() {
        let task_id = TaskId::generate();
        let tasks = vec![
            test_task_with_id(task_id.clone(), "First"),
            test_task("Different"),
            test_task_with_id(task_id, "Duplicate"),
        ];

        let result = detect_duplicate_ids(&tasks);
        assert_eq!(result.duplicate_indices, vec![2]);
    }

    #[rstest]
    fn test_detect_duplicate_ids_multiple_duplicates() {
        let task_id_1 = TaskId::generate();
        let task_id_2 = TaskId::generate();
        let tasks = vec![
            test_task_with_id(task_id_1.clone(), "First A"), // index 0
            test_task_with_id(task_id_2.clone(), "First B"), // index 1
            test_task_with_id(task_id_1.clone(), "Dup A 1"), // index 2 - duplicate of 0
            test_task_with_id(task_id_2, "Dup B 1"),         // index 3 - duplicate of 1
            test_task_with_id(task_id_1, "Dup A 2"),         // index 4 - duplicate of 0
        ];

        let result = detect_duplicate_ids(&tasks);
        assert_eq!(result.duplicate_indices, vec![2, 3, 4]);
    }

    #[rstest]
    fn test_classify_and_serialize_tasks_duplicate_version_conflict() {
        let task_id = TaskId::generate();
        let tasks = vec![
            test_task_with_id(task_id.clone(), "First"),
            test_task("Different"),
            test_task_with_id(task_id, "Duplicate"),
        ];

        let (classified, results) = classify_and_serialize_tasks(&tasks);

        // First and Different should be classified as new tasks
        assert_eq!(classified.new_tasks.len(), 2);
        assert!(classified.update_tasks.is_empty());

        // Results should have VersionConflict for the duplicate (index 2)
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
        match &results[2] {
            Err(RepositoryError::VersionConflict { expected, found }) => {
                // After first task (version=1) is saved, expected becomes 2
                assert_eq!(*expected, 2);
                assert_eq!(*found, 1); // The duplicate task has version 1
            }
            _ => panic!("Expected VersionConflict error for duplicate ID"),
        }
    }

    #[rstest]
    fn test_task_serialization_roundtrip() {
        let task = test_task("Test Task")
            .with_description("Description")
            .with_priority(Priority::High);

        let json = serde_json::to_value(&task).expect("Failed to serialize");
        let deserialized: Task = serde_json::from_value(json).expect("Failed to deserialize");

        assert_eq!(deserialized.title, task.title);
        assert_eq!(deserialized.description, task.description);
        assert_eq!(deserialized.priority, task.priority);
    }

    #[rstest]
    fn test_project_serialization_roundtrip() {
        let project = test_project("Test Project").with_description("Description");

        let json = serde_json::to_value(&project).expect("Failed to serialize");
        let deserialized: Project = serde_json::from_value(json).expect("Failed to deserialize");

        assert_eq!(deserialized.name, project.name);
        assert_eq!(deserialized.description, project.description);
    }

    #[rstest]
    fn test_task_event_serialization_roundtrip() {
        let task_id = TaskId::generate();
        let event = test_task_event(task_id, 1);

        let json = serde_json::to_value(&event.kind).expect("Failed to serialize");
        let deserialized: TaskEventKind =
            serde_json::from_value(json).expect("Failed to deserialize");

        assert!(matches!(deserialized, TaskEventKind::Created(_)));
    }

    // -------------------------------------------------------------------------
    // Integration Tests (require PostgreSQL)
    // -------------------------------------------------------------------------

    // Note: These tests require a running PostgreSQL instance with the schema.
    // They are disabled by default but can be enabled for integration testing.

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires PostgreSQL instance"]
    async fn test_postgres_task_repository_save_and_find() {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/test".into());
        let pool = PgPool::connect(&database_url).await.unwrap();
        let repository = PostgresTaskRepository::new(pool);

        let task = test_task("Test Task");
        let task_id = task.task_id.clone();

        // Save the task
        let save_result = repository.save(&task).run_async().await;
        assert!(save_result.is_ok(), "Save failed: {save_result:?}");

        // Find the task
        let find_result = repository.find_by_id(&task_id).run_async().await;
        assert!(find_result.is_ok());
        let found_task = find_result.unwrap();
        assert!(found_task.is_some());
        assert_eq!(found_task.unwrap().title, "Test Task");

        // Cleanup
        let _ = repository.delete(&task_id).run_async().await;
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires PostgreSQL instance"]
    async fn test_postgres_task_repository_save_update() {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/test".into());
        let pool = PgPool::connect(&database_url).await.unwrap();
        let repository = PostgresTaskRepository::new(pool);

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

        // Cleanup
        let _ = repository.delete(&task_id).run_async().await;
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires PostgreSQL instance"]
    async fn test_postgres_task_repository_version_conflict() {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/test".into());
        let pool = PgPool::connect(&database_url).await.unwrap();
        let repository = PostgresTaskRepository::new(pool);

        let task = test_task("Test Task");
        let task_id = task.task_id.clone();

        // Save the original task
        repository.save(&task).run_async().await.unwrap();

        // Try to save with same version (should fail)
        let conflicting_task = test_task_with_id(task_id.clone(), "Conflicting Task");
        let result = repository.save(&conflicting_task).run_async().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            RepositoryError::VersionConflict { expected, found } => {
                assert_eq!(expected, 2);
                assert_eq!(found, 1);
            }
            error => panic!("Expected VersionConflict error, got: {error:?}"),
        }

        // Cleanup
        let _ = repository.delete(&task_id).run_async().await;
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires PostgreSQL instance"]
    async fn test_postgres_task_repository_delete() {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/test".into());
        let pool = PgPool::connect(&database_url).await.unwrap();
        let repository = PostgresTaskRepository::new(pool);

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
    #[ignore = "Requires PostgreSQL instance"]
    async fn test_postgres_task_repository_delete_not_found() {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/test".into());
        let pool = PgPool::connect(&database_url).await.unwrap();
        let repository = PostgresTaskRepository::new(pool);

        let task_id = TaskId::generate();
        let result = repository.delete(&task_id).run_async().await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires PostgreSQL instance"]
    async fn test_postgres_task_repository_list_with_pagination() {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/test".into());
        let pool = PgPool::connect(&database_url).await.unwrap();
        let repository = PostgresTaskRepository::new(pool);

        // Save some tasks
        let mut task_ids = Vec::new();
        for i in 0..5 {
            let task = test_task(&format!("Task {i}"));
            task_ids.push(task.task_id.clone());
            repository.save(&task).run_async().await.unwrap();
        }

        // Get first page
        let pagination = Pagination::new(0, 2);
        let result = repository.list(pagination).run_async().await.unwrap();
        assert_eq!(result.items.len(), 2);
        assert!(result.total >= 5);

        // Cleanup
        for task_id in task_ids {
            let _ = repository.delete(&task_id).run_async().await;
        }
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires PostgreSQL instance"]
    async fn test_postgres_project_repository_save_and_find() {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/test".into());
        let pool = PgPool::connect(&database_url).await.unwrap();
        let repository = PostgresProjectRepository::new(pool);

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

        // Cleanup
        let _ = repository.delete(&project_id).run_async().await;
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires PostgreSQL instance"]
    async fn test_postgres_project_repository_version_conflict() {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/test".into());
        let pool = PgPool::connect(&database_url).await.unwrap();
        let repository = PostgresProjectRepository::new(pool);

        let project = test_project("Test Project");
        let project_id = project.project_id.clone();

        // Save the original project
        repository.save(&project).run_async().await.unwrap();

        // Try to save with same version (should fail)
        let conflicting_project = test_project_with_id(project_id.clone(), "Conflicting Project");
        let result = repository.save(&conflicting_project).run_async().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            RepositoryError::VersionConflict { expected, found } => {
                assert_eq!(expected, 2);
                assert_eq!(found, 1);
            }
            error => panic!("Expected VersionConflict error, got: {error:?}"),
        }

        // Cleanup
        let _ = repository.delete(&project_id).run_async().await;
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires PostgreSQL instance"]
    async fn test_postgres_event_store_append_and_load() {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/test".into());
        let pool = PgPool::connect(&database_url).await.unwrap();
        let event_store = PostgresEventStore::new(pool);

        let task_id = TaskId::generate();
        let event = test_task_event(task_id.clone(), 1);

        // Append the event
        let append_result = event_store.append(&event, 0).run_async().await;
        assert!(append_result.is_ok());

        // Load events
        let history = event_store.load_events(&task_id).run_async().await.unwrap();
        assert_eq!(history.len(), 1);

        // Check version
        let version = event_store
            .get_current_version(&task_id)
            .run_async()
            .await
            .unwrap();
        assert_eq!(version, 1);

        // Cleanup
        sqlx::query("DELETE FROM task_events WHERE task_id = $1")
            .bind(task_id.as_uuid())
            .execute(event_store.pool())
            .await
            .unwrap();
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires PostgreSQL instance"]
    async fn test_postgres_event_store_version_conflict() {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/test".into());
        let pool = PgPool::connect(&database_url).await.unwrap();
        let event_store = PostgresEventStore::new(pool);

        let task_id = TaskId::generate();
        let event1 = test_task_event(task_id.clone(), 1);

        // Append first event
        event_store.append(&event1, 0).run_async().await.unwrap();

        // Try to append with wrong expected version
        let event2 = test_task_event(task_id.clone(), 2);
        let result = event_store.append(&event2, 0).run_async().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            RepositoryError::VersionConflict { expected, found } => {
                assert_eq!(expected, 0);
                assert_eq!(found, 1);
            }
            error => panic!("Expected VersionConflict error, got: {error:?}"),
        }

        // Cleanup
        sqlx::query("DELETE FROM task_events WHERE task_id = $1")
            .bind(task_id.as_uuid())
            .execute(event_store.pool())
            .await
            .unwrap();
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires PostgreSQL instance"]
    async fn test_postgres_event_store_load_from_version() {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/test".into());
        let pool = PgPool::connect(&database_url).await.unwrap();
        let event_store = PostgresEventStore::new(pool);

        let task_id = TaskId::generate();

        // Append multiple events
        for i in 1..=5 {
            let event = test_task_event(task_id.clone(), i);
            event_store.append(&event, i - 1).run_async().await.unwrap();
        }

        // Load events from version 3
        let history = event_store
            .load_events_from_version(&task_id, 3)
            .run_async()
            .await
            .unwrap();

        // Should have events 4 and 5
        assert_eq!(history.len(), 2);

        // Cleanup
        sqlx::query("DELETE FROM task_events WHERE task_id = $1")
            .bind(task_id.as_uuid())
            .execute(event_store.pool())
            .await
            .unwrap();
    }

    #[rstest]
    #[tokio::test]
    #[ignore = "Requires PostgreSQL instance"]
    async fn test_postgres_event_store_get_current_version_no_events() {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/test".into());
        let pool = PgPool::connect(&database_url).await.unwrap();
        let event_store = PostgresEventStore::new(pool);

        let task_id = TaskId::generate();

        // Get version for non-existent task
        let version = event_store
            .get_current_version(&task_id)
            .run_async()
            .await
            .unwrap();
        assert_eq!(version, 0);
    }
}
