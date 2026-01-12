use std::time::Duration;

use super::{
    TestClient, create_mysql_pool, create_redis_connection, flush_redis, truncate_all_tables,
};

const API_BASE_URL: &str = "http://localhost:8080";

pub struct IntegrationTestContext {
    pub client: TestClient,
    pub mysql_pool: sqlx::MySqlPool,
    pub redis_connection: redis::aio::MultiplexedConnection,
}

impl IntegrationTestContext {
    pub async fn new() -> Self {
        wait_for_services().await;

        let mysql_pool = create_mysql_pool().await;
        let redis_connection = create_redis_connection().await;

        Self {
            client: TestClient::new(API_BASE_URL),
            mysql_pool,
            redis_connection,
        }
    }

    pub async fn cleanup_all(&mut self) {
        truncate_all_tables(&self.mysql_pool).await;
        flush_redis(&mut self.redis_connection).await;
    }

    pub async fn create_game(&self, player_name: &str) -> String {
        let request = serde_json::json!({
            "player_name": player_name,
        });

        let response = self.client.post("/api/v1/games", &request).await;
        assert!(
            response.is_success(),
            "Failed to create game: {:?}",
            response.body
        );

        response.body["game_id"]
            .as_str()
            .expect("game_id not found in response")
            .to_string()
    }

    pub async fn create_game_with_seed(&self, player_name: &str, seed: u64) -> String {
        let request = serde_json::json!({
            "player_name": player_name,
            "seed": seed,
        });

        let response = self.client.post("/api/v1/games", &request).await;
        assert!(
            response.is_success(),
            "Failed to create game: {:?}",
            response.body
        );

        response.body["game_id"]
            .as_str()
            .expect("game_id not found in response")
            .to_string()
    }
}

async fn wait_for_services() {
    let client = reqwest::Client::new();
    let health_url = format!("{}/api/v1/health", API_BASE_URL);

    for i in 0..30 {
        match client.get(&health_url).send().await {
            Ok(response) if response.status().is_success() => {
                return;
            }
            _ => {
                if i == 29 {
                    panic!(
                        "Services did not start in time. Make sure Docker Compose is running: \
                         cd samples/roguelike && docker-compose up -d"
                    );
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}
