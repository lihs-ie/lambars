//! Events handler.
//!
//! This module provides the HTTP handler for retrieving game events
//! with pagination support.

use axum::Json;
use axum::extract::{Path, Query, State};

use crate::dto::request::GetEventsParams;
use crate::dto::response::EventsResponse;
use crate::errors::ApiError;
use crate::state::AppState;
use roguelike_workflow::ports::{EventStore, GameSessionRepository, RandomGenerator, SessionCache};

// =============================================================================
// Constants
// =============================================================================

/// Default number of events to return.
const DEFAULT_EVENTS_LIMIT: u32 = 100;

/// Maximum number of events that can be returned in a single request.
const MAX_EVENTS_LIMIT: u32 = 1000;

// =============================================================================
// Get Events Handler
// =============================================================================

/// Retrieves game events for a session with pagination.
///
/// Events are returned in chronological order, starting from the
/// specified sequence number (exclusive).
///
/// # Endpoint
///
/// `GET /api/v1/games/{game_id}/events`
///
/// # Path Parameters
///
/// - `game_id` - The unique identifier of the game session (UUID format)
///
/// # Query Parameters
///
/// - `since` - The sequence number to start from (exclusive). If not provided, starts from the beginning.
/// - `limit` - Maximum number of events to return (default: 100, max: 1000)
///
/// # Response
///
/// - `200 OK` - Returns events with pagination info
/// - `404 Not Found` - Game session not found
///
/// # Examples
///
/// ```json
/// {
///   "events": [
///     {
///       "sequence": 11,
///       "type": "PlayerMoved",
///       "data": { "direction": "north" },
///       "occurred_at": "2026-01-12T12:00:00Z"
///     }
///   ],
///   "next_sequence": 12,
///   "has_more": false
/// }
/// ```
pub async fn get_events<Repository, Cache, Events, Random>(
    State(_state): State<AppState<Repository, Cache, Events, Random>>,
    Path(game_id): Path<String>,
    Query(params): Query<GetEventsParams>,
) -> Result<Json<EventsResponse>, ApiError>
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

    // Validate and normalize limit
    let limit = params.limit.unwrap_or(DEFAULT_EVENTS_LIMIT);
    if limit > MAX_EVENTS_LIMIT {
        return Err(ApiError::validation_field(
            "limit",
            format!("must be at most {}", MAX_EVENTS_LIMIT),
        ));
    }

    let _since = params.since.unwrap_or(0);
    let _limit = limit;

    // TODO: Implement actual event retrieval from event store
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

    mod get_events_handler {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn returns_not_found_for_missing_game() {
            let state = create_test_state();
            let game_id = uuid::Uuid::new_v4().to_string();
            let params = GetEventsParams::default();

            let result = get_events(State(state), Path(game_id), Query(params)).await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.status_code(), StatusCode::NOT_FOUND);
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

            // Should return not found (game doesn't exist) rather than validation error
            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.status_code(), StatusCode::NOT_FOUND);
        }
    }
}
