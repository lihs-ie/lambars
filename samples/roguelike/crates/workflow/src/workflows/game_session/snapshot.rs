use lambars::effect::AsyncIO;

use super::CreateSnapshotCommand;
use super::resume_game::SessionStateAccessor;
use crate::errors::WorkflowError;
use crate::ports::{SessionCache, SnapshotStore, WorkflowResult};

// =============================================================================
// Default Configuration
// =============================================================================

pub const DEFAULT_SNAPSHOT_INTERVAL: u64 = 100;

// =============================================================================
// CreateSnapshot Workflow
// =============================================================================

pub fn create_snapshot<'a, C, S>(
    cache: &'a C,
    snapshot_store: &'a S,
    interval: u64,
) -> impl Fn(CreateSnapshotCommand) -> AsyncIO<WorkflowResult<()>> + 'a
where
    C: SessionCache,
    S: SnapshotStore<GameSession = C::GameSession>,
    C::GameSession: SessionStateAccessor,
{
    move |command| {
        let cache = cache.clone();
        let snapshot_store = snapshot_store.clone();
        let game_identifier = *command.game_identifier();

        // Step 1: [IO] Load session from cache
        cache.get(&game_identifier).flat_map(move |cached| {
            match cached {
                None => {
                    // Session not found
                    AsyncIO::pure(Err(WorkflowError::not_found(
                        "GameSession",
                        game_identifier.to_string(),
                    )))
                }
                Some(session) => {
                    let event_sequence = session.event_sequence();

                    // Step 2: [IO] Load latest snapshot and check interval
                    snapshot_store
                        .load_latest_snapshot(&game_identifier)
                        .flat_map(move |snapshot_option| {
                            // [Pure] Calculate events since last snapshot
                            let last_snapshot_sequence =
                                snapshot_option.as_ref().map(|(_, seq)| *seq).unwrap_or(0);
                            let events_since_snapshot =
                                event_sequence.saturating_sub(last_snapshot_sequence);

                            // Step 3: [Pure] Check if snapshot is needed
                            if should_create_snapshot(events_since_snapshot, interval) {
                                // Step 4: [IO] Save snapshot
                                snapshot_store
                                    .save_snapshot(&game_identifier, &session, event_sequence)
                                    .fmap(|()| Ok(()))
                            } else {
                                // No snapshot needed
                                AsyncIO::pure(Ok(()))
                            }
                        })
                }
            }
        })
    }
}

pub fn create_snapshot_with_default_interval<'a, C, S>(
    cache: &'a C,
    snapshot_store: &'a S,
) -> impl Fn(CreateSnapshotCommand) -> AsyncIO<WorkflowResult<()>> + 'a
where
    C: SessionCache,
    S: SnapshotStore<GameSession = C::GameSession>,
    C::GameSession: SessionStateAccessor,
{
    create_snapshot(cache, snapshot_store, DEFAULT_SNAPSHOT_INTERVAL)
}

// =============================================================================
// Pure Functions
// =============================================================================

