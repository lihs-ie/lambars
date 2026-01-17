//! Dependency injection container for the bank application.
//!
//! This module provides the `AppDependencies` struct which holds all
//! infrastructure dependencies needed by the application. This follows
//! the dependency injection pattern to enable:
//!
//! - Easy testing with mock implementations
//! - Clear separation between interface and implementation
//! - Centralized dependency management
//!
//! # Example
//!
//! ```rust,ignore
//! use bank::infrastructure::{AppConfig, AppDependencies};
//! use std::sync::Arc;
//!
//! async fn setup() -> AppDependencies {
//!     let config = AppConfig::from_env().unwrap();
//!     let event_store = Arc::new(PostgresEventStore::new(pool));
//!     let read_model = Arc::new(RedisReadModelCache::new(client));
//!
//!     AppDependencies::new(config, event_store, read_model)
//! }
//! ```

use std::sync::Arc;

use super::config::AppConfig;
use super::event_store::EventStore;
use super::read_model::ReadModelCache;

/// Application dependency container.
///
/// Holds all infrastructure dependencies needed by the application.
/// All dependencies are held behind trait objects to allow for
/// different implementations (production vs test).
///
/// # Thread Safety
///
/// All dependencies are wrapped in `Arc` and implement `Send + Sync`,
/// making this container safe to share across threads.
///
/// # Example
///
/// ```rust,ignore
/// use bank::infrastructure::{AppConfig, AppDependencies};
///
/// let dependencies = AppDependencies::new(config, event_store, read_model);
///
/// // Access dependencies
/// let store = dependencies.event_store();
/// let cache = dependencies.read_model();
/// ```
#[derive(Clone)]
pub struct AppDependencies {
    /// Application configuration.
    config: AppConfig,
    /// Event store for event sourcing.
    event_store: Arc<dyn EventStore>,
    /// Read model cache for CQRS queries.
    read_model: Arc<dyn ReadModelCache>,
}

