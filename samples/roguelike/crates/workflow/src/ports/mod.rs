//! Abstract port definitions for the workflow layer.
//!
//! This module defines the abstract interfaces (ports) that the workflow layer
//! depends on. Concrete implementations (adapters) are provided by the
//! infrastructure layer.
//!
//! # Port Categories
//!
//! - **Repositories**: Persistent storage for aggregates
//! - **Event Stores**: Event sourcing storage
//! - **Caches**: Transient storage for performance optimization
//! - **External Services**: Third-party integrations (e.g., random generation)
//!
//! # Design Principles
//!
//! All ports return `AsyncIO<T>` to defer side effects until the workflow's edge.
//! This maintains referential transparency within the pure workflow logic.
//!
//! # Examples
//!
//! ```ignore
//! use roguelike_workflow::ports::GameSessionRepository;
//! use lambars::effect::AsyncIO;
//!
//! // Workflows receive ports as dependencies
//! fn create_game_workflow<R: GameSessionRepository>(
//!     repository: R,
//! ) -> impl Fn(CreateGameCommand) -> AsyncIO<Result<GameSession, WorkflowError>> {
//!     move |command| {
//!         // Implementation uses repository.save(), etc.
//!         // ...
//!     }
//! }
//! ```

use std::time::Duration;

use lambars::effect::AsyncIO;
use roguelike_domain::game_session::{GameIdentifier, GameSessionEvent, RandomSeed};

// =============================================================================
// Type Aliases for Workflow Results
// =============================================================================

/// Result type for workflow operations.
pub type WorkflowResult<T> = Result<T, crate::errors::WorkflowError>;

// =============================================================================
// GameSessionRepository
// =============================================================================

/// Repository port for game session persistence.
///
/// This trait defines the interface for storing and retrieving game sessions.
/// Implementations should handle the actual I/O operations (database, file system, etc.).
///
/// # Type Requirements
///
/// All trait bounds are required for thread-safe async execution:
/// - `Clone`: Allows sharing the repository across async tasks
/// - `Send + Sync`: Required for cross-thread access
/// - `'static`: Required for async lifetime bounds
///
/// # Examples
///
/// ```ignore
/// use roguelike_workflow::ports::GameSessionRepository;
/// use lambars::effect::AsyncIO;
///
/// struct InMemoryGameSessionRepository {
///     // ... implementation details
/// }
///
/// impl GameSessionRepository for InMemoryGameSessionRepository {
///     type GameSession = GameSession;
///
///     fn find_by_id(&self, id: &GameIdentifier) -> AsyncIO<Option<Self::GameSession>> {
///         // ... implementation
///     }
///     // ... other methods
/// }
/// ```
pub trait GameSessionRepository: Clone + Send + Sync + 'static {
    /// The game session type returned by this repository.
    ///
    /// This associated type allows different repository implementations
    /// to work with different game session representations.
    type GameSession: Clone + Send + Sync + 'static;

    /// Finds a game session by its identifier.
    ///
    /// # Arguments
    ///
    /// * `identifier` - The unique identifier of the game session.
    ///
    /// # Returns
    ///
    /// An `AsyncIO` that resolves to `Some(GameSession)` if found, `None` otherwise.
    fn find_by_id(&self, identifier: &GameIdentifier) -> AsyncIO<Option<Self::GameSession>>;

    /// Saves a game session.
    ///
    /// If a session with the same identifier exists, it will be updated.
    /// Otherwise, a new session will be created.
    ///
    /// # Arguments
    ///
    /// * `session` - The game session to save.
    ///
    /// # Returns
    ///
    /// An `AsyncIO` that resolves to `()` on success.
    fn save(&self, session: &Self::GameSession) -> AsyncIO<()>;

    /// Deletes a game session by its identifier.
    ///
    /// # Arguments
    ///
    /// * `identifier` - The unique identifier of the game session to delete.
    ///
    /// # Returns
    ///
    /// An `AsyncIO` that resolves to `()` on success.
    /// No error is raised if the session does not exist.
    fn delete(&self, identifier: &GameIdentifier) -> AsyncIO<()>;

    /// Lists all active game session identifiers.
    ///
    /// Active sessions are those that are currently in progress
    /// (not completed, abandoned, or expired).
    ///
    /// # Returns
    ///
    /// An `AsyncIO` that resolves to a vector of game identifiers.
    fn list_active(&self) -> AsyncIO<Vec<GameIdentifier>>;
}

