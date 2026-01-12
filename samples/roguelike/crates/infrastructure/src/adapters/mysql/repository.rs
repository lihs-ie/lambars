use lambars::effect::AsyncIO;
use roguelike_domain::common::TurnCount;
use roguelike_domain::enemy::Enemy;
use roguelike_domain::floor::Floor;
use roguelike_domain::game_session::{
    GameIdentifier, GameOutcome, GameSession, GameSessionEvent, GameStatus, RandomSeed,
};
use roguelike_domain::player::Player;
use roguelike_workflow::SessionStateAccessor;
use roguelike_workflow::ports::GameSessionRepository;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::MySqlPool;

// =============================================================================
// MySqlGameSessionRepository
// =============================================================================

#[derive(Clone)]
pub struct MySqlGameSessionRepository {
    pool: MySqlPool,
}

// =============================================================================
// Constructors
// =============================================================================

impl MySqlGameSessionRepository {
    #[must_use]
    pub const fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    #[must_use]
    pub const fn pool(&self) -> &MySqlPool {
        &self.pool
    }
}

// =============================================================================
// GameSessionRepository Implementation
// =============================================================================

impl GameSessionRepository for MySqlGameSessionRepository {
    type GameSession = GameSessionRecord;

    fn find_by_id(&self, identifier: &GameIdentifier) -> AsyncIO<Option<Self::GameSession>> {
        let pool = self.pool.clone();
        let game_id_str = identifier.to_string();

        AsyncIO::new(move || async move {
            // Parse the UUID string to get the binary bytes
            let game_uuid = match Uuid::parse_str(&game_id_str) {
                Ok(uuid) => uuid,
                Err(error) => {
                    tracing::error!("Failed to parse game identifier: {}", error);
                    return None;
                }
            };

            let result = sqlx::query(
                r#"
                SELECT
                    BIN_TO_UUID(game_id) as game_id,
                    BIN_TO_UUID(player_id) as player_id,
                    current_floor_level,
                    turn_count,
                    status,
                    random_seed,
                    event_sequence,
                    created_at,
                    updated_at
                FROM game_sessions
                WHERE game_id = UUID_TO_BIN(?)
                "#,
            )
            .bind(game_uuid.to_string())
            .fetch_optional(pool.as_inner())
            .await;

            match result {
                Ok(Some(row)) => {
                    let record = GameSessionRecord::from_row_with_string_uuid(&row);
                    Some(record)
                }
                Ok(None) => None,
                Err(error) => {
                    tracing::error!("Failed to find game session by id: {}", error);
                    None
                }
            }
        })
    }

    fn save(&self, session: &Self::GameSession) -> AsyncIO<()> {
        let pool = self.pool.clone();
        let session = session.clone();

        AsyncIO::new(move || async move {
            // Parse UUIDs from strings to get binary representation
            let game_uuid = match Uuid::parse_str(&session.game_id) {
                Ok(uuid) => uuid,
                Err(error) => {
                    tracing::error!("Failed to parse game_id: {}", error);
                    return;
                }
            };
            let player_uuid = match Uuid::parse_str(&session.player_id) {
                Ok(uuid) => uuid,
                Err(error) => {
                    tracing::error!("Failed to parse player_id: {}", error);
                    return;
                }
            };

            let result = sqlx::query(
                r#"
                INSERT INTO game_sessions (game_id, player_id, current_floor_level, turn_count, status, random_seed, event_sequence)
                VALUES (UUID_TO_BIN(?), UUID_TO_BIN(?), ?, ?, ?, ?, ?)
                ON DUPLICATE KEY UPDATE
                    current_floor_level = VALUES(current_floor_level),
                    turn_count = VALUES(turn_count),
                    status = VALUES(status),
                    event_sequence = VALUES(event_sequence)
                "#,
            )
            .bind(game_uuid.to_string())
            .bind(player_uuid.to_string())
            .bind(session.current_floor_level)
            .bind(session.turn_count as i64)
            .bind(&session.status)
            .bind(session.random_seed as i64)
            .bind(session.event_sequence as i64)
            .execute(pool.as_inner())
            .await;

            if let Err(error) = result {
                tracing::error!("Failed to save game session: {}", error);
            }
        })
    }

