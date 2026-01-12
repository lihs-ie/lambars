use std::sync::Arc;
use std::time::Duration;

use lambars::effect::AsyncIO;
use roguelike_domain::common::{Direction, DomainError, ValidationError};
use roguelike_domain::enemy::EntityIdentifier;
use roguelike_domain::game_session::{GameIdentifier, GameOutcome, GameSessionEvent};
use roguelike_domain::item::ItemIdentifier;
use roguelike_workflow::ports::SnapshotStore;
use roguelike_workflow::{
    CreateGameCommand, EventStore, GameSessionRepository, PlayerCommand, ProcessTurnCommand,
    RandomGenerator, SessionCache, SessionStateAccessor, TurnResult, WorkflowError, WorkflowResult,
    create_game, process_turn,
};
use uuid::Uuid;

const DEFAULT_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(300);

// =============================================================================
// NoOpSnapshotStore
// =============================================================================

#[derive(Clone)]
pub struct NoOpSnapshotStore<S> {
    _phantom: std::marker::PhantomData<S>,
}

impl<S> Default for NoOpSnapshotStore<S> {
    fn default() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: Clone + Send + Sync + 'static> SnapshotStore for NoOpSnapshotStore<S> {
    type GameSession = S;

    fn save_snapshot(
        &self,
        _session_identifier: &GameIdentifier,
        _session: &Self::GameSession,
        _sequence: u64,
    ) -> AsyncIO<()> {
        AsyncIO::pure(())
    }

    fn load_latest_snapshot(
        &self,
        _session_identifier: &GameIdentifier,
    ) -> AsyncIO<Option<(Self::GameSession, u64)>> {
        AsyncIO::pure(None)
    }
}

pub struct GameSessionProvider<Repository, Events, Cache, Random>
where
    Repository: GameSessionRepository,
    Events: EventStore,
    Cache: SessionCache<GameSession = Repository::GameSession>,
    Random: RandomGenerator,
{
    repository: Arc<Repository>,
    event_store: Arc<Events>,
    cache: Arc<Cache>,
    random: Arc<Random>,
}

impl<Repository, Events, Cache, Random> Clone
    for GameSessionProvider<Repository, Events, Cache, Random>
where
    Repository: GameSessionRepository,
    Events: EventStore,
    Cache: SessionCache<GameSession = Repository::GameSession>,
    Random: RandomGenerator,
{
    fn clone(&self) -> Self {
        Self {
            repository: Arc::clone(&self.repository),
            event_store: Arc::clone(&self.event_store),
            cache: Arc::clone(&self.cache),
            random: Arc::clone(&self.random),
        }
    }
}