// =============================================================================
// EventStore
// =============================================================================

/// Event store port for event sourcing.
///
/// This trait defines the interface for storing and retrieving domain events.
/// Events are immutable and append-only, providing a complete audit trail
/// of all state changes.
///
/// # Event Sourcing
///
/// Instead of storing the current state, event sourcing stores all events
/// that led to the current state. The current state can be reconstructed
/// by replaying all events from the beginning.
///
/// # Examples
///
/// ```ignore
/// use roguelike_workflow::ports::EventStore;
/// use lambars::effect::AsyncIO;
///
/// // Append events after a command is processed
/// let events = vec![GameStarted::new(...).into()];
/// let append_io = event_store.append(&session_id, &events);
///
/// // Load events to reconstruct state
/// let load_io = event_store.load_events(&session_id);
/// ```
pub trait EventStore: Clone + Send + Sync + 'static {
    /// Appends events to the event store.
    ///
    /// Events are stored in order and assigned sequence numbers automatically.
    ///
    /// # Arguments
    ///
    /// * `session_identifier` - The game session these events belong to.
    /// * `events` - The events to append.
    ///
    /// # Returns
    ///
    /// An `AsyncIO` that resolves to `()` on success.
    fn append(
        &self,
        session_identifier: &GameIdentifier,
        events: &[GameSessionEvent],
    ) -> AsyncIO<()>;

    /// Loads all events for a game session.
    ///
    /// Events are returned in the order they were appended.
    ///
    /// # Arguments
    ///
    /// * `session_identifier` - The game session to load events for.
    ///
    /// # Returns
    ///
    /// An `AsyncIO` that resolves to a vector of events.
    fn load_events(&self, session_identifier: &GameIdentifier) -> AsyncIO<Vec<GameSessionEvent>>;

    /// Loads events for a game session since a specific sequence number.
    ///
    /// This is useful for incremental event loading when using snapshots.
    ///
    /// # Arguments
    ///
    /// * `session_identifier` - The game session to load events for.
    /// * `sequence` - The sequence number to start from (exclusive).
    ///
    /// # Returns
    ///
    /// An `AsyncIO` that resolves to a vector of events with sequence > `sequence`.
    fn load_events_since(
        &self,
        session_identifier: &GameIdentifier,
        sequence: u64,
    ) -> AsyncIO<Vec<GameSessionEvent>>;
}

// =============================================================================
// SnapshotStore
// =============================================================================

/// Snapshot store port for event sourcing optimization.
///
/// Snapshots are periodic captures of the aggregate state that allow
/// faster reconstruction without replaying all events from the beginning.
///
/// # Usage Pattern
///
/// 1. Load the latest snapshot for a session
/// 2. If a snapshot exists, start from that state
/// 3. Load events since the snapshot's sequence number
/// 4. Apply those events to get the current state
/// 5. Periodically save new snapshots (e.g., every N events)
pub trait SnapshotStore: Clone + Send + Sync + 'static {
    /// The game session type stored in snapshots.
    type GameSession: Clone + Send + Sync + 'static;

    /// Saves a snapshot of the game session state.
    ///
    /// # Arguments
    ///
    /// * `session_identifier` - The game session identifier.
    /// * `state` - The current state of the game session.
    /// * `sequence` - The event sequence number this snapshot represents.
    ///
    /// # Returns
    ///
    /// An `AsyncIO` that resolves to `()` on success.
    fn save_snapshot(
        &self,
        session_identifier: &GameIdentifier,
        state: &Self::GameSession,
        sequence: u64,
    ) -> AsyncIO<()>;

    /// Loads the latest snapshot for a game session.
    ///
    /// # Arguments
    ///
    /// * `session_identifier` - The game session to load the snapshot for.
    ///
    /// # Returns
    ///
    /// An `AsyncIO` that resolves to `Some((state, sequence))` if a snapshot exists,
    /// `None` otherwise.
    fn load_latest_snapshot(
        &self,
        session_identifier: &GameIdentifier,
    ) -> AsyncIO<Option<(Self::GameSession, u64)>>;
}

