use axum::Json;
use axum::extract::{Path, Query, State};
use roguelike_domain::game_session::{GameIdentifier, GameSessionEvent};
use roguelike_workflow::SessionStateAccessor;
use roguelike_workflow::ports::{EventStore, GameSessionRepository, RandomGenerator, SessionCache};

use crate::dto::request::GetEventsParams;
use crate::dto::response::{EventsResponse, GameEventResponse};
use crate::errors::ApiError;
use crate::state::AppState;

// =============================================================================
// Constants
// =============================================================================

const DEFAULT_EVENTS_LIMIT: u32 = 100;

const MAX_EVENTS_LIMIT: u32 = 1000;

// =============================================================================
// Get Events Handler
// =============================================================================

pub async fn get_events<Repository, Cache, Events, Random>(
    State(state): State<AppState<Repository, Cache, Events, Random>>,
    Path(game_id): Path<String>,
    Query(params): Query<GetEventsParams>,
) -> Result<Json<EventsResponse>, ApiError>
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

    let limit = params.limit.unwrap_or(DEFAULT_EVENTS_LIMIT);
    if limit > MAX_EVENTS_LIMIT {
        return Err(ApiError::validation_field(
            "limit",
            format!("must be at most {}", MAX_EVENTS_LIMIT),
        ));
    }

    let since = params.since.unwrap_or(0);

    let events = if since > 0 {
        state
            .game_session_provider
            .get_events_since(&identifier, since)
            .run_async()
            .await?
    } else {
        state
            .game_session_provider
            .get_events(&identifier)
            .run_async()
            .await?
    };

    let total_events = events.len();
    let events: Vec<GameEventResponse> = events
        .into_iter()
        .take(limit as usize)
        .enumerate()
        .map(|(index, event)| event_to_response(since + index as u64, event))
        .collect();

    let has_more = total_events > limit as usize;
    let next_sequence = since + events.len() as u64;

    let response = EventsResponse {
        events,
        next_sequence,
        has_more,
    };
    Ok(Json(response))
}

fn event_to_response(sequence: u64, event: GameSessionEvent) -> GameEventResponse {
    let (event_type, data) = match &event {
        GameSessionEvent::Started(started) => (
            "GameStarted",
            serde_json::json!({
                "game_identifier": started.game_identifier().to_string(),
                "seed": started.seed().value()
            }),
        ),
        GameSessionEvent::Ended(ended) => (
            "GameEnded",
            serde_json::json!({
                "outcome": format!("{:?}", ended.outcome())
            }),
        ),
        GameSessionEvent::TurnStarted(turn_event) => (
            "TurnStarted",
            serde_json::json!({
                "turn": turn_event.turn().value()
            }),
        ),
        GameSessionEvent::TurnEnded(turn_event) => (
            "TurnEnded",
            serde_json::json!({
                "turn": turn_event.turn().value()
            }),
        ),
        _ => ("Unknown", serde_json::Value::Null),
    };

    GameEventResponse {
        sequence,
        event_type: event_type.to_string(),
        data,
        occurred_at: chrono::Utc::now(),
    }
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
    use roguelike_domain::game_session::{
        GameIdentifier, GameSessionEvent, GameStatus, RandomSeed,
    };
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

    mod get_events_handler {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn returns_empty_events_for_missing_game() {
            let state = create_test_state();
            let game_id = uuid::Uuid::new_v4().to_string();
            let params = GetEventsParams::default();

            let result = get_events(State(state), Path(game_id), Query(params)).await;

            assert!(result.is_ok());
            let Json(response) = result.unwrap();
            assert!(response.events.is_empty());
            assert!(!response.has_more);
        }

        #[rstest]
        #[tokio::test]
        async fn rejects_invalid_game_id_format() {
            let state = create_test_state();
            let params = GetEventsParams::default();

            let result = get_events(
                State(state),
                Path("invalid-uuid".to_string()),
                Query(params),
            )
            .await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
        }

        #[rstest]
        #[tokio::test]
        async fn rejects_limit_exceeding_maximum() {
            let state = create_test_state();
            let game_id = uuid::Uuid::new_v4().to_string();
            let params = GetEventsParams {
                since: None,
                limit: Some(MAX_EVENTS_LIMIT + 1),
            };

            let result = get_events(State(state), Path(game_id), Query(params)).await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
        }

        #[rstest]
        #[tokio::test]
        async fn accepts_valid_limit() {
            let state = create_test_state();
            let game_id = uuid::Uuid::new_v4().to_string();
            let params = GetEventsParams {
                since: Some(10),
                limit: Some(50),
            };

            let result = get_events(State(state), Path(game_id), Query(params)).await;

            // Should return empty events (game doesn't exist) rather than validation error
            assert!(result.is_ok());
            let Json(response) = result.unwrap();
            assert!(response.events.is_empty());
        }
    }
}
