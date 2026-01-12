use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

const REDIS_URL: &str = "redis://localhost:6379";
const REDIS_KEY_PREFIX: &str = "dev:roguelike:";

pub async fn create_redis_connection() -> redis::aio::MultiplexedConnection {
    let client = redis::Client::open(REDIS_URL).expect("Failed to create Redis client");
    client
        .get_multiplexed_async_connection()
        .await
        .expect("Failed to connect to Redis")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSession {
    pub game_identifier: String,
    pub player_identifier: String,
    pub current_floor_level: u32,
    pub turn_count: u64,
    pub status: String,
    pub random_seed: u64,
    pub event_sequence: u64,
}

pub async fn get_redis_cache(
    conn: &mut redis::aio::MultiplexedConnection,
    game_id: &str,
) -> Option<CachedSession> {
    let key = format!("{}session:{}", REDIS_KEY_PREFIX, game_id);
    let data: Option<String> = conn.get(&key).await.expect("Failed to get Redis cache");
    data.and_then(|json| serde_json::from_str(&json).ok())
}

pub async fn redis_key_exists(conn: &mut redis::aio::MultiplexedConnection, game_id: &str) -> bool {
    let key = format!("{}session:{}", REDIS_KEY_PREFIX, game_id);
    conn.exists(&key).await.expect("Failed to check Redis key")
}

pub async fn get_redis_ttl(conn: &mut redis::aio::MultiplexedConnection, game_id: &str) -> i64 {
    let key = format!("{}session:{}", REDIS_KEY_PREFIX, game_id);
    conn.ttl(&key).await.expect("Failed to get Redis TTL")
}

pub async fn invalidate_redis_cache(conn: &mut redis::aio::MultiplexedConnection, game_id: &str) {
    let key = format!("{}session:{}", REDIS_KEY_PREFIX, game_id);
    let _: () = conn.del(&key).await.expect("Failed to delete Redis cache");
}

pub async fn flush_redis(conn: &mut redis::aio::MultiplexedConnection) {
    redis::cmd("FLUSHDB")
        .query_async::<()>(conn)
        .await
        .expect("Failed to flush Redis");
}