impl AppDependencies {
    /// Creates a new `AppDependencies` container.
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration
    /// * `event_store` - Event store implementation
    /// * `read_model` - Read model cache implementation
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let dependencies = AppDependencies::new(
    ///     config,
    ///     Arc::new(postgres_event_store),
    ///     Arc::new(redis_cache),
    /// );
    /// ```
    #[must_use]
    pub fn new(
        config: AppConfig,
        event_store: Arc<dyn EventStore>,
        read_model: Arc<dyn ReadModelCache>,
    ) -> Self {
        Self {
            config,
            event_store,
            read_model,
        }
    }

    /// Returns a reference to the application configuration.
    #[must_use]
    pub const fn config(&self) -> &AppConfig {
        &self.config
    }

    /// Returns a reference to the event store.
    #[must_use]
    pub fn event_store(&self) -> &Arc<dyn EventStore> {
        &self.event_store
    }

    /// Returns a reference to the read model cache.
    #[must_use]
    pub fn read_model(&self) -> &Arc<dyn ReadModelCache> {
        &self.read_model
    }

    /// Returns the database URL from configuration.
    #[must_use]
    pub fn database_url(&self) -> &str {
        &self.config.database_url
    }

    /// Returns the Redis URL from configuration.
    #[must_use]
    pub fn redis_url(&self) -> &str {
        &self.config.redis_url
    }

    /// Returns the SQS endpoint from configuration.
    #[must_use]
    pub fn sqs_endpoint(&self) -> &str {
        &self.config.sqs_endpoint
    }

    /// Returns the SQS events queue URL from configuration.
    #[must_use]
    pub fn sqs_events_queue_url(&self) -> &str {
        &self.config.sqs_events_queue_url
    }

    /// Returns the SQS projections queue URL from configuration.
    #[must_use]
    pub fn sqs_projections_queue_url(&self) -> &str {
        &self.config.sqs_projections_queue_url
    }

    /// Returns the snapshot threshold from configuration.
    #[must_use]
    pub const fn snapshot_threshold(&self) -> u64 {
        self.config.snapshot_threshold
    }

    /// Returns the application host from configuration.
    #[must_use]
    pub fn app_host(&self) -> &str {
        &self.config.app_host
    }

    /// Returns the application port from configuration.
    #[must_use]
    pub const fn app_port(&self) -> u16 {
        self.config.app_port
    }
}

impl std::fmt::Debug for AppDependencies {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AppDependencies")
            .field("config", &self.config)
            .field("event_store", &"<dyn EventStore>")
            .field("read_model", &"<dyn ReadModelCache>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::{AccountId, Currency, Money};
    use crate::infrastructure::{CachedBalance, EventStoreError, ReadModelError};
    use lambars::effect::AsyncIO;
    use lambars::persistent::PersistentList;
    use rstest::rstest;

    use crate::domain::account::events::AccountEvent;

    // =========================================================================
    // Mock Implementations for Testing
    // =========================================================================

    struct MockEventStore;

    impl EventStore for MockEventStore {
        fn append_events(
            &self,
            _aggregate_id: &AccountId,
            _expected_version: u64,
            _events: Vec<AccountEvent>,
        ) -> AsyncIO<Result<(), EventStoreError>> {
            AsyncIO::pure(Ok(()))
        }

        fn load_events(
            &self,
            _aggregate_id: &AccountId,
        ) -> AsyncIO<Result<PersistentList<AccountEvent>, EventStoreError>> {
            AsyncIO::new(|| async { Ok(PersistentList::new()) })
        }

        fn load_events_from_version(
            &self,
            _aggregate_id: &AccountId,
            _from_version: u64,
        ) -> AsyncIO<Result<PersistentList<AccountEvent>, EventStoreError>> {
            AsyncIO::new(|| async { Ok(PersistentList::new()) })
        }
    }

    struct MockReadModelCache;

    impl ReadModelCache for MockReadModelCache {
        fn get_balance(
            &self,
            _account_id: &AccountId,
        ) -> AsyncIO<Result<Option<CachedBalance>, ReadModelError>> {
            AsyncIO::pure(Ok(None))
        }

        fn set_balance(
            &self,
            _account_id: &AccountId,
            _balance: &Money,
            _version: u64,
        ) -> AsyncIO<Result<(), ReadModelError>> {
            AsyncIO::pure(Ok(()))
        }

        fn invalidate(&self, _account_id: &AccountId) -> AsyncIO<Result<(), ReadModelError>> {
            AsyncIO::pure(Ok(()))
        }
    }

    // =========================================================================
    // Test Fixtures
    // =========================================================================

    fn create_test_config() -> AppConfig {
        AppConfig::new(
            "postgres://localhost/bank_test".to_string(),
            "redis://localhost:6379".to_string(),
            "http://localhost:4566".to_string(),
            "http://localhost:4566/events".to_string(),
            "http://localhost:4566/projections".to_string(),
            50,
            "127.0.0.1".to_string(),
            3000,
        )
    }

    fn create_test_dependencies() -> AppDependencies {
        AppDependencies::new(
            create_test_config(),
            Arc::new(MockEventStore),
            Arc::new(MockReadModelCache),
        )
    }

    // =========================================================================
    // AppDependencies::new Tests
    // =========================================================================

    #[rstest]
    fn app_dependencies_new_creates_container() {
        let config = create_test_config();
        let event_store = Arc::new(MockEventStore);
        let read_model = Arc::new(MockReadModelCache);

        let dependencies = AppDependencies::new(config.clone(), event_store, read_model);

        assert_eq!(dependencies.config(), &config);
    }

    // =========================================================================
    // AppDependencies Accessor Tests
    // =========================================================================

    #[rstest]
    fn app_dependencies_config_accessor() {
        let dependencies = create_test_dependencies();

        assert_eq!(
            dependencies.config().database_url,
            "postgres://localhost/bank_test"
        );
    }

    #[rstest]
    fn app_dependencies_database_url() {
        let dependencies = create_test_dependencies();

        assert_eq!(
            dependencies.database_url(),
            "postgres://localhost/bank_test"
        );
    }

    #[rstest]
    fn app_dependencies_redis_url() {
        let dependencies = create_test_dependencies();

        assert_eq!(dependencies.redis_url(), "redis://localhost:6379");
    }

    #[rstest]
    fn app_dependencies_sqs_endpoint() {
        let dependencies = create_test_dependencies();

        assert_eq!(dependencies.sqs_endpoint(), "http://localhost:4566");
    }

    #[rstest]
    fn app_dependencies_sqs_events_queue_url() {
        let dependencies = create_test_dependencies();

        assert_eq!(
            dependencies.sqs_events_queue_url(),
            "http://localhost:4566/events"
        );
    }

    #[rstest]
    fn app_dependencies_sqs_projections_queue_url() {
        let dependencies = create_test_dependencies();

        assert_eq!(
            dependencies.sqs_projections_queue_url(),
            "http://localhost:4566/projections"
        );
    }

    #[rstest]
    fn app_dependencies_snapshot_threshold() {
        let dependencies = create_test_dependencies();

        assert_eq!(dependencies.snapshot_threshold(), 50);
    }

    #[rstest]
    fn app_dependencies_app_host() {
        let dependencies = create_test_dependencies();

        assert_eq!(dependencies.app_host(), "127.0.0.1");
    }

    #[rstest]
    fn app_dependencies_app_port() {
        let dependencies = create_test_dependencies();

        assert_eq!(dependencies.app_port(), 3000);
    }

    // =========================================================================
    // AppDependencies Clone Tests
    // =========================================================================

    #[rstest]
    fn app_dependencies_clone() {
        let original = create_test_dependencies();
        let cloned = original.clone();

        // Config should be equal
        assert_eq!(original.config(), cloned.config());
        // URLs should match
        assert_eq!(original.database_url(), cloned.database_url());
        assert_eq!(original.redis_url(), cloned.redis_url());
    }

    // =========================================================================
    // AppDependencies Debug Tests
    // =========================================================================

    #[rstest]
    fn app_dependencies_debug() {
        let dependencies = create_test_dependencies();
        let debug_str = format!("{dependencies:?}");

        assert!(debug_str.contains("AppDependencies"));
        assert!(debug_str.contains("config"));
        assert!(debug_str.contains("event_store"));
        assert!(debug_str.contains("read_model"));
    }

    // =========================================================================
    // AppDependencies Service Access Tests
    // =========================================================================

    #[rstest]
    fn app_dependencies_event_store_accessor() {
        let dependencies = create_test_dependencies();

        // Should return Arc<dyn EventStore>
        let event_store = dependencies.event_store();
        assert!(Arc::strong_count(event_store) >= 1);
    }

    #[rstest]
    fn app_dependencies_read_model_accessor() {
        let dependencies = create_test_dependencies();

        // Should return Arc<dyn ReadModelCache>
        let read_model = dependencies.read_model();
        assert!(Arc::strong_count(read_model) >= 1);
    }

    #[rstest]
    #[tokio::test]
    async fn app_dependencies_event_store_can_be_used() {
        let dependencies = create_test_dependencies();
        let account_id = AccountId::generate();

        let result = dependencies
            .event_store()
            .load_events(&account_id)
            .run_async()
            .await;

        assert!(result.is_ok());
    }

    #[rstest]
    #[tokio::test]
    async fn app_dependencies_read_model_can_be_used() {
        let dependencies = create_test_dependencies();
        let account_id = AccountId::generate();

        let result = dependencies
            .read_model()
            .get_balance(&account_id)
            .run_async()
            .await;

        assert!(result.is_ok());
    }

    // =========================================================================
    // Thread Safety Tests
    // =========================================================================

    #[rstest]
    fn app_dependencies_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<AppDependencies>();
    }

