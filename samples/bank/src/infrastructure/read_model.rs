//! Read model cache abstraction for CQRS query side.
//!
//! This module provides caching infrastructure for the read side of CQRS.
//! It abstracts cache operations behind a trait to allow different
//! implementations (Redis, in-memory, etc.).
//!
//! # Design
//!
//! - **Two-level caching**: Supports L1 (local) and L2 (distributed) caches
//! - **Trait-based abstraction**: `ReadModelCache` trait for different implementations
//! - **`AsyncIO` integration**: All operations return `AsyncIO` for deferred execution
//!
//! # Example
//!
//! ```rust,ignore
//! use bank::infrastructure::ReadModelCache;
//!
//! async fn get_balance(cache: &impl ReadModelCache, account_id: &AccountId) {
//!     if let Some(cached) = cache.get_balance(account_id).run_async().await? {
//!         println!("Cached balance: {:?}", cached.balance);
//!     }
//! }
//! ```

use lambars::effect::AsyncIO;

use crate::domain::value_objects::{AccountId, Money, Timestamp};

/// Errors that can occur when interacting with the read model cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadModelError {
    /// A cache operation failed (connection error, timeout, etc.).
    CacheError(String),
    /// Serialization or deserialization of cached data failed.
    SerializationError(String),
}

impl std::fmt::Display for ReadModelError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CacheError(message) => {
                write!(formatter, "Cache error: {message}")
            }
            Self::SerializationError(message) => {
                write!(formatter, "Serialization error: {message}")
            }
        }
    }
}

impl std::error::Error for ReadModelError {}

/// A cached balance value with metadata.
///
/// Contains the balance along with versioning and timing information
/// for cache invalidation and consistency checking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedBalance {
    /// The cached balance amount.
    pub balance: Money,
    /// The event version this balance was computed from.
    ///
    /// Used to detect stale cache entries when events are added.
    pub version: u64,
    /// When this value was cached.
    ///
    /// Used for TTL-based expiration.
    pub cached_at: Timestamp,
}

impl CachedBalance {
    /// Creates a new `CachedBalance`.
    ///
    /// # Arguments
    ///
    /// * `balance` - The balance to cache
    /// * `version` - The event version this balance was computed from
    /// * `cached_at` - When this value is being cached
    #[must_use]
    pub const fn new(balance: Money, version: u64, cached_at: Timestamp) -> Self {
        Self {
            balance,
            version,
            cached_at,
        }
    }

    /// Creates a new `CachedBalance` with the current timestamp.
    ///
    /// # Arguments
    ///
    /// * `balance` - The balance to cache
    /// * `version` - The event version this balance was computed from
    #[must_use]
    pub fn now(balance: Money, version: u64) -> Self {
        Self {
            balance,
            version,
            cached_at: Timestamp::now(),
        }
    }

    /// Checks if this cached value has expired.
    ///
    /// # Arguments
    ///
    /// * `time_to_live_seconds` - The TTL in seconds
    ///
    /// # Returns
    ///
    /// `true` if the cached value is older than the TTL
    #[must_use]
    pub fn is_expired(&self, time_to_live_seconds: i64) -> bool {
        let now = Timestamp::now();
        let age = self.cached_at.duration_until(&now);
        age.num_seconds() > time_to_live_seconds
    }

    /// Checks if this cached value is stale compared to a given version.
    ///
    /// # Arguments
    ///
    /// * `current_version` - The current version to compare against
    ///
    /// # Returns
    ///
    /// `true` if the cached version is less than the current version
    #[must_use]
    pub const fn is_stale(&self, current_version: u64) -> bool {
        self.version < current_version
    }
}

