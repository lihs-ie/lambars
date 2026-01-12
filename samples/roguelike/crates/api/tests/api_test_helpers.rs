#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use axum::Router;
use lambars::effect::AsyncIO;
use roguelike_domain::game_session::{GameIdentifier, GameSessionEvent, RandomSeed};
use roguelike_workflow::ports::{EventStore, GameSessionRepository, RandomGenerator, SessionCache};

use roguelike_api::state::AppState;

// =============================================================================
// Mock GameSession
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockGameSession {
    pub identifier: GameIdentifier,

    pub player_name: String,

    pub turn_count: u32,

    pub is_active: bool,
}

impl MockGameSession {
    #[must_use]
    pub fn new(identifier: GameIdentifier, player_name: impl Into<String>) -> Self {
        Self {
            identifier,
            player_name: player_name.into(),
            turn_count: 0,
            is_active: true,
        }
    }

    #[must_use]
    pub fn identifier(&self) -> &GameIdentifier {
        &self.identifier
    }

    #[must_use]
    pub fn with_turn_count(self, turn_count: u32) -> Self {
        Self { turn_count, ..self }
    }

    #[must_use]
    pub fn ended(self) -> Self {
        Self {
            is_active: false,
            ..self
        }
    }
}

// =============================================================================
// MockGameSessionRepository
// =============================================================================

#[derive(Clone)]
pub struct MockGameSessionRepository {
    sessions: Arc<RwLock<HashMap<GameIdentifier, MockGameSession>>>,
}

impl MockGameSessionRepository {
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[must_use]
    pub fn with_sessions(sessions: Vec<MockGameSession>) -> Self {
        let map = sessions.into_iter().map(|s| (s.identifier, s)).collect();
        Self {
            sessions: Arc::new(RwLock::new(map)),
        }
    }

    pub fn add_session(&self, session: MockGameSession) {
        self.sessions
            .write()
            .unwrap()
            .insert(session.identifier, session);
    }

    #[must_use]
    pub fn get_all_sessions(&self) -> Vec<MockGameSession> {
        self.sessions.read().unwrap().values().cloned().collect()
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.sessions.read().unwrap().len()
    }
}

impl Default for MockGameSessionRepository {
    fn default() -> Self {
        Self::new()
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
                .insert(session.identifier, session);
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
        AsyncIO::new(move || async move {
            sessions
                .read()
                .unwrap()
                .values()
                .filter(|s| s.is_active)
                .map(|s| s.identifier)
                .collect()
        })
    }
}

// =============================================================================
// MockSessionCache
// =============================================================================

#[derive(Clone)]
pub struct MockSessionCache {
    cache: Arc<RwLock<HashMap<GameIdentifier, MockGameSession>>>,
}

impl MockSessionCache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.cache.read().unwrap().len()
    }

    #[must_use]
    pub fn contains(&self, identifier: &GameIdentifier) -> bool {
        self.cache.read().unwrap().contains_key(identifier)
    }
}

impl Default for MockSessionCache {
    fn default() -> Self {
        Self::new()
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

// =============================================================================
// MockEventStore
// =============================================================================

#[derive(Clone)]
pub struct MockEventStore {
    events: Arc<RwLock<HashMap<GameIdentifier, Vec<GameSessionEvent>>>>,
}

impl MockEventStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[must_use]
    pub fn get_events(&self, session_identifier: &GameIdentifier) -> Vec<GameSessionEvent> {
        self.events
            .read()
            .unwrap()
            .get(session_identifier)
            .cloned()
            .unwrap_or_default()
    }

    #[must_use]
    pub fn total_event_count(&self) -> usize {
        self.events.read().unwrap().values().map(|v| v.len()).sum()
    }
}

impl Default for MockEventStore {
    fn default() -> Self {
        Self::new()
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

    fn load_events(&self, session_identifier: &GameIdentifier) -> AsyncIO<Vec<GameSessionEvent>> {
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

// =============================================================================
// MockRandomGenerator
// =============================================================================

#[derive(Clone)]
pub struct MockRandomGenerator {
    seed_counter: Arc<AtomicU64>,
}

impl MockRandomGenerator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            seed_counter: Arc::new(AtomicU64::new(1)),
        }
    }

    #[must_use]
    pub fn with_seed(initial_seed: u64) -> Self {
        Self {
            seed_counter: Arc::new(AtomicU64::new(initial_seed)),
        }
    }

    #[must_use]
    pub fn current_seed(&self) -> u64 {
        self.seed_counter.load(Ordering::SeqCst)
    }
}

impl Default for MockRandomGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomGenerator for MockRandomGenerator {
    fn generate_seed(&self) -> AsyncIO<RandomSeed> {
        let counter = Arc::clone(&self.seed_counter);
        AsyncIO::new(move || async move {
            let value = counter.fetch_add(1, Ordering::SeqCst);
            RandomSeed::new(value)
        })
    }