    #[rstest]
    fn app_dependencies_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<AppDependencies>();
    }

    // =========================================================================
    // Mock Implementation Tests
    // =========================================================================

    #[rstest]
    #[tokio::test]
    async fn mock_event_store_append_events_returns_ok() {
        let store = MockEventStore;
        let account_id = AccountId::generate();

        let result = store
            .append_events(&account_id, 0, vec![])
            .run_async()
            .await;

        assert!(result.is_ok());
    }

    #[rstest]
    #[tokio::test]
    async fn mock_event_store_load_events_returns_empty_list() {
        let store = MockEventStore;
        let account_id = AccountId::generate();

        let result = store.load_events(&account_id).run_async().await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[rstest]
    #[tokio::test]
    async fn mock_read_model_cache_get_balance_returns_none() {
        let cache = MockReadModelCache;
        let account_id = AccountId::generate();

        let result = cache.get_balance(&account_id).run_async().await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[rstest]
    #[tokio::test]
    async fn mock_read_model_cache_set_balance_returns_ok() {
        let cache = MockReadModelCache;
        let account_id = AccountId::generate();
        let balance = Money::new(1000, Currency::JPY);

        let result = cache
            .set_balance(&account_id, &balance, 1)
            .run_async()
            .await;

        assert!(result.is_ok());
    }

    #[rstest]
    #[tokio::test]
    async fn mock_read_model_cache_invalidate_returns_ok() {
        let cache = MockReadModelCache;
        let account_id = AccountId::generate();

        let result = cache.invalidate(&account_id).run_async().await;

        assert!(result.is_ok());
    }
}
