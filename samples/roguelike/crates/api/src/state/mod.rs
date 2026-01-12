use std::sync::Arc;
use std::time::Duration;

use lambars::effect::AsyncIO;
use roguelike_domain::game_session::{GameIdentifier, GameSessionEvent, RandomSeed};
use roguelike_workflow::ports::{EventStore, GameSessionRepository, RandomGenerator, SessionCache};

// =============================================================================
// AppState
// =============================================================================

#[derive(Clone)]
pub struct AppState<Repository, Cache, Events, Random>
where
    Repository: GameSessionRepository,
    Cache: SessionCache<GameSession = Repository::GameSession>,
    Events: EventStore,
    Random: RandomGenerator,
{
    pub repository: Arc<Repository>,

    pub cache: Arc<Cache>,

    pub event_store: Arc<Events>,

    pub random: Arc<Random>,
}

impl<Repository, Cache, Events, Random> AppState<Repository, Cache, Events, Random>
where
    Repository: GameSessionRepository,
    Cache: SessionCache<GameSession = Repository::GameSession>,
    Events: EventStore,
    Random: RandomGenerator,
{
    #[must_use]
    pub fn new(repository: Repository, cache: Cache, event_store: Events, random: Random) -> Self {
        Self {
            repository: Arc::new(repository),
            cache: Arc::new(cache),
            event_store: Arc::new(event_store),
            random: Arc::new(random),
        }
    }

    #[must_use]
    pub fn from_arc(
        repository: Arc<Repository>,
        cache: Arc<Cache>,
        event_store: Arc<Events>,
        random: Arc<Random>,
    ) -> Self {
        Self {
            repository,
            cache,
            event_store,
            random,
        }
    }
}

// =============================================================================
// Type Aliases for Common State Types
// =============================================================================

// Note: The following type aliases are prepared for future dynamic dispatch usage.
// They use trait objects which require dyn-compatible traits.

pub type BoxedEventStore = Arc<dyn EventStore + Send + Sync>;

pub type BoxedRandomGenerator = Arc<dyn RandomGenerator + Send + Sync>;

// =============================================================================
// DynamicAppState - For runtime polymorphism
// =============================================================================

#[derive(Clone)]
#[allow(dead_code)]
pub struct DynamicAppState {
    repository_inner: Arc<dyn DynamicRepository>,

    cache_inner: Arc<dyn DynamicCache>,

    pub event_store: Arc<dyn DynamicEventStore>,

    pub random: Arc<dyn DynamicRandom>,
}

pub trait DynamicRepository: Send + Sync + 'static {
    fn find_by_id_dynamic(&self, identifier: &GameIdentifier) -> AsyncIO<Option<Vec<u8>>>;

    fn save_dynamic(&self, identifier: &GameIdentifier, data: &[u8]) -> AsyncIO<()>;

    fn delete_dynamic(&self, identifier: &GameIdentifier) -> AsyncIO<()>;

    fn list_active_dynamic(&self) -> AsyncIO<Vec<GameIdentifier>>;
}

pub trait DynamicCache: Send + Sync + 'static {
    fn get_dynamic(&self, identifier: &GameIdentifier) -> AsyncIO<Option<Vec<u8>>>;

    fn set_dynamic(
        &self,
        identifier: &GameIdentifier,
        data: &[u8],
        time_to_live: Duration,
    ) -> AsyncIO<()>;

    fn invalidate_dynamic(&self, identifier: &GameIdentifier) -> AsyncIO<()>;
}

pub trait DynamicEventStore: Send + Sync + 'static {
    fn append_dynamic(
        &self,
        session_identifier: &GameIdentifier,
        events: &[GameSessionEvent],
    ) -> AsyncIO<()>;

    fn load_events_dynamic(
        &self,
        session_identifier: &GameIdentifier,
    ) -> AsyncIO<Vec<GameSessionEvent>>;

    fn load_events_since_dynamic(
        &self,
        session_identifier: &GameIdentifier,
        sequence: u64,
    ) -> AsyncIO<Vec<GameSessionEvent>>;
}

