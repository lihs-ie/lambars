//! Routing definitions for the roguelike API.
//!
//! This module defines all API routes and configures the Axum router
//! with handlers, middleware, and state.

use axum::Router;
use axum::routing::{get, post};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::handlers;
use crate::middleware::{RequestIdLayer, ResponseTimeLayer};
use crate::state::AppState;
use roguelike_workflow::ports::{EventStore, GameSessionRepository, RandomGenerator, SessionCache};

// =============================================================================
// Router Creation
// =============================================================================

/// Creates the main API router with all routes and middleware.
///
/// # Type Parameters
///
/// - `Repository` - Game session repository implementation
/// - `Cache` - Session cache implementation
/// - `Events` - Event store implementation
/// - `Random` - Random generator implementation
///
/// # Arguments
///
/// * `state` - Application state containing all dependencies
///
/// # Returns
///
/// A configured Axum router ready to serve requests.
///
/// # Examples
///
/// ```ignore
/// use roguelike_api::routes::create_router;
/// use roguelike_api::state::AppState;
///
/// let state = AppState::new(repository, cache, event_store, random);
/// let router = create_router(state);
///
/// // Serve on port 3000
/// let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
/// axum::serve(listener, router).await?;
/// ```
pub fn create_router<Repository, Cache, Events, Random>(
    state: AppState<Repository, Cache, Events, Random>,
) -> Router
where
    Repository: GameSessionRepository + Send + Sync + 'static,
    Cache: SessionCache<GameSession = Repository::GameSession> + Send + Sync + 'static,
    Events: EventStore + Send + Sync + 'static,
    Random: RandomGenerator + Send + Sync + 'static,
{
    // Build API v1 routes
    let api_v1 = Router::new()
        // Health check
        .route(
            "/health",
            get(handlers::health_check::<Repository, Cache, Events, Random>),
        )
        // Game session management
        .route(
            "/games",
            post(handlers::create_game::<Repository, Cache, Events, Random>),
        )
        .route(
            "/games/{game_id}",
            get(handlers::get_game::<Repository, Cache, Events, Random>),
        )
        .route(
            "/games/{game_id}/end",
            post(handlers::end_game::<Repository, Cache, Events, Random>),
        )
        // Commands
        .route(
            "/games/{game_id}/commands",
            post(handlers::execute_command::<Repository, Cache, Events, Random>),
        )
        // Player
        .route(
            "/games/{game_id}/player",
            get(handlers::get_player::<Repository, Cache, Events, Random>),
        )
        .route(
            "/games/{game_id}/inventory",
            get(handlers::get_inventory::<Repository, Cache, Events, Random>),
        )
        // Floor
        .route(
            "/games/{game_id}/floor",
            get(handlers::get_floor::<Repository, Cache, Events, Random>),
        )
        .route(
            "/games/{game_id}/floor/visible",
            get(handlers::get_visible_area::<Repository, Cache, Events, Random>),
        )
        // Events
        .route(
            "/games/{game_id}/events",
            get(handlers::get_events::<Repository, Cache, Events, Random>),
        )
        // Leaderboard
        .route(
            "/leaderboard",
            get(handlers::get_leaderboard::<Repository, Cache, Events, Random>),
        );

    // Build the complete router with middleware
    Router::new()
        .nest("/api/v1", api_v1)
        .layer(ResponseTimeLayer::new())
        .layer(RequestIdLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(create_cors_layer())
        .with_state(state)
}

/// Creates the CORS layer configuration.
///
/// This configures Cross-Origin Resource Sharing for the API.
/// In production, you should restrict origins to specific domains.
fn create_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
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
    use tower::ServiceExt;

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

    fn create_test_app() -> Router {
        let state = AppState::new(
            MockRepository::new(),
            MockCache::new(),
            MockEventStore::new(),
            MockRandom::new(),
        );
        create_router(state)
    }

    // =========================================================================
    // Tests
    // =========================================================================

    mod health_endpoint {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn health_check_returns_200() {
            let app = create_test_app();

            let response = app
                .oneshot(
                    Request::builder()
                        .uri("/api/v1/health")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
        }

        #[rstest]
        #[tokio::test]
        async fn health_check_returns_json() {
            let app = create_test_app();

            let response = app
                .oneshot(
                    Request::builder()
                        .uri("/api/v1/health")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            let body = response.into_body().collect().await.unwrap().to_bytes();
            let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

            assert_eq!(json["status"], "healthy");
        }
    }

    mod games_endpoints {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn create_game_returns_201() {
            let app = create_test_app();

            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/games")
                        .header("content-type", "application/json")
                        .body(Body::from(r#"{"player_name": "Hero"}"#))
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::CREATED);
        }

        #[rstest]
        #[tokio::test]
        async fn get_game_returns_404_for_missing_game() {
            let app = create_test_app();
            let game_id = uuid::Uuid::new_v4();

            let response = app
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/v1/games/{}", game_id))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }
    }

    mod leaderboard_endpoint {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn get_leaderboard_returns_200() {
            let app = create_test_app();

            let response = app
                .oneshot(
                    Request::builder()
                        .uri("/api/v1/leaderboard")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
        }
    }

    mod middleware {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn adds_request_id_header() {
            let app = create_test_app();

            let response = app
                .oneshot(
                    Request::builder()
                        .uri("/api/v1/health")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert!(response.headers().contains_key("x-request-id"));
        }

        #[rstest]
        #[tokio::test]
        async fn adds_response_time_header() {
            let app = create_test_app();

            let response = app
                .oneshot(
                    Request::builder()
                        .uri("/api/v1/health")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert!(response.headers().contains_key("x-response-time"));
        }

        #[rstest]
        #[tokio::test]
        async fn preserves_provided_request_id() {
            let app = create_test_app();
            let request_id = "test-request-id-123";

            let response = app
                .oneshot(
                    Request::builder()
                        .uri("/api/v1/health")
                        .header("x-request-id", request_id)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.headers().get("x-request-id").unwrap(), request_id);
        }
    }
}
