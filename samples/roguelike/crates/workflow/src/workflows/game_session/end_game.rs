use lambars::effect::AsyncIO;
use lambars::pipe_async;
use roguelike_domain::game_session::{GameEnded, GameIdentifier, GameOutcome, GameSessionEvent};

use super::EndGameCommand;
use super::resume_game::SessionStateAccessor;
use crate::errors::WorkflowError;
use crate::ports::{
    EventStore, GameSessionRepository, SessionCache, SnapshotStore, WorkflowResult,
};

// =============================================================================
// Step 1: Extract End Game Parameters [Pure]
// =============================================================================

fn extract_end_game_params(command: EndGameCommand) -> (GameIdentifier, GameOutcome) {
    (*command.game_identifier(), *command.outcome())
}

// =============================================================================
// Step 2: Load Session [IO]
// =============================================================================

#[allow(clippy::type_complexity)]
fn load_session<C, R>(
    cache: C,
    repository: R,
) -> impl Fn(
    (GameIdentifier, GameOutcome),
) -> AsyncIO<Result<(C::GameSession, GameOutcome), WorkflowError>>
where
    C: SessionCache,
    R: GameSessionRepository<GameSession = C::GameSession>,
{
    move |(game_identifier, outcome)| {
        let cache = cache.clone();
        let repository = repository.clone();

        cache.get(&game_identifier).flat_map(move |cached| {
            match cached {
                Some(session) => AsyncIO::pure(Ok((session, outcome))),
                None => {
                    // Cache miss - load from repository
                    repository.find_by_id(&game_identifier).fmap(move |opt| {
                        opt.map(|session| (session, outcome)).ok_or_else(|| {
                            WorkflowError::not_found("GameSession", game_identifier.to_string())
                        })
                    })
                }
            }
        })
    }
}

// =============================================================================
// Step 3: Validate and Update Session [Pure]
// =============================================================================

#[allow(clippy::type_complexity)]
fn validate_and_update_session<S>(
    result: Result<(S, GameOutcome), WorkflowError>,
) -> Result<(S, GameIdentifier, Vec<GameSessionEvent>), WorkflowError>
where
    S: SessionStateAccessor + SessionStatusUpdater,
{
    let (session, outcome) = result?;

    // Validate session can be ended
    validate_can_end(&session)?;

    // Generate event and update session
    let game_ended_event = create_game_ended_event(outcome);
    let events = wrap_game_ended_event(game_ended_event);
    let updated_session = update_session_status(&session, outcome);
    let game_identifier = *session.identifier();

    Ok((updated_session, game_identifier, events))
}

// =============================================================================
// Step 4: Persist End Game [IO]
// =============================================================================

#[allow(clippy::type_complexity)]
fn persist_end_game<R, E, SS, C>(
    repository: R,
    event_store: E,
    snapshot_store: SS,
    cache: C,
) -> impl Fn(
    Result<(C::GameSession, GameIdentifier, Vec<GameSessionEvent>), WorkflowError>,
) -> AsyncIO<WorkflowResult<C::GameSession>>
where
    R: GameSessionRepository<GameSession = C::GameSession>,
    E: EventStore,
    SS: SnapshotStore<GameSession = C::GameSession>,
    C: SessionCache,
    C::GameSession: SessionStateAccessor,
{
    move |result| match result {
        Err(error) => AsyncIO::pure(Err(error)),
        Ok((updated_session, game_identifier, events)) => {
            let repository = repository.clone();
            let event_store = event_store.clone();
            let snapshot_store = snapshot_store.clone();
            let cache = cache.clone();

            let event_sequence = updated_session.event_sequence();
            let final_session = updated_session.clone();
            let session_for_snapshot = updated_session.clone();

            repository
                .save(&updated_session)
                .flat_map(move |()| event_store.append(&game_identifier, &events))
                .flat_map(move |()| {
                    snapshot_store.save_snapshot(
                        &game_identifier,
                        &session_for_snapshot,
                        event_sequence,
                    )
                })
                .flat_map(move |()| cache.invalidate(&game_identifier))
                .fmap(move |()| Ok(final_session))
        }
    }
}

