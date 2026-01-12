use axum::Json;
use axum::extract::{Path, State};

use crate::dto::request::ExecuteCommandRequest;
use crate::dto::response::{
    FloorSummaryResponse, GameEventResponse, GameSessionResponse, GameStatusResponse,
    PlayerResponse, PositionResponse, ResourceResponse, TurnResultResponse,
};
use crate::errors::ApiError;
use crate::state::AppState;
use roguelike_workflow::ports::{EventStore, GameSessionRepository, RandomGenerator, SessionCache};

// =============================================================================
// Execute Command Handler
// =============================================================================

pub async fn execute_command<Repository, Cache, Events, Random>(
    State(_state): State<AppState<Repository, Cache, Events, Random>>,
    Path(game_id): Path<String>,
    Json(request): Json<ExecuteCommandRequest>,
) -> Result<Json<TurnResultResponse>, ApiError>
where
    Repository: GameSessionRepository,
    Cache: SessionCache<GameSession = Repository::GameSession>,
    Events: EventStore,
    Random: RandomGenerator,
{
    // Validate game_id format
    if uuid::Uuid::parse_str(&game_id).is_err() {
        return Err(ApiError::validation_field(
            "game_id",
            "must be a valid UUID",
        ));
    }

    // TODO: Implement actual command execution workflow
    // This involves:
    // 1. Load game session from cache/repository
    // 2. Validate command against current game state
    // 3. Execute the command via workflow
    // 4. Process enemy turns
    // 5. Update game state
    // 6. Store events
    // 7. Update cache

    // For now, return a mock response
    let _command = &request.command;

    let response = TurnResultResponse {
        game: GameSessionResponse {
            game_id: game_id.clone(),
            player: PlayerResponse {
                player_id: uuid::Uuid::new_v4().to_string(),
                name: "Hero".to_string(),
                position: PositionResponse { x: 5, y: 4 },
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
                explored_percentage: 2.5,
            },
            turn_count: 1,
            status: GameStatusResponse::InProgress,
        },
        turn_events: vec![GameEventResponse {
            sequence: 1,
            event_type: "PlayerMoved".to_string(),
            data: serde_json::json!({
                "direction": "north"
            }),
            occurred_at: chrono::Utc::now(),
        }],
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

    fn create_test_state() -> AppState<MockRepository, MockCache, MockEventStore, MockRandom> {
        AppState::new(
            MockRepository::new(),
            MockCache::new(),
            MockEventStore::new(),
            MockRandom::new(),
        )
    }

    // =========================================================================
    // Tests
    // =========================================================================

    mod execute_command_handler {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn executes_move_command() {
            let state = create_test_state();
            let game_id = uuid::Uuid::new_v4().to_string();
            let request = ExecuteCommandRequest {
                command: CommandRequest::Move {
                    direction: DirectionRequest::North,
                },
            };

            let result = execute_command(State(state), Path(game_id.clone()), Json(request)).await;

            assert!(result.is_ok());
            let Json(response) = result.unwrap();
            assert_eq!(response.game.game_id, game_id);
            assert!(!response.game_over);
        }

        #[rstest]
        #[tokio::test]
        async fn executes_wait_command() {
            let state = create_test_state();
            let game_id = uuid::Uuid::new_v4().to_string();
            let request = ExecuteCommandRequest {
                command: CommandRequest::Wait,
            };

            let result = execute_command(State(state), Path(game_id), Json(request)).await;

            assert!(result.is_ok());
        }

        #[rstest]
        #[tokio::test]
        async fn executes_attack_command() {
            let state = create_test_state();
            let game_id = uuid::Uuid::new_v4().to_string();
            let target_id = uuid::Uuid::new_v4().to_string();
            let request = ExecuteCommandRequest {
                command: CommandRequest::Attack { target_id },
            };

            let result = execute_command(State(state), Path(game_id), Json(request)).await;

            assert!(result.is_ok());
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