/// Trait for read model cache implementations.
///
/// Defines the interface for caching balance queries on the read side.
/// Implementations must be thread-safe (`Send + Sync`).
///
/// # Operations
///
/// - `get_balance`: Retrieve a cached balance
/// - `set_balance`: Store a balance in the cache
/// - `invalidate`: Remove a cached balance
///
/// # Example Implementation
///
/// ```rust,ignore
/// use bank::infrastructure::{ReadModelCache, ReadModelError, CachedBalance};
/// use lambars::effect::AsyncIO;
///
/// struct InMemoryCache {
///     // ... storage
/// }
///
/// impl ReadModelCache for InMemoryCache {
///     fn get_balance(
///         &self,
///         account_id: &AccountId,
///     ) -> AsyncIO<Result<Option<CachedBalance>, ReadModelError>> {
///         // Implementation...
///     }
///     // ... other methods
/// }
/// ```
pub trait ReadModelCache: Send + Sync {
    /// Retrieves a cached balance for an account.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The account to look up
    ///
    /// # Returns
    ///
    /// An `AsyncIO` that, when run:
    /// - Returns `Ok(Some(CachedBalance))` if found
    /// - Returns `Ok(None)` if not cached
    /// - Returns `Err(ReadModelError)` if the cache operation fails
    fn get_balance(
        &self,
        account_id: &AccountId,
    ) -> AsyncIO<Result<Option<CachedBalance>, ReadModelError>>;

    /// Stores a balance in the cache.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The account to cache the balance for
    /// * `balance` - The balance to cache
    /// * `version` - The event version this balance was computed from
    ///
    /// # Returns
    ///
    /// An `AsyncIO` that, when run:
    /// - Returns `Ok(())` if the balance was successfully cached
    /// - Returns `Err(ReadModelError)` if the cache operation fails
    fn set_balance(
        &self,
        account_id: &AccountId,
        balance: &Money,
        version: u64,
    ) -> AsyncIO<Result<(), ReadModelError>>;

    /// Removes a cached balance.
    ///
    /// This is typically called when an account is modified to ensure
    /// subsequent queries fetch fresh data.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The account to invalidate
    ///
    /// # Returns
    ///
    /// An `AsyncIO` that, when run:
    /// - Returns `Ok(())` if the cache entry was invalidated (or didn't exist)
    /// - Returns `Err(ReadModelError)` if the cache operation fails
    fn invalidate(&self, account_id: &AccountId) -> AsyncIO<Result<(), ReadModelError>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::Currency;
    use rstest::rstest;

    // =========================================================================
    // ReadModelError Tests
    // =========================================================================

    #[rstest]
    fn read_model_error_cache_error_display() {
        let error = ReadModelError::CacheError("connection refused".to_string());

        assert_eq!(format!("{error}"), "Cache error: connection refused");
    }

    #[rstest]
    fn read_model_error_serialization_error_display() {
        let error = ReadModelError::SerializationError("invalid format".to_string());

        assert_eq!(format!("{error}"), "Serialization error: invalid format");
    }

    #[rstest]
    fn read_model_error_equality() {
        let error1 = ReadModelError::CacheError("error1".to_string());
        let error2 = ReadModelError::CacheError("error1".to_string());
        let error3 = ReadModelError::CacheError("error2".to_string());

        assert_eq!(error1, error2);
        assert_ne!(error1, error3);
    }