// =============================================================================
// EndGame Workflow
// =============================================================================

pub fn end_game<'a, R, E, S, C>(
    repository: &'a R,
    event_store: &'a E,
    snapshot_store: &'a S,
    cache: &'a C,
) -> impl Fn(EndGameCommand) -> AsyncIO<WorkflowResult<C::GameSession>> + 'a
where
    R: GameSessionRepository<GameSession = C::GameSession>,
    E: EventStore,
    S: SnapshotStore<GameSession = C::GameSession>,
    C: SessionCache,
    C::GameSession: SessionStateAccessor + SessionStatusUpdater,
{
    move |command| {
        // Clone dependencies for use in AsyncIO closures (they require 'static)
        let repository = repository.clone();
        let repository_for_save = repository.clone();
        let event_store = event_store.clone();
        let snapshot_store = snapshot_store.clone();
        let cache = cache.clone();
        let cache_for_invalidate = cache.clone();

        pipe_async!(
            AsyncIO::pure(command),
            => extract_end_game_params,                                                          // Pure: Command -> (GameId, Outcome)
            =>> load_session(cache, repository),                                                 // IO: (GameId, Outcome) -> AsyncIO<Result<(Session, Outcome), Error>>
            => validate_and_update_session,                                                      // Pure: Result<(Session, Outcome), Error> -> Result<(Session, GameId, Events), Error>
            =>> persist_end_game(repository_for_save, event_store, snapshot_store, cache_for_invalidate), // IO: Result -> AsyncIO<WorkflowResult<Session>>
        )
    }
}

// =============================================================================
// Session Status Updater Trait
// =============================================================================

pub trait SessionStatusUpdater: Clone + Send + Sync + 'static {
    fn with_outcome(&self, outcome: GameOutcome) -> Self;
}

// =============================================================================
// Pure Functions
// =============================================================================

#[must_use]
fn create_game_ended_event(outcome: GameOutcome) -> GameEnded {
    GameEnded::new(outcome)
}

#[must_use]
fn wrap_game_ended_event(event: GameEnded) -> Vec<GameSessionEvent> {
    vec![GameSessionEvent::from(event)]
}

fn validate_can_end<S>(session: &S) -> WorkflowResult<()>
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

