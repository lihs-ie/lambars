//! ResumeGame workflow implementation.
//!
//! This module provides the workflow for resuming existing game sessions
//! using the Event Sourcing pattern for state reconstruction.
//!
//! # Workflow Steps
//!
//! 1. [IO] Check cache for session
//! 2. [IO] If cache miss, load latest snapshot and events (composed with pipe!)
//! 3. [Pure] Reconstruct state from events (fold)
//! 4. [Pure] Validate session is active
//! 5. [IO] Update cache
//!
//! # Event Sourcing
//!
//! The session state is reconstructed by:
//! 1. Loading the latest snapshot (if available)
//! 2. Loading events that occurred after the snapshot
//! 3. Applying those events to the snapshot state using fold
//!
//! # Examples
//!
//! ```ignore
//! use roguelike_workflow::workflows::game_session::{resume_game, ResumeGameCommand};
//!
//! let workflow = resume_game(&repository, &event_store, &snapshot_store, &cache);
//! let command = ResumeGameCommand::new(game_identifier);
//! let result = workflow(command).run_async().await;
//! ```

use std::time::Duration;

use lambars::effect::AsyncIO;
use roguelike_domain::game_session::{GameIdentifier, GameSessionEvent, GameStatus};

use super::ResumeGameCommand;
use crate::errors::WorkflowError;
use crate::ports::{
    EventStore, GameSessionRepository, SessionCache, SnapshotStore, WorkflowResult,
};

// =============================================================================
// Workflow Configuration
// =============================================================================

/// Default cache time-to-live for game sessions.
const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300); // 5 minutes

// =============================================================================
// ResumeGame Workflow
// =============================================================================

/// Creates a workflow function for resuming existing game sessions.
///
/// This function implements the Event Sourcing pattern:
/// 1. First check the cache for a hot session
/// 2. If not cached, load the latest snapshot
/// 3. Load events since the snapshot
/// 4. Reconstruct the current state by applying events to the snapshot
/// 5. Validate the session is still active
/// 6. Cache the reconstructed session
///
/// # Type Parameters
///
/// * `R` - Repository type implementing `GameSessionRepository`
/// * `E` - Event store type implementing `EventStore`
/// * `S` - Snapshot store type implementing `SnapshotStore`
/// * `C` - Cache type implementing `SessionCache`
///
/// # Arguments
///
/// * `repository` - The game session repository
/// * `event_store` - The event store for event sourcing
/// * `snapshot_store` - The snapshot store for optimization
/// * `cache` - The session cache for fast access
///
/// # Returns
///
/// A function that takes a `ResumeGameCommand` and returns an `AsyncIO`
/// that produces the resumed game session or an error.
///
/// # Errors
///
/// - `WorkflowError::NotFound` - If the session doesn't exist
/// - `WorkflowError::Domain` - If the session is not active (completed/abandoned)
///
/// # Examples
///
/// ```ignore
/// use roguelike_workflow::workflows::game_session::{resume_game, ResumeGameCommand};
///
/// let workflow = resume_game(&repository, &event_store, &snapshot_store, &cache);
/// let command = ResumeGameCommand::new(game_identifier);
/// let session = workflow(command).run_async().await?;
/// ```
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
        let repository = repository.clone();
        let event_store = event_store.clone();
        let snapshot_store = snapshot_store.clone();
        let cache = cache.clone();
        let game_identifier = *command.game_identifier();

        // Step 1: [IO] Check cache first
        cache.get(&game_identifier).flat_map(move |cached| {
            match cached {
                Some(session) => {
                    // [Pure] Session found in cache, validate and return
                    let validation_result = validate_session_active(&session);
                    AsyncIO::pure(validation_result.map(|()| session))
                }
                None => {
                    // Step 2: [IO] Cache miss - load snapshot
                    let repository_clone = repository.clone();
                    let event_store_clone = event_store.clone();
                    let cache_clone = cache.clone();

                    snapshot_store
                        .load_latest_snapshot(&game_identifier)
                        .flat_map(move |snapshot_option| {
                            // [Pure] Get sequence number for event loading
                            let since_sequence =
                                snapshot_option.as_ref().map(|(_, seq)| *seq).unwrap_or(0);

                            // Step 3: [IO] Load events since snapshot
                            event_store_clone
                                .load_events_since(&game_identifier, since_sequence)
                                .flat_map(move |events| {
                                    // Step 4: [Pure/IO] Reconstruct and cache session
                                    reconstruct_and_cache_session(
                                        snapshot_option,
                                        events,
                                        game_identifier,
                                        repository_clone,
                                        cache_clone,
                                    )
                                })
                        })
                }
            }
        })
    }
}

