//! Event store abstraction for event sourcing.
//!
//! This module provides the trait definition and implementations for
//! event sourcing storage. Events are the source of truth for all
//! aggregate state.
//!
//! # Design
//!
//! - **Trait-based abstraction**: `EventStore` trait allows for different
//!   implementations (Postgres, in-memory for testing, etc.)
//! - **Optimistic locking**: Uses version numbers to prevent concurrent
//!   modification conflicts
//! - **`AsyncIO` integration**: All operations return `AsyncIO` for deferred
//!   execution and composability
//!
//! # Example
//!
//! ```rust,ignore
//! use bank::infrastructure::{EventStore, PostgresEventStore};
//!
//! async fn example(store: &impl EventStore) {
//!     let events = store.load_events(&account_id).run_async().await?;
//!     // Process events...
//! }
//! ```

use lambars::effect::AsyncIO;
use lambars::persistent::PersistentList;

use crate::domain::account::events::AccountEvent;
use crate::domain::value_objects::AccountId;

/// Errors that can occur when interacting with the event store.
///
/// These errors represent failures in event persistence and retrieval
/// operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventStoreError {
    /// Optimistic locking conflict: the aggregate was modified by another process.
    ///
    /// This occurs when attempting to append events with an expected version
    /// that doesn't match the current version in the store.
    ConcurrencyConflict {
        /// The version that was expected.
        expected: u64,
        /// The actual current version in the store.
        actual: u64,
    },
    /// A database operation failed.
    DatabaseError(String),
    /// Event serialization or deserialization failed.
    SerializationError(String),
}

impl std::fmt::Display for EventStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConcurrencyConflict { expected, actual } => {
                write!(
                    formatter,
                    "Concurrency conflict: expected version {expected}, actual {actual}"
                )
            }
            Self::DatabaseError(message) => {
                write!(formatter, "Database error: {message}")
            }
            Self::SerializationError(message) => {
                write!(formatter, "Serialization error: {message}")
            }
        }
    }
}

impl std::error::Error for EventStoreError {}

/// Trait for event store implementations.
///
/// Defines the interface for storing and retrieving domain events.
/// Implementations must be thread-safe (`Send + Sync`).
///
/// # Operations
///
/// - `append_events`: Store new events with optimistic locking
/// - `load_events`: Retrieve all events for an aggregate
/// - `load_events_from_version`: Retrieve events starting from a specific version
///
/// # Example Implementation
///
/// ```rust,ignore
/// use bank::infrastructure::{EventStore, EventStoreError};
/// use lambars::effect::AsyncIO;
/// use lambars::persistent::PersistentList;
///
/// struct InMemoryEventStore {
///     // ... storage
/// }
///
/// impl EventStore for InMemoryEventStore {
///     fn append_events(
///         &self,
///         aggregate_id: &AccountId,
///         expected_version: u64,
///         events: Vec<AccountEvent>,
///     ) -> AsyncIO<Result<(), EventStoreError>> {
///         // Implementation...
///     }
///     // ... other methods
/// }
/// ```
pub trait EventStore: Send + Sync {
    /// Appends events to the store with optimistic locking.
    ///
    /// # Arguments
    ///
    /// * `aggregate_id` - The ID of the aggregate these events belong to
    /// * `expected_version` - The expected current version (for optimistic locking)
    /// * `events` - The events to append
    ///
    /// # Returns
    ///
    /// An `AsyncIO` that, when run:
    /// - Returns `Ok(())` if events were successfully appended
    /// - Returns `Err(EventStoreError::ConcurrencyConflict)` if the expected
    ///   version doesn't match the actual version
    /// - Returns `Err(EventStoreError::DatabaseError)` if a database operation fails
    /// - Returns `Err(EventStoreError::SerializationError)` if event serialization fails
    fn append_events(
        &self,
        aggregate_id: &AccountId,
        expected_version: u64,
        events: Vec<AccountEvent>,
    ) -> AsyncIO<Result<(), EventStoreError>>;

    /// Loads all events for an aggregate.
    ///
    /// # Arguments
    ///
    /// * `aggregate_id` - The ID of the aggregate to load events for
    ///
    /// # Returns
    ///
    /// An `AsyncIO` that, when run:
    /// - Returns `Ok(PersistentList<AccountEvent>)` with all events in order
    /// - Returns `Err(EventStoreError::DatabaseError)` if a database operation fails
    /// - Returns `Err(EventStoreError::SerializationError)` if event deserialization fails
    fn load_events(
        &self,
        aggregate_id: &AccountId,
    ) -> AsyncIO<Result<PersistentList<AccountEvent>, EventStoreError>>;

