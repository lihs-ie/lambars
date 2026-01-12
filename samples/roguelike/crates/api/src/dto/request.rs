use serde::{Deserialize, Serialize};

use super::command::CommandRequest;

// =============================================================================
// Game Session Requests
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateGameRequest {
    pub player_name: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndGameRequest {
    pub outcome: GameOutcomeRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameOutcomeRequest {
    Victory,
    Defeat,
    Abandon,
}

// =============================================================================
// Command Requests
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecuteCommandRequest {
    pub command: CommandRequest,
}

// =============================================================================
// Query Parameters
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GetEventsParams {
    #[serde(default)]
    pub since: Option<u64>,

    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GetLeaderboardParams {
    #[serde(default, rename = "type")]
    pub leaderboard_type: Option<LeaderboardTypeRequest>,

    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaderboardTypeRequest {
    #[default]
    Global,
    Daily,
    Weekly,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GetFloorParams {
    #[serde(default = "default_include_fog")]
    pub include_fog: bool,
}

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