impl<Repository, Events, Cache, Random> GameSessionProvider<Repository, Events, Cache, Random>
where
    Repository: GameSessionRepository,
    Repository::GameSession: SessionStateAccessor,
    Events: EventStore,
    Cache: SessionCache<GameSession = Repository::GameSession>,
    Random: RandomGenerator,
{
    pub fn new(
        repository: Arc<Repository>,
        event_store: Arc<Events>,
        cache: Arc<Cache>,
        random: Arc<Random>,
    ) -> Self {
        Self {
            repository,
            event_store,
            cache,
            random,
        }
    }

    pub fn create_game(
        &self,
        command: CreateGameCommand,
    ) -> AsyncIO<WorkflowResult<Repository::GameSession>> {
        let workflow = create_game(
            &*self.repository,
            &*self.event_store,
            &*self.cache,
            &*self.random,
        );
        workflow(command)
    }

    pub fn get_game(
        &self,
        identifier: &GameIdentifier,
    ) -> AsyncIO<WorkflowResult<Repository::GameSession>> {
        let cache = Arc::clone(&self.cache);
        let repository = Arc::clone(&self.repository);
        let identifier = *identifier;

        AsyncIO::new(move || async move {
            if let Some(session) = cache.get(&identifier).run_async().await {
                return Ok(session);
            }

            repository
                .find_by_id(&identifier)
                .run_async()
                .await
                .ok_or_else(|| WorkflowError::not_found("GameSession", identifier.to_string()))
        })
    }

    pub fn get_game_with_cache(
        &self,
        identifier: &GameIdentifier,
    ) -> AsyncIO<WorkflowResult<Repository::GameSession>> {
        let cache = Arc::clone(&self.cache);
        let repository = Arc::clone(&self.repository);
        let identifier = *identifier;

        AsyncIO::new(move || async move {
            if let Some(session) = cache.get(&identifier).run_async().await {
                return Ok(session);
            }

            let session = repository
                .find_by_id(&identifier)
                .run_async()
                .await
                .ok_or_else(|| WorkflowError::not_found("GameSession", identifier.to_string()))?;

            cache
                .set(&identifier, &session, DEFAULT_CACHE_TIME_TO_LIVE)
                .run_async()
                .await;

            Ok(session)
        })
    }

    pub fn end_game(
        &self,
        identifier: &GameIdentifier,
        outcome: GameOutcome,
    ) -> AsyncIO<WorkflowResult<Repository::GameSession>> {
        let cache = Arc::clone(&self.cache);
        let event_store = Arc::clone(&self.event_store);
        let repository = Arc::clone(&self.repository);
        let identifier = *identifier;

        AsyncIO::new(move || async move {
            // Get current session
            let session = if let Some(session) = cache.get(&identifier).run_async().await {
                session
            } else {
                repository
                    .find_by_id(&identifier)
                    .run_async()
                    .await
                    .ok_or_else(|| {
                        WorkflowError::not_found("GameSession", identifier.to_string())
                    })?
            };

            // End the game
            let ended_session = session.end_game(outcome);

            // Create event and persist
            let event = roguelike_domain::game_session::GameEnded::new(outcome);
            event_store
                .append(&identifier, &[GameSessionEvent::Ended(event)])
                .run_async()
                .await;

            // Invalidate cache
            cache.invalidate(&identifier).run_async().await;

            Ok(ended_session)
        })
    }

    pub fn execute_command(
        &self,
        identifier: &GameIdentifier,
        command: &str,
    ) -> AsyncIO<WorkflowResult<Repository::GameSession>> {
        let cache = Arc::clone(&self.cache);
        let event_store = Arc::clone(&self.event_store);
        let identifier = *identifier;
        let command = command.to_string();

        AsyncIO::new(move || async move {
            // Parse command string to PlayerCommand
            let player_command = parse_command(&command)?;

            // Create ProcessTurnCommand
            let process_command = ProcessTurnCommand::new(identifier, player_command);

            // Create NoOpSnapshotStore for process_turn
            let snapshot_store = NoOpSnapshotStore::<Cache::GameSession>::default();

            // Execute process_turn workflow (dereference Arc to get references to traits)
            let workflow = process_turn(
                &*cache,
                &*event_store,
                &snapshot_store,
                DEFAULT_CACHE_TIME_TO_LIVE,
            );
            let result: TurnResult<Cache::GameSession> =
                workflow(process_command).run_async().await?;

            Ok(result.session)
        })
    }

    pub fn get_events(
        &self,
        identifier: &GameIdentifier,
    ) -> AsyncIO<WorkflowResult<Vec<GameSessionEvent>>> {
        let event_store = Arc::clone(&self.event_store);
        let identifier = *identifier;

        AsyncIO::new(move || async move {
            let events = event_store.load_events(&identifier).run_async().await;
            Ok(events)
        })
    }

    pub fn get_events_since(
        &self,
        identifier: &GameIdentifier,
        sequence: u64,
    ) -> AsyncIO<WorkflowResult<Vec<GameSessionEvent>>> {
        let event_store = Arc::clone(&self.event_store);
        let identifier = *identifier;

        AsyncIO::new(move || async move {
            let events = event_store
                .load_events_since(&identifier, sequence)
                .run_async()
                .await;
            Ok(events)
        })
    }

    pub fn repository(&self) -> &Arc<Repository> {
        &self.repository
    }

    pub fn event_store(&self) -> &Arc<Events> {
        &self.event_store
    }

    pub fn cache(&self) -> &Arc<Cache> {
        &self.cache
    }
}