    fn delete(&self, identifier: &GameIdentifier) -> AsyncIO<()> {
        let pool = self.pool.clone();
        let game_id_str = identifier.to_string();

        AsyncIO::new(move || async move {
            // Parse the UUID string to get the binary bytes
            let game_uuid = match Uuid::parse_str(&game_id_str) {
                Ok(uuid) => uuid,
                Err(error) => {
                    tracing::error!("Failed to parse game identifier: {}", error);
                    return;
                }
            };

            let result = sqlx::query(
                r#"
                DELETE FROM game_sessions
                WHERE game_id = UUID_TO_BIN(?)
                "#,
            )
            .bind(game_uuid.to_string())
            .execute(pool.as_inner())
            .await;

            if let Err(error) = result {
                tracing::error!("Failed to delete game session: {}", error);
            }
        })
    }

    fn list_active(&self) -> AsyncIO<Vec<GameIdentifier>> {
        let pool = self.pool.clone();

        AsyncIO::new(move || async move {
            let result = sqlx::query(
                r#"
                SELECT BIN_TO_UUID(game_id) as game_id
                FROM game_sessions
                WHERE status = 'in_progress'
                "#,
            )
            .fetch_all(pool.as_inner())
            .await;

            match result {
                Ok(rows) => rows
                    .iter()
                    .filter_map(|row| {
                        let game_id: String = row.get("game_id");
                        game_id.parse().ok()
                    })
                    .collect(),
                Err(error) => {
                    tracing::error!("Failed to list active game sessions: {}", error);
                    Vec::new()
                }
            }
        })
    }
}

// =============================================================================
// GameSessionRecord
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "GameSessionRecordRaw")]
pub struct GameSessionRecord {
    pub game_id: String,
    #[serde(skip)]
    game_identifier: GameIdentifier,
    pub player_id: String,
    pub current_floor_level: i32,
    pub turn_count: u64,
    pub status: String,
    pub random_seed: u64,
    pub event_sequence: u64,
}

#[derive(Deserialize)]
struct GameSessionRecordRaw {
    game_id: String,
    player_id: String,
    current_floor_level: i32,
    turn_count: u64,
    status: String,
    random_seed: u64,
    event_sequence: u64,
}

impl From<GameSessionRecordRaw> for GameSessionRecord {
    fn from(raw: GameSessionRecordRaw) -> Self {
        let game_identifier = raw
            .game_id
            .parse()
            .unwrap_or_else(|_| GameIdentifier::new());
        Self {
            game_id: raw.game_id,
            game_identifier,
            player_id: raw.player_id,
            current_floor_level: raw.current_floor_level,
            turn_count: raw.turn_count,
            status: raw.status,
            random_seed: raw.random_seed,
            event_sequence: raw.event_sequence,
        }
    }
}

impl GameSessionRecord {
    #[must_use]
    pub fn new(
        game_id: String,
        player_id: String,
        current_floor_level: i32,
        turn_count: u64,
        status: String,
        random_seed: u64,
        event_sequence: u64,
    ) -> Self {
        let game_identifier = game_id.parse().unwrap_or_else(|_| GameIdentifier::new());
        Self {
            game_id,
            game_identifier,
            player_id,
            current_floor_level,
            turn_count,
            status,
            random_seed,
            event_sequence,
        }
    }

    fn from_row_with_string_uuid(row: &sqlx::mysql::MySqlRow) -> Self {
        let game_id: String = row.get("game_id");
        let player_id: String = row.get("player_id");
        let game_identifier = game_id.parse().unwrap_or_else(|_| GameIdentifier::new());

        Self {
            game_id,
            game_identifier,
            player_id,
            current_floor_level: row.get::<u32, _>("current_floor_level") as i32,
            turn_count: row.get::<u64, _>("turn_count"),
            status: row.get("status"),
            random_seed: row.get::<u64, _>("random_seed"),
            event_sequence: row.get::<u64, _>("event_sequence"),
        }
    }

    #[must_use]
    pub fn from_game_session(session: &GameSession, player_id: &str) -> Self {
        Self {
            game_id: session.identifier().to_string(),
            game_identifier: *session.identifier(),
            player_id: player_id.to_string(),
            current_floor_level: session.current_floor().level().value() as i32,
            turn_count: session.turn_count().value(),
            status: status_to_string(session.status()),
            random_seed: session.seed().value(),
            event_sequence: session.event_sequence(),
        }
    }

    #[must_use]
    pub fn game_identifier(&self) -> Option<GameIdentifier> {
        self.game_id.parse().ok()
    }

    #[must_use]
    pub fn random_seed_value(&self) -> RandomSeed {
        RandomSeed::new(self.random_seed)
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        self.status == "in_progress" || self.status == "paused"
    }
}

