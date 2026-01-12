use std::time::Duration;

use lambars::effect::AsyncIO;
use roguelike_domain::game_session::{GameIdentifier, GameSessionEvent, RandomSeed};

// =============================================================================
// Type Aliases for Workflow Results
// =============================================================================

pub type WorkflowResult<T> = Result<T, crate::errors::WorkflowError>;

// =============================================================================
// GameSessionRepository
// =============================================================================

pub trait GameSessionRepository: Clone + Send + Sync + 'static {
    type GameSession: Clone + Send + Sync + 'static;

    fn find_by_id(&self, identifier: &GameIdentifier) -> AsyncIO<Option<Self::GameSession>>;

    fn save(&self, session: &Self::GameSession) -> AsyncIO<()>;

    fn delete(&self, identifier: &GameIdentifier) -> AsyncIO<()>;

    fn list_active(&self) -> AsyncIO<Vec<GameIdentifier>>;
}

// =============================================================================
// EventStore
// =============================================================================

pub trait EventStore: Clone + Send + Sync + 'static {
    fn append(
        &self,
        session_identifier: &GameIdentifier,
        events: &[GameSessionEvent],
    ) -> AsyncIO<()>;

    fn load_events(&self, session_identifier: &GameIdentifier) -> AsyncIO<Vec<GameSessionEvent>>;

    fn load_events_since(
        &self,
        session_identifier: &GameIdentifier,
        sequence: u64,
    ) -> AsyncIO<Vec<GameSessionEvent>>;
}

// =============================================================================
// SnapshotStore
// =============================================================================

pub trait SnapshotStore: Clone + Send + Sync + 'static {
    type GameSession: Clone + Send + Sync + 'static;

    fn save_snapshot(
        &self,
        session_identifier: &GameIdentifier,
        state: &Self::GameSession,
        sequence: u64,
    ) -> AsyncIO<()>;

    fn load_latest_snapshot(
        &self,
        session_identifier: &GameIdentifier,
    ) -> AsyncIO<Option<(Self::GameSession, u64)>>;
}

// =============================================================================
// SessionCache
// =============================================================================

pub trait SessionCache: Clone + Send + Sync + 'static {
    type GameSession: Clone + Send + Sync + 'static;

    fn get(&self, identifier: &GameIdentifier) -> AsyncIO<Option<Self::GameSession>>;

    fn set(
        &self,
        identifier: &GameIdentifier,
        session: &Self::GameSession,
        time_to_live: Duration,
    ) -> AsyncIO<()>;

    fn invalidate(&self, identifier: &GameIdentifier) -> AsyncIO<()>;
}

// =============================================================================
// RandomGenerator
// =============================================================================

pub trait RandomGenerator: Clone + Send + Sync + 'static {
    fn generate_seed(&self) -> AsyncIO<RandomSeed>;

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