// =============================================================================
// Command Parsing
// =============================================================================

fn validation_error(field: &str, message: &str) -> WorkflowError {
    WorkflowError::Domain(DomainError::Validation(
        ValidationError::constraint_violation(field.to_string(), message.to_string()),
    ))
}

fn parse_command(command: &str) -> WorkflowResult<PlayerCommand> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err(validation_error("command", "Command cannot be empty"));
    }

    match parts[0].to_lowercase().as_str() {
        "move" => parse_move_command(&parts),
        "attack" => parse_attack_command(&parts),
        "wait" => Ok(PlayerCommand::Wait),
        "use" => parse_use_item_command(&parts),
        "pickup" | "pick" => parse_pickup_item_command(&parts),
        "equip" => parse_equip_item_command(&parts),
        _ => Err(validation_error(
            "command",
            &format!("Unknown command: {}", parts[0]),
        )),
    }
}

fn parse_move_command(parts: &[&str]) -> WorkflowResult<PlayerCommand> {
    if parts.len() < 2 {
        return Err(validation_error(
            "command",
            "Move command requires a direction (north, south, east, west, up, down, left, right)",
        ));
    }

    let direction = match parts[1].to_lowercase().as_str() {
        "north" | "up" | "n" => Direction::Up,
        "south" | "down" | "s" => Direction::Down,
        "east" | "right" | "e" => Direction::Right,
        "west" | "left" | "w" => Direction::Left,
        _ => {
            return Err(validation_error(
                "direction",
                &format!("Invalid direction: {}", parts[1]),
            ));
        }
    };

    Ok(PlayerCommand::Move(direction))
}

fn parse_attack_command(parts: &[&str]) -> WorkflowResult<PlayerCommand> {
    if parts.len() < 2 {
        return Err(validation_error(
            "command",
            "Attack command requires a target ID",
        ));
    }

    let uuid: Uuid = parts[1]
        .parse()
        .map_err(|_| validation_error("target_id", "Invalid target ID format (must be UUID)"))?;
    let target_id = EntityIdentifier::from_uuid(uuid);

    Ok(PlayerCommand::Attack(target_id))
}

fn parse_use_item_command(parts: &[&str]) -> WorkflowResult<PlayerCommand> {
    if parts.len() < 2 {
        return Err(validation_error(
            "command",
            "Use command requires an item ID",
        ));
    }

    let uuid: Uuid = parts[1]
        .parse()
        .map_err(|_| validation_error("item_id", "Invalid item ID format (must be UUID)"))?;
    let item_id = ItemIdentifier::from_uuid(uuid);

    Ok(PlayerCommand::UseItem(item_id))
}

fn parse_pickup_item_command(parts: &[&str]) -> WorkflowResult<PlayerCommand> {
    // Handle both "pickup <id>" and "pick up <id>"
    let item_part = if parts.len() >= 3 && parts[1].to_lowercase() == "up" {
        parts.get(2)
    } else {
        parts.get(1)
    };

    match item_part {
        Some(id) => {
            let uuid: Uuid = id.parse().map_err(|_| {
                validation_error("item_id", "Invalid item ID format (must be UUID)")
            })?;
            let item_id = ItemIdentifier::from_uuid(uuid);
            Ok(PlayerCommand::PickUpItem(item_id))
        }
        None => Err(validation_error(
            "command",
            "Pickup command requires an item ID",
        )),
    }
}