pub trait DynamicRandom: Send + Sync + 'static {
    fn generate_seed_dynamic(&self) -> AsyncIO<RandomSeed>;

    fn next_u32_dynamic(&self, seed: &RandomSeed) -> (u32, RandomSeed);
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::collections::HashMap;
    use std::sync::RwLock;
    use std::sync::atomic::{AtomicU64, Ordering};

    // =========================================================================
    // Mock Implementations
    // =========================================================================

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

    #[derive(Clone)]
    struct MockRepository {
        sessions: Arc<RwLock<HashMap<GameIdentifier, MockGameSession>>>,
    }

    impl MockRepository {
        fn new() -> Self {
            Self {
                sessions: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }

    impl GameSessionRepository for MockRepository {
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

    #[derive(Clone)]
    struct MockCache {
        cache: Arc<RwLock<HashMap<GameIdentifier, MockGameSession>>>,
    }

    impl MockCache {
        fn new() -> Self {
            Self {
                cache: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }

    impl SessionCache for MockCache {
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

    #[derive(Clone)]
    struct MockEventStore {
        events: Arc<RwLock<HashMap<GameIdentifier, Vec<GameSessionEvent>>>>,
    }

    impl MockEventStore {
        fn new() -> Self {
            Self {
                events: Arc::new(RwLock::new(HashMap::new())),
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

    #[derive(Clone)]
    struct MockRandom {
        counter: Arc<AtomicU64>,
    }

    impl MockRandom {
        fn new() -> Self {
            Self {
                counter: Arc::new(AtomicU64::new(1)),
            }
        }
    }

    impl RandomGenerator for MockRandom {
        fn generate_seed(&self) -> AsyncIO<RandomSeed> {
            let counter = Arc::clone(&self.counter);
            AsyncIO::new(move || async move {
                let value = counter.fetch_add(1, Ordering::SeqCst);
                RandomSeed::new(value)
            })
        }

        fn next_u32(&self, seed: &RandomSeed) -> (u32, RandomSeed) {
            let next_value = seed.value().wrapping_mul(1103515245).wrapping_add(12345);
            let random_value = (next_value >> 16) as u32;
            (random_value, RandomSeed::new(next_value))
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================

    mod app_state {
        use super::*;

        #[rstest]
        fn new_creates_state() {
            let repository = MockRepository::new();
            let cache = MockCache::new();
            let event_store = MockEventStore::new();
            let random = MockRandom::new();

            let state = AppState::new(repository, cache, event_store, random);

            // Verify Arc wrapped correctly by checking reference count
            assert_eq!(Arc::strong_count(&state.repository), 1);
            assert_eq!(Arc::strong_count(&state.cache), 1);
            assert_eq!(Arc::strong_count(&state.event_store), 1);
            assert_eq!(Arc::strong_count(&state.random), 1);
        }

        #[rstest]
        fn clone_shares_arc_references() {
            let repository = MockRepository::new();
            let cache = MockCache::new();
            let event_store = MockEventStore::new();
            let random = MockRandom::new();

            let state1 = AppState::new(repository, cache, event_store, random);
            let state2 = state1.clone();

            // Both states should share the same Arc references
            assert_eq!(Arc::strong_count(&state1.repository), 2);
            assert_eq!(Arc::strong_count(&state2.repository), 2);
            assert!(Arc::ptr_eq(&state1.repository, &state2.repository));
        }

        #[rstest]
        fn from_arc_accepts_pre_wrapped_dependencies() {
            let repository = Arc::new(MockRepository::new());
            let cache = Arc::new(MockCache::new());
            let event_store = Arc::new(MockEventStore::new());
            let random = Arc::new(MockRandom::new());

            let state = AppState::from_arc(
                Arc::clone(&repository),
                Arc::clone(&cache),
                Arc::clone(&event_store),
                Arc::clone(&random),
            );

            // Should share the same Arc references
            assert!(Arc::ptr_eq(&repository, &state.repository));
            assert!(Arc::ptr_eq(&cache, &state.cache));
            assert!(Arc::ptr_eq(&event_store, &state.event_store));
            assert!(Arc::ptr_eq(&random, &state.random));
        }

        #[rstest]
        #[tokio::test]
        async fn repository_accessible_through_state() {
            let repository = MockRepository::new();
            let cache = MockCache::new();
            let event_store = MockEventStore::new();
            let random = MockRandom::new();

            let state = AppState::new(repository, cache, event_store, random);

            let identifier = GameIdentifier::new();
            let session = MockGameSession::new(identifier);

            state.repository.save(&session).run_async().await;
            let found = state.repository.find_by_id(&identifier).run_async().await;

            assert_eq!(found, Some(session));
        }

        #[rstest]
        #[tokio::test]
        async fn cache_accessible_through_state() {
            let repository = MockRepository::new();
            let cache = MockCache::new();
            let event_store = MockEventStore::new();
            let random = MockRandom::new();

            let state = AppState::new(repository, cache, event_store, random);

            let identifier = GameIdentifier::new();
            let session = MockGameSession::new(identifier);

            state
                .cache
                .set(&identifier, &session, Duration::from_secs(60))
                .run_async()
                .await;
            let found = state.cache.get(&identifier).run_async().await;

            assert_eq!(found, Some(session));
        }

        #[rstest]
        #[tokio::test]
        async fn random_accessible_through_state() {
            let repository = MockRepository::new();
            let cache = MockCache::new();
            let event_store = MockEventStore::new();
            let random = MockRandom::new();

            let state = AppState::new(repository, cache, event_store, random);

            let seed1 = state.random.generate_seed().run_async().await;
            let seed2 = state.random.generate_seed().run_async().await;

            assert_ne!(seed1, seed2);
        }
    }
}
