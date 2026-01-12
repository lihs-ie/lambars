use axum::Json;
use axum::extract::{Path, Query, State};

use crate::dto::request::GetFloorParams;
use crate::dto::response::{FloorResponse, VisibleAreaResponse};
use crate::errors::ApiError;
use crate::state::AppState;
use roguelike_workflow::SessionStateAccessor;
use roguelike_workflow::ports::{EventStore, GameSessionRepository, RandomGenerator, SessionCache};

// =============================================================================
// Get Floor Handler
// =============================================================================

pub async fn get_floor<Repository, Cache, Events, Random>(
    State(_state): State<AppState<Repository, Cache, Events, Random>>,
    Path(game_id): Path<String>,
    Query(_params): Query<GetFloorParams>,
) -> Result<Json<FloorResponse>, ApiError>
where
    Repository: GameSessionRepository,
    Repository::GameSession: SessionStateAccessor,
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

    // TODO: Implement actual floor retrieval from repository/cache
    // For now, return not found
    Err(ApiError::not_found("GameSession", &game_id))
}

// =============================================================================
// Get Visible Area Handler
// =============================================================================

pub async fn get_visible_area<Repository, Cache, Events, Random>(
    State(_state): State<AppState<Repository, Cache, Events, Random>>,
    Path(game_id): Path<String>,
) -> Result<Json<VisibleAreaResponse>, ApiError>
where
    Repository: GameSessionRepository,
    Repository::GameSession: SessionStateAccessor,
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

    // TODO: Implement actual visible area calculation from repository/cache
    // For now, return not found
    Err(ApiError::not_found("GameSession", &game_id))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use axum::http::StatusCode;
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

    use roguelike_domain::game_session::GameStatus;

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
    // Get Floor Tests
    // =========================================================================

    mod get_floor_handler {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn returns_not_found_for_missing_game() {
            let state = create_test_state();
            let game_id = uuid::Uuid::new_v4().to_string();
            let params = GetFloorParams::default();

            let result = get_floor(State(state), Path(game_id), Query(params)).await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.status_code(), StatusCode::NOT_FOUND);
        }

        #[rstest]
        #[tokio::test]
        async fn rejects_invalid_game_id_format() {
            let state = create_test_state();
            let params = GetFloorParams::default();

            let result = get_floor(
                State(state),
                Path("invalid-uuid".to_string()),
                Query(params),
            )
            .await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
        }
    }

    // =========================================================================
    // Get Visible Area Tests
    // =========================================================================

    mod get_visible_area_handler {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn returns_not_found_for_missing_game() {
            let state = create_test_state();
            let game_id = uuid::Uuid::new_v4().to_string();

            let result = get_visible_area(State(state), Path(game_id)).await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.status_code(), StatusCode::NOT_FOUND);
        }

        #[rstest]
        #[tokio::test]
        async fn rejects_invalid_game_id_format() {
            let state = create_test_state();

            let result = get_visible_area(State(state), Path("invalid-uuid".to_string())).await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
        }
    }
}
