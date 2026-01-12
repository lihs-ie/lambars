use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;

use crate::dto::request::{CreateGameRequest, EndGameRequest, GameOutcomeRequest};
use crate::dto::response::{
    FloorSummaryResponse, GameEndResponse, GameSessionResponse, GameStatusResponse, PlayerResponse,
    PositionResponse, ResourceResponse,
};
use crate::errors::ApiError;
use crate::state::AppState;
use roguelike_workflow::ports::{EventStore, GameSessionRepository, RandomGenerator, SessionCache};

// =============================================================================
// Create Game Handler
// =============================================================================

pub async fn create_game<Repository, Cache, Events, Random>(
    State(_state): State<AppState<Repository, Cache, Events, Random>>,
    Json(request): Json<CreateGameRequest>,
) -> Result<(StatusCode, Json<GameSessionResponse>), ApiError>
where
    Repository: GameSessionRepository,
    Cache: SessionCache<GameSession = Repository::GameSession>,
    Events: EventStore,
    Random: RandomGenerator,
{
    // Validate request
    if request.player_name.is_empty() {
        return Err(ApiError::validation_field(
            "player_name",
            "must not be empty",
        ));
    }

    if request.player_name.len() > 50 {
        return Err(ApiError::validation_field(
            "player_name",
            "must be 50 characters or less",
        ));
    }

    // TODO: Implement actual game creation workflow
    // For now, return a mock response
    let game_id = uuid::Uuid::new_v4().to_string();

    let response = GameSessionResponse {
        game_id: game_id.clone(),
        player: PlayerResponse {
            player_id: uuid::Uuid::new_v4().to_string(),
            name: request.player_name,
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
        status: GameStatusResponse::InProgress,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

// =============================================================================
// Get Game Handler
// =============================================================================

pub async fn get_game<Repository, Cache, Events, Random>(
    State(_state): State<AppState<Repository, Cache, Events, Random>>,
    Path(game_id): Path<String>,
) -> Result<Json<GameSessionResponse>, ApiError>
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

    // TODO: Implement actual game retrieval from repository/cache
    // For now, return not found
    Err(ApiError::not_found("GameSession", &game_id))
}

// =============================================================================
// End Game Handler
// =============================================================================

pub async fn end_game<Repository, Cache, Events, Random>(
    State(_state): State<AppState<Repository, Cache, Events, Random>>,
    Path(game_id): Path<String>,
    Json(request): Json<EndGameRequest>,
) -> Result<Json<GameEndResponse>, ApiError>
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

    // TODO: Implement actual game ending workflow
    // For now, return a mock response
    let outcome_string = match request.outcome {
        GameOutcomeRequest::Victory => "victory",
        GameOutcomeRequest::Defeat => "defeat",
        GameOutcomeRequest::Abandon => "abandon",
    };

    let response = GameEndResponse {
        game_id,
        final_score: 0,
        dungeon_depth: 1,
        turns_survived: 0,
        enemies_defeated: 0,
        outcome: outcome_string.to_string(),
    };

    Ok(Json(response))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
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
    // Create Game Tests
    // =========================================================================

    mod create_game_handler {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn creates_game_with_valid_request() {
            let state = create_test_state();
            let request = CreateGameRequest {
                player_name: "Hero".to_string(),
                seed: None,
            };

            let result = create_game(State(state), Json(request)).await;

            assert!(result.is_ok());
            let (status, Json(response)) = result.unwrap();
            assert_eq!(status, StatusCode::CREATED);
            assert_eq!(response.player.name, "Hero");
            assert_eq!(response.status, GameStatusResponse::InProgress);
            assert_eq!(response.turn_count, 0);
        }

        #[rstest]
        #[tokio::test]
        async fn creates_game_with_seed() {
            let state = create_test_state();
            let request = CreateGameRequest {
                player_name: "Hero".to_string(),
                seed: Some(12345),
            };

            let result = create_game(State(state), Json(request)).await;

            assert!(result.is_ok());
        }

        #[rstest]
        #[tokio::test]
        async fn rejects_empty_player_name() {
            let state = create_test_state();
            let request = CreateGameRequest {
                player_name: "".to_string(),
                seed: None,
            };

            let result = create_game(State(state), Json(request)).await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
        }

        #[rstest]
        #[tokio::test]
        async fn rejects_too_long_player_name() {
            let state = create_test_state();
            let request = CreateGameRequest {
                player_name: "a".repeat(51),
                seed: None,
            };

            let result = create_game(State(state), Json(request)).await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
        }
    }

    // =========================================================================
    // Get Game Tests
    // =========================================================================

    mod get_game_handler {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn returns_not_found_for_missing_game() {
            let state = create_test_state();
            let game_id = uuid::Uuid::new_v4().to_string();

            let result = get_game(State(state), Path(game_id)).await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.status_code(), StatusCode::NOT_FOUND);
        }

        #[rstest]
        #[tokio::test]
        async fn rejects_invalid_game_id_format() {
            let state = create_test_state();

            let result = get_game(State(state), Path("invalid-uuid".to_string())).await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
        }
    }

    // =========================================================================
    // End Game Tests
    // =========================================================================

    mod end_game_handler {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn ends_game_with_abandon() {
            let state = create_test_state();
            let game_id = uuid::Uuid::new_v4().to_string();
            let request = EndGameRequest {
                outcome: GameOutcomeRequest::Abandon,
            };

            let result = end_game(State(state), Path(game_id.clone()), Json(request)).await;

            // TODO: This should return not found once actual implementation is done
            assert!(result.is_ok());
            let Json(response) = result.unwrap();
            assert_eq!(response.game_id, game_id);
            assert_eq!(response.outcome, "abandon");
        }

        #[rstest]
        #[tokio::test]
        async fn ends_game_with_victory() {
            let state = create_test_state();
            let game_id = uuid::Uuid::new_v4().to_string();
            let request = EndGameRequest {
                outcome: GameOutcomeRequest::Victory,
            };

            let result = end_game(State(state), Path(game_id), Json(request)).await;

            assert!(result.is_ok());
            let Json(response) = result.unwrap();
            assert_eq!(response.outcome, "victory");
        }

        #[rstest]
        #[tokio::test]
        async fn ends_game_with_defeat() {
            let state = create_test_state();
            let game_id = uuid::Uuid::new_v4().to_string();
            let request = EndGameRequest {
                outcome: GameOutcomeRequest::Defeat,
            };

            let result = end_game(State(state), Path(game_id), Json(request)).await;

            assert!(result.is_ok());
            let Json(response) = result.unwrap();
            assert_eq!(response.outcome, "defeat");
        }

        #[rstest]
        #[tokio::test]
        async fn rejects_invalid_game_id_format() {
            let state = create_test_state();
            let request = EndGameRequest {
                outcome: GameOutcomeRequest::Abandon,
            };

            let result = end_game(
                State(state),
                Path("invalid-uuid".to_string()),
                Json(request),
            )
            .await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
        }
    }
}
