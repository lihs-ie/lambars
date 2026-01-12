//! Request DTOs for API endpoints.
//!
//! This module provides all request data structures used in API endpoints.
//! All DTOs are immutable and use serde for JSON serialization/deserialization.

use serde::{Deserialize, Serialize};

use super::command::CommandRequest;

// =============================================================================
// Game Session Requests
// =============================================================================

/// Request body for creating a new game session.
///
/// # Examples
///
/// ```json
/// {
///   "player_name": "Hero",
///   "seed": 12345
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateGameRequest {
    /// The player's display name (1-50 characters).
    pub player_name: String,

    /// Optional random seed for reproducible game generation.
    /// If not provided, a random seed will be generated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
}

/// Request body for ending a game session.
///
/// # Examples
///
/// ```json
/// {
///   "outcome": "abandon"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndGameRequest {
    /// The outcome of the game.
    pub outcome: GameOutcomeRequest,
}

/// Game outcome for end game requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameOutcomeRequest {
    /// Player won the game.
    Victory,
    /// Player was defeated.
    Defeat,
    /// Player abandoned the game.
    Abandon,
}

// =============================================================================
// Command Requests
// =============================================================================

/// Request body for executing a game command.
///
/// # Examples
///
/// ```json
/// {
///   "command": {
///     "type": "move",
///     "direction": "north"
///   }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecuteCommandRequest {
    /// The command to execute.
    pub command: CommandRequest,
}

// =============================================================================
// Query Parameters
// =============================================================================

/// Query parameters for getting events.
///
/// # Examples
///
/// ```text
/// GET /games/{game_id}/events?since=10&limit=50
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GetEventsParams {
    /// The sequence number to start from (exclusive).
    /// If not provided, starts from the beginning.
    #[serde(default)]
    pub since: Option<u64>,

    /// Maximum number of events to return.
    /// Defaults to 100, maximum is 1000.
    #[serde(default)]
    pub limit: Option<u32>,
}

/// Query parameters for getting the leaderboard.
///
/// # Examples
///
/// ```text
/// GET /leaderboard?type=global&limit=10
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GetLeaderboardParams {
    /// The type of leaderboard to retrieve.
    #[serde(default, rename = "type")]
    pub leaderboard_type: Option<LeaderboardTypeRequest>,

    /// Maximum number of entries to return.
    /// Defaults to 10, maximum is 100.
    #[serde(default)]
    pub limit: Option<u32>,
}

/// Leaderboard type for query parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaderboardTypeRequest {
    /// Global all-time leaderboard.
    #[default]
    Global,
    /// Daily leaderboard.
    Daily,
    /// Weekly leaderboard.
    Weekly,
}

/// Query parameters for getting floor information.
///
/// # Examples
///
/// ```text
/// GET /games/{game_id}/floor?include_fog=true
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GetFloorParams {
    /// Whether to include fog of war (hide unexplored areas).
    /// Defaults to true.
    #[serde(default = "default_include_fog")]
    pub include_fog: bool,
}

