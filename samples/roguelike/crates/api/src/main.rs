use std::env;

use roguelike_api::routes::create_router;
use roguelike_api::server::{Server, ServerConfig};
use roguelike_api::state::AppState;
use roguelike_infrastructure::adapters::SystemRandomGenerator;
use roguelike_infrastructure::adapters::mysql::{
    GameSessionRecord, MySqlEventStore, MySqlGameSessionRepository, MySqlPool, MySqlPoolConfig,
    MySqlPoolFactory,
};
use roguelike_infrastructure::adapters::redis::{
    RedisConfig, RedisConnectionFactory, RedisSessionCache,
};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    tracing::info!("Dungeon of Pure Functions - Starting Server");

    let config = load_config();

    let mysql_pool = create_mysql_pool().await?;
    let redis_connection = create_redis_connection()?;

    let repository = MySqlGameSessionRepository::new(mysql_pool.clone());
    let cache: RedisSessionCache<GameSessionRecord> = RedisSessionCache::new(redis_connection);
    let event_store = MySqlEventStore::new(mysql_pool);
    let random = SystemRandomGenerator::new();

    let state = AppState::new(repository, cache, event_store, random);

    let router = create_router(state);

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

async fn create_mysql_pool() -> anyhow::Result<MySqlPool> {
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "mysql://roguelike:roguelikepassword@localhost:3306/roguelike".to_string()
    });

    let config = MySqlPoolConfig::with_url(&database_url);

    MySqlPoolFactory::create_pool_async(&config)
        .run_async()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create MySQL pool: {}", e))
}

fn create_redis_connection()
-> anyhow::Result<roguelike_infrastructure::adapters::redis::RedisConnection> {
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

    let config = RedisConfig::with_url(&redis_url);

    RedisConnectionFactory::create_client(&config)
        .map_err(|e| anyhow::anyhow!("Failed to create Redis connection: {}", e))
}