#[must_use]
pub fn should_create_snapshot(events_since_snapshot: u64, interval: u64) -> bool {
    events_since_snapshot >= interval
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
    use roguelike_domain::game_session::{
        GameIdentifier, GameOutcome, GameSessionEvent, GameStatus, RandomSeed,
    };
    use roguelike_domain::player::Player;
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
        fn with_event_sequence(
            identifier: GameIdentifier,
            seed: RandomSeed,
            event_sequence: u64,
        ) -> Self {
            Self {
                identifier,
                seed,
                status: GameStatus::InProgress,
                event_sequence,
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

        fn apply_event(&self, _event: &GameSessionEvent) -> Self {
            let mut new_session = self.clone();
            new_session.event_sequence += 1;
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

    // =========================================================================
    // Pure Function Tests
    // =========================================================================

    mod pure_functions {
        use super::*;

        #[rstest]
        #[case(0, 100, false)]
        #[case(50, 100, false)]
        #[case(99, 100, false)]
        #[case(100, 100, true)]
        #[case(150, 100, true)]
        #[case(0, 0, true)]
        fn should_create_snapshot_tests(
            #[case] events_since: u64,
            #[case] interval: u64,
            #[case] expected: bool,
        ) {
            let result = should_create_snapshot(events_since, interval);
            assert_eq!(result, expected);
        }
    }

    // =========================================================================
    // Workflow Tests
    // =========================================================================

    mod workflow {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn create_snapshot_when_needed() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::with_event_sequence(identifier, seed, 100);

            let cache = MockSessionCache::with_session(session);
            let snapshot_store = MockSnapshotStore::new(); // No existing snapshot

            let workflow = create_snapshot(&cache, &snapshot_store, 50);
            let command = CreateSnapshotCommand::new(identifier);

            let result = workflow(command).run_async().await;

            assert!(result.is_ok());

            // Verify snapshot was created
            let snapshot = snapshot_store
                .load_latest_snapshot(&identifier)
                .run_async()
                .await;
            assert!(snapshot.is_some());
            let (_, sequence) = snapshot.unwrap();
            assert_eq!(sequence, 100);
        }

        #[rstest]
        #[tokio::test]
        async fn skip_snapshot_when_not_needed() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::with_event_sequence(identifier, seed, 30);

            let cache = MockSessionCache::with_session(session);
            let snapshot_store = MockSnapshotStore::new();

            let workflow = create_snapshot(&cache, &snapshot_store, 50);
            let command = CreateSnapshotCommand::new(identifier);

            let result = workflow(command).run_async().await;

            assert!(result.is_ok());

            // Verify no snapshot was created
            let snapshot = snapshot_store
                .load_latest_snapshot(&identifier)
                .run_async()
                .await;
            assert!(snapshot.is_none());
        }

        #[rstest]
        #[tokio::test]
        async fn create_snapshot_considers_existing_snapshot() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::with_event_sequence(identifier, seed, 120);
            let old_snapshot_session = MockGameSession::with_event_sequence(identifier, seed, 100);

            let cache = MockSessionCache::with_session(session);
            let snapshot_store =
                MockSnapshotStore::with_snapshot(identifier, old_snapshot_session, 100);

            // 120 - 100 = 20 events since last snapshot, interval is 50
            let workflow = create_snapshot(&cache, &snapshot_store, 50);
            let command = CreateSnapshotCommand::new(identifier);

            let result = workflow(command).run_async().await;

            assert!(result.is_ok());

            // Verify snapshot was NOT updated (still at 100)
            let snapshot = snapshot_store
                .load_latest_snapshot(&identifier)
                .run_async()
                .await;
            assert!(snapshot.is_some());
            let (_, sequence) = snapshot.unwrap();
            assert_eq!(sequence, 100); // Still old snapshot
        }

        #[rstest]
        #[tokio::test]
        async fn create_snapshot_updates_when_interval_exceeded() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::with_event_sequence(identifier, seed, 200);
            let old_snapshot_session = MockGameSession::with_event_sequence(identifier, seed, 100);

            let cache = MockSessionCache::with_session(session);
            let snapshot_store =
                MockSnapshotStore::with_snapshot(identifier, old_snapshot_session, 100);

            // 200 - 100 = 100 events since last snapshot, interval is 50
            let workflow = create_snapshot(&cache, &snapshot_store, 50);
            let command = CreateSnapshotCommand::new(identifier);

            let result = workflow(command).run_async().await;

            assert!(result.is_ok());

            // Verify snapshot was updated
            let snapshot = snapshot_store
                .load_latest_snapshot(&identifier)
                .run_async()
                .await;
            assert!(snapshot.is_some());
            let (_, sequence) = snapshot.unwrap();
            assert_eq!(sequence, 200); // New snapshot
        }

        #[rstest]
        #[tokio::test]
        async fn create_snapshot_not_found() {
            let identifier = GameIdentifier::new();

            let cache = MockSessionCache::new();
            let snapshot_store = MockSnapshotStore::new();

            let workflow = create_snapshot(&cache, &snapshot_store, 50);
            let command = CreateSnapshotCommand::new(identifier);

            let result = workflow(command).run_async().await;

            assert!(result.is_err());
            assert!(result.unwrap_err().is_not_found());
        }

        #[rstest]
        #[tokio::test]
        async fn using_default_interval() {
            let identifier = GameIdentifier::new();
            let seed = RandomSeed::new(42);
            let session = MockGameSession::with_event_sequence(identifier, seed, 150);

            let cache = MockSessionCache::with_session(session);
            let snapshot_store = MockSnapshotStore::new();

            // Default interval is 100
            let workflow =
                super::super::create_snapshot_with_default_interval(&cache, &snapshot_store);
            let command = CreateSnapshotCommand::new(identifier);

            let result = workflow(command).run_async().await;

            assert!(result.is_ok());

            // 150 >= 100, snapshot should be created
            let snapshot = snapshot_store
                .load_latest_snapshot(&identifier)
                .run_async()
                .await;
            assert!(snapshot.is_some());
        }
    }
}
