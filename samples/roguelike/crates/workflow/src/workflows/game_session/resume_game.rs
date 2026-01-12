use std::time::Duration;

use lambars::effect::AsyncIO;
use lambars::pipe_async;
use roguelike_domain::common::TurnCount;
use roguelike_domain::enemy::Enemy;
use roguelike_domain::floor::Floor;
use roguelike_domain::game_session::{
    GameIdentifier, GameOutcome, GameSessionEvent, GameStatus, RandomSeed,
};
use roguelike_domain::player::Player;

use super::ResumeGameCommand;
use crate::errors::WorkflowError;
use crate::ports::{
    EventStore, GameSessionRepository, SessionCache, SnapshotStore, WorkflowResult,
};

// =============================================================================
// Workflow Configuration
// =============================================================================

const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300); // 5 minutes

// =============================================================================
// Step 1: Extract Game Identifier [Pure]
// =============================================================================

fn extract_game_identifier(command: ResumeGameCommand) -> GameIdentifier {
    *command.game_identifier()
}

// =============================================================================
// Step 2: Get From Cache Or Load [IO]
// =============================================================================

type SessionWithCacheStatus<S> = (S, bool);

#[allow(clippy::type_complexity)]
fn get_from_cache_or_load<C, S, E, R>(
    cache: C,
    snapshot_store: S,
    event_store: E,
    repository: R,
) -> impl Fn(GameIdentifier) -> AsyncIO<Option<SessionWithCacheStatus<C::GameSession>>>
where
    C: SessionCache,
    S: SnapshotStore<GameSession = C::GameSession>,
    E: EventStore,
    R: GameSessionRepository<GameSession = C::GameSession>,
    C::GameSession: SessionStateAccessor,
{
    move |game_identifier| {
        let cache = cache.clone();
        let snapshot_store = snapshot_store.clone();
        let event_store = event_store.clone();
        let repository = repository.clone();

        cache.get(&game_identifier).flat_map(move |cached| {
            match cached {
                Some(session) => {
                    // Cache hit - return with cache status true
                    AsyncIO::pure(Some((session, true)))
                }
                None => {
                    // Cache miss - load from snapshot and events
                    load_from_snapshot_and_events(
                        snapshot_store,
                        event_store,
                        repository,
                        game_identifier,
                    )
                }
            }
        })
    }
}

fn load_from_snapshot_and_events<S, E, R, Session>(
    snapshot_store: S,
    event_store: E,
    repository: R,
    game_identifier: GameIdentifier,
) -> AsyncIO<Option<SessionWithCacheStatus<Session>>>
where
    S: SnapshotStore<GameSession = Session>,
    E: EventStore,
    R: GameSessionRepository<GameSession = Session>,
    Session: SessionStateAccessor,
{
    snapshot_store
        .load_latest_snapshot(&game_identifier)
        .flat_map(move |snapshot_option| {
            let event_store = event_store.clone();
            let repository = repository.clone();

            // Get sequence number for event loading
            let since_sequence = snapshot_option.as_ref().map(|(_, seq)| *seq).unwrap_or(0);

            event_store
                .load_events_since(&game_identifier, since_sequence)
                .flat_map(move |events| {
                    reconstruct_session(snapshot_option, events, game_identifier, repository)
                })
        })
}

fn reconstruct_session<R, Session>(
    snapshot_option: Option<(Session, u64)>,
    events: Vec<GameSessionEvent>,
    game_identifier: GameIdentifier,
    repository: R,
) -> AsyncIO<Option<SessionWithCacheStatus<Session>>>
where
    R: GameSessionRepository<GameSession = Session>,
    Session: SessionStateAccessor,
{
    if let Some((base_state, _sequence)) = snapshot_option {
        // Reconstruct from snapshot + events
        let reconstructed = reconstruct_from_events_internal(base_state, &events);
        AsyncIO::pure(Some((reconstructed, false)))
    } else if !events.is_empty() {
        // No snapshot but we have events - try to get from repository
        repository
            .find_by_id(&game_identifier)
            .fmap(move |opt| opt.map(|session| (session, false)))
    } else {
        // No snapshot and no events - session doesn't exist
        AsyncIO::pure(None)
    }
}

// =============================================================================
// Step 3: Validate And Cache Session [IO]
// =============================================================================

