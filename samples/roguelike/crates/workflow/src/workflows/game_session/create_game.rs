use std::time::Duration;

use lambars::effect::AsyncIO;
use lambars::pipe_async;
use roguelike_domain::game_session::{GameIdentifier, GameSessionEvent, GameStarted, RandomSeed};

use super::CreateGameCommand;
use super::resume_game::SessionStateAccessor;
use crate::errors::WorkflowError;
use crate::ports::{
    EventStore, GameSessionRepository, RandomGenerator, SessionCache, WorkflowResult,
};

// =============================================================================
// Workflow Configuration
// =============================================================================

const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300); // 5 minutes

// =============================================================================
// Step 1: Extract Seed [Pure]
// =============================================================================

fn extract_seed(command: CreateGameCommand) -> Option<RandomSeed> {
    command.seed()
}

// =============================================================================
// Step 2: Generate Seed [IO]
// =============================================================================

fn generate_seed_io<G: RandomGenerator>(
    random: G,
) -> impl Fn(Option<RandomSeed>) -> AsyncIO<RandomSeed> {
    move |provided_seed| match provided_seed {
        Some(seed) => AsyncIO::pure(seed),
        None => random.generate_seed(),
    }
}

// =============================================================================
// Step 3: Create Identifiers [Pure]
// =============================================================================

fn create_identifiers(seed: RandomSeed) -> (GameIdentifier, RandomSeed) {
    let game_identifier = GameIdentifier::new();
    (game_identifier, seed)
}

// =============================================================================
// Step 4-6: Create GameSession [Pure]
// Note: Floor/Player creation is simplified for current domain model
// =============================================================================

fn create_session_data(input: (GameIdentifier, RandomSeed)) -> (GameIdentifier, RandomSeed) {
    // In full implementation, this would create Floor, Player, and GameSession
    // For now, we pass through the identifiers
    input
}

// =============================================================================
// Step 7: Generate Events [Pure]
// =============================================================================

fn generate_game_started_event(
    input: (GameIdentifier, RandomSeed),
) -> (GameIdentifier, GameStarted) {
    let (game_identifier, seed) = input;
    let event = GameStarted::new(game_identifier, seed);
    (game_identifier, event)
}

// =============================================================================
// Step 8: Wrap Event [Pure]
// =============================================================================

fn wrap_event_in_list(
    input: (GameIdentifier, GameStarted),
) -> (GameIdentifier, Vec<GameSessionEvent>) {
    let (game_identifier, event) = input;
    (game_identifier, vec![GameSessionEvent::from(event)])
}

// =============================================================================
// Step 9: Check No Existing Session [IO]
// =============================================================================

#[allow(clippy::type_complexity)]
fn check_no_existing_session_io<R: GameSessionRepository>(
    repository: R,
) -> impl Fn(
    (GameIdentifier, Vec<GameSessionEvent>),
) -> AsyncIO<Result<(GameIdentifier, Vec<GameSessionEvent>), WorkflowError>> {
    move |(game_identifier, events)| {
        let events_clone = events.clone();
        repository
            .find_by_id(&game_identifier)
            .fmap(move |existing| {
                if existing.is_some() {
                    Err(WorkflowError::conflict(
                        "Game session with this identifier already exists",
                    ))
                } else {
                    Ok((game_identifier, events_clone))
                }
            })
    }
}

// =============================================================================
// Step 10: Append Events [IO]
// =============================================================================

#[allow(clippy::type_complexity)]
fn append_events_io<E: EventStore>(
    event_store: E,
) -> impl Fn(
    Result<(GameIdentifier, Vec<GameSessionEvent>), WorkflowError>,
) -> AsyncIO<Result<GameIdentifier, WorkflowError>> {
    move |result| match result {
        Err(error) => AsyncIO::pure(Err(error)),
        Ok((game_identifier, events)) => event_store
            .append(&game_identifier, &events)
            .fmap(move |()| Ok(game_identifier)),
    }
}

// =============================================================================
// Step 11: Load Created Session [IO]
// =============================================================================

#[allow(clippy::type_complexity)]
fn load_created_session_io<R: GameSessionRepository>(
    repository: R,
) -> impl Fn(
    Result<GameIdentifier, WorkflowError>,
) -> AsyncIO<Result<(GameIdentifier, R::GameSession), WorkflowError>> {
    move |result| match result {
        Err(error) => AsyncIO::pure(Err(error)),
        Ok(game_identifier) => repository.find_by_id(&game_identifier).fmap(move |opt| {
            opt.map(|session| (game_identifier, session))
                .ok_or_else(|| {
                    WorkflowError::repository("save", "Failed to create game session from events")
                })
        }),
    }
}

// =============================================================================
// Step 12: Cache Session [IO]
// =============================================================================

#[allow(clippy::type_complexity)]
fn cache_session_io<C: SessionCache>(
    cache: C,
    time_to_live: Duration,
) -> impl Fn(
    Result<(GameIdentifier, C::GameSession), WorkflowError>,
) -> AsyncIO<WorkflowResult<C::GameSession>> {
    move |result| match result {
        Err(error) => AsyncIO::pure(Err(error)),
        Ok((game_identifier, session)) => {
            let session_clone = session.clone();
            cache
                .set(&game_identifier, &session, time_to_live)
                .fmap(move |()| Ok(session_clone))
        }
    }
}

