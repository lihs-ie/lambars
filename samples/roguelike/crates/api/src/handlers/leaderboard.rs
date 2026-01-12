use axum::Json;
use axum::extract::{Query, State};

use crate::dto::request::{GetLeaderboardParams, LeaderboardTypeRequest};
use crate::dto::response::LeaderboardResponse;
use crate::errors::ApiError;
use crate::state::AppState;
use roguelike_workflow::ports::{EventStore, GameSessionRepository, RandomGenerator, SessionCache};

// =============================================================================
// Constants
// =============================================================================

const DEFAULT_LEADERBOARD_LIMIT: u32 = 10;

const MAX_LEADERBOARD_LIMIT: u32 = 100;

// =============================================================================
// Get Leaderboard Handler
// =============================================================================

pub async fn get_leaderboard<Repository, Cache, Events, Random>(
    State(_state): State<AppState<Repository, Cache, Events, Random>>,
    Query(params): Query<GetLeaderboardParams>,
) -> Result<Json<LeaderboardResponse>, ApiError>
where
    Repository: GameSessionRepository,
    Cache: SessionCache<GameSession = Repository::GameSession>,
    Events: EventStore,
    Random: RandomGenerator,
{
    // Validate and normalize limit
    let limit = params.limit.unwrap_or(DEFAULT_LEADERBOARD_LIMIT);
    if limit > MAX_LEADERBOARD_LIMIT {
        return Err(ApiError::validation_field(
            "limit",
            format!("must be at most {}", MAX_LEADERBOARD_LIMIT),
        ));
    }

    // Determine leaderboard type
    let leaderboard_type = params
        .leaderboard_type
        .unwrap_or(LeaderboardTypeRequest::Global);

    let type_string = match leaderboard_type {
        LeaderboardTypeRequest::Global => "global",
        LeaderboardTypeRequest::Daily => "daily",
        LeaderboardTypeRequest::Weekly => "weekly",
    };

    // TODO: Implement actual leaderboard retrieval from storage
    // For now, return an empty leaderboard
    let response = LeaderboardResponse {
        leaderboard_type: type_string.to_string(),
        entries: vec![],
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

    mod get_leaderboard_handler {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn returns_empty_leaderboard() {
            let state = create_test_state();
            let params = GetLeaderboardParams::default();

            let result = get_leaderboard(State(state), Query(params)).await;

            assert!(result.is_ok());
            let Json(response) = result.unwrap();
            assert_eq!(response.leaderboard_type, "global");
            assert!(response.entries.is_empty());
        }

        #[rstest]
        #[tokio::test]
        async fn returns_global_leaderboard_by_default() {
            let state = create_test_state();
            let params = GetLeaderboardParams::default();

            let result = get_leaderboard(State(state), Query(params)).await;

            assert!(result.is_ok());
            let Json(response) = result.unwrap();
            assert_eq!(response.leaderboard_type, "global");
        }

        #[rstest]
        #[case(LeaderboardTypeRequest::Global, "global")]
        #[case(LeaderboardTypeRequest::Daily, "daily")]
        #[case(LeaderboardTypeRequest::Weekly, "weekly")]
        #[tokio::test]
        async fn returns_requested_leaderboard_type(
            #[case] leaderboard_type: LeaderboardTypeRequest,
            #[case] expected_type: &str,
        ) {
            let state = create_test_state();
            let params = GetLeaderboardParams {
                leaderboard_type: Some(leaderboard_type),
                limit: None,
            };

            let result = get_leaderboard(State(state), Query(params)).await;

            assert!(result.is_ok());
            let Json(response) = result.unwrap();
            assert_eq!(response.leaderboard_type, expected_type);
        }

        #[rstest]
        #[tokio::test]
        async fn rejects_limit_exceeding_maximum() {
            let state = create_test_state();
            let params = GetLeaderboardParams {
                leaderboard_type: None,
                limit: Some(MAX_LEADERBOARD_LIMIT + 1),
            };

            let result = get_leaderboard(State(state), Query(params)).await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
        }

        #[rstest]
        #[tokio::test]
        async fn accepts_valid_limit() {
            let state = create_test_state();
            let params = GetLeaderboardParams {
                leaderboard_type: None,
                limit: Some(50),
            };

            let result = get_leaderboard(State(state), Query(params)).await;

            assert!(result.is_ok());
        }
    }
}
