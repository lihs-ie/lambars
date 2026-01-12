mod api_test_helpers;

use api_test_helpers::{
    MockEventStore, MockGameSession, MockGameSessionRepository, MockRandomGenerator,
    MockSessionCache, TestAppBuilder, create_test_session, create_test_session_with_name,
};
use roguelike_domain::game_session::GameIdentifier;
use rstest::rstest;

// =============================================================================
// Test Module: Health Check
// =============================================================================

mod health {
    use super::*;

    #[rstest]
    fn health_endpoint_placeholder() {
        // This test will be implemented when handlers are ready
        // For now, we verify the test infrastructure compiles
        let _builder = TestAppBuilder::new();
    }
}

// =============================================================================
// Test Module: Game Sessions
// =============================================================================

mod game_sessions {
    use super::*;

    #[rstest]
    fn create_game_placeholder() {
        // Test: POST /api/v1/games creates a new game session
        // This will be implemented when handlers are ready
        let _builder = TestAppBuilder::new();
    }

    #[rstest]
    fn get_game_placeholder() {
        // Test: GET /api/v1/games/:id returns game state
        let identifier = GameIdentifier::new();
        let session = MockGameSession::new(identifier, "TestPlayer");
        let _builder = TestAppBuilder::new().with_session(session);
    }

    #[rstest]
    fn get_game_not_found_placeholder() {
        // Test: GET /api/v1/games/:id returns 404 for unknown game
        let _builder = TestAppBuilder::new();
    }

    #[rstest]
    fn end_game_placeholder() {
        // Test: POST /api/v1/games/:id/end ends the game
        let session = create_test_session();
        let _builder = TestAppBuilder::new().with_session(session);
    }
}

// =============================================================================
// Test Module: Commands
// =============================================================================

mod commands {
    use super::*;

    #[rstest]
    fn execute_move_command_placeholder() {
        // Test: POST /api/v1/games/:id/commands with move command
        let session = create_test_session();
        let _builder = TestAppBuilder::new().with_session(session);
    }

    #[rstest]
    fn execute_wait_command_placeholder() {
        // Test: POST /api/v1/games/:id/commands with wait command
        let session = create_test_session();
        let _builder = TestAppBuilder::new().with_session(session);
    }

    #[rstest]
    fn execute_command_invalid_game_placeholder() {
        // Test: POST /api/v1/games/:id/commands returns 404 for unknown game
        let _builder = TestAppBuilder::new();
    }
}

// =============================================================================
// Test Module: Player
// =============================================================================

mod player {
    use super::*;

    #[rstest]
    fn get_player_details_placeholder() {
        // Test: GET /api/v1/games/:id/player returns player details
        let session = create_test_session();
        let _builder = TestAppBuilder::new().with_session(session);
    }

    #[rstest]
    fn get_inventory_placeholder() {
        // Test: GET /api/v1/games/:id/player/inventory returns inventory
        let session = create_test_session();
        let _builder = TestAppBuilder::new().with_session(session);
    }
}

// =============================================================================
// Test Module: Floor
// =============================================================================

mod floor {
    use super::*;

    #[rstest]
    fn get_floor_placeholder() {
        // Test: GET /api/v1/games/:id/floor returns floor state
        let session = create_test_session();
        let _builder = TestAppBuilder::new().with_session(session);
    }

    #[rstest]
    fn get_floor_with_fog_placeholder() {
        // Test: GET /api/v1/games/:id/floor?include_fog=true respects fog
        let session = create_test_session();
        let _builder = TestAppBuilder::new().with_session(session);
    }

    #[rstest]
    fn get_visible_area_placeholder() {
        // Test: GET /api/v1/games/:id/floor/visible returns visible area
        let session = create_test_session();
        let _builder = TestAppBuilder::new().with_session(session);
    }
}

// =============================================================================
// Test Module: Events
// =============================================================================

mod events {
    use super::*;

    #[rstest]
    fn get_events_placeholder() {
        // Test: GET /api/v1/games/:id/events returns event history
        let session = create_test_session();
        let _builder = TestAppBuilder::new().with_session(session);
    }

    #[rstest]
    fn get_events_with_pagination_placeholder() {
        // Test: GET /api/v1/games/:id/events?since=10&limit=20 respects pagination
        let session = create_test_session();
        let _builder = TestAppBuilder::new().with_session(session);
    }
}

// =============================================================================
// Test Module: Leaderboard
// =============================================================================

mod leaderboard {
    use super::*;

    #[rstest]
    fn get_leaderboard_placeholder() {
        // Test: GET /api/v1/leaderboard returns global leaderboard
        let _builder = TestAppBuilder::new();
    }

    #[rstest]
    fn get_daily_leaderboard_placeholder() {
        // Test: GET /api/v1/leaderboard?type=daily returns daily leaderboard
        let _builder = TestAppBuilder::new();
    }
}

// =============================================================================
// Test Module: Error Handling
// =============================================================================

mod error_handling {
    use super::*;
    use roguelike_api::errors::ApiError;