fn update_session_status<S>(session: &S, outcome: GameOutcome) -> S
where
    S: SessionStatusUpdater,
{
    session.with_outcome(outcome)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use roguelike_domain::game_session::{GameIdentifier, GameStatus, RandomSeed};
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
    }

    impl SessionStatusUpdater for MockGameSession {
        fn with_outcome(&self, outcome: GameOutcome) -> Self {
            let mut new_session = self.clone();
            new_session.status = outcome.to_status();
            new_session.event_sequence += 1;
            new_session
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
            _time_to_live: std::time::Duration,
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
        fn extract_end_game_params_extracts_correctly() {
            let identifier = GameIdentifier::new();
            let command = EndGameCommand::victory(identifier);

            let (extracted_id, extracted_outcome) = extract_end_game_params(command);

            assert_eq!(extracted_id, identifier);
            assert_eq!(extracted_outcome, GameOutcome::Victory);
        }

        #[rstest]
        #[case(GameOutcome::Victory)]
        #[case(GameOutcome::Defeat)]
        #[case(GameOutcome::Abandoned)]
        fn extract_end_game_params_extracts_all_outcomes(#[case] outcome: GameOutcome) {
            let identifier = GameIdentifier::new();
            let command = EndGameCommand::new(identifier, outcome);

            let (_, extracted_outcome) = extract_end_game_params(command);

            assert_eq!(extracted_outcome, outcome);
        }

        #[rstest]
        #[case(GameOutcome::Victory)]
        #[case(GameOutcome::Defeat)]
        #[case(GameOutcome::Abandoned)]
        fn create_game_ended_event_creates_event(#[case] outcome: GameOutcome) {
            let event = create_game_ended_event(outcome);
            assert_eq!(event.outcome(), &outcome);
        }

        #[rstest]
        fn validate_can_end_accepts_in_progress() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::with_status(identifier, seed, GameStatus::InProgress);

            let result = validate_can_end(&session);

            assert!(result.is_ok());
        }

        #[rstest]
        fn validate_can_end_accepts_paused() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::with_status(identifier, seed, GameStatus::Paused);

            let result = validate_can_end(&session);

            assert!(result.is_ok());
        }

        #[rstest]
        fn validate_can_end_rejects_victory() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::with_status(identifier, seed, GameStatus::Victory);

            let result = validate_can_end(&session);

            assert!(result.is_err());
        }

        #[rstest]
        fn validate_can_end_rejects_defeat() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::with_status(identifier, seed, GameStatus::Defeat);

            let result = validate_can_end(&session);

            assert!(result.is_err());
        }

        #[rstest]
        fn update_session_status_updates_to_victory() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::new(identifier, seed);

            let updated = update_session_status(&session, GameOutcome::Victory);

            assert_eq!(updated.status, GameStatus::Victory);
        }

        #[rstest]
        fn update_session_status_updates_to_defeat() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::new(identifier, seed);

            let updated = update_session_status(&session, GameOutcome::Defeat);

            assert_eq!(updated.status, GameStatus::Defeat);
        }

        #[rstest]
        fn validate_and_update_session_with_valid_session() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::new(identifier, seed);

            let result = validate_and_update_session(Ok((session, GameOutcome::Victory)));

            assert!(result.is_ok());
            let (updated_session, game_id, events) = result.unwrap();
            assert_eq!(updated_session.status, GameStatus::Victory);
            assert_eq!(game_id, identifier);
            assert_eq!(events.len(), 1);
            assert!(events[0].is_game_ended());
        }

        #[rstest]
        fn validate_and_update_session_with_completed_session() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::with_status(identifier, seed, GameStatus::Victory);

            let result = validate_and_update_session(Ok((session, GameOutcome::Defeat)));

            assert!(result.is_err());
        }

        #[rstest]
        fn validate_and_update_session_propagates_error() {
            let error = WorkflowError::not_found("GameSession", "test".to_string());
            let result: Result<(MockGameSession, GameIdentifier, Vec<GameSessionEvent>), _> =
                validate_and_update_session(Err(error));

            assert!(result.is_err());
            assert!(result.unwrap_err().is_not_found());
        }
    }

    // =========================================================================
    // Workflow Tests
    // =========================================================================

    mod workflow {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn end_game_with_victory() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::new(identifier, seed);

            let repository = MockGameSessionRepository::with_session(session.clone());
            let event_store = MockEventStore::new();
            let snapshot_store = MockSnapshotStore::new();
            let cache = MockSessionCache::with_session(session);

            let workflow = end_game(&repository, &event_store, &snapshot_store, &cache);
            let command = EndGameCommand::victory(identifier);

            let result = workflow(command).run_async().await;

            assert!(result.is_ok());
            let ended_session = result.unwrap();
            assert_eq!(ended_session.status, GameStatus::Victory);
        }

        #[rstest]
        #[tokio::test]
        async fn end_game_with_defeat() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::new(identifier, seed);

            let repository = MockGameSessionRepository::with_session(session.clone());
            let event_store = MockEventStore::new();
            let snapshot_store = MockSnapshotStore::new();
            let cache = MockSessionCache::with_session(session);

            let workflow = end_game(&repository, &event_store, &snapshot_store, &cache);
            let command = EndGameCommand::defeat(identifier);

            let result = workflow(command).run_async().await;

            assert!(result.is_ok());
            let ended_session = result.unwrap();
            assert_eq!(ended_session.status, GameStatus::Defeat);
        }

        #[rstest]
        #[tokio::test]
        async fn end_game_appends_event() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::new(identifier, seed);

            let repository = MockGameSessionRepository::with_session(session.clone());
            let event_store = MockEventStore::new();
            let snapshot_store = MockSnapshotStore::new();
            let cache = MockSessionCache::with_session(session);

            let workflow = end_game(&repository, &event_store, &snapshot_store, &cache);
            let command = EndGameCommand::victory(identifier);

            let _ = workflow(command).run_async().await;

            // Verify event was appended
            let events = event_store.load_events(&identifier).run_async().await;
            assert_eq!(events.len(), 1);
            assert!(events[0].is_game_ended());
        }

        #[rstest]
        #[tokio::test]
        async fn end_game_creates_snapshot() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::new(identifier, seed);

            let repository = MockGameSessionRepository::with_session(session.clone());
            let event_store = MockEventStore::new();
            let snapshot_store = MockSnapshotStore::new();
            let cache = MockSessionCache::with_session(session);

            let workflow = end_game(&repository, &event_store, &snapshot_store, &cache);
            let command = EndGameCommand::victory(identifier);

            let _ = workflow(command).run_async().await;

            // Verify snapshot was created
            let snapshot = snapshot_store
                .load_latest_snapshot(&identifier)
                .run_async()
                .await;
            assert!(snapshot.is_some());
            let (snapshot_session, _) = snapshot.unwrap();
            assert_eq!(snapshot_session.status, GameStatus::Victory);
        }

        #[rstest]
        #[tokio::test]
        async fn end_game_invalidates_cache() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::new(identifier, seed);

            let repository = MockGameSessionRepository::with_session(session.clone());
            let event_store = MockEventStore::new();
            let snapshot_store = MockSnapshotStore::new();
            let cache = MockSessionCache::with_session(session);

            let workflow = end_game(&repository, &event_store, &snapshot_store, &cache);
            let command = EndGameCommand::victory(identifier);

            let _ = workflow(command).run_async().await;

            // Verify cache was invalidated
            let cached = cache.get(&identifier).run_async().await;
            assert!(cached.is_none());
        }

        #[rstest]
        #[tokio::test]
        async fn end_game_not_found() {
            let identifier = GameIdentifier::new();

            let repository = MockGameSessionRepository::new();
            let event_store = MockEventStore::new();
            let snapshot_store = MockSnapshotStore::new();
            let cache = MockSessionCache::new();

            let workflow = end_game(&repository, &event_store, &snapshot_store, &cache);
            let command = EndGameCommand::victory(identifier);

            let result = workflow(command).run_async().await;

            assert!(result.is_err());
            assert!(result.unwrap_err().is_not_found());
        }

        #[rstest]
        #[tokio::test]
        async fn end_game_already_completed() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::with_status(identifier, seed, GameStatus::Victory);

            let repository = MockGameSessionRepository::with_session(session.clone());
            let event_store = MockEventStore::new();
            let snapshot_store = MockSnapshotStore::new();
            let cache = MockSessionCache::with_session(session);

            let workflow = end_game(&repository, &event_store, &snapshot_store, &cache);
            let command = EndGameCommand::defeat(identifier);

            let result = workflow(command).run_async().await;

            assert!(result.is_err());
            assert!(result.unwrap_err().is_domain());
        }

        #[rstest]
        #[tokio::test]
        async fn end_game_loads_from_repository_when_not_cached() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::new(identifier, seed);

            let repository = MockGameSessionRepository::with_session(session);
            let event_store = MockEventStore::new();
            let snapshot_store = MockSnapshotStore::new();
            let cache = MockSessionCache::new(); // Empty cache

            let workflow = end_game(&repository, &event_store, &snapshot_store, &cache);
            let command = EndGameCommand::victory(identifier);

            let result = workflow(command).run_async().await;

            assert!(result.is_ok());
            let ended_session = result.unwrap();
            assert_eq!(ended_session.status, GameStatus::Victory);
        }
    }
}