fn parse_equip_item_command(parts: &[&str]) -> WorkflowResult<PlayerCommand> {
    if parts.len() < 2 {
        return Err(validation_error(
            "command",
            "Equip command requires an item ID",
        ));
    }

    let uuid: Uuid = parts[1]
        .parse()
        .map_err(|_| validation_error("item_id", "Invalid item ID format (must be UUID)"))?;
    let item_id = ItemIdentifier::from_uuid(uuid);

    Ok(PlayerCommand::EquipItem(item_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use roguelike_domain::common::TurnCount;
    use roguelike_domain::enemy::Enemy;
    use roguelike_domain::floor::Floor;
    use roguelike_domain::game_session::{GameOutcome, GameStatus, RandomSeed};
    use roguelike_domain::player::Player;
    use rstest::rstest;
    use std::collections::HashMap;
    use std::sync::RwLock;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct MockGameSession {
        identifier: GameIdentifier,
        seed: RandomSeed,
    }

    impl MockGameSession {
        fn new(identifier: GameIdentifier, seed: RandomSeed) -> Self {
            Self { identifier, seed }
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
            1
        }

        fn apply_event(&self, _event: &GameSessionEvent) -> Self {
            self.clone()
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

        fn end_game(&self, _outcome: GameOutcome) -> Self {
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
                if let Some(event_list) = events.read().unwrap().get(&identifier) {
                    for event in event_list {
                        if let GameSessionEvent::Started(started) = event {
                            return Some(MockGameSession::new(identifier, *started.seed()));
                        }
                    }
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

    mod create_game_tests {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn creates_game_successfully() {
            let event_store = MockEventStore::new();
            let repository = MockRepository::with_events(event_store.events_arc());
            let cache = MockCache::new();
            let random = MockRandom::new();

            let provider = GameSessionProvider::new(
                Arc::new(repository),
                Arc::new(event_store),
                Arc::new(cache),
                Arc::new(random),
            );

            let command = CreateGameCommand::new("Hero".to_string(), None);
            let result = provider.create_game(command).run_async().await;

            assert!(result.is_ok());
        }
    }

    mod get_game_tests {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn returns_not_found_for_missing_game() {
            let event_store = MockEventStore::new();
            let repository = MockRepository::new();
            let cache = MockCache::new();
            let random = MockRandom::new();

            let provider = GameSessionProvider::new(
                Arc::new(repository),
                Arc::new(event_store),
                Arc::new(cache),
                Arc::new(random),
            );

            let identifier = GameIdentifier::new();
            let result = provider.get_game(&identifier).run_async().await;

            assert!(result.is_err());
            assert!(result.unwrap_err().is_not_found());
        }

        #[rstest]
        #[tokio::test]
        async fn returns_game_from_cache() {
            let event_store = MockEventStore::new();
            let repository = MockRepository::new();
            let cache = MockCache::new();
            let random = MockRandom::new();

            let identifier = GameIdentifier::new();
            let session = MockGameSession::new(identifier, RandomSeed::new(42));
            cache
                .set(&identifier, &session, Duration::from_secs(60))
                .run_async()
                .await;

            let provider = GameSessionProvider::new(
                Arc::new(repository),
                Arc::new(event_store),
                Arc::new(cache),
                Arc::new(random),
            );

            let result = provider.get_game(&identifier).run_async().await;

            assert!(result.is_ok());
            assert_eq!(result.unwrap(), session);
        }
    }

    mod end_game_tests {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn returns_not_implemented() {
            let event_store = MockEventStore::new();
            let repository = MockRepository::new();
            let cache = MockCache::new();
            let random = MockRandom::new();

            let provider = GameSessionProvider::new(
                Arc::new(repository),
                Arc::new(event_store),
                Arc::new(cache),
                Arc::new(random),
            );

            let identifier = GameIdentifier::new();
            let result = provider
                .end_game(&identifier, GameOutcome::Abandoned)
                .run_async()
                .await;

            assert!(result.is_err());
        }
    }

    mod execute_command_tests {
        use super::*;

        #[rstest]
        #[tokio::test]
        async fn returns_error_for_missing_game() {
            let event_store = MockEventStore::new();
            let repository = MockRepository::new();
            let cache = MockCache::new();
            let random = MockRandom::new();

            let provider = GameSessionProvider::new(
                Arc::new(repository),
                Arc::new(event_store),
                Arc::new(cache),
                Arc::new(random),
            );

            let identifier = GameIdentifier::new();
            let result = provider
                .execute_command(&identifier, "move north")
                .run_async()
                .await;

            assert!(result.is_err());
        }
    }
}