#[allow(clippy::type_complexity)]
fn validate_and_cache_session<C>(
    cache: C,
) -> impl Fn(Option<SessionWithCacheStatus<C::GameSession>>) -> AsyncIO<WorkflowResult<C::GameSession>>
where
    C: SessionCache,
    C::GameSession: SessionStateAccessor,
{
    move |session_option| {
        let cache = cache.clone();

        match session_option {
            Some((session, was_cached)) => {
                // Validate session is active
                match validate_session_active(&session) {
                    Ok(()) => {
                        if was_cached {
                            // Already cached, just return
                            AsyncIO::pure(Ok(session))
                        } else {
                            // Not cached, cache it first
                            let session_clone = session.clone();
                            let game_identifier = *session.identifier();
                            cache
                                .set(&game_identifier, &session, DEFAULT_CACHE_TIME_TO_LIVE)
                                .fmap(move |()| Ok(session_clone))
                        }
                    }
                    Err(error) => AsyncIO::pure(Err(error)),
                }
            }
            None => {
                // Session not found
                AsyncIO::pure(Err(WorkflowError::not_found(
                    "GameSession",
                    "unknown".to_string(),
                )))
            }
        }
    }
}

// =============================================================================
// ResumeGame Workflow
// =============================================================================

pub fn resume_game<'a, R, E, S, C>(
    repository: &'a R,
    event_store: &'a E,
    snapshot_store: &'a S,
    cache: &'a C,
) -> impl Fn(ResumeGameCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
where
    R: GameSessionRepository<GameSession = C::GameSession>,
    E: EventStore,
    S: SnapshotStore<GameSession = C::GameSession>,
    C: SessionCache,
    C::GameSession: SessionStateAccessor,
{
    move |command| {
        // Clone dependencies for use in AsyncIO closures (they require 'static)
        let repository = repository.clone();
        let event_store = event_store.clone();
        let snapshot_store = snapshot_store.clone();
        let cache = cache.clone();
        let cache_for_set = cache.clone();

        pipe_async!(
            AsyncIO::pure(command),
            => extract_game_identifier,                                                  // Pure
            =>> get_from_cache_or_load(cache, snapshot_store, event_store, repository),  // IO
            =>> validate_and_cache_session(cache_for_set),                               // IO
        )
    }
}

// =============================================================================
// Session State Accessor Trait
// =============================================================================

pub trait SessionStateAccessor: Clone + Send + Sync + 'static {
    fn status(&self) -> GameStatus;

    fn identifier(&self) -> &GameIdentifier;

    fn event_sequence(&self) -> u64;

    fn apply_event(&self, event: &GameSessionEvent) -> Self;

    fn player(&self) -> &Player;

    fn current_floor(&self) -> &Floor;

    fn enemies(&self) -> &[Enemy];

    fn turn_count(&self) -> TurnCount;

    fn seed(&self) -> &RandomSeed;

    fn with_player(&self, player: Player) -> Self;

    fn with_floor(&self, floor: Floor) -> Self;

    fn with_enemies(&self, enemies: Vec<Enemy>) -> Self;

    fn increment_turn(&self) -> Self;

    fn end_game(&self, outcome: GameOutcome) -> Self;
}

// =============================================================================
// Pure Functions
// =============================================================================

pub fn reconstruct_from_events<S>(initial: S, events: &[GameSessionEvent]) -> S
where
    S: SessionStateAccessor,
{
    reconstruct_from_events_internal(initial, events)
}

fn reconstruct_from_events_internal<S>(initial: S, events: &[GameSessionEvent]) -> S
where
    S: SessionStateAccessor,
{
    events
        .iter()
        .fold(initial, |state, event| state.apply_event(event))
}

