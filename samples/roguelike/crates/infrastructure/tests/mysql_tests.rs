//! MySQL integration tests.
//!
//! These tests verify the MySQL adapters work correctly with a real database.
//!
//! # Requirements
//!
//! A MySQL container must be running at `localhost:3306` with:
//! - Database: `roguelike`
//! - User: `roguelike`
//! - Password: `roguelikepassword`

use roguelike_domain::game_session::GameIdentifier;
use roguelike_infrastructure::adapters::mysql::{
    GameSessionRecord, MySqlGameSessionRepository, MySqlPoolConfig, MySqlPoolFactory,
};
use roguelike_workflow::ports::GameSessionRepository;
use rstest::rstest;
use uuid::Uuid;

/// The database URL for the test MySQL container.
const DATABASE_URL: &str = "mysql://roguelike:roguelikepassword@localhost:3306/roguelike";

// =============================================================================
// Connection Tests
// =============================================================================

/// Tests that we can successfully connect to the MySQL database.
#[rstest]
#[tokio::test]
async fn test_mysql_connection() {
    let config = MySqlPoolConfig::with_url(DATABASE_URL);
    let pool = MySqlPoolFactory::create_pool_async(&config)
        .run_async()
        .await
        .expect("Failed to create pool");

    // Verify the pool is open and functional
    assert!(!pool.is_closed());
}

/// Tests that the pool can execute a simple query.
#[rstest]
#[tokio::test]
async fn test_mysql_pool_query() {
    let config = MySqlPoolConfig::with_url(DATABASE_URL);
    let pool = MySqlPoolFactory::create_pool_async(&config)
        .run_async()
        .await
        .expect("Failed to create pool");

    // Execute a simple query to verify connectivity
    let result: (i32,) = sqlx::query_as("SELECT 1")
        .fetch_one(pool.as_inner())
        .await
        .expect("Failed to execute query");

    assert_eq!(result.0, 1);
}

// =============================================================================
// Repository Tests
// =============================================================================

/// Tests the complete CRUD cycle for game session records.
#[rstest]
#[tokio::test]
async fn test_game_session_repository_save_and_find() {
    let config = MySqlPoolConfig::with_url(DATABASE_URL);
    let pool = MySqlPoolFactory::create_pool_async(&config)
        .run_async()
        .await
        .expect("Failed to create pool");
    let repository = MySqlGameSessionRepository::new(pool);

    // Create a unique game ID for this test
    let game_id = Uuid::new_v4().to_string();
    let player_id = Uuid::new_v4().to_string();

    let record = GameSessionRecord::new(
        game_id.clone(),
        player_id,
        1,
        10,
        "in_progress".to_string(),
        12345,
        5,
    );

    // Save the record
    repository.save(&record).run_async().await;

    // Find the record
    let game_identifier = game_id
        .parse::<GameIdentifier>()
        .expect("Failed to parse game identifier");
    let found = repository.find_by_id(&game_identifier).run_async().await;

    assert!(found.is_some());
    let found_record = found.unwrap();
    assert_eq!(found_record.game_id, record.game_id);
    assert_eq!(found_record.current_floor_level, record.current_floor_level);
    assert_eq!(found_record.turn_count, record.turn_count);
    assert_eq!(found_record.status, record.status);

    // Cleanup: delete the record
    repository.delete(&game_identifier).run_async().await;

    // Verify deletion
    let not_found = repository.find_by_id(&game_identifier).run_async().await;
    assert!(not_found.is_none());
}

/// Tests that finding a non-existent record returns None.
#[rstest]
#[tokio::test]
async fn test_game_session_repository_find_not_found() {
    let config = MySqlPoolConfig::with_url(DATABASE_URL);
    let pool = MySqlPoolFactory::create_pool_async(&config)
        .run_async()
        .await
        .expect("Failed to create pool");
    let repository = MySqlGameSessionRepository::new(pool);

    let nonexistent_id = GameIdentifier::new();
    let result = repository.find_by_id(&nonexistent_id).run_async().await;

    assert!(result.is_none());
}

/// Tests updating an existing game session record.
#[rstest]
#[tokio::test]
async fn test_game_session_repository_update() {
    let config = MySqlPoolConfig::with_url(DATABASE_URL);
    let pool = MySqlPoolFactory::create_pool_async(&config)
        .run_async()
        .await
        .expect("Failed to create pool");
    let repository = MySqlGameSessionRepository::new(pool);

    // Create and save initial record
    let game_id = Uuid::new_v4().to_string();
    let player_id = Uuid::new_v4().to_string();

    let initial_record = GameSessionRecord::new(
        game_id.clone(),
        player_id.clone(),
        1,
        10,
        "in_progress".to_string(),
        12345,
        5,
    );

    repository.save(&initial_record).run_async().await;

    // Update the record
    let updated_record = GameSessionRecord::new(
        game_id.clone(),
        player_id,
        3,
        50,
        "in_progress".to_string(),
        12345,
        20,
    );

    repository.save(&updated_record).run_async().await;

    // Verify the update
    let game_identifier = game_id
        .parse::<GameIdentifier>()
        .expect("Failed to parse game identifier");
    let found = repository.find_by_id(&game_identifier).run_async().await;

    assert!(found.is_some());
    let found_record = found.unwrap();
    assert_eq!(found_record.current_floor_level, 3);
    assert_eq!(found_record.turn_count, 50);
    assert_eq!(found_record.event_sequence, 20);

    // Cleanup
    repository.delete(&game_identifier).run_async().await;
}

/// Tests listing active game sessions.
#[rstest]
#[tokio::test]
async fn test_game_session_repository_list_active() {
    let config = MySqlPoolConfig::with_url(DATABASE_URL);
    let pool = MySqlPoolFactory::create_pool_async(&config)
        .run_async()
        .await
        .expect("Failed to create pool");
    let repository = MySqlGameSessionRepository::new(pool);

    // Create test records
    let game_id1 = Uuid::new_v4().to_string();
    let game_id2 = Uuid::new_v4().to_string();
    let player_id = Uuid::new_v4().to_string();

    let record1 = GameSessionRecord::new(
        game_id1.clone(),
        player_id.clone(),
        1,
        10,
        "in_progress".to_string(),
        12345,
        5,
    );

    let record2 = GameSessionRecord::new(
        game_id2.clone(),
        player_id.clone(),
        2,
        20,
        "victory".to_string(),
        54321,
        10,
    );

    repository.save(&record1).run_async().await;
    repository.save(&record2).run_async().await;

    // List active sessions
    let active = repository.list_active().run_async().await;

    // record1 should be in the list (in_progress), record2 should not (victory)
    let game_identifier1 = game_id1
        .parse::<GameIdentifier>()
        .expect("Failed to parse game identifier");

    assert!(active.contains(&game_identifier1));

    // Cleanup
    let game_identifier2 = game_id2
        .parse::<GameIdentifier>()
        .expect("Failed to parse game identifier");
    repository.delete(&game_identifier1).run_async().await;
    repository.delete(&game_identifier2).run_async().await;
}

/// Tests deleting a non-existent record does not cause an error.
#[rstest]
#[tokio::test]
async fn test_game_session_repository_delete_nonexistent() {
    let config = MySqlPoolConfig::with_url(DATABASE_URL);
    let pool = MySqlPoolFactory::create_pool_async(&config)
        .run_async()
        .await
        .expect("Failed to create pool");
    let repository = MySqlGameSessionRepository::new(pool);

    let nonexistent_id = GameIdentifier::new();

    // This should not panic or cause an error
    repository.delete(&nonexistent_id).run_async().await;
}
