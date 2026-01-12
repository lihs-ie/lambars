//! MySQL implementation of GameSessionRepository.
//!
//! This module provides a MySQL-backed implementation of the
//! [`GameSessionRepository`] trait for persistent game session storage.

use lambars::effect::AsyncIO;
use roguelike_domain::game_session::{GameIdentifier, GameSession, GameStatus, RandomSeed};
use roguelike_workflow::ports::GameSessionRepository;
use sqlx::Row;

use super::MySqlPool;

// =============================================================================
// MySqlGameSessionRepository
// =============================================================================

/// MySQL-backed game session repository.
///
/// This struct provides persistent storage for game sessions using MySQL.
/// It implements the [`GameSessionRepository`] trait from the workflow layer.
///
/// # Note
///
/// The current implementation stores only session metadata (identifiers, status,
/// turn count, etc.). Full game session reconstruction with player, floor, and
/// enemy data requires additional repository implementations or a more complex
/// serialization strategy.
///
/// # Examples
///
/// ```rust,ignore
/// use roguelike_infrastructure::adapters::mysql::{MySqlPool, MySqlPoolFactory, MySqlPoolConfig};
/// use roguelike_infrastructure::adapters::mysql::MySqlGameSessionRepository;
///
/// let config = MySqlPoolConfig::with_url("mysql://localhost/roguelike");
/// let pool = MySqlPoolFactory::create_pool(&config)?;
/// let repository = MySqlGameSessionRepository::new(pool);
/// ```
#[derive(Clone)]
pub struct MySqlGameSessionRepository {
    pool: MySqlPool,
}

// =============================================================================
// Constructors
// =============================================================================

impl MySqlGameSessionRepository {
    /// Creates a new MySQL game session repository.
    ///
    /// # Arguments
    ///
    /// * `pool` - The MySQL connection pool to use for database operations.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use roguelike_infrastructure::adapters::mysql::{MySqlPool, MySqlGameSessionRepository};
    ///
    /// let pool = // ... obtain pool
    /// let repository = MySqlGameSessionRepository::new(pool);
    /// ```
    #[must_use]
    pub const fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Returns a reference to the underlying connection pool.
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
        let game_id = identifier.to_string();

        AsyncIO::new(move || async move {
            let result = sqlx::query(
                r#"
                SELECT game_id, player_id, current_floor_level, turn_count, status, random_seed, event_sequence, created_at, updated_at
                FROM game_sessions
                WHERE game_id = ?
                "#,
            )
            .bind(&game_id)
            .fetch_optional(pool.as_inner())
            .await;

            match result {
                Ok(Some(row)) => {
                    let record = GameSessionRecord::from_row(&row);
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
            let result = sqlx::query(
                r#"
                INSERT INTO game_sessions (game_id, player_id, current_floor_level, turn_count, status, random_seed, event_sequence)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                ON DUPLICATE KEY UPDATE
                    current_floor_level = VALUES(current_floor_level),
                    turn_count = VALUES(turn_count),
                    status = VALUES(status),
                    event_sequence = VALUES(event_sequence)
                "#,
            )
            .bind(&session.game_id)
            .bind(&session.player_id)
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
        let game_id = identifier.to_string();

        AsyncIO::new(move || async move {
            let result = sqlx::query(
                r#"
                DELETE FROM game_sessions
                WHERE game_id = ?
                "#,
            )
            .bind(&game_id)
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
                SELECT game_id
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

/// A simplified record type for game session persistence.
///
/// This struct represents the database row structure for game sessions.
/// It contains only the metadata needed for basic game session tracking.
///
/// # Note
///
/// Full game session reconstruction with all domain objects (Player, Floor,
/// Enemies, etc.) would require either:
/// - Separate repositories for each entity type
/// - JSON/binary serialization of the full GameSession
/// - Event sourcing to reconstruct state from events
///
/// The current implementation uses this simplified record for basic CRUD
/// operations while the full persistence strategy is being developed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameSessionRecord {
    /// The unique game identifier (UUID string).
    pub game_id: String,
    /// The player identifier (UUID string).
    pub player_id: String,
    /// Current floor level.
    pub current_floor_level: i32,
    /// Current turn count.
    pub turn_count: u64,
    /// Game status as a string.
    pub status: String,
    /// Random seed for game reproducibility.
    pub random_seed: u64,
    /// Event sequence number.
    pub event_sequence: u64,
}

impl GameSessionRecord {
    /// Creates a new game session record.
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
        Self {
            game_id,
            player_id,
            current_floor_level,
            turn_count,
            status,
            random_seed,
            event_sequence,
        }
    }

    /// Creates a record from a database row.
    fn from_row(row: &sqlx::mysql::MySqlRow) -> Self {
        Self {
            game_id: row.get("game_id"),
            player_id: row.get("player_id"),
            current_floor_level: row.get("current_floor_level"),
            turn_count: row.get::<i64, _>("turn_count") as u64,
            status: row.get("status"),
            random_seed: row.get::<i64, _>("random_seed") as u64,
            event_sequence: row.get::<i64, _>("event_sequence") as u64,
        }
    }

    /// Creates a record from a full GameSession.
    ///
    /// This extracts the metadata needed for persistence from the full
    /// domain object.
    #[must_use]
    pub fn from_game_session(session: &GameSession, player_id: &str) -> Self {
        Self {
            game_id: session.identifier().to_string(),
            player_id: player_id.to_string(),
            current_floor_level: session.current_floor().level().value() as i32,
            turn_count: session.turn_count().value(),
            status: status_to_string(session.status()),
            random_seed: session.seed().value(),
            event_sequence: session.event_sequence(),
        }
    }

    /// Returns the game identifier parsed from the stored string.
    ///
    /// # Errors
    ///
    /// Returns `None` if the stored game_id is not a valid UUID.
    #[must_use]
    pub fn game_identifier(&self) -> Option<GameIdentifier> {
        self.game_id.parse().ok()
    }

    /// Returns the random seed as a domain type.
    #[must_use]
    pub fn random_seed_value(&self) -> RandomSeed {
        RandomSeed::new(self.random_seed)
    }

    /// Returns whether the session is active (in_progress or paused).
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.status == "in_progress" || self.status == "paused"
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Converts a GameStatus to its string representation for database storage.
fn status_to_string(status: &GameStatus) -> String {
    match status {
        GameStatus::InProgress => "in_progress".to_string(),
        GameStatus::Victory => "victory".to_string(),
        GameStatus::Defeat => "defeat".to_string(),
        GameStatus::Paused => "paused".to_string(),
    }
}

/// Converts a string from the database to a GameStatus.
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
        fn status_to_string_converts_correctly(
            #[case] status: GameStatus,
            #[case] expected: &str,
        ) {
            assert_eq!(status_to_string(&status), expected);
        }

        #[rstest]
        #[case("in_progress", GameStatus::InProgress)]
        #[case("victory", GameStatus::Victory)]
        #[case("defeat", GameStatus::Defeat)]
        #[case("paused", GameStatus::Paused)]
        fn string_to_status_converts_correctly(
            #[case] input: &str,
            #[case] expected: GameStatus,
        ) {
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
