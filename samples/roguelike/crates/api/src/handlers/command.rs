use axum::Json;
use axum::extract::{Path, State};
use roguelike_domain::game_session::{GameIdentifier, GameStatus};
use roguelike_workflow::SessionStateAccessor;
use roguelike_workflow::ports::{EventStore, GameSessionRepository, RandomGenerator, SessionCache};

use crate::dto::request::ExecuteCommandRequest;
use crate::dto::response::{
    FloorSummaryResponse, GameSessionResponse, GameStatusResponse, PlayerResponse,
    PositionResponse, ResourceResponse, TurnResultResponse,
};
use crate::errors::ApiError;
use crate::state::AppState;

// =============================================================================
// Execute Command Handler
// =============================================================================

pub async fn execute_command<Repository, Cache, Events, Random>(
    State(state): State<AppState<Repository, Cache, Events, Random>>,
    Path(game_id): Path<String>,
    Json(request): Json<ExecuteCommandRequest>,
) -> Result<Json<TurnResultResponse>, ApiError>
where
    Repository: GameSessionRepository,
    Repository::GameSession: SessionStateAccessor,
    Cache: SessionCache<GameSession = Repository::GameSession>,
    Events: EventStore,
    Random: RandomGenerator,
{
    let identifier: GameIdentifier = game_id
        .parse()
        .map_err(|_| ApiError::validation_field("game_id", "must be a valid UUID"))?;

    let command_string = format!("{:?}", request.command);
    let session = state
        .game_session_provider
        .execute_command(&identifier, &command_string)
        .run_async()
        .await?;

    let status = match session.status() {
        GameStatus::InProgress => GameStatusResponse::InProgress,
        GameStatus::Victory => GameStatusResponse::Victory,
        GameStatus::Defeat => GameStatusResponse::Defeat,
        GameStatus::Paused => GameStatusResponse::Paused,
    };

    let response = TurnResultResponse {
        game: GameSessionResponse {
            game_id: session.identifier().to_string(),
            player: PlayerResponse {
                player_id: uuid::Uuid::new_v4().to_string(),
                name: "Player".to_string(),
                position: PositionResponse { x: 5, y: 5 },
                health: ResourceResponse {
                    current: 100,
                    max: 100,
                },
                mana: ResourceResponse {
                    current: 50,
                    max: 50,
                },
                level: 1,
                experience: 0,
            },
            floor: FloorSummaryResponse {
                level: 1,
                width: 50,
                height: 40,
                explored_percentage: 0.0,
            },
            turn_count: 0,
            status,
        },
        turn_events: vec![],
        game_over: false,
        game_over_reason: None,
    };

    Ok(Json(response))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::command::{CommandRequest, DirectionRequest};
    use crate::state::AppState;
    use lambars::effect::AsyncIO;
    use roguelike_domain::game_session::{GameIdentifier, GameSessionEvent, RandomSeed};
    use roguelike_workflow::ports::{
        EventStore, GameSessionRepository, RandomGenerator, SessionCache,
    };
    use rstest::rstest;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, RwLock};
    use std::time::Duration;

    // =========================================================================
    // Mock Implementations
    // =========================================================================

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct MockGameSession {
        identifier: GameIdentifier,
    }

    impl MockGameSession {
        fn new(identifier: GameIdentifier) -> Self {
            Self { identifier }
        }
    }

    impl SessionStateAccessor for MockGameSession {
        fn status(&self) -> GameStatus {
            GameStatus::InProgress
        }

        fn identifier(&self) -> &GameIdentifier {
            &self.identifier
        }

        fn event_sequence(&self) -> u64 {
            0
        }

        fn apply_event(&self, _event: &GameSessionEvent) -> Self {
            self.clone()
        }
    }

    #[derive(Clone)]
    struct MockRepository {
        sessions: Arc<RwLock<HashMap<GameIdentifier, MockGameSession>>>,
        events: Arc<RwLock<HashMap<GameIdentifier, Vec<GameSessionEvent>>>>,
    }

    impl MockRepository {
        fn new() -> Self {
            Self {
                sessions: Arc::new(RwLock::new(HashMap::new())),
                events: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        fn with_events(
            events: Arc<RwLock<HashMap<GameIdentifier, Vec<GameSessionEvent>>>>,
        ) -> Self {
            Self {
                sessions: Arc::new(RwLock::new(HashMap::new())),
                events,
            }
        }
    }

    impl GameSessionRepository for MockRepository {
        type GameSession = MockGameSession;

        fn find_by_id(&self, identifier: &GameIdentifier) -> AsyncIO<Option<Self::GameSession>> {
            let sessions = Arc::clone(&self.sessions);
            let events = Arc::clone(&self.events);
            let identifier = *identifier;
            AsyncIO::new(move || async move {
                if let Some(session) = sessions.read().unwrap().get(&identifier).cloned() {
                    return Some(session);
                }
                if events.read().unwrap().contains_key(&identifier) {
                    return Some(MockGameSession::new(identifier));
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

    fn create_test_state() -> AppState<MockRepository, MockCache, MockEventStore, MockRandom> {
        let event_store = MockEventStore::new();
        let repository = MockRepository::with_events(event_store.events_arc());
        AppState::new(repository, MockCache::new(), event_store, MockRandom::new())
    }

    // =========================================================================
    // Tests
    // =========================================================================

    mod execute_command_handler {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn returns_not_implemented_for_move_command() {
            let state = create_test_state();
            let game_id = uuid::Uuid::new_v4().to_string();
            let request = ExecuteCommandRequest {
                command: CommandRequest::Move {
                    direction: DirectionRequest::North,
                },
            };

            let result = execute_command(State(state), Path(game_id), Json(request)).await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.status_code(), axum::http::StatusCode::NOT_IMPLEMENTED);
        }

        #[rstest]
        #[tokio::test]
        async fn returns_not_implemented_for_wait_command() {
            let state = create_test_state();
            let game_id = uuid::Uuid::new_v4().to_string();
            let request = ExecuteCommandRequest {
                command: CommandRequest::Wait,
            };

            let result = execute_command(State(state), Path(game_id), Json(request)).await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.status_code(), axum::http::StatusCode::NOT_IMPLEMENTED);
        }

        #[rstest]
        #[tokio::test]
        async fn returns_not_implemented_for_attack_command() {
            let state = create_test_state();
            let game_id = uuid::Uuid::new_v4().to_string();
            let target_id = uuid::Uuid::new_v4().to_string();
            let request = ExecuteCommandRequest {
                command: CommandRequest::Attack { target_id },
            };

            let result = execute_command(State(state), Path(game_id), Json(request)).await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.status_code(), axum::http::StatusCode::NOT_IMPLEMENTED);
        }

        #[rstest]
        #[tokio::test]
        async fn rejects_invalid_game_id() {
            let state = create_test_state();
            let request = ExecuteCommandRequest {
                command: CommandRequest::Wait,
            };

            let result = execute_command(
                State(state),
                Path("invalid-uuid".to_string()),
                Json(request),
            )
            .await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.status_code(), axum::http::StatusCode::BAD_REQUEST);
        }
    }
}
