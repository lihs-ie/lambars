use sqlx::{MySqlPool, Row};

const DATABASE_URL: &str = "mysql://roguelike:roguelikepassword@localhost:3306/roguelike";

pub async fn create_mysql_pool() -> MySqlPool {
    MySqlPool::connect(DATABASE_URL)
        .await
        .expect("Failed to connect to MySQL")
}

#[derive(Debug, Clone)]
pub struct GameSessionRecord {
    pub game_id: String,
    pub player_id: String,
    pub current_floor_level: u32,
    pub turn_count: u64,
    pub status: String,
    pub random_seed: u64,
    pub event_sequence: u64,
}

#[derive(Debug, Clone)]
pub struct GameEventRecord {
    pub event_id: String,
    pub game_id: String,
    pub sequence_number: u64,
    pub event_type: String,
    pub event_data: serde_json::Value,
}

pub async fn query_game_session(pool: &MySqlPool, game_id: &str) -> Option<GameSessionRecord> {
    let row = sqlx::query(
        r#"
        SELECT
            BIN_TO_UUID(game_id) as game_id,
            BIN_TO_UUID(player_id) as player_id,
            current_floor_level,
            turn_count,
            status,
            random_seed,
            event_sequence
        FROM game_sessions
        WHERE game_id = UUID_TO_BIN(?)
        "#,
    )
    .bind(game_id)
    .fetch_optional(pool)
    .await
    .expect("Failed to query game_sessions");

    row.map(|r| GameSessionRecord {
        game_id: r.get("game_id"),
        player_id: r.get("player_id"),
        current_floor_level: r.get("current_floor_level"),
        turn_count: r.get("turn_count"),
        status: r.get("status"),
        random_seed: r.get("random_seed"),
        event_sequence: r.get("event_sequence"),
    })
}

pub async fn query_game_events(pool: &MySqlPool, game_id: &str) -> Vec<GameEventRecord> {
    let rows = sqlx::query(
        r#"
        SELECT
            BIN_TO_UUID(event_id) as event_id,
            BIN_TO_UUID(game_id) as game_id,
            sequence_number,
            event_type,
            event_data
        FROM game_events
        WHERE game_id = UUID_TO_BIN(?)
        ORDER BY sequence_number
        "#,
    )
    .bind(game_id)
    .fetch_all(pool)
    .await
    .expect("Failed to query game_events");

    rows.into_iter()
        .map(|r| GameEventRecord {
            event_id: r.get("event_id"),
            game_id: r.get("game_id"),
            sequence_number: r.get("sequence_number"),
            event_type: r.get("event_type"),
            event_data: r.get("event_data"),
        })
        .collect()
}

pub async fn count_game_events(pool: &MySqlPool, game_id: &str) -> i64 {
    let row = sqlx::query(
        r#"
        SELECT COUNT(*) as count
        FROM game_events
        WHERE game_id = UUID_TO_BIN(?)
        "#,
    )
    .bind(game_id)
    .fetch_one(pool)
    .await
    .expect("Failed to count game_events");

    row.get("count")
}

pub async fn cleanup_game_data(pool: &MySqlPool, game_id: &str) {
    sqlx::query("DELETE FROM game_sessions WHERE game_id = UUID_TO_BIN(?)")
        .bind(game_id)
        .execute(pool)
        .await
        .expect("Failed to delete game_sessions");
}

pub async fn truncate_all_tables(pool: &MySqlPool) {
    let tables_in_order = vec![
        "game_events",
        "game_snapshots",
        "items",
        "enemies",
        "players",
        "floors",
        "game_sessions",
    ];

    for table in tables_in_order {
        sqlx::query(&format!("DELETE FROM {}", table))
            .execute(pool)
            .await
            .unwrap_or_else(|e| panic!("Failed to delete from {}: {}", table, e));
    }
}