// =============================================================================
// SessionCache
// =============================================================================

/// Cache port for game session caching.
///
/// Caching provides fast access to frequently used game sessions,
/// reducing database load and improving response times.
///
/// # Cache Semantics
///
/// - Entries have a TTL (time-to-live) and will be automatically evicted
/// - Cache misses are not errors; the workflow should fall back to the repository
/// - Cache invalidation should be performed when the session state changes
///
/// # Examples
///
/// ```ignore
/// use roguelike_workflow::ports::SessionCache;
/// use std::time::Duration;
///
/// // Try cache first, fall back to repository
/// let cached = cache.get(&session_id);
/// // If None, load from repository and cache it
/// cache.set(&session_id, &session, Duration::from_secs(300));
/// ```
pub trait SessionCache: Clone + Send + Sync + 'static {
    /// The game session type stored in the cache.
    type GameSession: Clone + Send + Sync + 'static;

    /// Gets a game session from the cache.
    ///
    /// # Arguments
    ///
    /// * `identifier` - The game session identifier.
    ///
    /// # Returns
    ///
    /// An `AsyncIO` that resolves to `Some(GameSession)` if found, `None` otherwise.
    fn get(&self, identifier: &GameIdentifier) -> AsyncIO<Option<Self::GameSession>>;

    /// Sets a game session in the cache with a TTL.
    ///
    /// # Arguments
    ///
    /// * `identifier` - The game session identifier.
    /// * `session` - The game session to cache.
    /// * `time_to_live` - How long the entry should remain in the cache.
    ///
    /// # Returns
    ///
    /// An `AsyncIO` that resolves to `()` on success.
    fn set(
        &self,
        identifier: &GameIdentifier,
        session: &Self::GameSession,
        time_to_live: Duration,
    ) -> AsyncIO<()>;

    /// Invalidates a cache entry.
    ///
    /// # Arguments
    ///
    /// * `identifier` - The game session identifier to invalidate.
    ///
    /// # Returns
    ///
    /// An `AsyncIO` that resolves to `()` on success.
    /// No error is raised if the entry does not exist.
    fn invalidate(&self, identifier: &GameIdentifier) -> AsyncIO<()>;
}

// =============================================================================
// RandomGenerator
// =============================================================================