fn validate_session_active<S>(session: &S) -> WorkflowResult<()>
where
    S: SessionStateAccessor,
{
    let status = session.status();
    if status.is_active() {
        Ok(())
    } else {
        Err(WorkflowError::Domain(
            roguelike_domain::common::DomainError::GameSession(
                roguelike_domain::game_session::GameSessionError::session_already_completed(),
            ),
        ))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use roguelike_domain::game_session::{GameEnded, GameStarted, RandomSeed};
    use rstest::rstest;
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    // =========================================================================
    // Mock Implementations
    // =========================================================================

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct MockGameSession {
        identifier: GameIdentifier,
        seed: RandomSeed,
        status: GameStatus,
        event_sequence: u64,
    }

    impl MockGameSession {
        fn new(identifier: GameIdentifier, seed: RandomSeed) -> Self {
            Self {
                identifier,
                seed,
                status: GameStatus::InProgress,
                event_sequence: 1,
            }
        }

        fn with_status(identifier: GameIdentifier, seed: RandomSeed, status: GameStatus) -> Self {
            Self {
                identifier,
                seed,
                status,
                event_sequence: 1,
            }
        }
    }

    impl SessionStateAccessor for MockGameSession {
        fn status(&self) -> GameStatus {
            self.status
        }

        fn identifier(&self) -> &GameIdentifier {
            &self.identifier
        }

        fn event_sequence(&self) -> u64 {
            self.event_sequence
        }

        fn apply_event(&self, event: &GameSessionEvent) -> Self {
            let mut new_session = self.clone();
            new_session.event_sequence += 1;
            if let GameSessionEvent::Ended(ended) = event {
                new_session.status = ended.outcome().to_status();
            }
            new_session
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

        fn end_game(&self, outcome: GameOutcome) -> Self {
            Self {
                status: outcome.to_status(),
                ..self.clone()
            }
        }
    }

    #[derive(Clone)]
    struct MockGameSessionRepository {
        sessions: Arc<RwLock<HashMap<GameIdentifier, MockGameSession>>>,
    }

    impl MockGameSessionRepository {
        fn new() -> Self {
            Self {
                sessions: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        fn with_session(session: MockGameSession) -> Self {
            let repo = Self::new();
            repo.sessions
                .write()
                .unwrap()
                .insert(session.identifier, session);
            repo
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

        fn with_events(identifier: GameIdentifier, events: Vec<GameSessionEvent>) -> Self {
            let store = Self::new();
            store.events.write().unwrap().insert(identifier, events);
            store
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
    struct MockSnapshotStore {
        snapshots: Arc<RwLock<HashMap<GameIdentifier, (MockGameSession, u64)>>>,
    }

    impl MockSnapshotStore {
        fn new() -> Self {
            Self {
                snapshots: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        fn with_snapshot(
            identifier: GameIdentifier,
            session: MockGameSession,
            sequence: u64,
        ) -> Self {
            let store = Self::new();
            store
                .snapshots
                .write()
                .unwrap()
                .insert(identifier, (session, sequence));
            store
        }
    }

    impl SnapshotStore for MockSnapshotStore {
        type GameSession = MockGameSession;

        fn save_snapshot(
            &self,
            session_identifier: &GameIdentifier,
            state: &Self::GameSession,
            sequence: u64,
        ) -> AsyncIO<()> {
            let snapshots = Arc::clone(&self.snapshots);
            let session_identifier = *session_identifier;
            let state = state.clone();
            AsyncIO::new(move || async move {
                snapshots
                    .write()
                    .unwrap()
                    .insert(session_identifier, (state, sequence));
            })
        }

        fn load_latest_snapshot(
            &self,
            session_identifier: &GameIdentifier,
        ) -> AsyncIO<Option<(Self::GameSession, u64)>> {
            let snapshots = Arc::clone(&self.snapshots);
            let session_identifier = *session_identifier;
            AsyncIO::new(move || async move {
                snapshots.read().unwrap().get(&session_identifier).cloned()
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

        fn with_session(session: MockGameSession) -> Self {
            let cache = Self::new();
            cache
                .cache
                .write()
                .unwrap()
                .insert(session.identifier, session);
            cache
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

    // =========================================================================
    // Pure Function Tests
    // =========================================================================

    mod pure_functions {
        use super::*;

        #[rstest]
        fn extract_game_identifier_returns_identifier() {
            let identifier = GameIdentifier::new();
            let command = ResumeGameCommand::new(identifier);

            let result = extract_game_identifier(command);

            assert_eq!(result, identifier);
        }

        #[rstest]
        fn reconstruct_from_events_with_no_events() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let initial = MockGameSession::new(identifier, seed);

            let reconstructed = reconstruct_from_events(initial.clone(), &[]);

            assert_eq!(reconstructed, initial);
        }

        #[rstest]
        fn reconstruct_from_events_applies_events() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let initial = MockGameSession::new(identifier, seed);
            let ended_event = GameSessionEvent::Ended(GameEnded::victory());

            let reconstructed = reconstruct_from_events(initial, &[ended_event]);

            assert_eq!(reconstructed.status, GameStatus::Victory);
            assert_eq!(reconstructed.event_sequence, 2);
        }

        #[rstest]
        fn validate_session_active_accepts_in_progress() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::with_status(identifier, seed, GameStatus::InProgress);

            let result = validate_session_active(&session);

            assert!(result.is_ok());
        }

        #[rstest]
        fn validate_session_active_accepts_paused() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::with_status(identifier, seed, GameStatus::Paused);

            let result = validate_session_active(&session);

            assert!(result.is_ok());
        }

        #[rstest]
        fn validate_session_active_rejects_victory() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::with_status(identifier, seed, GameStatus::Victory);

            let result = validate_session_active(&session);

            assert!(result.is_err());
        }

        #[rstest]
        fn validate_session_active_rejects_defeat() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::with_status(identifier, seed, GameStatus::Defeat);

            let result = validate_session_active(&session);

            assert!(result.is_err());
        }
    }

    // =========================================================================
    // Workflow Tests
    // =========================================================================

    mod workflow {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn resume_game_from_cache() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::new(identifier, seed);

            let repository = MockGameSessionRepository::new();
            let event_store = MockEventStore::new();
            let snapshot_store = MockSnapshotStore::new();
            let cache = MockSessionCache::with_session(session.clone());

            let workflow = resume_game(&repository, &event_store, &snapshot_store, &cache);
            let command = ResumeGameCommand::new(identifier);

            let result = workflow(command).run_async().await;

            assert!(result.is_ok());
            assert_eq!(result.unwrap(), session);
        }

        #[rstest]
        #[tokio::test]
        async fn resume_game_from_snapshot() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::new(identifier, seed);

            let repository = MockGameSessionRepository::with_session(session.clone());
            let event_store = MockEventStore::with_events(
                identifier,
                vec![GameSessionEvent::Started(GameStarted::new(
                    identifier, seed,
                ))],
            );
            let snapshot_store = MockSnapshotStore::with_snapshot(identifier, session.clone(), 1);
            let cache = MockSessionCache::new();

            let workflow = resume_game(&repository, &event_store, &snapshot_store, &cache);
            let command = ResumeGameCommand::new(identifier);

            let result = workflow(command).run_async().await;

            assert!(result.is_ok());
        }

        #[rstest]
        #[tokio::test]
        async fn resume_game_not_found() {
            let identifier = GameIdentifier::new();

            let repository = MockGameSessionRepository::new();
            let event_store = MockEventStore::new();
            let snapshot_store = MockSnapshotStore::new();
            let cache = MockSessionCache::new();

            let workflow = resume_game(&repository, &event_store, &snapshot_store, &cache);
            let command = ResumeGameCommand::new(identifier);

            let result = workflow(command).run_async().await;

            assert!(result.is_err());
            assert!(result.unwrap_err().is_not_found());
        }

        #[rstest]
        #[tokio::test]
        async fn resume_game_completed_session_fails() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::with_status(identifier, seed, GameStatus::Victory);

            let repository = MockGameSessionRepository::new();
            let event_store = MockEventStore::new();
            let snapshot_store = MockSnapshotStore::new();
            let cache = MockSessionCache::with_session(session);

            let workflow = resume_game(&repository, &event_store, &snapshot_store, &cache);
            let command = ResumeGameCommand::new(identifier);

            let result = workflow(command).run_async().await;

            assert!(result.is_err());
            assert!(result.unwrap_err().is_domain());
        }

        #[rstest]
        #[tokio::test]
        async fn resume_game_caches_reconstructed_session() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::new(identifier, seed);

            let repository = MockGameSessionRepository::with_session(session.clone());
            let event_store = MockEventStore::with_events(
                identifier,
                vec![GameSessionEvent::Started(GameStarted::new(
                    identifier, seed,
                ))],
            );
            let snapshot_store = MockSnapshotStore::with_snapshot(identifier, session.clone(), 1);
            let cache = MockSessionCache::new();

            let workflow = resume_game(&repository, &event_store, &snapshot_store, &cache);
            let command = ResumeGameCommand::new(identifier);

            let _ = workflow(command).run_async().await;

            // Verify session was cached
            let cached = cache.get(&identifier).run_async().await;
            assert!(cached.is_some());
        }
    }
}