    /// Loads events for an aggregate starting from a specific version.
    ///
    /// This is useful for partial replay when combined with snapshots.
    ///
    /// # Arguments
    ///
    /// * `aggregate_id` - The ID of the aggregate to load events for
    /// * `from_version` - The version to start loading from (exclusive)
    ///
    /// # Returns
    ///
    /// An `AsyncIO` that, when run:
    /// - Returns `Ok(PersistentList<AccountEvent>)` with events from the specified version
    /// - Returns `Err(EventStoreError::DatabaseError)` if a database operation fails
    /// - Returns `Err(EventStoreError::SerializationError)` if event deserialization fails
    fn load_events_from_version(
        &self,
        aggregate_id: &AccountId,
        from_version: u64,
    ) -> AsyncIO<Result<PersistentList<AccountEvent>, EventStoreError>>;
}

/// Postgres-based event store implementation.
///
/// Stores events in a Postgres database with support for:
/// - Optimistic locking via version numbers
/// - JSON serialization of event data
/// - Efficient queries by aggregate ID and version
///
/// # Database Schema
///
/// Expects a table with the appropriate event sourcing schema.
#[derive(Clone)]
pub struct PostgresEventStore {
    /// The database connection pool.
    pool: sqlx::PgPool,
}

impl PostgresEventStore {
    /// Creates a new `PostgresEventStore` with the given connection pool.
    ///
    /// # Arguments
    ///
    /// * `pool` - A Postgres connection pool
    #[must_use]
    pub const fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    /// Returns a reference to the underlying connection pool.
    ///
    /// This can be useful for running custom queries or managing
    /// database connections.
    #[must_use]
    pub const fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }
}

impl EventStore for PostgresEventStore {
    fn append_events(
        &self,
        aggregate_id: &AccountId,
        expected_version: u64,
        events: Vec<AccountEvent>,
    ) -> AsyncIO<Result<(), EventStoreError>> {
        let pool = self.pool.clone();
        let id = *aggregate_id;

        AsyncIO::new(move || {
            async move {
                let mut transaction = pool
                    .begin()
                    .await
                    .map_err(|error| EventStoreError::DatabaseError(error.to_string()))?;

                // Check current version (optimistic lock)
                let current_version: Option<i64> = sqlx::query_scalar(
                    "SELECT COALESCE(MAX(version), 0) FROM events WHERE aggregate_id = $1",
                )
                .bind(id.as_uuid())
                .fetch_one(&mut *transaction)
                .await
                .map_err(|error| EventStoreError::DatabaseError(error.to_string()))?;

                #[allow(clippy::cast_sign_loss)]
                let actual_version = current_version.unwrap_or(0) as u64;
                if actual_version != expected_version {
                    return Err(EventStoreError::ConcurrencyConflict {
                        expected: expected_version,
                        actual: actual_version,
                    });
                }

                // Insert events
                for (index, event) in events.iter().enumerate() {
                    let version = expected_version + index as u64 + 1;
                    #[allow(clippy::cast_possible_wrap)]
                    let version_i64 = version as i64;
                    let event_data = serde_json::to_value(event)
                        .map_err(|error| EventStoreError::SerializationError(error.to_string()))?;

                    sqlx::query(
                        "INSERT INTO events (aggregate_id, aggregate_type, event_type, event_data, version) \
                         VALUES ($1, $2, $3, $4, $5)",
                    )
                    .bind(id.as_uuid())
                    .bind("Account")
                    .bind(event.event_type())
                    .bind(event_data)
                    .bind(version_i64)
                    .execute(&mut *transaction)
                    .await
                    .map_err(|error| EventStoreError::DatabaseError(error.to_string()))?;
                }

                transaction
                    .commit()
                    .await
                    .map_err(|error| EventStoreError::DatabaseError(error.to_string()))?;

                Ok(())
            }
        })
    }

    fn load_events(
        &self,
        aggregate_id: &AccountId,
    ) -> AsyncIO<Result<PersistentList<AccountEvent>, EventStoreError>> {
        let pool = self.pool.clone();
        let id = *aggregate_id;

        AsyncIO::new(move || async move {
            let rows: Vec<(serde_json::Value,)> = sqlx::query_as(
                "SELECT event_data FROM events WHERE aggregate_id = $1 ORDER BY version ASC",
            )
            .bind(id.as_uuid())
            .fetch_all(&pool)
            .await
            .map_err(|error| EventStoreError::DatabaseError(error.to_string()))?;

            let events: Result<Vec<AccountEvent>, _> = rows
                .into_iter()
                .map(|(event_data,)| {
                    serde_json::from_value(event_data)
                        .map_err(|error| EventStoreError::SerializationError(error.to_string()))
                })
                .collect();

            events.map(|event_list| event_list.into_iter().collect())
        })
    }

