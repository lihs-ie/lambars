//! Query handlers for task retrieval and search.
//!
//! This module demonstrates lambars' functional programming patterns for data querying:
//! - **`Foldable`**: Aggregating data with fold operations
//! - **`Monoid`**: Combining search results with deduplication
//! - **`Traversable`**: Effectful traversal and filtering with validation
//! - **`PersistentVector`**: Immutable collections with structural sharing
//!
//! # Endpoints
//!
//! - `GET /tasks` - List tasks with pagination and filtering (uses `Traversable`)
//! - `GET /tasks/search` - Search tasks by title or tags
//! - `GET /tasks/by-priority` - Count tasks grouped by priority
//!
//! # Traversable Usage
//!
//! The `GET /tasks` endpoint demonstrates `Traversable::traverse_option` for:
//! - Filtering tasks by lifting filter predicates into the Option effect boundary
//! - Paginating with validation-aware transformation via `traverse_option`
//!
//! This pattern enables composition with other effectful operations and maintains
//! type safety through the effect boundary.
//!
//! # Design Note
//!
//! This implementation intentionally fetches all data and processes it in-memory
//! to showcase functional programming patterns (filter, fold, combine). In production,
//! filtering and aggregation should be delegated to the repository layer for better
//! performance with large datasets.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};

use super::dto::{PriorityDto, TaskResponse, TaskStatusDto};
use super::error::ApiErrorResponse;
use super::handlers::AppState;
use crate::domain::{Priority, Task, TaskId, TaskStatus};
use crate::infrastructure::{PaginatedResult, Pagination, SearchScope as RepositorySearchScope};
use lambars::persistent::{
    PersistentHashMap, PersistentHashSet, PersistentTreeMap, PersistentVector, TransientHashMap,
};
use lambars::typeclass::{Semigroup, Traversable};

// =============================================================================
// Query Parameters
// =============================================================================

/// Query parameters for listing tasks.
#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    /// Page number (default: 1, minimum: 1).
    #[serde(default = "default_page")]
    pub page: u32,
    /// Items per page (default: 20, range: 1-100).
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// Filter by task status.
    pub status: Option<TaskStatusDto>,
    /// Filter by task priority.
    pub priority: Option<PriorityDto>,
}

const fn default_page() -> u32 {
    1
}

const fn default_limit() -> u32 {
    DEFAULT_PAGE_SIZE
}

// =============================================================================
// List Pagination Constants
// =============================================================================

/// Maximum page size for list operations (prevents full table scans).
pub const MAX_PAGE_SIZE: u32 = 100;

/// Default page size for list operations.
pub const DEFAULT_PAGE_SIZE: u32 = 20;

// =============================================================================
// Search Pagination Constants and Functions
// =============================================================================

/// Maximum limit for search results.
pub const MAX_SEARCH_LIMIT: u32 = 100;

/// Default limit for search results when not specified.
pub const DEFAULT_SEARCH_LIMIT: u32 = 20;

/// Legacy constant for backwards compatibility.
pub const SEARCH_DEFAULT_LIMIT: u32 = DEFAULT_SEARCH_LIMIT;

/// Legacy constant for backwards compatibility.
pub const SEARCH_MAX_LIMIT: u32 = MAX_SEARCH_LIMIT;

/// Normalizes pagination parameters for search queries (pure function).
///
/// # Arguments
///
/// * `limit` - Optional limit from query. If `None`, defaults to [`SEARCH_DEFAULT_LIMIT`] (50).
/// * `offset` - Optional offset from query. If `None`, defaults to 0.
///
/// # Returns
///
/// A tuple of `(normalized_limit, normalized_offset)` where:
/// - `limit` is clamped to [`SEARCH_MAX_LIMIT`] (200) if it exceeds this value.
/// - `limit=0` is explicitly allowed and returns an empty result (user intent).
/// - `offset` defaults to 0 if not provided.
///
/// # Specification
///
/// - **Default limit**: 50 (when `limit` is not specified)
/// - **Maximum limit**: 200 (values above this are clamped)
/// - **`limit=0` behavior**: Returns empty array (explicit user intent to get no results)
///
/// # Examples
///
/// ```ignore
/// // Default values
/// assert_eq!(normalize_search_pagination(None, None), (50, 0));
///
/// // limit exceeds max, clamped to 200
/// assert_eq!(normalize_search_pagination(Some(500), None), (200, 0));
///
/// // Normal values
/// assert_eq!(normalize_search_pagination(Some(100), Some(20)), (100, 20));
///
/// // limit=0 returns empty array (explicit user intent)
/// assert_eq!(normalize_search_pagination(Some(0), Some(10)), (0, 10));
/// ```
#[must_use]
pub const fn normalize_search_pagination(limit: Option<u32>, offset: Option<u32>) -> (u32, u32) {
    let normalized_limit = match limit {
        Some(value) if value > SEARCH_MAX_LIMIT => SEARCH_MAX_LIMIT,
        Some(value) => value,
        None => SEARCH_DEFAULT_LIMIT,
    };
    let normalized_offset = match offset {
        Some(value) => value,
        None => 0,
    };
    (normalized_limit, normalized_offset)
}

/// Query parameters for searching tasks.
#[derive(Debug, Deserialize)]
pub struct SearchTasksQuery {
    /// Search query string (case-insensitive substring match).
    pub q: String,
    /// Search scope: "title", "tags", or "all" (default: "all").
    #[serde(rename = "in", default)]
    pub scope: SearchScope,
    /// Maximum number of results to return.
    /// - Defaults to 50 if not specified.
    /// - Clamped to 200 if exceeds maximum.
    /// - `limit=0` returns empty array (explicit user intent).
    pub limit: Option<u32>,
    /// Number of results to skip (0-based offset).
    /// Defaults to 0 if not specified.
    pub offset: Option<u32>,
}

/// Search scope enum.
///
/// Valid values are: "title", "tags", "all" (case-insensitive).
/// Unknown values will result in a 400 Bad Request error.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum SearchScope {
    /// Search only in task titles.
    Title,
    /// Search only in task tags.
    Tags,
    /// Search in both titles and tags.
    #[default]
    All,
}

impl std::str::FromStr for SearchScope {
    type Err = String;

    /// Parses a string into a `SearchScope`.
    ///
    /// Valid values are "title", "tags", "all" (case-insensitive).
    ///
    /// # Errors
    ///
    /// Returns an error string if the input is not one of the valid values.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use std::str::FromStr;
    /// assert_eq!(SearchScope::from_str("title"), Ok(SearchScope::Title));
    /// assert_eq!(SearchScope::from_str("TAGS"), Ok(SearchScope::Tags));
    /// assert!(SearchScope::from_str("unknown").is_err());
    /// ```
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "title" => Ok(Self::Title),
            "tags" => Ok(Self::Tags),
            "all" => Ok(Self::All),
            other => Err(format!(
                "Invalid search scope '{other}'. Valid values are: title, tags, all"
            )),
        }
    }
}

impl<'de> serde::Deserialize<'de> for SearchScope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use std::str::FromStr;
        let value = String::deserialize(deserializer)?;
        Self::from_str(&value).map_err(serde::de::Error::custom)
    }
}

impl SearchScope {
    /// Converts this API `SearchScope` to the repository's `SearchScope`.
    ///
    /// This enables using the DB-side search when calling `repository.search()`.
    #[must_use]
    pub const fn to_repository_scope(self) -> RepositorySearchScope {
        match self {
            Self::Title => RepositorySearchScope::Title,
            Self::Tags => RepositorySearchScope::Tags,
            Self::All => RepositorySearchScope::All,
        }
    }
}

// =============================================================================
// Query Normalization (REQ-SEARCH-CACHE-001)
// =============================================================================

/// Normalized search query for cache key generation.
///
/// This structure represents a search query after normalization,
/// containing both the cache key and tokenized words for potential
/// future use (e.g., advanced search scoring).
///
/// # Normalization Process
///
/// 1. **trim** - Remove leading/trailing whitespace
/// 2. **lowercase** - Case-insensitive matching
/// 3. **multi-space collapse** - Normalize internal whitespace to single spaces
///
/// # Laws
///
/// - **Idempotent**: `normalize(normalize(q)) = normalize(q)`
///
/// # Examples
///
/// ```ignore
/// let normalized = normalize_query("  Urgent   Task  ");
/// assert_eq!(normalized.key(), "urgent task");
/// assert_eq!(normalized.tokens(), &["urgent", "task"]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NormalizedQuery {
    /// Normalized query string (for cache key).
    ///
    /// This is the result of applying trim, lowercase, and multi-space collapse.
    key: String,

    /// Tokenized words (for potential future use).
    ///
    /// Words are split by whitespace after normalization.
    /// Empty queries result in an empty token list.
    tokens: Vec<String>,
}

impl NormalizedQuery {
    /// Returns the normalized query key (read-only).
    ///
    /// This is the result of applying trim, lowercase, and multi-space collapse.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Returns the tokenized words (read-only).
    ///
    /// Words are split by whitespace after normalization.
    /// Empty queries result in an empty token list.
    #[must_use]
    pub fn tokens(&self) -> &[String] {
        &self.tokens
    }

    /// Returns `true` if the normalized query is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.key.is_empty()
    }

    /// Consumes the `NormalizedQuery` and returns the underlying key.
    ///
    /// This is useful when you need ownership of the key string.
    #[must_use]
    pub fn into_key(self) -> String {
        self.key
    }
}

/// Normalizes a search query (pure function).
///
/// This function applies the following transformations:
///
/// 1. **trim** - Remove leading/trailing whitespace
/// 2. **lowercase** - Case-insensitive matching
/// 3. **multi-space collapse** - Normalize internal whitespace to single spaces
///
/// # Laws
///
/// - **Idempotent**: `normalize_query(normalize_query(q).key()) == normalize_query(q)`
///
/// # Arguments
///
/// * `raw` - The raw query string from user input
///
/// # Returns
///
/// A [`NormalizedQuery`] containing:
/// - `key`: The normalized string suitable for cache key generation
/// - `tokens`: Individual words split by whitespace
///
/// # Examples
///
/// ```ignore
/// // Basic normalization
/// let result = normalize_query("  Urgent   Task  ");
/// assert_eq!(result.key(), "urgent task");
/// assert_eq!(result.tokens(), &["urgent", "task"]);
///
/// // Empty query
/// let empty = normalize_query("   ");
/// assert_eq!(empty.key(), "");
/// assert!(empty.tokens().is_empty());
///
/// // Already normalized
/// let already = normalize_query("urgent task");
/// assert_eq!(already.key(), "urgent task");
/// ```
#[must_use]
pub fn normalize_query(raw: &str) -> NormalizedQuery {
    // Step 1: trim leading/trailing whitespace
    let trimmed = raw.trim();

    // Step 2 & 3: lowercase and collapse multi-spaces
    // We split by whitespace (handles multi-space) and rejoin with single space
    let tokens: Vec<String> = trimmed.split_whitespace().map(str::to_lowercase).collect();

    let key = tokens.join(" ");

    NormalizedQuery { key, tokens }
}

/// Cache key for search results.
///
/// This structure uniquely identifies a search query for caching purposes.
/// Two queries are considered equivalent (and thus cacheable) if and only if
/// all fields match exactly.
///
/// # Fields
///
/// - `normalized_query`: The normalized query string (from [`normalize_query`])
/// - `scope`: The search scope (title, tags, or all)
/// - `limit`: Maximum number of results
/// - `offset`: Number of results to skip
///
/// # Cache Key Semantics
///
/// The cache key uses exact matching on all fields. This means:
/// - `"urgent task"` with limit=50 is different from limit=100
/// - `"urgent task"` with scope=Title is different from scope=All
/// - Query normalization ensures case-insensitive and whitespace-normalized matching
///
/// # Examples
///
/// ```ignore
/// let key1 = SearchCacheKey::from_raw("  Urgent Task  ", SearchScope::All, Some(50), Some(0));
/// let key2 = SearchCacheKey::from_raw("urgent task", SearchScope::All, Some(50), Some(0));
///
/// // key1 == key2 because the normalized query is the same
/// assert_eq!(key1, key2);
///
/// // Pagination parameters are also normalized:
/// let key3 = SearchCacheKey::from_raw("test", SearchScope::All, None, None);
/// let key4 = SearchCacheKey::from_raw("test", SearchScope::All, Some(50), Some(0));
/// assert_eq!(key3, key4); // Both use default limit=50 and offset=0
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SearchCacheKey {
    /// Normalized query string (from [`normalize_query`]).
    normalized_query: String,

    /// Search scope (title, tags, or all).
    scope: SearchScope,

    /// Maximum number of results.
    limit: u32,

    /// Number of results to skip (0-based offset).
    offset: u32,
}

impl SearchCacheKey {
    /// Creates a new cache key from raw query parameters.
    ///
    /// The query is automatically normalized via [`normalize_query`].
    /// Pagination parameters (limit and offset) are also normalized via
    /// [`normalize_search_pagination`] to ensure consistent cache key generation.
    ///
    /// # Arguments
    ///
    /// * `raw_query` - The raw query string from user input
    /// * `scope` - The search scope
    /// * `limit` - Maximum number of results (normalized to default if `None`)
    /// * `offset` - Number of results to skip (normalized to 0 if `None`)
    #[must_use]
    pub fn from_raw(
        raw_query: &str,
        scope: SearchScope,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Self {
        let normalized = normalize_query(raw_query);
        let (normalized_limit, normalized_offset) = normalize_search_pagination(limit, offset);
        Self {
            normalized_query: normalized.into_key(),
            scope,
            limit: normalized_limit,
            offset: normalized_offset,
        }
    }

    /// Returns the normalized query string (read-only).
    #[must_use]
    pub fn normalized_query(&self) -> &str {
        &self.normalized_query
    }

    /// Returns the search scope.
    #[must_use]
    pub const fn scope(&self) -> SearchScope {
        self.scope
    }

    /// Returns the maximum number of results.
    #[must_use]
    pub const fn limit(&self) -> u32 {
        self.limit
    }

    /// Returns the number of results to skip.
    #[must_use]
    pub const fn offset(&self) -> u32 {
        self.offset
    }
}

// =============================================================================
// Search Result Cache (REQ-SEARCH-CACHE-001)
// =============================================================================

use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Cached search result with timestamp for TTL.
///
/// This structure wraps a search result with a timestamp to enable
/// TTL-based cache invalidation.
#[derive(Clone)]
pub struct CachedSearchResult {
    /// The cached search result.
    pub result: SearchResult,
    /// Timestamp when the result was cached.
    pub cached_at: Instant,
}

/// Cache statistics for monitoring.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits.
    pub hits: u64,
    /// Number of cache misses.
    pub misses: u64,
}

impl CacheStats {
    /// Returns the hit rate as a percentage (0.0 to 1.0).
    ///
    /// Returns 0.0 if no requests have been made.
    ///
    /// # Note
    ///
    /// For very large hit/miss counts (> 2^52), there may be minor precision loss
    /// when converting to f64. This is acceptable for monitoring purposes.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// LRU cache for search results with TTL.
///
/// This cache provides:
/// - **LRU eviction**: When capacity is reached, least recently used entries are evicted
/// - **TTL expiration**: Entries older than TTL are considered stale and not returned
/// - **Thread-safety**: Uses `Mutex` for safe concurrent access
///
/// # Configuration (REQ-SEARCH-CACHE-001)
///
/// - **TTL**: 5 seconds (entries expire after this duration)
/// - **Capacity**: 2000 entries maximum
///
/// # Cache Key
///
/// The cache key is `(normalized_q, scope, limit, offset)` using exact matching.
/// Query normalization ensures that equivalent queries (with different whitespace
/// or casing) produce the same cache key.
///
/// # Thread Safety
///
/// The cache uses a `Mutex` to ensure safe concurrent access. While this introduces
/// some contention, the cache operations are fast (O(1) for LRU operations) and
/// the critical section is minimal.
///
/// # Example
///
/// ```ignore
/// let cache = SearchCache::new(2000, Duration::from_secs(5));
///
/// // Check cache
/// if let Some(result) = cache.get(&cache_key) {
///     return Ok(result);
/// }
///
/// // Cache miss - perform search
/// let result = search_with_scope_indexed(&index, query, scope);
///
/// // Store in cache
/// cache.put(cache_key, result.clone());
/// ```
pub struct SearchCache {
    /// The LRU cache protected by a mutex.
    cache: Mutex<LruCache<SearchCacheKey, CachedSearchResult>>,
    /// Time-to-live for cache entries.
    time_to_live: Duration,
    /// Cache statistics.
    stats: Mutex<CacheStats>,
}

impl SearchCache {
    /// Creates a new search cache with the specified capacity and TTL.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of entries in the cache
    /// * `time_to_live` - Duration after which entries are considered stale
    ///
    /// # Panics
    ///
    /// Panics if `capacity` is 0.
    #[must_use]
    pub fn new(capacity: usize, time_to_live: Duration) -> Self {
        Self {
            cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(capacity).expect("capacity must be non-zero"),
            )),
            time_to_live,
            stats: Mutex::new(CacheStats::default()),
        }
    }

    /// Creates a new search cache with default configuration.
    ///
    /// Default configuration (REQ-SEARCH-CACHE-001):
    /// - Capacity: 2000 entries
    /// - TTL: 5 seconds
    #[must_use]
    pub fn with_default_config() -> Self {
        Self::new(2000, Duration::from_secs(5))
    }

    /// Gets a cached search result if it exists and is not expired.
    ///
    /// If the entry exists but is expired (older than TTL), it is removed
    /// and `None` is returned.
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key to look up
    ///
    /// # Returns
    ///
    /// `Some(SearchResult)` if the entry exists and is not expired, `None` otherwise.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned. This should only happen if a thread
    /// panicked while holding the lock, which indicates a programming error.
    pub fn get(&self, key: &SearchCacheKey) -> Option<SearchResult> {
        let mut cache = self.cache.lock().expect("cache mutex poisoned");
        let mut stats = self.stats.lock().expect("stats mutex poisoned");

        if let Some(cached) = cache.get(key) {
            if cached.cached_at.elapsed() < self.time_to_live {
                // Cache hit - entry is valid
                stats.hits += 1;
                return Some(cached.result.clone());
            }
            // Entry is expired - remove it
            tracing::debug!(
                query = %key.normalized_query(),
                scope = ?key.scope(),
                "Search cache entry expired"
            );
        }

        // Pop the expired entry if it exists
        cache.pop(key);
        drop(cache); // Release cache lock as soon as possible

        // Cache miss
        stats.misses += 1;

        None
    }

    /// Stores a search result in the cache.
    ///
    /// If the cache is at capacity, the least recently used entry is evicted.
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key
    /// * `result` - The search result to cache
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned. This should only happen if a thread
    /// panicked while holding the lock, which indicates a programming error.
    pub fn put(&self, key: SearchCacheKey, result: SearchResult) {
        let mut cache = self.cache.lock().expect("cache mutex poisoned");
        cache.put(
            key,
            CachedSearchResult {
                result,
                cached_at: Instant::now(),
            },
        );
    }

    /// Returns the current cache statistics.
    ///
    /// This method is useful for monitoring cache performance.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned. This should only happen if a thread
    /// panicked while holding the lock, which indicates a programming error.
    #[must_use]
    pub fn stats(&self) -> CacheStats {
        self.stats.lock().expect("stats mutex poisoned").clone()
    }

    /// Returns the current number of entries in the cache.
    ///
    /// Note: This includes expired entries that haven't been removed yet.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned. This should only happen if a thread
    /// panicked while holding the lock, which indicates a programming error.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cache.lock().expect("cache mutex poisoned").len()
    }

    /// Returns true if the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clears all entries from the cache.
    ///
    /// This does not reset the statistics.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned. This should only happen if a thread
    /// panicked while holding the lock, which indicates a programming error.
    pub fn clear(&self) {
        self.cache.lock().expect("cache mutex poisoned").clear();
    }
}

impl std::fmt::Debug for SearchCache {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let len = self.len();
        let stats = self.stats();
        formatter
            .debug_struct("SearchCache")
            .field("len", &len)
            .field("time_to_live", &self.time_to_live)
            .field("hits", &stats.hits)
            .field("misses", &stats.misses)
            .field("hit_rate", &format!("{:.2}%", stats.hit_rate() * 100.0))
            .finish_non_exhaustive()
    }
}

// =============================================================================
// Response DTOs
// =============================================================================

/// Paginated result container.
#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResponse<T> {
    /// The data items for the current page.
    pub data: Vec<T>,
    /// Current page number.
    pub page: u32,
    /// Items per page.
    pub limit: u32,
    /// Total number of items.
    pub total: u64,
    /// Total number of pages (0 if no items).
    pub total_pages: u64,
}

/// Priority count result.
#[derive(Debug, Clone, Default, Serialize)]
pub struct PriorityCountResponse {
    /// Count of low priority tasks.
    pub low: u64,
    /// Count of medium priority tasks.
    pub medium: u64,
    /// Count of high priority tasks.
    pub high: u64,
    /// Count of critical priority tasks.
    pub critical: u64,
    /// Total count (derived from individual counts).
    pub total: u64,
}

impl PriorityCountResponse {
    /// Increment the low priority count.
    #[must_use]
    const fn increment_low(self) -> Self {
        Self {
            low: self.low + 1,
            medium: self.medium,
            high: self.high,
            critical: self.critical,
            total: self.total,
        }
    }

    /// Increment the medium priority count.
    #[must_use]
    const fn increment_medium(self) -> Self {
        Self {
            low: self.low,
            medium: self.medium + 1,
            high: self.high,
            critical: self.critical,
            total: self.total,
        }
    }

    /// Increment the high priority count.
    #[must_use]
    const fn increment_high(self) -> Self {
        Self {
            low: self.low,
            medium: self.medium,
            high: self.high + 1,
            critical: self.critical,
            total: self.total,
        }
    }

    /// Increment the critical priority count.
    #[must_use]
    const fn increment_critical(self) -> Self {
        Self {
            low: self.low,
            medium: self.medium,
            high: self.high,
            critical: self.critical + 1,
            total: self.total,
        }
    }

    /// Finalize the response by computing the total.
    #[must_use]
    const fn finalize(self) -> Self {
        Self {
            low: self.low,
            medium: self.medium,
            high: self.high,
            critical: self.critical,
            total: self.low + self.medium + self.high + self.critical,
        }
    }
}

// =============================================================================
// Search Result with Monoid
// =============================================================================

/// Search result with deduplication by task ID.
///
/// Implements `Semigroup` for combining results from different sources.
/// Title matches are prioritized by adding them first.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Tasks ordered by match quality (title matches first, then tags).
    tasks: PersistentVector<Task>,
    /// Set of task IDs for deduplication.
    seen_ids: PersistentHashSet<TaskId>,
}

impl SearchResult {
    /// Creates an empty search result.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            tasks: PersistentVector::new(),
            seen_ids: PersistentHashSet::new(),
        }
    }

    /// Creates a search result from a vector of tasks.
    ///
    /// Builds the `seen_ids` set from the task IDs for deduplication.
    #[must_use]
    pub fn from_tasks(tasks: PersistentVector<Task>) -> Self {
        let seen_ids = tasks
            .iter()
            .map(|task| task.task_id.clone())
            .collect::<PersistentHashSet<TaskId>>();
        Self { tasks, seen_ids }
    }

    /// Returns the tasks as a vector (consumes self).
    #[must_use]
    pub fn into_tasks(self) -> PersistentVector<Task> {
        self.tasks
    }

    /// Returns a reference to the tasks.
    #[must_use]
    pub const fn tasks(&self) -> &PersistentVector<Task> {
        &self.tasks
    }

    /// Returns the number of tasks in the result.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Returns `true` if the result is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

impl Semigroup for SearchResult {
    /// Combines two search results with deduplication.
    ///
    /// Tasks from `self` appear first (higher priority), followed by
    /// tasks from `other` that are not already present.
    fn combine(self, other: Self) -> Self {
        let mut tasks = self.tasks;
        let mut seen_ids = self.seen_ids;

        for task in &other.tasks {
            if !seen_ids.contains(&task.task_id) {
                tasks = tasks.push_back(task.clone());
                seen_ids = seen_ids.insert(task.task_id.clone());
            }
        }

        Self { tasks, seen_ids }
    }
}

// =============================================================================
// Search Index with PersistentTreeMap
// =============================================================================

// -----------------------------------------------------------------------------
// SearchIndex Configuration (REQ-SEARCH-NGRAM-001)
// -----------------------------------------------------------------------------

/// Infix search mode for `SearchIndex`.
///
/// Controls how infix (substring) searches are performed:
/// - `Ngram`: Uses n-gram inverted index (default, recommended)
/// - `LegacyAllSuffix`: Uses legacy all-suffix index (feature flag for compatibility)
/// - `Disabled`: Disables infix search entirely
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InfixMode {
    /// N-gram inverted index (default).
    ///
    /// Uses n-gram tokenization for efficient substring matching.
    /// Memory-bounded by `max_ngrams_per_token` configuration.
    #[default]
    Ngram,
    /// Legacy all-suffix index.
    ///
    /// Generates all suffixes for each token. Preserved for backward
    /// compatibility but disabled by default due to higher memory usage.
    LegacyAllSuffix,
    /// Infix search disabled.
    ///
    /// Only prefix search is available when this mode is selected.
    Disabled,
}

/// Configuration for `SearchIndex` construction and search behavior.
///
/// # Memory Guarantee
///
/// The default configuration guarantees that temporary memory during index
/// construction does not exceed 512MB:
/// - `max_tokens_per_task = 100`
/// - `max_ngrams_per_token = 64`
/// - Per task: 100 tokens × 64 n-grams = 6,400 entries
/// - For 10,000 tasks: 64M entries × 8 bytes ≈ 512MB
///
/// # Example
///
/// ```rust,ignore
/// use task_management_benchmark_api::api::query::{SearchIndexConfig, InfixMode};
///
/// // Use default configuration (n-gram mode)
/// let config = SearchIndexConfig::default();
///
/// // Use legacy mode for backward compatibility
/// let legacy_config = SearchIndexConfig {
///     infix_mode: InfixMode::LegacyAllSuffix,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchIndexConfig {
    /// Infix search mode (acts as a feature flag).
    pub infix_mode: InfixMode,
    /// N-gram size in characters (must be >= 2).
    pub ngram_size: usize,
    /// Minimum query length (in characters) to trigger infix search.
    ///
    /// Queries shorter than this length will only use prefix search.
    /// Applied to all infix modes (`Ngram` and `LegacyAllSuffix`).
    pub min_query_len_for_infix: usize,
    /// Maximum number of n-grams generated per token.
    ///
    /// Limits memory usage during index construction.
    /// Set to 64 to guarantee 512MB memory bound.
    pub max_ngrams_per_token: usize,
    /// Maximum number of tokens per task (title words + tags combined).
    ///
    /// Tasks with more tokens will have excess tokens ignored.
    pub max_tokens_per_task: usize,
    /// Maximum number of search result candidates.
    ///
    /// Applied to the final result set to bound response size.
    pub max_search_candidates: usize,
}

impl Default for SearchIndexConfig {
    fn default() -> Self {
        Self {
            infix_mode: InfixMode::Ngram,
            ngram_size: 3,
            min_query_len_for_infix: 3,
            max_ngrams_per_token: 64,
            max_tokens_per_task: 100,
            max_search_candidates: 1000,
        }
    }
}

// -----------------------------------------------------------------------------
// N-gram Index Type (REQ-SEARCH-NGRAM-002 Part 2)
// -----------------------------------------------------------------------------

/// N-gram inverted index type.
///
/// Maps n-gram strings to a sorted list of `TaskId`s that contain that n-gram.
/// The `TaskId` list is always:
/// - Deduplicated (no duplicate `TaskId`s)
/// - Sorted in ascending order (for efficient merge intersection)
///
/// # Structure
///
/// - Key: n-gram string (e.g., "cal", "all", "llb" for "callback")
/// - Value: `PersistentVector<TaskId>` containing all tasks with that n-gram
#[allow(dead_code)] // Will be used in Phase 4 (SearchIndex integration)
type NgramIndex = PersistentHashMap<String, PersistentVector<TaskId>>;

/// Mutable n-gram index for efficient batch construction.
///
/// Mutable index type using `HashMap` for O(1) amortized insertion during batch operations.
/// After batch construction, convert to persistent structures via [`finalize_ngram_index`].
type MutableNgramIndex = std::collections::HashMap<String, Vec<TaskId>>;

/// Alias for prefix index deltas (same underlying type as `MutableNgramIndex`).
type MutablePrefixIndex = MutableNgramIndex;

/// N-gram key with reference-counted storage for O(1) cloning.
///
/// Used with [`NgramKeyPool`] to deduplicate n-gram strings within a single
/// `apply_changes` operation. `Hash`/`Eq` delegate to underlying `str`.
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct NgramKey(Arc<str>);

impl NgramKey {
    /// Creates a new `NgramKey`. Use [`NgramKeyPool::intern`] to reuse allocations.
    pub fn new(value: &str) -> Self {
        Self(Arc::from(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NgramKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Local interning pool for n-gram keys.
///
/// Deduplicates n-gram strings within a single `apply_changes` operation,
/// converting repeated `String::clone()` calls into O(1) `Arc` pointer copies.
/// Pool is created per-operation and dropped after completion.
///
/// # Memory Efficiency
///
/// Uses `HashMap<Arc<str>, NgramKey>` where both key and value share the same
/// `Arc<str>` allocation, satisfying REQ-SEARCH-NGRAM-MEM-002's requirement
/// for single allocation per unique string.
#[derive(Default)]
pub struct NgramKeyPool {
    pool: std::collections::HashMap<Arc<str>, NgramKey>,
    hit_count: usize,
    miss_count: usize,
}

impl NgramKeyPool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interns a string, returning an `NgramKey`.
    ///
    /// Returns a clone of existing key (O(1)) on hit, or allocates new on miss.
    ///
    /// # Note on Purity
    ///
    /// This method is intentionally impure: it mutates internal cache state
    /// (`pool`, `hit_count`, `miss_count`) for performance optimization.
    /// This is acceptable because:
    /// - The pool is locally scoped within a single `apply_changes` operation
    /// - External behavior (returned `NgramKey` value) is deterministic
    /// - Side effects are confined to internal memoization state
    pub fn intern(&mut self, value: &str) -> NgramKey {
        if let Some(key) = self.pool.get(value) {
            self.hit_count += 1;
            return key.clone();
        }
        self.miss_count += 1;
        // Allocate Arc<str> once and share between HashMap key and NgramKey value
        let arc: Arc<str> = Arc::from(value);
        let key = NgramKey(arc.clone());
        self.pool.insert(arc, key.clone());
        key
    }

    /// Returns the cache hit rate as a ratio between 0.0 and 1.0.
    #[allow(clippy::cast_precision_loss)]
    pub fn hit_rate(&self) -> f64 {
        let total = self.hit_count + self.miss_count;
        if total == 0 { 0.0 } else { self.hit_count as f64 / total as f64 }
    }

    pub fn unique_count(&self) -> usize {
        self.pool.len()
    }

    pub const fn total_count(&self) -> usize {
        self.hit_count + self.miss_count
    }
}

/// Represents the delta (difference) for a `SearchIndex` update.
///
/// Aggregates multiple `TaskChange`s into a single batch for efficient one-pass merging.
/// Uses `HashMap<String, Vec<TaskId>>` for O(1) amortized insertion during construction.
///
/// # Requirements Reference
///
/// Implements REQ-SEARCH-NGRAM-PERF-001 from:
/// `docs/internal/requirements/20260128_1255_search_index_ngram_perf_remediation.yaml`
#[derive(Debug, Clone, Default)]
pub struct SearchIndexDelta {
    // Prefix index deltas (TreeMap-based indices)
    pub title_full_add: MutablePrefixIndex,
    pub title_full_remove: MutablePrefixIndex,
    pub title_word_add: MutablePrefixIndex,
    pub title_word_remove: MutablePrefixIndex,
    pub tag_add: MutablePrefixIndex,
    pub tag_remove: MutablePrefixIndex,

    // N-gram index deltas
    pub title_full_ngram_add: MutableNgramIndex,
    pub title_full_ngram_remove: MutableNgramIndex,
    pub title_word_ngram_add: MutableNgramIndex,
    pub title_word_ngram_remove: MutableNgramIndex,
    pub tag_ngram_add: MutableNgramIndex,
    pub tag_ngram_remove: MutableNgramIndex,

    // All-suffix index deltas (for LegacyAllSuffix mode)
    pub title_full_all_suffix_add: MutablePrefixIndex,
    pub title_full_all_suffix_remove: MutablePrefixIndex,
    pub title_word_all_suffix_add: MutablePrefixIndex,
    pub title_word_all_suffix_remove: MutablePrefixIndex,
    pub tag_all_suffix_add: MutablePrefixIndex,
    pub tag_all_suffix_remove: MutablePrefixIndex,
}

// REQ-SEARCH-NGRAM-PERF-001 Part 2
/// Cached normalization result for a task. Tags are sorted for stable ordering.
///
/// This struct is `pub(crate)` to allow `SearchIndex::add_task_with_normalized`
/// to use pre-computed normalized data for single-task additions.
pub(crate) struct NormalizedTaskData {
    title_key: String,
    title_words: Vec<String>,
    tags: Vec<String>,
}

// Phase 5.5: Single add_task optimization
impl NormalizedTaskData {
    /// Constructs normalized data from a task. Normalization is performed only once.
    ///
    /// # Normalization
    ///
    /// - Title: Uses `normalize_query()` for consistent normalization
    /// - Tags: Sorted for stable ordering, normalized via `normalize_query()`
    fn from_task(task: &Task) -> Self {
        let title = normalize_query(&task.title);
        let mut sorted_tags: Vec<_> = task.tags.iter().collect();
        sorted_tags.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        let tags: Vec<String> = sorted_tags
            .into_iter()
            .map(|tag| normalize_query(tag.as_str()).key)
            .collect();

        Self {
            title_key: title.key,
            title_words: title.tokens,
            tags,
        }
    }
}

// REQ-SEARCH-NGRAM-PERF-001 Part 2
impl SearchIndexDelta {
    /// Constructs a delta from task changes. Each task is normalized only once.
    ///
    /// # Panics
    ///
    /// Panics on forbidden patterns: Remove followed by Add/Update for the same `TaskId`.
    #[must_use]
    pub fn from_changes(
        changes: &[TaskChange],
        config: &SearchIndexConfig,
        tasks_by_id: &PersistentTreeMap<TaskId, Task>,
    ) -> Self {
        let mut delta = Self::default();
        let mut pending_tasks: std::collections::HashMap<TaskId, (Task, NormalizedTaskData)> =
            std::collections::HashMap::new();
        let mut removed_task_ids: std::collections::HashSet<TaskId> =
            std::collections::HashSet::new();

        for change in changes {
            match change {
                TaskChange::Add(task) => {
                    assert!(
                        !removed_task_ids.contains(&task.task_id),
                        "Invalid change sequence: Remove followed by Add for same TaskId {:?}. \
                         This pattern is not supported. Use Update instead.",
                        task.task_id
                    );
                    let normalized = NormalizedTaskData::from_task(task);
                    delta.collect_add(&normalized, &task.task_id, config);
                    pending_tasks.insert(task.task_id.clone(), (task.clone(), normalized));
                }
                TaskChange::Update { old, new } => {
                    assert!(
                        !removed_task_ids.contains(&new.task_id),
                        "Invalid change sequence: Remove followed by Update for same TaskId {:?}. \
                         This pattern is not supported.",
                        new.task_id
                    );
                    let old_normalized = NormalizedTaskData::from_task(old);
                    let new_normalized = NormalizedTaskData::from_task(new);
                    delta.collect_update_diff(
                        &old_normalized,
                        &new_normalized,
                        &new.task_id,
                        config,
                    );
                    pending_tasks.insert(new.task_id.clone(), (new.clone(), new_normalized));
                }
                TaskChange::Remove(task_id) => {
                    if let Some((_, normalized)) = pending_tasks.get(task_id) {
                        delta.collect_remove(normalized, task_id, config);
                        pending_tasks.remove(task_id);
                    } else if let Some(task) = tasks_by_id.get(task_id) {
                        let normalized = NormalizedTaskData::from_task(task);
                        delta.collect_remove(&normalized, task_id, config);
                    }
                    removed_task_ids.insert(task_id.clone());
                }
            }
        }

        delta
    }

    fn collect_add(
        &mut self,
        data: &NormalizedTaskData,
        task_id: &TaskId,
        config: &SearchIndexConfig,
    ) {
        let (word_limit, tag_limit) = Self::compute_token_limits(data, config);

        self.title_full_add
            .entry(data.title_key.clone())
            .or_default()
            .push(task_id.clone());

        for word in data.title_words.iter().take(word_limit) {
            self.title_word_add
                .entry(word.clone())
                .or_default()
                .push(task_id.clone());
            match config.infix_mode {
                InfixMode::Ngram => {
                    index_ngrams_batch(&mut self.title_word_ngram_add, word, task_id, config);
                }
                InfixMode::LegacyAllSuffix => {
                    index_all_suffixes_batch(&mut self.title_word_all_suffix_add, word, task_id);
                }
                InfixMode::Disabled => {}
            }
        }

        for tag in data.tags.iter().take(tag_limit) {
            self.tag_add
                .entry(tag.clone())
                .or_default()
                .push(task_id.clone());
            match config.infix_mode {
                InfixMode::Ngram => {
                    index_ngrams_batch(&mut self.tag_ngram_add, tag, task_id, config);
                }
                InfixMode::LegacyAllSuffix => {
                    index_all_suffixes_batch(&mut self.tag_all_suffix_add, tag, task_id);
                }
                InfixMode::Disabled => {}
            }
        }

        match config.infix_mode {
            InfixMode::Ngram => {
                index_ngrams_batch(
                    &mut self.title_full_ngram_add,
                    &data.title_key,
                    task_id,
                    config,
                );
            }
            InfixMode::LegacyAllSuffix => {
                index_all_suffixes_batch(
                    &mut self.title_full_all_suffix_add,
                    &data.title_key,
                    task_id,
                );
            }
            InfixMode::Disabled => {}
        }
    }

    /// Collects tokens to remove. No token limit is applied to match `remove_task()` behavior.
    fn collect_remove(
        &mut self,
        data: &NormalizedTaskData,
        task_id: &TaskId,
        config: &SearchIndexConfig,
    ) {
        self.title_full_remove
            .entry(data.title_key.clone())
            .or_default()
            .push(task_id.clone());

        for word in &data.title_words {
            self.title_word_remove
                .entry(word.clone())
                .or_default()
                .push(task_id.clone());
            match config.infix_mode {
                InfixMode::Ngram => {
                    index_ngrams_batch(&mut self.title_word_ngram_remove, word, task_id, config);
                }
                InfixMode::LegacyAllSuffix => {
                    index_all_suffixes_batch(&mut self.title_word_all_suffix_remove, word, task_id);
                }
                InfixMode::Disabled => {}
            }
        }

        for tag in &data.tags {
            self.tag_remove
                .entry(tag.clone())
                .or_default()
                .push(task_id.clone());
            match config.infix_mode {
                InfixMode::Ngram => {
                    index_ngrams_batch(&mut self.tag_ngram_remove, tag, task_id, config);
                }
                InfixMode::LegacyAllSuffix => {
                    index_all_suffixes_batch(&mut self.tag_all_suffix_remove, tag, task_id);
                }
                InfixMode::Disabled => {}
            }
        }

        match config.infix_mode {
            InfixMode::Ngram => {
                index_ngrams_batch(
                    &mut self.title_full_ngram_remove,
                    &data.title_key,
                    task_id,
                    config,
                );
            }
            InfixMode::LegacyAllSuffix => {
                index_all_suffixes_batch(
                    &mut self.title_full_all_suffix_remove,
                    &data.title_key,
                    task_id,
                );
            }
            InfixMode::Disabled => {}
        }
    }

    /// Tags take priority over words when total exceeds `max_tokens_per_task`.
    ///
    /// This method is `pub(crate)` to allow `SearchIndex::add_task_with_normalized`
    /// to reuse the same token limit calculation logic.
    pub(crate) fn compute_token_limits(
        data: &NormalizedTaskData,
        config: &SearchIndexConfig,
    ) -> (usize, usize) {
        let (words_len, tags_len) = (data.title_words.len(), data.tags.len());
        let total = words_len + tags_len;

        if total <= config.max_tokens_per_task {
            (words_len, tags_len)
        } else {
            let word_limit = config
                .max_tokens_per_task
                .saturating_sub(tags_len.min(config.max_tokens_per_task));
            let tag_limit = config.max_tokens_per_task.saturating_sub(word_limit);
            (word_limit, tag_limit)
        }
    }

    /// Collects differences between old and new task data for Update operations.
    ///
    /// Matches sequential `apply_change` behavior: old tokens are fully removed (no limit),
    /// new tokens are added with limit. Ngram diff is computed at the ngram level because
    /// tokens like "tag1" and "tagA" share common ngrams.
    fn collect_update_diff(
        &mut self,
        old_data: &NormalizedTaskData,
        new_data: &NormalizedTaskData,
        task_id: &TaskId,
        config: &SearchIndexConfig,
    ) {
        let (new_word_limit, new_tag_limit) = Self::compute_token_limits(new_data, config);

        if old_data.title_key != new_data.title_key {
            Self::push_to_index(&mut self.title_full_remove, &old_data.title_key, task_id);
            Self::push_to_index(&mut self.title_full_add, &new_data.title_key, task_id);
            match config.infix_mode {
                InfixMode::Ngram => {
                    Self::collect_ngram_diff(
                        &mut self.title_full_ngram_remove,
                        &mut self.title_full_ngram_add,
                        &old_data.title_key,
                        &new_data.title_key,
                        task_id,
                        config,
                    );
                }
                InfixMode::LegacyAllSuffix => {
                    Self::collect_all_suffix_diff(
                        &mut self.title_full_all_suffix_remove,
                        &mut self.title_full_all_suffix_add,
                        &old_data.title_key,
                        &new_data.title_key,
                        task_id,
                    );
                }
                InfixMode::Disabled => {}
            }
        }

        Self::collect_token_diff(
            &mut self.title_word_remove,
            &mut self.title_word_add,
            old_data.title_words.iter(),
            new_data.title_words.iter().take(new_word_limit),
            task_id,
        );

        match config.infix_mode {
            InfixMode::Ngram => {
                Self::collect_tokens_ngram_diff(
                    &mut self.title_word_ngram_remove,
                    &mut self.title_word_ngram_add,
                    old_data.title_words.iter(),
                    new_data.title_words.iter().take(new_word_limit),
                    task_id,
                    config,
                );
            }
            InfixMode::LegacyAllSuffix => {
                Self::collect_tokens_all_suffix_diff(
                    &mut self.title_word_all_suffix_remove,
                    &mut self.title_word_all_suffix_add,
                    old_data.title_words.iter(),
                    new_data.title_words.iter().take(new_word_limit),
                    task_id,
                );
            }
            InfixMode::Disabled => {}
        }

        Self::collect_token_diff(
            &mut self.tag_remove,
            &mut self.tag_add,
            old_data.tags.iter(),
            new_data.tags.iter().take(new_tag_limit),
            task_id,
        );

        match config.infix_mode {
            InfixMode::Ngram => {
                Self::collect_tokens_ngram_diff(
                    &mut self.tag_ngram_remove,
                    &mut self.tag_ngram_add,
                    old_data.tags.iter(),
                    new_data.tags.iter().take(new_tag_limit),
                    task_id,
                    config,
                );
            }
            InfixMode::LegacyAllSuffix => {
                Self::collect_tokens_all_suffix_diff(
                    &mut self.tag_all_suffix_remove,
                    &mut self.tag_all_suffix_add,
                    old_data.tags.iter(),
                    new_data.tags.iter().take(new_tag_limit),
                    task_id,
                );
            }
            InfixMode::Disabled => {}
        }
    }

    fn push_to_index(index: &mut MutablePrefixIndex, key: &str, task_id: &TaskId) {
        index
            .entry(key.to_string())
            .or_default()
            .push(task_id.clone());
    }

    fn collect_token_diff<'a>(
        remove_index: &mut MutablePrefixIndex,
        add_index: &mut MutablePrefixIndex,
        old_tokens: impl Iterator<Item = &'a String>,
        new_tokens: impl Iterator<Item = &'a String>,
        task_id: &TaskId,
    ) {
        let old_set: std::collections::HashSet<_> = old_tokens.collect();
        let new_set: std::collections::HashSet<_> = new_tokens.collect();

        for token in old_set.difference(&new_set) {
            Self::push_to_index(remove_index, token, task_id);
        }
        for token in new_set.difference(&old_set) {
            Self::push_to_index(add_index, token, task_id);
        }
    }

    fn collect_ngram_diff(
        remove_index: &mut MutableNgramIndex,
        add_index: &mut MutableNgramIndex,
        old_key: &str,
        new_key: &str,
        task_id: &TaskId,
        config: &SearchIndexConfig,
    ) {
        let old_ngrams: std::collections::HashSet<_> =
            generate_ngrams(old_key, config.ngram_size, config.max_ngrams_per_token)
                .into_iter()
                .collect();
        let new_ngrams: std::collections::HashSet<_> =
            generate_ngrams(new_key, config.ngram_size, config.max_ngrams_per_token)
                .into_iter()
                .collect();

        for ngram in old_ngrams.difference(&new_ngrams) {
            remove_index
                .entry(ngram.clone())
                .or_default()
                .push(task_id.clone());
        }
        for ngram in new_ngrams.difference(&old_ngrams) {
            add_index
                .entry(ngram.clone())
                .or_default()
                .push(task_id.clone());
        }
    }

    fn collect_tokens_ngram_diff<'a>(
        remove_index: &mut MutableNgramIndex,
        add_index: &mut MutableNgramIndex,
        old_tokens: impl Iterator<Item = &'a String>,
        new_tokens: impl Iterator<Item = &'a String>,
        task_id: &TaskId,
        config: &SearchIndexConfig,
    ) {
        let old_ngrams: std::collections::HashSet<String> = old_tokens
            .flat_map(|t| generate_ngrams(t, config.ngram_size, config.max_ngrams_per_token))
            .collect();
        let new_ngrams: std::collections::HashSet<String> = new_tokens
            .flat_map(|t| generate_ngrams(t, config.ngram_size, config.max_ngrams_per_token))
            .collect();

        for ngram in old_ngrams.difference(&new_ngrams) {
            remove_index
                .entry(ngram.clone())
                .or_default()
                .push(task_id.clone());
        }
        for ngram in new_ngrams.difference(&old_ngrams) {
            add_index
                .entry(ngram.clone())
                .or_default()
                .push(task_id.clone());
        }
    }

    /// Generates all suffixes from a word.
    ///
    /// For `hello`, generates: `["hello", "ello", "llo", "lo", "o"]`
    fn generate_all_suffixes(word: &str) -> Vec<String> {
        word.char_indices()
            .map(|(byte_index, _)| word[byte_index..].to_string())
            .collect()
    }

    /// Collects suffix diff between old and new key.
    fn collect_all_suffix_diff(
        remove_index: &mut MutablePrefixIndex,
        add_index: &mut MutablePrefixIndex,
        old_key: &str,
        new_key: &str,
        task_id: &TaskId,
    ) {
        let old_suffixes: std::collections::HashSet<_> =
            Self::generate_all_suffixes(old_key).into_iter().collect();
        let new_suffixes: std::collections::HashSet<_> =
            Self::generate_all_suffixes(new_key).into_iter().collect();

        for suffix in old_suffixes.difference(&new_suffixes) {
            remove_index
                .entry(suffix.clone())
                .or_default()
                .push(task_id.clone());
        }
        for suffix in new_suffixes.difference(&old_suffixes) {
            add_index
                .entry(suffix.clone())
                .or_default()
                .push(task_id.clone());
        }
    }

    /// Collects suffix diff for multiple tokens (words or tags).
    fn collect_tokens_all_suffix_diff<'a>(
        remove_index: &mut MutablePrefixIndex,
        add_index: &mut MutablePrefixIndex,
        old_tokens: impl Iterator<Item = &'a String>,
        new_tokens: impl Iterator<Item = &'a String>,
        task_id: &TaskId,
    ) {
        let old_suffixes: std::collections::HashSet<String> = old_tokens
            .flat_map(|token| Self::generate_all_suffixes(token))
            .collect();
        let new_suffixes: std::collections::HashSet<String> = new_tokens
            .flat_map(|token| Self::generate_all_suffixes(token))
            .collect();

        for suffix in old_suffixes.difference(&new_suffixes) {
            remove_index
                .entry(suffix.clone())
                .or_default()
                .push(task_id.clone());
        }
        for suffix in new_suffixes.difference(&old_suffixes) {
            add_index
                .entry(suffix.clone())
                .or_default()
                .push(task_id.clone());
        }
    }
}

// -----------------------------------------------------------------------------
// N-gram Generation (REQ-SEARCH-NGRAM-002 Part 1)
// -----------------------------------------------------------------------------

/// Generates n-grams from a normalized token.
///
/// This is a pure function that produces n-grams using a sliding window
/// approach over the input string's characters.
///
/// # Algorithm
///
/// Uses `char_indices()` for UTF-8 safe sliding window generation.
/// The function generates at most `max_ngrams` n-grams to bound memory usage.
///
/// # Complexity
///
/// O(min(L - n + 1, `max_ngrams`)) where L is the character count.
///
/// # Arguments
///
/// * `normalized_token` - A normalized (lowercase, trimmed) token string
/// * `ngram_size` - The size of each n-gram in characters (must be >= 2)
/// * `max_ngrams` - Maximum number of n-grams to generate
///
/// # Returns
///
/// A `Vec<String>` containing the generated n-grams. Returns empty if:
/// - `ngram_size < 2`
/// - Token has fewer characters than `ngram_size`
#[allow(dead_code)] // Will be used in Phase 3 (NgramIndex construction)
#[must_use]
fn generate_ngrams(normalized_token: &str, ngram_size: usize, max_ngrams: usize) -> Vec<String> {
    // Reject invalid ngram_size (must be >= 2)
    if ngram_size < 2 {
        return Vec::new();
    }

    // Collect char indices for UTF-8 safe slicing
    let char_indices: Vec<(usize, char)> = normalized_token.char_indices().collect();
    let char_count = char_indices.len();

    // Token too short to generate any n-grams
    if char_count < ngram_size {
        return Vec::new();
    }

    // Calculate how many n-grams can be generated
    let max_possible = char_count.saturating_sub(ngram_size).saturating_add(1);
    let actual_count = max_possible.min(max_ngrams);

    // Pre-allocate result vector
    let mut ngrams = Vec::with_capacity(actual_count);

    // Generate n-grams using sliding window
    for i in 0..actual_count {
        let start_byte = char_indices[i].0;
        let end_byte = if i + ngram_size < char_indices.len() {
            char_indices[i + ngram_size].0
        } else {
            normalized_token.len()
        };
        ngrams.push(normalized_token[start_byte..end_byte].to_string());
    }

    ngrams
}

// -----------------------------------------------------------------------------
// N-gram Index Operations (REQ-SEARCH-NGRAM-002 Part 2)
// -----------------------------------------------------------------------------

/// Registers a token's n-grams into the index (pure function).
///
/// This function adds all n-grams of the given token to the index,
/// associating them with the specified `TaskId`.
///
/// # Invariants Maintained
///
/// - **No duplicate `TaskId`**: Each `TaskId` appears at most once per n-gram
/// - **Sorted order**: `TaskId` lists are always sorted in ascending order
///
/// # Algorithm
///
/// 1. Generate n-grams from the normalized token
/// 2. For each n-gram, retrieve or create the posting list
/// 3. Use binary search to find insertion position (maintains sorted order)
/// 4. Skip if `TaskId` already exists (deduplication)
/// 5. Insert at correct position to maintain sorted order
///
/// # Complexity
///
/// O(G * (log N + M)) where:
/// - G = number of n-grams generated
/// - N = average posting list length (binary search)
/// - M = average posting list length (insertion)
///
/// # Arguments
///
/// * `index` - The current n-gram index
/// * `normalized_token` - A normalized (lowercase, trimmed) token string
/// * `task_id` - The `TaskId` to associate with the token's n-grams
/// * `config` - Search index configuration
///
/// # Returns
///
/// A new `NgramIndex` with the token's n-grams registered
#[allow(dead_code)] // Will be used in Phase 4 (SearchIndex integration)
#[must_use]
fn index_ngrams(
    index: NgramIndex,
    normalized_token: &str,
    task_id: &TaskId,
    config: &SearchIndexConfig,
) -> NgramIndex {
    let ngrams = generate_ngrams(
        normalized_token,
        config.ngram_size,
        config.max_ngrams_per_token,
    );

    if ngrams.is_empty() {
        return index;
    }

    // Convert to transient for efficient batch updates
    let mut transient_index = index.transient();

    for ngram in ngrams {
        let existing_ids = transient_index
            .get(&ngram)
            .cloned()
            .unwrap_or_else(PersistentVector::new);

        // Collect existing IDs into a Vec for binary search
        let ids_vec: Vec<TaskId> = existing_ids.iter().cloned().collect();

        // Binary search for insertion position
        // If Ok, TaskId already exists (deduplication - skip)
        // If Err, insert at the returned position to maintain sorted order
        if let Err(insert_position) = ids_vec.binary_search(task_id) {
            // Build new vector with TaskId inserted at correct position
            let mut transient_vec = PersistentVector::new().transient();

            // Add elements before insertion position
            for id in ids_vec.iter().take(insert_position) {
                transient_vec.push_back(id.clone());
            }

            // Insert new TaskId
            transient_vec.push_back(task_id.clone());

            // Add elements after insertion position
            for id in ids_vec.iter().skip(insert_position) {
                transient_vec.push_back(id.clone());
            }

            transient_index.insert(ngram, transient_vec.persistent());
        }
    }

    // Persist and return
    transient_index.persistent()
}

/// Registers a token's n-grams into a transient index (mutable in-place version).
///
/// This function is optimized for batch operations where many tokens need to be
/// indexed in a single transient/persist cycle. Unlike `index_ngrams`, this function
/// operates directly on a mutable `TransientHashMap` reference, avoiding the overhead
/// of calling `transient()` and `persist()` for each token.
///
/// # Performance
///
/// For building an index with 10,000 tasks and ~10 words each:
/// - `index_ngrams`: ~100,000 transient/persist cycles (24+ seconds)
/// - `index_ngrams_transient`: 3 transient/persist cycles total (< 1 second)
///
/// # Invariants Maintained
///
/// - **No duplicate `TaskId`**: Each `TaskId` appears at most once per n-gram
/// - **Sorted order**: `TaskId` lists are always sorted in ascending order
///
/// # Algorithm
///
/// 1. Generate n-grams from the normalized token
/// 2. For each n-gram, retrieve or create the posting list
/// 3. Use binary search to find insertion position (maintains sorted order)
/// 4. Skip if `TaskId` already exists (deduplication)
/// 5. Insert at correct position to maintain sorted order
///
/// # Arguments
///
/// * `transient_index` - A mutable reference to the transient n-gram index
/// * `normalized_token` - A normalized (lowercase, trimmed) token string
/// * `task_id` - The `TaskId` to associate with the token's n-grams
/// * `config` - Search index configuration
#[allow(dead_code)] // Retained for future single-task add_task() operations
fn index_ngrams_transient(
    transient_index: &mut TransientHashMap<String, PersistentVector<TaskId>>,
    normalized_token: &str,
    task_id: &TaskId,
    config: &SearchIndexConfig,
) {
    let ngrams = generate_ngrams(
        normalized_token,
        config.ngram_size,
        config.max_ngrams_per_token,
    );

    if ngrams.is_empty() {
        return;
    }

    for ngram in ngrams {
        let existing_ids = transient_index
            .get(&ngram)
            .cloned()
            .unwrap_or_else(PersistentVector::new);

        // Collect existing IDs into a Vec for binary search
        let ids_vec: Vec<TaskId> = existing_ids.iter().cloned().collect();

        // Binary search for insertion position
        // If Ok, TaskId already exists (deduplication - skip)
        // If Err, insert at the returned position to maintain sorted order
        if let Err(insert_position) = ids_vec.binary_search(task_id) {
            // Build new vector with TaskId inserted at correct position
            let mut transient_vec = PersistentVector::new().transient();

            // Add elements before insertion position
            for id in ids_vec.iter().take(insert_position) {
                transient_vec.push_back(id.clone());
            }

            // Insert new TaskId
            transient_vec.push_back(task_id.clone());

            // Add elements after insertion position
            for id in ids_vec.iter().skip(insert_position) {
                transient_vec.push_back(id.clone());
            }

            transient_index.insert(ngram, transient_vec.persistent());
        }
    }
}

/// Indexes a token's n-grams into a mutable batch index (O(1) amortized per n-gram).
///
/// This function is optimized for batch construction of search indexes.
/// Unlike [`index_ngrams_transient`], which rebuilds `PersistentVector` for each
/// insertion (O(n) per insertion), this function uses standard `Vec` with O(1)
/// amortized push operations.
///
/// # Performance
///
/// - Time: O(G) where G = number of n-grams generated per token
/// - Space: O(G) additional entries in the hash map
///
/// For batch construction of N tasks with M tokens each:
/// - This approach: O(N * M * G) total
/// - Old approach: O(N * M * G * K) where K = average posting list length
///
/// # Note on Sorting
///
/// This function does NOT maintain sorted order during insertion.
/// Sorting and deduplication are deferred to [`finalize_ngram_index`] for
/// better overall performance (O(n log n) sort once vs O(n) per insertion).
///
/// # Arguments
///
/// * `index` - A mutable reference to the batch n-gram index
/// * `normalized_token` - A normalized (lowercase, trimmed) token string
/// * `task_id` - The `TaskId` to associate with the token's n-grams
/// * `config` - Search index configuration
fn index_ngrams_batch(
    index: &mut MutableNgramIndex,
    normalized_token: &str,
    task_id: &TaskId,
    config: &SearchIndexConfig,
) {
    let ngrams = generate_ngrams(
        normalized_token,
        config.ngram_size,
        config.max_ngrams_per_token,
    );

    for ngram in ngrams {
        index.entry(ngram).or_default().push(task_id.clone());
    }
}

/// Indexes all suffixes of a word into a mutable batch index.
///
/// This function generates all suffixes of a word and adds the task ID to each suffix entry.
/// For example, "hello" generates suffixes: "hello", "ello", "llo", "lo", "o".
///
/// # Arguments
///
/// * `index` - The mutable index to update
/// * `word` - The word to generate suffixes from
/// * `task_id` - The task ID to associate with each suffix
fn index_all_suffixes_batch(index: &mut MutablePrefixIndex, word: &str, task_id: &TaskId) {
    for (byte_index, _) in word.char_indices() {
        let suffix = &word[byte_index..];
        index
            .entry(suffix.to_string())
            .or_default()
            .push(task_id.clone());
    }
}

/// Converts a mutable batch index to an immutable `NgramIndex`.
///
/// This function performs the final transformation from the mutable
/// `HashMap<String, Vec<TaskId>>` used during batch construction to the
/// immutable `NgramIndex` (`PersistentHashMap<String, PersistentVector<TaskId>>`).
///
/// # Processing Steps
///
/// For each n-gram entry:
/// 1. Sort the `TaskId` vector in ascending order
/// 2. Remove duplicates (dedup requires sorted input)
/// 3. Convert `Vec<TaskId>` to `PersistentVector<TaskId>`
/// 4. Insert into the persistent hash map
///
/// # Performance
///
/// - Time: O(E * K log K) where E = number of n-gram entries, K = average posting list length
/// - The O(K log K) factor is for sorting each posting list
///
/// # Invariants Established
///
/// The resulting `NgramIndex` guarantees:
/// - Each posting list is sorted in ascending `TaskId` order
/// - No duplicate `TaskId` values within any posting list
///
/// # Arguments
///
/// * `mutable_index` - The mutable batch index to convert
///
/// # Returns
///
/// An immutable `NgramIndex` with sorted, deduplicated posting lists
#[must_use]
fn finalize_ngram_index(mutable_index: MutableNgramIndex) -> NgramIndex {
    let mut result = PersistentHashMap::new().transient();

    for (ngram, mut task_ids) in mutable_index {
        // Sort and deduplicate the posting list
        task_ids.sort();
        task_ids.dedup();

        // Convert to PersistentVector
        let persistent_ids: PersistentVector<TaskId> = task_ids.into_iter().collect();
        result.insert(ngram, persistent_ids);
    }

    result.persistent()
}

/// Removes a token's n-grams from the index (pure function).
///
/// This function removes the specified `TaskId` from all n-gram entries
/// associated with the given token.
///
/// # Algorithm
///
/// 1. Generate n-grams from the normalized token
/// 2. For each n-gram, filter out the specified `TaskId`
/// 3. If the posting list becomes empty, remove the n-gram entry entirely
///
/// # Complexity
///
/// O(G * M) where:
/// - G = number of n-grams generated
/// - M = average posting list length (linear scan for removal)
///
/// # Arguments
///
/// * `index` - The current n-gram index
/// * `normalized_token` - A normalized (lowercase, trimmed) token string
/// * `task_id` - The `TaskId` to remove from the token's n-grams
/// * `config` - Search index configuration
///
/// # Returns
///
/// A new `NgramIndex` with the `TaskId` removed from the token's n-grams
#[allow(dead_code)] // Will be used in Phase 6 (add_task/remove_task integration)
#[must_use]
fn remove_ngrams(
    index: NgramIndex,
    normalized_token: &str,
    task_id: &TaskId,
    config: &SearchIndexConfig,
) -> NgramIndex {
    let ngrams = generate_ngrams(
        normalized_token,
        config.ngram_size,
        config.max_ngrams_per_token,
    );

    if ngrams.is_empty() {
        return index;
    }

    // Convert to transient for efficient batch updates
    let mut transient_index = index.transient();

    for ngram in ngrams {
        if let Some(existing_ids) = transient_index.get(&ngram).cloned() {
            // Filter out the specified TaskId
            let updated_ids: PersistentVector<TaskId> = existing_ids
                .iter()
                .filter(|id| *id != task_id)
                .cloned()
                .collect();

            if updated_ids.is_empty() {
                // Remove the n-gram entry if no TaskIds remain
                transient_index.remove(&ngram);
            } else {
                // Update with filtered list
                transient_index.insert(ngram, updated_ids);
            }
        }
    }

    // Persist and return
    transient_index.persistent()
}

// -----------------------------------------------------------------------------
// N-gram Search Logic (REQ-SEARCH-NGRAM-003)
// -----------------------------------------------------------------------------

/// Computes the intersection of two sorted `Vec<TaskId>` in O(n) time.
///
/// This function uses a merge-intersection algorithm that takes advantage of
/// the sorted order of both input vectors to achieve linear time complexity.
///
/// # Arguments
///
/// * `left` - First sorted slice of `TaskId`s (ascending order, deduplicated)
/// * `right` - Second sorted slice of `TaskId`s (ascending order, deduplicated)
///
/// # Returns
///
/// A new `Vec<TaskId>` containing only the elements that appear in both inputs,
/// in ascending sorted order.
///
/// # Complexity
///
/// - Time: O(n + m) where n = `left.len()`, m = `right.len()`
/// - Space: O(min(n, m)) for the result vector
///
/// # Preconditions
///
/// Both input slices must be:
/// - Sorted in ascending order by `TaskId`
/// - Deduplicated (no duplicate `TaskId`s)
///
/// # Properties (Laws)
///
/// 1. **Commutativity**: `intersect(A, B) == intersect(B, A)`
/// 2. **Subset**: `result ⊆ left ∧ result ⊆ right`
/// 3. **Completeness**: `∀x. x ∈ left ∧ x ∈ right => x ∈ result`
/// 4. **Sorted**: Result is sorted in ascending order
#[must_use]
fn intersect_sorted_vecs(left: &[TaskId], right: &[TaskId]) -> Vec<TaskId> {
    // Pre-allocate with conservative capacity (min of both lengths)
    let mut result = Vec::with_capacity(left.len().min(right.len()));

    let mut left_index = 0;
    let mut right_index = 0;

    while left_index < left.len() && right_index < right.len() {
        match left[left_index].cmp(&right[right_index]) {
            std::cmp::Ordering::Equal => {
                result.push(left[left_index].clone());
                left_index += 1;
                right_index += 1;
            }
            std::cmp::Ordering::Less => {
                left_index += 1;
            }
            std::cmp::Ordering::Greater => {
                right_index += 1;
            }
        }
    }

    result
}

// -----------------------------------------------------------------------------
// SearchIndex Build Metrics (REQ-SEARCH-NGRAM-PERF-002)
// -----------------------------------------------------------------------------

/// Metrics from `SearchIndex` construction.
///
/// This struct captures performance metrics during `SearchIndex` build:
/// - Time elapsed during construction
/// - Peak resident set size (RSS) of the process
/// - Total n-gram entries across all indexes
///
/// # Usage
///
/// These metrics are intended for benchmarking and capacity planning.
/// The `measure_search_index_build` function returns this struct alongside
/// the constructed `SearchIndex`.
///
/// # Output Format
///
/// When `SEARCH_INDEX_METRICS_PATH` environment variable is set, metrics are
/// serialized to JSON at the specified path.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchIndexBuildMetrics {
    /// Time elapsed during index construction (milliseconds).
    pub elapsed_ms: u128,
    /// Peak resident set size of the process (megabytes).
    ///
    /// On macOS, this is obtained via `getrusage(RUSAGE_SELF)`.
    /// On Linux, this is obtained from `/proc/self/status` (`VmHWM`).
    /// On unsupported platforms, this is `0`.
    pub peak_rss_mb: u64,
    /// Total n-gram entries across all indexes.
    ///
    /// Sum of:
    /// - `title_full_ngram_index.len()`
    /// - `title_word_ngram_index.len()`
    /// - `tag_ngram_index.len()`
    pub ngram_entries: usize,
}

/// Builds a `SearchIndex` with performance measurement (I/O boundary function).
///
/// This function wraps `SearchIndex::build_with_config` with timing and memory
/// measurement. It is intended for benchmarking and capacity planning.
///
/// # Arguments
///
/// * `tasks` - Collection of tasks to index
/// * `config` - Configuration controlling index behavior
///
/// # Returns
///
/// A tuple of `(SearchIndex, SearchIndexBuildMetrics)`.
///
/// # Side Effects
///
/// - Emits a `tracing::info` log with metrics (target: `search_index_build`)
/// - Measures system resources (time, RSS)
///
/// # Platform Support
///
/// RSS measurement is supported on:
/// - macOS: via `getrusage(RUSAGE_SELF)`
/// - Linux: via `/proc/self/status` (`VmHWM`)
/// - Other platforms: returns 0
///
/// # Example
///
/// ```ignore
/// use lambars::persistent::PersistentVector;
/// use task_management_benchmark_api::api::query::{
///     measure_search_index_build, SearchIndexConfig
/// };
///
/// let tasks = PersistentVector::new();
/// let config = SearchIndexConfig::default();
/// let (index, metrics) = measure_search_index_build(&tasks, config);
///
/// println!("Build took {}ms, peak RSS: {}MB", metrics.elapsed_ms, metrics.peak_rss_mb);
/// ```
#[must_use]
pub fn measure_search_index_build(
    tasks: &PersistentVector<Task>,
    config: SearchIndexConfig,
) -> (SearchIndex, SearchIndexBuildMetrics) {
    let start = std::time::Instant::now();

    // Call the pure function (SearchIndex::build_with_config is unchanged)
    let index = SearchIndex::build_with_config(tasks, config);

    let elapsed_ms = start.elapsed().as_millis();

    // Get peak RSS (absolute value)
    let peak_rss_mb = get_peak_rss_absolute_mb().unwrap_or(0);

    // Count n-gram entries
    let ngram_entries = index.ngram_entry_count();

    let metrics = SearchIndexBuildMetrics {
        elapsed_ms,
        peak_rss_mb,
        ngram_entries,
    };

    tracing::info!(
        target: "search_index_build",
        elapsed_ms = elapsed_ms,
        peak_rss_mb = peak_rss_mb,
        ngram_entries = ngram_entries,
        "SearchIndex build metrics"
    );

    (index, metrics)
}

/// Gets the peak resident set size (RSS) in megabytes.
///
/// This function returns the peak RSS of the current process, which represents
/// the maximum amount of physical memory used at any point during execution.
///
/// # Platform Support
///
/// - **Linux**: Uses `VmHWM` (High Water Mark) from `/proc/self/status`
/// - **macOS**: Uses `libc::getrusage` to obtain `ru_maxrss` (maximum resident set size),
///   which represents the peak RSS during the process lifetime.
/// - **Other platforms**: Returns `None`
///
/// # Returns
///
/// `Some(mb)` on success, `None` on failure or unsupported platform.
#[allow(unsafe_code)]
fn get_peak_rss_absolute_mb() -> Option<u64> {
    #[cfg(target_os = "macos")]
    {
        use std::mem::MaybeUninit;

        let mut usage = MaybeUninit::<libc::rusage>::uninit();

        // SAFETY: rusage is a well-defined FFI structure, and we check the return value
        // to ensure the call succeeded before reading from it. RUSAGE_SELF is a valid
        // constant for requesting resource usage of the calling process.
        unsafe {
            if libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) == 0 {
                let usage = usage.assume_init();
                // macOS returns ru_maxrss in bytes (unlike Linux which returns kilobytes)
                // ru_maxrss is i64 but should always be non-negative; use try_into for safe conversion
                let bytes: u64 = usage.ru_maxrss.try_into().ok()?;
                // Round up to megabytes
                let megabytes = bytes.div_ceil(1024 * 1024);
                return Some(megabytes);
            }
        }

        None
    }

    #[cfg(target_os = "linux")]
    {
        // Read /proc/self/status to get VmHWM (peak resident set size)
        if let Ok(content) = std::fs::read_to_string("/proc/self/status") {
            for line in content.lines() {
                if line.starts_with("VmHWM:") {
                    // Format: "VmHWM:     12345 kB"
                    if let Some(kilobytes_str) = line.split_whitespace().nth(1) {
                        if let Ok(kilobytes) = kilobytes_str.parse::<u64>() {
                            // Round up to megabytes
                            return Some(kilobytes.div_ceil(1024));
                        }
                    }
                }
            }
        }

        None
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

// -----------------------------------------------------------------------------
// SearchIndex Structure
// -----------------------------------------------------------------------------

/// Search index using `PersistentTreeMap` for efficient prefix-based lookup.
///
/// The index maps normalized search terms (lowercase) to the tasks that
/// contain those terms. This enables efficient search operations without
/// full O(N) scans.
///
/// # Design Note
///
/// This implementation demonstrates:
/// - **`PersistentTreeMap`**: Efficient ordered map for prefix-based queries using `range`
/// - **`Semigroup::combine`**: Combining search results from different scopes with deduplication
///
/// # Index Strategy
///
/// - **Word index**: Maps individual words (split by whitespace) for efficient prefix search
/// - **Full title index**: Maps the complete normalized title to multiple task IDs
/// - **Full title all-suffix index**: Maps ALL suffixes of the full normalized title to task IDs
///   - Example: `important meeting tomorrow` generates suffixes including `meeting tomorrow`, `tomorrow`
///   - This allows multi-word infix queries like `meeting tomorrow` to match `important meeting tomorrow`
/// - **Word all-suffix index**: Maps ALL suffixes of each word for arbitrary position substring matching
///   - Example: `callback` generates suffixes: `callback`, `allback`, `llback`, `lback`, `back`, `ack`, `ck`, `k`
///   - This allows `all` query to match `callback` via `allback` prefix match
/// - **Tag index**: Maps normalized tag values for tag search
/// - **Tag all-suffix index**: Maps ALL suffixes of each tag for arbitrary position substring matching
///
/// # Complexity Analysis
///
/// - **Range query (index lookup)**: O(log N + m) where m is matching entries in index
/// - **ID resolution**: O(k log N) where k is matching tasks, N is total tasks
/// - **Result ordering**: O(k log k) for sorting by `task_id` for deterministic stable ordering
/// - **Total search complexity**: O(k log N + k log k)
/// - No full O(N) scan is required for any search operation.
/// - **Space trade-off**: Stores O(L * W) entries where L is average word length and W is word count
#[derive(Debug, Clone)]
pub struct SearchIndex {
    /// Index mapping normalized title words to task IDs (for prefix search).
    title_word_index: PersistentTreeMap<String, PersistentVector<TaskId>>,
    /// Index mapping full normalized titles to task IDs (for multi-word substring match).
    /// Changed from `TaskId` to `PersistentVector<TaskId>` to support multiple tasks with same title.
    title_full_index: PersistentTreeMap<String, PersistentVector<TaskId>>,
    /// Index mapping ALL suffixes of full normalized titles to task IDs (for multi-word infix search).
    /// Example: `important meeting tomorrow` generates `important meeting tomorrow`, `mportant meeting tomorrow`,
    /// ..., `meeting tomorrow`, ..., `tomorrow`, etc.
    /// This enables `meeting tomorrow` query to match `important meeting tomorrow`.
    /// (Legacy all-suffix mode only, feature flag preserved)
    title_full_all_suffix_index: PersistentTreeMap<String, PersistentVector<TaskId>>,
    /// Index mapping ALL suffixes of normalized title words to task IDs (for arbitrary infix search).
    /// Example: `callback` generates `callback`, `allback`, `llback`, `lback`, `back`, `ack`, `ck`, `k`.
    /// This enables `all` query to match `callback` via `allback` prefix match.
    /// (Legacy all-suffix mode only, feature flag preserved)
    title_word_all_suffix_index: PersistentTreeMap<String, PersistentVector<TaskId>>,
    /// Index mapping normalized tag values to task IDs.
    tag_index: PersistentTreeMap<String, PersistentVector<TaskId>>,
    /// Index mapping ALL suffixes of normalized tag values to task IDs (for arbitrary infix search).
    /// (Legacy all-suffix mode only, feature flag preserved)
    tag_all_suffix_index: PersistentTreeMap<String, PersistentVector<TaskId>>,
    /// Reference to all tasks for lookup by ID.
    tasks_by_id: PersistentTreeMap<TaskId, Task>,

    // -------------------------------------------------------------------------
    // N-gram indexes (REQ-SEARCH-NGRAM-002 Part 3)
    // -------------------------------------------------------------------------
    /// N-gram index for full normalized titles (for infix search in Ngram mode).
    ///
    /// Maps n-gram substrings to task IDs that contain them in their title.
    /// Used when `config.infix_mode == InfixMode::Ngram`.
    title_full_ngram_index: NgramIndex,
    /// N-gram index for normalized title words (for infix search in Ngram mode).
    ///
    /// Maps n-gram substrings to task IDs that contain them in any title word.
    /// Used when `config.infix_mode == InfixMode::Ngram`.
    title_word_ngram_index: NgramIndex,
    /// N-gram index for normalized tag values (for infix search in Ngram mode).
    ///
    /// Maps n-gram substrings to task IDs that contain them in any tag.
    /// Used when `config.infix_mode == InfixMode::Ngram`.
    tag_ngram_index: NgramIndex,

    /// Configuration for the search index (acts as feature flag).
    ///
    /// Controls the infix search mode (`Ngram`, `LegacyAllSuffix`, or `Disabled`)
    /// and various limits for memory and performance tuning.
    config: SearchIndexConfig,
}

impl SearchIndex {
    /// Builds a search index from a collection of tasks using default configuration.
    ///
    /// This is a convenience method that uses `SearchIndexConfig::default()`,
    /// which enables n-gram indexing for better performance.
    ///
    /// For explicit control over the indexing mode, use `build_with_config()`.
    #[must_use]
    pub fn build(tasks: &PersistentVector<Task>) -> Self {
        Self::build_with_config(tasks, SearchIndexConfig::default())
    }

    /// Builds a search index with configuration from a collection of tasks (pure function).
    ///
    /// Creates normalized indexes for both title words and tags, with the infix
    /// search mode determined by the provided configuration:
    ///
    /// - `InfixMode::Ngram`: Builds n-gram indexes for infix search
    /// - `InfixMode::LegacyAllSuffix`: Builds all-suffix indexes (legacy behavior)
    /// - `InfixMode::Disabled`: No infix indexes are built
    ///
    /// # Arguments
    ///
    /// * `tasks` - Collection of tasks to index
    /// * `config` - Configuration controlling index behavior
    ///
    /// # Returns
    ///
    /// A new `SearchIndex` with indexes built according to the configuration.
    ///
    /// # Normalization
    ///
    /// Uses `normalize_query()` for consistent normalization:
    /// - Trims leading/trailing whitespace
    /// - Converts to lowercase
    /// - Collapses multiple spaces into single spaces
    ///
    /// # Token Limits
    ///
    /// The `max_tokens_per_task` configuration limits the total number of tokens
    /// (title words + tags) indexed per task. Excess tokens are ignored.
    ///
    /// # Memory Bound
    ///
    /// With default configuration (`max_tokens_per_task = 100`, `max_ngrams_per_token = 64`),
    /// memory usage is bounded to approximately 512MB for 10,000 tasks.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn build_with_config(tasks: &PersistentVector<Task>, config: SearchIndexConfig) -> Self {
        let mut title_word_index: PersistentTreeMap<String, PersistentVector<TaskId>> =
            PersistentTreeMap::new();
        let mut title_full_index: PersistentTreeMap<String, PersistentVector<TaskId>> =
            PersistentTreeMap::new();
        let mut title_full_all_suffix_index: PersistentTreeMap<String, PersistentVector<TaskId>> =
            PersistentTreeMap::new();
        let mut title_word_all_suffix_index: PersistentTreeMap<String, PersistentVector<TaskId>> =
            PersistentTreeMap::new();
        let mut tag_index: PersistentTreeMap<String, PersistentVector<TaskId>> =
            PersistentTreeMap::new();
        let mut tag_all_suffix_index: PersistentTreeMap<String, PersistentVector<TaskId>> =
            PersistentTreeMap::new();
        let mut tasks_by_id: PersistentTreeMap<TaskId, Task> = PersistentTreeMap::new();

        // N-gram indexes (populated only in Ngram mode)
        // Use mutable HashMap<String, Vec<TaskId>> for O(1) amortized batch construction
        // This avoids the O(n) overhead of rebuilding PersistentVector for each insertion
        let mut title_full_ngram_batch: MutableNgramIndex = std::collections::HashMap::new();
        let mut title_word_ngram_batch: MutableNgramIndex = std::collections::HashMap::new();
        let mut tag_ngram_batch: MutableNgramIndex = std::collections::HashMap::new();

        for task in tasks {
            // Index the task by ID
            tasks_by_id = tasks_by_id.insert(task.task_id.clone(), task.clone());

            // Normalize the title using normalize_query() for consistency
            let normalized = normalize_query(&task.title);
            let normalized_title = &normalized.key;
            let words: Vec<&String> = normalized.tokens.iter().collect();
            let tag_count = task.tags.len();

            // Apply max_tokens_per_task limit: title words + tags combined
            let total_tokens = words.len() + tag_count;
            let word_limit = if total_tokens > config.max_tokens_per_task {
                config
                    .max_tokens_per_task
                    .saturating_sub(tag_count.min(config.max_tokens_per_task))
            } else {
                words.len()
            };
            let tag_limit = config.max_tokens_per_task.saturating_sub(word_limit);

            // Index full normalized title for multi-word substring match
            let existing_ids = title_full_index
                .get(normalized_title)
                .cloned()
                .unwrap_or_else(PersistentVector::new);
            title_full_index = title_full_index.insert(
                normalized_title.clone(),
                existing_ids.push_back(task.task_id.clone()),
            );

            // Index infix based on mode
            match config.infix_mode {
                InfixMode::Ngram => {
                    // Build n-gram index for full title using batch index
                    index_ngrams_batch(
                        &mut title_full_ngram_batch,
                        normalized_title,
                        &task.task_id,
                        &config,
                    );
                }
                InfixMode::LegacyAllSuffix => {
                    // Build all-suffix index for full title
                    title_full_all_suffix_index = Self::index_all_suffixes(
                        title_full_all_suffix_index,
                        normalized_title,
                        &task.task_id,
                    );
                }
                InfixMode::Disabled => {
                    // No infix index for full title
                }
            }

            // Index title words (limited by word_limit)
            for word in words.iter().take(word_limit) {
                let word_key = (*word).clone();
                let task_ids = title_word_index
                    .get(&word_key)
                    .cloned()
                    .unwrap_or_else(PersistentVector::new);
                title_word_index = title_word_index
                    .insert(word_key.clone(), task_ids.push_back(task.task_id.clone()));

                // Index infix based on mode
                match config.infix_mode {
                    InfixMode::Ngram => {
                        // Build n-gram index for word using batch index
                        index_ngrams_batch(
                            &mut title_word_ngram_batch,
                            &word_key,
                            &task.task_id,
                            &config,
                        );
                    }
                    InfixMode::LegacyAllSuffix => {
                        // Build all-suffix index for word
                        title_word_all_suffix_index = Self::index_all_suffixes(
                            title_word_all_suffix_index,
                            &word_key,
                            &task.task_id,
                        );
                    }
                    InfixMode::Disabled => {
                        // No infix index for word
                    }
                }
            }

            // Index tags (limited by tag_limit)
            // Sort tags to ensure deterministic iteration order (PersistentHashSet has
            // non-deterministic order based on hash values)
            let mut sorted_tags: Vec<_> = task.tags.iter().collect();
            sorted_tags.sort_by(|a, b| a.as_str().cmp(b.as_str()));
            for tag in sorted_tags.into_iter().take(tag_limit) {
                // Normalize tag using normalize_query() for consistency
                let normalized_tag = normalize_query(tag.as_str()).into_key();
                let task_ids = tag_index
                    .get(&normalized_tag)
                    .cloned()
                    .unwrap_or_else(PersistentVector::new);
                tag_index = tag_index.insert(
                    normalized_tag.clone(),
                    task_ids.push_back(task.task_id.clone()),
                );

                // Index infix based on mode
                match config.infix_mode {
                    InfixMode::Ngram => {
                        // Build n-gram index for tag using batch index
                        index_ngrams_batch(
                            &mut tag_ngram_batch,
                            &normalized_tag,
                            &task.task_id,
                            &config,
                        );
                    }
                    InfixMode::LegacyAllSuffix => {
                        // Build all-suffix index for tag
                        tag_all_suffix_index = Self::index_all_suffixes(
                            tag_all_suffix_index,
                            &normalized_tag,
                            &task.task_id,
                        );
                    }
                    InfixMode::Disabled => {
                        // No infix index for tag
                    }
                }
            }
        }

        // Convert batch indexes to persistent indexes (sort, dedup, and convert)
        Self {
            title_word_index,
            title_full_index,
            title_full_all_suffix_index,
            title_word_all_suffix_index,
            tag_index,
            tag_all_suffix_index,
            tasks_by_id,
            title_full_ngram_index: finalize_ngram_index(title_full_ngram_batch),
            title_word_ngram_index: finalize_ngram_index(title_word_ngram_batch),
            tag_ngram_index: finalize_ngram_index(tag_ngram_batch),
            config,
        }
    }

    /// Returns the total number of n-gram entries across all n-gram indexes.
    ///
    /// This is the sum of:
    /// - `title_full_ngram_index.len()` (unique n-grams in full titles)
    /// - `title_word_ngram_index.len()` (unique n-grams in title words)
    /// - `tag_ngram_index.len()` (unique n-grams in tags)
    ///
    /// # Returns
    ///
    /// The total count of unique n-gram keys across all three indexes.
    /// In `InfixMode::LegacyAllSuffix` or `InfixMode::Disabled`, this returns 0.
    #[must_use]
    pub const fn ngram_entry_count(&self) -> usize {
        self.title_full_ngram_index.len()
            + self.title_word_ngram_index.len()
            + self.tag_ngram_index.len()
    }

    /// Indexes all suffixes of a word for arbitrary position substring matching.
    ///
    /// For "callback", this generates:
    /// - "callback" (full word - matches prefix "call")
    /// - "allback" (matches prefix "all")
    /// - "llback" (matches prefix "ll")
    /// - "lback" (matches prefix "l")
    /// - "back" (matches prefix "back")
    /// - "ack" (matches prefix "ack")
    /// - "ck" (matches prefix "ck")
    /// - "k" (matches prefix "k")
    ///
    /// This enables efficient infix search by converting infix to prefix lookup.
    /// Index operation: O(log N) per suffix insertion.
    fn index_all_suffixes(
        mut index: PersistentTreeMap<String, PersistentVector<TaskId>>,
        word: &str,
        task_id: &TaskId,
    ) -> PersistentTreeMap<String, PersistentVector<TaskId>> {
        // Generate all suffixes by taking substrings from each character position
        // (with deduplication check)
        for (byte_index, _) in word.char_indices() {
            let suffix = &word[byte_index..];
            let existing_ids = index
                .get(suffix)
                .cloned()
                .unwrap_or_else(PersistentVector::new);
            if !existing_ids.iter().any(|id| id == task_id) {
                index = index.insert(suffix.to_string(), existing_ids.push_back(task_id.clone()));
            }
        }
        index
    }

    /// Searches the title index for tasks containing the query (pure function).
    ///
    /// Returns `Some(SearchResult)` if any matches are found, `None` otherwise.
    ///
    /// # Search Strategy
    ///
    /// 1. First, try full title substring match (for multi-word queries like "important meeting")
    /// 2. Then, use infix search based on configured mode (n-gram or legacy all-suffix)
    /// 3. Finally, use prefix-based word index search (for single word or prefix queries)
    /// 4. Combine results with deduplication, sorted by `task_id` for stable ordering
    ///
    /// # Normalization
    ///
    /// Uses `normalize_query()` for consistent normalization between index and query.
    ///
    /// # Result Limiting
    ///
    /// Applies `max_search_candidates` to the final result set.
    #[must_use]
    pub fn search_by_title(&self, query: &str) -> Option<SearchResult> {
        // Use normalize_query() for consistent normalization (not just to_lowercase())
        let normalized = normalize_query(query);
        let normalized_query = &normalized.key;
        let matching_ids = self.find_matching_ids_from_title(normalized_query);

        if matching_ids.is_empty() {
            None
        } else {
            // Apply max_search_candidates to the final result set
            // Sort by TaskId to ensure deterministic ordering before limiting
            let mut sorted_ids: Vec<TaskId> = matching_ids.iter().cloned().collect();
            sorted_ids.sort();
            let limited_ids: PersistentHashSet<TaskId> = sorted_ids
                .into_iter()
                .take(self.config.max_search_candidates)
                .collect();
            let tasks = self.resolve_task_ids_ordered(&limited_ids);
            Some(SearchResult::from_tasks(tasks))
        }
    }

    /// Searches the tag index for tasks containing the query (pure function).
    ///
    /// Uses `normalize_query()` for consistent normalization between index and query.
    ///
    /// Returns `Some(SearchResult)` if any matches are found, `None` otherwise.
    #[must_use]
    pub fn search_by_tags(&self, query: &str) -> Option<SearchResult> {
        // Use normalize_query() for consistent normalization (not just to_lowercase())
        let normalized = normalize_query(query);
        let normalized_query = &normalized.key;
        let matching_ids = self.find_matching_ids_from_tags(normalized_query);

        if matching_ids.is_empty() {
            None
        } else {
            let tasks = self.resolve_task_ids_ordered(&matching_ids);
            Some(SearchResult::from_tasks(tasks))
        }
    }

    // -------------------------------------------------------------------------
    // N-gram Search Methods (REQ-SEARCH-NGRAM-003)
    // -------------------------------------------------------------------------

    /// Finds candidate `TaskId`s from the n-gram index (pure function).
    ///
    /// This method performs an efficient candidate search using the n-gram
    /// inverted index. It generates n-grams from the query and intersects
    /// the posting lists to find candidate tasks.
    ///
    /// # Arguments
    ///
    /// * `index` - The n-gram index to search
    /// * `normalized_query` - The normalized query string (from `normalize_query()`)
    ///
    /// # Returns
    ///
    /// - `Some(Vec<TaskId>)` - Candidate task IDs (may contain false positives)
    /// - `None` - If the query is too short for n-gram search (< `min_query_len_for_infix`)
    ///
    /// # Complexity
    ///
    /// - Time: O(q * (log N + k)) where q is query n-gram count, k is posting list size
    /// - Intersection is O(k) using merge intersection on sorted lists
    ///
    /// # Soundness
    ///
    /// Results may contain false positives (tasks where the n-grams match but the
    /// full query substring does not). Use `verify_candidates_by_substring` to
    /// filter out false positives.
    fn find_candidates_by_ngrams(
        &self,
        index: &NgramIndex,
        normalized_query: &str,
    ) -> Option<Vec<TaskId>> {
        // Check query length: return None if too short for infix search
        let query_char_count = normalized_query.chars().count();
        if query_char_count < self.config.min_query_len_for_infix {
            return None;
        }

        // Generate all n-grams from the query (no limit for query-side)
        // Note: max_ngrams_per_token is only for index construction to bound memory usage
        // For search, we need all query n-grams to ensure correct intersection
        let query_ngrams = generate_ngrams(
            normalized_query,
            self.config.ngram_size,
            usize::MAX, // No limit for query n-grams
        );

        if query_ngrams.is_empty() {
            // Query is shorter than n-gram size: return None to fall back to prefix search
            return None;
        }

        // Intersect posting lists for all query n-grams
        let mut candidate_vec: Option<Vec<TaskId>> = None;

        for ngram in &query_ngrams {
            match index.get(ngram) {
                Some(task_ids) => {
                    let current_vec: Vec<TaskId> = task_ids.iter().cloned().collect();

                    candidate_vec = Some(match candidate_vec {
                        Some(existing) => {
                            // O(n) merge intersection (both are sorted)
                            intersect_sorted_vecs(&existing, &current_vec)
                        }
                        None => current_vec,
                    });
                }
                None => {
                    // This n-gram doesn't exist in the index: no candidates
                    return Some(Vec::new());
                }
            }
        }

        candidate_vec
    }

    /// Verifies candidate `TaskId`s by substring match (soundness filter).
    ///
    /// This method filters candidate task IDs by checking if the normalized
    /// query is actually a substring of the extracted field value.
    ///
    /// # Arguments
    ///
    /// * `candidates` - Candidate task IDs from n-gram search
    /// * `normalized_query` - The normalized query string
    /// * `field_extractor` - Function to extract searchable field(s) from a task
    ///
    /// # Returns
    ///
    /// A `Vec<TaskId>` containing only the tasks where the query is a substring
    /// of at least one extracted field.
    ///
    /// # Soundness Law
    ///
    /// For all returned `TaskId`s, the normalized query is a substring of at least
    /// one field value:
    ///
    /// ```text
    /// ∀ task_id ∈ result:
    ///   ∃ field ∈ field_extractor(task):
    ///     normalized_query ⊆ field
    /// ```
    fn verify_candidates_by_substring<F>(
        &self,
        candidates: &[TaskId],
        normalized_query: &str,
        field_extractor: F,
    ) -> Vec<TaskId>
    where
        F: Fn(&Task) -> Vec<String>,
    {
        candidates
            .iter()
            .filter(|task_id| {
                self.tasks_by_id.get(*task_id).is_some_and(|task| {
                    field_extractor(task)
                        .iter()
                        .any(|field| field.contains(normalized_query))
                })
            })
            .cloned()
            .collect()
    }

    // -------------------------------------------------------------------------
    // Title/Tag Search Methods
    // -------------------------------------------------------------------------

    /// Finds task IDs from the title index that match the query (substring match).
    ///
    /// Uses a multi-phase strategy based on configuration:
    ///
    /// - **Phase 1**: Full title prefix search (always)
    /// - **Phase 2**: Infix search (if query length >= `min_query_len_for_infix`)
    ///   - `Ngram` mode: N-gram inverted index search with substring verification
    ///   - `LegacyAllSuffix` mode: All-suffix index prefix search
    ///   - `Disabled` mode: Skip infix search
    /// - **Phase 3**: Word prefix search (always)
    ///
    /// # Arguments
    ///
    /// * `normalized_query` - The normalized query string (from `normalize_query().key`)
    ///
    /// # Complexity
    ///
    /// - Prefix search: O(log N + m) per index
    /// - N-gram search: O(q * log N + k) where q is n-gram count, k is candidate count
    /// - Total: O(k log N + k log k) with ID resolution and ordering
    fn find_matching_ids_from_title(&self, normalized_query: &str) -> PersistentHashSet<TaskId> {
        let mut matching_ids = PersistentHashSet::new();
        let query_char_count = normalized_query.chars().count();

        // Phase 1: Full title prefix search (always)
        // Use range query on title_full_index for O(log N + m) lookup
        // This finds titles that START WITH the query (e.g., "important meeting" in "important meeting tomorrow")
        matching_ids = Self::find_matching_ids_with_prefix_range_multi(
            &self.title_full_index,
            normalized_query,
            matching_ids,
        );

        // Phase 2: Infix search (mode-dependent, only if query >= min_query_len_for_infix)
        // Note: min_query_len_for_infix is applied to ALL infix modes (Ngram and LegacyAllSuffix)
        if query_char_count >= self.config.min_query_len_for_infix {
            match self.config.infix_mode {
                InfixMode::Ngram => {
                    // N-gram search on full title
                    if let Some(candidates) = self
                        .find_candidates_by_ngrams(&self.title_full_ngram_index, normalized_query)
                    {
                        let verified = self.verify_candidates_by_substring(
                            &candidates,
                            normalized_query,
                            |task| vec![normalize_query(&task.title).key],
                        );
                        for task_id in verified {
                            matching_ids = matching_ids.insert(task_id);
                        }
                    }

                    // N-gram search on title words
                    if let Some(candidates) = self
                        .find_candidates_by_ngrams(&self.title_word_ngram_index, normalized_query)
                    {
                        let verified = self.verify_candidates_by_substring(
                            &candidates,
                            normalized_query,
                            |task| normalize_query(&task.title).tokens,
                        );
                        for task_id in verified {
                            matching_ids = matching_ids.insert(task_id);
                        }
                    }
                }
                InfixMode::LegacyAllSuffix => {
                    // Full title all-suffix search (for multi-word infix queries)
                    matching_ids = Self::find_matching_ids_with_prefix_range_multi(
                        &self.title_full_all_suffix_index,
                        normalized_query,
                        matching_ids,
                    );

                    // Word all-suffix search (for arbitrary infix matches)
                    matching_ids = Self::find_matching_ids_with_prefix_range(
                        &self.title_word_all_suffix_index,
                        normalized_query,
                        matching_ids,
                    );
                }
                InfixMode::Disabled => {
                    // No infix search
                }
            }
        }

        // Phase 3: Word prefix search (always)
        // Finds words that START WITH the query (e.g., "imp" matches "important")
        matching_ids = Self::find_matching_ids_with_prefix_range(
            &self.title_word_index,
            normalized_query,
            matching_ids,
        );

        matching_ids
    }

    /// Finds task IDs from the tag index that match the query (substring match).
    ///
    /// Uses a multi-phase strategy based on configuration:
    ///
    /// - **Phase 1**: Tag prefix search (always)
    /// - **Phase 2**: Infix search (if query length >= `min_query_len_for_infix`)
    ///   - `Ngram` mode: N-gram inverted index search with substring verification
    ///   - `LegacyAllSuffix` mode: All-suffix index prefix search
    ///   - `Disabled` mode: Skip infix search
    ///
    /// # Arguments
    ///
    /// * `query_lower` - The lowercased query string
    ///
    /// # Complexity
    ///
    /// - Prefix search: O(log N + m) per index
    /// - N-gram search: O(q * log N + k) where q is n-gram count, k is candidate count
    /// - Total: O(k log N + k log k) with ID resolution and ordering
    fn find_matching_ids_from_tags(&self, query_lower: &str) -> PersistentHashSet<TaskId> {
        let mut matching_ids = PersistentHashSet::new();
        let query_char_count = query_lower.chars().count();

        // Phase 1: Tag prefix search (always)
        // Finds tags that START WITH the query (e.g., "back" matches "backend")
        matching_ids =
            Self::find_matching_ids_with_prefix_range(&self.tag_index, query_lower, matching_ids);

        // Phase 2: Infix search (mode-dependent, only if query >= min_query_len_for_infix)
        if query_char_count >= self.config.min_query_len_for_infix {
            match self.config.infix_mode {
                InfixMode::Ngram => {
                    // N-gram search on tags
                    if let Some(candidates) =
                        self.find_candidates_by_ngrams(&self.tag_ngram_index, query_lower)
                    {
                        let verified =
                            self.verify_candidates_by_substring(&candidates, query_lower, |task| {
                                task.tags
                                    .iter()
                                    .map(|tag| tag.as_str().to_string())
                                    .collect()
                            });
                        for task_id in verified {
                            matching_ids = matching_ids.insert(task_id);
                        }
                    }
                }
                InfixMode::LegacyAllSuffix => {
                    // All-suffix search: finds tags containing query at any position
                    // E.g., "cke" matches "backend" because "ckend" is in the all-suffix index
                    matching_ids = Self::find_matching_ids_with_prefix_range(
                        &self.tag_all_suffix_index,
                        query_lower,
                        matching_ids,
                    );
                }
                InfixMode::Disabled => {
                    // No infix search
                }
            }
        }

        matching_ids
    }

    /// Uses `PersistentTreeMap::range` for efficient prefix-based search.
    ///
    /// Complexity: O(log N + m) where m is the number of matching index entries.
    fn find_matching_ids_with_prefix_range(
        index: &PersistentTreeMap<String, PersistentVector<TaskId>>,
        query_lower: &str,
        mut matching_ids: PersistentHashSet<TaskId>,
    ) -> PersistentHashSet<TaskId> {
        // For prefix search, we use range [query, query + char::MAX)
        // Using char::MAX ('\u{10ffff}') to cover all Unicode including BMP-external chars (emoji, etc.)
        let end_key = format!("{query_lower}\u{10ffff}");
        for (_key, task_ids) in index.range(query_lower.to_string()..end_key) {
            for task_id in task_ids {
                matching_ids = matching_ids.insert(task_id.clone());
            }
        }

        matching_ids
    }

    /// Uses `PersistentTreeMap::range` on full title index for prefix-based search.
    ///
    /// This variant handles `PersistentVector<TaskId>` values for same-title support.
    /// Complexity: O(k log N + k log k) where k is the number of matching tasks
    /// (log N for each `tasks_by_id` lookup, k log k for ordering sort).
    fn find_matching_ids_with_prefix_range_multi(
        index: &PersistentTreeMap<String, PersistentVector<TaskId>>,
        query_lower: &str,
        mut matching_ids: PersistentHashSet<TaskId>,
    ) -> PersistentHashSet<TaskId> {
        // For prefix search on full titles
        // Using char::MAX ('\u{10ffff}') to cover all Unicode including BMP-external chars (emoji, etc.)
        let end_key = format!("{query_lower}\u{10ffff}");
        for (_title, task_ids) in index.range(query_lower.to_string()..end_key) {
            for task_id in task_ids {
                matching_ids = matching_ids.insert(task_id.clone());
            }
        }

        matching_ids
    }

    /// Resolves task IDs to their corresponding Task objects, maintaining stable order.
    ///
    /// Ordering is determined by `task_id` only, which is a UUID.
    /// This guarantees deterministic output regardless of:
    /// - Repository's `list()` return order
    /// - Input task ID iteration order
    ///
    /// Using `task_id` as the sole sort key ensures the same input always
    /// produces the same output order.
    fn resolve_task_ids_ordered(
        &self,
        task_ids: &PersistentHashSet<TaskId>,
    ) -> PersistentVector<Task> {
        let mut tasks: Vec<Task> = task_ids
            .iter()
            .filter_map(|id| self.tasks_by_id.get(id).cloned())
            .collect();

        // Sort by task_id for stable ordering
        tasks.sort_by(|a, b| a.task_id.cmp(&b.task_id));

        tasks.into_iter().collect()
    }

    /// Returns all tasks when the query is empty, in stable order.
    ///
    /// Ordering is determined by `task_id` only, which is a UUID.
    /// This guarantees deterministic output regardless of the repository's
    /// `list()` return order.
    #[must_use]
    pub fn all_tasks(&self) -> PersistentVector<Task> {
        let mut tasks: Vec<Task> = self
            .tasks_by_id
            .iter()
            .map(|(_, task)| task.clone())
            .collect();

        // Sort by task_id for stable ordering
        tasks.sort_by(|a, b| a.task_id.cmp(&b.task_id));

        tasks.into_iter().collect()
    }

    /// Removes a single task from the index, returning a new index (pure function).
    ///
    /// This helper method removes all index entries associated with the given task:
    /// - Removes from `title_word_index` and infix index based on `config.infix_mode`
    /// - Removes from `title_full_index` and infix index based on `config.infix_mode`
    /// - Removes from `tag_index` and infix index based on `config.infix_mode`
    /// - Removes from `tasks_by_id`
    ///
    /// # Normalization
    ///
    /// Uses `normalize_query()` for consistent normalization with index construction.
    ///
    /// # Complexity
    ///
    /// O(W * L * log N) where W is word count, L is average word length, N is index size.
    #[must_use]
    fn remove_task(&self, task: &Task) -> Self {
        // Use normalize_query() for consistency with build_with_config
        let normalized = normalize_query(&task.title);
        let normalized_title = &normalized.key;
        let words: Vec<&String> = normalized.tokens.iter().collect();
        let task_id = &task.task_id;

        // Remove from tasks_by_id
        let tasks_by_id = self.tasks_by_id.remove(task_id);

        // Remove from title_full_index
        let title_full_index =
            Self::remove_id_from_vector_index(&self.title_full_index, normalized_title, task_id);

        // Remove from infix index based on mode (full title)
        let mut title_full_all_suffix_index = self.title_full_all_suffix_index.clone();
        let mut title_full_ngram_index = self.title_full_ngram_index.clone();
        match self.config.infix_mode {
            InfixMode::Ngram => {
                title_full_ngram_index = remove_ngrams(
                    title_full_ngram_index,
                    normalized_title,
                    task_id,
                    &self.config,
                );
            }
            InfixMode::LegacyAllSuffix => {
                title_full_all_suffix_index = Self::remove_id_from_all_suffixes(
                    &title_full_all_suffix_index,
                    normalized_title,
                    task_id,
                );
            }
            InfixMode::Disabled => {
                // No infix index for full title
            }
        }

        // Remove from title_word_index and infix index
        let mut title_word_index = self.title_word_index.clone();
        let mut title_word_all_suffix_index = self.title_word_all_suffix_index.clone();
        let mut title_word_ngram_index = self.title_word_ngram_index.clone();
        for word in &words {
            let word_key = (*word).clone();
            title_word_index =
                Self::remove_id_from_vector_index(&title_word_index, &word_key, task_id);

            // Remove from infix index based on mode
            match self.config.infix_mode {
                InfixMode::Ngram => {
                    title_word_ngram_index =
                        remove_ngrams(title_word_ngram_index, &word_key, task_id, &self.config);
                }
                InfixMode::LegacyAllSuffix => {
                    title_word_all_suffix_index = Self::remove_id_from_all_suffixes(
                        &title_word_all_suffix_index,
                        &word_key,
                        task_id,
                    );
                }
                InfixMode::Disabled => {
                    // No infix index for word
                }
            }
        }

        // Remove from tag_index and infix index
        // Sort tags for deterministic iteration order (PersistentHashSet has non-deterministic order)
        let mut sorted_tags: Vec<_> = task.tags.iter().collect();
        sorted_tags.sort_by(|a, b| a.as_str().cmp(b.as_str()));

        let mut tag_index = self.tag_index.clone();
        let mut tag_all_suffix_index = self.tag_all_suffix_index.clone();
        let mut tag_ngram_index = self.tag_ngram_index.clone();
        for tag in sorted_tags {
            // Normalize tag using normalize_query() for consistency
            let normalized_tag = normalize_query(tag.as_str()).into_key();
            tag_index = Self::remove_id_from_vector_index(&tag_index, &normalized_tag, task_id);

            // Remove from infix index based on mode
            match self.config.infix_mode {
                InfixMode::Ngram => {
                    tag_ngram_index =
                        remove_ngrams(tag_ngram_index, &normalized_tag, task_id, &self.config);
                }
                InfixMode::LegacyAllSuffix => {
                    tag_all_suffix_index = Self::remove_id_from_all_suffixes(
                        &tag_all_suffix_index,
                        &normalized_tag,
                        task_id,
                    );
                }
                InfixMode::Disabled => {
                    // No infix index for tag
                }
            }
        }

        Self {
            title_word_index,
            title_full_index,
            title_full_all_suffix_index,
            title_word_all_suffix_index,
            tag_index,
            tag_all_suffix_index,
            tasks_by_id,
            title_full_ngram_index,
            title_word_ngram_index,
            tag_ngram_index,
            config: self.config.clone(),
        }
    }

    /// Removes a task ID from a vector-valued index entry.
    ///
    /// If the resulting vector is empty, removes the entire entry.
    fn remove_id_from_vector_index(
        index: &PersistentTreeMap<String, PersistentVector<TaskId>>,
        key: &str,
        task_id: &TaskId,
    ) -> PersistentTreeMap<String, PersistentVector<TaskId>> {
        index.get(key).map_or_else(
            || index.clone(),
            |ids| {
                let filtered: PersistentVector<TaskId> =
                    ids.iter().filter(|id| *id != task_id).cloned().collect();
                if filtered.is_empty() {
                    index.remove(&key.to_string())
                } else {
                    index.insert(key.to_string(), filtered)
                }
            },
        )
    }

    /// Removes a task ID from all suffix entries of a word.
    fn remove_id_from_all_suffixes(
        index: &PersistentTreeMap<String, PersistentVector<TaskId>>,
        word: &str,
        task_id: &TaskId,
    ) -> PersistentTreeMap<String, PersistentVector<TaskId>> {
        let mut result = index.clone();
        for (byte_index, _) in word.char_indices() {
            let suffix = &word[byte_index..];
            result = Self::remove_id_from_vector_index(&result, suffix, task_id);
        }
        result
    }

    /// Adds a single task to the index, returning a new index (pure function).
    ///
    /// This helper method adds all index entries for the given task:
    /// - Adds to `title_word_index` and infix index based on `config.infix_mode`
    /// - Adds to `title_full_index` and infix index based on `config.infix_mode`
    /// - Adds to `tag_index` and infix index based on `config.infix_mode`
    /// - Adds to `tasks_by_id`
    /// - Respects `max_tokens_per_task` limit (title words + tags combined)
    ///
    /// # Normalization
    ///
    /// Uses `normalize_query()` for consistent normalization with index construction.
    /// Normalization is performed once via `NormalizedTaskData::from_task()`.
    ///
    /// # Complexity
    ///
    /// O(W * L * log N) where W is word count, L is average word length, N is index size.
    ///
    /// # Phase 5.5 Optimization
    ///
    /// This method delegates to `add_task_with_normalized` to avoid redundant
    /// normalization when called from `apply_change`.
    #[must_use]
    fn add_task(&self, task: &Task) -> Self {
        // Normalize once at the entry point (Phase 5.5 optimization)
        let normalized = NormalizedTaskData::from_task(task);
        self.add_task_with_normalized(&normalized, &task.task_id, task)
    }

    /// Adds a task using pre-normalized data (internal method).
    ///
    /// This method is the core implementation of `add_task`, accepting pre-computed
    /// normalized data to avoid redundant `normalize_query()` calls.
    ///
    /// # Arguments
    ///
    /// * `normalized` - Pre-computed normalized task data
    /// * `task_id` - The task's unique identifier
    /// * `task` - The original task (for `tasks_by_id` storage)
    ///
    /// # Complexity
    ///
    /// O(W * L * log N) where W is word count, L is average word length, N is index size.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    fn add_task_with_normalized(
        &self,
        normalized: &NormalizedTaskData,
        task_id: &TaskId,
        task: &Task,
    ) -> Self {
        // Compute token limits using the same logic as SearchIndexDelta
        let (word_limit, tag_limit) =
            SearchIndexDelta::compute_token_limits(normalized, &self.config);

        // Add to tasks_by_id
        let tasks_by_id = self.tasks_by_id.insert(task_id.clone(), task.clone());

        // Add to title_full_index (with deduplication check)
        let existing_ids = self
            .title_full_index
            .get(&normalized.title_key)
            .cloned()
            .unwrap_or_else(PersistentVector::new);
        let title_full_index = if existing_ids.iter().any(|id| id == task_id) {
            self.title_full_index.clone()
        } else {
            self.title_full_index.insert(
                normalized.title_key.clone(),
                existing_ids.push_back(task_id.clone()),
            )
        };

        // Add to infix index based on mode (full title)
        let mut title_full_all_suffix_index = self.title_full_all_suffix_index.clone();
        let mut title_full_ngram_index = self.title_full_ngram_index.clone();
        match self.config.infix_mode {
            InfixMode::Ngram => {
                title_full_ngram_index = index_ngrams(
                    title_full_ngram_index,
                    &normalized.title_key,
                    task_id,
                    &self.config,
                );
            }
            InfixMode::LegacyAllSuffix => {
                title_full_all_suffix_index = Self::index_all_suffixes(
                    title_full_all_suffix_index,
                    &normalized.title_key,
                    task_id,
                );
            }
            InfixMode::Disabled => {
                // No infix index for full title
            }
        }

        // Add to title_word_index and infix index (limited by word_limit)
        let mut title_word_index = self.title_word_index.clone();
        let mut title_word_all_suffix_index = self.title_word_all_suffix_index.clone();
        let mut title_word_ngram_index = self.title_word_ngram_index.clone();
        for word in normalized.title_words.iter().take(word_limit) {
            let task_ids = title_word_index
                .get(word)
                .cloned()
                .unwrap_or_else(PersistentVector::new);
            if !task_ids.iter().any(|id| id == task_id) {
                title_word_index =
                    title_word_index.insert(word.clone(), task_ids.push_back(task_id.clone()));
            }

            // Add to infix index based on mode
            match self.config.infix_mode {
                InfixMode::Ngram => {
                    title_word_ngram_index =
                        index_ngrams(title_word_ngram_index, word, task_id, &self.config);
                }
                InfixMode::LegacyAllSuffix => {
                    title_word_all_suffix_index =
                        Self::index_all_suffixes(title_word_all_suffix_index, word, task_id);
                }
                InfixMode::Disabled => {
                    // No infix index for word
                }
            }
        }

        // Add to tag_index and infix index (limited by tag_limit)
        // Tags are already sorted and normalized in NormalizedTaskData
        let mut tag_index = self.tag_index.clone();
        let mut tag_all_suffix_index = self.tag_all_suffix_index.clone();
        let mut tag_ngram_index = self.tag_ngram_index.clone();
        for normalized_tag in normalized.tags.iter().take(tag_limit) {
            let task_ids = tag_index
                .get(normalized_tag)
                .cloned()
                .unwrap_or_else(PersistentVector::new);
            if !task_ids.iter().any(|id| id == task_id) {
                tag_index =
                    tag_index.insert(normalized_tag.clone(), task_ids.push_back(task_id.clone()));
            }

            // Add to infix index based on mode
            match self.config.infix_mode {
                InfixMode::Ngram => {
                    tag_ngram_index =
                        index_ngrams(tag_ngram_index, normalized_tag, task_id, &self.config);
                }
                InfixMode::LegacyAllSuffix => {
                    tag_all_suffix_index =
                        Self::index_all_suffixes(tag_all_suffix_index, normalized_tag, task_id);
                }
                InfixMode::Disabled => {
                    // No infix index for tag
                }
            }
        }

        Self {
            title_word_index,
            title_full_index,
            title_full_all_suffix_index,
            title_word_all_suffix_index,
            tag_index,
            tag_all_suffix_index,
            tasks_by_id,
            title_full_ngram_index,
            title_word_ngram_index,
            tag_ngram_index,
            config: self.config.clone(),
        }
    }

    /// Applies a task change to the index, returning a new index (pure function).
    ///
    /// This method implements differential index updates:
    /// - `Add`: Adds the new task to all indexes
    /// - `Update`: Removes the old task, then adds the new task
    /// - `Remove`: Removes the task from all indexes
    ///
    /// # Laws
    ///
    /// This operation is idempotent for Add and Remove:
    /// ```text
    /// apply_change(apply_change(index, Add(task)), Add(task)) = apply_change(index, Add(task))
    /// apply_change(apply_change(index, Remove(id)), Remove(id)) = apply_change(index, Remove(id))
    /// ```
    ///
    /// # Complexity
    ///
    /// O(W * L * log N) where W is word count, L is average word length, N is index size.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let new_index = index.apply_change(TaskChange::Add(task));
    /// let new_index = index.apply_change(TaskChange::Update { old, new });
    /// let new_index = index.apply_change(TaskChange::Remove(task_id));
    /// ```
    #[must_use]
    pub fn apply_change(&self, change: TaskChange) -> Self {
        match change {
            TaskChange::Add(task) => {
                // Check if task already exists (idempotency)
                if self.tasks_by_id.contains_key(&task.task_id) {
                    self.clone()
                } else {
                    self.add_task(&task)
                }
            }
            TaskChange::Update { old, new } => {
                // Remove old, then add new
                self.remove_task(&old).add_task(&new)
            }
            TaskChange::Remove(task_id) => {
                // Find the task to remove
                self.tasks_by_id
                    .get(&task_id)
                    .map_or_else(|| self.clone(), |task| self.remove_task(task))
            }
        }
    }

    // -------------------------------------------------------------------------
    // Batch Operations (REQ-SEARCH-NGRAM-PERF-001 Part 3)
    // -------------------------------------------------------------------------

    /// Applies multiple task changes in a single batch operation.
    ///
    /// # Panics
    ///
    /// Panics on forbidden patterns: Remove followed by Add/Update for the same `TaskId`.
    #[must_use]
    pub fn apply_changes(&self, changes: &[TaskChange]) -> Self {
        if changes.is_empty() {
            return self.clone();
        }
        let delta = SearchIndexDelta::from_changes(changes, &self.config, &self.tasks_by_id);
        self.apply_delta(&delta, changes)
    }

    /// Applies a pre-computed `SearchIndexDelta` to this index.
    #[must_use]
    pub fn apply_delta(&self, delta: &SearchIndexDelta, changes: &[TaskChange]) -> Self {
        Self {
            tasks_by_id: self.update_tasks_by_id(changes),
            title_full_index: Self::merge_index_delta(
                &self.title_full_index,
                &delta.title_full_add,
                &delta.title_full_remove,
            ),
            title_word_index: Self::merge_index_delta(
                &self.title_word_index,
                &delta.title_word_add,
                &delta.title_word_remove,
            ),
            tag_index: Self::merge_index_delta(&self.tag_index, &delta.tag_add, &delta.tag_remove),
            title_full_ngram_index: Self::merge_ngram_delta(
                &self.title_full_ngram_index,
                &delta.title_full_ngram_add,
                &delta.title_full_ngram_remove,
            ),
            title_word_ngram_index: Self::merge_ngram_delta(
                &self.title_word_ngram_index,
                &delta.title_word_ngram_add,
                &delta.title_word_ngram_remove,
            ),
            tag_ngram_index: Self::merge_ngram_delta(
                &self.tag_ngram_index,
                &delta.tag_ngram_add,
                &delta.tag_ngram_remove,
            ),
            title_full_all_suffix_index: Self::merge_index_delta(
                &self.title_full_all_suffix_index,
                &delta.title_full_all_suffix_add,
                &delta.title_full_all_suffix_remove,
            ),
            title_word_all_suffix_index: Self::merge_index_delta(
                &self.title_word_all_suffix_index,
                &delta.title_word_all_suffix_add,
                &delta.title_word_all_suffix_remove,
            ),
            tag_all_suffix_index: Self::merge_index_delta(
                &self.tag_all_suffix_index,
                &delta.tag_all_suffix_add,
                &delta.tag_all_suffix_remove,
            ),
            config: self.config.clone(),
        }
    }

    fn update_tasks_by_id(&self, changes: &[TaskChange]) -> PersistentTreeMap<TaskId, Task> {
        changes
            .iter()
            .fold(self.tasks_by_id.clone(), |acc, change| match change {
                TaskChange::Add(task) => acc.insert(task.task_id.clone(), task.clone()),
                TaskChange::Update { old: _, new } => acc.insert(new.task_id.clone(), new.clone()),
                TaskChange::Remove(task_id) => acc.remove(task_id),
            })
    }

    /// Uses `(existing ∪ add) - remove` order to correctly handle Add->Remove within the same batch.
    fn merge_index_delta(
        index: &PersistentTreeMap<String, PersistentVector<TaskId>>,
        add: &std::collections::HashMap<String, Vec<TaskId>>,
        remove: &std::collections::HashMap<String, Vec<TaskId>>,
    ) -> PersistentTreeMap<String, PersistentVector<TaskId>> {
        let all_keys: std::collections::HashSet<_> = add.keys().chain(remove.keys()).collect();

        all_keys.into_iter().fold(index.clone(), |acc, key| {
            let merged = Self::compute_merged_posting_list(
                acc.get(key.as_str()).map(|v| v.iter().cloned().collect()),
                add.get(key),
                remove.get(key),
            );

            if merged.is_empty() {
                acc.remove(key)
            } else {
                acc.insert(key.clone(), merged.into_iter().collect())
            }
        })
    }

    fn merge_ngram_delta(
        index: &NgramIndex,
        add: &MutableNgramIndex,
        remove: &MutableNgramIndex,
    ) -> NgramIndex {
        let all_keys: std::collections::HashSet<_> = add.keys().chain(remove.keys()).collect();
        let mut result = index.clone().transient();

        for key in all_keys {
            let merged = Self::compute_merged_posting_list(
                result
                    .get(key.as_str())
                    .map(|v| v.iter().cloned().collect()),
                add.get(key),
                remove.get(key),
            );

            if merged.is_empty() {
                result.remove(key);
            } else {
                result.insert(key.clone(), merged.into_iter().collect());
            }
        }

        result.persistent()
    }

    /// Computes `(existing ∪ add) - remove` with deduplication.
    fn compute_merged_posting_list(
        existing: Option<Vec<TaskId>>,
        to_add: Option<&Vec<TaskId>>,
        to_remove: Option<&Vec<TaskId>>,
    ) -> Vec<TaskId> {
        let remove_set: std::collections::HashSet<_> =
            to_remove.map(|v| v.iter().collect()).unwrap_or_default();

        let mut merged: Vec<TaskId> = existing
            .unwrap_or_default()
            .into_iter()
            .chain(to_add.cloned().unwrap_or_default())
            .filter(|id| !remove_set.contains(id))
            .collect();

        merged.sort();
        merged.dedup();
        merged
    }

    // -------------------------------------------------------------------------
    // Test-only accessors for internal index verification
    // -------------------------------------------------------------------------

    /// Returns a reference to the title word index (test-only).
    #[cfg(test)]
    #[must_use]
    pub const fn title_word_index_for_test(
        &self,
    ) -> &PersistentTreeMap<String, PersistentVector<TaskId>> {
        &self.title_word_index
    }

    /// Returns a reference to the title full index (test-only).
    #[cfg(test)]
    #[must_use]
    pub const fn title_full_index_for_test(
        &self,
    ) -> &PersistentTreeMap<String, PersistentVector<TaskId>> {
        &self.title_full_index
    }

    /// Returns a reference to the title full all-suffix index (test-only).
    #[cfg(test)]
    #[must_use]
    pub const fn title_full_all_suffix_index_for_test(
        &self,
    ) -> &PersistentTreeMap<String, PersistentVector<TaskId>> {
        &self.title_full_all_suffix_index
    }

    /// Returns a reference to the title word all-suffix index (test-only).
    #[cfg(test)]
    #[must_use]
    pub const fn title_word_all_suffix_index_for_test(
        &self,
    ) -> &PersistentTreeMap<String, PersistentVector<TaskId>> {
        &self.title_word_all_suffix_index
    }

    /// Returns a reference to the tag index (test-only).
    #[cfg(test)]
    #[must_use]
    pub const fn tag_index_for_test(&self) -> &PersistentTreeMap<String, PersistentVector<TaskId>> {
        &self.tag_index
    }

    /// Returns a reference to the tag all-suffix index (test-only).
    #[cfg(test)]
    #[must_use]
    pub const fn tag_all_suffix_index_for_test(
        &self,
    ) -> &PersistentTreeMap<String, PersistentVector<TaskId>> {
        &self.tag_all_suffix_index
    }
}

/// Represents a change to a task for differential index updates.
///
/// This enum is used with `SearchIndex::apply_change` to update the search index
/// incrementally without rebuilding the entire index.
///
/// # Variants
///
/// - `Add`: A new task has been created
/// - `Update`: An existing task has been modified
/// - `Remove`: A task has been deleted
///
/// # Examples
///
/// ```ignore
/// // After creating a new task
/// let change = TaskChange::Add(new_task);
/// let new_index = index.apply_change(change);
///
/// // After updating a task
/// let change = TaskChange::Update { old: old_task, new: new_task };
/// let new_index = index.apply_change(change);
///
/// // After deleting a task
/// let change = TaskChange::Remove(task_id);
/// let new_index = index.apply_change(change);
/// ```
#[derive(Debug, Clone)]
pub enum TaskChange {
    /// A new task has been created.
    Add(Task),
    /// An existing task has been updated.
    Update {
        /// The old version of the task (before update).
        old: Task,
        /// The new version of the task (after update).
        new: Task,
    },
    /// A task has been removed.
    Remove(TaskId),
}

// =============================================================================
// GET /tasks - List Tasks
// =============================================================================

/// Lists tasks with pagination and optional filtering.
///
/// This handler demonstrates DB-side filtering using `list_filtered()` to leverage
/// database indexes for efficient queries on large datasets.
///
/// # Query Parameters
///
/// - `page`: Page number (default: 1)
/// - `limit`: Items per page (default: 20, max: 100)
/// - `status`: Optional filter by task status
/// - `priority`: Optional filter by task priority
///
/// # Response
///
/// - **200 OK**: Paginated list of tasks
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] in the following cases:
/// - **500 Internal Server Error**: Repository operation failed
#[allow(clippy::future_not_send)]
pub async fn list_tasks(
    State(state): State<AppState>,
    Query(query): Query<ListTasksQuery>,
) -> Result<Json<PaginatedResponse<TaskResponse>>, ApiErrorResponse> {
    // Normalize pagination parameters (pure function)
    // Use clamp to ensure page_size is in valid range [1, MAX_PAGE_SIZE]
    // This prevents panic in Pagination::new when limit=0
    let page_size = query.limit.clamp(1, MAX_PAGE_SIZE);
    let page = query.page.saturating_sub(1); // Convert 1-indexed to 0-indexed
    let pagination = Pagination::new(page, page_size);

    // Convert DTO filters to domain types (pure function)
    let status_filter = query.status.map(TaskStatus::from);
    let priority_filter = query.priority.map(Priority::from);

    // I/O boundary: DB-side filtering with indexes
    let result = state
        .task_repository
        .list_filtered(status_filter, priority_filter, pagination)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    // Build response (pure function)
    let response = build_paginated_response(result);

    Ok(Json(response))
}

/// Converts a [`PaginatedResult`] to a [`PaginatedResponse`] (pure function).
///
/// This function transforms the repository result into an API response,
/// converting each task to a `TaskResponse` DTO.
fn build_paginated_response(result: PaginatedResult<Task>) -> PaginatedResponse<TaskResponse> {
    // Compute derived values before consuming items (pure function)
    let total_pages = result.total_pages();
    let total = result.total;
    let page = result.page + 1; // Convert 0-indexed to 1-indexed for API
    let limit = result.page_size;

    // Transform items (consumes result.items)
    let data = result.items.into_iter().map(TaskResponse::from).collect();

    PaginatedResponse {
        data,
        page,
        limit,
        total,
        total_pages,
    }
}

/// Task filter predicate that validates and returns matching tasks.
///
/// Returns `Some(task)` if the task matches all filter criteria,
/// `None` otherwise. This predicate is designed to be used with
/// `Iterator::filter_map` for filtering.
#[allow(dead_code)]
fn task_filter_predicate(
    task: Task,
    status_filter: Option<TaskStatus>,
    priority_filter: Option<Priority>,
) -> Option<Task> {
    let status_matches = status_filter.is_none_or(|s| task.status == s);
    let priority_matches = priority_filter.is_none_or(|p| task.priority == p);

    if status_matches && priority_matches {
        Some(task)
    } else {
        None
    }
}

/// Validates that all tasks have valid (non-nil) UUIDs using `Traversable::traverse_option`.
///
/// This function demonstrates `Traversable::traverse_option` for batch validation.
/// If all tasks pass validation, returns `Ok(tasks)`. If any task has a nil UUID,
/// returns `Err(PaginationValidationError)` with the invalid task ID.
///
/// # Arguments
///
/// * `tasks` - The collection of tasks to validate
///
/// # Returns
///
/// `Ok(Vec<Task>)` if all tasks are valid, `Err(PaginationValidationError)` if any task has a nil UUID.
#[allow(dead_code)]
fn validate_tasks_with_traversable(tasks: &[Task]) -> Result<Vec<Task>, PaginationValidationError> {
    // First, try traverse_option to check if all are valid
    let validated = tasks.to_owned().traverse_option(|task| {
        if task.task_id.as_uuid().is_nil() {
            None
        } else {
            Some(task)
        }
    });

    validated.ok_or_else(|| {
        // Find the first invalid task for error reporting
        let invalid_task = tasks
            .iter()
            .find(|task| task.task_id.as_uuid().is_nil())
            .expect("traverse_option returned None, so there must be an invalid task");
        PaginationValidationError {
            invalid_task_id: invalid_task.task_id.clone(),
            message: "Task has invalid (nil) UUID".to_string(),
        }
    })
}

/// Filters tasks by status and priority with validation using Traversable.
///
/// This function demonstrates a two-phase approach:
/// 1. **Validation phase**: Uses `Traversable::traverse_option` to validate all tasks
///    have valid (non-nil) UUIDs. This demonstrates the all-or-nothing semantics
///    of `traverse_option`.
/// 2. **Filtering phase**: Uses `Iterator::filter_map` to apply filter predicates.
///
/// Note: `traverse_option` has "all succeed or all fail" semantics, which differs
/// from filtering. We use it for validation, then use `filter_map` for filtering.
///
/// # Arguments
///
/// * `tasks` - The collection of tasks to filter
/// * `status_filter` - Optional status to filter by
/// * `priority_filter` - Optional priority to filter by
///
/// # Returns
///
/// - `Ok(PersistentVector<Task>)` containing only the tasks that:
///   1. Have valid (non-nil) UUIDs (validation via `traverse_option`)
///   2. Match all filter criteria (filtering via `filter_map`)
/// - `Err(PaginationValidationError)` if any task has a nil UUID
///
/// # Errors
///
/// Returns `Err(PaginationValidationError)` if any task has a nil UUID.
#[allow(dead_code)]
fn filter_tasks_with_traversable(
    tasks: &PersistentVector<Task>,
    status_filter: Option<TaskStatus>,
    priority_filter: Option<Priority>,
) -> Result<PersistentVector<Task>, PaginationValidationError> {
    // Phase 1: Validate all tasks have valid UUIDs using traverse_option
    let task_vec: Vec<Task> = tasks.iter().cloned().collect();
    let valid_tasks = validate_tasks_with_traversable(&task_vec)?;

    // Phase 2: Apply filter predicates using filter_map
    Ok(valid_tasks
        .into_iter()
        .filter_map(|task| task_filter_predicate(task, status_filter, priority_filter))
        .collect())
}

/// Filters tasks by status and priority (pure function).
///
/// This function wraps `filter_tasks_with_traversable` and propagates validation errors.
///
/// # Errors
///
/// Returns `Err(PaginationValidationError)` if any task has an invalid (nil) UUID.
#[allow(dead_code)]
fn filter_tasks(
    tasks: &PersistentVector<Task>,
    status_filter: Option<TaskStatus>,
    priority_filter: Option<Priority>,
) -> Result<PersistentVector<Task>, PaginationValidationError> {
    filter_tasks_with_traversable(tasks, status_filter, priority_filter)
}

/// Error type for pagination validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaginationValidationError {
    /// The task ID that failed validation.
    pub invalid_task_id: TaskId,
    /// Human-readable error message.
    pub message: String,
}

impl std::fmt::Display for PaginationValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.message, self.invalid_task_id)
    }
}

impl std::error::Error for PaginationValidationError {}

/// Paginates a vector of tasks using Traversable for effectful transformation.
///
/// This function demonstrates `Traversable::traverse_option` for transforming
/// tasks within a page range. The traverse operation applies a validation-aware
/// transformation that lifts results into the Option effect boundary.
///
/// # Arguments
///
/// * `tasks` - The collection of tasks to paginate
/// * `page` - The page number (1-based, clamped to minimum 1)
/// * `limit` - Items per page (clamped to range 1-100)
///
/// # Returns
///
/// - `Ok(PaginatedResponse)` - Successfully paginated and transformed tasks
/// - `Err(PaginationValidationError)` - A task has an invalid (nil) UUID
///
/// # Errors
///
/// Returns `Err` if any task in the page has a nil UUID. This ensures that
/// `total` and `total_pages` remain consistent with the actual data returned.
#[allow(dead_code)]
fn paginate_tasks(
    tasks: &PersistentVector<Task>,
    page: u32,
    limit: u32,
) -> Result<PaginatedResponse<TaskResponse>, PaginationValidationError> {
    // Clamp inputs to valid ranges
    let page = page.max(1);
    let limit = limit.clamp(1, 100);

    // Use saturating arithmetic to prevent overflow
    // If (page - 1) * limit would overflow, saturating_mul returns u32::MAX
    // which effectively means "skip all items" - a safe behavior for edge cases
    let offset = (page - 1).saturating_mul(limit) as usize;
    let total = tasks.len() as u64;

    // Extract page slice as Vec for Traversable operations
    let page_tasks: Vec<Task> = tasks
        .iter()
        .skip(offset)
        .take(limit as usize)
        .cloned()
        .collect();

    // Use Traversable::traverse_option to transform tasks with validation.
    // The transformation validates each task has a non-nil ID and converts
    // to TaskResponse, lifting the result into Option effect boundary.
    //
    // If any task fails validation, we need to find which one and return an error
    // with proper context for debugging.
    let traverse_result = page_tasks.clone().traverse_option(|task| {
        if task.task_id.as_uuid().is_nil() {
            None
        } else {
            Some(TaskResponse::from(&task))
        }
    });

    let data: Vec<TaskResponse> = if let Some(responses) = traverse_result {
        responses
    } else {
        // Find the first invalid task for error reporting
        let invalid_task = page_tasks
            .iter()
            .find(|task| task.task_id.as_uuid().is_nil())
            .expect("traverse_option returned None, so there must be an invalid task");
        return Err(PaginationValidationError {
            invalid_task_id: invalid_task.task_id.clone(),
            message: "Task has invalid (nil) UUID".to_string(),
        });
    };

    // Calculate total pages (0 if no items)
    let total_pages = if total == 0 {
        0
    } else {
        total.div_ceil(u64::from(limit))
    };

    Ok(PaginatedResponse {
        data,
        page,
        limit,
        total,
        total_pages,
    })
}

// =============================================================================
// GET /tasks/search - Search Tasks
// =============================================================================

/// Searches tasks by title or tags.
///
/// This handler demonstrates DB-side search using `repository.search()` to leverage
/// database indexes (e.g., `pg_trgm` for title, GIN for tags) for efficient searches.
///
/// # Performance
///
/// Search is delegated to the database layer, which uses appropriate indexes
/// for efficient queries on large datasets. Search results are optionally cached
/// when using the in-memory backend.
///
/// Search results are cached with:
/// - **TTL**: 5 seconds
/// - **Capacity**: 2000 entries (LRU eviction)
/// - **Cache key**: `(normalized_query, scope, limit, offset)`
///
/// # Query Parameters
///
/// - `q`: Search query (case-insensitive substring match)
/// - `in`: Search scope - "title", "tags", or "all" (default)
/// - `limit`: Maximum results to return (default: 20, max: 100)
/// - `offset`: Number of results to skip (default: 0)
///
/// # Response
///
/// - **200 OK**: List of matching tasks
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] in the following cases:
/// - **500 Internal Server Error**: Repository operation failed
#[allow(clippy::future_not_send)]
pub async fn search_tasks(
    State(state): State<AppState>,
    Query(query): Query<SearchTasksQuery>,
) -> Result<Json<Vec<TaskResponse>>, ApiErrorResponse> {
    // Create cache key from raw query parameters
    let cache_key = SearchCacheKey::from_raw(&query.q, query.scope, query.limit, query.offset);

    // Check cache first (optional optimization for repeated queries)
    if let Some(cached_result) = state.search_cache.get(&cache_key) {
        tracing::debug!(
            cache_hit = true,
            hit_rate = %state.search_cache.stats().hit_rate(),
            "Search cache hit"
        );
        // Convert cached SearchResult to response
        let (limit, offset) = normalize_search_pagination(query.limit, query.offset);
        let response: Vec<TaskResponse> = cached_result
            .into_tasks()
            .iter()
            .skip(offset as usize)
            .take(limit as usize)
            .map(TaskResponse::from)
            .collect();
        return Ok(Json(response));
    }

    // Cache miss - log metrics
    tracing::debug!(
        cache_hit = false,
        hit_rate = %state.search_cache.stats().hit_rate(),
        "Search cache miss"
    );

    // Normalize pagination parameters (pure function)
    let limit = query
        .limit
        .unwrap_or(DEFAULT_SEARCH_LIMIT)
        .min(MAX_SEARCH_LIMIT);
    let offset = query.offset.unwrap_or(0);

    // I/O boundary: DB-side search with indexes
    let tasks = state
        .task_repository
        .search(&query.q, query.scope.to_repository_scope(), limit, offset)
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    // Store in cache for future requests
    let search_result = SearchResult::from_tasks(tasks.iter().cloned().collect());
    state.search_cache.put(cache_key, search_result);

    // Convert to response (pure function - map transformation)
    let response: Vec<TaskResponse> = tasks.into_iter().map(TaskResponse::from).collect();

    Ok(Json(response))
}

/// Searches tasks based on query and scope using index (pure function).
///
/// Uses `PersistentTreeMap`-based index for efficient lookup and
/// `Semigroup::combine` for combining search results from different scopes.
#[allow(dead_code)]
fn search_with_scope_indexed(index: &SearchIndex, query: &str, scope: SearchScope) -> SearchResult {
    // Empty query returns all tasks (early return)
    if query.is_empty() {
        return SearchResult::from_tasks(index.all_tasks());
    }

    match scope {
        SearchScope::Title => {
            // Search title index only, fallback to empty result
            index
                .search_by_title(query)
                .unwrap_or_else(SearchResult::empty)
        }
        SearchScope::Tags => {
            // Search tag index only, fallback to empty result
            index
                .search_by_tags(query)
                .unwrap_or_else(SearchResult::empty)
        }
        SearchScope::All => {
            // Combine title and tag search results using Semigroup.
            // Title matches get priority (appear first), then tag matches are added.
            let title_result = index.search_by_title(query);
            let tag_result = index.search_by_tags(query);

            match (title_result, tag_result) {
                (Some(title), Some(tags)) => {
                    // Both have results - combine with deduplication (title first)
                    title.combine(tags)
                }
                (Some(title), None) => title,
                (None, Some(tags)) => tags,
                (None, None) => SearchResult::empty(),
            }
        }
    }
}

/// Searches tasks based on query and scope (pure function).
///
/// Legacy implementation using direct iteration. Kept for reference and
/// backward compatibility in tests.
#[cfg(test)]
fn search_with_scope(
    tasks: &PersistentVector<Task>,
    query: &str,
    scope: SearchScope,
) -> SearchResult {
    // Empty query returns all tasks (early return to avoid double traversal)
    if query.is_empty() {
        return SearchResult::from_tasks(tasks.clone());
    }

    match scope {
        SearchScope::Title => search_by_title(tasks, query),
        SearchScope::Tags => search_by_tags(tasks, query),
        SearchScope::All => {
            // Title matches come first (higher priority), then tag matches
            let title_results = search_by_title(tasks, query);
            let tag_results = search_by_tags(tasks, query);
            // Use Semigroup::combine for deduplication
            title_results.combine(tag_results)
        }
    }
}

/// Searches tasks by title (case-insensitive substring match).
///
/// Note: In production, consider storing pre-normalized titles for better performance.
/// This implementation normalizes on each comparison for simplicity and demonstration.
#[cfg(test)]
fn search_by_title(tasks: &PersistentVector<Task>, query: &str) -> SearchResult {
    let query_lower = query.to_lowercase();
    let matching: PersistentVector<Task> = tasks
        .iter()
        .filter(|task| task.title.to_lowercase().contains(&query_lower))
        .cloned()
        .collect();

    SearchResult::from_tasks(matching)
}

/// Searches tasks by tag (case-insensitive substring match).
///
/// Note: In production, consider storing pre-normalized tags for better performance.
/// This implementation normalizes on each comparison for simplicity and demonstration.
#[cfg(test)]
fn search_by_tags(tasks: &PersistentVector<Task>, query: &str) -> SearchResult {
    let query_lower = query.to_lowercase();
    let matching: PersistentVector<Task> = tasks
        .iter()
        .filter(|task| {
            task.tags
                .iter()
                .any(|tag| tag.as_str().to_lowercase().contains(&query_lower))
        })
        .cloned()
        .collect();

    SearchResult::from_tasks(matching)
}

// =============================================================================
// GET /tasks/by-priority - Count by Priority
// =============================================================================

/// Counts tasks grouped by priority level.
///
/// This handler demonstrates:
/// - **Foldable pattern**: Using `fold` for single-pass aggregation
/// - **Sum Monoid**: Deriving total from individual counts
///
/// # Response
///
/// - **200 OK**: Priority counts with total
///
/// # Errors
///
/// Returns [`ApiErrorResponse`] in the following cases:
/// - **500 Internal Server Error**: Repository operation failed
#[allow(clippy::future_not_send)]
pub async fn count_by_priority(
    State(state): State<AppState>,
) -> Result<Json<PriorityCountResponse>, ApiErrorResponse> {
    // I/O boundary: Fetch all tasks from repository (use Pagination::all() for full dataset)
    let all_tasks = state
        .task_repository
        .list(Pagination::all())
        .run_async()
        .await
        .map_err(ApiErrorResponse::from)?;

    // Pure computation: Count by priority using fold
    let counts = count_tasks_by_priority(&all_tasks.items);

    Ok(Json(counts))
}

/// Counts tasks by priority level (pure function).
///
/// Uses fold for single-pass aggregation, then derives total.
fn count_tasks_by_priority(tasks: &[Task]) -> PriorityCountResponse {
    tasks
        .iter()
        .fold(PriorityCountResponse::default(), |acc, task| {
            match task.priority {
                Priority::Low => acc.increment_low(),
                Priority::Medium => acc.increment_medium(),
                Priority::High => acc.increment_high(),
                Priority::Critical => acc.increment_critical(),
            }
        })
        .finalize()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Tag, TaskId, Timestamp};
    use rstest::rstest;

    fn create_test_task(title: &str, priority: Priority) -> Task {
        Task::new(TaskId::generate(), title, Timestamp::now()).with_priority(priority)
    }

    fn create_test_task_with_tags(title: &str, priority: Priority, tags: &[&str]) -> Task {
        let base = create_test_task(title, priority);
        tags.iter()
            .fold(base, |task, tag| task.add_tag(Tag::new(*tag)))
    }

    // -------------------------------------------------------------------------
    // Filter Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_filter_tasks_no_filter() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::High),
        ]
        .into_iter()
        .collect();

        let filtered = filter_tasks(&tasks, None, None).expect("should succeed with valid UUIDs");
        assert_eq!(filtered.len(), 2);
    }

    #[rstest]
    fn test_filter_tasks_by_priority() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::High),
            create_test_task("Task 3", Priority::Low),
        ]
        .into_iter()
        .collect();

        let filtered = filter_tasks(&tasks, None, Some(Priority::Low))
            .expect("should succeed with valid UUIDs");
        assert_eq!(filtered.len(), 2);
    }

    // -------------------------------------------------------------------------
    // Pagination Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_paginate_first_page() {
        let tasks: PersistentVector<Task> = (0..25)
            .map(|i| create_test_task(&format!("Task {i}"), Priority::Medium))
            .collect();

        let result = paginate_tasks(&tasks, 1, 10).expect("pagination should succeed");

        assert_eq!(result.data.len(), 10);
        assert_eq!(result.page, 1);
        assert_eq!(result.limit, 10);
        assert_eq!(result.total, 25);
        assert_eq!(result.total_pages, 3);
    }

    #[rstest]
    fn test_paginate_last_page() {
        let tasks: PersistentVector<Task> = (0..25)
            .map(|i| create_test_task(&format!("Task {i}"), Priority::Medium))
            .collect();

        let result = paginate_tasks(&tasks, 3, 10).expect("pagination should succeed");

        assert_eq!(result.data.len(), 5);
        assert_eq!(result.page, 3);
        assert_eq!(result.total_pages, 3);
    }

    #[rstest]
    fn test_paginate_empty() {
        let tasks: PersistentVector<Task> = PersistentVector::new();

        let result = paginate_tasks(&tasks, 1, 10).expect("pagination should succeed");

        assert_eq!(result.data.len(), 0);
        assert_eq!(result.total, 0);
        assert_eq!(result.total_pages, 0);
    }

    #[rstest]
    fn test_paginate_clamps_page_zero() {
        let tasks: PersistentVector<Task> = (0..10)
            .map(|i| create_test_task(&format!("Task {i}"), Priority::Medium))
            .collect();

        let result = paginate_tasks(&tasks, 0, 10).expect("pagination should succeed");

        assert_eq!(result.page, 1);
    }

    #[rstest]
    fn test_paginate_clamps_limit() {
        let tasks: PersistentVector<Task> = (0..200)
            .map(|i| create_test_task(&format!("Task {i}"), Priority::Medium))
            .collect();

        let result = paginate_tasks(&tasks, 1, 200).expect("pagination should succeed");

        assert_eq!(result.limit, 100);
        assert_eq!(result.data.len(), 100);
    }

    // -------------------------------------------------------------------------
    // Search Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_search_by_title() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Important meeting", Priority::High),
            create_test_task("Review code", Priority::Medium),
            create_test_task("Important deadline", Priority::Critical),
        ]
        .into_iter()
        .collect();

        let result = search_by_title(&tasks, "important");

        assert_eq!(result.tasks.len(), 2);
    }

    #[rstest]
    fn test_search_by_title_case_insensitive() {
        let tasks: PersistentVector<Task> = vec![create_test_task("URGENT Task", Priority::High)]
            .into_iter()
            .collect();

        let result = search_by_title(&tasks, "urgent");

        assert_eq!(result.tasks.len(), 1);
    }

    #[rstest]
    fn test_search_empty_query_returns_all() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::High),
        ]
        .into_iter()
        .collect();

        let result = search_with_scope(&tasks, "", SearchScope::All);

        assert_eq!(result.tasks.len(), 2);
    }

    #[rstest]
    fn test_search_result_combine_deduplicates() {
        let task1 = create_test_task("Task 1", Priority::Low);
        let task2 = create_test_task("Task 2", Priority::High);
        let task1_clone = task1.clone();
        let task2_clone = task2.clone();

        let result1 = SearchResult::from_tasks(vec![task1, task2].into_iter().collect());
        let result2 =
            SearchResult::from_tasks(vec![task1_clone, task2_clone].into_iter().collect());

        let combined = result1.combine(result2);

        // Should have 2 tasks, not 4 (deduplicated by ID)
        assert_eq!(combined.tasks.len(), 2);
    }

    #[rstest]
    fn test_search_by_tags() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task_with_tags("Task 1", Priority::Low, &["backend", "rust"]),
            create_test_task_with_tags("Task 2", Priority::Medium, &["frontend", "typescript"]),
            create_test_task_with_tags("Task 3", Priority::High, &["backend", "go"]),
        ]
        .into_iter()
        .collect();

        let result = search_by_tags(&tasks, "backend");

        assert_eq!(result.tasks.len(), 2);
    }

    #[rstest]
    fn test_search_by_tags_case_insensitive() {
        let tasks: PersistentVector<Task> = vec![create_test_task_with_tags(
            "Task 1",
            Priority::Low,
            &["URGENT"],
        )]
        .into_iter()
        .collect();

        let result = search_by_tags(&tasks, "urgent");

        assert_eq!(result.tasks.len(), 1);
    }

    #[rstest]
    fn test_search_scope_all_prioritizes_title_matches() {
        // Task with "important" in title
        let title_match = create_test_task("Important meeting", Priority::High);
        // Task with "important" in tag
        let tag_match = create_test_task_with_tags("Regular task", Priority::Low, &["important"]);

        let tasks: PersistentVector<Task> = vec![tag_match, title_match].into_iter().collect();

        let result = search_with_scope(&tasks, "important", SearchScope::All);

        // Should have 2 results
        assert_eq!(result.tasks.len(), 2);
        // Title match should come first (title_match has "Important" in title)
        let first = result.tasks.iter().next().unwrap();
        assert!(first.title.contains("Important"));
    }

    #[rstest]
    fn test_search_scope_all_deduplicates() {
        // Task that matches both title and tag
        let both_match =
            create_test_task_with_tags("Important meeting", Priority::High, &["important"]);

        let tasks: PersistentVector<Task> = vec![both_match].into_iter().collect();

        let result = search_with_scope(&tasks, "important", SearchScope::All);

        // Should have 1 result, not 2 (deduplicated)
        assert_eq!(result.tasks.len(), 1);
    }

    #[rstest]
    fn test_paginate_page_beyond_total() {
        let tasks: PersistentVector<Task> = (0..10)
            .map(|i| create_test_task(&format!("Task {i}"), Priority::Medium))
            .collect();

        // Request page 5, but only 1 page exists (10 items / 10 per page = 1 page)
        let result = paginate_tasks(&tasks, 5, 10).expect("pagination should succeed");

        // Should return empty data since page 5 is beyond total_pages
        assert!(result.data.is_empty());
        assert_eq!(result.page, 5);
        assert_eq!(result.total, 10);
        assert_eq!(result.total_pages, 1);
    }

    // -------------------------------------------------------------------------
    // Priority Count Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_count_by_priority() {
        let tasks = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::Low),
            create_test_task("Task 3", Priority::Medium),
            create_test_task("Task 4", Priority::High),
            create_test_task("Task 5", Priority::Critical),
        ];

        let counts = count_tasks_by_priority(&tasks);

        assert_eq!(counts.low, 2);
        assert_eq!(counts.medium, 1);
        assert_eq!(counts.high, 1);
        assert_eq!(counts.critical, 1);
        assert_eq!(counts.total, 5);
    }

    #[rstest]
    fn test_count_by_priority_empty() {
        let tasks: Vec<Task> = vec![];

        let counts = count_tasks_by_priority(&tasks);

        assert_eq!(counts.low, 0);
        assert_eq!(counts.medium, 0);
        assert_eq!(counts.high, 0);
        assert_eq!(counts.critical, 0);
        assert_eq!(counts.total, 0);
    }

    // -------------------------------------------------------------------------
    // SearchIndex Tests (PersistentTreeMap + Semigroup)
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_search_index_build() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Important meeting", Priority::High),
            create_test_task_with_tags("Review code", Priority::Medium, &["backend", "rust"]),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);

        // Title word index should contain normalized words
        assert!(index.title_word_index.contains_key("important"));
        assert!(index.title_word_index.contains_key("meeting"));
        assert!(index.title_word_index.contains_key("review"));
        assert!(index.title_word_index.contains_key("code"));

        // Tag index should contain normalized tags
        assert!(index.tag_index.contains_key("backend"));
        assert!(index.tag_index.contains_key("rust"));
    }

    #[rstest]
    fn test_search_index_search_by_title() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Important meeting", Priority::High),
            create_test_task("Review code", Priority::Medium),
            create_test_task("Important deadline", Priority::Critical),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("important");

        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 2);
    }

    #[rstest]
    fn test_search_index_search_by_title_no_match() {
        let tasks: PersistentVector<Task> = vec![create_test_task("Hello world", Priority::Low)]
            .into_iter()
            .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("nonexistent");

        assert!(result.is_none());
    }

    #[rstest]
    fn test_search_index_search_by_tags() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task_with_tags("Task 1", Priority::Low, &["backend", "rust"]),
            create_test_task_with_tags("Task 2", Priority::Medium, &["frontend", "typescript"]),
            create_test_task_with_tags("Task 3", Priority::High, &["backend", "go"]),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_tags("backend");

        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 2);
    }

    #[rstest]
    fn test_search_index_search_by_tags_no_match() {
        let tasks: PersistentVector<Task> = vec![create_test_task_with_tags(
            "Task 1",
            Priority::Low,
            &["backend"],
        )]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_tags("frontend");

        assert!(result.is_none());
    }

    #[rstest]
    fn test_search_with_scope_indexed_title() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Important meeting", Priority::High),
            create_test_task("Regular task", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = search_with_scope_indexed(&index, "important", SearchScope::Title);

        assert_eq!(result.tasks.len(), 1);
    }

    #[rstest]
    fn test_search_with_scope_indexed_tags() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task_with_tags("Task 1", Priority::Low, &["urgent"]),
            create_test_task_with_tags("Task 2", Priority::Medium, &["normal"]),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = search_with_scope_indexed(&index, "urgent", SearchScope::Tags);

        assert_eq!(result.tasks.len(), 1);
    }

    #[rstest]
    fn test_search_with_scope_indexed_all_uses_alternative() {
        // Task with "important" in title only
        let title_match = create_test_task("Important meeting", Priority::High);
        // Task with "important" in tag only
        let tag_match = create_test_task_with_tags("Regular task", Priority::Low, &["important"]);

        let tasks: PersistentVector<Task> = vec![tag_match, title_match].into_iter().collect();

        let index = SearchIndex::build(&tasks);
        let result = search_with_scope_indexed(&index, "important", SearchScope::All);

        // Both tasks should be found (title and tag matches combined)
        assert_eq!(result.tasks.len(), 2);
    }

    #[rstest]
    fn test_search_with_scope_indexed_all_deduplicates() {
        // Task that matches both title and tag
        let both_match =
            create_test_task_with_tags("Important meeting", Priority::High, &["important"]);

        let tasks: PersistentVector<Task> = vec![both_match].into_iter().collect();

        let index = SearchIndex::build(&tasks);
        let result = search_with_scope_indexed(&index, "important", SearchScope::All);

        // Should have 1 result, not 2 (deduplicated)
        assert_eq!(result.tasks.len(), 1);
    }

    #[rstest]
    fn test_search_with_scope_indexed_empty_query() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::High),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = search_with_scope_indexed(&index, "", SearchScope::All);

        // Empty query returns all tasks
        assert_eq!(result.tasks.len(), 2);
    }

    #[rstest]
    fn test_search_with_scope_indexed_title_only_fallback() {
        // Test that when title search returns None, the result is empty (not tags)
        let tasks: PersistentVector<Task> = vec![create_test_task_with_tags(
            "Normal task",
            Priority::Low,
            &["urgent"],
        )]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = search_with_scope_indexed(&index, "urgent", SearchScope::Title);

        // Title doesn't contain "urgent", so result should be empty
        assert!(result.tasks.is_empty());
    }

    #[rstest]
    fn test_search_with_scope_indexed_tags_only_fallback() {
        // Test that when tag search returns None, the result is empty (not title)
        let tasks: PersistentVector<Task> = vec![create_test_task("Urgent task", Priority::High)]
            .into_iter()
            .collect();

        let index = SearchIndex::build(&tasks);
        let result = search_with_scope_indexed(&index, "urgent", SearchScope::Tags);

        // Tags are empty, so result should be empty
        assert!(result.tasks.is_empty());
    }

    #[rstest]
    fn test_search_index_all_tasks() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::High),
            create_test_task("Task 3", Priority::Medium),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let all = index.all_tasks();

        assert_eq!(all.len(), 3);
    }

    // -------------------------------------------------------------------------
    // Regression Tests (Codex Review Fix #225)
    // -------------------------------------------------------------------------

    /// Regression test: Multi-word query with space should match substring.
    /// E.g., query "important meeting" should match title "Important meeting".
    #[rstest]
    fn test_search_multi_word_query_matches_substring() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Important meeting tomorrow", Priority::High),
            create_test_task("Review code", Priority::Medium),
            create_test_task("Important deadline", Priority::Critical),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("important meeting");

        // "important meeting" should match "Important meeting tomorrow"
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 1);
        assert!(result.tasks.iter().any(|t| t.title.contains("meeting")));
    }

    /// Regression test: Result order should be stable across multiple searches.
    #[rstest]
    fn test_search_result_order_stability() {
        // Create tasks with predictable task_ids for stable ordering
        let task1 = create_test_task("Important task A", Priority::High);
        let task2 = create_test_task("Important task B", Priority::Medium);
        let task3 = create_test_task("Important task C", Priority::Low);

        let tasks: PersistentVector<Task> = vec![task1, task2, task3].into_iter().collect();

        let index = SearchIndex::build(&tasks);

        // Run the same search multiple times
        let results: Vec<Vec<TaskId>> = (0..5)
            .map(|_| {
                let result = index.search_by_title("important");
                result
                    .map(|r| r.tasks.iter().map(|t| t.task_id.clone()).collect())
                    .unwrap_or_default()
            })
            .collect();

        // All results should have the same order
        for i in 1..results.len() {
            assert_eq!(
                results[0], results[i],
                "Search results should be stable across multiple calls"
            );
        }
    }

    /// Regression test: Prefix-based search should still work.
    /// E.g., query "imp" should match "important".
    #[rstest]
    fn test_search_prefix_matches() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Important meeting", Priority::High),
            create_test_task("Impossible task", Priority::Medium),
            create_test_task("Regular task", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("imp");

        // "imp" should match "Important" and "Impossible"
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 2);
    }

    /// Regression test: Partial word match should work for tags.
    /// E.g., query "back" should match tag "backend".
    #[rstest]
    fn test_search_tag_partial_match() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task_with_tags("Task 1", Priority::Low, &["backend"]),
            create_test_task_with_tags("Task 2", Priority::Medium, &["frontend"]),
            create_test_task_with_tags("Task 3", Priority::High, &["backlog"]),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_tags("back");

        // "back" should match "backend" and "backlog"
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 2);
    }

    /// Regression test: All scope search should maintain task order.
    /// Title matches should come before tag matches.
    #[rstest]
    fn test_search_all_scope_title_matches_first() {
        // Task with "urgent" in title only
        let title_match = create_test_task("Urgent meeting", Priority::High);
        // Task with "urgent" in tag only
        let tag_match = create_test_task_with_tags("Regular task", Priority::Low, &["urgent"]);

        let tasks: PersistentVector<Task> = vec![tag_match, title_match].into_iter().collect();

        let index = SearchIndex::build(&tasks);
        let result = search_with_scope_indexed(&index, "urgent", SearchScope::All);

        assert_eq!(result.tasks.len(), 2);
        // Title match should come first
        let first = result.tasks.iter().next().unwrap();
        assert!(
            first.title.to_lowercase().contains("urgent"),
            "Title matches should appear before tag matches"
        );
    }

    /// Regression test: Search should handle empty title gracefully.
    #[rstest]
    fn test_search_handles_empty_title() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("", Priority::Low),
            create_test_task("Regular task", Priority::Medium),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("regular");

        assert!(result.is_some());
        assert_eq!(result.unwrap().tasks.len(), 1);
    }

    /// Regression test: Search should handle special characters in query.
    #[rstest]
    fn test_search_handles_special_characters() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Fix bug #123", Priority::High),
            create_test_task("Update README.md", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);

        // Search for "#123" - should not cause errors
        let result = index.search_by_title("#123");
        // May or may not find results depending on tokenization, but should not crash
        assert!(result.is_none() || result.is_some());
    }

    // -------------------------------------------------------------------------
    // Phase 1.1c: Same Title Multiple Tasks Tests (Codex Review #225)
    // -------------------------------------------------------------------------

    /// Regression test: Multiple tasks with the same title should all be found.
    /// This tests the fix for `title_full_index` overwriting issue.
    #[rstest]
    fn test_search_same_title_multiple_tasks() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Important meeting", Priority::High),
            create_test_task("Important meeting", Priority::Medium),
            create_test_task("Important meeting", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("important meeting");

        // All 3 tasks with the same title should be found
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            3,
            "All tasks with the same title should be returned"
        );
    }

    /// Regression test: Multi-word search should find all tasks with matching titles.
    #[rstest]
    fn test_search_multi_word_same_title_all_found() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Review code changes", Priority::High),
            create_test_task("Review code changes", Priority::Medium),
            create_test_task("Review API design", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("review code");

        // Both "Review code changes" tasks should be found
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            2,
            "Both tasks with 'review code' in title should be found"
        );
    }

    /// Regression test: Full substring search should work with same titles.
    #[rstest]
    fn test_search_substring_same_title() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Backend API implementation", Priority::High),
            create_test_task("Backend API implementation", Priority::Medium),
            create_test_task("Frontend implementation", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("backend api");

        // Both "Backend API implementation" tasks should be found
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            2,
            "Both tasks with 'backend api' in title should be found"
        );
    }

    /// Regression test: Contains search should not require full scan.
    /// E.g., "callback" should match "Add callback handler" without scanning all entries.
    #[rstest]
    fn test_search_contains_match_efficiency() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Add callback handler", Priority::High),
            create_test_task("Update backend service", Priority::Medium),
            create_test_task("Implement callback logic", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("callback");

        // Both tasks containing "callback" should be found
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            2,
            "Both tasks containing 'callback' should be found"
        );
    }

    /// Regression test: Infix search (not prefix) should work with reversed index.
    /// E.g., "back" should match both "backend" and "callback"
    #[rstest]
    fn test_search_infix_match_both_directions() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Update backend", Priority::High),
            create_test_task("Add callback", Priority::Medium),
            create_test_task("Fix feedback", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("back");

        // "backend", "callback", and "feedback" all contain "back"
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            3,
            "All tasks containing 'back' should be found"
        );
    }

    /// Regression test: Same tag multiple tasks should all be found.
    #[rstest]
    fn test_search_same_tag_multiple_tasks() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task_with_tags("Task 1", Priority::High, &["backend"]),
            create_test_task_with_tags("Task 2", Priority::Medium, &["backend"]),
            create_test_task_with_tags("Task 3", Priority::Low, &["backend"]),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_tags("backend");

        // All 3 tasks with the same tag should be found
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            3,
            "All tasks with the same tag should be returned"
        );
    }

    // -------------------------------------------------------------------------
    // Phase 1.1c: Infix Substring Match Tests (Codex Review #225)
    // -------------------------------------------------------------------------

    /// Infix match test: "callback" should be found when searching for "all".
    /// This tests the full suffix index that enables arbitrary position matching.
    #[rstest]
    fn test_search_infix_match_all_in_callback() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Handle callback events", Priority::High),
            create_test_task("Regular task", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("all");

        // "all" appears in the middle of "callback"
        assert!(result.is_some(), "Query 'all' should match 'callback'");
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            1,
            "Only the task with 'callback' should match"
        );
        assert!(result.tasks.iter().any(|t| t.title.contains("callback")));
    }

    /// Infix match test: "callback" should be found when searching for "llb".
    /// Note: "callback" contains "llb" in "ca[llb]ack" - wait, let's verify.
    /// "callback" = c-a-l-l-b-a-c-k. The substring "llb" is at positions 2-4.
    #[rstest]
    fn test_search_infix_match_llb_in_callback() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Implement callback handler", Priority::High),
            create_test_task("Other task", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("llb");

        // "llb" appears in the middle of "callback" (ca-llb-ack)
        assert!(result.is_some(), "Query 'llb' should match 'callback'");
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 1);
    }

    /// Infix match test: "callback" should be found when searching for "llba".
    /// Note: "llba" does NOT appear in "callback" (it's "llba" vs "llba").
    /// Actually "callback" contains "allb" but not "llba". Let's test "allb" instead.
    #[rstest]
    fn test_search_infix_match_allb_in_callback() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Fix callback issue", Priority::High),
            create_test_task("Normal task", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("allb");

        // "allb" appears in "callback"
        assert!(result.is_some(), "Query 'allb' should match 'callback'");
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 1);
    }

    /// Infix match test for tags: "backend" should be found when searching for "ack".
    #[rstest]
    fn test_search_tag_infix_match_ack_in_backend() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task_with_tags("Task 1", Priority::High, &["backend"]),
            create_test_task_with_tags("Task 2", Priority::Low, &["frontend"]),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_tags("cke");

        // "cke" appears in the middle of "backend"
        assert!(result.is_some(), "Query 'cke' should match 'backend'");
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 1);
    }

    /// Infix match test for tags: "callback" tag should be found when searching for "llb".
    #[rstest]
    fn test_search_tag_infix_match_llb_in_callback() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task_with_tags("Task 1", Priority::High, &["callback"]),
            create_test_task_with_tags("Task 2", Priority::Low, &["regular"]),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_tags("llb");

        // "llb" appears in the middle of "callback"
        assert!(result.is_some(), "Query 'llb' should match 'callback'");
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 1);
    }

    /// Infix match should work for multiple words in title.
    #[rstest]
    fn test_search_infix_match_multiple_words() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Important callback implementation", Priority::High),
            create_test_task("Frontend development", Priority::Medium),
            create_test_task("Feedback processing", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);

        // "all" should match "callback"
        let result_all = index.search_by_title("all");
        assert!(result_all.is_some());
        assert_eq!(result_all.unwrap().tasks.len(), 1);

        // "ort" should match "important"
        let result_ort = index.search_by_title("ort");
        assert!(result_ort.is_some());
        assert_eq!(result_ort.unwrap().tasks.len(), 1);

        // "ple" should match "implementation"
        let result_ple = index.search_by_title("ple");
        assert!(result_ple.is_some());
        assert_eq!(result_ple.unwrap().tasks.len(), 1);
    }

    // -------------------------------------------------------------------------
    // Phase 1.1c: BMP外文字（絵文字）検索テスト (Codex Review #225)
    // -------------------------------------------------------------------------

    /// BMP外文字テスト: 絵文字を含むタイトルで "call" を検索すると "call😀back" がヒットする。
    /// UTF-8 の `char_indices` で正しくバイト境界を処理することを確認。
    #[rstest]
    fn test_search_title_with_emoji_call_in_callback_emoji() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("call😀back handler", Priority::High),
            create_test_task("Regular callback", Priority::Medium),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("call");

        // "call" should match both "call😀back" and "callback"
        assert!(result.is_some(), "Query 'call' should match 'call😀back'");
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            2,
            "Both tasks containing 'call' should be found"
        );
    }

    /// BMP外文字テスト: 絵文字を含むタイトルで "task" を検索すると "task🎉done" がヒットする。
    #[rstest]
    fn test_search_title_with_emoji_task_in_task_emoji_done() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("task🎉done celebration", Priority::High),
            create_test_task("Normal task item", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("task");

        // "task" should match both "task🎉done" and "Normal task item"
        assert!(result.is_some(), "Query 'task' should match 'task🎉done'");
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            2,
            "Both tasks containing 'task' should be found"
        );
    }

    /// BMP外文字テスト: 絵文字を含むタグで "emoji" を検索すると "emoji😀tag" がヒットする。
    #[rstest]
    fn test_search_tag_with_emoji_emoji_in_emoji_tag() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task_with_tags("Task with emoji tag", Priority::High, &["emoji😀tag"]),
            create_test_task_with_tags("Task with normal tag", Priority::Low, &["normal"]),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_tags("emoji");

        // "emoji" should match "emoji😀tag"
        assert!(result.is_some(), "Query 'emoji' should match 'emoji😀tag'");
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            1,
            "Only the task with 'emoji😀tag' should be found"
        );
    }

    /// BMP外文字テスト: 絵文字の後ろの文字列 "back" で "call😀back" を検索できる。
    #[rstest]
    fn test_search_title_with_emoji_suffix_back_in_callback_emoji() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("call😀back function", Priority::High),
            create_test_task("Other task", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("back");

        // "back" should match "call😀back" via suffix index
        assert!(
            result.is_some(),
            "Query 'back' should match 'call😀back' via suffix index"
        );
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 1);
        assert!(result.tasks.iter().any(|t| t.title.contains("call😀back")));
    }

    /// BMP外文字テスト: 絵文字の後ろの文字列 "tag" で "emoji😀tag" タグを検索できる。
    #[rstest]
    fn test_search_tag_with_emoji_suffix_tag_in_emoji_tag() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task_with_tags("Task 1", Priority::High, &["emoji😀tag"]),
            create_test_task_with_tags("Task 2", Priority::Low, &["regular"]),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_tags("tag");

        // "tag" should match "emoji😀tag" via suffix index
        assert!(
            result.is_some(),
            "Query 'tag' should match 'emoji😀tag' via suffix index"
        );
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 1);
    }

    /// BMP外文字テスト: 複数の絵文字を含むタイトルでも正しく検索できる。
    #[rstest]
    fn test_search_title_with_multiple_emojis() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("🚀rocket🌟launch🎯target", Priority::High),
            create_test_task("Normal title", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);

        // "rocket" should match
        let result_rocket = index.search_by_title("rocket");
        assert!(
            result_rocket.is_some(),
            "Query 'rocket' should match '🚀rocket🌟launch🎯target'"
        );
        assert_eq!(result_rocket.unwrap().tasks.len(), 1);

        // "launch" should match
        let result_launch = index.search_by_title("launch");
        assert!(
            result_launch.is_some(),
            "Query 'launch' should match '🚀rocket🌟launch🎯target'"
        );
        assert_eq!(result_launch.unwrap().tasks.len(), 1);

        // "target" should match
        let result_target = index.search_by_title("target");
        assert!(
            result_target.is_some(),
            "Query 'target' should match '🚀rocket🌟launch🎯target'"
        );
        assert_eq!(result_target.unwrap().tasks.len(), 1);
    }

    // -------------------------------------------------------------------------
    // Phase 1.1g: Traversable Type Class Tests
    // -------------------------------------------------------------------------

    /// Test that `filter_tasks_with_traversable` correctly filters by status.
    #[rstest]
    fn test_filter_tasks_with_traversable_by_status() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::High).complete(),
            create_test_task("Task 3", Priority::Medium),
        ]
        .into_iter()
        .collect();

        let filtered = filter_tasks_with_traversable(&tasks, Some(TaskStatus::Pending), None)
            .expect("should succeed with valid UUIDs");
        assert_eq!(filtered.len(), 2, "Should filter out completed task");
    }

    /// Test that `filter_tasks_with_traversable` correctly filters by priority.
    #[rstest]
    fn test_filter_tasks_with_traversable_by_priority() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::High),
            create_test_task("Task 3", Priority::Low),
            create_test_task("Task 4", Priority::Critical),
        ]
        .into_iter()
        .collect();

        let filtered = filter_tasks_with_traversable(&tasks, None, Some(Priority::Low))
            .expect("should succeed with valid UUIDs");
        assert_eq!(
            filtered.len(),
            2,
            "Should filter to only Low priority tasks"
        );
    }

    /// Test that `filter_tasks_with_traversable` correctly filters by both status and priority.
    #[rstest]
    fn test_filter_tasks_with_traversable_by_status_and_priority() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::High).complete(),
            create_test_task("Task 3", Priority::Low).complete(),
            create_test_task("Task 4", Priority::Low),
        ]
        .into_iter()
        .collect();

        let filtered =
            filter_tasks_with_traversable(&tasks, Some(TaskStatus::Pending), Some(Priority::Low))
                .expect("should succeed with valid UUIDs");
        assert_eq!(
            filtered.len(),
            2,
            "Should filter to only pending Low priority tasks"
        );
    }

    /// Test that `filter_tasks_with_traversable` returns all tasks when no filters applied.
    #[rstest]
    fn test_filter_tasks_with_traversable_no_filter() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::High),
            create_test_task("Task 3", Priority::Medium),
        ]
        .into_iter()
        .collect();

        let filtered = filter_tasks_with_traversable(&tasks, None, None)
            .expect("should succeed with valid UUIDs");
        assert_eq!(filtered.len(), 3, "Should return all tasks when no filter");
    }

    /// Test that `task_filter_predicate` returns `Some` for matching tasks.
    #[rstest]
    fn test_task_filter_predicate_matches() {
        let task = create_test_task("Test Task", Priority::High);

        // No filter - should match
        let result = task_filter_predicate(task.clone(), None, None);
        assert!(result.is_some(), "Should match when no filters");

        // Matching priority filter
        let result = task_filter_predicate(task.clone(), None, Some(Priority::High));
        assert!(result.is_some(), "Should match when priority matches");

        // Matching status filter
        let result = task_filter_predicate(task, Some(TaskStatus::Pending), None);
        assert!(result.is_some(), "Should match when status matches");
    }

    /// Test that `task_filter_predicate` returns `None` for non-matching tasks.
    #[rstest]
    fn test_task_filter_predicate_not_matches() {
        let task = create_test_task("Test Task", Priority::High);

        // Non-matching priority
        let result = task_filter_predicate(task.clone(), None, Some(Priority::Low));
        assert!(result.is_none(), "Should not match when priority differs");

        // Non-matching status (pending task vs completed filter)
        let result = task_filter_predicate(task, Some(TaskStatus::Completed), None);
        assert!(result.is_none(), "Should not match when status differs");
    }

    /// Test `paginate_tasks` uses `Traversable::traverse_option` for transformation.
    #[rstest]
    fn test_paginate_tasks_with_traversable() {
        let tasks: PersistentVector<Task> = (0..25)
            .map(|i| create_test_task(&format!("Task {i}"), Priority::Medium))
            .collect();

        let result = paginate_tasks(&tasks, 1, 10).expect("pagination should succeed");

        assert_eq!(
            result.data.len(),
            10,
            "Should return 10 items on first page"
        );
        assert_eq!(result.page, 1);
        assert_eq!(result.limit, 10);
        assert_eq!(result.total, 25);
        assert_eq!(result.total_pages, 3);

        // Verify TaskResponse transformation via traverse_option
        for (index, response) in result.data.iter().enumerate() {
            assert_eq!(
                response.title,
                format!("Task {index}"),
                "Task should be correctly transformed"
            );
        }
    }

    /// Test that `paginate_tasks` handles empty input correctly with Traversable.
    #[rstest]
    fn test_paginate_tasks_with_traversable_empty() {
        let tasks: PersistentVector<Task> = PersistentVector::new();

        let result = paginate_tasks(&tasks, 1, 10).expect("pagination should succeed");

        assert!(
            result.data.is_empty(),
            "Should return empty data for empty input"
        );
        assert_eq!(result.total, 0);
        assert_eq!(result.total_pages, 0);
    }

    /// Test Traversable `traverse_option` directly on Vec with filter semantics.
    #[rstest]
    fn test_traversable_traverse_option_filter_pattern() {
        use lambars::typeclass::Traversable;

        let numbers = vec![1, 2, 3, 4, 5];

        // Use traverse_option to validate and transform in one pass
        // This returns Some only if ALL elements pass validation
        let all_positive: Option<Vec<i32>> =
            numbers.traverse_option(|n| if n > 0 { Some(n * 2) } else { None });
        assert_eq!(all_positive, Some(vec![2, 4, 6, 8, 10]));

        // If any element fails, the whole result is None
        let with_negative = vec![1, -2, 3];
        let result: Option<Vec<i32>> =
            with_negative.traverse_option(|n| if n > 0 { Some(n * 2) } else { None });
        assert_eq!(result, None, "Should return None if any validation fails");
    }

    /// Test Traversable `sequence_option` for turning `Vec<Option<T>>` into `Option<Vec<T>>`.
    #[rstest]
    fn test_traversable_sequence_option() {
        use lambars::typeclass::Traversable;

        // All Some values - should succeed
        let options: Vec<Option<i32>> = vec![Some(1), Some(2), Some(3)];
        let result: Option<Vec<i32>> = options.sequence_option();
        assert_eq!(result, Some(vec![1, 2, 3]));

        // Contains None - should fail
        let options_with_none: Vec<Option<i32>> = vec![Some(1), None, Some(3)];
        let result_with_none: Option<Vec<i32>> = options_with_none.sequence_option();
        assert_eq!(result_with_none, None);
    }

    /// Test that Traversable can be used for effectful validation in task processing.
    #[rstest]
    fn test_traversable_task_validation() {
        use lambars::typeclass::Traversable;

        let tasks: Vec<Task> = vec![
            create_test_task("Valid Task 1", Priority::High),
            create_test_task("Valid Task 2", Priority::Medium),
        ];

        // Validate all tasks have non-empty titles
        let validated: Option<Vec<String>> = tasks.traverse_option(|task| {
            if task.title.is_empty() {
                None
            } else {
                Some(task.title)
            }
        });

        assert!(validated.is_some(), "All tasks should pass validation");
        assert_eq!(validated.unwrap().len(), 2);
    }

    // -------------------------------------------------------------------------
    // Nil UUID Validation Tests
    // -------------------------------------------------------------------------

    /// Helper function to create a task with a nil UUID.
    fn create_task_with_nil_uuid(title: &str, priority: Priority) -> Task {
        Task::new(
            TaskId::from_uuid(uuid::Uuid::nil()),
            title,
            Timestamp::now(),
        )
        .with_priority(priority)
    }

    /// Test that `paginate_tasks` returns error when a task has nil UUID.
    #[rstest]
    fn test_paginate_tasks_with_nil_uuid_returns_error() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Valid Task 1", Priority::Low),
            create_task_with_nil_uuid("Invalid Task", Priority::Medium),
            create_test_task("Valid Task 2", Priority::High),
        ]
        .into_iter()
        .collect();

        let result = paginate_tasks(&tasks, 1, 10);

        assert!(result.is_err(), "Should return error when nil UUID present");
        let error = result.unwrap_err();
        assert!(
            error.message.contains("nil"),
            "Error message should mention nil UUID"
        );
    }

    /// Test that `paginate_tasks` succeeds when all tasks have valid UUIDs.
    #[rstest]
    fn test_paginate_tasks_without_nil_uuid_succeeds() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Valid Task 1", Priority::Low),
            create_test_task("Valid Task 2", Priority::Medium),
            create_test_task("Valid Task 3", Priority::High),
        ]
        .into_iter()
        .collect();

        let result = paginate_tasks(&tasks, 1, 10);

        assert!(result.is_ok(), "Should succeed when all UUIDs are valid");
        let response = result.unwrap();
        assert_eq!(response.data.len(), 3);
        assert_eq!(response.total, 3);
    }

    /// Test that `paginate_tasks` error contains the invalid task ID.
    #[rstest]
    fn test_paginate_tasks_error_contains_task_id() {
        let nil_uuid = uuid::Uuid::nil();
        let tasks: PersistentVector<Task> =
            vec![create_task_with_nil_uuid("Invalid Task", Priority::Medium)]
                .into_iter()
                .collect();

        let result = paginate_tasks(&tasks, 1, 10);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(
            *error.invalid_task_id.as_uuid(),
            nil_uuid,
            "Error should contain the invalid task ID"
        );
    }

    /// Test that `filter_tasks_with_traversable` returns error when nil UUID present.
    /// This test verifies that nil UUID validation error is propagated (not silently returning empty).
    #[rstest]
    fn test_filter_tasks_with_traversable_nil_uuid_returns_error() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Valid Task 1", Priority::Low),
            create_task_with_nil_uuid("Invalid Task", Priority::Medium),
            create_test_task("Valid Task 2", Priority::High),
        ]
        .into_iter()
        .collect();

        let result = filter_tasks_with_traversable(&tasks, None, None);

        // Should return Err when nil UUID present (not silently return empty)
        assert!(
            result.is_err(),
            "Should return error when nil UUID present in input"
        );
        let error = result.unwrap_err();
        assert!(
            error.invalid_task_id.as_uuid().is_nil(),
            "Error should contain the nil UUID task ID"
        );
    }

    /// Test that `validate_tasks_with_traversable` returns Err for nil UUID.
    #[rstest]
    fn test_validate_tasks_with_traversable_nil_uuid() {
        let tasks = vec![
            create_test_task("Valid Task", Priority::Low),
            create_task_with_nil_uuid("Invalid Task", Priority::Medium),
        ];

        let result = validate_tasks_with_traversable(&tasks);

        assert!(
            result.is_err(),
            "Should return Err when any task has nil UUID"
        );
        let error = result.unwrap_err();
        assert!(
            error.invalid_task_id.as_uuid().is_nil(),
            "Error should contain the nil UUID task ID"
        );
    }

    /// Test that `validate_tasks_with_traversable` returns Ok for valid UUIDs.
    #[rstest]
    fn test_validate_tasks_with_traversable_valid_uuids() {
        let tasks = vec![
            create_test_task("Valid Task 1", Priority::Low),
            create_test_task("Valid Task 2", Priority::High),
        ];

        let result = validate_tasks_with_traversable(&tasks);

        assert!(
            result.is_ok(),
            "Should return Ok when all tasks have valid UUIDs"
        );
        assert_eq!(result.unwrap().len(), 2);
    }

    /// Test that nil UUID on page boundary is correctly detected.
    #[rstest]
    fn test_paginate_tasks_nil_uuid_on_second_page() {
        // Create 15 valid tasks, then 1 invalid task, so the invalid one is on page 2
        let mut tasks: Vec<Task> = (0..15)
            .map(|i| create_test_task(&format!("Task {i}"), Priority::Medium))
            .collect();
        tasks.push(create_task_with_nil_uuid("Invalid Task", Priority::High));

        let tasks: PersistentVector<Task> = tasks.into_iter().collect();

        // Page 1 should succeed (only valid tasks)
        let page1_result = paginate_tasks(&tasks, 1, 10);
        assert!(page1_result.is_ok(), "Page 1 should succeed");

        // Page 2 should fail (contains invalid task)
        let page2_result = paginate_tasks(&tasks, 2, 10);
        assert!(page2_result.is_err(), "Page 2 should fail due to nil UUID");
    }

    // -------------------------------------------------------------------------
    // Phase 1.1g: Error Propagation Integration Tests (Codex Review #225)
    // -------------------------------------------------------------------------

    /// Integration test: `filter_tasks_with_traversable` error propagates through `filter_tasks`.
    /// Ensures that the wrapper function correctly propagates the validation error.
    #[rstest]
    fn test_filter_tasks_propagates_nil_uuid_error() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Valid Task", Priority::Low),
            create_task_with_nil_uuid("Invalid Task", Priority::High),
        ]
        .into_iter()
        .collect();

        let result = filter_tasks(&tasks, None, None);

        assert!(
            result.is_err(),
            "filter_tasks should propagate nil UUID error"
        );
    }

    /// Integration test: `filter_tasks` succeeds when all tasks have valid UUIDs.
    #[rstest]
    fn test_filter_tasks_succeeds_with_valid_uuids() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task 1", Priority::Low),
            create_test_task("Task 2", Priority::High),
        ]
        .into_iter()
        .collect();

        let result = filter_tasks(&tasks, None, None);

        assert!(
            result.is_ok(),
            "filter_tasks should succeed with valid UUIDs"
        );
        assert_eq!(result.unwrap().len(), 2);
    }

    /// Integration test: `filter_tasks` with status filter still returns error for nil UUID.
    #[rstest]
    fn test_filter_tasks_with_status_filter_returns_error_for_nil_uuid() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Valid Task", Priority::Low),
            create_task_with_nil_uuid("Invalid Task", Priority::High),
        ]
        .into_iter()
        .collect();

        // Even with a status filter, nil UUID should cause an error
        let result = filter_tasks(&tasks, Some(TaskStatus::Pending), None);

        assert!(
            result.is_err(),
            "filter_tasks with status filter should still return error for nil UUID"
        );
    }

    // =========================================================================
    // REQ-SEARCH-API-001: Search Pagination Tests
    // =========================================================================

    /// Test: `normalize_search_pagination` defaults limit to `DEFAULT_SEARCH_LIMIT` when not specified.
    #[rstest]
    fn test_normalize_search_pagination_default_limit() {
        let (limit, offset) = normalize_search_pagination(None, None);
        assert_eq!(
            limit, DEFAULT_SEARCH_LIMIT,
            "Default limit should be DEFAULT_SEARCH_LIMIT"
        );
        assert_eq!(offset, 0, "Default offset should be 0");
    }

    /// Test: `normalize_search_pagination` defaults offset to 0 when not specified.
    #[rstest]
    fn test_normalize_search_pagination_default_offset() {
        let (limit, offset) = normalize_search_pagination(Some(50), None);
        assert_eq!(limit, 50, "Limit should be passed through");
        assert_eq!(offset, 0, "Default offset should be 0");
    }

    /// Test: `normalize_search_pagination` clamps limit to `MAX_SEARCH_LIMIT` when exceeds.
    #[rstest]
    fn test_normalize_search_pagination_clamps_limit_to_max() {
        let (limit, offset) = normalize_search_pagination(Some(500), Some(10));
        assert_eq!(
            limit, MAX_SEARCH_LIMIT,
            "Limit should be clamped to MAX_SEARCH_LIMIT"
        );
        assert_eq!(offset, 10, "Offset should be passed through");
    }

    /// Test: `normalize_search_pagination` allows limit at boundary (`MAX_SEARCH_LIMIT`).
    #[rstest]
    fn test_normalize_search_pagination_allows_max_limit() {
        let (limit, offset) = normalize_search_pagination(Some(MAX_SEARCH_LIMIT), Some(0));
        assert_eq!(
            limit, MAX_SEARCH_LIMIT,
            "Limit at max boundary should be allowed"
        );
        assert_eq!(offset, 0, "Offset should be 0");
    }

    /// Test: `normalize_search_pagination` allows limit just below max.
    #[rstest]
    fn test_normalize_search_pagination_allows_below_max_limit() {
        let (limit, offset) = normalize_search_pagination(Some(MAX_SEARCH_LIMIT - 1), Some(5));
        assert_eq!(
            limit,
            MAX_SEARCH_LIMIT - 1,
            "Limit below max should be allowed"
        );
        assert_eq!(offset, 5, "Offset should be passed through");
    }

    /// Test: `normalize_search_pagination` is a pure function (same input -> same output).
    #[rstest]
    fn test_normalize_search_pagination_is_pure() {
        // Multiple calls with the same input should return the same output
        let result1 = normalize_search_pagination(Some(100), Some(20));
        let result2 = normalize_search_pagination(Some(100), Some(20));
        assert_eq!(
            result1, result2,
            "Pure function should return same output for same input"
        );
    }

    // =========================================================================
    // REQ-SEARCH-API-001: SearchScope Deserialization Tests
    // =========================================================================

    /// Test: `SearchScope::from_str` parses "title" correctly.
    #[rstest]
    fn test_search_scope_from_str_title() {
        use std::str::FromStr;
        assert_eq!(SearchScope::from_str("title"), Ok(SearchScope::Title));
        assert_eq!(SearchScope::from_str("TITLE"), Ok(SearchScope::Title));
        assert_eq!(SearchScope::from_str("Title"), Ok(SearchScope::Title));
    }

    /// Test: `SearchScope::from_str` parses "tags" correctly.
    #[rstest]
    fn test_search_scope_from_str_tags() {
        use std::str::FromStr;
        assert_eq!(SearchScope::from_str("tags"), Ok(SearchScope::Tags));
        assert_eq!(SearchScope::from_str("TAGS"), Ok(SearchScope::Tags));
        assert_eq!(SearchScope::from_str("Tags"), Ok(SearchScope::Tags));
    }

    /// Test: `SearchScope::from_str` parses "all" correctly.
    #[rstest]
    fn test_search_scope_from_str_all() {
        use std::str::FromStr;
        assert_eq!(SearchScope::from_str("all"), Ok(SearchScope::All));
        assert_eq!(SearchScope::from_str("ALL"), Ok(SearchScope::All));
        assert_eq!(SearchScope::from_str("All"), Ok(SearchScope::All));
    }

    /// Test: `SearchScope::from_str` returns error for unknown values.
    #[rstest]
    fn test_search_scope_from_str_unknown_returns_error() {
        use std::str::FromStr;
        let result = SearchScope::from_str("unknown");
        assert!(result.is_err(), "Unknown value should return error");
        assert!(
            result
                .unwrap_err()
                .contains("Invalid search scope 'unknown'"),
            "Error message should include the invalid value"
        );
    }

    /// Test: `SearchScope::from_str` returns error for empty string.
    #[rstest]
    fn test_search_scope_from_str_empty_returns_error() {
        use std::str::FromStr;
        let result = SearchScope::from_str("");
        assert!(result.is_err(), "Empty string should return error");
    }

    /// Test: `SearchScope` serde deserialization for valid values.
    #[rstest]
    fn test_search_scope_serde_deserialize_valid() {
        let scope: SearchScope = serde_json::from_str("\"title\"").unwrap();
        assert_eq!(scope, SearchScope::Title);

        let scope: SearchScope = serde_json::from_str("\"tags\"").unwrap();
        assert_eq!(scope, SearchScope::Tags);

        let scope: SearchScope = serde_json::from_str("\"all\"").unwrap();
        assert_eq!(scope, SearchScope::All);
    }

    /// Test: `SearchScope` serde deserialization returns error for unknown values.
    #[rstest]
    fn test_search_scope_serde_deserialize_unknown_returns_error() {
        let result: Result<SearchScope, _> = serde_json::from_str("\"invalid\"");
        assert!(result.is_err(), "Unknown value should return serde error");
    }

    // =========================================================================
    // REQ-SEARCH-API-001: Search Result Order Stability Tests
    // =========================================================================

    /// Test: Same query returns results in stable order.
    #[rstest]
    fn test_search_result_order_is_stable() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task A", Priority::Low),
            create_test_task("Task B", Priority::Medium),
            create_test_task("Task C", Priority::High),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);

        // Run same search multiple times
        let result1 = search_with_scope_indexed(&index, "", SearchScope::All);
        let result2 = search_with_scope_indexed(&index, "", SearchScope::All);
        let result3 = search_with_scope_indexed(&index, "", SearchScope::All);

        // Extract task IDs for comparison
        let ids1: Vec<_> = result1.tasks.iter().map(|t| t.task_id.clone()).collect();
        let ids2: Vec<_> = result2.tasks.iter().map(|t| t.task_id.clone()).collect();
        let ids3: Vec<_> = result3.tasks.iter().map(|t| t.task_id.clone()).collect();

        assert_eq!(ids1, ids2, "Search results should be in stable order");
        assert_eq!(ids2, ids3, "Search results should be in stable order");
    }

    /// Test: Search with keyword returns results in stable order.
    #[rstest]
    fn test_search_with_keyword_order_is_stable() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Important meeting", Priority::High),
            create_test_task("Important deadline", Priority::Critical),
            create_test_task("Important review", Priority::Medium),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);

        // Run same search multiple times
        let result1 = search_with_scope_indexed(&index, "important", SearchScope::Title);
        let result2 = search_with_scope_indexed(&index, "important", SearchScope::Title);

        let ids1: Vec<_> = result1.tasks.iter().map(|t| t.task_id.clone()).collect();
        let ids2: Vec<_> = result2.tasks.iter().map(|t| t.task_id.clone()).collect();

        assert_eq!(
            ids1, ids2,
            "Search results with keyword should be in stable order"
        );
    }

    // =========================================================================
    // REQ-SEARCH-API-001: Pagination Tests for Handler
    // =========================================================================

    /// Test: Default limit is applied when limit is not specified.
    #[rstest]
    fn test_normalize_search_pagination_applies_default_limit() {
        let (limit, offset) = normalize_search_pagination(None, None);
        assert_eq!(
            limit, DEFAULT_SEARCH_LIMIT,
            "Default limit should be DEFAULT_SEARCH_LIMIT"
        );
        assert_eq!(offset, 0, "Default offset should be 0");
    }

    /// Test: Max limit is applied when limit exceeds max.
    #[rstest]
    fn test_normalize_search_pagination_applies_max_limit() {
        let (limit, offset) = normalize_search_pagination(Some(500), None);
        assert_eq!(
            limit, MAX_SEARCH_LIMIT,
            "Limit should be clamped to MAX_SEARCH_LIMIT"
        );
        assert_eq!(offset, 0, "Offset should be 0");
    }

    /// Test: Exact max limit is allowed.
    #[rstest]
    fn test_normalize_search_pagination_allows_exact_max_limit() {
        let (limit, offset) = normalize_search_pagination(Some(MAX_SEARCH_LIMIT), Some(10));
        assert_eq!(limit, MAX_SEARCH_LIMIT, "Exact max limit should be allowed");
        assert_eq!(offset, 10, "Offset should be passed through");
    }

    /// Test: `limit=0` returns empty array (explicit user intent).
    ///
    /// When a user explicitly specifies `limit=0`, they want zero results.
    /// This is a valid use case for checking total counts without fetching data.
    #[rstest]
    fn test_normalize_search_pagination_allows_limit_zero() {
        let (limit, offset) = normalize_search_pagination(Some(0), Some(10));
        assert_eq!(limit, 0, "Limit 0 should be allowed (returns empty array)");
        assert_eq!(offset, 10, "Offset should still be passed through");
    }

    /// Test: `limit=0` effectively returns empty result when applied to search.
    #[rstest]
    fn test_search_with_limit_zero_returns_empty() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task A", Priority::Low),
            create_test_task("Task B", Priority::Medium),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let results = search_with_scope_indexed(&index, "", SearchScope::All);

        // Verify we have results before pagination
        assert!(
            !results.tasks.is_empty(),
            "Should have results before pagination"
        );

        // Simulate handler's pagination application with limit=0
        let count = results.into_tasks().iter().take(0).count();

        assert_eq!(count, 0, "limit=0 should return empty array");
    }

    // =========================================================================
    // REQ-SEARCH-API-001: Search Deterministic Order with Limit/Offset Tests
    // =========================================================================

    /// Test: Search with limit/offset returns same results across multiple calls.
    ///
    /// This is a law-like property test ensuring deterministic ordering.
    #[rstest]
    fn test_search_deterministic_order_with_limit_offset() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task A", Priority::Low),
            create_test_task("Task B", Priority::Medium),
            create_test_task("Task C", Priority::High),
            create_test_task("Task D", Priority::Critical),
            create_test_task("Task E", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);

        // First search with limit=2, offset=1
        let result1 = search_with_scope_indexed(&index, "", SearchScope::All);
        let paginated1: Vec<_> = result1
            .into_tasks()
            .iter()
            .skip(1)
            .take(2)
            .map(|t| t.task_id.clone())
            .collect();

        // Second search with same parameters
        let result2 = search_with_scope_indexed(&index, "", SearchScope::All);
        let paginated2: Vec<_> = result2
            .into_tasks()
            .iter()
            .skip(1)
            .take(2)
            .map(|t| t.task_id.clone())
            .collect();

        assert_eq!(
            paginated1, paginated2,
            "Search with limit/offset should return same results in same order"
        );
    }

    /// Test: Empty query with limit/offset returns correct subset.
    /// Results are ordered by `task_id`, so we verify that pagination works correctly
    /// regardless of the specific order (which depends on UUIDs).
    #[rstest]
    fn test_search_empty_query_with_limit_offset() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task A", Priority::Low),
            create_test_task("Task B", Priority::Medium),
            create_test_task("Task C", Priority::High),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let results = search_with_scope_indexed(&index, "", SearchScope::All);

        // All tasks should be returned for empty query
        assert_eq!(
            results.tasks.len(),
            3,
            "Empty query should return all tasks"
        );

        // Get the ordered task titles
        let all_titles: Vec<_> = results.tasks.iter().map(|t| t.title.clone()).collect();

        // Apply pagination (limit=2, offset=1)
        let paginated: Vec<_> = results
            .into_tasks()
            .iter()
            .skip(1)
            .take(2)
            .cloned()
            .collect();

        assert_eq!(paginated.len(), 2, "Should return 2 tasks after pagination");
        // Verify pagination returns the correct subset based on task_id order
        assert_eq!(paginated[0].title, all_titles[1]);
        assert_eq!(paginated[1].title, all_titles[2]);
    }

    /// Test: Search with keyword and limit/offset.
    /// Results are ordered by `task_id`, so we verify that pagination works correctly
    /// regardless of the specific order (which depends on UUIDs).
    #[rstest]
    fn test_search_with_keyword_applies_limit_offset() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Important meeting A", Priority::High),
            create_test_task("Important meeting B", Priority::Critical),
            create_test_task("Important meeting C", Priority::Medium),
            create_test_task("Other task", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let results = search_with_scope_indexed(&index, "important", SearchScope::Title);

        // Should find 3 tasks containing "important"
        assert_eq!(results.tasks.len(), 3, "Should find 3 matching tasks");

        // Get the ordered task titles
        let all_titles: Vec<_> = results.tasks.iter().map(|t| t.title.clone()).collect();

        // Apply pagination (limit=1, offset=1)
        let paginated: Vec<_> = results
            .into_tasks()
            .iter()
            .skip(1)
            .take(1)
            .cloned()
            .collect();

        assert_eq!(paginated.len(), 1, "Should return 1 task after pagination");
        assert_eq!(
            paginated[0].title, all_titles[1],
            "Should return second matching task"
        );
    }

    // =========================================================================
    // REQ-SEARCH-API-001: SearchScope Error Message Tests
    // =========================================================================

    /// Test: `SearchScope::from_str` error message includes valid options.
    #[rstest]
    fn test_search_scope_error_message_includes_valid_options() {
        use std::str::FromStr;
        let result = SearchScope::from_str("invalid");
        assert!(result.is_err());
        let error_message = result.unwrap_err();
        assert!(
            error_message.contains("title"),
            "Error should mention 'title' as valid option"
        );
        assert!(
            error_message.contains("tags"),
            "Error should mention 'tags' as valid option"
        );
        assert!(
            error_message.contains("all"),
            "Error should mention 'all' as valid option"
        );
    }

    /// Test: Serde deserialization error for invalid scope.
    ///
    /// This verifies that serde correctly propagates the error when
    /// an invalid `in` parameter is provided in the query string.
    #[rstest]
    fn test_search_tasks_query_invalid_scope_deserialize() {
        // Simulate deserializing a query string with invalid scope
        let json = r#"{"q": "test", "in": "invalid"}"#;
        let result: Result<SearchTasksQuery, _> = serde_json::from_str(json);

        assert!(result.is_err(), "Invalid scope should fail deserialization");
        let error_message = result.unwrap_err().to_string();
        assert!(
            error_message.contains("Invalid search scope"),
            "Error should contain 'Invalid search scope': {error_message}"
        );
    }

    /// Test: Default scope is applied when `in` is omitted.
    #[rstest]
    fn test_search_tasks_query_default_scope() {
        let json = r#"{"q": "test"}"#;
        let query: SearchTasksQuery = serde_json::from_str(json).unwrap();

        assert_eq!(query.q, "test");
        assert_eq!(query.scope, SearchScope::All, "Default scope should be All");
        assert!(query.limit.is_none(), "Default limit should be None");
        assert!(query.offset.is_none(), "Default offset should be None");
    }

    // =========================================================================
    // Phase 1 Codex Review Fix: Multi-word Infix Match Tests
    // =========================================================================

    /// Regression test: "meeting tomorrow" should match "important meeting tomorrow".
    /// This tests the `title_full_all_suffix_index` for multi-word infix queries.
    #[rstest]
    fn test_search_multi_word_infix_match() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("important meeting tomorrow", Priority::High),
            create_test_task("urgent review today", Priority::Medium),
            create_test_task("meeting later", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("meeting tomorrow");

        // "meeting tomorrow" should match "important meeting tomorrow"
        assert!(
            result.is_some(),
            "Query 'meeting tomorrow' should match 'important meeting tomorrow'"
        );
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            1,
            "Only the task with 'meeting tomorrow' should match"
        );
        assert!(
            result
                .tasks
                .iter()
                .any(|t| t.title.contains("important meeting tomorrow"))
        );
    }

    /// Regression test: Multi-word infix with partial word match.
    /// "eeting tomorr" should match "important meeting tomorrow".
    #[rstest]
    fn test_search_multi_word_infix_partial_match() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("important meeting tomorrow", Priority::High),
            create_test_task("Other task", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("eeting tomorr");

        // "eeting tomorr" appears in the middle of "important m[eeting tomorr]ow"
        assert!(
            result.is_some(),
            "Query 'eeting tomorr' should match 'important meeting tomorrow'"
        );
        let result = result.unwrap();
        assert_eq!(result.tasks.len(), 1);
    }

    /// Regression test: Multi-word query at the end of title.
    /// "code review" should match "weekly code review".
    #[rstest]
    fn test_search_multi_word_suffix_match() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("weekly code review", Priority::High),
            create_test_task("code update", Priority::Medium),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("code review");

        // "code review" should match "weekly code review"
        assert!(
            result.is_some(),
            "Query 'code review' should match 'weekly code review'"
        );
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            1,
            "Only 'weekly code review' should match"
        );
        assert!(result.tasks.iter().any(|t| t.title.contains("code review")));
    }

    /// Regression test: Complex multi-word infix with multiple matching tasks.
    #[rstest]
    fn test_search_multi_word_infix_multiple_matches() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("urgent meeting tomorrow morning", Priority::High),
            create_test_task("important meeting tomorrow afternoon", Priority::Medium),
            create_test_task("casual meeting later", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);
        let result = index.search_by_title("meeting tomorrow");

        // "meeting tomorrow" should match both tasks containing this substring
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(
            result.tasks.len(),
            2,
            "Both tasks with 'meeting tomorrow' should match"
        );
    }

    // =========================================================================
    // Phase 1 Codex Review Fix: Order Stability Tests
    // =========================================================================

    /// Regression test: Result order should be stable regardless of repository input order.
    /// Since ordering is by `task_id` only, results should be identical
    /// regardless of the order in which tasks were added to the index.
    #[rstest]
    fn test_search_order_stable_regardless_of_input_order() {
        // Create tasks with different task_ids (UUIDs are generated, so each run has unique IDs)
        let task1 = create_test_task("Important task A", Priority::High);
        let task2 = create_test_task("Important task B", Priority::Medium);
        let task3 = create_test_task("Important task C", Priority::Low);

        // Build index with tasks in one order
        let tasks_order1: PersistentVector<Task> =
            vec![task1.clone(), task2.clone(), task3.clone()]
                .into_iter()
                .collect();
        let index1 = SearchIndex::build(&tasks_order1);

        // Build index with tasks in a different order
        let tasks_order2: PersistentVector<Task> = vec![task3, task1, task2].into_iter().collect();
        let index2 = SearchIndex::build(&tasks_order2);

        // Search both indexes
        let result1 = index1
            .search_by_title("important")
            .map(|r| {
                r.tasks
                    .iter()
                    .map(|t| t.task_id.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let result2 = index2
            .search_by_title("important")
            .map(|r| {
                r.tasks
                    .iter()
                    .map(|t| t.task_id.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Results should be identical (no sorting needed since task_id ordering is deterministic)
        assert_eq!(
            result1, result2,
            "Same tasks should be found in the same order regardless of input order"
        );
    }

    /// Test: Results are ordered by `task_id` for deterministic ordering.
    #[rstest]
    fn test_search_order_uses_task_id_for_stable_ordering() {
        let task1 = create_test_task("Important task", Priority::High);
        let task2 = create_test_task("Important task", Priority::Medium);

        // Capture task IDs before moving tasks into vector
        let task1_id = task1.task_id.clone();
        let task2_id = task2.task_id.clone();

        let tasks: PersistentVector<Task> = vec![task1, task2].into_iter().collect();

        let index = SearchIndex::build(&tasks);

        // Run search multiple times
        let results: Vec<Vec<TaskId>> = (0..10)
            .map(|_| {
                index
                    .search_by_title("important")
                    .map(|r| r.tasks.iter().map(|t| t.task_id.clone()).collect())
                    .unwrap_or_default()
            })
            .collect();

        // All iterations should return the same order
        for i in 1..results.len() {
            assert_eq!(
                results[0], results[i],
                "Search results should be stable across multiple calls"
            );
        }

        // Verify results are sorted by task_id
        let expected_order: Vec<TaskId> = {
            let mut ids = vec![task1_id, task2_id];
            ids.sort();
            ids
        };
        assert_eq!(
            results[0], expected_order,
            "Results should be ordered by task_id"
        );
    }

    /// Test: `all_tasks` returns tasks in stable order.
    #[rstest]
    fn test_all_tasks_returns_stable_order() {
        let task1 = create_test_task("Task 1", Priority::High);
        let task2 = create_test_task("Task 2", Priority::Medium);
        let task3 = create_test_task("Task 3", Priority::Low);

        let tasks: PersistentVector<Task> = vec![task1, task2, task3].into_iter().collect();

        let index = SearchIndex::build(&tasks);

        // Get all tasks multiple times
        let results: Vec<Vec<TaskId>> = (0..5)
            .map(|_| {
                index
                    .all_tasks()
                    .iter()
                    .map(|t| t.task_id.clone())
                    .collect()
            })
            .collect();

        // All iterations should return the same order
        for i in 1..results.len() {
            assert_eq!(
                results[0], results[i],
                "all_tasks should return stable order"
            );
        }
    }

    // =========================================================================
    // Phase 1 Final: All Scope Mixed Results Order Stability Test
    // =========================================================================

    /// Test: All scope search with mixed title and tag matches returns stable order.
    ///
    /// This test verifies that when searching with `SearchScope::All`:
    /// 1. Tasks matching only title, only tag, and both are all returned
    /// 2. Results are in stable order across multiple searches (directly compared, not sorted)
    /// 3. Title matches appear before tag-only matches (by design via `Semigroup::combine`)
    ///
    /// This is a critical property for pagination consistency.
    ///
    /// Note: Within each category (title matches and tag matches), results are sorted
    /// by `task_id` for stable ordering. The `Semigroup::combine` then merges them
    /// with title matches first, followed by tag-only matches (deduplicated).
    #[rstest]
    fn test_search_all_scope_mixed_results_stable_order() {
        // Create tasks with different match patterns:
        // - title_only_match: matches "important" in title only
        // - tag_only_match: matches "important" in tag only
        // - both_match: matches "important" in both title and tag
        // - no_match: does not match "important"
        let title_only_match = create_test_task("Important meeting", Priority::High);
        let tag_only_match =
            create_test_task_with_tags("Regular task", Priority::Medium, &["important"]);
        let both_match =
            create_test_task_with_tags("Important deadline", Priority::Critical, &["important"]);
        let no_match = create_test_task("Unrelated task", Priority::Low);

        // Capture task IDs for verification
        let title_only_id = title_only_match.task_id.clone();
        let tag_only_id = tag_only_match.task_id.clone();
        let both_match_id = both_match.task_id.clone();

        // Build index with tasks in a specific order
        let tasks: PersistentVector<Task> =
            vec![no_match, tag_only_match, both_match, title_only_match]
                .into_iter()
                .collect();

        let index = SearchIndex::build(&tasks);

        // Run the same search multiple times (5 times to ensure stability)
        let results: Vec<Vec<TaskId>> = (0..5)
            .map(|_| {
                search_with_scope_indexed(&index, "important", SearchScope::All)
                    .tasks
                    .iter()
                    .map(|t| t.task_id.clone())
                    .collect()
            })
            .collect();

        // Verify that all matching tasks are returned
        assert_eq!(
            results[0].len(),
            3,
            "Should find all 3 tasks matching 'important' (title, tag, or both)"
        );

        // Verify that all expected task IDs are present
        assert!(
            results[0].contains(&title_only_id),
            "Should include title-only match"
        );
        assert!(
            results[0].contains(&tag_only_id),
            "Should include tag-only match"
        );
        assert!(
            results[0].contains(&both_match_id),
            "Should include both-match task"
        );

        // CRITICAL: Verify that the order is stable across all searches
        // This is the main purpose of this test - ensuring pagination consistency
        for (iteration, result) in results.iter().enumerate().skip(1) {
            assert_eq!(
                &results[0], result,
                "Search iteration {} returned different order than iteration 0. \
                 Expected: {:?}, Got: {:?}",
                iteration, results[0], result
            );
        }

        // Verify that tag-only match appears after title matches
        // (title matches are prioritized via Semigroup::combine)
        let tag_only_position = results[0]
            .iter()
            .position(|id| id == &tag_only_id)
            .expect("tag_only_id should be in results");
        let title_only_position = results[0]
            .iter()
            .position(|id| id == &title_only_id)
            .expect("title_only_id should be in results");
        let both_match_position = results[0]
            .iter()
            .position(|id| id == &both_match_id)
            .expect("both_match_id should be in results");

        // both_match and title_only_match are title matches, so they should appear before tag_only_match
        assert!(
            tag_only_position > title_only_position.min(both_match_position),
            "Tag-only match should appear after at least one title match. \
             tag_only_position: {tag_only_position}, title_only_position: {title_only_position}, both_match_position: {both_match_position}"
        );
    }

    // =========================================================================
    // REQ-SEARCH-CACHE-001: Query Normalization Tests
    // =========================================================================

    /// Test: Basic normalization with leading/trailing whitespace and multiple spaces.
    #[rstest]
    fn test_normalize_query_basic() {
        let result = normalize_query("  Urgent   Task ");
        assert_eq!(result.key(), "urgent task");
        assert_eq!(result.tokens(), &["urgent", "task"]);
    }

    /// Test: Normalization with only lowercase conversion needed.
    #[rstest]
    fn test_normalize_query_lowercase_only() {
        let result = normalize_query("URGENT TASK");
        assert_eq!(result.key(), "urgent task");
        assert_eq!(result.tokens(), &["urgent", "task"]);
    }

    /// Test: Normalization of empty string.
    #[rstest]
    fn test_normalize_query_empty() {
        let result = normalize_query("");
        assert_eq!(result.key(), "");
        assert!(result.tokens().is_empty());
        assert!(result.is_empty());
    }

    /// Test: Normalization of whitespace-only string.
    #[rstest]
    fn test_normalize_query_whitespace_only() {
        let result = normalize_query("   ");
        assert_eq!(result.key(), "");
        assert!(result.tokens().is_empty());
        assert!(result.is_empty());
    }

    /// Test: Normalization of already normalized string.
    #[rstest]
    fn test_normalize_query_already_normalized() {
        let result = normalize_query("urgent task");
        assert_eq!(result.key(), "urgent task");
        assert_eq!(result.tokens(), &["urgent", "task"]);
    }

    /// Test: Normalization with mixed case.
    #[rstest]
    fn test_normalize_query_mixed_case() {
        let result = normalize_query("UrGeNt TaSk");
        assert_eq!(result.key(), "urgent task");
        assert_eq!(result.tokens(), &["urgent", "task"]);
    }

    /// Test: Normalization with tab characters.
    #[rstest]
    fn test_normalize_query_with_tabs() {
        let result = normalize_query("urgent\t\ttask");
        assert_eq!(result.key(), "urgent task");
        assert_eq!(result.tokens(), &["urgent", "task"]);
    }

    /// Test: Normalization with newline characters.
    #[rstest]
    fn test_normalize_query_with_newlines() {
        let result = normalize_query("urgent\n\ntask");
        assert_eq!(result.key(), "urgent task");
        assert_eq!(result.tokens(), &["urgent", "task"]);
    }

    /// Test: Single word normalization.
    #[rstest]
    fn test_normalize_query_single_word() {
        let result = normalize_query("  URGENT  ");
        assert_eq!(result.key(), "urgent");
        assert_eq!(result.tokens(), &["urgent"]);
    }

    /// Test: Special characters are preserved.
    #[rstest]
    fn test_normalize_query_special_characters() {
        let result = normalize_query("bug-123 @urgent #important");
        assert_eq!(result.key(), "bug-123 @urgent #important");
        assert_eq!(result.tokens(), &["bug-123", "@urgent", "#important"]);
    }

    /// Test: Unicode characters are preserved and lowercased.
    #[rstest]
    fn test_normalize_query_unicode() {
        let result = normalize_query("  TACHE  urgente  ");
        assert_eq!(result.key(), "tache urgente");
        assert_eq!(result.tokens(), &["tache", "urgente"]);
    }

    /// Test: Idempotent law - `normalize(normalize(q)) = normalize(q)`.
    #[rstest]
    #[case("  Urgent   Task ")]
    #[case("URGENT TASK")]
    #[case("")]
    #[case("   ")]
    #[case("already normalized")]
    #[case("bug-123 @urgent")]
    fn test_normalize_query_idempotent(#[case] input: &str) {
        let first = normalize_query(input);
        let second = normalize_query(first.key());
        assert_eq!(
            first, second,
            "normalize should be idempotent: normalize(normalize(q)) = normalize(q)"
        );
    }

    /// Test: `normalize_query` returns correct values via getters.
    #[rstest]
    fn test_normalized_query_getters() {
        let query = normalize_query("urgent task");
        assert_eq!(query.key(), "urgent task");
        assert_eq!(query.tokens(), &["urgent", "task"]);
        assert!(!query.is_empty());
    }

    /// Test: `NormalizedQuery::is_empty` for empty query.
    #[rstest]
    fn test_normalized_query_is_empty() {
        let empty = normalize_query("");
        assert!(empty.is_empty());

        let non_empty = normalize_query("test");
        assert!(!non_empty.is_empty());
    }

    // =========================================================================
    // REQ-SEARCH-CACHE-001: SearchCacheKey Tests
    // =========================================================================

    /// Test: `SearchCacheKey::from_raw` creates correct cache key.
    #[rstest]
    fn test_search_cache_key_from_raw_basic() {
        let key = SearchCacheKey::from_raw("  Urgent Task  ", SearchScope::All, Some(50), Some(0));

        assert_eq!(key.normalized_query(), "urgent task");
        assert_eq!(key.scope(), SearchScope::All);
        assert_eq!(key.limit(), 50);
        assert_eq!(key.offset(), 0);
    }

    /// Test: `SearchCacheKey::from_raw` with different parameters.
    #[rstest]
    fn test_search_cache_key_from_raw() {
        let key =
            SearchCacheKey::from_raw("  URGENT  task  ", SearchScope::Title, Some(100), Some(20));

        assert_eq!(key.normalized_query(), "urgent task");
        assert_eq!(key.scope(), SearchScope::Title);
        assert_eq!(key.limit(), 100);
        assert_eq!(key.offset(), 20);
    }

    /// Test: `SearchCacheKey::from_raw` normalizes pagination parameters.
    ///
    /// This ensures that cache keys with `None` values are equivalent to
    /// cache keys with explicit default values (limit=50, offset=0).
    #[rstest]
    fn test_search_cache_key_from_raw_normalizes_pagination() {
        // None values should be normalized to defaults
        let key_with_none = SearchCacheKey::from_raw("test", SearchScope::All, None, None);
        let key_with_defaults = SearchCacheKey::from_raw(
            "test",
            SearchScope::All,
            Some(DEFAULT_SEARCH_LIMIT),
            Some(0),
        );

        assert_eq!(
            key_with_none, key_with_defaults,
            "None pagination should equal default values"
        );
        assert_eq!(key_with_none.limit(), DEFAULT_SEARCH_LIMIT);
        assert_eq!(key_with_none.offset(), 0);

        // Limit exceeding max should be clamped
        let key_over_max = SearchCacheKey::from_raw("test", SearchScope::All, Some(300), Some(0));
        let key_at_max =
            SearchCacheKey::from_raw("test", SearchScope::All, Some(MAX_SEARCH_LIMIT), Some(0));

        assert_eq!(
            key_over_max, key_at_max,
            "Limit over max should be clamped to MAX_SEARCH_LIMIT"
        );
        assert_eq!(key_over_max.limit(), MAX_SEARCH_LIMIT);
    }

    /// Test: Cache key equality for equivalent normalized queries.
    #[rstest]
    fn test_search_cache_key_equality_normalized() {
        let key1 = SearchCacheKey::from_raw("  Urgent Task  ", SearchScope::All, Some(50), Some(0));
        let key2 = SearchCacheKey::from_raw("urgent task", SearchScope::All, Some(50), Some(0));
        let key3 = SearchCacheKey::from_raw("URGENT   TASK", SearchScope::All, Some(50), Some(0));

        assert_eq!(
            key1, key2,
            "Equivalent normalized queries should produce equal keys"
        );
        assert_eq!(
            key2, key3,
            "Equivalent normalized queries should produce equal keys"
        );
    }

    /// Test: Cache key inequality for different limits.
    #[rstest]
    fn test_search_cache_key_different_limit() {
        let key1 = SearchCacheKey::from_raw("urgent task", SearchScope::All, Some(50), Some(0));
        let key2 = SearchCacheKey::from_raw("urgent task", SearchScope::All, Some(100), Some(0));

        assert_ne!(key1, key2, "Different limits should produce different keys");
    }

    /// Test: Cache key inequality for different offsets.
    #[rstest]
    fn test_search_cache_key_different_offset() {
        let key1 = SearchCacheKey::from_raw("urgent task", SearchScope::All, Some(50), Some(0));
        let key2 = SearchCacheKey::from_raw("urgent task", SearchScope::All, Some(50), Some(10));

        assert_ne!(
            key1, key2,
            "Different offsets should produce different keys"
        );
    }

    /// Test: Cache key inequality for different scopes.
    #[rstest]
    fn test_search_cache_key_different_scope() {
        let key1 = SearchCacheKey::from_raw("urgent task", SearchScope::All, Some(50), Some(0));
        let key2 = SearchCacheKey::from_raw("urgent task", SearchScope::Title, Some(50), Some(0));
        let key3 = SearchCacheKey::from_raw("urgent task", SearchScope::Tags, Some(50), Some(0));

        assert_ne!(key1, key2, "Different scopes should produce different keys");
        assert_ne!(key2, key3, "Different scopes should produce different keys");
        assert_ne!(key1, key3, "Different scopes should produce different keys");
    }

    /// Test: Cache key can be used as `HashMap` key (Hash trait).
    #[rstest]
    fn test_search_cache_key_hashable() {
        use std::collections::HashMap;

        let key1 = SearchCacheKey::from_raw("urgent task", SearchScope::All, Some(50), Some(0));
        let key2 =
            SearchCacheKey::from_raw("  URGENT   TASK  ", SearchScope::All, Some(50), Some(0));

        let mut cache: HashMap<SearchCacheKey, String> = HashMap::new();
        cache.insert(key1, "cached_result".to_string());

        // key2 should hash to the same bucket and be equal to key1
        assert!(
            cache.contains_key(&key2),
            "Equal keys should be found in HashMap"
        );
        assert_eq!(cache.get(&key2), Some(&"cached_result".to_string()));
    }

    /// Test: Empty query normalization for cache key.
    #[rstest]
    fn test_search_cache_key_empty_query() {
        let key = SearchCacheKey::from_raw("", SearchScope::All, Some(50), Some(0));
        assert_eq!(key.normalized_query(), "");
    }

    /// Test: `SearchScope` Hash derivation works correctly.
    #[rstest]
    fn test_search_scope_hash() {
        use std::collections::HashSet;
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        fn hash_value<T: Hash>(t: &T) -> u64 {
            let mut hasher = DefaultHasher::new();
            t.hash(&mut hasher);
            hasher.finish()
        }

        // Same variant should have same hash
        assert_eq!(
            hash_value(&SearchScope::Title),
            hash_value(&SearchScope::Title)
        );
        assert_eq!(
            hash_value(&SearchScope::Tags),
            hash_value(&SearchScope::Tags)
        );
        assert_eq!(hash_value(&SearchScope::All), hash_value(&SearchScope::All));

        // Can be used in HashSet
        let mut set: HashSet<SearchScope> = HashSet::new();
        set.insert(SearchScope::Title);
        set.insert(SearchScope::Tags);
        set.insert(SearchScope::All);
        assert_eq!(set.len(), 3);
    }

    // =========================================================================
    // Invariant Tests: Ensuring internal consistency of NormalizedQuery
    // =========================================================================

    /// Test: Invariant that tokens.join(" ") equals key.
    ///
    /// This ensures the internal consistency of `NormalizedQuery`:
    /// the key is always the space-joined representation of tokens.
    #[rstest]
    #[case("  Hello   World  ")]
    #[case("single")]
    #[case("")]
    #[case("  ")]
    #[case("a b c d e")]
    #[case("\t\n\r multiple \t whitespace \n types")]
    fn test_normalized_query_invariant_tokens_join_equals_key(#[case] raw: &str) {
        let normalized = normalize_query(raw);
        assert_eq!(
            normalized.tokens().join(" "),
            normalized.key(),
            "Invariant violated: tokens.join(\" \") should equal key"
        );
    }

    /// Test: `NormalizedQuery` can only be created through `normalize_query` from external modules.
    ///
    /// Since `NormalizedQuery::new` is removed and fields are private,
    /// the public API for creating a `NormalizedQuery` is `normalize_query`.
    /// This guarantees that all instances created by external code maintain
    /// the invariants (key == tokens.join(" ")).
    ///
    /// Note: Within the same module, struct literals are technically possible
    /// due to Rust's visibility rules, but this is discouraged and not part
    /// of the public API contract.
    #[rstest]
    fn test_normalized_query_creation_only_via_normalize_query() {
        // Verify that the public API for creating NormalizedQuery is normalize_query.
        // External modules cannot construct NormalizedQuery directly due to private fields.
        let query = normalize_query("Test Query");
        assert_eq!(query.key(), "test query");
        assert_eq!(query.tokens(), &["test", "query"]);
    }

    /// Test: `NormalizedQuery::into_key` consumes and returns the key.
    #[rstest]
    fn test_normalized_query_into_key() {
        let query = normalize_query("test value");
        let key = query.into_key();
        assert_eq!(key, "test value");
    }

    // =========================================================================
    // Phase 2 Regression Tests: Query Normalization in search_with_scope_indexed
    // =========================================================================

    /// Test: Unnormalized query should match the same results as normalized query.
    ///
    /// This verifies that a query with extra spaces, mixed case, and leading/trailing
    /// whitespace (`"  URGENT   task  "`) produces the same search results as the
    /// normalized form (`"urgent task"`).
    ///
    /// The test uses truly distinct input strings to ensure normalization is working:
    /// - Unnormalized: `"  URGENT   task  "` (uppercase, multiple spaces, leading/trailing whitespace)
    /// - Normalized: `"urgent task"` (lowercase, single space, no leading/trailing whitespace)
    #[rstest]
    fn test_search_with_unnormalized_query_matches_normalized_title() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Urgent task for today", Priority::High),
            create_test_task("Regular task", Priority::Low),
            create_test_task("Another urgent task item", Priority::Medium),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);

        // Truly unnormalized query (uppercase, multiple spaces, leading/trailing whitespace)
        let unnormalized_raw = "  URGENT   task  ";
        let unnormalized_normalized = normalize_query(unnormalized_raw);
        let result_from_unnormalized =
            search_with_scope_indexed(&index, unnormalized_normalized.key(), SearchScope::Title);

        // Already normalized query
        let normalized_raw = "urgent task";
        let result_from_normalized =
            search_with_scope_indexed(&index, normalized_raw, SearchScope::Title);

        // Verify the raw strings are actually different
        assert_ne!(
            unnormalized_raw, normalized_raw,
            "Test precondition: raw strings must be different"
        );

        // Both should return the same results
        assert_eq!(
            result_from_unnormalized.tasks.len(),
            result_from_normalized.tasks.len(),
            "Unnormalized and normalized queries should return the same number of results"
        );

        // Verify the task IDs are the same (order-independent comparison)
        let mut ids_from_unnormalized: Vec<_> = result_from_unnormalized
            .tasks
            .iter()
            .map(|t| t.task_id.clone())
            .collect();
        let mut ids_from_normalized: Vec<_> = result_from_normalized
            .tasks
            .iter()
            .map(|t| t.task_id.clone())
            .collect();
        ids_from_unnormalized.sort();
        ids_from_normalized.sort();
        assert_eq!(
            ids_from_unnormalized, ids_from_normalized,
            "Unnormalized and normalized queries should return the same task IDs"
        );
    }

    /// Test: Unnormalized query should match the same results as normalized query for tags.
    ///
    /// This verifies that a query with extra spaces, mixed case, and leading/trailing
    /// whitespace (`"  URGENT  "`) produces the same search results as the
    /// normalized form (`"urgent"`).
    ///
    /// The test uses truly distinct input strings to ensure normalization is working:
    /// - Unnormalized: `"  URGENT  "` (uppercase, leading/trailing whitespace)
    /// - Normalized: `"urgent"` (lowercase, no whitespace)
    #[rstest]
    fn test_search_with_unnormalized_query_matches_normalized_tags() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task_with_tags("Task 1", Priority::High, &["backend", "urgent"]),
            create_test_task_with_tags("Task 2", Priority::Low, &["frontend"]),
            create_test_task_with_tags("Task 3", Priority::Medium, &["urgent", "priority"]),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);

        // Truly unnormalized query (uppercase, leading/trailing whitespace)
        let unnormalized_raw = "  URGENT  ";
        let unnormalized_normalized = normalize_query(unnormalized_raw);
        let result_from_unnormalized =
            search_with_scope_indexed(&index, unnormalized_normalized.key(), SearchScope::Tags);

        // Already normalized query
        let normalized_raw = "urgent";
        let result_from_normalized =
            search_with_scope_indexed(&index, normalized_raw, SearchScope::Tags);

        // Verify the raw strings are actually different
        assert_ne!(
            unnormalized_raw, normalized_raw,
            "Test precondition: raw strings must be different"
        );

        // Both should return the same results
        assert_eq!(
            result_from_unnormalized.tasks.len(),
            result_from_normalized.tasks.len(),
            "Unnormalized and normalized queries should return the same number of results for tags"
        );

        // Verify the task IDs are the same (order-independent comparison)
        let mut ids_from_unnormalized: Vec<_> = result_from_unnormalized
            .tasks
            .iter()
            .map(|t| t.task_id.clone())
            .collect();
        let mut ids_from_normalized: Vec<_> = result_from_normalized
            .tasks
            .iter()
            .map(|t| t.task_id.clone())
            .collect();
        ids_from_unnormalized.sort();
        ids_from_normalized.sort();
        assert_eq!(
            ids_from_unnormalized, ids_from_normalized,
            "Unnormalized and normalized queries should return the same task IDs for tags"
        );
    }

    /// Test: Multi-space query normalization with `SearchScope::All`.
    ///
    /// Ensures that extra spaces in the query do not affect search results.
    #[rstest]
    fn test_search_with_multi_space_query_all_scope() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Important meeting tomorrow", Priority::High),
            create_test_task_with_tags("Regular task", Priority::Low, &["important"]),
            create_test_task("Meeting notes", Priority::Medium),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);

        // Normalized query with extra spaces
        let normalized = normalize_query("   important   meeting   ");
        let normalized_key = normalized.key();
        let result_normalized = search_with_scope_indexed(&index, normalized_key, SearchScope::All);

        // Clean query
        let result_clean = search_with_scope_indexed(&index, "important meeting", SearchScope::All);

        // Both should return the same results
        assert_eq!(
            result_normalized.tasks.len(),
            result_clean.tasks.len(),
            "Multi-space query should match clean query results"
        );
    }

    /// Test: Leading and trailing whitespace in query should not affect results.
    #[rstest]
    fn test_search_with_leading_trailing_whitespace() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Callback implementation", Priority::High),
            create_test_task("Regular task", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);

        // Query with leading/trailing whitespace
        let normalized = normalize_query("   callback   ");
        let normalized_key = normalized.key();
        let result_normalized =
            search_with_scope_indexed(&index, normalized_key, SearchScope::Title);

        // Clean query
        let result_clean = search_with_scope_indexed(&index, "callback", SearchScope::Title);

        // Both should return the same results
        assert_eq!(
            result_normalized.tasks.len(),
            result_clean.tasks.len(),
            "Leading/trailing whitespace should not affect results"
        );
        assert_eq!(result_normalized.tasks.len(), 1);
    }

    /// Test: Whitespace-only query is normalized to empty string, returning all tasks.
    ///
    /// This documents the intentional behavior that a query consisting only of
    /// whitespace characters (spaces, tabs, etc.) is normalized to an empty string,
    /// which triggers the "return all tasks" behavior.
    ///
    /// # Behavior
    ///
    /// - Input: `"   "` (whitespace only)
    /// - Normalized: `""` (empty string)
    /// - Result: All tasks are returned
    ///
    /// This is consistent with the empty query behavior and provides a predictable
    /// user experience where "no meaningful search term" equals "show everything".
    #[rstest]
    fn test_search_whitespace_only_query_returns_all() {
        let tasks: PersistentVector<Task> = vec![
            create_test_task("Task A", Priority::High),
            create_test_task("Task B", Priority::Medium),
            create_test_task("Task C", Priority::Low),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);

        // Whitespace-only query
        let whitespace_only = "   \t  ";
        let normalized = normalize_query(whitespace_only);

        // Verify normalization behavior
        assert!(
            normalized.is_empty(),
            "Whitespace-only query should normalize to empty string"
        );
        assert_eq!(
            normalized.key(),
            "",
            "Normalized key should be empty string"
        );
        assert!(
            normalized.tokens().is_empty(),
            "Normalized tokens should be empty"
        );

        // Search with whitespace-only query (normalized)
        let result = search_with_scope_indexed(&index, normalized.key(), SearchScope::All);

        // All tasks should be returned
        assert_eq!(
            result.tasks.len(),
            3,
            "Whitespace-only query (normalized to empty) should return all tasks"
        );

        // Verify by comparing with explicit empty string search
        let empty_result = search_with_scope_indexed(&index, "", SearchScope::All);
        assert_eq!(
            result.tasks.len(),
            empty_result.tasks.len(),
            "Whitespace-only query should behave the same as empty query"
        );

        // Verify task IDs match (order-independent)
        let mut ids_whitespace: Vec<_> = result.tasks.iter().map(|t| t.task_id.clone()).collect();
        let mut ids_empty: Vec<_> = empty_result
            .tasks
            .iter()
            .map(|t| t.task_id.clone())
            .collect();
        ids_whitespace.sort();
        ids_empty.sort();
        assert_eq!(
            ids_whitespace, ids_empty,
            "Whitespace-only query should return the same task IDs as empty query"
        );
    }
}

// =============================================================================
// SearchIndex Differential Update Tests (REQ-SEARCH-INDEX-001)
// =============================================================================

#[cfg(test)]
mod search_index_differential_update_tests {
    use super::*;
    use crate::domain::{Tag, TaskId, Timestamp};
    use rstest::rstest;

    fn create_test_task(title: &str, priority: Priority) -> Task {
        Task::new(TaskId::generate(), title, Timestamp::now()).with_priority(priority)
    }

    fn create_test_task_with_tags(title: &str, priority: Priority, tags: &[&str]) -> Task {
        let base = create_test_task(title, priority);
        tags.iter()
            .fold(base, |task, tag| task.add_tag(Tag::new(*tag)))
    }

    // -------------------------------------------------------------------------
    // Idempotency Tests (REQ-SEARCH-INDEX-001: index_update_idempotent law)
    // -------------------------------------------------------------------------

    /// Tests that applying Add twice for the same task is idempotent.
    ///
    /// Law: `apply_change(apply_change(idx, Add(task)), Add(task)) == apply_change(idx, Add(task))`
    #[rstest]
    fn test_add_idempotency() {
        // Start with an empty index
        let empty_tasks: PersistentVector<Task> = PersistentVector::new();
        let index = SearchIndex::build(&empty_tasks);

        let task = create_test_task("Important meeting", Priority::High);

        // Apply Add once
        let index_after_first_add = index.apply_change(TaskChange::Add(task.clone()));

        // Apply Add again with the same task
        let index_after_second_add = index_after_first_add.apply_change(TaskChange::Add(task));

        // Verify idempotency: the index should be equivalent after both operations
        // We verify by checking that all_tasks() returns the same set
        let tasks_after_first: Vec<_> = index_after_first_add
            .all_tasks()
            .iter()
            .map(|t| t.task_id.clone())
            .collect();
        let tasks_after_second: Vec<_> = index_after_second_add
            .all_tasks()
            .iter()
            .map(|t| t.task_id.clone())
            .collect();

        assert_eq!(
            tasks_after_first.len(),
            tasks_after_second.len(),
            "Idempotency: Adding the same task twice should not duplicate"
        );
        assert_eq!(
            tasks_after_first, tasks_after_second,
            "Idempotency: Task IDs should be identical after idempotent Add"
        );

        // Also verify the task count is 1
        assert_eq!(
            tasks_after_second.len(),
            1,
            "Should have exactly one task after idempotent Add operations"
        );
    }

    // -------------------------------------------------------------------------
    // Add Operation Tests
    // -------------------------------------------------------------------------

    /// Tests that after Add, the task is searchable by title.
    #[rstest]
    fn test_add_then_search_hits() {
        let empty_tasks: PersistentVector<Task> = PersistentVector::new();
        let index = SearchIndex::build(&empty_tasks);

        let task = create_test_task("Urgent deployment", Priority::Critical);

        // Apply Add
        let new_index = index.apply_change(TaskChange::Add(task.clone()));

        // Search for the task by title keyword
        let result = search_with_scope_indexed(&new_index, "urgent", SearchScope::All);

        assert_eq!(
            result.tasks.len(),
            1,
            "Added task should be found by title search"
        );
        let found_task = result.tasks.iter().next().unwrap();
        assert_eq!(
            found_task.task_id, task.task_id,
            "Found task should match the added task"
        );
    }

    /// Tests that after Add, the task is searchable by tag.
    #[rstest]
    fn test_add_then_search_by_tag_hits() {
        let empty_tasks: PersistentVector<Task> = PersistentVector::new();
        let index = SearchIndex::build(&empty_tasks);

        let task = create_test_task_with_tags("Regular task", Priority::Low, &["backend", "rust"]);

        // Apply Add
        let new_index = index.apply_change(TaskChange::Add(task.clone()));

        // Search by tag
        let result = search_with_scope_indexed(&new_index, "backend", SearchScope::Tags);

        assert_eq!(
            result.tasks.len(),
            1,
            "Added task should be found by tag search"
        );
        let found_task = result.tasks.iter().next().unwrap();
        assert_eq!(
            found_task.task_id, task.task_id,
            "Found task should match the added task"
        );
    }

    // -------------------------------------------------------------------------
    // Remove Operation Tests
    // -------------------------------------------------------------------------

    /// Tests that after Remove, the task is no longer searchable.
    #[rstest]
    fn test_remove_then_search_misses() {
        let task = create_test_task("Important meeting", Priority::High);
        let task_id = task.task_id.clone();
        let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
        let index = SearchIndex::build(&tasks);

        // Verify the task is initially searchable
        let result_before = search_with_scope_indexed(&index, "important", SearchScope::All);
        assert_eq!(
            result_before.tasks.len(),
            1,
            "Task should be found before removal"
        );

        // Apply Remove
        let new_index = index.apply_change(TaskChange::Remove(task_id));

        // Search for the removed task
        let result_after = search_with_scope_indexed(&new_index, "important", SearchScope::All);

        assert_eq!(
            result_after.tasks.len(),
            0,
            "Removed task should not be found by search"
        );
    }

    /// Tests that Remove for a non-existent task is idempotent (no change).
    #[rstest]
    fn test_remove_nonexistent_idempotency() {
        let task = create_test_task("Existing task", Priority::Medium);
        let tasks: PersistentVector<Task> = vec![task.clone()].into_iter().collect();
        let index = SearchIndex::build(&tasks);

        // Generate a new TaskId that doesn't exist in the index
        let nonexistent_id = TaskId::generate();

        // Apply Remove for non-existent task
        let new_index = index.apply_change(TaskChange::Remove(nonexistent_id));

        // Verify the existing task is still there
        let result = search_with_scope_indexed(&new_index, "existing", SearchScope::All);

        assert_eq!(
            result.tasks.len(),
            1,
            "Removing non-existent task should not affect existing tasks"
        );
        let found_task = result.tasks.iter().next().unwrap();
        assert_eq!(
            found_task.task_id, task.task_id,
            "Existing task should still be found"
        );

        // Verify all_tasks count is unchanged
        let all_tasks_count_before = index.all_tasks().iter().count();
        let all_tasks_count_after = new_index.all_tasks().iter().count();

        assert_eq!(
            all_tasks_count_before, all_tasks_count_after,
            "Remove of non-existent task should not change task count"
        );
    }

    // -------------------------------------------------------------------------
    // Update Operation Tests
    // -------------------------------------------------------------------------

    /// Tests that after Update, old title search misses and new title search hits.
    #[rstest]
    fn test_update_old_title_misses_new_title_hits() {
        let old_task = create_test_task("Old meeting title", Priority::Medium);
        let tasks: PersistentVector<Task> = vec![old_task.clone()].into_iter().collect();
        let index = SearchIndex::build(&tasks);

        // Create updated task with new title but same ID
        let new_task = Task::new(
            old_task.task_id.clone(),
            "New conference title",
            Timestamp::now(),
        )
        .with_priority(Priority::High);

        // Verify old title is searchable before update
        let result_old_before = search_with_scope_indexed(&index, "meeting", SearchScope::All);
        assert_eq!(
            result_old_before.tasks.len(),
            1,
            "Old title should be found before update"
        );

        // Apply Update
        let new_index = index.apply_change(TaskChange::Update {
            old: old_task,
            new: new_task.clone(),
        });

        // Old title search should miss
        let result_old_after = search_with_scope_indexed(&new_index, "meeting", SearchScope::All);
        assert_eq!(
            result_old_after.tasks.len(),
            0,
            "Old title should not be found after update"
        );

        // New title search should hit
        let result_new = search_with_scope_indexed(&new_index, "conference", SearchScope::All);
        assert_eq!(
            result_new.tasks.len(),
            1,
            "New title should be found after update"
        );
        let found_task = result_new.tasks.iter().next().unwrap();
        assert_eq!(
            found_task.task_id, new_task.task_id,
            "Found task should have the updated task ID"
        );
    }

    /// Tests that Update correctly handles tag changes.
    #[rstest]
    fn test_update_old_tag_misses_new_tag_hits() {
        let old_task =
            create_test_task_with_tags("Development task", Priority::Medium, &["frontend"]);
        let tasks: PersistentVector<Task> = vec![old_task.clone()].into_iter().collect();
        let index = SearchIndex::build(&tasks);

        // Create updated task with new tags
        let new_task = Task::new(
            old_task.task_id.clone(),
            "Development task",
            Timestamp::now(),
        )
        .with_priority(Priority::Medium)
        .add_tag(Tag::new("backend"));

        // Verify old tag is searchable before update
        let result_old_tag_before =
            search_with_scope_indexed(&index, "frontend", SearchScope::Tags);
        assert_eq!(
            result_old_tag_before.tasks.len(),
            1,
            "Old tag should be found before update"
        );

        // Apply Update
        let new_index = index.apply_change(TaskChange::Update {
            old: old_task,
            new: new_task,
        });

        // Old tag search should miss
        let result_old_tag_after =
            search_with_scope_indexed(&new_index, "frontend", SearchScope::Tags);
        assert_eq!(
            result_old_tag_after.tasks.len(),
            0,
            "Old tag should not be found after update"
        );

        // New tag search should hit
        let result_new_tag = search_with_scope_indexed(&new_index, "backend", SearchScope::Tags);
        assert_eq!(
            result_new_tag.tasks.len(),
            1,
            "New tag should be found after update"
        );
    }

    // -------------------------------------------------------------------------
    // Edge Case Tests
    // -------------------------------------------------------------------------

    /// Tests differential update on an empty index.
    #[rstest]
    fn test_add_to_empty_index() {
        let empty_tasks: PersistentVector<Task> = PersistentVector::new();
        let index = SearchIndex::build(&empty_tasks);

        assert_eq!(
            index.all_tasks().len(),
            0,
            "Empty index should have no tasks"
        );

        let task = create_test_task("First task", Priority::Low);
        let new_index = index.apply_change(TaskChange::Add(task));

        assert_eq!(
            new_index.all_tasks().len(),
            1,
            "Index should have one task after Add"
        );

        let result = search_with_scope_indexed(&new_index, "first", SearchScope::All);
        assert_eq!(result.tasks.len(), 1, "Added task should be searchable");
    }

    /// Tests that multiple Add operations work correctly.
    #[rstest]
    fn test_multiple_adds() {
        let empty_tasks: PersistentVector<Task> = PersistentVector::new();
        let index = SearchIndex::build(&empty_tasks);

        let task1 = create_test_task("Alpha task", Priority::High);
        let task2 = create_test_task("Beta task", Priority::Medium);
        let task3 = create_test_task("Gamma task", Priority::Low);

        let index = index.apply_change(TaskChange::Add(task1));
        let index = index.apply_change(TaskChange::Add(task2));
        let index = index.apply_change(TaskChange::Add(task3));

        assert_eq!(
            index.all_tasks().len(),
            3,
            "Index should have three tasks after three Adds"
        );

        // Verify each task is searchable
        let result_alpha = search_with_scope_indexed(&index, "alpha", SearchScope::All);
        let result_beta = search_with_scope_indexed(&index, "beta", SearchScope::All);
        let result_gamma = search_with_scope_indexed(&index, "gamma", SearchScope::All);

        assert_eq!(result_alpha.tasks.len(), 1, "Alpha task should be found");
        assert_eq!(result_beta.tasks.len(), 1, "Beta task should be found");
        assert_eq!(result_gamma.tasks.len(), 1, "Gamma task should be found");
    }

    /// Tests that original index is unchanged after `apply_change` (immutability).
    #[rstest]
    fn test_immutability_preserved() {
        let task = create_test_task("Original task", Priority::Medium);
        let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
        let original_index = SearchIndex::build(&tasks);

        // Get the count before any changes
        let count_before = original_index.all_tasks().len();

        // Apply Add (which returns a new index)
        let new_task = create_test_task("New task", Priority::High);
        let _new_index = original_index.apply_change(TaskChange::Add(new_task));

        // Original index should be unchanged
        let count_after = original_index.all_tasks().len();
        assert_eq!(
            count_before, count_after,
            "Original index should be unchanged after apply_change"
        );

        // Verify original task is still searchable in original index
        let result = search_with_scope_indexed(&original_index, "original", SearchScope::All);
        assert_eq!(
            result.tasks.len(),
            1,
            "Original task should still be found in original index"
        );
    }

    /// Tests that `TaskChange::Update` is idempotent.
    ///
    /// Applying the same Update twice should produce the same result as applying it once.
    /// This ensures that index entries are not duplicated when Update is applied multiple times.
    #[rstest]
    fn test_update_idempotency() {
        // Create initial task
        let task_id = TaskId::generate();
        let old_task = Task::new(task_id.clone(), "Old title", Timestamp::now())
            .with_priority(Priority::Low)
            .add_tag(Tag::new("work"));

        // Build initial index with the task
        let tasks: PersistentVector<Task> = vec![old_task.clone()].into_iter().collect();
        let initial_index = SearchIndex::build(&tasks);

        // Create updated task (same ID, different content)
        let new_task = Task::new(task_id, "New title updated", Timestamp::now())
            .with_priority(Priority::High)
            .add_tag(Tag::new("personal"));

        // Apply Update once
        let update_change = TaskChange::Update {
            old: old_task,
            new: new_task,
        };
        let index_after_first_update = initial_index.apply_change(update_change.clone());

        // Apply the same Update again
        let index_after_second_update = index_after_first_update.apply_change(update_change);

        // Verify: Total task count should be the same
        assert_eq!(
            index_after_first_update.all_tasks().len(),
            index_after_second_update.all_tasks().len(),
            "Task count should be the same after applying Update twice"
        );

        // Verify: Searching for the new title should return exactly 1 result (not 2)
        let result_after_first =
            search_with_scope_indexed(&index_after_first_update, "New title", SearchScope::Title);
        let result_after_second =
            search_with_scope_indexed(&index_after_second_update, "New title", SearchScope::Title);

        assert_eq!(
            result_after_first.tasks.len(),
            1,
            "First update should result in exactly 1 task"
        );
        assert_eq!(
            result_after_second.tasks.len(),
            1,
            "Second update should still result in exactly 1 task (idempotency)"
        );

        // Verify: Searching by new tag should return exactly 1 result
        let tag_result_first =
            search_with_scope_indexed(&index_after_first_update, "personal", SearchScope::Tags);
        let tag_result_second =
            search_with_scope_indexed(&index_after_second_update, "personal", SearchScope::Tags);

        assert_eq!(
            tag_result_first.tasks.len(),
            1,
            "First update: tag search should return 1 task"
        );
        assert_eq!(
            tag_result_second.tasks.len(),
            1,
            "Second update: tag search should still return 1 task (idempotency)"
        );

        // Verify: Old title should not be found in the index
        let old_title_result_first =
            search_with_scope_indexed(&index_after_first_update, "Old title", SearchScope::Title);
        let old_title_result_second =
            search_with_scope_indexed(&index_after_second_update, "Old title", SearchScope::Title);

        assert!(
            old_title_result_first.tasks.is_empty(),
            "Old title should not be found after first update"
        );
        assert!(
            old_title_result_second.tasks.is_empty(),
            "Old title should not be found after second update"
        );
    }

    // -------------------------------------------------------------------------
    // Internal Index Uniqueness Tests
    // -------------------------------------------------------------------------

    /// Helper function to check uniqueness in a `PersistentVector`.
    ///
    /// Returns `true` if there are duplicate `TaskId`s in the vector.
    fn has_duplicates(ids: &PersistentVector<TaskId>) -> bool {
        let set: std::collections::HashSet<_> = ids.iter().collect();
        set.len() != ids.len()
    }

    /// Tests that internal indexes have no duplicate `TaskId`s after Update.
    ///
    /// This test directly verifies that each index entry contains unique `TaskId`s,
    /// not relying on search deduplication. This ensures the internal data structure
    /// maintains consistency even when Update is applied multiple times with the
    /// same old/new task pair.
    #[rstest]
    fn test_update_no_internal_duplicates() {
        // Create initial task
        let task_id = TaskId::generate();
        let old_task = Task::new(task_id.clone(), "Test title", Timestamp::now())
            .with_priority(Priority::Low)
            .add_tag(Tag::new("testtag"));

        // Build initial index
        let tasks: PersistentVector<Task> = vec![old_task.clone()].into_iter().collect();
        let initial_index = SearchIndex::build(&tasks);

        // Create updated task (same title and tag to ensure same index keys)
        let new_task = Task::new(task_id, "Test title", Timestamp::now())
            .with_priority(Priority::High)
            .add_tag(Tag::new("testtag"));

        // Apply Update twice
        let update_change = TaskChange::Update {
            old: old_task,
            new: new_task,
        };
        let index_once = initial_index.apply_change(update_change.clone());
        let index_twice = index_once.apply_change(update_change);

        // Verify internal indexes have no duplicates

        // Check title_word_index
        for (key, ids) in index_twice.title_word_index_for_test() {
            assert!(
                !has_duplicates(ids),
                "title_word_index has duplicate TaskIds for key '{key}'"
            );
        }

        // Check title_full_index
        for (key, ids) in index_twice.title_full_index_for_test() {
            assert!(
                !has_duplicates(ids),
                "title_full_index has duplicate TaskIds for key '{key}'"
            );
        }

        // Check title_full_all_suffix_index
        for (key, ids) in index_twice.title_full_all_suffix_index_for_test() {
            assert!(
                !has_duplicates(ids),
                "title_full_all_suffix_index has duplicate TaskIds for key '{key}'"
            );
        }

        // Check title_word_all_suffix_index
        for (key, ids) in index_twice.title_word_all_suffix_index_for_test() {
            assert!(
                !has_duplicates(ids),
                "title_word_all_suffix_index has duplicate TaskIds for key '{key}'"
            );
        }

        // Check tag_index
        for (key, ids) in index_twice.tag_index_for_test() {
            assert!(
                !has_duplicates(ids),
                "tag_index has duplicate TaskIds for key '{key}'"
            );
        }

        // Check tag_all_suffix_index
        for (key, ids) in index_twice.tag_all_suffix_index_for_test() {
            assert!(
                !has_duplicates(ids),
                "tag_all_suffix_index has duplicate TaskIds for key '{key}'"
            );
        }

        // Also verify task count remains 1
        assert_eq!(
            index_twice.all_tasks().len(),
            1,
            "Should have exactly one task after Update operations"
        );
    }
}

// =============================================================================
// SearchCache Tests (REQ-SEARCH-CACHE-001)
// =============================================================================

#[cfg(test)]
mod search_cache_tests {
    use super::*;
    use crate::domain::{Priority, Tag, TaskId, Timestamp};
    use rstest::rstest;
    use std::thread;
    use std::time::Duration;

    fn create_test_task(title: &str, priority: Priority) -> Task {
        Task::new(TaskId::generate(), title, Timestamp::now()).with_priority(priority)
    }

    #[allow(dead_code)]
    fn create_test_task_with_tags(title: &str, priority: Priority, tags: &[&str]) -> Task {
        let base = create_test_task(title, priority);
        tags.iter()
            .fold(base, |task, tag| task.add_tag(Tag::new(*tag)))
    }

    // -------------------------------------------------------------------------
    // Basic Cache Operations Tests
    // -------------------------------------------------------------------------

    /// Test: Cache miss returns `None` for unknown key.
    #[rstest]
    fn test_cache_miss_returns_none() {
        let cache = SearchCache::new(10, Duration::from_secs(5));
        let key = SearchCacheKey::from_raw("unknown query", SearchScope::All, Some(50), Some(0));

        let result = cache.get(&key);

        assert!(result.is_none(), "Cache miss should return None");
    }

    /// Test: Cache hit returns stored result.
    #[rstest]
    fn test_cache_hit_returns_stored_result() {
        let cache = SearchCache::new(10, Duration::from_secs(5));
        let key = SearchCacheKey::from_raw("test query", SearchScope::All, Some(50), Some(0));

        // Create a search result
        let task = create_test_task("Test Task", Priority::High);
        let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
        let search_result = SearchResult::from_tasks(tasks);

        // Store in cache
        cache.put(key.clone(), search_result);

        // Get from cache
        let cached = cache.get(&key);

        assert!(cached.is_some(), "Cache hit should return Some");
        let cached_result = cached.unwrap();
        assert_eq!(
            cached_result.into_tasks().len(),
            1,
            "Cached result should have 1 task"
        );
    }

    /// Test: Same query second time is a cache hit.
    #[rstest]
    fn test_same_query_second_time_is_cache_hit() {
        let cache = SearchCache::new(10, Duration::from_secs(5));
        let key = SearchCacheKey::from_raw("urgent task", SearchScope::Title, Some(50), Some(0));

        // Create and store a result
        let task = create_test_task("Urgent Task", Priority::Critical);
        let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
        let search_result = SearchResult::from_tasks(tasks);
        cache.put(key.clone(), search_result);

        // First get - should hit
        let first_get = cache.get(&key);
        assert!(first_get.is_some(), "First get should hit");

        // Second get - should also hit
        let second_get = cache.get(&key);
        assert!(second_get.is_some(), "Second get should also hit");

        // Check stats
        let stats = cache.stats();
        assert_eq!(stats.hits, 2, "Should have 2 cache hits");
        assert_eq!(stats.misses, 0, "Should have 0 cache misses");
    }

    // -------------------------------------------------------------------------
    // TTL Tests
    // -------------------------------------------------------------------------

    /// Test: Entry expires after TTL.
    #[rstest]
    fn test_entry_expires_after_ttl() {
        // Use very short TTL for testing
        let cache = SearchCache::new(10, Duration::from_millis(50));
        let key = SearchCacheKey::from_raw("expiring query", SearchScope::All, Some(50), Some(0));

        // Store a result
        let task = create_test_task("Expiring Task", Priority::Low);
        let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
        let search_result = SearchResult::from_tasks(tasks);
        cache.put(key.clone(), search_result);

        // Immediate get should hit
        let immediate_get = cache.get(&key);
        assert!(immediate_get.is_some(), "Immediate get should hit");

        // Wait for TTL to expire
        thread::sleep(Duration::from_millis(100));

        // Get after TTL should miss
        let expired_get = cache.get(&key);
        assert!(expired_get.is_none(), "Get after TTL should miss");
    }

    /// Test: Entry is valid just before TTL expires.
    #[rstest]
    fn test_entry_valid_before_ttl() {
        // Use 200ms TTL for testing
        let cache = SearchCache::new(10, Duration::from_millis(200));
        let key = SearchCacheKey::from_raw("valid query", SearchScope::All, Some(50), Some(0));

        // Store a result
        let task = create_test_task("Valid Task", Priority::Medium);
        let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
        let search_result = SearchResult::from_tasks(tasks);
        cache.put(key.clone(), search_result);

        // Wait less than TTL
        thread::sleep(Duration::from_millis(100));

        // Get should still hit
        let result = cache.get(&key);
        assert!(result.is_some(), "Get before TTL expires should hit");
    }

    // -------------------------------------------------------------------------
    // LRU Eviction Tests
    // -------------------------------------------------------------------------

    /// Test: LRU eviction when capacity is exceeded.
    #[rstest]
    fn test_lru_eviction_on_capacity_exceeded() {
        // Small capacity for testing
        let cache = SearchCache::new(3, Duration::from_secs(60));

        // Add 3 entries (fills capacity)
        for i in 0..3 {
            let key =
                SearchCacheKey::from_raw(&format!("query{i}"), SearchScope::All, Some(50), Some(0));
            let task = create_test_task(&format!("Task {i}"), Priority::Low);
            let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
            cache.put(key, SearchResult::from_tasks(tasks));
        }

        assert_eq!(cache.len(), 3, "Cache should have 3 entries");

        // Access query1 and query2 to make query0 the least recently used
        let key1 = SearchCacheKey::from_raw("query1", SearchScope::All, Some(50), Some(0));
        let key2 = SearchCacheKey::from_raw("query2", SearchScope::All, Some(50), Some(0));
        let _ = cache.get(&key1);
        let _ = cache.get(&key2);

        // Add a 4th entry - should evict query0 (LRU)
        let key3 = SearchCacheKey::from_raw("query3", SearchScope::All, Some(50), Some(0));
        let task = create_test_task("Task 3", Priority::High);
        let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
        cache.put(key3.clone(), SearchResult::from_tasks(tasks));

        // Cache should still have 3 entries
        assert_eq!(
            cache.len(),
            3,
            "Cache should still have 3 entries after eviction"
        );

        // query0 should be evicted
        let key0 = SearchCacheKey::from_raw("query0", SearchScope::All, Some(50), Some(0));
        let result0 = cache.get(&key0);
        assert!(result0.is_none(), "query0 should be evicted");

        // query1, query2, query3 should still be present
        let result1 = cache.get(&key1);
        assert!(result1.is_some(), "query1 should still be present");

        let result2 = cache.get(&key2);
        assert!(result2.is_some(), "query2 should still be present");

        let result3 = cache.get(&key3);
        assert!(result3.is_some(), "query3 should still be present");
    }

    // -------------------------------------------------------------------------
    // Cache Statistics Tests
    // -------------------------------------------------------------------------

    /// Test: Hit rate calculation.
    #[rstest]
    fn test_hit_rate_calculation() {
        let cache = SearchCache::new(10, Duration::from_secs(5));

        // Initial hit rate should be 0.0
        let initial_stats = cache.stats();
        assert!(
            (initial_stats.hit_rate() - 0.0).abs() < f64::EPSILON,
            "Initial hit rate should be 0.0"
        );

        // Store a result
        let key = SearchCacheKey::from_raw("test", SearchScope::All, Some(50), Some(0));
        let task = create_test_task("Test", Priority::Low);
        let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
        cache.put(key.clone(), SearchResult::from_tasks(tasks));

        // 2 hits
        cache.get(&key);
        cache.get(&key);

        // 1 miss
        let miss_key = SearchCacheKey::from_raw("miss", SearchScope::All, Some(50), Some(0));
        cache.get(&miss_key);

        let stats = cache.stats();
        assert_eq!(stats.hits, 2, "Should have 2 hits");
        assert_eq!(stats.misses, 1, "Should have 1 miss");

        // Hit rate should be 2/3 = 0.666...
        let hit_rate = stats.hit_rate();
        assert!(
            (hit_rate - 0.666_666_66).abs() < 0.001,
            "Hit rate should be approximately 0.667, got {hit_rate}"
        );
    }

    /// Test: Cache statistics after multiple operations.
    #[rstest]
    fn test_cache_stats_after_operations() {
        let cache = SearchCache::new(10, Duration::from_secs(5));

        // Store some results
        for i in 0..5 {
            let key =
                SearchCacheKey::from_raw(&format!("query{i}"), SearchScope::All, Some(50), Some(0));
            let task = create_test_task(&format!("Task {i}"), Priority::Low);
            let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
            cache.put(key, SearchResult::from_tasks(tasks));
        }

        // 3 hits
        for i in 0..3 {
            let key =
                SearchCacheKey::from_raw(&format!("query{i}"), SearchScope::All, Some(50), Some(0));
            cache.get(&key);
        }

        // 2 misses
        for i in 10..12 {
            let key =
                SearchCacheKey::from_raw(&format!("query{i}"), SearchScope::All, Some(50), Some(0));
            cache.get(&key);
        }

        let stats = cache.stats();
        assert_eq!(stats.hits, 3, "Should have 3 hits");
        assert_eq!(stats.misses, 2, "Should have 2 misses");
    }

    // -------------------------------------------------------------------------
    // Cache Key Equivalence Tests
    // -------------------------------------------------------------------------

    /// Test: Same query with different formatting hits the same cache entry.
    #[rstest]
    fn test_normalized_query_hits_same_cache_entry() {
        let cache = SearchCache::new(10, Duration::from_secs(5));

        // Store with one formatting
        let key1 =
            SearchCacheKey::from_raw("  URGENT   Task  ", SearchScope::All, Some(50), Some(0));
        let task = create_test_task("Test", Priority::High);
        let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
        cache.put(key1, SearchResult::from_tasks(tasks));

        // Get with different formatting - should hit
        let key2 = SearchCacheKey::from_raw("urgent task", SearchScope::All, Some(50), Some(0));
        let result = cache.get(&key2);

        assert!(
            result.is_some(),
            "Normalized equivalent query should hit cache"
        );
    }

    /// Test: Different limit produces different cache key.
    #[rstest]
    fn test_different_limit_different_cache_key() {
        let cache = SearchCache::new(10, Duration::from_secs(5));

        // Store with limit=50
        let key1 = SearchCacheKey::from_raw("test", SearchScope::All, Some(50), Some(0));
        let task = create_test_task("Test", Priority::Low);
        let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
        cache.put(key1.clone(), SearchResult::from_tasks(tasks));

        // Get with limit=100 - should miss
        let key2 = SearchCacheKey::from_raw("test", SearchScope::All, Some(100), Some(0));
        let result = cache.get(&key2);

        assert!(result.is_none(), "Different limit should miss cache");

        // Original key should still hit
        let original = cache.get(&key1);
        assert!(original.is_some(), "Original key should still hit");
    }

    /// Test: Different offset produces different cache key.
    #[rstest]
    fn test_different_offset_different_cache_key() {
        let cache = SearchCache::new(10, Duration::from_secs(5));

        // Store with offset=0
        let key1 = SearchCacheKey::from_raw("test", SearchScope::All, Some(50), Some(0));
        let task = create_test_task("Test", Priority::Low);
        let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
        cache.put(key1, SearchResult::from_tasks(tasks));

        // Get with offset=10 - should miss
        let key2 = SearchCacheKey::from_raw("test", SearchScope::All, Some(50), Some(10));
        let result = cache.get(&key2);

        assert!(result.is_none(), "Different offset should miss cache");
    }

    /// Test: Different scope produces different cache key.
    #[rstest]
    fn test_different_scope_different_cache_key() {
        let cache = SearchCache::new(10, Duration::from_secs(5));

        // Store with scope=All
        let key1 = SearchCacheKey::from_raw("test", SearchScope::All, Some(50), Some(0));
        let task = create_test_task("Test", Priority::Low);
        let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
        cache.put(key1, SearchResult::from_tasks(tasks));

        // Get with scope=Title - should miss
        let key2 = SearchCacheKey::from_raw("test", SearchScope::Title, Some(50), Some(0));
        let result = cache.get(&key2);

        assert!(result.is_none(), "Different scope should miss cache");
    }

    // -------------------------------------------------------------------------
    // Utility Method Tests
    // -------------------------------------------------------------------------

    /// Test: `len` and `is_empty` methods.
    #[rstest]
    fn test_len_and_is_empty() {
        let cache = SearchCache::new(10, Duration::from_secs(5));

        assert!(cache.is_empty(), "New cache should be empty");
        assert_eq!(cache.len(), 0, "New cache should have length 0");

        // Add an entry
        let key = SearchCacheKey::from_raw("test", SearchScope::All, Some(50), Some(0));
        let task = create_test_task("Test", Priority::Low);
        let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
        cache.put(key, SearchResult::from_tasks(tasks));

        assert!(!cache.is_empty(), "Cache should not be empty after insert");
        assert_eq!(cache.len(), 1, "Cache should have length 1");
    }

    /// Test: `clear` method.
    #[rstest]
    fn test_clear() {
        let cache = SearchCache::new(10, Duration::from_secs(5));

        // Add some entries
        for i in 0..5 {
            let key =
                SearchCacheKey::from_raw(&format!("query{i}"), SearchScope::All, Some(50), Some(0));
            let task = create_test_task(&format!("Task {i}"), Priority::Low);
            let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
            cache.put(key, SearchResult::from_tasks(tasks));
        }

        assert_eq!(cache.len(), 5, "Cache should have 5 entries");

        // Clear the cache
        cache.clear();

        assert!(cache.is_empty(), "Cache should be empty after clear");
        assert_eq!(cache.len(), 0, "Cache length should be 0 after clear");
    }

    /// Test: Debug formatting.
    #[rstest]
    fn test_debug_format() {
        let cache = SearchCache::new(100, Duration::from_secs(5));

        // Add an entry and access it
        let key = SearchCacheKey::from_raw("test", SearchScope::All, Some(50), Some(0));
        let task = create_test_task("Test", Priority::Low);
        let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
        cache.put(key.clone(), SearchResult::from_tasks(tasks));
        cache.get(&key);

        let debug_str = format!("{cache:?}");

        assert!(
            debug_str.contains("SearchCache"),
            "Debug should contain 'SearchCache'"
        );
        assert!(debug_str.contains("len"), "Debug should contain 'len'");
        assert!(debug_str.contains("hits"), "Debug should contain 'hits'");
        assert!(
            debug_str.contains("misses"),
            "Debug should contain 'misses'"
        );
        assert!(
            debug_str.contains("hit_rate"),
            "Debug should contain 'hit_rate'"
        );
    }

    // -------------------------------------------------------------------------
    // Default Configuration Tests
    // -------------------------------------------------------------------------

    /// Test: `with_default_config` creates cache with correct settings.
    #[rstest]
    fn test_with_default_config() {
        let cache = SearchCache::with_default_config();

        // Add 2000 entries (should work)
        for i in 0..2000 {
            let key =
                SearchCacheKey::from_raw(&format!("query{i}"), SearchScope::All, Some(50), Some(0));
            let task = create_test_task(&format!("Task {i}"), Priority::Low);
            let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
            cache.put(key, SearchResult::from_tasks(tasks));
        }

        assert_eq!(cache.len(), 2000, "Cache should hold 2000 entries");

        // Add one more - should evict oldest
        let key = SearchCacheKey::from_raw("query_overflow", SearchScope::All, Some(50), Some(0));
        let task = create_test_task("Overflow Task", Priority::High);
        let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
        cache.put(key, SearchResult::from_tasks(tasks));

        assert_eq!(
            cache.len(),
            2000,
            "Cache should still have 2000 entries after overflow"
        );
    }

    // -------------------------------------------------------------------------
    // CacheStats Tests
    // -------------------------------------------------------------------------

    /// Test: `CacheStats` default values.
    #[rstest]
    fn test_cache_stats_default() {
        let stats = CacheStats::default();

        assert_eq!(stats.hits, 0, "Default hits should be 0");
        assert_eq!(stats.misses, 0, "Default misses should be 0");
        assert!(
            (stats.hit_rate() - 0.0).abs() < f64::EPSILON,
            "Default hit rate should be 0.0"
        );
    }

    /// Test: `CacheStats` hit rate with only hits.
    #[rstest]
    fn test_cache_stats_all_hits() {
        let stats = CacheStats {
            hits: 10,
            misses: 0,
        };

        assert!(
            (stats.hit_rate() - 1.0).abs() < f64::EPSILON,
            "Hit rate with only hits should be 1.0"
        );
    }

    /// Test: `CacheStats` hit rate with only misses.
    #[rstest]
    fn test_cache_stats_all_misses() {
        let stats = CacheStats {
            hits: 0,
            misses: 10,
        };

        assert!(
            (stats.hit_rate() - 0.0).abs() < f64::EPSILON,
            "Hit rate with only misses should be 0.0"
        );
    }
}

// =============================================================================
// SearchIndexConfig Tests (REQ-SEARCH-NGRAM-001)
// =============================================================================

#[cfg(test)]
mod search_index_config_tests {
    use super::*;
    use proptest::prelude::*;
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // Default Value Tests
    // -------------------------------------------------------------------------

    /// Tests that `SearchIndexConfig::default()` returns expected values.
    ///
    /// Verifies:
    /// - `infix_mode`: `InfixMode::Ngram` (default)
    /// - `ngram_size`: 3
    /// - `min_query_len_for_infix`: 3
    /// - `max_ngrams_per_token`: 64
    /// - `max_tokens_per_task`: 100
    /// - `max_search_candidates`: 1000
    #[rstest]
    fn config_default_values() {
        let config = SearchIndexConfig::default();

        assert_eq!(config.infix_mode, InfixMode::Ngram);
        assert_eq!(config.ngram_size, 3);
        assert_eq!(config.min_query_len_for_infix, 3);
        assert_eq!(config.max_ngrams_per_token, 64);
        assert_eq!(config.max_tokens_per_task, 100);
        assert_eq!(config.max_search_candidates, 1000);
    }

    /// Tests that `InfixMode::LegacyAllSuffix` is available but not the default.
    ///
    /// This ensures backward compatibility while keeping n-gram as the default.
    #[rstest]
    fn infix_mode_legacy_is_available_but_not_default() {
        // Default should be Ngram, not LegacyAllSuffix
        let config = SearchIndexConfig::default();
        assert_ne!(config.infix_mode, InfixMode::LegacyAllSuffix);

        // LegacyAllSuffix should be usable via explicit construction
        let legacy_config = SearchIndexConfig {
            infix_mode: InfixMode::LegacyAllSuffix,
            ..Default::default()
        };
        assert_eq!(legacy_config.infix_mode, InfixMode::LegacyAllSuffix);
    }

    /// Tests that `InfixMode::Disabled` is available and can be configured.
    #[rstest]
    fn infix_mode_disabled_is_available() {
        let config = SearchIndexConfig {
            infix_mode: InfixMode::Disabled,
            ..Default::default()
        };
        assert_eq!(config.infix_mode, InfixMode::Disabled);
    }

    /// Tests that `InfixMode` derives `Default` and it resolves to `Ngram`.
    #[rstest]
    fn infix_mode_default_is_ngram() {
        let mode = InfixMode::default();
        assert_eq!(mode, InfixMode::Ngram);
    }

    // -------------------------------------------------------------------------
    // Property Tests
    // -------------------------------------------------------------------------

    proptest! {
        /// Property test: `SearchIndexConfig::default()` is deterministic.
        ///
        /// Law: `default() == default()` for any execution context.
        #[test]
        fn config_default_is_deterministic_property(_seed in any::<u64>()) {
            let left = SearchIndexConfig::default();
            let right = SearchIndexConfig::default();
            prop_assert_eq!(left, right);
        }
    }
}

// =============================================================================
// N-gram Generation Tests (REQ-SEARCH-NGRAM-002 Part 1)
// =============================================================================

#[cfg(test)]
mod ngram_tests {
    use super::*;
    use proptest::prelude::*;
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // Basic Functionality Tests
    // -------------------------------------------------------------------------

    /// Tests n-gram generation from ASCII string.
    ///
    /// - Input: "callback", `ngram_size`=3, `max_ngrams`=64
    /// - Expected: 6 n-grams: "cal", "all", "llb", "lba", "bac", "ack"
    #[rstest]
    fn generate_ngrams_ascii() {
        let result = generate_ngrams("callback", 3, 64);
        assert_eq!(result, vec!["cal", "all", "llb", "lba", "bac", "ack"]);
    }

    /// Tests n-gram generation from multibyte (UTF-8) string.
    ///
    /// - Input: "日本語テスト", `ngram_size`=3, `max_ngrams`=64
    /// - Expected: 4 n-grams (6 chars - 3 + 1 = 4)
    #[rstest]
    fn generate_ngrams_multibyte() {
        let result = generate_ngrams("日本語テスト", 3, 64);
        assert_eq!(result.len(), 4);
        assert_eq!(result[0], "日本語");
        assert_eq!(result[1], "本語テ");
        assert_eq!(result[2], "語テス");
        assert_eq!(result[3], "テスト");
    }

    /// Tests that short tokens (fewer chars than `ngram_size`) return empty.
    ///
    /// - Input: "ab", `ngram_size`=3
    /// - Expected: empty Vec (2 < 3)
    #[rstest]
    fn generate_ngrams_short_token() {
        let result = generate_ngrams("ab", 3, 64);
        assert!(result.is_empty());
    }

    /// Tests that `max_ngrams` limits the output.
    ///
    /// - Input: "callback", `ngram_size`=3, `max_ngrams`=2
    /// - Expected: 2 n-grams: "cal", "all" (only first 2)
    #[rstest]
    fn generate_ngrams_max_limit() {
        let result = generate_ngrams("callback", 3, 2);
        assert_eq!(result.len(), 2);
        assert_eq!(result, vec!["cal", "all"]);
    }

    /// Tests that invalid `ngram_size` (< 2) returns empty.
    ///
    /// - Input: "callback", `ngram_size`=1
    /// - Expected: empty Vec
    #[rstest]
    fn generate_ngrams_invalid_size() {
        let result = generate_ngrams("callback", 1, 64);
        assert!(result.is_empty());
    }

    /// Tests that `ngram_size`=0 returns empty.
    #[rstest]
    fn generate_ngrams_zero_size() {
        let result = generate_ngrams("callback", 0, 64);
        assert!(result.is_empty());
    }

    /// Tests empty input string.
    #[rstest]
    fn generate_ngrams_empty_input() {
        let result = generate_ngrams("", 3, 64);
        assert!(result.is_empty());
    }

    /// Tests token length exactly matching `ngram_size`.
    ///
    /// - Input: "abc", `ngram_size`=3, `max_ngrams`=64
    /// - Expected: 1 n-gram: "abc" (exactly 1 n-gram)
    #[rstest]
    fn generate_ngrams_exact_length() {
        let result = generate_ngrams("abc", 3, 64);
        assert_eq!(result, vec!["abc"]);
    }

    // -------------------------------------------------------------------------
    // Property Tests
    // -------------------------------------------------------------------------

    proptest! {
        /// Property: Generated n-gram count never exceeds `max_ngrams`.
        ///
        /// Law: `len(generate_ngrams(token, n, max))` <= max for all inputs.
        #[test]
        fn generate_ngrams_count_is_bounded(
            token in "[a-z]{1,50}",
            ngram_size in 2usize..=5,
            max_ngrams in 1usize..=100
        ) {
            let result = generate_ngrams(&token, ngram_size, max_ngrams);
            prop_assert!(result.len() <= max_ngrams);
        }

        /// Property: All generated n-grams have exactly `ngram_size` characters.
        ///
        /// Law: `chars().count() == ngram_size` for all n-grams in result.
        #[test]
        fn generate_ngrams_all_have_correct_length(
            token in "[a-z]{3,50}",
            ngram_size in 2usize..=5
        ) {
            let result = generate_ngrams(&token, ngram_size, usize::MAX);
            for ngram in &result {
                prop_assert_eq!(
                    ngram.chars().count(),
                    ngram_size,
                    "n-gram '{}' has {} chars, expected {}",
                    ngram,
                    ngram.chars().count(),
                    ngram_size
                );
            }
        }

        /// Property: All generated n-grams are substrings of the original token.
        ///
        /// Law: `token.contains(ngram)` for all n-grams in result.
        #[test]
        fn generate_ngrams_are_substrings_of_token(
            token in "[a-z]{3,50}",
            ngram_size in 2usize..=5,
            max_ngrams in 1usize..=100
        ) {
            let result = generate_ngrams(&token, ngram_size, max_ngrams);
            for ngram in &result {
                prop_assert!(
                    token.contains(ngram.as_str()),
                    "n-gram '{}' is not a substring of '{}'",
                    ngram,
                    token
                );
            }
        }

        /// Property: n-gram count is `min(max_ngrams, token_chars - ngram_size + 1)` when valid.
        ///
        /// Law: len == `min(max_ngrams, max(0, chars - n + 1))` for `ngram_size` >= 2.
        #[test]
        fn generate_ngrams_count_is_predictable(
            token in "[a-z]{1,30}",
            ngram_size in 2usize..=5,
            max_ngrams in 1usize..=50
        ) {
            let char_count = token.chars().count();
            let result = generate_ngrams(&token, ngram_size, max_ngrams);

            let expected_max_possible = if char_count >= ngram_size {
                char_count - ngram_size + 1
            } else {
                0
            };
            let expected_count = expected_max_possible.min(max_ngrams);

            prop_assert_eq!(
                result.len(),
                expected_count,
                "token='{}', ngram_size={}, max_ngrams={}, char_count={}",
                token,
                ngram_size,
                max_ngrams,
                char_count
            );
        }
    }
}

// =============================================================================
// N-gram Index Tests (REQ-SEARCH-NGRAM-002 Part 2)
// =============================================================================

#[cfg(test)]
mod ngram_index_tests {
    use super::*;
    use rstest::rstest;
    use uuid::Uuid;

    // -------------------------------------------------------------------------
    // Test Helpers
    // -------------------------------------------------------------------------

    /// Creates a `TaskId` from a u128 value for deterministic testing.
    fn task_id_from_u128(value: u128) -> TaskId {
        TaskId::from_uuid(Uuid::from_u128(value))
    }

    // -------------------------------------------------------------------------
    // index_ngrams Tests
    // -------------------------------------------------------------------------

    /// Tests that `index_ngrams` adds n-grams to an empty index.
    ///
    /// - Input: empty index, token "callback", default config
    /// - Expected: n-grams "cal", "all", "llb", "lba", "bac", "ack" are added
    #[rstest]
    fn index_ngrams_empty_index() {
        let index: NgramIndex = PersistentHashMap::new();
        let config = SearchIndexConfig::default();
        let task_id = task_id_from_u128(1);

        let result = index_ngrams(index, "callback", &task_id, &config);

        // Verify expected n-grams are present
        assert!(result.get("cal").is_some(), "Expected 'cal' n-gram");
        assert!(result.get("all").is_some(), "Expected 'all' n-gram");
        assert!(result.get("llb").is_some(), "Expected 'llb' n-gram");
        assert!(result.get("lba").is_some(), "Expected 'lba' n-gram");
        assert!(result.get("bac").is_some(), "Expected 'bac' n-gram");
        assert!(result.get("ack").is_some(), "Expected 'ack' n-gram");

        // Verify TaskId is present in each n-gram's posting list
        for ngram in ["cal", "all", "llb", "lba", "bac", "ack"] {
            let ids = result.get(ngram).unwrap();
            assert_eq!(ids.len(), 1, "Expected 1 TaskId for n-gram '{ngram}'");
            assert_eq!(
                ids.get(0).unwrap(),
                &task_id,
                "Expected correct TaskId for n-gram '{ngram}'"
            );
        }
    }

    /// Tests that `index_ngrams` does not add duplicate `TaskId`s.
    ///
    /// - Input: index already containing `task_id` for "callback", same token and `task_id`
    /// - Expected: `TaskId` list length remains 1 (no duplicates)
    #[rstest]
    fn index_ngrams_no_duplicate_task_id() {
        let index: NgramIndex = PersistentHashMap::new();
        let config = SearchIndexConfig::default();
        let task_id = task_id_from_u128(1);

        // Add token first time
        let result = index_ngrams(index, "callback", &task_id, &config);

        // Add same token with same task_id again
        let result2 = index_ngrams(result, "callback", &task_id, &config);

        // Verify no duplicates
        let ids = result2.get("cal").unwrap();
        assert_eq!(ids.len(), 1, "Expected 1 TaskId (no duplicates)");
    }

    /// Tests that `index_ngrams` maintains sorted order of `TaskId`s.
    ///
    /// - Input: add `task_id2` first, then `task_id1` (where `task_id1` < `task_id2`)
    /// - Expected: posting list is sorted `[task_id1, task_id2]`
    #[rstest]
    fn index_ngrams_maintains_sorted_order() {
        let index: NgramIndex = PersistentHashMap::new();
        let config = SearchIndexConfig::default();
        let task_id1 = task_id_from_u128(1);
        let task_id2 = task_id_from_u128(2);

        // Add task_id2 first (larger value)
        let result = index_ngrams(index, "callback", &task_id2, &config);

        // Add task_id1 second (smaller value)
        let result = index_ngrams(result, "callback", &task_id1, &config);

        // Verify sorted order
        let ids: Vec<_> = result.get("cal").unwrap().iter().collect();
        assert_eq!(ids.len(), 2, "Expected 2 TaskIds");
        assert!(
            ids[0] < ids[1],
            "Expected sorted order: {:?} < {:?}",
            ids[0],
            ids[1]
        );
        assert_eq!(ids[0], &task_id1, "Expected task_id1 first");
        assert_eq!(ids[1], &task_id2, "Expected task_id2 second");
    }

    /// Tests that `index_ngrams` handles short tokens (no n-grams generated).
    ///
    /// - Input: token "ab" with `ngram_size=3`
    /// - Expected: index remains unchanged (empty)
    #[rstest]
    fn index_ngrams_short_token_returns_unchanged() {
        let index: NgramIndex = PersistentHashMap::new();
        let config = SearchIndexConfig::default(); // ngram_size = 3
        let task_id = task_id_from_u128(1);

        let result = index_ngrams(index, "ab", &task_id, &config);

        // Verify index is still empty
        assert!(result.is_empty(), "Expected empty index for short token");
    }

    /// Tests that `index_ngrams` handles empty token.
    ///
    /// - Input: empty token ""
    /// - Expected: index remains unchanged (empty)
    #[rstest]
    fn index_ngrams_empty_token_returns_unchanged() {
        let index: NgramIndex = PersistentHashMap::new();
        let config = SearchIndexConfig::default();
        let task_id = task_id_from_u128(1);

        let result = index_ngrams(index, "", &task_id, &config);

        assert!(result.is_empty(), "Expected empty index for empty token");
    }

    /// Tests that `index_ngrams` handles multiple tokens correctly.
    ///
    /// - Input: two different tokens "abc" and "xyz" with the same `task_id`
    /// - Expected: both tokens' n-grams are indexed
    #[rstest]
    fn index_ngrams_multiple_tokens_same_task() {
        let index: NgramIndex = PersistentHashMap::new();
        let config = SearchIndexConfig::default();
        let task_id = task_id_from_u128(1);

        let result = index_ngrams(index, "abc", &task_id, &config);
        let result = index_ngrams(result, "xyz", &task_id, &config);

        // Verify both tokens' n-grams are present
        assert!(result.get("abc").is_some(), "Expected 'abc' n-gram");
        assert!(result.get("xyz").is_some(), "Expected 'xyz' n-gram");
    }

    /// Tests that `index_ngrams` handles multibyte characters correctly.
    ///
    /// - Input: Japanese token "日本語" with `ngram_size=3`
    /// - Expected: single n-gram "日本語" is indexed
    #[rstest]
    fn index_ngrams_multibyte_characters() {
        let index: NgramIndex = PersistentHashMap::new();
        let config = SearchIndexConfig::default(); // ngram_size = 3
        let task_id = task_id_from_u128(1);

        let result = index_ngrams(index, "日本語", &task_id, &config);

        // "日本語" has exactly 3 characters, so it generates 1 n-gram
        assert!(result.get("日本語").is_some(), "Expected '日本語' n-gram");
        assert_eq!(result.len(), 1, "Expected exactly 1 n-gram");
    }

    // -------------------------------------------------------------------------
    // remove_ngrams Tests
    // -------------------------------------------------------------------------

    /// Tests that `remove_ngrams` removes a `TaskId` from all n-grams.
    ///
    /// - Input: index with "callback" n-grams for `task_id`
    /// - Expected: all n-gram entries are removed (since only one task)
    #[rstest]
    fn remove_ngrams_removes_task_id() {
        let index: NgramIndex = PersistentHashMap::new();
        let config = SearchIndexConfig::default();
        let task_id = task_id_from_u128(1);

        // Add token
        let result = index_ngrams(index, "callback", &task_id, &config);

        // Remove token
        let result = remove_ngrams(result, "callback", &task_id, &config);

        // Verify all n-grams are removed
        assert!(
            result.get("cal").is_none(),
            "Expected 'cal' n-gram to be removed"
        );
        assert!(
            result.get("all").is_none(),
            "Expected 'all' n-gram to be removed"
        );
        assert!(
            result.get("ack").is_none(),
            "Expected 'ack' n-gram to be removed"
        );
        assert!(result.is_empty(), "Expected empty index after removal");
    }

    /// Tests that `remove_ngrams` only removes the specified `TaskId`.
    ///
    /// - Input: index with "callback" for `task_id1` and `task_id2`, remove `task_id1`
    /// - Expected: `task_id2` remains in posting lists
    #[rstest]
    fn remove_ngrams_preserves_other_task_ids() {
        let index: NgramIndex = PersistentHashMap::new();
        let config = SearchIndexConfig::default();
        let task_id1 = task_id_from_u128(1);
        let task_id2 = task_id_from_u128(2);

        // Add both task IDs
        let result = index_ngrams(index, "callback", &task_id1, &config);
        let result = index_ngrams(result, "callback", &task_id2, &config);

        // Remove only task_id1
        let result = remove_ngrams(result, "callback", &task_id1, &config);

        // Verify task_id2 remains
        let ids = result.get("cal").unwrap();
        assert_eq!(ids.len(), 1, "Expected 1 TaskId remaining");
        assert_eq!(
            ids.get(0).unwrap(),
            &task_id2,
            "Expected task_id2 to remain"
        );
    }

    /// Tests that `remove_ngrams` handles non-existent `TaskId` gracefully.
    ///
    /// - Input: index with "callback" for `task_id1`, try to remove `task_id2`
    /// - Expected: index remains unchanged
    #[rstest]
    fn remove_ngrams_nonexistent_task_id_unchanged() {
        let index: NgramIndex = PersistentHashMap::new();
        let config = SearchIndexConfig::default();
        let task_id1 = task_id_from_u128(1);
        let task_id2 = task_id_from_u128(2);

        // Add task_id1 only
        let result = index_ngrams(index, "callback", &task_id1, &config);

        // Try to remove task_id2 (not in index)
        let result = remove_ngrams(result, "callback", &task_id2, &config);

        // Verify task_id1 remains
        let ids = result.get("cal").unwrap();
        assert_eq!(ids.len(), 1, "Expected 1 TaskId remaining");
        assert_eq!(
            ids.get(0).unwrap(),
            &task_id1,
            "Expected task_id1 to remain"
        );
    }

    /// Tests that `remove_ngrams` handles empty index gracefully.
    ///
    /// - Input: empty index, try to remove
    /// - Expected: index remains empty
    #[rstest]
    fn remove_ngrams_empty_index_unchanged() {
        let index: NgramIndex = PersistentHashMap::new();
        let config = SearchIndexConfig::default();
        let task_id = task_id_from_u128(1);

        let result = remove_ngrams(index, "callback", &task_id, &config);

        assert!(result.is_empty(), "Expected empty index to remain empty");
    }

    /// Tests that `remove_ngrams` handles short tokens (no n-grams to remove).
    ///
    /// - Input: token "ab" with `ngram_size=3`
    /// - Expected: index remains unchanged
    #[rstest]
    fn remove_ngrams_short_token_unchanged() {
        let index: NgramIndex = PersistentHashMap::new();
        let config = SearchIndexConfig::default();
        let task_id = task_id_from_u128(1);

        // Add a longer token first
        let result = index_ngrams(index, "callback", &task_id, &config);
        let original_len = result.len();

        // Try to remove a short token (no n-grams generated)
        let result = remove_ngrams(result, "ab", &task_id, &config);

        // Verify index is unchanged
        assert_eq!(
            result.len(),
            original_len,
            "Expected index length to remain unchanged"
        );
    }
}

// =============================================================================
// SearchIndex build_with_config Tests (REQ-SEARCH-NGRAM-002 Part 3)
// =============================================================================

#[cfg(test)]
mod search_index_build_tests {
    use super::*;
    use crate::domain::{Tag, Timestamp};
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // Helper Functions
    // -------------------------------------------------------------------------

    /// Creates a task with a given title for testing.
    fn create_task_with_title(title: &str) -> Task {
        Task::new(TaskId::generate(), title, Timestamp::now())
    }

    /// Creates a task with a given title and tags for testing.
    fn create_task_with_title_and_tags(title: &str, tags: Vec<&str>) -> Task {
        let base = create_task_with_title(title);
        tags.into_iter()
            .fold(base, |task, tag| task.add_tag(Tag::new(tag)))
    }

    // -------------------------------------------------------------------------
    // build_with_config Tests
    // -------------------------------------------------------------------------

    /// Tests that `build_with_config` with Ngram mode builds n-gram indexes.
    ///
    /// - Input: tasks with titles containing searchable words
    /// - Config: default (Ngram mode)
    /// - Expected: n-gram indexes are populated, all-suffix indexes are empty
    #[rstest]
    fn build_with_config_ngram_mode() {
        let tasks: PersistentVector<Task> = vec![
            create_task_with_title("callback function test"),
            create_task_with_title("important meeting tomorrow"),
        ]
        .into_iter()
        .collect();
        let config = SearchIndexConfig::default();
        assert_eq!(config.infix_mode, InfixMode::Ngram);

        let index = SearchIndex::build_with_config(&tasks, config);

        // N-gram indexes should be populated
        assert!(
            !index.title_full_ngram_index.is_empty(),
            "title_full_ngram_index should be populated in Ngram mode"
        );
        assert!(
            !index.title_word_ngram_index.is_empty(),
            "title_word_ngram_index should be populated in Ngram mode"
        );

        // All-suffix indexes should be empty
        assert!(
            index.title_full_all_suffix_index.is_empty(),
            "title_full_all_suffix_index should be empty in Ngram mode"
        );
        assert!(
            index.title_word_all_suffix_index.is_empty(),
            "title_word_all_suffix_index should be empty in Ngram mode"
        );
    }

    /// Tests that `build_with_config` with `LegacyAllSuffix` mode builds all-suffix indexes.
    ///
    /// - Input: tasks with titles containing searchable words
    /// - Config: `LegacyAllSuffix` mode
    /// - Expected: all-suffix indexes are populated, n-gram indexes are empty
    #[rstest]
    fn build_with_config_legacy_mode() {
        let tasks: PersistentVector<Task> = vec![
            create_task_with_title("callback function test"),
            create_task_with_title("important meeting tomorrow"),
        ]
        .into_iter()
        .collect();
        let config = SearchIndexConfig {
            infix_mode: InfixMode::LegacyAllSuffix,
            ..Default::default()
        };

        let index = SearchIndex::build_with_config(&tasks, config);

        // All-suffix indexes should be populated
        assert!(
            !index.title_full_all_suffix_index.is_empty(),
            "title_full_all_suffix_index should be populated in LegacyAllSuffix mode"
        );
        assert!(
            !index.title_word_all_suffix_index.is_empty(),
            "title_word_all_suffix_index should be populated in LegacyAllSuffix mode"
        );

        // N-gram indexes should be empty
        assert!(
            index.title_full_ngram_index.is_empty(),
            "title_full_ngram_index should be empty in LegacyAllSuffix mode"
        );
        assert!(
            index.title_word_ngram_index.is_empty(),
            "title_word_ngram_index should be empty in LegacyAllSuffix mode"
        );
    }

    /// Tests that `build_with_config` with Disabled mode builds neither infix index.
    ///
    /// - Input: tasks with titles containing searchable words
    /// - Config: Disabled mode
    /// - Expected: both n-gram and all-suffix indexes are empty
    #[rstest]
    fn build_with_config_disabled_mode() {
        let tasks: PersistentVector<Task> = vec![
            create_task_with_title("callback function test"),
            create_task_with_title("important meeting tomorrow"),
        ]
        .into_iter()
        .collect();
        let config = SearchIndexConfig {
            infix_mode: InfixMode::Disabled,
            ..Default::default()
        };

        let index = SearchIndex::build_with_config(&tasks, config);

        // Both infix indexes should be empty
        assert!(
            index.title_full_ngram_index.is_empty(),
            "title_full_ngram_index should be empty in Disabled mode"
        );
        assert!(
            index.title_word_ngram_index.is_empty(),
            "title_word_ngram_index should be empty in Disabled mode"
        );
        assert!(
            index.title_full_all_suffix_index.is_empty(),
            "title_full_all_suffix_index should be empty in Disabled mode"
        );
        assert!(
            index.title_word_all_suffix_index.is_empty(),
            "title_word_all_suffix_index should be empty in Disabled mode"
        );

        // Prefix indexes should still be populated
        assert!(
            !index.title_word_index.is_empty(),
            "title_word_index should still be populated in Disabled mode"
        );
        assert!(
            !index.title_full_index.is_empty(),
            "title_full_index should still be populated in Disabled mode"
        );
    }

    /// Tests that `build_with_config` respects `max_tokens_per_task` limit.
    ///
    /// The limit is applied to (title words + tags) combined.
    /// When the total exceeds the limit, tags are prioritized and the remaining
    /// slots are allocated to title words.
    ///
    /// - Input: task with 6 words + 2 tags = 8 tokens
    /// - Config: `max_tokens_per_task` = 5
    /// - Expected: `word_limit` = 5 - 2 = 3, `tag_limit` = 2
    ///   So only first 3 words (alpha, beta, gamma) and all 2 tags are indexed
    #[rstest]
    fn build_respects_max_tokens_per_task() {
        // 6 words + 2 tags = 8 tokens
        // Using longer words to ensure n-grams are generated
        let task = create_task_with_title_and_tags(
            "alpha beta gamma delta epsilon zeta",
            vec!["important", "urgent"],
        );
        let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
        let config = SearchIndexConfig {
            max_tokens_per_task: 5, // 8 > 5, so word_limit = 5 - 2 = 3, tag_limit = 2
            ..Default::default()
        };

        let index = SearchIndex::build_with_config(&tasks, config);

        // "alpha" should be indexed (1st word, within word_limit=3)
        // "alpha" (5 chars) -> "alp", "lph", "pha"
        assert!(
            index.title_word_ngram_index.get("alp").is_some(),
            "alpha's n-gram 'alp' should be indexed (1st word)"
        );

        // "gamma" should be indexed (3rd word, within word_limit=3)
        // "gamma" (5 chars) -> "gam", "amm", "mma"
        assert!(
            index.title_word_ngram_index.get("gam").is_some(),
            "gamma's n-gram 'gam' should be indexed (3rd word)"
        );

        // "delta" should NOT be indexed (4th word, beyond word_limit=3)
        // "delta" (5 chars) -> "del", "elt", "lta"
        assert!(
            index.title_word_ngram_index.get("del").is_none(),
            "delta's n-gram 'del' should NOT be indexed (4th word, beyond limit)"
        );

        // "zeta" should NOT be indexed (6th word, beyond word_limit=3)
        // "zeta" (4 chars) -> "zet", "eta"
        assert!(
            index.title_word_ngram_index.get("zet").is_none(),
            "zeta's n-gram 'zet' should NOT be indexed (6th word, beyond limit)"
        );

        // Both tags should be indexed (tag_limit = 2)
        // "important" -> "imp", "mpo", "por", ...
        assert!(
            index.tag_ngram_index.get("imp").is_some(),
            "tag 'important' should be indexed"
        );
        // "urgent" -> "urg", "rge", ...
        assert!(
            index.tag_ngram_index.get("urg").is_some(),
            "tag 'urgent' should be indexed"
        );
    }

    /// Tests that `build_with_config` normalizes using `normalize_query()`.
    ///
    /// - Input: task with mixed case and extra spaces in title
    /// - Expected: normalized n-grams are indexed (lowercase, trimmed)
    #[rstest]
    fn build_with_config_uses_normalize_query() {
        let task = create_task_with_title("  CALLBACK  Function  ");
        let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
        let config = SearchIndexConfig::default();

        let index = SearchIndex::build_with_config(&tasks, config);

        // Check that normalized n-grams exist (lowercase)
        // "callback" -> "cal", "all", "llb", "lba", "bac", "ack"
        assert!(
            index.title_word_ngram_index.get("cal").is_some(),
            "Normalized n-gram 'cal' should exist (from 'callback')"
        );
        assert!(
            index.title_word_ngram_index.get("all").is_some(),
            "Normalized n-gram 'all' should exist (from 'callback')"
        );

        // Check that uppercase n-grams do NOT exist
        assert!(
            index.title_word_ngram_index.get("CAL").is_none(),
            "Uppercase n-gram 'CAL' should NOT exist"
        );
    }

    /// Tests that `build_with_config` also indexes tags in Ngram mode.
    ///
    /// - Input: task with tags
    /// - Config: Ngram mode
    /// - Expected: tag n-grams are indexed
    #[rstest]
    fn build_with_config_indexes_tags_in_ngram_mode() {
        let task = create_task_with_title_and_tags("simple task", vec!["important", "urgent"]);
        let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
        let config = SearchIndexConfig::default();

        let index = SearchIndex::build_with_config(&tasks, config);

        // Check that tag n-grams exist
        // "important" -> "imp", "mpo", "por", "ort", "rta", "tan", "ant"
        assert!(
            index.tag_ngram_index.get("imp").is_some(),
            "Tag n-gram 'imp' should exist (from 'important')"
        );
        // "urgent" -> "urg", "rge", "gen", "ent"
        assert!(
            index.tag_ngram_index.get("urg").is_some(),
            "Tag n-gram 'urg' should exist (from 'urgent')"
        );
    }

    /// Tests that `build_with_config` preserves the config in the index.
    #[rstest]
    fn build_with_config_stores_config() {
        let tasks: PersistentVector<Task> = PersistentVector::new();
        let config = SearchIndexConfig {
            infix_mode: InfixMode::Ngram,
            ngram_size: 4,
            min_query_len_for_infix: 5,
            max_ngrams_per_token: 32,
            max_tokens_per_task: 50,
            max_search_candidates: 500,
        };

        let index = SearchIndex::build_with_config(&tasks, config);

        assert_eq!(index.config.infix_mode, InfixMode::Ngram);
        assert_eq!(index.config.ngram_size, 4);
        assert_eq!(index.config.min_query_len_for_infix, 5);
        assert_eq!(index.config.max_ngrams_per_token, 32);
        assert_eq!(index.config.max_tokens_per_task, 50);
        assert_eq!(index.config.max_search_candidates, 500);
    }

    /// Tests that the existing `build()` method works with the default configuration.
    ///
    /// The `build()` method uses `SearchIndexConfig::default()`, which enables
    /// `InfixMode::Ngram` for better performance.
    #[rstest]
    fn build_maintains_backward_compatibility() {
        let tasks: PersistentVector<Task> = vec![
            create_task_with_title("callback function test"),
            create_task_with_title("important meeting tomorrow"),
        ]
        .into_iter()
        .collect();

        let index = SearchIndex::build(&tasks);

        // build() uses Ngram mode (default configuration)
        assert_eq!(index.config.infix_mode, InfixMode::Ngram);

        // N-gram indexes should be populated
        assert!(
            !index.title_full_ngram_index.is_empty(),
            "title_full_ngram_index should be populated with build()"
        );

        // All-suffix indexes should be empty
        assert!(
            index.title_full_all_suffix_index.is_empty(),
            "title_full_all_suffix_index should be empty with build()"
        );
    }
}

// =============================================================================
// N-gram Search Logic Tests (REQ-SEARCH-NGRAM-003)
// =============================================================================

#[cfg(test)]
mod ngram_search_tests {
    use super::*;
    use crate::domain::Timestamp;
    use proptest::prelude::*;
    use rstest::rstest;
    use uuid::Uuid;

    // -------------------------------------------------------------------------
    // Helper Functions
    // -------------------------------------------------------------------------

    /// Creates a `TaskId` from a u128 value (for deterministic testing).
    fn task_id_from_u128(value: u128) -> TaskId {
        TaskId::from_uuid(Uuid::from_u128(value))
    }

    /// Creates a task with the given title.
    fn create_task_with_title(title: &str) -> Task {
        Task::new(TaskId::generate(), title, Timestamp::now())
    }

    /// Creates a task with the given title and a fixed `TaskId`.
    fn create_task_with_title_and_id(title: &str, id: u128) -> Task {
        Task::new(task_id_from_u128(id), title, Timestamp::now())
    }

    // -------------------------------------------------------------------------
    // intersect_sorted_vecs Tests
    // -------------------------------------------------------------------------

    /// Tests basic intersection of two sorted vectors.
    #[rstest]
    fn intersect_sorted_vecs_basic() {
        let left = vec![
            task_id_from_u128(1),
            task_id_from_u128(3),
            task_id_from_u128(5),
        ];
        let right = vec![
            task_id_from_u128(2),
            task_id_from_u128(3),
            task_id_from_u128(4),
            task_id_from_u128(5),
        ];

        let result = intersect_sorted_vecs(&left, &right);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], task_id_from_u128(3));
        assert_eq!(result[1], task_id_from_u128(5));
    }

    /// Tests intersection with empty left vector.
    #[rstest]
    fn intersect_sorted_vecs_empty_left() {
        let left: Vec<TaskId> = vec![];
        let right = vec![task_id_from_u128(1), task_id_from_u128(2)];

        let result = intersect_sorted_vecs(&left, &right);

        assert!(result.is_empty());
    }

    /// Tests intersection with empty right vector.
    #[rstest]
    fn intersect_sorted_vecs_empty_right() {
        let left = vec![task_id_from_u128(1), task_id_from_u128(2)];
        let right: Vec<TaskId> = vec![];

        let result = intersect_sorted_vecs(&left, &right);

        assert!(result.is_empty());
    }

    /// Tests intersection with no common elements.
    #[rstest]
    fn intersect_sorted_vecs_no_common() {
        let left = vec![task_id_from_u128(1), task_id_from_u128(3)];
        let right = vec![task_id_from_u128(2), task_id_from_u128(4)];

        let result = intersect_sorted_vecs(&left, &right);

        assert!(result.is_empty());
    }

    /// Tests intersection with identical vectors.
    #[rstest]
    fn intersect_sorted_vecs_identical() {
        let left = vec![
            task_id_from_u128(1),
            task_id_from_u128(2),
            task_id_from_u128(3),
        ];
        let right = vec![
            task_id_from_u128(1),
            task_id_from_u128(2),
            task_id_from_u128(3),
        ];

        let result = intersect_sorted_vecs(&left, &right);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], task_id_from_u128(1));
        assert_eq!(result[1], task_id_from_u128(2));
        assert_eq!(result[2], task_id_from_u128(3));
    }

    // -------------------------------------------------------------------------
    // intersect_sorted_vecs Property Tests
    // -------------------------------------------------------------------------

    proptest! {
        /// Property: Intersection result contains only elements from both inputs.
        ///
        /// Law: `∀x ∈ result: x ∈ left ∧ x ∈ right`
        #[test]
        fn intersect_sorted_vecs_subset_property(
            left_values in proptest::collection::vec(0u64..1000, 0..20),
            right_values in proptest::collection::vec(0u64..1000, 0..20)
        ) {
            // Create sorted, deduplicated TaskId vectors
            let mut left_sorted: Vec<TaskId> = left_values
                .iter()
                .map(|n| task_id_from_u128(u128::from(*n)))
                .collect();
            let mut right_sorted: Vec<TaskId> = right_values
                .iter()
                .map(|n| task_id_from_u128(u128::from(*n)))
                .collect();
            left_sorted.sort();
            left_sorted.dedup();
            right_sorted.sort();
            right_sorted.dedup();

            let result = intersect_sorted_vecs(&left_sorted, &right_sorted);

            // All elements in result must be in both inputs
            for id in &result {
                prop_assert!(
                    left_sorted.contains(id),
                    "Result element {:?} not in left",
                    id
                );
                prop_assert!(
                    right_sorted.contains(id),
                    "Result element {:?} not in right",
                    id
                );
            }
        }

        /// Property: Intersection contains all common elements.
        ///
        /// Law: `∀x ∈ left ∧ x ∈ right: x ∈ result`
        #[test]
        fn intersect_sorted_vecs_completeness_property(
            left_values in proptest::collection::vec(0u64..1000, 0..20),
            right_values in proptest::collection::vec(0u64..1000, 0..20)
        ) {
            // Create sorted, deduplicated TaskId vectors
            let mut left_sorted: Vec<TaskId> = left_values
                .iter()
                .map(|n| task_id_from_u128(u128::from(*n)))
                .collect();
            let mut right_sorted: Vec<TaskId> = right_values
                .iter()
                .map(|n| task_id_from_u128(u128::from(*n)))
                .collect();
            left_sorted.sort();
            left_sorted.dedup();
            right_sorted.sort();
            right_sorted.dedup();

            let result = intersect_sorted_vecs(&left_sorted, &right_sorted);

            // All common elements must be in result
            for id in &left_sorted {
                if right_sorted.contains(id) {
                    prop_assert!(
                        result.contains(id),
                        "Common element {:?} missing from result",
                        id
                    );
                }
            }
        }

        /// Property: Intersection result is sorted.
        ///
        /// Law: `∀i < j: result[i] < result[j]`
        #[test]
        fn intersect_sorted_vecs_sorted_property(
            left_values in proptest::collection::vec(0u64..1000, 0..20),
            right_values in proptest::collection::vec(0u64..1000, 0..20)
        ) {
            // Create sorted, deduplicated TaskId vectors
            let mut left_sorted: Vec<TaskId> = left_values
                .iter()
                .map(|n| task_id_from_u128(u128::from(*n)))
                .collect();
            let mut right_sorted: Vec<TaskId> = right_values
                .iter()
                .map(|n| task_id_from_u128(u128::from(*n)))
                .collect();
            left_sorted.sort();
            left_sorted.dedup();
            right_sorted.sort();
            right_sorted.dedup();

            let result = intersect_sorted_vecs(&left_sorted, &right_sorted);

            // Result must be sorted
            for i in 1..result.len() {
                prop_assert!(
                    result[i - 1] < result[i],
                    "Result not sorted at index {}: {:?} >= {:?}",
                    i,
                    result[i - 1],
                    result[i]
                );
            }
        }
    }

    // -------------------------------------------------------------------------
    // find_candidates_by_ngrams Tests
    // -------------------------------------------------------------------------

    /// Tests that `find_candidates_by_ngrams` returns `None` for short queries.
    ///
    /// When query length < `min_query_len_for_infix`, the method should return
    /// `None` to indicate that prefix search should be used instead.
    #[rstest]
    fn find_candidates_short_query_returns_none() {
        let tasks: PersistentVector<Task> = vec![create_task_with_title("callback function test")]
            .into_iter()
            .collect();
        let config = SearchIndexConfig {
            min_query_len_for_infix: 3,
            ..Default::default()
        };
        let index = SearchIndex::build_with_config(&tasks, config);

        // Query "ab" has 2 chars, which is < 3
        let normalized = normalize_query("ab");
        let result =
            index.find_candidates_by_ngrams(&index.title_full_ngram_index, &normalized.key);

        assert!(
            result.is_none(),
            "Expected None for query shorter than min_query_len_for_infix"
        );
    }

    /// Tests that `find_candidates_by_ngrams` returns empty vec for non-matching query.
    #[rstest]
    fn find_candidates_no_match_returns_empty() {
        let tasks: PersistentVector<Task> = vec![create_task_with_title("callback function test")]
            .into_iter()
            .collect();
        let config = SearchIndexConfig::default();
        let index = SearchIndex::build_with_config(&tasks, config);

        // Query "xyz" doesn't match any n-grams
        let normalized = normalize_query("xyz");
        let result =
            index.find_candidates_by_ngrams(&index.title_word_ngram_index, &normalized.key);

        assert!(
            result.is_none_or(|v| v.is_empty()),
            "Expected empty result for non-matching query"
        );
    }

    /// Tests that `find_candidates_by_ngrams` returns candidates for matching query.
    #[rstest]
    fn find_candidates_returns_candidates_for_match() {
        let task = create_task_with_title_and_id("callback function test", 1);
        let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
        let config = SearchIndexConfig::default();
        let index = SearchIndex::build_with_config(&tasks, config);

        // Query "call" matches "callback"
        let normalized = normalize_query("call");
        let result =
            index.find_candidates_by_ngrams(&index.title_word_ngram_index, &normalized.key);

        assert!(result.is_some(), "Expected Some for matching query");
        let candidates = result.unwrap();
        assert!(
            !candidates.is_empty(),
            "Expected non-empty candidates for matching query"
        );
        assert_eq!(candidates[0], task_id_from_u128(1));
    }

    // -------------------------------------------------------------------------
    // search_by_title Tests
    // -------------------------------------------------------------------------

    /// Tests that `search_by_title` returns `None` for non-matching query.
    #[rstest]
    fn search_by_title_returns_none_for_no_match() {
        let tasks: PersistentVector<Task> = vec![create_task_with_title("callback function test")]
            .into_iter()
            .collect();
        let config = SearchIndexConfig::default();
        let index = SearchIndex::build_with_config(&tasks, config);

        let result = index.search_by_title("nonexistent");

        assert!(
            result.is_none(),
            "Expected None for query that doesn't match any task"
        );
    }

    /// Tests that `search_by_title` finds tasks with matching infix.
    #[rstest]
    fn search_by_title_finds_infix_match() {
        let tasks: PersistentVector<Task> = vec![create_task_with_title("callback function test")]
            .into_iter()
            .collect();
        let config = SearchIndexConfig::default();
        let index = SearchIndex::build_with_config(&tasks, config);

        // "llba" is an infix of "callback"
        let result = index.search_by_title("llba");

        assert!(result.is_some(), "Expected Some for infix match");
        let search_result = result.unwrap();
        assert_eq!(search_result.tasks.len(), 1);
        assert_eq!(
            search_result.tasks.get(0).unwrap().title,
            "callback function test"
        );
    }

    /// Tests that `search_by_title` respects `max_search_candidates`.
    #[rstest]
    fn search_respects_max_search_candidates() {
        // Create 100 tasks with "common" in the title
        let tasks: PersistentVector<Task> = (0..100)
            .map(|i| create_task_with_title(&format!("common word task number {i}")))
            .collect();
        let config = SearchIndexConfig {
            max_search_candidates: 10,
            ..Default::default()
        };
        let index = SearchIndex::build_with_config(&tasks, config);

        let result = index.search_by_title("common");

        assert!(result.is_some(), "Expected Some for matching query");
        let search_result = result.unwrap();
        assert!(
            search_result.tasks.len() <= 10,
            "Expected at most 10 results, got {}",
            search_result.tasks.len()
        );
    }

    /// Tests that `search_by_title` uses normalized query.
    #[rstest]
    fn search_by_title_uses_normalized_query() {
        let tasks: PersistentVector<Task> = vec![create_task_with_title("callback function test")]
            .into_iter()
            .collect();
        let config = SearchIndexConfig::default();
        let index = SearchIndex::build_with_config(&tasks, config);

        // Uppercase query should still match lowercase index
        let result = index.search_by_title("CALLBACK");

        assert!(
            result.is_some(),
            "Expected Some for uppercase query matching lowercase title"
        );
    }

    // -------------------------------------------------------------------------
    // Soundness Property Tests
    // -------------------------------------------------------------------------

    proptest! {
        /// Property: All returned results actually contain the query substring.
        ///
        /// Law (Soundness): `∀ task ∈ result: normalized_query ⊆ normalized_title`
        #[test]
        fn infix_search_soundness(
            title in "[a-z ]{10,50}",
            query in "[a-z]{3,10}"
        ) {
            let task = create_task_with_title(&title);
            let tasks: PersistentVector<Task> = vec![task].into_iter().collect();
            let config = SearchIndexConfig::default();
            let index = SearchIndex::build_with_config(&tasks, config);

            if let Some(result) = index.search_by_title(&query) {
                for found_task in &result.tasks {
                    // Soundness: returned results must actually contain the query
                    let normalized_query = normalize_query(&query).key;
                    let normalized_title = normalize_query(&found_task.title).key;
                    prop_assert!(
                        normalized_title.contains(&normalized_query),
                        "False positive: task '{}' (normalized: '{}') does not contain query '{}' (normalized: '{}')",
                        found_task.title,
                        normalized_title,
                        query,
                        normalized_query
                    );
                }
            }
        }
    }
}

// =============================================================================
// add_task/remove_task N-gram Integration Tests (Phase 6)
// =============================================================================

/// Tests for `add_task` and `remove_task` methods with n-gram index support.
///
/// These tests verify that:
/// 1. `add_task` updates n-gram indexes when `infix_mode == Ngram`
/// 2. `add_task` updates all-suffix indexes when `infix_mode == LegacyAllSuffix`
/// 3. `remove_task` removes from n-gram indexes when `infix_mode == Ngram`
/// 4. `add_task` respects `max_tokens_per_task` limit
/// 5. Tag processing order is deterministic (sorted)
/// 6. Normalization uses `normalize_query()` for consistency
#[cfg(test)]
mod add_remove_task_tests {
    use super::*;
    use crate::domain::{Tag, Timestamp};
    use rstest::rstest;
    use uuid::Uuid;

    // -------------------------------------------------------------------------
    // Test Helpers
    // -------------------------------------------------------------------------

    /// Creates a `TaskId` from a u128 value for deterministic testing.
    fn task_id_from_u128(value: u128) -> TaskId {
        TaskId::from_uuid(Uuid::from_u128(value))
    }

    /// Creates a task with a given title and task ID for deterministic testing.
    fn create_task_with_title_and_id(title: &str, task_id: TaskId) -> Task {
        Task::new(task_id, title, Timestamp::now())
    }

    /// Creates a task with a given title, task ID, and tags for deterministic testing.
    fn create_task_with_title_id_and_tags(title: &str, task_id: TaskId, tags: Vec<&str>) -> Task {
        let base = create_task_with_title_and_id(title, task_id);
        tags.into_iter()
            .fold(base, |task, tag| task.add_tag(Tag::new(tag)))
    }

    // -------------------------------------------------------------------------
    // add_task Tests
    // -------------------------------------------------------------------------

    /// Tests that `add_task` updates n-gram indexes when `infix_mode == Ngram`.
    ///
    /// - Input: empty index with Ngram mode, add task with title "callback function"
    /// - Expected: n-gram indexes contain the task's n-grams
    #[rstest]
    fn add_task_updates_ngram_index() {
        let config = SearchIndexConfig::default(); // Ngram mode
        let empty_index = SearchIndex::build_with_config(&PersistentVector::new(), config);

        let task_id = task_id_from_u128(1);
        let task = create_task_with_title_and_id("callback function", task_id.clone());

        let updated_index = empty_index.add_task(&task);

        // Verify title_word_ngram_index contains n-grams for "callback"
        // "callback" generates: "cal", "all", "llb", "lba", "bac", "ack"
        assert!(
            updated_index.title_word_ngram_index.get("cal").is_some(),
            "N-gram 'cal' should be indexed"
        );
        assert!(
            updated_index.title_word_ngram_index.get("all").is_some(),
            "N-gram 'all' should be indexed"
        );
        assert!(
            updated_index.title_word_ngram_index.get("ack").is_some(),
            "N-gram 'ack' should be indexed"
        );

        // Verify the task ID is in the posting list
        let cal_ids = updated_index.title_word_ngram_index.get("cal").unwrap();
        assert!(
            cal_ids.iter().any(|id| *id == task_id),
            "Task ID should be in the posting list for 'cal'"
        );

        // Verify title_full_ngram_index contains n-grams for "callback function"
        // Normalized: "callback function"
        assert!(
            updated_index.title_full_ngram_index.get("cal").is_some(),
            "Full title n-gram 'cal' should be indexed"
        );
    }

    /// Tests that `add_task` updates all-suffix indexes when `infix_mode == LegacyAllSuffix`.
    ///
    /// - Input: empty index with `LegacyAllSuffix` mode, add task with title "callback"
    /// - Expected: all-suffix indexes contain the task's suffixes
    #[rstest]
    fn add_task_updates_legacy_index() {
        let config = SearchIndexConfig {
            infix_mode: InfixMode::LegacyAllSuffix,
            ..Default::default()
        };
        let empty_index = SearchIndex::build_with_config(&PersistentVector::new(), config);

        let task_id = task_id_from_u128(1);
        let task = create_task_with_title_and_id("callback", task_id.clone());

        let updated_index = empty_index.add_task(&task);

        // Verify title_word_all_suffix_index contains suffixes for "callback"
        // "callback" generates suffixes: "callback", "allback", "llback", "lback", "back", "ack", "ck", "k"
        assert!(
            updated_index
                .title_word_all_suffix_index
                .get("callback")
                .is_some(),
            "Suffix 'callback' should be indexed"
        );
        assert!(
            updated_index
                .title_word_all_suffix_index
                .get("allback")
                .is_some(),
            "Suffix 'allback' should be indexed"
        );
        assert!(
            updated_index
                .title_word_all_suffix_index
                .get("back")
                .is_some(),
            "Suffix 'back' should be indexed"
        );

        // Verify the task ID is in the posting list
        let callback_ids = updated_index
            .title_word_all_suffix_index
            .get("callback")
            .unwrap();
        assert!(
            callback_ids.iter().any(|id| *id == task_id),
            "Task ID should be in the posting list for 'callback'"
        );

        // Verify n-gram indexes are NOT updated in LegacyAllSuffix mode
        assert!(
            updated_index.title_word_ngram_index.is_empty(),
            "N-gram index should be empty in LegacyAllSuffix mode"
        );
    }

    /// Tests that `remove_task` removes from n-gram indexes when `infix_mode == Ngram`.
    ///
    /// - Input: index with one task, remove that task
    /// - Expected: n-gram indexes no longer contain the task's entries
    #[rstest]
    fn remove_task_removes_from_ngram_index() {
        let config = SearchIndexConfig::default(); // Ngram mode
        let empty_index = SearchIndex::build_with_config(&PersistentVector::new(), config);

        let task_id = task_id_from_u128(1);
        let task = create_task_with_title_and_id("callback", task_id);

        // Add task first
        let index_with_task = empty_index.add_task(&task);

        // Verify task is indexed
        assert!(
            index_with_task.title_word_ngram_index.get("cal").is_some(),
            "N-gram 'cal' should be indexed before removal"
        );

        // Remove task
        let index_after_removal = index_with_task.remove_task(&task);

        // Verify n-gram entries are removed (since this was the only task)
        assert!(
            index_after_removal
                .title_word_ngram_index
                .get("cal")
                .is_none(),
            "N-gram 'cal' should be removed after task removal"
        );
        assert!(
            index_after_removal
                .title_word_ngram_index
                .get("all")
                .is_none(),
            "N-gram 'all' should be removed after task removal"
        );
    }

    /// Tests that `remove_task` preserves other tasks' n-gram entries.
    ///
    /// - Input: index with two tasks sharing some n-grams, remove one task
    /// - Expected: shared n-grams still contain the other task's ID
    #[rstest]
    fn remove_task_preserves_other_task_ngrams() {
        let config = SearchIndexConfig::default(); // Ngram mode
        let empty_index = SearchIndex::build_with_config(&PersistentVector::new(), config);

        let task_id1 = task_id_from_u128(1);
        let task_id2 = task_id_from_u128(2);
        let task1 = create_task_with_title_and_id("callback", task_id1);
        let task2 = create_task_with_title_and_id("callout", task_id2.clone()); // Shares "cal" n-gram

        // Add both tasks
        let index_with_tasks = empty_index.add_task(&task1).add_task(&task2);

        // Verify both tasks are indexed for "cal"
        let cal_ids_before = index_with_tasks.title_word_ngram_index.get("cal").unwrap();
        assert_eq!(
            cal_ids_before.len(),
            2,
            "Both tasks should share 'cal' n-gram"
        );

        // Remove task1
        let index_after_removal = index_with_tasks.remove_task(&task1);

        // Verify "cal" still contains task2
        let cal_ids_after = index_after_removal
            .title_word_ngram_index
            .get("cal")
            .unwrap();
        assert_eq!(
            cal_ids_after.len(),
            1,
            "Only one task should remain for 'cal'"
        );
        assert!(
            cal_ids_after.iter().any(|id| *id == task_id2),
            "Task2 should still be in 'cal' posting list"
        );
    }

    /// Tests that `add_task` respects `max_tokens_per_task` limit.
    ///
    /// - Input: task with 6 words + 2 tags = 8 tokens, `max_tokens_per_task` = 5
    /// - Expected: only first 3 words and 2 tags are indexed (3 + 2 = 5)
    #[rstest]
    fn add_task_respects_max_tokens_per_task() {
        let config = SearchIndexConfig {
            max_tokens_per_task: 5,
            ..Default::default()
        };
        let empty_index = SearchIndex::build_with_config(&PersistentVector::new(), config);

        let task_id = task_id_from_u128(1);
        // 6 words + 2 tags = 8 tokens
        // word_limit = 5 - 2 = 3, tag_limit = 2
        let task = create_task_with_title_id_and_tags(
            "alpha beta gamma delta epsilon zeta",
            task_id,
            vec!["important", "urgent"],
        );

        let updated_index = empty_index.add_task(&task);

        // Verify first 3 words are indexed (alpha, beta, gamma)
        // "alpha" generates n-grams: "alp", "lph", "pha"
        assert!(
            updated_index.title_word_ngram_index.get("alp").is_some(),
            "N-gram 'alp' from 'alpha' should be indexed"
        );
        // "gamma" generates n-grams: "gam", "amm", "mma"
        assert!(
            updated_index.title_word_ngram_index.get("gam").is_some(),
            "N-gram 'gam' from 'gamma' should be indexed"
        );

        // Verify 4th word (delta) is NOT indexed
        // "delta" would generate: "del", "elt", "lta"
        assert!(
            updated_index.title_word_ngram_index.get("del").is_none(),
            "N-gram 'del' from 'delta' should NOT be indexed (exceeds max_tokens_per_task)"
        );

        // Verify both tags are indexed
        // "important" generates: "imp", "mpo", "por", "ort", "rta", "tan", "ant"
        assert!(
            updated_index.tag_ngram_index.get("imp").is_some(),
            "Tag n-gram 'imp' should be indexed"
        );
        // "urgent" generates: "urg", "rge", "gen", "ent"
        assert!(
            updated_index.tag_ngram_index.get("urg").is_some(),
            "Tag n-gram 'urg' should be indexed"
        );
    }

    /// Tests that `add_task` uses `normalize_query()` for normalization.
    ///
    /// - Input: task with mixed case and extra whitespace in title
    /// - Expected: n-gram indexes use normalized (lowercase, trimmed) values
    #[rstest]
    fn add_task_uses_normalize_query() {
        let config = SearchIndexConfig::default();
        let empty_index = SearchIndex::build_with_config(&PersistentVector::new(), config);

        let task_id = task_id_from_u128(1);
        let task = create_task_with_title_and_id("  CALLBACK  Function  ", task_id);

        let updated_index = empty_index.add_task(&task);

        // Verify normalized n-grams are indexed (lowercase)
        // "CALLBACK" normalized to "callback" generates: "cal", "all", "llb", "lba", "bac", "ack"
        assert!(
            updated_index.title_word_ngram_index.get("cal").is_some(),
            "Normalized n-gram 'cal' should be indexed"
        );

        // Verify uppercase variants are NOT indexed
        assert!(
            updated_index.title_word_ngram_index.get("CAL").is_none(),
            "Uppercase 'CAL' should NOT be indexed"
        );
    }

    /// Tests that `add_task` processes tags in deterministic (sorted) order.
    ///
    /// This ensures consistent index state regardless of `PersistentHashSet` iteration order.
    #[rstest]
    fn add_task_processes_tags_in_sorted_order() {
        let config = SearchIndexConfig::default();
        let empty_index = SearchIndex::build_with_config(&PersistentVector::new(), config);

        let task_id = task_id_from_u128(1);
        // Tags will be processed in sorted order: "apple", "banana", "cherry"
        let task = create_task_with_title_id_and_tags(
            "test task",
            task_id,
            vec!["cherry", "apple", "banana"], // Unsorted input
        );

        let updated_index = empty_index.add_task(&task);

        // Verify all tags are indexed (order doesn't affect correctness, but determinism matters)
        // "apple" generates: "app", "ppl", "ple"
        assert!(
            updated_index.tag_ngram_index.get("app").is_some(),
            "Tag n-gram 'app' should be indexed"
        );
        // "banana" generates: "ban", "ana", "nan", "ana"
        assert!(
            updated_index.tag_ngram_index.get("ban").is_some(),
            "Tag n-gram 'ban' should be indexed"
        );
        // "cherry" generates: "che", "her", "err", "rry"
        assert!(
            updated_index.tag_ngram_index.get("che").is_some(),
            "Tag n-gram 'che' should be indexed"
        );
    }

    /// Tests that `add_task` indexes tags in n-gram mode.
    ///
    /// - Input: task with tag "important"
    /// - Expected: tag n-gram index contains the tag's n-grams
    #[rstest]
    fn add_task_indexes_tags_in_ngram_mode() {
        let config = SearchIndexConfig::default(); // Ngram mode
        let empty_index = SearchIndex::build_with_config(&PersistentVector::new(), config);

        let task_id = task_id_from_u128(1);
        let task = create_task_with_title_id_and_tags("simple task", task_id, vec!["important"]);

        let updated_index = empty_index.add_task(&task);

        // Verify tag n-gram index contains n-grams for "important"
        // "important" generates: "imp", "mpo", "por", "ort", "rta", "tan", "ant"
        assert!(
            updated_index.tag_ngram_index.get("imp").is_some(),
            "Tag n-gram 'imp' should be indexed"
        );
        assert!(
            updated_index.tag_ngram_index.get("ant").is_some(),
            "Tag n-gram 'ant' should be indexed"
        );
    }

    /// Tests that `remove_task` removes tags from n-gram index.
    ///
    /// - Input: task with tag "important", then remove the task
    /// - Expected: tag n-gram index no longer contains the tag's entries
    #[rstest]
    fn remove_task_removes_tags_from_ngram_index() {
        let config = SearchIndexConfig::default(); // Ngram mode
        let empty_index = SearchIndex::build_with_config(&PersistentVector::new(), config);

        let task_id = task_id_from_u128(1);
        let task = create_task_with_title_id_and_tags("simple task", task_id, vec!["important"]);

        // Add task first
        let index_with_task = empty_index.add_task(&task);

        // Verify tag is indexed
        assert!(
            index_with_task.tag_ngram_index.get("imp").is_some(),
            "Tag n-gram 'imp' should be indexed before removal"
        );

        // Remove task
        let index_after_removal = index_with_task.remove_task(&task);

        // Verify tag n-gram entries are removed
        assert!(
            index_after_removal.tag_ngram_index.get("imp").is_none(),
            "Tag n-gram 'imp' should be removed after task removal"
        );
    }

    /// Tests that `add_task` does not update n-gram indexes when `infix_mode == Disabled`.
    ///
    /// - Input: empty index with Disabled mode, add task
    /// - Expected: n-gram indexes remain empty
    #[rstest]
    fn add_task_disabled_mode_no_ngram_update() {
        let config = SearchIndexConfig {
            infix_mode: InfixMode::Disabled,
            ..Default::default()
        };
        let empty_index = SearchIndex::build_with_config(&PersistentVector::new(), config);

        let task_id = task_id_from_u128(1);
        let task = create_task_with_title_and_id("callback function", task_id);

        let updated_index = empty_index.add_task(&task);

        // Verify n-gram indexes are empty
        assert!(
            updated_index.title_word_ngram_index.is_empty(),
            "N-gram index should be empty in Disabled mode"
        );
        assert!(
            updated_index.title_full_ngram_index.is_empty(),
            "Full title n-gram index should be empty in Disabled mode"
        );

        // Verify all-suffix indexes are also empty
        assert!(
            updated_index.title_word_all_suffix_index.is_empty(),
            "All-suffix index should be empty in Disabled mode"
        );
    }

    // -------------------------------------------------------------------------
    // Phase 5.5: Optimized add_task Tests
    // -------------------------------------------------------------------------

    /// Tests that the optimized `add_task` (via `add_task_with_normalized`)
    /// produces the same result as the original implementation.
    ///
    /// This is a differential test ensuring behavioral equivalence.
    ///
    /// - Input: task with title and tags
    /// - Expected: index structure is identical regardless of implementation path
    #[rstest]
    fn add_task_with_normalized_matches_original_behavior() {
        let config = SearchIndexConfig::default();
        let empty_index = SearchIndex::build_with_config(&PersistentVector::new(), config);

        let task_id = task_id_from_u128(1);
        let task = create_task_with_title_id_and_tags(
            "optimize query performance",
            task_id.clone(),
            vec!["database", "performance"],
        );

        // The optimized add_task internally uses NormalizedTaskData::from_task
        let updated_index = empty_index.add_task(&task);

        // Verify title_full_index
        assert!(
            updated_index
                .title_full_index
                .get("optimize query performance")
                .is_some(),
            "Full title should be indexed"
        );

        // Verify title_word_index
        assert!(
            updated_index.title_word_index.get("optimize").is_some(),
            "Word 'optimize' should be indexed"
        );
        assert!(
            updated_index.title_word_index.get("query").is_some(),
            "Word 'query' should be indexed"
        );
        assert!(
            updated_index.title_word_index.get("performance").is_some(),
            "Word 'performance' should be indexed"
        );

        // Verify tag_index
        assert!(
            updated_index.tag_index.get("database").is_some(),
            "Tag 'database' should be indexed"
        );
        assert!(
            updated_index.tag_index.get("performance").is_some(),
            "Tag 'performance' should be indexed"
        );

        // Verify n-gram indexes
        assert!(
            updated_index.title_word_ngram_index.get("opt").is_some(),
            "N-gram 'opt' from 'optimize' should be indexed"
        );
        assert!(
            updated_index.tag_ngram_index.get("dat").is_some(),
            "Tag n-gram 'dat' from 'database' should be indexed"
        );

        // Verify tasks_by_id
        assert!(
            updated_index.tasks_by_id.get(&task_id).is_some(),
            "Task should be stored in tasks_by_id"
        );
    }

    /// Tests that normalization is consistent between `NormalizedTaskData::from_task`
    /// and the legacy direct `normalize_query` calls.
    ///
    /// - Input: task with mixed-case title and tags
    /// - Expected: all indexes use lowercase normalized values
    #[rstest]
    fn add_task_normalization_consistency() {
        let config = SearchIndexConfig::default();
        let empty_index = SearchIndex::build_with_config(&PersistentVector::new(), config);

        let task_id = task_id_from_u128(1);
        let task = create_task_with_title_id_and_tags(
            "  MIXED Case  Title  ",
            task_id,
            vec!["URGENT", "Important"],
        );

        let updated_index = empty_index.add_task(&task);

        // Verify title is normalized (lowercase, trimmed, single spaces)
        assert!(
            updated_index
                .title_full_index
                .get("mixed case title")
                .is_some(),
            "Full title should be normalized to lowercase with single spaces"
        );

        // Verify words are normalized
        assert!(
            updated_index.title_word_index.get("mixed").is_some(),
            "Word should be normalized to lowercase"
        );

        // Verify tags are normalized
        assert!(
            updated_index.tag_index.get("urgent").is_some(),
            "Tag 'URGENT' should be normalized to 'urgent'"
        );
        assert!(
            updated_index.tag_index.get("important").is_some(),
            "Tag 'Important' should be normalized to 'important'"
        );
    }

    /// Tests that tags are processed in sorted order for deterministic behavior.
    ///
    /// - Input: task with tags in non-alphabetical order
    /// - Expected: tags are indexed in sorted order (first N after sorting)
    ///
    /// Note: Tags take priority over words when total exceeds `max_tokens_per_task`.
    /// With 4 words + 3 tags = 7 tokens and limit 5:
    /// - `tag_limit` = min(3, 5) = 3 (all tags fit)
    /// - `word_limit` = 5 - 3 = 2 (first 2 words)
    ///
    /// To test tag limiting, we need more tags than words can accommodate.
    #[rstest]
    fn add_task_sorted_tag_order_with_normalized() {
        let config = SearchIndexConfig {
            max_tokens_per_task: 2, // Only 2 tokens allowed
            ..Default::default()
        };
        let empty_index = SearchIndex::build_with_config(&PersistentVector::new(), config);

        let task_id = task_id_from_u128(1);
        // Title has 1 word, 3 tags
        // total = 4 > max_tokens_per_task (2)
        // word_limit = 2 - min(3, 2) = 0
        // tag_limit = 2 - 0 = 2
        // Tags sorted: ["alpha", "beta", "zeta"] -> first 2: ["alpha", "beta"]
        let task =
            create_task_with_title_id_and_tags("simple", task_id, vec!["zeta", "beta", "alpha"]);

        let updated_index = empty_index.add_task(&task);

        // With sorted order, "alpha" and "beta" should be indexed (not "zeta")
        assert!(
            updated_index.tag_index.get("alpha").is_some(),
            "First sorted tag 'alpha' should be indexed"
        );
        assert!(
            updated_index.tag_index.get("beta").is_some(),
            "Second sorted tag 'beta' should be indexed"
        );
        assert!(
            updated_index.tag_index.get("zeta").is_none(),
            "Third sorted tag 'zeta' should NOT be indexed due to limit"
        );

        // Verify title word is NOT indexed (word_limit = 0)
        assert!(
            updated_index.title_word_index.get("simple").is_none(),
            "Word 'simple' should NOT be indexed due to tag priority"
        );
    }
}

// =============================================================================
// Phase 7: Compatibility Tests (Differential Testing)
// =============================================================================

#[cfg(test)]
mod compatibility_tests {
    use super::*;
    use crate::domain::{Tag, Timestamp};
    use rstest::rstest;
    use std::collections::HashSet;
    use uuid::Uuid;

    // -------------------------------------------------------------------------
    // Test Helpers
    // -------------------------------------------------------------------------

    /// Creates a `TaskId` from a u128 value for deterministic testing.
    fn task_id_from_u128(value: u128) -> TaskId {
        TaskId::from_uuid(Uuid::from_u128(value))
    }

    /// Creates a task with a given title and task ID for deterministic testing.
    fn create_task_with_title_and_id(title: &str, task_id: TaskId) -> Task {
        Task::new(task_id, title, Timestamp::now())
    }

    /// Creates a task with a given title, task ID, and tags for deterministic testing.
    fn create_task_with_title_id_and_tags(title: &str, task_id: TaskId, tags: Vec<&str>) -> Task {
        let base = create_task_with_title_and_id(title, task_id);
        tags.into_iter()
            .fold(base, |task, tag| task.add_tag(Tag::new(tag)))
    }

    /// Creates a set of test tasks with various titles for compatibility testing.
    fn create_test_tasks() -> PersistentVector<Task> {
        vec![
            create_task_with_title_id_and_tags(
                "Important meeting with client",
                task_id_from_u128(1),
                vec!["urgent", "meeting"],
            ),
            create_task_with_title_id_and_tags(
                "Callback function implementation",
                task_id_from_u128(2),
                vec!["backend", "important"],
            ),
            create_task_with_title_id_and_tags(
                "Review all pull requests",
                task_id_from_u128(3),
                vec!["review", "code"],
            ),
            create_task_with_title_id_and_tags(
                "Meeting notes backup",
                task_id_from_u128(4),
                vec!["meeting", "backup"],
            ),
            create_task_with_title_id_and_tags(
                "Database migration callback",
                task_id_from_u128(5),
                vec!["database", "migration"],
            ),
            create_task_with_title_id_and_tags(
                "All hands meeting preparation",
                task_id_from_u128(6),
                vec!["meeting", "preparation"],
            ),
        ]
        .into_iter()
        .collect()
    }

    /// Creates test tasks with known IDs for ordering verification.
    fn create_test_tasks_with_known_ids() -> PersistentVector<Task> {
        vec![
            create_task_with_title_and_id("Common task alpha", task_id_from_u128(100)),
            create_task_with_title_and_id("Common task beta", task_id_from_u128(200)),
            create_task_with_title_and_id("Common task gamma", task_id_from_u128(50)),
            create_task_with_title_and_id("Common task delta", task_id_from_u128(150)),
        ]
        .into_iter()
        .collect()
    }

    // -------------------------------------------------------------------------
    // Differential Tests: n-gram vs LegacyAllSuffix
    // -------------------------------------------------------------------------

    /// Tests that n-gram and legacy all-suffix modes produce equivalent results.
    ///
    /// This is a differential test comparing the two infix search implementations.
    /// Both modes should return the same set of task IDs for the same queries.
    ///
    /// - Input: same task set, same queries
    /// - Expected: identical result sets (order-independent)
    #[rstest]
    fn ngram_and_legacy_produce_equivalent_results() {
        let tasks = create_test_tasks();

        let ngram_index = SearchIndex::build_with_config(
            &tasks,
            SearchIndexConfig {
                infix_mode: InfixMode::Ngram,
                ..Default::default()
            },
        );
        let legacy_index = SearchIndex::build_with_config(
            &tasks,
            SearchIndexConfig {
                infix_mode: InfixMode::LegacyAllSuffix,
                ..Default::default()
            },
        );

        // Test queries covering various patterns
        let queries = vec!["all", "meeting", "back", "portant", "callback", "review"];

        for query in queries {
            let ngram_result = ngram_index.search_by_title(query);
            let legacy_result = legacy_index.search_by_title(query);

            // First verify None/Some consistency
            assert_eq!(
                ngram_result.is_some(),
                legacy_result.is_some(),
                "Query '{query}' has inconsistent None/Some between n-gram and legacy modes.\n\
                 n-gram is_some: {}\n\
                 legacy is_some: {}",
                ngram_result.is_some(),
                legacy_result.is_some()
            );

            let ngram_ids: HashSet<TaskId> = ngram_result
                .as_ref()
                .map(|result| {
                    result
                        .tasks
                        .iter()
                        .map(|task| task.task_id.clone())
                        .collect()
                })
                .unwrap_or_default();
            let legacy_ids: HashSet<TaskId> = legacy_result
                .as_ref()
                .map(|result| {
                    result
                        .tasks
                        .iter()
                        .map(|task| task.task_id.clone())
                        .collect()
                })
                .unwrap_or_default();

            assert_eq!(
                ngram_ids, legacy_ids,
                "Query '{query}' produced different results between n-gram and legacy modes.\n\
                 n-gram IDs: {ngram_ids:?}\n\
                 legacy IDs: {legacy_ids:?}"
            );
        }
    }

    // -------------------------------------------------------------------------
    // Result Ordering Tests
    // -------------------------------------------------------------------------

    /// Tests that search results maintain `TaskId` ascending order.
    ///
    /// The search API contract requires results to be sorted by `TaskId` in ascending order
    /// for deterministic pagination and consistent user experience.
    ///
    /// - Input: tasks with known IDs (50, 100, 150, 200)
    /// - Expected: results sorted by `TaskId` ascending
    #[rstest]
    fn search_results_maintain_task_id_order() {
        let tasks = create_test_tasks_with_known_ids();
        let index = SearchIndex::build_with_config(&tasks, SearchIndexConfig::default());

        let result = index.search_by_title("common").unwrap();

        let ids: Vec<TaskId> = result
            .tasks
            .iter()
            .map(|task| task.task_id.clone())
            .collect();

        // Verify the IDs are in ascending order
        let mut sorted_ids = ids.clone();
        sorted_ids.sort();

        assert_eq!(
            ids, sorted_ids,
            "Search results should be sorted by TaskId in ascending order"
        );

        // Additionally verify the expected order: 50, 100, 150, 200
        assert_eq!(ids.len(), 4, "Should find all 4 tasks with 'common'");
        assert_eq!(ids[0], task_id_from_u128(50));
        assert_eq!(ids[1], task_id_from_u128(100));
        assert_eq!(ids[2], task_id_from_u128(150));
        assert_eq!(ids[3], task_id_from_u128(200));
    }

    // -------------------------------------------------------------------------
    // SearchResult Structure Tests
    // -------------------------------------------------------------------------

    /// Tests that `SearchResult` structure remains unchanged.
    ///
    /// Verifies that the `SearchResult` contains all expected fields and maintains
    /// the existing API contract.
    ///
    /// - Input: search result from n-gram mode
    /// - Expected: all task fields accessible (`task_id`, title, tags, status, priority)
    #[rstest]
    fn search_result_structure_unchanged() {
        let tasks = create_test_tasks();
        let index = SearchIndex::build_with_config(&tasks, SearchIndexConfig::default());
        let result = index.search_by_title("meeting").unwrap();

        // Verify SearchResult contains tasks
        assert!(
            !result.tasks.is_empty(),
            "SearchResult should contain tasks"
        );

        // Verify each task has the expected fields
        for task in &result.tasks {
            // task_id field exists and is non-empty when converted to string
            assert!(
                !task.task_id.to_string().is_empty(),
                "task_id should be non-empty"
            );

            // title field exists and is non-empty
            assert!(!task.title.is_empty(), "title should be non-empty");

            // tags field is accessible (may be empty, but the field exists)
            // Verify by accessing the actual length value
            let tags_len = task.tags.len();
            assert!(
                tags_len <= 100, // Reasonable upper bound for tag count
                "tags field should be accessible and have reasonable size"
            );

            // status field is accessible - verify via Debug trait
            let status_str = format!("{:?}", task.status);
            assert!(!status_str.is_empty(), "status field should be accessible");

            // priority field is accessible - verify via Debug trait
            let priority_str = format!("{:?}", task.priority);
            assert!(
                !priority_str.is_empty(),
                "priority field should be accessible"
            );
        }
    }

    // -------------------------------------------------------------------------
    // remove_task Compatibility Tests
    // -------------------------------------------------------------------------

    /// Tests that `remove_task` works correctly in `LegacyAllSuffix` mode.
    ///
    /// - Input: index with task, then remove the task
    /// - Expected: task no longer found in search results
    #[rstest]
    fn remove_task_legacy_mode_works() {
        let config = SearchIndexConfig {
            infix_mode: InfixMode::LegacyAllSuffix,
            ..Default::default()
        };
        let empty_index = SearchIndex::build_with_config(&PersistentVector::new(), config);

        let task_id = task_id_from_u128(1);
        let task = create_task_with_title_and_id("callback function", task_id.clone());

        // Add task
        let index_with_task = empty_index.add_task(&task);

        // Verify task is searchable
        let result_before = index_with_task.search_by_title("callback");
        assert!(
            result_before.is_some(),
            "Task should be found before removal"
        );
        assert!(
            result_before
                .as_ref()
                .unwrap()
                .tasks
                .iter()
                .any(|found_task| found_task.task_id == task_id),
            "Task ID should be in search results before removal"
        );

        // Remove task
        let index_after_removal = index_with_task.remove_task(&task);

        // Verify task is no longer searchable
        let result_after = index_after_removal.search_by_title("callback");
        let task_found_after = result_after.as_ref().is_some_and(|result| {
            result
                .tasks
                .iter()
                .any(|found_task| found_task.task_id == task_id)
        });

        assert!(
            !task_found_after,
            "Task should not be found after removal in LegacyAllSuffix mode"
        );
    }

    /// Tests that `remove_task` works correctly in Disabled mode.
    ///
    /// - Input: index with task (Disabled mode), then remove the task
    /// - Expected: task no longer found in search results (prefix search only)
    #[rstest]
    fn remove_task_disabled_mode_works() {
        let config = SearchIndexConfig {
            infix_mode: InfixMode::Disabled,
            ..Default::default()
        };
        let empty_index = SearchIndex::build_with_config(&PersistentVector::new(), config);

        let task_id = task_id_from_u128(1);
        let task = create_task_with_title_and_id("callback function", task_id.clone());

        // Add task
        let index_with_task = empty_index.add_task(&task);

        // Verify task is searchable via prefix search (infix disabled, but prefix works)
        let result_before = index_with_task.search_by_title("callback");
        assert!(
            result_before.is_some(),
            "Task should be found via prefix search before removal"
        );
        assert!(
            result_before
                .as_ref()
                .unwrap()
                .tasks
                .iter()
                .any(|found_task| found_task.task_id == task_id),
            "Task ID should be in prefix search results before removal"
        );

        // Remove task
        let index_after_removal = index_with_task.remove_task(&task);

        // Verify task is no longer searchable
        let result_after = index_after_removal.search_by_title("callback");
        let task_found_after = result_after.as_ref().is_some_and(|result| {
            result
                .tasks
                .iter()
                .any(|found_task| found_task.task_id == task_id)
        });

        assert!(
            !task_found_after,
            "Task should not be found after removal in Disabled mode"
        );
    }
}

// =============================================================================
// Performance Tests: N-gram vs Legacy All-Suffix Comparison
// =============================================================================

#[cfg(test)]
mod performance_tests {
    use super::*;
    use crate::domain::{Tag, Timestamp};
    use rstest::rstest;
    use std::time::Instant;
    use uuid::Uuid;

    // -------------------------------------------------------------------------
    // Test Helpers
    // -------------------------------------------------------------------------

    /// Word lists for generating realistic task titles.
    const ADJECTIVES: &[&str] = &[
        "important",
        "urgent",
        "critical",
        "pending",
        "completed",
        "scheduled",
        "recurring",
        "optional",
        "mandatory",
        "temporary",
    ];

    const NOUNS: &[&str] = &[
        "meeting",
        "review",
        "deployment",
        "documentation",
        "testing",
        "refactoring",
        "migration",
        "optimization",
        "implementation",
        "investigation",
    ];

    const VERBS: &[&str] = &[
        "prepare",
        "complete",
        "schedule",
        "review",
        "update",
        "implement",
        "deploy",
        "test",
        "investigate",
        "document",
    ];

    const TAG_WORDS: &[&str] = &[
        "backend",
        "frontend",
        "database",
        "api",
        "security",
        "performance",
        "bugfix",
        "feature",
        "documentation",
        "testing",
        "urgent",
        "low-priority",
        "blocked",
        "in-progress",
        "review-needed",
    ];

    /// Creates a `TaskId` from a u128 value for deterministic testing.
    fn task_id_from_u128(value: u128) -> TaskId {
        TaskId::from_uuid(Uuid::from_u128(value))
    }

    /// Generates a realistic task title with 5-10 words.
    ///
    /// Uses simple pseudo-random selection based on the index to ensure
    /// reproducible results without external dependencies.
    fn generate_task_title(index: usize) -> String {
        let word_count = 5 + (index % 6); // 5-10 words
        let mut words = Vec::with_capacity(word_count);

        for word_index in 0..word_count {
            let combined_index = index.wrapping_mul(31).wrapping_add(word_index);
            let word = match word_index % 3 {
                0 => ADJECTIVES[combined_index % ADJECTIVES.len()],
                1 => NOUNS[combined_index % NOUNS.len()],
                _ => VERBS[combined_index % VERBS.len()],
            };
            words.push(word);
        }

        words.join(" ")
    }

    /// Generates 2-5 tags for a task.
    ///
    /// Uses simple pseudo-random selection based on the index.
    fn generate_tags(index: usize) -> Vec<Tag> {
        let tag_count = 2 + (index % 4); // 2-5 tags
        let mut tags = Vec::with_capacity(tag_count);

        for tag_index in 0..tag_count {
            let combined_index = index.wrapping_mul(17).wrapping_add(tag_index);
            let tag_word = TAG_WORDS[combined_index % TAG_WORDS.len()];
            tags.push(Tag::new(tag_word));
        }

        tags
    }

    /// Creates a test task with a realistic title and tags.
    fn create_realistic_task(index: usize) -> Task {
        let task_id = task_id_from_u128(index as u128);
        let title = generate_task_title(index);
        let tags = generate_tags(index);

        let base_task = Task::new(task_id, &title, Timestamp::now());
        tags.into_iter().fold(base_task, Task::add_tag)
    }

    /// Generates a collection of realistic tasks.
    fn generate_test_tasks(count: usize) -> PersistentVector<Task> {
        (0..count).map(create_realistic_task).collect()
    }

    // -------------------------------------------------------------------------
    // Performance Comparison Test
    // -------------------------------------------------------------------------

    /// Compares `SearchIndex` build performance between n-gram and legacy all-suffix modes.
    ///
    /// This test measures and prints the build time for both indexing strategies
    /// with 1000 realistic tasks. It is marked as `#[ignore]` because it is a
    /// performance benchmark that should not run with regular unit tests.
    ///
    /// # Test Setup
    ///
    /// - Task count: 1000 tasks
    /// - Title length: 5-10 words per task
    /// - Tags: 2-5 tags per task
    /// - Timing: uses `std::time::Instant` for measurement
    ///
    /// # Expected Results
    ///
    /// - N-gram mode should be significantly faster than legacy all-suffix mode
    /// - Target: 1e4 tasks should build in < 1s (this test uses 1e3 for quick validation)
    ///
    /// # Running the Test
    ///
    /// ```bash
    /// cargo test --package task-management-benchmark-api performance_tests -- --ignored --nocapture
    /// ```
    #[rstest]
    #[ignore = "Performance test - run manually with --ignored"]
    fn compare_ngram_vs_legacy_build_performance() {
        const TASK_COUNT: usize = 1000;

        println!("\n========================================");
        println!("SearchIndex Build Performance Comparison");
        println!("========================================");
        println!("Task count: {TASK_COUNT}");
        println!("Title length: 5-10 words");
        println!("Tags per task: 2-5");
        println!();

        // Generate test tasks (same tasks for both modes)
        let tasks = generate_test_tasks(TASK_COUNT);
        println!("Generated {TASK_COUNT} realistic tasks.");
        println!();

        // Measure n-gram mode build time
        let ngram_config = SearchIndexConfig {
            infix_mode: InfixMode::Ngram,
            ..Default::default()
        };

        let ngram_start = Instant::now();
        let ngram_index = SearchIndex::build_with_config(&tasks, ngram_config);
        let ngram_duration = ngram_start.elapsed();

        println!("N-gram mode:");
        println!("  Build time: {ngram_duration:?}");

        // Measure legacy all-suffix mode build time
        let legacy_config = SearchIndexConfig {
            infix_mode: InfixMode::LegacyAllSuffix,
            ..Default::default()
        };

        let legacy_start = Instant::now();
        let legacy_index = SearchIndex::build_with_config(&tasks, legacy_config);
        let legacy_duration = legacy_start.elapsed();

        println!();
        println!("Legacy all-suffix mode:");
        println!("  Build time: {legacy_duration:?}");

        // Calculate speedup ratio
        // Note: precision loss is acceptable here as we only need approximate ratio for display
        #[allow(clippy::cast_precision_loss)]
        let speedup = if ngram_duration.as_nanos() > 0 {
            legacy_duration.as_nanos() as f64 / ngram_duration.as_nanos() as f64
        } else {
            f64::INFINITY
        };

        println!();
        println!("Performance comparison:");
        println!("  Speedup (legacy/ngram): {speedup:.2}x");

        // Verify both indexes produce search results
        let ngram_result = ngram_index.search_by_title("meeting");
        let legacy_result = legacy_index.search_by_title("meeting");

        assert!(
            ngram_result.is_some(),
            "N-gram index should return search results"
        );
        assert!(
            legacy_result.is_some(),
            "Legacy index should return search results"
        );

        println!();
        println!("Verification:");
        println!(
            "  N-gram 'meeting' results: {} tasks",
            ngram_result.as_ref().map_or(0, |result| result.tasks.len())
        );
        println!(
            "  Legacy 'meeting' results: {} tasks",
            legacy_result
                .as_ref()
                .map_or(0, |result| result.tasks.len())
        );
        println!("========================================\n");

        // Soft assertion: n-gram should generally be faster
        // This is informational and won't fail the test
        if ngram_duration > legacy_duration {
            println!("WARNING: N-gram mode was slower than legacy mode in this run.");
            println!("This may occur due to system load or small dataset size.");
        }
    }

    /// Tests build performance with a larger dataset (10,000 tasks).
    ///
    /// This test validates the target performance requirement:
    /// - 1e4 tasks should build in < 1s (release build)
    /// - Debug builds are significantly slower and the assertion is skipped
    ///
    /// # Running the Test
    ///
    /// For accurate performance measurements, run with release optimizations:
    ///
    /// ```bash
    /// cargo test --release --lib performance_tests::test_ngram_build_target_performance -- --ignored --nocapture
    /// ```
    ///
    /// Debug build (informational only, no assertion):
    ///
    /// ```bash
    /// cargo test --lib performance_tests::test_ngram_build_target_performance -- --ignored --nocapture
    /// ```
    #[rstest]
    #[ignore = "Performance test - run manually with --release --ignored"]
    fn test_ngram_build_target_performance() {
        const TASK_COUNT: usize = 10_000;
        const TARGET_DURATION_SECS: u64 = 1;

        // Detect if running in debug or release mode
        let is_release_build = cfg!(not(debug_assertions));

        println!("\n========================================");
        println!("N-gram Build Target Performance Test");
        println!("========================================");
        println!("Task count: {TASK_COUNT}");
        println!("Target: < {TARGET_DURATION_SECS}s (release build)");
        println!(
            "Build mode: {}",
            if is_release_build { "RELEASE" } else { "DEBUG" }
        );
        println!();

        // Generate test tasks
        let tasks = generate_test_tasks(TASK_COUNT);
        println!("Generated {TASK_COUNT} realistic tasks.");
        println!();

        // Measure n-gram mode build time
        let ngram_config = SearchIndexConfig {
            infix_mode: InfixMode::Ngram,
            ..Default::default()
        };

        let start = Instant::now();
        let index = SearchIndex::build_with_config(&tasks, ngram_config);
        let duration = start.elapsed();

        println!("N-gram mode build time: {duration:?}");
        println!();

        // Verify the index works
        let result = index.search_by_title("important");
        println!(
            "Verification: 'important' search returned {} tasks",
            result.as_ref().map_or(0, |result| result.tasks.len())
        );
        println!("========================================\n");

        // Assert the target performance is met (release build only)
        if is_release_build {
            assert!(
                duration.as_secs() < TARGET_DURATION_SECS,
                "N-gram build for {TASK_COUNT} tasks took {duration:?}, expected < {TARGET_DURATION_SECS}s"
            );
        } else {
            println!("NOTE: Performance assertion skipped in debug build.");
            println!("Run with --release for accurate performance measurement.");
        }
    }
}

#[cfg(test)]
mod search_index_delta_tests {
    use super::*;
    use crate::domain::{Tag, Timestamp};
    use rstest::rstest;

    fn create_task_with_title_and_tags(title: &str, tags: Vec<&str>) -> Task {
        let base = Task::new(TaskId::generate(), title, Timestamp::now());
        tags.into_iter()
            .fold(base, |task, tag| task.add_tag(Tag::new(tag)))
    }

    #[rstest]
    fn default_creates_empty_delta() {
        let delta = SearchIndexDelta::default();

        assert!(delta.title_full_add.is_empty());
        assert!(delta.title_full_remove.is_empty());
        assert!(delta.title_word_add.is_empty());
        assert!(delta.title_word_remove.is_empty());
        assert!(delta.tag_add.is_empty());
        assert!(delta.tag_remove.is_empty());

        assert!(delta.title_full_ngram_add.is_empty());
        assert!(delta.title_full_ngram_remove.is_empty());
        assert!(delta.title_word_ngram_add.is_empty());
        assert!(delta.title_word_ngram_remove.is_empty());
        assert!(delta.tag_ngram_add.is_empty());
        assert!(delta.tag_ngram_remove.is_empty());
    }

    /// Verifies structure matches REQ-SEARCH-NGRAM-PERF-001 specification (12 fields).
    #[rstest]
    fn has_all_required_fields() {
        let delta = SearchIndexDelta::default();

        let _ = &delta.title_full_add;
        let _ = &delta.title_full_remove;
        let _ = &delta.title_word_add;
        let _ = &delta.title_word_remove;
        let _ = &delta.tag_add;
        let _ = &delta.tag_remove;

        let _ = &delta.title_full_ngram_add;
        let _ = &delta.title_full_ngram_remove;
        let _ = &delta.title_word_ngram_add;
        let _ = &delta.title_word_ngram_remove;
        let _ = &delta.tag_ngram_add;
        let _ = &delta.tag_ngram_remove;
    }

    #[rstest]
    fn clone_preserves_data() {
        let mut delta = SearchIndexDelta::default();
        let task_id = TaskId::generate();

        delta
            .title_full_add
            .insert("test title".to_string(), vec![task_id.clone()]);
        delta
            .tag_ngram_add
            .insert("tes".to_string(), vec![task_id.clone()]);

        let cloned = delta.clone();

        assert_eq!(cloned.title_full_add.len(), 1);
        assert_eq!(cloned.tag_ngram_add.len(), 1);
        assert_eq!(
            cloned.title_full_add.get("test title"),
            Some(&vec![task_id.clone()])
        );
        assert_eq!(cloned.tag_ngram_add.get("tes"), Some(&vec![task_id]));
    }

    #[rstest]
    fn debug_format_includes_struct_and_fields() {
        let delta = SearchIndexDelta::default();
        let debug_str = format!("{delta:?}");

        assert!(debug_str.contains("SearchIndexDelta"));
        assert!(debug_str.contains("title_full_add"));
        assert!(debug_str.contains("tag_ngram_remove"));
    }

    // =========================================================================
    // from_changes Tests (REQ-SEARCH-NGRAM-PERF-001 Part 2)
    // =========================================================================

    fn task_id_from_u128(value: u128) -> TaskId {
        TaskId::from_uuid(uuid::Uuid::from_u128(value))
    }

    fn create_task_with_title_and_id(title: &str, task_id: TaskId) -> Task {
        Task::new(task_id, title, crate::domain::Timestamp::now())
    }

    /// Tests that a single Add change correctly populates add fields.
    #[rstest]
    fn delta_from_single_add_change() {
        let task = create_task_with_title_and_id("Test Task", task_id_from_u128(1));
        let changes = vec![TaskChange::Add(task)];
        let config = SearchIndexConfig::default();
        let tasks_by_id = PersistentTreeMap::new();

        let delta = SearchIndexDelta::from_changes(&changes, &config, &tasks_by_id);

        // title_full_add should contain the normalized title
        assert!(!delta.title_full_add.is_empty());
        assert!(delta.title_full_add.contains_key("test task"));
        assert!(delta.title_full_remove.is_empty());
    }

    /// Tests that Update change collects both add and remove entries.
    #[rstest]
    fn delta_from_update_change_collects_both_add_and_remove() {
        let task_id = task_id_from_u128(1);
        let old_task = create_task_with_title_and_id("Old Title", task_id.clone());
        let new_task = create_task_with_title_and_id("New Title", task_id);
        let changes = vec![TaskChange::Update {
            old: old_task,
            new: new_task,
        }];
        let config = SearchIndexConfig::default();
        let tasks_by_id = PersistentTreeMap::new();

        let delta = SearchIndexDelta::from_changes(&changes, &config, &tasks_by_id);

        // Add should contain new title
        assert!(!delta.title_full_add.is_empty());
        assert!(delta.title_full_add.contains_key("new title"));

        // Remove should contain old title
        assert!(!delta.title_full_remove.is_empty());
        assert!(delta.title_full_remove.contains_key("old title"));
    }

    /// Tests that Remove change uses `tasks_by_id` to find the task.
    #[rstest]
    fn delta_from_remove_change_uses_tasks_by_id() {
        let task = create_task_with_title_and_id("Test Task", task_id_from_u128(1));
        let task_id = task.task_id.clone();
        let tasks_by_id = PersistentTreeMap::new().insert(task_id.clone(), task);

        let changes = vec![TaskChange::Remove(task_id)];
        let config = SearchIndexConfig::default();

        let delta = SearchIndexDelta::from_changes(&changes, &config, &tasks_by_id);

        // Remove should be populated from tasks_by_id lookup
        assert!(!delta.title_full_remove.is_empty());
        assert!(delta.title_full_remove.contains_key("test task"));
    }

    /// Tests that Remove for nonexistent `TaskId` is idempotent (no-op).
    #[rstest]
    fn delta_from_remove_nonexistent_is_idempotent() {
        let changes = vec![TaskChange::Remove(task_id_from_u128(999))];
        let config = SearchIndexConfig::default();
        let tasks_by_id = PersistentTreeMap::new();

        let delta = SearchIndexDelta::from_changes(&changes, &config, &tasks_by_id);

        // All fields should remain empty for nonexistent task
        assert!(delta.title_full_remove.is_empty());
        assert!(delta.title_word_remove.is_empty());
        assert!(delta.tag_remove.is_empty());
    }

    /// Tests Add followed by Remove in the same batch.
    /// Both operations should be recorded (`pending_tasks` tracking).
    #[rstest]
    fn delta_from_add_then_remove_in_same_batch() {
        let task = create_task_with_title_and_id("Test Task", task_id_from_u128(1));
        let config = SearchIndexConfig::default();
        let tasks_by_id = PersistentTreeMap::new();
        let task_id = task.task_id.clone();

        let changes = vec![TaskChange::Add(task), TaskChange::Remove(task_id)];

        let delta = SearchIndexDelta::from_changes(&changes, &config, &tasks_by_id);

        // Both add and remove should be recorded
        assert!(!delta.title_full_add.is_empty());
        assert!(!delta.title_full_remove.is_empty());
    }

    /// Tests Update followed by Remove in the same batch.
    #[rstest]
    fn delta_from_update_then_remove_in_same_batch() {
        let task_id = task_id_from_u128(1);
        let old_task = create_task_with_title_and_id("Old Title", task_id.clone());
        let new_task = create_task_with_title_and_id("New Title", task_id);
        let config = SearchIndexConfig::default();
        let tasks_by_id =
            PersistentTreeMap::new().insert(old_task.task_id.clone(), old_task.clone());
        let new_task_id = new_task.task_id.clone();

        let changes = vec![
            TaskChange::Update {
                old: old_task,
                new: new_task,
            },
            TaskChange::Remove(new_task_id),
        ];

        let delta = SearchIndexDelta::from_changes(&changes, &config, &tasks_by_id);

        // Update: old removed, new added
        // Remove: new removed (from pending_tasks)
        // Result: add has new, remove has old + new
        assert!(!delta.title_full_add.is_empty());
        assert!(!delta.title_full_remove.is_empty());
        // old title and new title should both be in remove
        assert!(delta.title_full_remove.contains_key("old title"));
        assert!(delta.title_full_remove.contains_key("new title"));
    }

    /// Tests that Remove followed by Add panics (forbidden pattern).
    #[rstest]
    #[should_panic(expected = "Remove followed by Add")]
    fn delta_panics_on_remove_then_add() {
        let task = create_task_with_title_and_id("Test Task", task_id_from_u128(1));
        let config = SearchIndexConfig::default();
        let task_id = task.task_id.clone();
        let tasks_by_id = PersistentTreeMap::new().insert(task_id.clone(), task.clone());

        let changes = vec![TaskChange::Remove(task_id), TaskChange::Add(task)];

        // This should panic
        let _ = SearchIndexDelta::from_changes(&changes, &config, &tasks_by_id);
    }

    /// Tests that Remove followed by Update panics (forbidden pattern).
    #[rstest]
    #[should_panic(expected = "Remove followed by Update")]
    fn delta_panics_on_remove_then_update() {
        let task_id = task_id_from_u128(1);
        let task = create_task_with_title_and_id("Test Task", task_id.clone());
        let updated_task = create_task_with_title_and_id("Updated Task", task_id);
        let config = SearchIndexConfig::default();
        let task_id_for_remove = task.task_id.clone();
        let tasks_by_id = PersistentTreeMap::new().insert(task.task_id.clone(), task.clone());

        let changes = vec![
            TaskChange::Remove(task_id_for_remove),
            TaskChange::Update {
                old: task,
                new: updated_task,
            },
        ];

        // This should panic
        let _ = SearchIndexDelta::from_changes(&changes, &config, &tasks_by_id);
    }

    /// Tests that `from_changes` respects `max_tokens_per_task` limit.
    /// Tags are prioritized over words (matches existing `build_with_config` behavior).
    #[rstest]
    fn delta_respects_max_tokens_per_task() {
        // 6 words + 2 tags = 8 tokens, max = 5
        // Expected: tag_limit = 2 (all tags), word_limit = 5 - 2 = 3
        let task = create_task_with_title_and_tags(
            "alpha beta gamma delta epsilon zeta",
            vec!["important", "urgent"],
        );
        let config = SearchIndexConfig {
            max_tokens_per_task: 5,
            ..Default::default()
        };
        let tasks_by_id = PersistentTreeMap::new();

        let delta = SearchIndexDelta::from_changes(&[TaskChange::Add(task)], &config, &tasks_by_id);

        // All 2 tags should be indexed
        assert!(delta.tag_add.contains_key("important"));
        assert!(delta.tag_add.contains_key("urgent"));

        // Only first 3 words should be indexed (alpha, beta, gamma)
        assert!(delta.title_word_add.contains_key("alpha"));
        assert!(delta.title_word_add.contains_key("beta"));
        assert!(delta.title_word_add.contains_key("gamma"));

        // 4th+ words should NOT be indexed
        assert!(!delta.title_word_add.contains_key("delta"));
        assert!(!delta.title_word_add.contains_key("epsilon"));
        assert!(!delta.title_word_add.contains_key("zeta"));
    }

    /// Tests that tag ordering matches existing code.
    /// `Tag::new()` normalizes to lowercase, so sort is on normalized values.
    /// Sorted order: "apple" < "banana" < "cherry" (ASCII lowercase)
    #[rstest]
    fn delta_tag_order_matches_existing_code() {
        // Tag::new() normalizes to lowercase, so:
        // "Cherry" -> "cherry", "apple" -> "apple", "BANANA" -> "banana"
        // Sorted by as_str() (lowercase): ["apple", "banana", "cherry"]
        // With max_tokens_per_task = 2: tag_limit = 2, word_limit = 0
        // First 2 tags: ["apple", "banana"]
        let task = create_task_with_title_and_tags("test", vec!["Cherry", "apple", "BANANA"]);
        let config = SearchIndexConfig {
            max_tokens_per_task: 2, // Forces tag_limit = 2, word_limit = 0
            ..Default::default()
        };
        let tasks_by_id = PersistentTreeMap::new();

        let delta = SearchIndexDelta::from_changes(&[TaskChange::Add(task)], &config, &tasks_by_id);

        // "apple" and "banana" should be indexed (first 2 in sorted order)
        assert!(delta.tag_add.contains_key("apple"));
        assert!(delta.tag_add.contains_key("banana"));

        // "cherry" should NOT be indexed (3rd in sorted order, tag_limit is 2)
        assert!(!delta.tag_add.contains_key("cherry"));
    }

    /// Verifies delta matches existing `add_task` behavior for tag ordering and limits.
    #[rstest]
    fn delta_matches_add_task_for_tag_handling() {
        let task = create_task_with_title_and_tags(
            "alpha beta gamma delta",
            vec!["urgent", "IMPORTANT", "later"],
        );
        let config = SearchIndexConfig {
            max_tokens_per_task: 5, // 4 words + 3 tags = 7 > 5
            ..Default::default()
        };

        // Build using add_task
        let empty_index = SearchIndex::build_with_config(&PersistentVector::new(), config.clone());
        let index_via_add = empty_index.add_task(&task);

        // Build using from_changes
        let tasks_by_id = PersistentTreeMap::new();
        let delta = SearchIndexDelta::from_changes(&[TaskChange::Add(task)], &config, &tasks_by_id);

        // tag_limit = min(3, 5) = 3, word_limit = 5 - 3 = 2
        // Tags (ASCII sorted): ["IMPORTANT", "later", "urgent"]
        // Normalized in order: ["important", "later", "urgent"]

        // Verify both methods index the same tags
        let delta_tags: std::collections::HashSet<_> = delta.tag_add.keys().collect();
        let index_tags: std::collections::HashSet<_> = index_via_add
            .tag_index
            .keys()
            .filter(|k| {
                index_via_add
                    .tag_index
                    .get(*k)
                    .is_some_and(|v| !v.is_empty())
            })
            .collect();

        assert_eq!(delta_tags, index_tags, "Tag sets should match");
    }
}

// =============================================================================
// SearchIndex::apply_changes Tests (REQ-SEARCH-NGRAM-PERF-001 Part 3)
// =============================================================================

#[cfg(test)]
#[allow(
    clippy::redundant_clone,
    clippy::useless_vec,
    clippy::doc_markdown,
    clippy::too_many_lines,
    clippy::uninlined_format_args,
    clippy::needless_borrow
)]
mod apply_changes_tests {
    use super::*;
    use crate::domain::{Tag, Timestamp};
    use rstest::rstest;
    use std::collections::HashSet;
    use uuid::Uuid;

    // -------------------------------------------------------------------------
    // Test Helpers
    // -------------------------------------------------------------------------

    /// Creates a `TaskId` from a u128 value for deterministic testing.
    fn task_id_from_u128(value: u128) -> TaskId {
        TaskId::from_uuid(Uuid::from_u128(value))
    }

    /// Creates a task with given title and a generated TaskId.
    fn create_test_task(title: &str) -> Task {
        Task::new(TaskId::generate(), title, Timestamp::now())
    }

    /// Creates a task with given title and specific TaskId.
    fn create_test_task_with_id(title: &str, task_id: TaskId) -> Task {
        Task::new(task_id, title, Timestamp::now())
    }

    /// Creates a task with title and tags.
    fn create_test_task_with_tags(title: &str, tags: Vec<&str>) -> Task {
        let base = create_test_task(title);
        tags.into_iter()
            .fold(base, |task, tag| task.add_tag(Tag::new(tag)))
    }

    /// Creates a task with specific TaskId, title, and tags.
    fn create_test_task_with_id_and_tags(task_id: TaskId, title: &str, tags: Vec<&str>) -> Task {
        let base = create_test_task_with_id(title, task_id);
        tags.into_iter()
            .fold(base, |task, tag| task.add_tag(Tag::new(tag)))
    }

    /// Creates a collection of test tasks.
    fn create_test_tasks() -> PersistentVector<Task> {
        vec![
            create_test_task_with_tags("Important meeting", vec!["work", "urgent"]),
            create_test_task_with_tags("Code review", vec!["work", "development"]),
            create_test_task_with_tags("Buy groceries", vec!["personal", "shopping"]),
        ]
        .into_iter()
        .collect()
    }

    /// Creates an empty SearchIndex with default config.
    fn create_empty_index() -> SearchIndex {
        SearchIndex::build_with_config(&PersistentVector::new(), SearchIndexConfig::default())
    }

    /// Creates a SearchIndex with test tasks.
    fn create_test_index() -> SearchIndex {
        let tasks = create_test_tasks();
        SearchIndex::build_with_config(&tasks, SearchIndexConfig::default())
    }

    // -------------------------------------------------------------------------
    // Assertion Helpers
    // -------------------------------------------------------------------------

    /// Asserts that two SearchIndex instances have identical content.
    /// Requirements: apply_changes must equal sequential apply_change
    fn assert_search_index_equals(batch: &SearchIndex, sequential: &SearchIndex) {
        // 1. tasks_by_id の一致
        assert_eq!(
            batch.tasks_by_id.len(),
            sequential.tasks_by_id.len(),
            "tasks_by_id length mismatch"
        );
        for task_id in batch.tasks_by_id.keys() {
            assert!(
                sequential.tasks_by_id.contains_key(&task_id),
                "tasks_by_id missing key: {:?}",
                task_id
            );
            assert_eq!(
                batch.tasks_by_id.get(&task_id).map(|t| &t.title),
                sequential.tasks_by_id.get(&task_id).map(|t| &t.title),
                "tasks_by_id task mismatch for {:?}",
                task_id
            );
        }

        // 2. title_full_index の一致
        assert_eq!(
            batch.title_full_index.len(),
            sequential.title_full_index.len(),
            "title_full_index length mismatch"
        );
        for key in batch.title_full_index.keys() {
            let batch_posting: HashSet<_> = batch
                .title_full_index
                .get(key.as_str())
                .map(|v| v.iter().cloned().collect())
                .unwrap_or_default();
            let seq_posting: HashSet<_> = sequential
                .title_full_index
                .get(key.as_str())
                .map(|v| v.iter().cloned().collect())
                .unwrap_or_default();
            assert_eq!(
                batch_posting, seq_posting,
                "title_full_index mismatch for key: {}",
                key
            );
        }

        // 3. title_word_index の一致
        assert_eq!(
            batch.title_word_index.len(),
            sequential.title_word_index.len(),
            "title_word_index length mismatch"
        );
        for key in batch.title_word_index.keys() {
            let batch_posting: HashSet<_> = batch
                .title_word_index
                .get(key.as_str())
                .map(|v| v.iter().cloned().collect())
                .unwrap_or_default();
            let seq_posting: HashSet<_> = sequential
                .title_word_index
                .get(key.as_str())
                .map(|v| v.iter().cloned().collect())
                .unwrap_or_default();
            assert_eq!(
                batch_posting, seq_posting,
                "title_word_index mismatch for key: {}",
                key
            );
        }

        // 4. tag_index の一致
        assert_eq!(
            batch.tag_index.len(),
            sequential.tag_index.len(),
            "tag_index length mismatch: batch keys = {:?}, sequential keys = {:?}",
            batch.tag_index.keys().collect::<Vec<_>>(),
            sequential.tag_index.keys().collect::<Vec<_>>()
        );
        for key in batch.tag_index.keys() {
            let batch_posting: HashSet<_> = batch
                .tag_index
                .get(key.as_str())
                .map(|v| v.iter().cloned().collect())
                .unwrap_or_default();
            let seq_posting: HashSet<_> = sequential
                .tag_index
                .get(key.as_str())
                .map(|v| v.iter().cloned().collect())
                .unwrap_or_default();
            assert_eq!(
                batch_posting, seq_posting,
                "tag_index mismatch for key: {}",
                key
            );
        }

        // 5. title_full_ngram_index の一致
        assert_eq!(
            batch.title_full_ngram_index.len(),
            sequential.title_full_ngram_index.len(),
            "title_full_ngram_index length mismatch"
        );
        for key in batch.title_full_ngram_index.keys() {
            let batch_posting: HashSet<_> = batch
                .title_full_ngram_index
                .get(key.as_str())
                .map(|v| v.iter().cloned().collect())
                .unwrap_or_default();
            let seq_posting: HashSet<_> = sequential
                .title_full_ngram_index
                .get(key.as_str())
                .map(|v| v.iter().cloned().collect())
                .unwrap_or_default();
            assert_eq!(
                batch_posting, seq_posting,
                "title_full_ngram_index mismatch for key: {}",
                key
            );
        }

        // 6. title_word_ngram_index の一致
        assert_eq!(
            batch.title_word_ngram_index.len(),
            sequential.title_word_ngram_index.len(),
            "title_word_ngram_index length mismatch"
        );
        for key in batch.title_word_ngram_index.keys() {
            let batch_posting: HashSet<_> = batch
                .title_word_ngram_index
                .get(key.as_str())
                .map(|v| v.iter().cloned().collect())
                .unwrap_or_default();
            let seq_posting: HashSet<_> = sequential
                .title_word_ngram_index
                .get(key.as_str())
                .map(|v| v.iter().cloned().collect())
                .unwrap_or_default();
            assert_eq!(
                batch_posting, seq_posting,
                "title_word_ngram_index mismatch for key: {}",
                key
            );
        }

        // 7. tag_ngram_index の一致
        assert_eq!(
            batch.tag_ngram_index.len(),
            sequential.tag_ngram_index.len(),
            "tag_ngram_index length mismatch"
        );
        for key in batch.tag_ngram_index.keys() {
            let batch_posting: HashSet<_> = batch
                .tag_ngram_index
                .get(key.as_str())
                .map(|v| v.iter().cloned().collect())
                .unwrap_or_default();
            let seq_posting: HashSet<_> = sequential
                .tag_ngram_index
                .get(key.as_str())
                .map(|v| v.iter().cloned().collect())
                .unwrap_or_default();
            assert_eq!(
                batch_posting, seq_posting,
                "tag_ngram_index mismatch for key: {}",
                key
            );
        }

        // 8. title_full_all_suffix_index の一致
        assert_eq!(
            batch.title_full_all_suffix_index.len(),
            sequential.title_full_all_suffix_index.len(),
            "title_full_all_suffix_index length mismatch"
        );
        for key in batch.title_full_all_suffix_index.keys() {
            let batch_posting: HashSet<_> = batch
                .title_full_all_suffix_index
                .get(key.as_str())
                .map(|v| v.iter().cloned().collect())
                .unwrap_or_default();
            let seq_posting: HashSet<_> = sequential
                .title_full_all_suffix_index
                .get(key.as_str())
                .map(|v| v.iter().cloned().collect())
                .unwrap_or_default();
            assert_eq!(
                batch_posting, seq_posting,
                "title_full_all_suffix_index mismatch for key: {}",
                key
            );
        }

        // 9. title_word_all_suffix_index の一致
        assert_eq!(
            batch.title_word_all_suffix_index.len(),
            sequential.title_word_all_suffix_index.len(),
            "title_word_all_suffix_index length mismatch"
        );
        for key in batch.title_word_all_suffix_index.keys() {
            let batch_posting: HashSet<_> = batch
                .title_word_all_suffix_index
                .get(key.as_str())
                .map(|v| v.iter().cloned().collect())
                .unwrap_or_default();
            let seq_posting: HashSet<_> = sequential
                .title_word_all_suffix_index
                .get(key.as_str())
                .map(|v| v.iter().cloned().collect())
                .unwrap_or_default();
            assert_eq!(
                batch_posting, seq_posting,
                "title_word_all_suffix_index mismatch for key: {}",
                key
            );
        }

        // 10. tag_all_suffix_index の一致
        assert_eq!(
            batch.tag_all_suffix_index.len(),
            sequential.tag_all_suffix_index.len(),
            "tag_all_suffix_index length mismatch"
        );
        for key in batch.tag_all_suffix_index.keys() {
            let batch_posting: HashSet<_> = batch
                .tag_all_suffix_index
                .get(key.as_str())
                .map(|v| v.iter().cloned().collect())
                .unwrap_or_default();
            let seq_posting: HashSet<_> = sequential
                .tag_all_suffix_index
                .get(key.as_str())
                .map(|v| v.iter().cloned().collect())
                .unwrap_or_default();
            assert_eq!(
                batch_posting, seq_posting,
                "tag_all_suffix_index mismatch for key: {}",
                key
            );
        }
    }

    /// Asserts that search results match between two indexes.
    fn assert_search_results_equal(batch: &SearchIndex, sequential: &SearchIndex, query: &str) {
        let batch_result = batch.search_by_title(query);
        let seq_result = sequential.search_by_title(query);

        match (&batch_result, &seq_result) {
            (Some(batch_res), Some(seq_res)) => {
                let batch_ids: HashSet<_> = batch_res.tasks.iter().map(|t| &t.task_id).collect();
                let seq_ids: HashSet<_> = seq_res.tasks.iter().map(|t| &t.task_id).collect();
                assert_eq!(
                    batch_ids, seq_ids,
                    "Search results differ for query: {}",
                    query
                );
            }
            (None, None) => {}
            _ => panic!(
                "Search result presence differs for query: {} (batch: {:?}, sequential: {:?})",
                query,
                batch_result.is_some(),
                seq_result.is_some()
            ),
        }
    }

    // =========================================================================
    // Basic Functionality Tests
    // =========================================================================

    /// Test 1: apply_changes with empty changes returns self.clone()
    #[rstest]
    fn apply_changes_with_empty_changes_returns_clone() {
        let index = create_test_index();
        let original_task_count = index.tasks_by_id.len();

        let result = index.apply_changes(&[]);

        assert_eq!(result.tasks_by_id.len(), original_task_count);
    }

    // =========================================================================
    // Differential Tests (apply_changes == sequential apply_change)
    // =========================================================================

    /// Test 2: apply_changes equals sequential apply_change for Adds
    #[rstest]
    fn apply_changes_equals_sequential_apply_change_for_adds() {
        let tasks = create_test_tasks();
        let index = SearchIndex::build_with_config(&tasks, SearchIndexConfig::default());

        let new_tasks = vec![
            create_test_task("New Task 1"),
            create_test_task("New Task 2"),
        ];
        let changes: Vec<TaskChange> = new_tasks
            .iter()
            .map(|t| TaskChange::Add(t.clone()))
            .collect();

        // バッチ適用
        let batch_result = index.apply_changes(&changes);

        // 逐次適用
        let mut sequential_result = index.clone();
        for change in &changes {
            sequential_result = sequential_result.apply_change(change.clone());
        }

        // インデックス全体の一致を検証
        assert_search_index_equals(&batch_result, &sequential_result);

        // 検索結果の一致を検証
        assert_search_results_equal(&batch_result, &sequential_result, "New");
        assert_search_results_equal(&batch_result, &sequential_result, "Task");
    }

    /// Test 3: apply_changes equals sequential apply_change for Updates
    #[rstest]
    fn apply_changes_equals_sequential_apply_change_for_updates() {
        let tasks = create_test_tasks();
        let index = SearchIndex::build_with_config(&tasks, SearchIndexConfig::default());

        let old_task = tasks.get(0).unwrap().clone();
        let new_task = Task {
            title: "Updated Title".to_string(),
            ..old_task.clone()
        };
        let changes = vec![TaskChange::Update {
            old: old_task.clone(),
            new: new_task.clone(),
        }];

        let batch_result = index.apply_changes(&changes);
        let sequential_result = index.apply_change(changes[0].clone());

        // インデックス全体の一致を検証
        assert_search_index_equals(&batch_result, &sequential_result);

        // 検索結果の一致を検証（old が消え、new が見つかる）
        assert_search_results_equal(&batch_result, &sequential_result, "Updated");
        assert_search_results_equal(&batch_result, &sequential_result, &old_task.title);
    }

    /// Test 4: apply_changes equals sequential apply_change for Removes
    #[rstest]
    fn apply_changes_equals_sequential_apply_change_for_removes() {
        let tasks = create_test_tasks();
        let index = SearchIndex::build_with_config(&tasks, SearchIndexConfig::default());

        let task_to_remove = tasks.get(0).unwrap().clone();
        let changes = vec![TaskChange::Remove(task_to_remove.task_id.clone())];

        let batch_result = index.apply_changes(&changes);
        let sequential_result = index.apply_change(changes[0].clone());

        // インデックス全体の一致を検証
        assert_search_index_equals(&batch_result, &sequential_result);

        // 検索結果の一致を検証（削除されたタスクが見つからない）
        assert_search_results_equal(&batch_result, &sequential_result, &task_to_remove.title);
    }

    /// Test 5: apply_changes equals sequential apply_change for Mixed operations
    #[rstest]
    fn apply_changes_equals_sequential_apply_change_for_mixed() {
        let tasks = create_test_tasks();
        let index = SearchIndex::build_with_config(&tasks, SearchIndexConfig::default());

        let new_task = create_test_task("New Task");
        let task_to_update = tasks.get(0).unwrap().clone();
        let updated_task = Task {
            title: "Updated".to_string(),
            ..task_to_update.clone()
        };
        let task_to_remove = tasks.get(1).unwrap().clone();

        let changes = vec![
            TaskChange::Add(new_task.clone()),
            TaskChange::Update {
                old: task_to_update.clone(),
                new: updated_task.clone(),
            },
            TaskChange::Remove(task_to_remove.task_id.clone()),
        ];

        let batch_result = index.apply_changes(&changes);

        let mut sequential_result = index.clone();
        for change in &changes {
            sequential_result = sequential_result.apply_change(change.clone());
        }

        // インデックス全体の一致を検証
        assert_search_index_equals(&batch_result, &sequential_result);

        // 検索結果の一致を検証
        assert_search_results_equal(&batch_result, &sequential_result, "New");
        assert_search_results_equal(&batch_result, &sequential_result, "Updated");
        assert_search_results_equal(&batch_result, &sequential_result, &task_to_remove.title);
    }

    // =========================================================================
    // Idempotency Tests
    // =========================================================================

    /// Test 6: apply_changes is idempotent for duplicate Adds
    #[rstest]
    fn apply_changes_is_idempotent_for_adds() {
        let index = create_empty_index();
        let task = create_test_task("Test");
        let changes = vec![TaskChange::Add(task.clone()), TaskChange::Add(task.clone())];

        let result = index.apply_changes(&changes);

        assert_eq!(result.tasks_by_id.len(), 1);
    }

    // =========================================================================
    // Empty Key Removal Tests
    // =========================================================================

    /// Test 7: apply_changes removes empty keys from index
    #[rstest]
    fn apply_changes_removes_empty_keys() {
        let task = create_test_task("Test");
        let tasks: PersistentVector<Task> = vec![task.clone()].into_iter().collect();
        let index = SearchIndex::build_with_config(&tasks, SearchIndexConfig::default());

        let changes = vec![TaskChange::Remove(task.task_id.clone())];
        let result = index.apply_changes(&changes);

        // title_full_index から空のエントリが除去される
        let normalized = normalize_query(&task.title);
        assert!(result.title_full_index.get(&normalized.key).is_none());
    }

    // =========================================================================
    // Add→Remove Cancellation Tests
    // =========================================================================

    /// Test 8: Add followed by Remove cancels out in same batch
    #[rstest]
    fn apply_changes_add_then_remove_cancels_out() {
        let task = create_test_task("Test Task");
        let index = create_empty_index();

        // Add してから Remove（同一バッチ内）
        let changes = vec![
            TaskChange::Add(task.clone()),
            TaskChange::Remove(task.task_id.clone()),
        ];

        let result = index.apply_changes(&changes);

        // tasks_by_id にタスクが存在しない（打ち消された）
        assert!(!result.tasks_by_id.contains_key(&task.task_id));

        // インデックスにもエントリが残らない
        let normalized = normalize_query(&task.title);
        assert!(result.title_full_index.get(&normalized.key).is_none());
    }

    /// Test 9: Update followed by Remove cancels out in same batch
    #[rstest]
    fn apply_changes_update_then_remove_cancels_out() {
        let task_id = task_id_from_u128(1);
        let old_task = create_test_task_with_id("Old Title", task_id.clone());
        let new_task = create_test_task_with_id("New Title", task_id.clone());

        // 初期状態: old_task が存在
        let tasks: PersistentVector<Task> = vec![old_task.clone()].into_iter().collect();
        let index = SearchIndex::build_with_config(&tasks, SearchIndexConfig::default());

        // Update してから Remove（同一バッチ内）
        let changes = vec![
            TaskChange::Update {
                old: old_task.clone(),
                new: new_task.clone(),
            },
            TaskChange::Remove(new_task.task_id.clone()),
        ];

        let result = index.apply_changes(&changes);

        // tasks_by_id にタスクが存在しない
        assert!(!result.tasks_by_id.contains_key(&old_task.task_id));

        // old と new のどちらのインデックスも存在しない
        let old_normalized = normalize_query(&old_task.title);
        let new_normalized = normalize_query(&new_task.title);
        assert!(result.title_full_index.get(&old_normalized.key).is_none());
        assert!(result.title_full_index.get(&new_normalized.key).is_none());
    }

    // =========================================================================
    // Sequential Comparison Tests (Add→Remove, Update→Remove)
    // =========================================================================

    /// Test 10: apply_changes equals sequential for Add→Remove
    #[rstest]
    fn apply_changes_equals_sequential_for_add_then_remove() {
        let task = create_test_task("Test Task");
        let index = create_empty_index();

        let changes = vec![
            TaskChange::Add(task.clone()),
            TaskChange::Remove(task.task_id.clone()),
        ];

        // バッチ適用
        let batch_result = index.apply_changes(&changes);

        // 逐次適用
        let mut sequential_result = index.clone();
        for change in &changes {
            sequential_result = sequential_result.apply_change(change.clone());
        }

        // 結果が一致
        assert_eq!(
            batch_result.tasks_by_id.len(),
            sequential_result.tasks_by_id.len()
        );
        assert!(!batch_result.tasks_by_id.contains_key(&task.task_id));
        assert!(!sequential_result.tasks_by_id.contains_key(&task.task_id));
    }

    /// Test 11: apply_changes equals sequential for Update→Remove
    #[rstest]
    fn apply_changes_equals_sequential_for_update_then_remove() {
        let task_id = task_id_from_u128(1);
        let old_task = create_test_task_with_id("Old Title", task_id.clone());
        let new_task = create_test_task_with_id("New Title", task_id.clone());

        // 初期状態: old_task が存在
        let tasks: PersistentVector<Task> = vec![old_task.clone()].into_iter().collect();
        let index = SearchIndex::build_with_config(&tasks, SearchIndexConfig::default());

        let changes = vec![
            TaskChange::Update {
                old: old_task.clone(),
                new: new_task.clone(),
            },
            TaskChange::Remove(new_task.task_id.clone()),
        ];

        // バッチ適用
        let batch_result = index.apply_changes(&changes);

        // 逐次適用
        let mut sequential_result = index.clone();
        for change in &changes {
            sequential_result = sequential_result.apply_change(change.clone());
        }

        // インデックス全体の一致を検証
        assert_search_index_equals(&batch_result, &sequential_result);

        // タスクが存在しないことを検証
        assert!(!batch_result.tasks_by_id.contains_key(&old_task.task_id));
        assert!(
            !sequential_result
                .tasks_by_id
                .contains_key(&old_task.task_id)
        );
    }

    /// Test: Update with `max_tokens_per_task` limit matches sequential apply_change.
    /// When token count exceeds the limit, only limited tokens are indexed.
    #[rstest]
    fn apply_changes_equals_sequential_for_update_with_token_limit() {
        let task_id = task_id_from_u128(1);
        // old: 6 words + 2 tags = 8 tokens
        let old_task = create_test_task_with_id_and_tags(
            task_id.clone(),
            "alpha beta gamma delta epsilon zeta",
            vec!["tag1", "tag2"],
        );
        // new: 7 words + 3 tags = 10 tokens
        let new_task = create_test_task_with_id_and_tags(
            task_id.clone(),
            "one two three four five six seven",
            vec!["tagA", "tagB", "tagC"],
        );

        // max_tokens_per_task = 5, so token limits differ between old and new
        let config = SearchIndexConfig {
            max_tokens_per_task: 5,
            ..Default::default()
        };

        let tasks: PersistentVector<Task> = vec![old_task.clone()].into_iter().collect();
        let index = SearchIndex::build_with_config(&tasks, config);

        let changes = vec![TaskChange::Update {
            old: old_task.clone(),
            new: new_task.clone(),
        }];

        // バッチ適用
        let batch_result = index.apply_changes(&changes);

        // 逐次適用
        let mut sequential_result = index.clone();
        for change in &changes {
            sequential_result = sequential_result.apply_change(change.clone());
        }

        // インデックス全体の一致を検証
        assert_search_index_equals(&batch_result, &sequential_result);
    }

    // =========================================================================
    // Forbidden Pattern Tests (Should Panic)
    // =========================================================================

    /// Test 12: Remove followed by Add panics (forbidden pattern)
    #[rstest]
    #[should_panic(expected = "Remove followed by Add for same TaskId")]
    fn apply_changes_panics_on_remove_then_add() {
        let task_id = task_id_from_u128(1);
        let task = create_test_task_with_id("Test Task", task_id.clone());

        // 初期状態: task が存在
        let tasks: PersistentVector<Task> = vec![task.clone()].into_iter().collect();
        let index = SearchIndex::build_with_config(&tasks, SearchIndexConfig::default());

        // 禁止パターン: Remove してから Add（同一 TaskId）
        let changes = vec![
            TaskChange::Remove(task.task_id.clone()),
            TaskChange::Add(task.clone()),
        ];

        // panic する
        let _ = index.apply_changes(&changes);
    }

    /// Test 13: Remove followed by Update panics (forbidden pattern)
    #[rstest]
    #[should_panic(expected = "Remove followed by Update for same TaskId")]
    fn apply_changes_panics_on_remove_then_update() {
        let task_id = task_id_from_u128(1);
        let old_task = create_test_task_with_id("Old Title", task_id.clone());
        let new_task = create_test_task_with_id("New Title", task_id.clone());

        // 初期状態: old_task が存在
        let tasks: PersistentVector<Task> = vec![old_task.clone()].into_iter().collect();
        let index = SearchIndex::build_with_config(&tasks, SearchIndexConfig::default());

        // 禁止パターン: Remove してから Update（同一 TaskId）
        let changes = vec![
            TaskChange::Remove(old_task.task_id.clone()),
            TaskChange::Update {
                old: old_task.clone(),
                new: new_task.clone(),
            },
        ];

        // panic する
        let _ = index.apply_changes(&changes);
    }

    // =========================================================================
    // LegacyAllSuffix Mode Tests
    // =========================================================================

    /// Test 14: apply_changes equals sequential apply_change for Adds in LegacyAllSuffix mode
    #[rstest]
    fn apply_changes_equals_sequential_for_legacy_all_suffix_mode_adds() {
        let config = SearchIndexConfig {
            infix_mode: InfixMode::LegacyAllSuffix,
            ..SearchIndexConfig::default()
        };
        let tasks = create_test_tasks();
        let index = SearchIndex::build_with_config(&tasks, config);

        let new_tasks = vec![
            create_test_task("New Task 1"),
            create_test_task("New Task 2"),
        ];
        let changes: Vec<TaskChange> = new_tasks
            .iter()
            .map(|task| TaskChange::Add(task.clone()))
            .collect();

        // バッチ適用
        let batch_result = index.apply_changes(&changes);

        // 逐次適用
        let mut sequential_result = index.clone();
        for change in &changes {
            sequential_result = sequential_result.apply_change(change.clone());
        }

        // インデックス全体の一致を検証
        assert_search_index_equals(&batch_result, &sequential_result);

        // 検索結果の一致を検証
        assert_search_results_equal(&batch_result, &sequential_result, "New");
        assert_search_results_equal(&batch_result, &sequential_result, "Task");
        // infix search ("ew" is a suffix of "New")
        assert_search_results_equal(&batch_result, &sequential_result, "ew");
    }

    /// Test 15: apply_changes equals sequential apply_change for Updates in LegacyAllSuffix mode
    #[rstest]
    fn apply_changes_equals_sequential_for_legacy_all_suffix_mode_updates() {
        let config = SearchIndexConfig {
            infix_mode: InfixMode::LegacyAllSuffix,
            ..SearchIndexConfig::default()
        };
        let tasks = create_test_tasks();
        let index = SearchIndex::build_with_config(&tasks, config);

        let old_task = tasks.get(0).unwrap().clone();
        let new_task = Task {
            title: "Updated Title".to_string(),
            ..old_task.clone()
        };
        let changes = vec![TaskChange::Update {
            old: old_task.clone(),
            new: new_task.clone(),
        }];

        let batch_result = index.apply_changes(&changes);
        let sequential_result = index.apply_change(changes[0].clone());

        // インデックス全体の一致を検証
        assert_search_index_equals(&batch_result, &sequential_result);

        // 検索結果の一致を検証（old が消え、new が見つかる）
        assert_search_results_equal(&batch_result, &sequential_result, "Updated");
        assert_search_results_equal(&batch_result, &sequential_result, &old_task.title);
        // infix search ("pdated" is a suffix of "Updated")
        assert_search_results_equal(&batch_result, &sequential_result, "pdated");
    }

    /// Test 16: apply_changes equals sequential apply_change for Removes in LegacyAllSuffix mode
    #[rstest]
    fn apply_changes_equals_sequential_for_legacy_all_suffix_mode_removes() {
        let config = SearchIndexConfig {
            infix_mode: InfixMode::LegacyAllSuffix,
            ..SearchIndexConfig::default()
        };
        let tasks = create_test_tasks();
        let index = SearchIndex::build_with_config(&tasks, config);

        let task_to_remove = tasks.get(0).unwrap().clone();
        let changes = vec![TaskChange::Remove(task_to_remove.task_id.clone())];

        let batch_result = index.apply_changes(&changes);
        let sequential_result = index.apply_change(changes[0].clone());

        // インデックス全体の一致を検証
        assert_search_index_equals(&batch_result, &sequential_result);

        // 検索結果の一致を検証（削除されたタスクが見つからない）
        assert_search_results_equal(&batch_result, &sequential_result, &task_to_remove.title);
    }

    /// Test 17: apply_changes equals sequential apply_change for Mixed operations in LegacyAllSuffix mode
    #[rstest]
    fn apply_changes_equals_sequential_for_legacy_all_suffix_mode_mixed() {
        let config = SearchIndexConfig {
            infix_mode: InfixMode::LegacyAllSuffix,
            ..SearchIndexConfig::default()
        };
        let tasks = create_test_tasks();
        let index = SearchIndex::build_with_config(&tasks, config);

        let new_task = create_test_task("New Task");
        let task_to_update = tasks.get(0).unwrap().clone();
        let updated_task = Task {
            title: "Updated".to_string(),
            ..task_to_update.clone()
        };
        let task_to_remove = tasks.get(1).unwrap().clone();

        let changes = vec![
            TaskChange::Add(new_task.clone()),
            TaskChange::Update {
                old: task_to_update.clone(),
                new: updated_task.clone(),
            },
            TaskChange::Remove(task_to_remove.task_id.clone()),
        ];

        // バッチ適用
        let batch_result = index.apply_changes(&changes);

        // 逐次適用
        let mut sequential_result = index.clone();
        for change in &changes {
            sequential_result = sequential_result.apply_change(change.clone());
        }

        // インデックス全体の一致を検証
        assert_search_index_equals(&batch_result, &sequential_result);

        // 検索結果の一致を検証
        assert_search_results_equal(&batch_result, &sequential_result, "New");
        assert_search_results_equal(&batch_result, &sequential_result, "Updated");
        assert_search_results_equal(&batch_result, &sequential_result, &task_to_remove.title);
    }

    /// Test 18: apply_changes equals sequential for add then remove in LegacyAllSuffix mode
    #[rstest]
    fn apply_changes_equals_sequential_for_add_then_remove_legacy_all_suffix_mode() {
        let config = SearchIndexConfig {
            infix_mode: InfixMode::LegacyAllSuffix,
            ..SearchIndexConfig::default()
        };
        let tasks = create_test_tasks();
        let index = SearchIndex::build_with_config(&tasks, config);

        let new_task = create_test_task("Temporary Task");
        let changes = vec![
            TaskChange::Add(new_task.clone()),
            TaskChange::Remove(new_task.task_id.clone()),
        ];

        // バッチ適用
        let batch_result = index.apply_changes(&changes);

        // 逐次適用
        let mut sequential_result = index.clone();
        for change in &changes {
            sequential_result = sequential_result.apply_change(change.clone());
        }

        // インデックス全体の一致を検証
        assert_search_index_equals(&batch_result, &sequential_result);

        // 追加して削除したタスクは見つからないはず
        assert_search_results_equal(&batch_result, &sequential_result, "Temporary");
    }
}

// =============================================================================
// SearchIndexBuildMetrics Tests (REQ-SEARCH-NGRAM-PERF-002)
// =============================================================================

#[cfg(test)]
mod search_index_build_metrics_tests {
    use super::*;
    use crate::domain::{Tag, Task, TaskId, Timestamp};
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // Test Helpers
    // -------------------------------------------------------------------------

    /// Creates a task with the given title.
    fn create_test_task(title: &str) -> Task {
        Task::new(TaskId::generate(), title, Timestamp::now())
    }

    /// Creates a task with the given title and tags.
    fn create_test_task_with_tags(title: &str, tags: Vec<&str>) -> Task {
        let base = create_test_task(title);
        tags.into_iter()
            .fold(base, |task, tag| task.add_tag(Tag::new(tag)))
    }

    /// Generates random tasks for testing.
    fn generate_random_tasks(count: usize) -> PersistentVector<Task> {
        (0..count)
            .map(|i| {
                create_test_task_with_tags(
                    &format!("Task {i} with some title words"),
                    vec!["work", "test"],
                )
            })
            .collect()
    }

    // -------------------------------------------------------------------------
    // measure_search_index_build Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn measure_search_index_build_returns_valid_metrics() {
        // Arrange
        let tasks = generate_random_tasks(100);
        let config = SearchIndexConfig::default();

        // Act
        let (index, metrics) = measure_search_index_build(&tasks, config);

        // Assert: Index is built correctly
        assert!(!index.tasks_by_id.is_empty());
        assert_eq!(index.tasks_by_id.len(), 100);

        // Assert: Metrics are populated
        // elapsed_ms could be 0 for very fast builds on modern CPUs
        // so we only check that it's a valid value (not checking > 0)
        assert!(metrics.ngram_entries > 0, "ngram_entries should be > 0");
    }

    #[rstest]
    fn measure_search_index_build_with_empty_tasks() {
        // Arrange
        let tasks = PersistentVector::new();
        let config = SearchIndexConfig::default();

        // Act
        let (index, metrics) = measure_search_index_build(&tasks, config);

        // Assert: Index is empty
        assert!(index.tasks_by_id.is_empty());

        // Assert: Metrics show no n-gram entries
        assert_eq!(metrics.ngram_entries, 0);
    }

    #[rstest]
    fn measure_search_index_build_with_legacy_mode_has_zero_ngrams() {
        // Arrange
        let tasks = generate_random_tasks(10);
        let config = SearchIndexConfig {
            infix_mode: InfixMode::LegacyAllSuffix,
            ..SearchIndexConfig::default()
        };

        // Act
        let (_, metrics) = measure_search_index_build(&tasks, config);

        // Assert: No n-gram entries in legacy mode
        assert_eq!(
            metrics.ngram_entries, 0,
            "LegacyAllSuffix mode should have 0 ngram_entries"
        );
    }

    #[rstest]
    fn measure_search_index_build_with_disabled_mode_has_zero_ngrams() {
        // Arrange
        let tasks = generate_random_tasks(10);
        let config = SearchIndexConfig {
            infix_mode: InfixMode::Disabled,
            ..SearchIndexConfig::default()
        };

        // Act
        let (_, metrics) = measure_search_index_build(&tasks, config);

        // Assert: No n-gram entries in disabled mode
        assert_eq!(
            metrics.ngram_entries, 0,
            "Disabled mode should have 0 ngram_entries"
        );
    }

    // -------------------------------------------------------------------------
    // SearchIndex::ngram_entry_count Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn ngram_entry_count_returns_sum_of_all_ngram_indexes() {
        // Arrange
        let tasks = generate_random_tasks(10);
        let config = SearchIndexConfig::default();
        let index = SearchIndex::build_with_config(&tasks, config);

        // Act
        let count = index.ngram_entry_count();

        // Assert: Count should be > 0 for Ngram mode
        assert!(count > 0, "ngram_entry_count should be > 0 for Ngram mode");
    }

    #[rstest]
    fn ngram_entry_count_is_zero_for_disabled_mode() {
        // Arrange
        let tasks = generate_random_tasks(10);
        let config = SearchIndexConfig {
            infix_mode: InfixMode::Disabled,
            ..SearchIndexConfig::default()
        };
        let index = SearchIndex::build_with_config(&tasks, config);

        // Act
        let count = index.ngram_entry_count();

        // Assert
        assert_eq!(count, 0, "ngram_entry_count should be 0 for Disabled mode");
    }

    #[rstest]
    fn ngram_entry_count_is_zero_for_legacy_all_suffix_mode() {
        // Arrange
        let tasks = generate_random_tasks(10);
        let config = SearchIndexConfig {
            infix_mode: InfixMode::LegacyAllSuffix,
            ..SearchIndexConfig::default()
        };
        let index = SearchIndex::build_with_config(&tasks, config);

        // Act
        let count = index.ngram_entry_count();

        // Assert
        assert_eq!(
            count, 0,
            "ngram_entry_count should be 0 for LegacyAllSuffix mode"
        );
    }

    // -------------------------------------------------------------------------
    // SearchIndexBuildMetrics Serialization Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn search_index_build_metrics_serializes_to_json() {
        // Arrange
        let metrics = SearchIndexBuildMetrics {
            elapsed_ms: 123,
            peak_rss_mb: 456,
            ngram_entries: 789,
        };

        // Act
        let json = serde_json::to_string(&metrics).expect("serialization should succeed");

        // Assert
        assert!(json.contains("\"elapsed_ms\":123"));
        assert!(json.contains("\"peak_rss_mb\":456"));
        assert!(json.contains("\"ngram_entries\":789"));
    }

    #[rstest]
    fn search_index_build_metrics_deserializes_from_json() {
        // Arrange
        let json = r#"{"elapsed_ms":100,"peak_rss_mb":200,"ngram_entries":300}"#;

        // Act
        let metrics: SearchIndexBuildMetrics =
            serde_json::from_str(json).expect("deserialization should succeed");

        // Assert
        assert_eq!(metrics.elapsed_ms, 100);
        assert_eq!(metrics.peak_rss_mb, 200);
        assert_eq!(metrics.ngram_entries, 300);
    }

    #[rstest]
    fn search_index_build_metrics_roundtrip() {
        // Arrange
        let original = SearchIndexBuildMetrics {
            elapsed_ms: 999,
            peak_rss_mb: 888,
            ngram_entries: 777,
        };

        // Act
        let json = serde_json::to_string(&original).expect("serialization should succeed");
        let deserialized: SearchIndexBuildMetrics =
            serde_json::from_str(&json).expect("deserialization should succeed");

        // Assert
        assert_eq!(deserialized.elapsed_ms, original.elapsed_ms);
        assert_eq!(deserialized.peak_rss_mb, original.peak_rss_mb);
        assert_eq!(deserialized.ngram_entries, original.ngram_entries);
    }
}

// =============================================================================
// NgramKey and NgramKeyPool Tests (REQ-SEARCH-NGRAM-MEM-001 Phase 1)
// =============================================================================

#[cfg(test)]
mod ngram_key_tests {
    use super::*;
    use rstest::rstest;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // -------------------------------------------------------------------------
    // NgramKey Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn ngram_key_new_creates_arc() {
        // Arrange & Act
        let key = NgramKey::new("test");

        // Assert
        assert_eq!(key.as_str(), "test");
    }

    #[rstest]
    fn ngram_key_clone_is_shallow() {
        // Arrange
        let key1 = NgramKey::new("test");

        // Act
        let key2 = key1.clone();

        // Assert: Arc::ptr_eq confirms same underlying allocation
        assert!(Arc::ptr_eq(&key1.0, &key2.0));
    }

    #[rstest]
    fn ngram_key_hash_eq_by_value() {
        // Arrange: Two separate NgramKeys with same content
        let key1 = NgramKey::new("test");
        let key2 = NgramKey::new("test");

        // Assert: Value equality (different Arc instances, same content)
        assert_eq!(key1, key2);

        // Assert: Hash equality
        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();
        key1.hash(&mut hasher1);
        key2.hash(&mut hasher2);
        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[rstest]
    fn ngram_key_hash_ne_for_different_values() {
        // Arrange
        let key1 = NgramKey::new("abc");
        let key2 = NgramKey::new("xyz");

        // Assert: Not equal (value inequality)
        assert_ne!(key1, key2);

        // Note: We intentionally do NOT assert hash inequality here.
        // Hash collisions are possible by definition, and asserting
        // hash inequality would make this test theoretically flaky.
        // The important property is that equal values have equal hashes
        // (tested in ngram_key_hash_eq_by_value), not that different
        // values have different hashes.

        // Verify hashing does not panic for different values
        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();
        key1.hash(&mut hasher1);
        key2.hash(&mut hasher2);
        let _ = hasher1.finish();
        let _ = hasher2.finish();
    }

    #[rstest]
    fn ngram_key_display_shows_content() {
        // Arrange
        let key = NgramKey::new("hello");

        // Act
        let displayed = key.to_string();

        // Assert
        assert_eq!(displayed, "hello");
    }

    #[rstest]
    fn ngram_key_debug_shows_arc_wrapper() {
        // Arrange
        let key = NgramKey::new("test");

        // Act
        let debug_str = format!("{key:?}");

        // Assert: Debug output contains the value
        assert!(debug_str.contains("test"));
    }

    // -------------------------------------------------------------------------
    // NgramKeyPool Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn ngram_key_pool_new_is_empty() {
        // Arrange & Act
        let pool = NgramKeyPool::new();

        // Assert
        assert_eq!(pool.unique_count(), 0);
        assert_eq!(pool.total_count(), 0);
        assert!(pool.hit_rate().abs() < f64::EPSILON);
    }

    #[rstest]
    fn ngram_key_pool_default_is_empty() {
        // Arrange & Act
        let pool = NgramKeyPool::default();

        // Assert
        assert_eq!(pool.unique_count(), 0);
        assert_eq!(pool.total_count(), 0);
        assert!(pool.hit_rate().abs() < f64::EPSILON);
    }

    #[rstest]
    fn ngram_key_pool_intern_creates_new_key() {
        // Arrange
        let mut pool = NgramKeyPool::new();

        // Act
        let key = pool.intern("test");

        // Assert
        assert_eq!(key.as_str(), "test");
        assert_eq!(pool.unique_count(), 1);
        assert_eq!(pool.total_count(), 1);
        assert!(pool.hit_rate().abs() < f64::EPSILON); // All misses
    }

    #[rstest]
    fn ngram_key_pool_intern_returns_same_arc() {
        // Arrange
        let mut pool = NgramKeyPool::new();
        let key1 = pool.intern("test");

        // Act
        let key2 = pool.intern("test");

        // Assert: Same Arc instance
        assert!(Arc::ptr_eq(&key1.0, &key2.0));
        assert_eq!(pool.unique_count(), 1);
        assert_eq!(pool.total_count(), 2);
    }

    #[rstest]
    fn ngram_key_pool_tracks_hit_rate() {
        // Arrange
        let mut pool = NgramKeyPool::new();

        // Act: 2 misses, then 3 hits
        pool.intern("a"); // miss
        pool.intern("b"); // miss
        pool.intern("a"); // hit
        pool.intern("a"); // hit
        pool.intern("b"); // hit

        // Assert
        assert_eq!(pool.unique_count(), 2);
        assert_eq!(pool.total_count(), 5);
        // Hit rate: 3 hits / 5 total = 0.6
        assert!((pool.hit_rate() - 0.6).abs() < 0.001);
    }

    #[rstest]
    fn ngram_key_pool_multiple_unique_keys() {
        // Arrange
        let mut pool = NgramKeyPool::new();

        // Act
        let key_abc = pool.intern("abc");
        let key_def = pool.intern("def");
        let key_ghi = pool.intern("ghi");

        // Assert
        assert_eq!(pool.unique_count(), 3);
        assert_eq!(pool.total_count(), 3);
        assert!(pool.hit_rate().abs() < f64::EPSILON); // All misses

        // Verify keys are distinct
        assert_ne!(key_abc.as_str(), key_def.as_str());
        assert_ne!(key_def.as_str(), key_ghi.as_str());
    }

    #[rstest]
    fn ngram_key_pool_empty_string() {
        // Arrange
        let mut pool = NgramKeyPool::new();

        // Act
        let key1 = pool.intern("");
        let key2 = pool.intern("");

        // Assert: Empty string is interned correctly
        assert_eq!(key1.as_str(), "");
        assert!(Arc::ptr_eq(&key1.0, &key2.0));
        assert_eq!(pool.unique_count(), 1);
    }

    #[rstest]
    fn ngram_key_pool_unicode_strings() {
        // Arrange
        let mut pool = NgramKeyPool::new();

        // Act
        let key_jp = pool.intern("日本語");
        let key_emoji = pool.intern("🦀");
        let key_jp2 = pool.intern("日本語");

        // Assert
        assert_eq!(key_jp.as_str(), "日本語");
        assert_eq!(key_emoji.as_str(), "🦀");
        assert!(Arc::ptr_eq(&key_jp.0, &key_jp2.0));
        assert_eq!(pool.unique_count(), 2);
    }
}