    #[rstest]
    fn read_model_error_clone() {
        let original = ReadModelError::SerializationError("test".to_string());
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[rstest]
    fn read_model_error_debug() {
        let error = ReadModelError::CacheError("timeout".to_string());
        let debug_str = format!("{error:?}");

        assert!(debug_str.contains("CacheError"));
        assert!(debug_str.contains("timeout"));
    }

    #[rstest]
    fn read_model_error_is_error_trait() {
        fn assert_error<E: std::error::Error>(_: &E) {}

        let error = ReadModelError::CacheError("test".to_string());
        assert_error(&error);
    }

    // =========================================================================
    // CachedBalance Tests
    // =========================================================================

    #[rstest]
    fn cached_balance_new() {
        let balance = Money::new(10000, Currency::JPY);
        let timestamp = Timestamp::now();
        let cached = CachedBalance::new(balance.clone(), 5, timestamp);

        assert_eq!(cached.balance, balance);
        assert_eq!(cached.version, 5);
        assert_eq!(cached.cached_at, timestamp);
    }

    #[rstest]
    fn cached_balance_now() {
        let balance = Money::new(5000, Currency::JPY);
        let before = Timestamp::now();
        let cached = CachedBalance::now(balance.clone(), 10);
        let after = Timestamp::now();

        assert_eq!(cached.balance, balance);
        assert_eq!(cached.version, 10);
        // cached_at should be between before and after
        assert!(!cached.cached_at.is_before(&before));
        assert!(!cached.cached_at.is_after(&after));
    }

    #[rstest]
    fn cached_balance_is_stale_when_version_is_less() {
        let balance = Money::new(1000, Currency::JPY);
        let cached = CachedBalance::now(balance, 5);

        assert!(cached.is_stale(6));
        assert!(cached.is_stale(10));
        assert!(cached.is_stale(100));
    }

    #[rstest]
    fn cached_balance_is_not_stale_when_version_is_equal_or_greater() {
        let balance = Money::new(1000, Currency::JPY);
        let cached = CachedBalance::now(balance, 5);

        assert!(!cached.is_stale(5));
        assert!(!cached.is_stale(4));
        assert!(!cached.is_stale(0));
    }

    #[rstest]
    fn cached_balance_is_expired_when_old() {
        let balance = Money::new(1000, Currency::JPY);
        // Create a timestamp in the past
        let past_timestamp =
            Timestamp::from_unix_seconds(Timestamp::now().unix_seconds() - 1000).unwrap();
        let cached = CachedBalance::new(balance, 1, past_timestamp);

        assert!(cached.is_expired(500)); // TTL of 500 seconds
        assert!(cached.is_expired(100)); // TTL of 100 seconds
    }

    #[rstest]
    fn cached_balance_is_not_expired_when_recent() {
        let balance = Money::new(1000, Currency::JPY);
        let cached = CachedBalance::now(balance, 1);

        // With a TTL of 300 seconds, a just-created entry should not be expired
        assert!(!cached.is_expired(300));
        assert!(!cached.is_expired(1));
    }

    #[rstest]
    fn cached_balance_clone() {
        let balance = Money::new(2000, Currency::JPY);
        let original = CachedBalance::now(balance, 3);
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[rstest]
    fn cached_balance_debug() {
        let balance = Money::new(3000, Currency::JPY);
        let cached = CachedBalance::now(balance, 7);
        let debug_str = format!("{cached:?}");

        assert!(debug_str.contains("CachedBalance"));
        assert!(debug_str.contains("balance"));
        assert!(debug_str.contains("version"));
        assert!(debug_str.contains('7'));
    }

    #[rstest]
    fn cached_balance_equality() {
        let balance1 = Money::new(1000, Currency::JPY);
        let balance2 = Money::new(1000, Currency::JPY);
        let balance3 = Money::new(2000, Currency::JPY);
        let timestamp = Timestamp::now();

        let cached1 = CachedBalance::new(balance1.clone(), 1, timestamp);
        let cached2 = CachedBalance::new(balance2, 1, timestamp);
        let cached3 = CachedBalance::new(balance3, 1, timestamp);
        let cached4 = CachedBalance::new(balance1, 2, timestamp);

        assert_eq!(cached1, cached2);
        assert_ne!(cached1, cached3);
        assert_ne!(cached1, cached4);
    }

    // =========================================================================
    // CachedBalance Edge Cases
    // =========================================================================

    #[rstest]
    fn cached_balance_with_zero_version() {
        let balance = Money::new(0, Currency::JPY);
        let cached = CachedBalance::now(balance, 0);

        assert_eq!(cached.version, 0);
        assert!(!cached.is_stale(0));
        assert!(cached.is_stale(1));
    }

    #[rstest]
    fn cached_balance_with_max_version() {
        let balance = Money::new(1000, Currency::JPY);
        let cached = CachedBalance::now(balance, u64::MAX);

        assert_eq!(cached.version, u64::MAX);
        assert!(!cached.is_stale(u64::MAX));
        assert!(!cached.is_stale(0));
    }

    #[rstest]
    fn cached_balance_is_expired_boundary() {
        let balance = Money::new(1000, Currency::JPY);
        // Create a timestamp exactly at the TTL boundary
        let ttl_seconds = 300i64;
        let past_timestamp =
            Timestamp::from_unix_seconds(Timestamp::now().unix_seconds() - ttl_seconds).unwrap();
        let cached = CachedBalance::new(balance, 1, past_timestamp);

        // At exactly TTL seconds, it should not be expired (> not >=)
        // Due to potential timing variations, we just verify the logic works
        // The boundary behavior is: expired when age > ttl
        assert!(!cached.is_expired(ttl_seconds + 1));
    }
}