// =============================================================================
// CreateGame Workflow
// =============================================================================

pub fn create_game<'a, R, E, C, G>(
    repository: &'a R,
    event_store: &'a E,
    cache: &'a C,
    random: &'a G,
) -> impl Fn(CreateGameCommand) -> AsyncIO<WorkflowResult<R::GameSession>> + 'a
where
    R: GameSessionRepository,
    R::GameSession: SessionStateAccessor,
    E: EventStore,
    C: SessionCache<GameSession = R::GameSession>,
    G: RandomGenerator,
{
    move |command| {
        // Clone dependencies for use in AsyncIO closures (they require 'static)
        let repository = repository.clone();
        let repository_for_load = repository.clone();
        let event_store = event_store.clone();
        let cache = cache.clone();
        let random = random.clone();

        pipe_async!(
            AsyncIO::pure(command),
            => extract_seed,                                         // Pure: Command -> Option<Seed>
            =>> generate_seed_io(random),                            // IO: Option<Seed> -> AsyncIO<Seed>
            => create_identifiers,                                   // Pure: Seed -> (GameId, RandomSeed)
            => create_session_data,                                  // Pure
            => generate_game_started_event,                          // Pure
            => wrap_event_in_list,                                   // Pure
            =>> check_no_existing_session_io(repository),            // IO
            =>> append_events_io(event_store),                       // IO
            =>> load_created_session_io(repository_for_load),        // IO
            =>> cache_session_io(cache, DEFAULT_CACHE_TIME_TO_LIVE), // IO
        )
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use roguelike_domain::common::TurnCount;
    use roguelike_domain::enemy::Enemy;
    use roguelike_domain::floor::Floor;
    use roguelike_domain::game_session::GameOutcome;
    use roguelike_domain::player::Player;
    use rstest::rstest;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, RwLock};

    // =========================================================================
    // Mock Implementations
    // =========================================================================

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct MockGameSession {
        identifier: GameIdentifier,
        seed: RandomSeed,
    }

    impl MockGameSession {
        fn new(identifier: GameIdentifier, seed: RandomSeed) -> Self {
            Self { identifier, seed }
        }
    }

    impl SessionStateAccessor for MockGameSession {
        fn status(&self) -> roguelike_domain::game_session::GameStatus {
            roguelike_domain::game_session::GameStatus::InProgress
        }

        fn identifier(&self) -> &GameIdentifier {
            &self.identifier
        }

        fn event_sequence(&self) -> u64 {
            1
        }

        fn apply_event(&self, _event: &GameSessionEvent) -> Self {
            self.clone()
        }

        fn player(&self) -> &Player {
            unimplemented!("MockGameSession does not contain Player")
        }

        fn current_floor(&self) -> &Floor {
            unimplemented!("MockGameSession does not contain Floor")
        }

        fn enemies(&self) -> &[Enemy] {
            unimplemented!("MockGameSession does not contain Enemies")
        }

        fn turn_count(&self) -> TurnCount {
            TurnCount::zero()
        }

        fn seed(&self) -> &RandomSeed {
            &self.seed
        }

        fn with_player(&self, _player: Player) -> Self {
            self.clone()
        }

        fn with_floor(&self, _floor: Floor) -> Self {
            self.clone()
        }

        fn with_enemies(&self, _enemies: Vec<Enemy>) -> Self {
            self.clone()
        }

        fn increment_turn(&self) -> Self {
            self.clone()
        }

        fn end_game(&self, _outcome: GameOutcome) -> Self {
            self.clone()
        }
    }

    #[derive(Clone)]
    struct MockGameSessionRepository {
        sessions: Arc<RwLock<HashMap<GameIdentifier, MockGameSession>>>,
        events: Arc<RwLock<HashMap<GameIdentifier, Vec<GameSessionEvent>>>>,
    }

    impl MockGameSessionRepository {
        fn with_events(
            events: Arc<RwLock<HashMap<GameIdentifier, Vec<GameSessionEvent>>>>,
        ) -> Self {
            Self {
                sessions: Arc::new(RwLock::new(HashMap::new())),
                events,
            }
        }
    }

    impl GameSessionRepository for MockGameSessionRepository {
        type GameSession = MockGameSession;

        fn find_by_id(&self, identifier: &GameIdentifier) -> AsyncIO<Option<Self::GameSession>> {
            let sessions = Arc::clone(&self.sessions);
            let events = Arc::clone(&self.events);
            let identifier = *identifier;
            AsyncIO::new(move || async move {
                if let Some(session) = sessions.read().unwrap().get(&identifier).cloned() {
                    return Some(session);
                }
                if let Some(event_list) = events.read().unwrap().get(&identifier) {
                    for event in event_list {
                        if let GameSessionEvent::Started(started) = event {
                            let session = MockGameSession::new(identifier, *started.seed());
                            return Some(session);
                        }
                    }
                }
                None
            })
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
            AsyncIO::new(move || async move { sessions.read().unwrap().keys().copied().collect() })
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

        fn events_arc(&self) -> Arc<RwLock<HashMap<GameIdentifier, Vec<GameSessionEvent>>>> {
            Arc::clone(&self.events)
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
        cache: Arc<RwLock<HashMap<GameIdentifier, MockGameSession>>>,
    }

    impl MockSessionCache {
        fn new() -> Self {
            Self {
                cache: Arc::new(RwLock::new(HashMap::new())),
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
            let next_value = seed.value().wrapping_mul(1103515245).wrapping_add(12345);
            let random_value = (next_value >> 16) as u32;
            (random_value, RandomSeed::new(next_value))
        }
    }

    // =========================================================================
    // Pure Function Tests
    // =========================================================================

    mod pure_functions {
        use super::*;

        #[rstest]
        fn create_identifiers_returns_game_id_and_seed() {
            let seed = RandomSeed::new(42);
            let (game_id, returned_seed) = create_identifiers(seed);
            assert!(!game_id.to_string().is_empty());
            assert_eq!(returned_seed, seed);
        }

        #[rstest]
        fn generate_game_started_event_creates_event() {
            let game_id = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let (returned_id, event) = generate_game_started_event((game_id, seed));
            assert_eq!(returned_id, game_id);
            assert_eq!(event.game_identifier(), &game_id);
            assert_eq!(event.seed(), &seed);
        }

        #[rstest]
        fn wrap_event_in_list_wraps_correctly() {
            let game_id = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let event = GameStarted::new(game_id, seed);
            let (returned_id, events) = wrap_event_in_list((game_id, event));
            assert_eq!(returned_id, game_id);
            assert_eq!(events.len(), 1);
            assert!(events[0].is_game_started());
        }

        #[rstest]
        fn pipe_composes_pure_functions() {
            use lambars::pipe;
            let seed = RandomSeed::new(42);
            let (game_id, events) = pipe!(
                seed,
                create_identifiers,
                create_session_data,
                generate_game_started_event,
                wrap_event_in_list
            );
            assert!(!game_id.to_string().is_empty());
            assert_eq!(events.len(), 1);
            assert!(events[0].is_game_started());
        }
    }

    // =========================================================================
    // Workflow Tests
    // =========================================================================

    mod workflow {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn create_game_with_provided_seed() {
            let event_store = MockEventStore::new();
            let repository = MockGameSessionRepository::with_events(event_store.events_arc());
            let cache = MockSessionCache::new();
            let random = MockRandomGenerator::new();

            let workflow = create_game(&repository, &event_store, &cache, &random);
            let seed = RandomSeed::new(42);
            let command = CreateGameCommand::with_seed("Hero".to_string(), seed);

            let result = workflow(command).run_async().await;

            assert!(result.is_ok());
            let session = result.unwrap();
            assert_eq!(session.seed, seed);
        }

        #[rstest]
        #[tokio::test]
        async fn create_game_generates_seed_when_not_provided() {
            let event_store = MockEventStore::new();
            let repository = MockGameSessionRepository::with_events(event_store.events_arc());
            let cache = MockSessionCache::new();
            let random = MockRandomGenerator::new();

            let workflow = create_game(&repository, &event_store, &cache, &random);
            let command = CreateGameCommand::new("Hero".to_string(), None);

            let result = workflow(command).run_async().await;

            assert!(result.is_ok());
            let session = result.unwrap();
            assert_eq!(session.seed, RandomSeed::new(1));
        }

        #[rstest]
        #[tokio::test]
        async fn create_game_appends_event_to_store() {
            let event_store = MockEventStore::new();
            let repository = MockGameSessionRepository::with_events(event_store.events_arc());
            let cache = MockSessionCache::new();
            let random = MockRandomGenerator::new();

            let workflow = create_game(&repository, &event_store, &cache, &random);
            let seed = RandomSeed::new(42);
            let command = CreateGameCommand::with_seed("Hero".to_string(), seed);

            let result = workflow(command).run_async().await;

            assert!(result.is_ok());
            let session = result.unwrap();

            let events = event_store
                .load_events(&session.identifier)
                .run_async()
                .await;
            assert_eq!(events.len(), 1);
            assert!(events[0].is_game_started());
        }

        #[rstest]
        #[tokio::test]
        async fn create_game_caches_session() {
            let event_store = MockEventStore::new();
            let repository = MockGameSessionRepository::with_events(event_store.events_arc());
            let cache = MockSessionCache::new();
            let random = MockRandomGenerator::new();

            let workflow = create_game(&repository, &event_store, &cache, &random);
            let command = CreateGameCommand::with_seed("Hero".to_string(), RandomSeed::new(42));

            let result = workflow(command).run_async().await;

            assert!(result.is_ok());
            let session = result.unwrap();

            let cached = cache.get(&session.identifier).run_async().await;
            assert!(cached.is_some());
            assert_eq!(cached.unwrap(), session);
        }
    }
}