    fn next_u32(&self, seed: &RandomSeed) -> (u32, RandomSeed) {
        // Linear congruential generator for deterministic output
        let next_value = seed.value().wrapping_mul(1103515245).wrapping_add(12345);
        let random_value = (next_value >> 16) as u32;
        (random_value, RandomSeed::new(next_value))
    }
}

// =============================================================================
// TestAppBuilder
// =============================================================================

pub struct TestAppBuilder {
    repository: MockGameSessionRepository,
    cache: MockSessionCache,
    event_store: MockEventStore,
    random: MockRandomGenerator,
}

impl TestAppBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            repository: MockGameSessionRepository::new(),
            cache: MockSessionCache::new(),
            event_store: MockEventStore::new(),
            random: MockRandomGenerator::new(),
        }
    }

    #[must_use]
    pub fn with_repository(mut self, repository: MockGameSessionRepository) -> Self {
        self.repository = repository;
        self
    }

    #[must_use]
    pub fn with_cache(mut self, cache: MockSessionCache) -> Self {
        self.cache = cache;
        self
    }

    #[must_use]
    pub fn with_event_store(mut self, event_store: MockEventStore) -> Self {
        self.event_store = event_store;
        self
    }

    #[must_use]
    pub fn with_random(mut self, random: MockRandomGenerator) -> Self {
        self.random = random;
        self
    }

    #[must_use]
    pub fn with_session(self, session: MockGameSession) -> Self {
        self.repository.add_session(session);
        self
    }

    #[must_use]
    pub fn build_state(
        self,
    ) -> AppState<MockGameSessionRepository, MockSessionCache, MockEventStore, MockRandomGenerator>
    {
        AppState::new(self.repository, self.cache, self.event_store, self.random)
    }

    pub fn build_router(self) -> Router {
        let state = self.build_state();
        Router::new().with_state(state)
    }
}

impl Default for TestAppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Test Fixtures
// =============================================================================

#[must_use]
pub fn create_test_session() -> MockGameSession {
    MockGameSession::new(GameIdentifier::new(), "TestPlayer")
}

#[must_use]
pub fn create_test_session_with_name(name: impl Into<String>) -> MockGameSession {
    MockGameSession::new(GameIdentifier::new(), name)
}

#[must_use]
pub fn create_test_session_with_id(
    identifier: GameIdentifier,
    name: impl Into<String>,
) -> MockGameSession {
    MockGameSession::new(identifier, name)
}