/// Random generator port for deterministic game generation.
///
/// This trait abstracts random number generation to enable:
/// - Reproducible game sessions using the same seed
/// - Testability by providing deterministic implementations
/// - Separation of pure game logic from random I/O
///
/// # Deterministic Randomness
///
/// Once a seed is generated (which involves I/O), all subsequent random
/// operations using that seed are pure functions. This allows game logic
/// to remain deterministic and reproducible.
///
/// # Examples
///
/// ```ignore
/// use roguelike_workflow::ports::RandomGenerator;
///
/// // Generate a seed (I/O operation)
/// let seed = generator.generate_seed();
///
/// // Use seed for deterministic random numbers (pure)
/// let (value, next_seed) = generator.next_u32(&seed);
/// ```
pub trait RandomGenerator: Clone + Send + Sync + 'static {
    /// Generates a new random seed.
    ///
    /// This is an I/O operation that typically uses system entropy.
    /// The returned seed can be stored for reproducibility.
    ///
    /// # Returns
    ///
    /// An `AsyncIO` that resolves to a new random seed.
    fn generate_seed(&self) -> AsyncIO<RandomSeed>;

    /// Generates the next random u32 value from a seed.
    ///
    /// This is a pure function that deterministically produces the next
    /// random value and the next seed state.
    ///
    /// # Arguments
    ///
    /// * `seed` - The current random seed.
    ///
    /// # Returns
    ///
    /// A tuple of (random_value, next_seed).
    fn next_u32(&self, seed: &RandomSeed) -> (u32, RandomSeed);
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    // =========================================================================
    // Mock Implementations for Testing
    // =========================================================================

    /// A simple mock game session for testing.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct MockGameSession {
        identifier: GameIdentifier,
        turn: u32,
    }

    impl MockGameSession {
        fn new(identifier: GameIdentifier) -> Self {
            Self {
                identifier,
                turn: 0,
            }
        }

        fn identifier(&self) -> &GameIdentifier {
            &self.identifier
        }
    }

    /// Mock implementation of GameSessionRepository.
    #[derive(Clone)]
    struct MockGameSessionRepository {
        sessions:
            Arc<std::sync::RwLock<std::collections::HashMap<GameIdentifier, MockGameSession>>>,
    }

    impl MockGameSessionRepository {
        fn new() -> Self {
            Self {
                sessions: Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
            }
        }
    }

    impl GameSessionRepository for MockGameSessionRepository {
        type GameSession = MockGameSession;

        fn find_by_id(&self, identifier: &GameIdentifier) -> AsyncIO<Option<Self::GameSession>> {
            let sessions = Arc::clone(&self.sessions);
            let identifier = *identifier;
            AsyncIO::new(move || async move { sessions.read().unwrap().get(&identifier).cloned() })
        }

        fn save(&self, session: &Self::GameSession) -> AsyncIO<()> {
            let sessions = Arc::clone(&self.sessions);
            let session = session.clone();
            AsyncIO::new(move || async move {
                sessions
                    .write()
                    .unwrap()
                    .insert(*session.identifier(), session);
            })
        }

        fn delete(&self, identifier: &GameIdentifier) -> AsyncIO<()> {
            let sessions = Arc::clone(&self.sessions);
            let identifier = *identifier;
            AsyncIO::new(move || async move {
                sessions.write().unwrap().remove(&identifier);
            })
        }

        fn list_active(&self) -> AsyncIO<Vec<GameIdentifier>> {
            let sessions = Arc::clone(&self.sessions);
            AsyncIO::new(move || async move { sessions.read().unwrap().keys().copied().collect() })
        }
    }

    /// Mock implementation of EventStore.
    #[derive(Clone)]
    struct MockEventStore {
        events: Arc<
            std::sync::RwLock<std::collections::HashMap<GameIdentifier, Vec<GameSessionEvent>>>,
        >,
    }

    impl MockEventStore {
        fn new() -> Self {
            Self {
                events: Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
            }
        }
    }

    impl EventStore for MockEventStore {
        fn append(
            &self,
            session_identifier: &GameIdentifier,
            events: &[GameSessionEvent],
        ) -> AsyncIO<()> {
            let store = Arc::clone(&self.events);
            let session_identifier = *session_identifier;
            let events = events.to_vec();
            AsyncIO::new(move || async move {
                store
                    .write()
                    .unwrap()
                    .entry(session_identifier)
                    .or_default()
                    .extend(events);
            })
        }

        fn load_events(
            &self,
            session_identifier: &GameIdentifier,
        ) -> AsyncIO<Vec<GameSessionEvent>> {
            let store = Arc::clone(&self.events);
            let session_identifier = *session_identifier;
            AsyncIO::new(move || async move {
                store
                    .read()
                    .unwrap()
                    .get(&session_identifier)
                    .cloned()
                    .unwrap_or_default()
            })
        }

        fn load_events_since(
            &self,
            session_identifier: &GameIdentifier,
            sequence: u64,
        ) -> AsyncIO<Vec<GameSessionEvent>> {
            let store = Arc::clone(&self.events);
            let session_identifier = *session_identifier;
            AsyncIO::new(move || async move {
                store
                    .read()
                    .unwrap()
                    .get(&session_identifier)
                    .map(|events| events.iter().skip(sequence as usize).cloned().collect())
                    .unwrap_or_default()
            })
        }
    }

    /// Mock implementation of SessionCache.
    #[derive(Clone)]
    struct MockSessionCache {
        cache: Arc<std::sync::RwLock<std::collections::HashMap<GameIdentifier, MockGameSession>>>,
    }

    impl MockSessionCache {
        fn new() -> Self {
            Self {
                cache: Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
            }
        }
    }

    impl SessionCache for MockSessionCache {
        type GameSession = MockGameSession;

        fn get(&self, identifier: &GameIdentifier) -> AsyncIO<Option<Self::GameSession>> {
            let cache = Arc::clone(&self.cache);
            let identifier = *identifier;
            AsyncIO::new(move || async move { cache.read().unwrap().get(&identifier).cloned() })
        }

        fn set(
            &self,
            identifier: &GameIdentifier,
            session: &Self::GameSession,
            _time_to_live: Duration,
        ) -> AsyncIO<()> {
            let cache = Arc::clone(&self.cache);
            let identifier = *identifier;
            let session = session.clone();
            AsyncIO::new(move || async move {
                cache.write().unwrap().insert(identifier, session);
            })
        }

        fn invalidate(&self, identifier: &GameIdentifier) -> AsyncIO<()> {
            let cache = Arc::clone(&self.cache);
            let identifier = *identifier;
            AsyncIO::new(move || async move {
                cache.write().unwrap().remove(&identifier);
            })
        }
    }

    /// Mock implementation of RandomGenerator.
    #[derive(Clone)]
    struct MockRandomGenerator {
        counter: Arc<AtomicU64>,
    }

    impl MockRandomGenerator {
        fn new() -> Self {
            Self {
                counter: Arc::new(AtomicU64::new(1)),
            }
        }
    }

    impl RandomGenerator for MockRandomGenerator {
        fn generate_seed(&self) -> AsyncIO<RandomSeed> {
            let counter = Arc::clone(&self.counter);
            AsyncIO::new(move || async move {
                let value = counter.fetch_add(1, Ordering::SeqCst);
                RandomSeed::new(value)
            })
        }

        fn next_u32(&self, seed: &RandomSeed) -> (u32, RandomSeed) {
            // Simple LCG for testing
            let next_value = seed.value().wrapping_mul(1103515245).wrapping_add(12345);
            let random_value = (next_value >> 16) as u32;
            (random_value, RandomSeed::new(next_value))
        }
    }

    // =========================================================================
    // GameSessionRepository Tests
    // =========================================================================

    mod game_session_repository {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn save_and_find_by_id() {
            let repository = MockGameSessionRepository::new();
            let identifier = GameIdentifier::new();
            let session = MockGameSession::new(identifier);

            repository.save(&session).run_async().await;
            let found = repository.find_by_id(&identifier).run_async().await;

            assert_eq!(found, Some(session));
        }

        #[rstest]
        #[tokio::test]
        async fn find_by_id_returns_none_for_missing() {
            let repository = MockGameSessionRepository::new();
            let identifier = GameIdentifier::new();

            let found = repository.find_by_id(&identifier).run_async().await;

            assert!(found.is_none());
        }

        #[rstest]
        #[tokio::test]
        async fn delete_removes_session() {
            let repository = MockGameSessionRepository::new();
            let identifier = GameIdentifier::new();
            let session = MockGameSession::new(identifier);

            repository.save(&session).run_async().await;
            repository.delete(&identifier).run_async().await;
            let found = repository.find_by_id(&identifier).run_async().await;

            assert!(found.is_none());
        }

        #[rstest]
        #[tokio::test]
        async fn list_active_returns_all_sessions() {
            let repository = MockGameSessionRepository::new();
            let identifier1 = GameIdentifier::new();
            let identifier2 = GameIdentifier::new();
            let session1 = MockGameSession::new(identifier1);
            let session2 = MockGameSession::new(identifier2);

            repository.save(&session1).run_async().await;
            repository.save(&session2).run_async().await;
            let active = repository.list_active().run_async().await;

            assert_eq!(active.len(), 2);
            assert!(active.contains(&identifier1));
            assert!(active.contains(&identifier2));
        }
    }

    // =========================================================================
    // EventStore Tests
    // =========================================================================

    mod event_store {
        use roguelike_domain::game_session::{GameStarted, RandomSeed as DomainRandomSeed};

        use super::*;

        #[rstest]
        #[tokio::test]
        async fn append_and_load_events() {
            let store = MockEventStore::new();
            let identifier = GameIdentifier::new();
            let event =
                GameSessionEvent::Started(GameStarted::new(identifier, DomainRandomSeed::new(42)));

            store
                .append(&identifier, std::slice::from_ref(&event))
                .run_async()
                .await;
            let loaded = store.load_events(&identifier).run_async().await;

            assert_eq!(loaded.len(), 1);
            assert_eq!(loaded[0], event);
        }

        #[rstest]
        #[tokio::test]
        async fn load_events_returns_empty_for_missing() {
            let store = MockEventStore::new();
            let identifier = GameIdentifier::new();

            let loaded = store.load_events(&identifier).run_async().await;

            assert!(loaded.is_empty());
        }

        #[rstest]
        #[tokio::test]
        async fn load_events_since_filters_correctly() {
            let store = MockEventStore::new();
            let identifier = GameIdentifier::new();
            let event1 =
                GameSessionEvent::Started(GameStarted::new(identifier, DomainRandomSeed::new(1)));
            let event2 =
                GameSessionEvent::Started(GameStarted::new(identifier, DomainRandomSeed::new(2)));

            store
                .append(&identifier, &[event1.clone(), event2.clone()])
                .run_async()
                .await;
            let loaded = store.load_events_since(&identifier, 1).run_async().await;

            assert_eq!(loaded.len(), 1);
            assert_eq!(loaded[0], event2);
        }
    }

    // =========================================================================
    // SessionCache Tests
    // =========================================================================

    mod session_cache {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn set_and_get() {
            let cache = MockSessionCache::new();
            let identifier = GameIdentifier::new();
            let session = MockGameSession::new(identifier);

            cache
                .set(&identifier, &session, Duration::from_secs(60))
                .run_async()
                .await;
            let found = cache.get(&identifier).run_async().await;

            assert_eq!(found, Some(session));
        }

        #[rstest]
        #[tokio::test]
        async fn get_returns_none_for_missing() {
            let cache = MockSessionCache::new();
            let identifier = GameIdentifier::new();

            let found = cache.get(&identifier).run_async().await;

            assert!(found.is_none());
        }

        #[rstest]
        #[tokio::test]
        async fn invalidate_removes_entry() {
            let cache = MockSessionCache::new();
            let identifier = GameIdentifier::new();
            let session = MockGameSession::new(identifier);

            cache
                .set(&identifier, &session, Duration::from_secs(60))
                .run_async()
                .await;
            cache.invalidate(&identifier).run_async().await;
            let found = cache.get(&identifier).run_async().await;

            assert!(found.is_none());
        }
    }

    // =========================================================================
    // RandomGenerator Tests
    // =========================================================================

    mod random_generator {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn generate_seed_returns_different_seeds() {
            let generator = MockRandomGenerator::new();

            let seed1 = generator.generate_seed().run_async().await;
            let seed2 = generator.generate_seed().run_async().await;

            assert_ne!(seed1, seed2);
        }

        #[rstest]
        fn next_u32_is_deterministic() {
            let generator = MockRandomGenerator::new();
            let seed = RandomSeed::new(12345);

            let (value1, next_seed1) = generator.next_u32(&seed);
            let (value2, next_seed2) = generator.next_u32(&seed);

            assert_eq!(value1, value2);
            assert_eq!(next_seed1, next_seed2);
        }

        #[rstest]
        fn next_u32_produces_sequence() {
            let generator = MockRandomGenerator::new();
            let seed = RandomSeed::new(42);

            let (value1, seed1) = generator.next_u32(&seed);
            let (value2, seed2) = generator.next_u32(&seed1);
            let (value3, _) = generator.next_u32(&seed2);

            // Values should be different (with high probability)
            assert_ne!(value1, value2);
            assert_ne!(value2, value3);
        }
    }
}