/// Reconstructs session from snapshot/events and caches it.
///
/// This helper function handles the reconstruction strategy based on
/// available snapshot and events.
fn reconstruct_and_cache_session<R, C>(
    snapshot_option: Option<(C::GameSession, u64)>,
    events: Vec<GameSessionEvent>,
    game_identifier: GameIdentifier,
    repository: R,
    cache: C,
) -> AsyncIO<WorkflowResult<C::GameSession>>
where
    R: GameSessionRepository<GameSession = C::GameSession>,
    C: SessionCache,
    C::GameSession: SessionStateAccessor,
{
    if let Some((base_state, _sequence)) = snapshot_option {
        // [Pure] Reconstruct from snapshot + events
        let reconstructed = reconstruct_from_events_internal(base_state, &events);

        // [Pure] Validate session is active, then [IO] cache
        match validate_session_active(&reconstructed) {
            Ok(()) => {
                let session_clone = reconstructed.clone();
                cache
                    .set(&game_identifier, &reconstructed, DEFAULT_CACHE_TIME_TO_LIVE)
                    .fmap(move |()| Ok(session_clone))
            }
            Err(error) => AsyncIO::pure(Err(error)),
        }
    } else if !events.is_empty() {
        // No snapshot but we have events - try to get from repository
        repository
            .find_by_id(&game_identifier)
            .flat_map(move |session_option| match session_option {
                Some(session) => {
                    // [Pure] Validate session, then [IO] cache
                    match validate_session_active(&session) {
                        Ok(()) => {
                            let session_clone = session.clone();
                            cache
                                .set(&game_identifier, &session, DEFAULT_CACHE_TIME_TO_LIVE)
                                .fmap(move |()| Ok(session_clone))
                        }
                        Err(error) => AsyncIO::pure(Err(error)),
                    }
                }
                None => AsyncIO::pure(Err(WorkflowError::not_found(
                    "GameSession",
                    game_identifier.to_string(),
                ))),
            })
    } else {
        // No snapshot and no events - session doesn't exist
        AsyncIO::pure(Err(WorkflowError::not_found(
            "GameSession",
            game_identifier.to_string(),
        )))
    }
}

// =============================================================================
// Session State Accessor Trait
// =============================================================================

/// Trait for accessing game session state.
///
/// This trait abstracts over different game session implementations,
/// allowing the workflow to work with any type that can provide
/// status and event application.
pub trait SessionStateAccessor: Clone + Send + Sync + 'static {
    /// Returns the current game status.
    fn status(&self) -> GameStatus;

    /// Returns the game identifier.
    fn identifier(&self) -> &GameIdentifier;

    /// Returns the event sequence number.
    fn event_sequence(&self) -> u64;

    /// Applies an event to produce a new session state.
    ///
    /// This is a pure function that returns a new session without
    /// modifying the original.
    fn apply_event(&self, event: &GameSessionEvent) -> Self;
}

// =============================================================================
// Pure Functions
// =============================================================================

/// Reconstructs a game session from events.
///
/// This is a pure function that applies a sequence of events to a base state
/// using a fold operation. It's the core of the Event Sourcing pattern.
///
/// # Type Parameters
///
/// * `S` - Session type implementing `SessionStateAccessor`
///
/// # Arguments
///
/// * `initial` - The initial state (from snapshot or empty)
/// * `events` - The events to apply
///
/// # Returns
///
/// The reconstructed session state after applying all events.
///
/// # Examples
///
/// ```ignore
/// let reconstructed = reconstruct_from_events(snapshot_state, &events);
/// ```
pub fn reconstruct_from_events<S>(initial: S, events: &[GameSessionEvent]) -> S
where
    S: SessionStateAccessor,
{
    reconstruct_from_events_internal(initial, events)
}

/// Internal implementation of event reconstruction.
fn reconstruct_from_events_internal<S>(initial: S, events: &[GameSessionEvent]) -> S
where
    S: SessionStateAccessor,
{
    events
        .iter()
        .fold(initial, |state, event| state.apply_event(event))
}

/// Validates that a session is active and can be resumed.
///
/// A session can only be resumed if it's in an active state
/// (InProgress or Paused).
///
/// # Type Parameters
///
/// * `S` - Session type implementing `SessionStateAccessor`
///
/// # Arguments
///
/// * `session` - The session to validate
///
/// # Returns
///
/// - `Ok(())` if the session is active
/// - `Err(WorkflowError::Domain)` if the session is completed
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