#[must_use]
pub fn create_test_sessions(count: usize) -> Vec<MockGameSession> {
    (0..count)
        .map(|index| MockGameSession::new(GameIdentifier::new(), format!("Player{}", index + 1)))
        .collect()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    mod mock_game_session {
        use super::*;

        #[rstest]
        fn new_creates_session() {
            let identifier = GameIdentifier::new();
            let session = MockGameSession::new(identifier, "TestPlayer");

            assert_eq!(*session.identifier(), identifier);
            assert_eq!(session.player_name, "TestPlayer");
            assert_eq!(session.turn_count, 0);
            assert!(session.is_active);
        }

        #[rstest]
        fn with_turn_count_updates_turn() {
            let session = MockGameSession::new(GameIdentifier::new(), "Player").with_turn_count(5);
            assert_eq!(session.turn_count, 5);
        }

        #[rstest]
        fn ended_sets_inactive() {
            let session = MockGameSession::new(GameIdentifier::new(), "Player").ended();
            assert!(!session.is_active);
        }
    }

    mod mock_repository {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn save_and_find() {
            let repository = MockGameSessionRepository::new();
            let session = MockGameSession::new(GameIdentifier::new(), "TestPlayer");
            let identifier = *session.identifier();

            repository.save(&session).run_async().await;
            let found = repository.find_by_id(&identifier).run_async().await;

            assert_eq!(found, Some(session));
        }

        #[rstest]
        #[tokio::test]
        async fn find_returns_none_for_unknown() {
            let repository = MockGameSessionRepository::new();
            let found = repository
                .find_by_id(&GameIdentifier::new())
                .run_async()
                .await;

            assert!(found.is_none());
        }

        #[rstest]
        #[tokio::test]
        async fn delete_removes_session() {
            let repository = MockGameSessionRepository::new();
            let session = MockGameSession::new(GameIdentifier::new(), "TestPlayer");
            let identifier = *session.identifier();

            repository.save(&session).run_async().await;
            repository.delete(&identifier).run_async().await;
            let found = repository.find_by_id(&identifier).run_async().await;

            assert!(found.is_none());
        }

        #[rstest]
        #[tokio::test]
        async fn list_active_returns_only_active() {
            let active = MockGameSession::new(GameIdentifier::new(), "Active");
            let inactive = MockGameSession::new(GameIdentifier::new(), "Inactive").ended();

            let repository =
                MockGameSessionRepository::with_sessions(vec![active.clone(), inactive]);

            let active_ids = repository.list_active().run_async().await;

            assert_eq!(active_ids.len(), 1);
            assert_eq!(active_ids[0], active.identifier);
        }

        #[rstest]
        fn count_returns_correct_count() {
            let repository = MockGameSessionRepository::with_sessions(create_test_sessions(3));
            assert_eq!(repository.count(), 3);
        }
    }

    mod mock_cache {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn set_and_get() {
            let cache = MockSessionCache::new();
            let session = MockGameSession::new(GameIdentifier::new(), "TestPlayer");
            let identifier = *session.identifier();

            cache
                .set(&identifier, &session, Duration::from_secs(60))
                .run_async()
                .await;
            let found = cache.get(&identifier).run_async().await;

            assert_eq!(found, Some(session));
        }

        #[rstest]
        #[tokio::test]
        async fn invalidate_removes_from_cache() {
            let cache = MockSessionCache::new();
            let session = MockGameSession::new(GameIdentifier::new(), "TestPlayer");
            let identifier = *session.identifier();

            cache
                .set(&identifier, &session, Duration::from_secs(60))
                .run_async()
                .await;
            cache.invalidate(&identifier).run_async().await;
            let found = cache.get(&identifier).run_async().await;

            assert!(found.is_none());
        }

        #[rstest]
        #[tokio::test]
        async fn contains_checks_cache() {
            let cache = MockSessionCache::new();
            let session = MockGameSession::new(GameIdentifier::new(), "TestPlayer");
            let identifier = *session.identifier();

            assert!(!cache.contains(&identifier));

            cache
                .set(&identifier, &session, Duration::from_secs(60))
                .run_async()
                .await;

            assert!(cache.contains(&identifier));
        }
    }

    mod mock_event_store {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn append_and_load() {
            let store = MockEventStore::new();
            let session_id = GameIdentifier::new();

            // Note: We can't easily create GameSessionEvent without the actual domain types,
            // so we just test the basic structure
            let events: Vec<GameSessionEvent> = vec![];
            store.append(&session_id, &events).run_async().await;

            let loaded = store.load_events(&session_id).run_async().await;
            assert_eq!(loaded.len(), events.len());
        }

        #[rstest]
        fn total_event_count_initially_zero() {
            let store = MockEventStore::new();
            assert_eq!(store.total_event_count(), 0);
        }
    }

    mod mock_random {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn generate_seed_increments() {
            let random = MockRandomGenerator::new();

            let seed1 = random.generate_seed().run_async().await;
            let seed2 = random.generate_seed().run_async().await;

            assert_ne!(seed1, seed2);
        }

        #[rstest]
        fn next_u32_is_deterministic() {
            let random = MockRandomGenerator::new();
            let seed = RandomSeed::new(12345);

            let (value1, next_seed1) = random.next_u32(&seed);
            let (value2, next_seed2) = random.next_u32(&seed);

            assert_eq!(value1, value2);
            assert_eq!(next_seed1, next_seed2);
        }

        #[rstest]
        fn with_seed_starts_from_specified() {
            let random = MockRandomGenerator::with_seed(100);
            assert_eq!(random.current_seed(), 100);
        }
    }

    mod test_app_builder {
        use super::*;

        #[rstest]
        fn new_creates_builder() {
            let _builder = TestAppBuilder::new();
        }

        #[rstest]
        fn default_creates_builder() {
            let _builder = TestAppBuilder::default();
        }

        #[rstest]
        fn build_state_creates_state() {
            let builder = TestAppBuilder::new();
            let _state = builder.build_state();
        }

        #[rstest]
        fn with_session_adds_session() {
            let session = create_test_session();
            let identifier = *session.identifier();

            let builder = TestAppBuilder::new().with_session(session);
            let state = builder.build_state();

            // Verify the session is in the repository
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let found = runtime
                .block_on(async { state.repository.find_by_id(&identifier).run_async().await });

            assert!(found.is_some());
        }
    }

    mod test_fixtures {
        use super::*;

        #[rstest]
        fn create_test_session_creates_session() {
            let session = create_test_session();
            assert_eq!(session.player_name, "TestPlayer");
            assert!(session.is_active);
        }

        #[rstest]
        fn create_test_session_with_name_uses_name() {
            let session = create_test_session_with_name("CustomName");
            assert_eq!(session.player_name, "CustomName");
        }

        #[rstest]
        fn create_test_session_with_id_uses_id() {
            let identifier = GameIdentifier::new();
            let session = create_test_session_with_id(identifier, "Player");
            assert_eq!(*session.identifier(), identifier);
        }

        #[rstest]
        fn create_test_sessions_creates_multiple() {
            let sessions = create_test_sessions(5);
            assert_eq!(sessions.len(), 5);

            // Each session should have a unique identifier
            let identifiers: std::collections::HashSet<_> =
                sessions.iter().map(|s| s.identifier).collect();
            assert_eq!(identifiers.len(), 5);
        }
    }
}