/// Default value for include_fog parameter.
fn default_include_fog() -> bool {
    true
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    mod create_game_request {
        use super::*;

        #[rstest]
        fn deserialize_with_seed() {
            let json = r#"{"player_name": "Hero", "seed": 12345}"#;
            let request: CreateGameRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.player_name, "Hero");
            assert_eq!(request.seed, Some(12345));
        }

        #[rstest]
        fn deserialize_without_seed() {
            let json = r#"{"player_name": "Hero"}"#;
            let request: CreateGameRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.player_name, "Hero");
            assert!(request.seed.is_none());
        }

        #[rstest]
        fn serialize_with_seed() {
            let request = CreateGameRequest {
                player_name: "Hero".to_string(),
                seed: Some(12345),
            };
            let json = serde_json::to_string(&request).unwrap();
            assert!(json.contains("player_name"));
            assert!(json.contains("Hero"));
            assert!(json.contains("seed"));
            assert!(json.contains("12345"));
        }

        #[rstest]
        fn serialize_without_seed() {
            let request = CreateGameRequest {
                player_name: "Hero".to_string(),
                seed: None,
            };
            let json = serde_json::to_string(&request).unwrap();
            assert!(json.contains("player_name"));
            assert!(!json.contains("seed"));
        }
    }

    mod end_game_request {
        use super::*;

        #[rstest]
        #[case("victory", GameOutcomeRequest::Victory)]
        #[case("defeat", GameOutcomeRequest::Defeat)]
        #[case("abandon", GameOutcomeRequest::Abandon)]
        fn deserialize_outcome(#[case] outcome_str: &str, #[case] expected: GameOutcomeRequest) {
            let json = format!(r#"{{"outcome": "{}"}}"#, outcome_str);
            let request: EndGameRequest = serde_json::from_str(&json).unwrap();
            assert_eq!(request.outcome, expected);
        }

        #[rstest]
        fn serialize_outcome() {
            let request = EndGameRequest {
                outcome: GameOutcomeRequest::Victory,
            };
            let json = serde_json::to_string(&request).unwrap();
            assert!(json.contains("victory"));
        }
    }

    mod get_events_params {
        use super::*;

        #[rstest]
        fn deserialize_with_all_params() {
            let json = r#"{"since": 10, "limit": 50}"#;
            let params: GetEventsParams = serde_json::from_str(json).unwrap();
            assert_eq!(params.since, Some(10));
            assert_eq!(params.limit, Some(50));
        }

        #[rstest]
        fn deserialize_with_defaults() {
            let json = r#"{}"#;
            let params: GetEventsParams = serde_json::from_str(json).unwrap();
            assert!(params.since.is_none());
            assert!(params.limit.is_none());
        }

        #[rstest]
        fn default_trait() {
            let params = GetEventsParams::default();
            assert!(params.since.is_none());
            assert!(params.limit.is_none());
        }
    }

    mod get_leaderboard_params {
        use super::*;

        #[rstest]
        fn deserialize_with_all_params() {
            let json = r#"{"type": "daily", "limit": 20}"#;
            let params: GetLeaderboardParams = serde_json::from_str(json).unwrap();
            assert_eq!(params.leaderboard_type, Some(LeaderboardTypeRequest::Daily));
            assert_eq!(params.limit, Some(20));
        }

        #[rstest]
        fn deserialize_with_defaults() {
            let json = r#"{}"#;
            let params: GetLeaderboardParams = serde_json::from_str(json).unwrap();
            assert!(params.leaderboard_type.is_none());
            assert!(params.limit.is_none());
        }

        #[rstest]
        #[case("global", LeaderboardTypeRequest::Global)]
        #[case("daily", LeaderboardTypeRequest::Daily)]
        #[case("weekly", LeaderboardTypeRequest::Weekly)]
        fn deserialize_leaderboard_types(
            #[case] type_str: &str,
            #[case] expected: LeaderboardTypeRequest,
        ) {
            let json = format!(r#"{{"type": "{}"}}"#, type_str);
            let params: GetLeaderboardParams = serde_json::from_str(&json).unwrap();
            assert_eq!(params.leaderboard_type, Some(expected));
        }
    }

    mod get_floor_params {
        use super::*;

        #[rstest]
        fn deserialize_with_include_fog_true() {
            let json = r#"{"include_fog": true}"#;
            let params: GetFloorParams = serde_json::from_str(json).unwrap();
            assert!(params.include_fog);
        }

        #[rstest]
        fn deserialize_with_include_fog_false() {
            let json = r#"{"include_fog": false}"#;
            let params: GetFloorParams = serde_json::from_str(json).unwrap();
            assert!(!params.include_fog);
        }

        #[rstest]
        fn deserialize_with_default() {
            let json = r#"{}"#;
            let params: GetFloorParams = serde_json::from_str(json).unwrap();
            assert!(params.include_fog);
        }
    }
}
