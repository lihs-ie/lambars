//! CreateGame workflow implementation.
//!
//! This module provides the workflow for creating new game sessions.
//! The workflow is composed using `pipe!` macro with independent named functions.
//!
//! # Workflow Steps (from design document)
//!
//! 1. [IO] Generate Seed - シードが指定されていない場合、乱数シードを生成
//! 2. [Pure] Create Identifiers - ゲームとプレイヤーの識別子を生成
//! 3. [Pure] Generate Initial Floor - 最初のダンジョンフロアを生成
//! 4. [Pure] Create Player - 初期プレイヤーを作成
//! 5. [Pure] Create GameSession - ゲームセッションを作成
//! 6. [Pure] Generate Events - GameStarted イベントを生成
//! 7. [IO] Persist Session - セッションをリポジトリに保存
//! 8. [IO] Append Events - イベントをイベントストアに追加
//! 9. [IO] Cache Session - セッションをキャッシュに保存
//!
//! # Examples
//!
//! ```ignore
//! use roguelike_workflow::workflows::game_session::{create_game, CreateGameCommand};
//!
//! let workflow = create_game(&repository, &event_store, &cache, &random);
//! let command = CreateGameCommand::new("Hero".to_string(), None);
//! let result = workflow(command).run_async().await;
//! ```

use std::time::Duration;

use lambars::effect::AsyncIO;
use lambars::pipe;
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

/// Default cache time-to-live for game sessions.
const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300); // 5 minutes

// =============================================================================
// Step 1: Generate Seed [IO]
// =============================================================================

/// Generates or uses provided seed.
///
/// This is the first step of the workflow that returns an AsyncIO
/// containing the random seed.
fn generate_seed<G: RandomGenerator>(
    random: &G,
    provided_seed: Option<RandomSeed>,
) -> AsyncIO<RandomSeed> {
    match provided_seed {
        Some(seed) => AsyncIO::pure(seed),
        None => random.generate_seed(),
    }
}

// =============================================================================
// Step 2: Create Identifiers [Pure]
// =============================================================================

/// Creates game and player identifiers from seed.
///
/// Input: RandomSeed
/// Output: (GameIdentifier, RandomSeed)
fn create_identifiers(seed: RandomSeed) -> (GameIdentifier, RandomSeed) {
    let game_identifier = GameIdentifier::new();
    (game_identifier, seed)
}

// =============================================================================
// Step 3-5: Create GameSession [Pure]
// Note: Floor/Player creation is simplified for current domain model
// =============================================================================

/// Creates game session data from identifiers.
///
/// Input: (GameIdentifier, RandomSeed)
/// Output: (GameIdentifier, RandomSeed) - passed through for event generation
fn create_session_data(input: (GameIdentifier, RandomSeed)) -> (GameIdentifier, RandomSeed) {
    // In full implementation, this would create Floor, Player, and GameSession
    // For now, we pass through the identifiers
    input
}

// =============================================================================
// Step 6: Generate Events [Pure]
// =============================================================================

/// Generates GameStarted event from session data.
///
/// Input: (GameIdentifier, RandomSeed)
/// Output: (GameIdentifier, GameStarted)
fn generate_game_started_event(
    input: (GameIdentifier, RandomSeed),
) -> (GameIdentifier, GameStarted) {
    let (game_identifier, seed) = input;
    let event = GameStarted::new(game_identifier, seed);
    (game_identifier, event)
}

/// Wraps event into event list for persistence.
///
/// Input: (GameIdentifier, GameStarted)
/// Output: (GameIdentifier, Vec<GameSessionEvent>)
fn wrap_event_in_list(
    input: (GameIdentifier, GameStarted),
) -> (GameIdentifier, Vec<GameSessionEvent>) {
    let (game_identifier, event) = input;
    (game_identifier, vec![GameSessionEvent::from(event)])
}

// =============================================================================
// Step 7: Persist Session [IO]
// =============================================================================