// =============================================================================
// SessionStateAccessor Implementation
// =============================================================================

impl SessionStateAccessor for GameSessionRecord {
    fn status(&self) -> GameStatus {
        string_to_status(&self.status)
    }

    fn identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    fn event_sequence(&self) -> u64 {
        self.event_sequence
    }

    fn apply_event(&self, _event: &GameSessionEvent) -> Self {
        self.clone()
    }

    fn player(&self) -> &Player {
        unimplemented!(
            "GameSessionRecord does not contain full Player data; use GameSession domain object instead"
        )
    }

    fn current_floor(&self) -> &Floor {
        unimplemented!(
            "GameSessionRecord does not contain full Floor data; use GameSession domain object instead"
        )
    }

    fn enemies(&self) -> &[Enemy] {
        unimplemented!(
            "GameSessionRecord does not contain Enemy data; use GameSession domain object instead"
        )
    }

    fn turn_count(&self) -> TurnCount {
        TurnCount::new(self.turn_count)
    }

    fn seed(&self) -> &RandomSeed {
        unimplemented!(
            "GameSessionRecord does not store RandomSeed by reference; use random_seed_value() method instead"
        )
    }

    fn with_player(&self, _player: Player) -> Self {
        unimplemented!(
            "GameSessionRecord is a database record; use GameSession domain object for mutations"
        )
    }

    fn with_floor(&self, _floor: Floor) -> Self {
        unimplemented!(
            "GameSessionRecord is a database record; use GameSession domain object for mutations"
        )
    }

    fn with_enemies(&self, _enemies: Vec<Enemy>) -> Self {
        unimplemented!(
            "GameSessionRecord is a database record; use GameSession domain object for mutations"
        )
    }

    fn increment_turn(&self) -> Self {
        Self {
            turn_count: self.turn_count + 1,
            ..self.clone()
        }
    }

