use std::collections::HashMap;
use std::env;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use lambars::effect::AsyncIO;
use roguelike_api::routes::create_router;
use roguelike_api::server::{Server, ServerConfig};
use roguelike_api::state::AppState;
use roguelike_domain::game_session::{GameIdentifier, GameSessionEvent, RandomSeed};
use roguelike_workflow::ports::{EventStore, GameSessionRepository, RandomGenerator, SessionCache};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

// =============================================================================
// Mock Implementations for Development
// =============================================================================

// Note: These mock implementations are for development and testing purposes.
// In production, these should be replaced with real database/cache implementations.

#[derive(Debug, Clone, PartialEq, Eq)]
struct InMemoryGameSession {
    identifier: GameIdentifier,
}

impl InMemoryGameSession {
    fn identifier(&self) -> &GameIdentifier {
        &self.identifier
    }
}

#[derive(Clone)]
struct InMemoryRepository {
    sessions: Arc<RwLock<HashMap<GameIdentifier, InMemoryGameSession>>>,
}

impl InMemoryRepository {
    fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl GameSessionRepository for InMemoryRepository {
    type GameSession = InMemoryGameSession;

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
struct InMemoryCache {
    cache: Arc<RwLock<HashMap<GameIdentifier, InMemoryGameSession>>>,
}

impl InMemoryCache {
    fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl SessionCache for InMemoryCache {
    type GameSession = InMemoryGameSession;

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
struct InMemoryEventStore {
    events: Arc<RwLock<HashMap<GameIdentifier, Vec<GameSessionEvent>>>>,
}

impl InMemoryEventStore {
    fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl EventStore for InMemoryEventStore {
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

    fn load_events(&self, session_identifier: &GameIdentifier) -> AsyncIO<Vec<GameSessionEvent>> {
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
struct ThreadSafeRandom {
    counter: Arc<AtomicU64>,
}

impl ThreadSafeRandom {
    fn new() -> Self {
        Self {
            counter: Arc::new(AtomicU64::new(1)),
        }
    }
}

impl RandomGenerator for ThreadSafeRandom {
    fn generate_seed(&self) -> AsyncIO<RandomSeed> {
        let counter = Arc::clone(&self.counter);
        AsyncIO::new(move || async move {
            let value = counter.fetch_add(1, Ordering::SeqCst);
            // Mix with current time for better randomness
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
            RandomSeed::new(value.wrapping_mul(now))
        })
    }

    fn next_u32(&self, seed: &RandomSeed) -> (u32, RandomSeed) {
        // Linear Congruential Generator
        let next_value = seed.value().wrapping_mul(1103515245).wrapping_add(12345);
        let random_value = (next_value >> 16) as u32;
        (random_value, RandomSeed::new(next_value))
    }
}

// =============================================================================
// Main Entry Point
// =============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    init_tracing();

    tracing::info!("Dungeon of Pure Functions - Starting Server");

    // Load configuration from environment
    let config = load_config();

    // Create in-memory implementations (for development)
    let repository = InMemoryRepository::new();
    let cache = InMemoryCache::new();
    let event_store = InMemoryEventStore::new();
    let random = ThreadSafeRandom::new();

    // Create application state
    let state = AppState::new(repository, cache, event_store, random);

    // Create router with all routes and middleware
    let router = create_router(state);

    // Start server
    let server = Server::new(config);
    server.run(router).await
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("roguelike_api=debug,tower_http=debug,info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_target(true).with_thread_ids(true))
        .init();
}

fn load_config() -> ServerConfig {
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    ServerConfig::new(host, port)
}