/// Creates a function that checks for existing session.
///
/// Returns a function suitable for use in pipe! that transforms
/// AsyncIO<(GameIdentifier, Vec<GameSessionEvent>)> to AsyncIO<...>
fn check_no_existing_session<R: GameSessionRepository>(
    repository: &R,
) -> impl Fn(
    (GameIdentifier, Vec<GameSessionEvent>),
) -> AsyncIO<Result<(GameIdentifier, Vec<GameSessionEvent>), WorkflowError>>
       + '_ {
    move |(game_identifier, events)| {
        let events_clone = events.clone();
        repository.find_by_id(&game_identifier).fmap(move |existing| {
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
// Step 8: Append Events [IO]
// =============================================================================

/// Creates a function that appends events to event store.
fn append_events<E: EventStore>(
    event_store: &E,
) -> impl Fn(
    AsyncIO<Result<(GameIdentifier, Vec<GameSessionEvent>), WorkflowError>>,
) -> AsyncIO<Result<GameIdentifier, WorkflowError>>
       + '_ {
    move |io| {
        let event_store = event_store.clone();
        io.flat_map(move |result| match result {
            Err(e) => AsyncIO::pure(Err(e)),
            Ok((game_identifier, events)) => event_store
                .append(&game_identifier, &events)
                .fmap(move |()| Ok(game_identifier)),
        })
    }
}

// =============================================================================
// Step 9: Load and Cache Session [IO]
// =============================================================================

/// Creates a function that loads created session from repository.
fn load_created_session<R: GameSessionRepository>(
    repository: &R,
) -> impl Fn(
    AsyncIO<Result<GameIdentifier, WorkflowError>>,
) -> AsyncIO<Result<(GameIdentifier, R::GameSession), WorkflowError>>
       + '_ {
    move |io| {
        let repository = repository.clone();
        io.flat_map(move |result| match result {
            Err(e) => AsyncIO::pure(Err(e)),
            Ok(game_identifier) => repository.find_by_id(&game_identifier).fmap(move |opt| {
                opt.map(|session| (game_identifier, session))
                    .ok_or_else(|| {
                        WorkflowError::repository("save", "Failed to create game session from events")
                    })
            }),
        })
    }
}

/// Creates a function that caches the session.
fn cache_session<C: SessionCache>(
    cache: &C,
    ttl: Duration,
) -> impl Fn(
    AsyncIO<Result<(GameIdentifier, C::GameSession), WorkflowError>>,
) -> AsyncIO<WorkflowResult<C::GameSession>>
       + '_ {
    move |io| {
        let cache = cache.clone();
        io.flat_map(move |result| match result {
            Err(e) => AsyncIO::pure(Err(e)),
            Ok((game_identifier, session)) => {
                let session_clone = session.clone();
                cache
                    .set(&game_identifier, &session, ttl)
                    .fmap(move |()| Ok(session_clone))
            }
        })
    }
}

// =============================================================================
// CreateGame Workflow
// =============================================================================

/// Creates a workflow function for creating new game sessions.
///
/// The workflow is composed using `pipe!` macro with independent named functions:
///
/// ```text
/// pipe!(
///     command,
///     generate_seed,
///     create_identifiers,
///     create_session_data,
///     generate_game_started_event,
///     wrap_event_in_list,
///     check_no_existing_session,
///     append_events,
///     load_created_session,
///     cache_session
/// )
/// ```
///
/// # Type Parameters
///
/// * `R` - Repository type implementing `GameSessionRepository`
/// * `E` - Event store type implementing `EventStore`
/// * `C` - Cache type implementing `SessionCache`
/// * `G` - Random generator type implementing `RandomGenerator`
///
/// # Arguments
///
/// * `repository` - The game session repository for persistence
/// * `event_store` - The event store for event sourcing
/// * `cache` - The session cache for fast access
/// * `random` - The random generator for seed generation
///
/// # Returns
///
/// A function that takes a `CreateGameCommand` and returns an `AsyncIO`
/// that produces the created game session or an error.
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
        // Clone dependencies for use in closures
        let repository = repository.clone();
        let event_store = event_store.clone();
        let cache = cache.clone();

        // Step 1: [IO] Generate seed
        let seed_io = generate_seed(random, command.seed());

        // Steps 2-9: Composed using pipe!
        seed_io.flat_map(move |seed| {
            // [Pure] Steps 2-6: Domain logic pipeline
            let (game_identifier, events) = pipe!(
                seed,
                create_identifiers,
                create_session_data,
                generate_game_started_event,
                wrap_event_in_list
            );

            // [IO] Steps 7-9: Persistence pipeline
            pipe!(
                check_no_existing_session(&repository)((game_identifier, events)),
                append_events(&event_store),
                load_created_session(&repository),
                cache_session(&cache, DEFAULT_CACHE_TIME_TO_LIVE)
            )
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
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
