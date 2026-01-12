use lambars::effect::AsyncIO;
use roguelike_domain::game_session::{GameIdentifier, GameSessionEvent, GameStarted, RandomSeed};
use roguelike_workflow::ports::EventStore;
use sqlx::Row;
use uuid::Uuid;

use super::MySqlPool;

#[derive(Clone)]
pub struct MySqlEventStore {
    pool: MySqlPool,
}

impl MySqlEventStore {
    #[must_use]
    pub const fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    #[must_use]
    pub const fn pool(&self) -> &MySqlPool {
        &self.pool
    }
}

impl EventStore for MySqlEventStore {
    fn append(
        &self,
        session_identifier: &GameIdentifier,
        events: &[GameSessionEvent],
    ) -> AsyncIO<()> {
        let pool = self.pool.clone();
        let game_id_str = session_identifier.to_string();
        let events = events.to_vec();

        AsyncIO::new(move || async move {
            let game_uuid = match Uuid::parse_str(&game_id_str) {
                Ok(uuid) => uuid,
                Err(error) => {
                    tracing::error!("Failed to parse game identifier: {}", error);
                    return;
                }
            };

            for (sequence, event) in events.iter().enumerate() {
                if let GameSessionEvent::Started(started) = event {
                    if let Err(error) = create_session_record(
                        pool.as_inner(),
                        game_uuid,
                        started.seed().value(),
                    )
                    .await
                    {
                        tracing::error!("Failed to create game session record: {}", error);
                        return;
                    }
                }

                let (event_type, event_data) = serialize_event(event);

                let event_id = Uuid::new_v4();

                let result = sqlx::query(
                    r#"
                    INSERT INTO game_events (event_id, game_id, sequence_number, event_type, event_data)
                    VALUES (?, ?, ?, ?, ?)
                    "#,
                )
                .bind(event_id)
                .bind(game_uuid)
                .bind(sequence as i64)
                .bind(event_type)
                .bind(event_data)
                .execute(pool.as_inner())
                .await;

                if let Err(error) = result {
                    tracing::error!("Failed to append event: {}", error);
                }
            }
        })
    }

    fn load_events(&self, session_identifier: &GameIdentifier) -> AsyncIO<Vec<GameSessionEvent>> {
        let pool = self.pool.clone();
        let game_id_str = session_identifier.to_string();

        AsyncIO::new(move || async move {
            let game_uuid = match Uuid::parse_str(&game_id_str) {
                Ok(uuid) => uuid,
                Err(error) => {
                    tracing::error!("Failed to parse game identifier: {}", error);
                    return Vec::new();
                }
            };

            let result = sqlx::query(
                r#"
                SELECT sequence_number, event_type, event_data
                FROM game_events
                WHERE game_id = ?
                ORDER BY sequence_number
                "#,
            )
            .bind(game_uuid)
            .fetch_all(pool.as_inner())
            .await;

            match result {
                Ok(rows) => rows
                    .iter()
                    .filter_map(|row| {
                        let sequence: u64 = row.get("sequence_number");
                        let event_type: String = row.get("event_type");
                        let event_data: serde_json::Value = row.get("event_data");
                        let event_data_str = event_data.to_string();

                        parse_event(&game_id_str, sequence, &event_type, &event_data_str)
                    })
                    .collect(),
                Err(error) => {
                    tracing::error!("Failed to load events: {}", error);
                    Vec::new()
                }
            }
        })
    }

    fn load_events_since(
        &self,
        session_identifier: &GameIdentifier,
        sequence: u64,
    ) -> AsyncIO<Vec<GameSessionEvent>> {
        let pool = self.pool.clone();
        let game_id_str = session_identifier.to_string();

        AsyncIO::new(move || async move {
            let game_uuid = match Uuid::parse_str(&game_id_str) {
                Ok(uuid) => uuid,
                Err(error) => {
                    tracing::error!("Failed to parse game identifier: {}", error);
                    return Vec::new();
                }
            };

            let result = sqlx::query(
                r#"
                SELECT sequence_number, event_type, event_data
                FROM game_events
                WHERE game_id = ? AND sequence_number >= ?
                ORDER BY sequence_number
                "#,
            )
            .bind(game_uuid)
            .bind(sequence as i64)
            .fetch_all(pool.as_inner())
            .await;

            match result {
                Ok(rows) => rows
                    .iter()
                    .filter_map(|row| {
                        let seq: u64 = row.get("sequence_number");
                        let event_type: String = row.get("event_type");
                        let event_data: serde_json::Value = row.get("event_data");
                        let event_data_str = event_data.to_string();

                        parse_event(&game_id_str, seq, &event_type, &event_data_str)
                    })
                    .collect(),
                Err(error) => {
                    tracing::error!("Failed to load events since sequence {}: {}", sequence, error);
                    Vec::new()
                }
            }
        })
    }
}

async fn create_session_record(
    pool: &sqlx::MySqlPool,
    game_uuid: Uuid,
    seed: u64,
) -> Result<(), sqlx::Error> {
    let player_uuid = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO game_sessions (game_id, player_id, current_floor_level, turn_count, status, random_seed, event_sequence)
        VALUES (?, ?, 1, 0, 'in_progress', ?, 0)
        "#,
    )
    .bind(game_uuid)
    .bind(player_uuid)
    .bind(seed as i64)
    .execute(pool)
    .await?;

    Ok(())
}

fn serialize_event(event: &GameSessionEvent) -> (&'static str, String) {
    match event {
        GameSessionEvent::Started(started) => {
            let data = serde_json::json!({
                "game_identifier": started.game_identifier().to_string(),
                "seed": started.seed().value()
            });
            ("Started", data.to_string())
        }
        _ => ("Unknown", "{}".to_string()),
    }
}

fn parse_event(game_id: &str, _sequence: u64, event_type: &str, event_data: &str) -> Option<GameSessionEvent> {
    match event_type {
        "Started" => {
            let data: serde_json::Value = serde_json::from_str(event_data).ok()?;
            let seed = data["seed"].as_u64()?;
            let game_identifier: GameIdentifier = game_id.parse().ok()?;

            Some(GameSessionEvent::Started(GameStarted::new(
                game_identifier,
                RandomSeed::new(seed),
            )))
        }
        _ => {
            tracing::warn!("Unknown event type: {}", event_type);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn parse_started_event() {
        let game_id = "550e8400-e29b-41d4-a716-446655440000";
        let event_data = r#"{"game_identifier": "550e8400-e29b-41d4-a716-446655440000", "seed": 42}"#;
        let event = parse_event(game_id, 0, "Started", event_data);

        assert!(event.is_some());
        match event.unwrap() {
            GameSessionEvent::Started(started) => {
                assert_eq!(started.seed().value(), 42);
            }
            _ => panic!("Expected Started event"),
        }
    }

    #[rstest]
    fn parse_unknown_event_returns_none() {
        let event = parse_event("game-id", 0, "Unknown", "{}");
        assert!(event.is_none());
    }

    #[rstest]
    fn parse_invalid_json_returns_none() {
        let event = parse_event("game-id", 0, "Started", "invalid json");
        assert!(event.is_none());
    }

    #[rstest]
    fn event_store_is_clone() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<MySqlEventStore>();
    }

    #[rstest]
    fn event_store_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<MySqlEventStore>();
    }

    #[rstest]
    fn event_store_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<MySqlEventStore>();
    }
}