    #[rstest]
    fn not_found_error_returns_404() {
        let error = ApiError::not_found("GameSession", "abc-123");
        assert_eq!(error.status_code(), axum::http::StatusCode::NOT_FOUND);
    }

    #[rstest]
    fn validation_error_returns_400() {
        let error = ApiError::validation("Invalid input");
        assert_eq!(error.status_code(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[rstest]
    fn conflict_error_returns_409() {
        let error = ApiError::conflict("Resource conflict");
        assert_eq!(error.status_code(), axum::http::StatusCode::CONFLICT);
    }

    #[rstest]
    fn internal_error_returns_500() {
        let error = ApiError::internal("Internal error");
        assert_eq!(
            error.status_code(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}

// =============================================================================
// Test Module: DTOs
// =============================================================================

mod dtos {
    use roguelike_api::dto::{
        command::{CommandRequest, DirectionRequest, EquipmentSlotRequest},
        request::{CreateGameRequest, EndGameRequest, GameOutcomeRequest},
        response::{ErrorResponse, GameStatusResponse, HealthStatusResponse},
    };
    use rstest::rstest;

    #[rstest]
    fn command_request_deserializes_move() {
        let json = r#"{"type": "move", "direction": "north"}"#;
        let command: CommandRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(
            command,
            CommandRequest::Move {
                direction: DirectionRequest::North
            }
        ));
    }

    #[rstest]
    fn command_request_deserializes_wait() {
        let json = r#"{"type": "wait"}"#;
        let command: CommandRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(command, CommandRequest::Wait));
    }

    #[rstest]
    fn command_request_deserializes_unequip() {
        let json = r#"{"type": "unequip", "slot": "weapon"}"#;
        let command: CommandRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(
            command,
            CommandRequest::Unequip {
                slot: EquipmentSlotRequest::Weapon
            }
        ));
    }

    #[rstest]
    fn create_game_request_deserializes() {
        let json = r#"{"player_name": "Hero", "seed": 12345}"#;
        let request: CreateGameRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.player_name, "Hero");
        assert_eq!(request.seed, Some(12345));
    }

    #[rstest]
    fn end_game_request_deserializes() {
        let json = r#"{"outcome": "victory"}"#;
        let request: EndGameRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.outcome, GameOutcomeRequest::Victory);
    }

    #[rstest]
    fn game_status_response_serializes() {
        let status = GameStatusResponse::InProgress;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""in_progress""#);
    }

    #[rstest]
    fn health_status_response_serializes() {
        let status = HealthStatusResponse::Healthy;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""healthy""#);
    }

    #[rstest]
    fn error_response_creates_correctly() {
        let error = ErrorResponse::new("NOT_FOUND", "Game not found");
        assert_eq!(error.error.code, "NOT_FOUND");
        assert_eq!(error.error.message, "Game not found");
        assert!(error.error.details.is_none());
    }
}

// =============================================================================
// Test Module: Middleware
// =============================================================================

mod middleware {
    use roguelike_api::middleware::{RequestId, RequestIdLayer, ResponseTimeLayer};
    use rstest::rstest;
    use std::time::Duration;

    #[rstest]
    fn request_id_generates_unique_ids() {
        let id1 = RequestId::generate();
        let id2 = RequestId::generate();
        assert_ne!(id1, id2);
    }

    #[rstest]
    fn request_id_layer_creates_service() {
        let _layer = RequestIdLayer::new();
    }

    #[rstest]
    fn response_time_layer_creates_service() {
        let _layer = ResponseTimeLayer::new();
    }

    #[rstest]
    fn response_time_layer_with_min_duration() {
        let layer = ResponseTimeLayer::with_min_duration(Duration::from_millis(100));
        assert_eq!(
            layer.min_duration_to_log(),
            Some(Duration::from_millis(100))
        );
    }
}

// =============================================================================
// Test Module: State
// =============================================================================

mod state {
    use super::*;
    use roguelike_api::state::AppState;
    use std::sync::Arc;

    #[rstest]
    fn app_state_creates_with_mocks() {
        let state = AppState::new(
            MockGameSessionRepository::new(),
            MockSessionCache::new(),
            MockEventStore::new(),
            MockRandomGenerator::new(),
        );

        // State should be clonable (required for Axum)
        let _cloned = state.clone();

        // Verify Arc reference counting
        assert_eq!(Arc::strong_count(&state.repository), 2);
    }

    #[rstest]
    #[tokio::test]
    async fn app_state_repository_works() {
        use roguelike_workflow::ports::GameSessionRepository;

        let state = AppState::new(
            MockGameSessionRepository::new(),
            MockSessionCache::new(),
            MockEventStore::new(),
            MockRandomGenerator::new(),
        );

        let session = create_test_session_with_name("TestPlayer");
        let identifier = *session.identifier();

        state.repository.save(&session).run_async().await;
        let found = state.repository.find_by_id(&identifier).run_async().await;

        assert_eq!(found, Some(session));
    }
}