    fn load_events_from_version(
        &self,
        aggregate_id: &AccountId,
        from_version: u64,
    ) -> AsyncIO<Result<PersistentList<AccountEvent>, EventStoreError>> {
        let pool = self.pool.clone();
        let id = *aggregate_id;

        AsyncIO::new(move || async move {
            #[allow(clippy::cast_possible_wrap)]
            let from_version_i64 = from_version as i64;
            let rows: Vec<(serde_json::Value,)> = sqlx::query_as(
                "SELECT event_data FROM events \
                     WHERE aggregate_id = $1 AND version > $2 \
                     ORDER BY version ASC",
            )
            .bind(id.as_uuid())
            .bind(from_version_i64)
            .fetch_all(&pool)
            .await
            .map_err(|error| EventStoreError::DatabaseError(error.to_string()))?;

            let events: Result<Vec<AccountEvent>, _> = rows
                .into_iter()
                .map(|(event_data,)| {
                    serde_json::from_value(event_data)
                        .map_err(|error| EventStoreError::SerializationError(error.to_string()))
                })
                .collect();

            events.map(|event_list| event_list.into_iter().collect())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // EventStoreError Tests
    // =========================================================================

    #[rstest]
    fn event_store_error_concurrency_conflict_display() {
        let error = EventStoreError::ConcurrencyConflict {
            expected: 5,
            actual: 7,
        };

        assert_eq!(
            format!("{error}"),
            "Concurrency conflict: expected version 5, actual 7"
        );
    }

    #[rstest]
    fn event_store_error_database_error_display() {
        let error = EventStoreError::DatabaseError("connection refused".to_string());

        assert_eq!(format!("{error}"), "Database error: connection refused");
    }

    #[rstest]
    fn event_store_error_serialization_error_display() {
        let error = EventStoreError::SerializationError("invalid JSON".to_string());

        assert_eq!(format!("{error}"), "Serialization error: invalid JSON");
    }

    #[rstest]
    fn event_store_error_equality() {
        let error1 = EventStoreError::ConcurrencyConflict {
            expected: 1,
            actual: 2,
        };
        let error2 = EventStoreError::ConcurrencyConflict {
            expected: 1,
            actual: 2,
        };
        let error3 = EventStoreError::ConcurrencyConflict {
            expected: 1,
            actual: 3,
        };

        assert_eq!(error1, error2);
        assert_ne!(error1, error3);
    }

    #[rstest]
    fn event_store_error_clone() {
        let original = EventStoreError::DatabaseError("test error".to_string());
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[rstest]
    fn event_store_error_debug() {
        let error = EventStoreError::SerializationError("parse failed".to_string());
        let debug_str = format!("{error:?}");

        assert!(debug_str.contains("SerializationError"));
        assert!(debug_str.contains("parse failed"));
    }

    #[rstest]
    fn event_store_error_is_error_trait() {
        // Verify that EventStoreError implements std::error::Error
        fn assert_error<E: std::error::Error>(_: &E) {}

        let error = EventStoreError::DatabaseError("test".to_string());
        assert_error(&error);
    }

    // =========================================================================
    // EventStoreError Variant Tests
    // =========================================================================

    #[rstest]
    #[case(0, 1)]
    #[case(10, 5)]
    #[case(100, 99)]
    fn event_store_error_concurrency_conflict_values(#[case] expected: u64, #[case] actual: u64) {
        let error = EventStoreError::ConcurrencyConflict { expected, actual };

        match error {
            EventStoreError::ConcurrencyConflict {
                expected: exp,
                actual: act,
            } => {
                assert_eq!(exp, expected);
                assert_eq!(act, actual);
            }
            _ => panic!("Wrong error variant"),
        }
    }

    #[rstest]
    #[case("connection refused")]
    #[case("timeout")]
    #[case("authentication failed")]
    fn event_store_error_database_error_messages(#[case] message: &str) {
        let error = EventStoreError::DatabaseError(message.to_string());

        match error {
            EventStoreError::DatabaseError(msg) => {
                assert_eq!(msg, message);
            }
            _ => panic!("Wrong error variant"),
        }
    }

    #[rstest]
    #[case("invalid JSON")]
    #[case("missing field 'id'")]
    #[case("unknown variant")]
    fn event_store_error_serialization_messages(#[case] message: &str) {
        let error = EventStoreError::SerializationError(message.to_string());

        match error {
            EventStoreError::SerializationError(msg) => {
                assert_eq!(msg, message);
            }
            _ => panic!("Wrong error variant"),
        }
    }

    // =========================================================================
    // PostgresEventStore Tests (Structure only - no DB connection)
    // =========================================================================

    // Note: Actual PostgreSQL integration tests would require a test database.
    // These tests verify the structure and basic properties of PostgresEventStore.

    // PostgresEventStore::new and PostgresEventStore::pool tests would require
    // a database connection pool. These would be integration tests, not unit tests.
}