    fn end_game(&self, outcome: GameOutcome) -> Self {
        Self {
            status: match outcome {
                GameOutcome::Victory => "victory".to_string(),
                GameOutcome::Defeat | GameOutcome::Abandoned => "defeat".to_string(),
            },
            ..self.clone()
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

fn status_to_string(status: &GameStatus) -> String {
    match status {
        GameStatus::InProgress => "in_progress".to_string(),
        GameStatus::Victory => "victory".to_string(),
        GameStatus::Defeat => "defeat".to_string(),
        GameStatus::Paused => "paused".to_string(),
    }
}

#[allow(dead_code)]
fn string_to_status(status: &str) -> GameStatus {
    match status {
        "in_progress" => GameStatus::InProgress,
        "victory" => GameStatus::Victory,
        "defeat" => GameStatus::Defeat,
        "paused" => GameStatus::Paused,
        _ => GameStatus::InProgress,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // GameSessionRecord Tests
    // =========================================================================

    mod game_session_record {
        use super::*;

        #[rstest]
        fn new_creates_record() {
            let record = GameSessionRecord::new(
                "550e8400-e29b-41d4-a716-446655440000".to_string(),
                "660e8400-e29b-41d4-a716-446655440001".to_string(),
                1,
                10,
                "in_progress".to_string(),
                12345,
                5,
            );

            assert_eq!(record.game_id, "550e8400-e29b-41d4-a716-446655440000");
            assert_eq!(record.player_id, "660e8400-e29b-41d4-a716-446655440001");
            assert_eq!(record.current_floor_level, 1);
            assert_eq!(record.turn_count, 10);
            assert_eq!(record.status, "in_progress");
            assert_eq!(record.random_seed, 12345);
            assert_eq!(record.event_sequence, 5);
        }

        #[rstest]
        fn game_identifier_parses_valid_uuid() {
            let record = GameSessionRecord::new(
                "550e8400-e29b-41d4-a716-446655440000".to_string(),
                "player".to_string(),
                1,
                0,
                "in_progress".to_string(),
                0,
                0,
            );

            assert!(record.game_identifier().is_some());
        }

        #[rstest]
        fn game_identifier_returns_none_for_invalid_uuid() {
            let record = GameSessionRecord::new(
                "invalid-uuid".to_string(),
                "player".to_string(),
                1,
                0,
                "in_progress".to_string(),
                0,
                0,
            );

            assert!(record.game_identifier().is_none());
        }

        #[rstest]
        fn random_seed_value_returns_correct_seed() {
            let record = GameSessionRecord::new(
                "550e8400-e29b-41d4-a716-446655440000".to_string(),
                "player".to_string(),
                1,
                0,
                "in_progress".to_string(),
                42,
                0,
            );

            assert_eq!(record.random_seed_value().value(), 42);
        }

        #[rstest]
        #[case("in_progress", true)]
        #[case("paused", true)]
        #[case("victory", false)]
        #[case("defeat", false)]
        fn is_active_returns_correct_value(#[case] status: &str, #[case] expected: bool) {
            let record = GameSessionRecord::new(
                "550e8400-e29b-41d4-a716-446655440000".to_string(),
                "player".to_string(),
                1,
                0,
                status.to_string(),
                0,
                0,
            );

            assert_eq!(record.is_active(), expected);
        }

        #[rstest]
        fn clone_creates_independent_copy() {
            let record = GameSessionRecord::new(
                "550e8400-e29b-41d4-a716-446655440000".to_string(),
                "player".to_string(),
                1,
                10,
                "in_progress".to_string(),
                12345,
                5,
            );

            let cloned = record.clone();

            assert_eq!(record, cloned);
        }

        #[rstest]
        fn equality() {
            let record1 = GameSessionRecord::new(
                "550e8400-e29b-41d4-a716-446655440000".to_string(),
                "player".to_string(),
                1,
                10,
                "in_progress".to_string(),
                12345,
                5,
            );

            let record2 = GameSessionRecord::new(
                "550e8400-e29b-41d4-a716-446655440000".to_string(),
                "player".to_string(),
                1,
                10,
                "in_progress".to_string(),
                12345,
                5,
            );

            let record3 = GameSessionRecord::new(
                "660e8400-e29b-41d4-a716-446655440001".to_string(),
                "player".to_string(),
                1,
                10,
                "in_progress".to_string(),
                12345,
                5,
            );

            assert_eq!(record1, record2);
            assert_ne!(record1, record3);
        }

        #[rstest]
        fn debug_format() {
            let record = GameSessionRecord::new(
                "550e8400-e29b-41d4-a716-446655440000".to_string(),
                "player".to_string(),
                1,
                10,
                "in_progress".to_string(),
                12345,
                5,
            );

            let debug = format!("{:?}", record);
            assert!(debug.contains("GameSessionRecord"));
            assert!(debug.contains("550e8400-e29b-41d4-a716-446655440000"));
        }
    }

    // =========================================================================
    // Status Conversion Tests
    // =========================================================================

    mod status_conversion {
        use super::*;

        #[rstest]
        #[case(GameStatus::InProgress, "in_progress")]
        #[case(GameStatus::Victory, "victory")]
        #[case(GameStatus::Defeat, "defeat")]
        #[case(GameStatus::Paused, "paused")]
        fn status_to_string_converts_correctly(#[case] status: GameStatus, #[case] expected: &str) {
            assert_eq!(status_to_string(&status), expected);
        }

        #[rstest]
        #[case("in_progress", GameStatus::InProgress)]
        #[case("victory", GameStatus::Victory)]
        #[case("defeat", GameStatus::Defeat)]
        #[case("paused", GameStatus::Paused)]
        fn string_to_status_converts_correctly(#[case] input: &str, #[case] expected: GameStatus) {
            assert_eq!(string_to_status(input), expected);
        }

        #[rstest]
        fn string_to_status_defaults_to_in_progress() {
            assert_eq!(string_to_status("unknown"), GameStatus::InProgress);
        }

        #[rstest]
        fn roundtrip_conversion() {
            let statuses = vec![
                GameStatus::InProgress,
                GameStatus::Victory,
                GameStatus::Defeat,
                GameStatus::Paused,
            ];

            for status in statuses {
                let string = status_to_string(&status);
                let back = string_to_status(&string);
                assert_eq!(status, back);
            }
        }
    }

    // =========================================================================
    // MySqlGameSessionRepository Tests
    // =========================================================================

    mod mysql_game_session_repository {
        use super::*;

        #[rstest]
        fn repository_is_clone() {
            fn assert_clone<T: Clone>() {}
            assert_clone::<MySqlGameSessionRepository>();
        }

        #[rstest]
        fn repository_is_send() {
            fn assert_send<T: Send>() {}
            assert_send::<MySqlGameSessionRepository>();
        }

        #[rstest]
        fn repository_is_sync() {
            fn assert_sync<T: Sync>() {}
            assert_sync::<MySqlGameSessionRepository>();
        }
    }
}
