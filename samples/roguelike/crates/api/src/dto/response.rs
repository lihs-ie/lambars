use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// =============================================================================
// Game Session Responses
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameSessionResponse {
    pub game_id: String,

    pub player: PlayerResponse,

    pub floor: FloorSummaryResponse,

    pub turn_count: u32,

    pub status: GameStatusResponse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameStatusResponse {
    InProgress,
    Victory,
    Defeat,
    Paused,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameEndResponse {
    pub game_id: String,

    pub final_score: u64,

    pub dungeon_depth: u32,

    pub turns_survived: u32,

    pub enemies_defeated: u32,

    pub outcome: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnResultResponse {
    pub game: GameSessionResponse,

    pub turn_events: Vec<GameEventResponse>,

    pub game_over: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_over_reason: Option<String>,
}

// =============================================================================
// Player Responses
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerResponse {
    pub player_id: String,

    pub name: String,

    pub position: PositionResponse,

    pub health: ResourceResponse,

    pub mana: ResourceResponse,

    pub level: u32,

    pub experience: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerDetailResponse {
    pub player_id: String,

    pub name: String,

    pub position: PositionResponse,

    pub health: ResourceResponse,

    pub mana: ResourceResponse,

    pub level: u32,

    pub experience: u64,

    pub experience_to_next_level: u64,

    pub base_stats: BaseStatsResponse,

    pub combat_stats: CombatStatsResponse,

    pub equipment: EquipmentResponse,

    pub status_effects: Vec<StatusEffectResponse>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionResponse {
    pub x: i32,

    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceResponse {
    pub current: u32,

    pub max: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseStatsResponse {
    pub strength: u32,

    pub dexterity: u32,

    pub intelligence: u32,

    pub vitality: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatStatsResponse {
    pub attack: u32,

    pub defense: u32,

    pub speed: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipmentResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weapon: Option<ItemResponse>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub armor: Option<ItemResponse>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub helmet: Option<ItemResponse>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessory: Option<ItemResponse>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusEffectResponse {
    pub effect_type: String,

    pub remaining_turns: u32,

    pub magnitude: f64,
}

// =============================================================================
// Floor Responses
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FloorSummaryResponse {
    pub level: u32,

    pub width: u32,

    pub height: u32,

    pub explored_percentage: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FloorResponse {
    pub level: u32,

    pub width: u32,

    pub height: u32,

    pub tiles: Vec<Vec<TileResponse>>,

    pub visible_enemies: Vec<EnemySummaryResponse>,

    pub visible_items: Vec<DroppedItemResponse>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stairs_up: Option<PositionResponse>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stairs_down: Option<PositionResponse>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisibleAreaResponse {
    pub player_position: PositionResponse,

    pub visible_tiles: Vec<VisibleTileResponse>,

    pub visible_enemies: Vec<EnemySummaryResponse>,

    pub visible_items: Vec<DroppedItemResponse>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisibleTileResponse {
    pub position: PositionResponse,

    pub tile: TileResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileResponse {
    pub kind: TileKindResponse,

    pub is_explored: bool,

    pub is_visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TileKindResponse {
    Floor,
    Wall,
    DoorOpen,
    DoorClosed,
    StairsUp,
    StairsDown,
    Trap,
    Unknown,
}

// =============================================================================
// Enemy Responses
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnemySummaryResponse {
    pub enemy_id: String,

    pub enemy_type: String,

    pub position: PositionResponse,

    pub health_percentage: f64,
}

// =============================================================================
// Item Responses
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemResponse {
    pub item_id: String,

    pub name: String,

    pub kind: ItemKindResponse,

    pub description: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rarity: Option<ItemRarityResponse>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKindResponse {
    Weapon,
    Armor,
    Consumable,
    Material,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemRarityResponse {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStackResponse {
    pub item: ItemResponse,

    pub quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroppedItemResponse {
    pub item_id: String,

    pub name: String,

    pub position: PositionResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryResponse {
    pub capacity: u32,

    pub used_slots: u32,

    pub items: Vec<ItemStackResponse>,
}

// =============================================================================
// Event Responses
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameEventResponse {
    pub sequence: u64,

    #[serde(rename = "type")]
    pub event_type: String,

    pub data: JsonValue,

    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventsResponse {
    pub events: Vec<GameEventResponse>,

    pub next_sequence: u64,

    pub has_more: bool,
}

// =============================================================================
// Leaderboard Responses
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeaderboardResponse {
    #[serde(rename = "type")]
    pub leaderboard_type: String,

    pub entries: Vec<LeaderboardEntryResponse>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeaderboardEntryResponse {
    pub rank: u32,

    pub player_name: String,

    pub score: u64,

    pub dungeon_depth: u32,

    pub outcome: String,

    pub completed_at: DateTime<Utc>,
}

// =============================================================================
// Health Check Response
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: HealthStatusResponse,

    pub version: String,

    pub components: ComponentsResponse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatusResponse {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentsResponse {
    pub database: ComponentStatusResponse,

    pub cache: ComponentStatusResponse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentStatusResponse {
    Up,
    Down,
}

// =============================================================================
// Error Response
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorDetailResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorDetailResponse {
    pub code: String,

    pub message: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<JsonValue>,
}

impl ErrorResponse {
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: ErrorDetailResponse {
                code: code.into(),
                message: message.into(),
                details: None,
            },
        }
    }

    #[must_use]
    pub fn with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: JsonValue,
    ) -> Self {
        Self {
            error: ErrorDetailResponse {
                code: code.into(),
                message: message.into(),
                details: Some(details),
            },
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    mod game_session_response {
        use super::*;

        #[rstest]
        fn serialize_game_session() {
            let response = GameSessionResponse {
                game_id: "game-123".to_string(),
                player: PlayerResponse {
                    player_id: "player-456".to_string(),
                    name: "Hero".to_string(),
                    position: PositionResponse { x: 5, y: 10 },
                    health: ResourceResponse {
                        current: 80,
                        max: 100,
                    },
                    mana: ResourceResponse {
                        current: 30,
                        max: 50,
                    },
                    level: 3,
                    experience: 1500,
                },
                floor: FloorSummaryResponse {
                    level: 2,
                    width: 50,
                    height: 40,
                    explored_percentage: 35.5,
                },
                turn_count: 42,
                status: GameStatusResponse::InProgress,
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("game-123"));
            assert!(json.contains("Hero"));
            assert!(json.contains("in_progress"));
        }

        #[rstest]
        fn deserialize_game_status() {
            let json = r#""in_progress""#;
            let status: GameStatusResponse = serde_json::from_str(json).unwrap();
            assert_eq!(status, GameStatusResponse::InProgress);
        }

        #[rstest]
        #[case("in_progress", GameStatusResponse::InProgress)]
        #[case("victory", GameStatusResponse::Victory)]
        #[case("defeat", GameStatusResponse::Defeat)]
        #[case("paused", GameStatusResponse::Paused)]
        fn deserialize_all_game_statuses(
            #[case] status_str: &str,
            #[case] expected: GameStatusResponse,
        ) {
            let json = format!(r#""{}""#, status_str);
            let status: GameStatusResponse = serde_json::from_str(&json).unwrap();
            assert_eq!(status, expected);
        }
    }

    mod position_response {
        use super::*;

        #[rstest]
        fn serialize_position() {
            let position = PositionResponse { x: 10, y: 20 };
            let json = serde_json::to_string(&position).unwrap();
            assert!(json.contains(r#""x":10"#));
            assert!(json.contains(r#""y":20"#));
        }

        #[rstest]
        fn deserialize_position() {
            let json = r#"{"x": 5, "y": -3}"#;
            let position: PositionResponse = serde_json::from_str(json).unwrap();
            assert_eq!(position.x, 5);
            assert_eq!(position.y, -3);
        }
    }

    mod tile_response {
        use super::*;

        #[rstest]
        #[case("floor", TileKindResponse::Floor)]
        #[case("wall", TileKindResponse::Wall)]
        #[case("door_open", TileKindResponse::DoorOpen)]
        #[case("door_closed", TileKindResponse::DoorClosed)]
        #[case("stairs_up", TileKindResponse::StairsUp)]
        #[case("stairs_down", TileKindResponse::StairsDown)]
        #[case("trap", TileKindResponse::Trap)]
        #[case("unknown", TileKindResponse::Unknown)]
        fn deserialize_all_tile_kinds(#[case] kind_str: &str, #[case] expected: TileKindResponse) {
            let json = format!(r#""{}""#, kind_str);
            let kind: TileKindResponse = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, expected);
        }

        #[rstest]
        fn serialize_tile() {
            let tile = TileResponse {
                kind: TileKindResponse::Floor,
                is_explored: true,
                is_visible: false,
            };
            let json = serde_json::to_string(&tile).unwrap();
            assert!(json.contains(r#""kind":"floor""#));
            assert!(json.contains(r#""is_explored":true"#));
            assert!(json.contains(r#""is_visible":false"#));
        }
    }

    mod item_response {
        use super::*;

        #[rstest]
        #[case("weapon", ItemKindResponse::Weapon)]
        #[case("armor", ItemKindResponse::Armor)]
        #[case("consumable", ItemKindResponse::Consumable)]
        #[case("material", ItemKindResponse::Material)]
        fn deserialize_all_item_kinds(#[case] kind_str: &str, #[case] expected: ItemKindResponse) {
            let json = format!(r#""{}""#, kind_str);
            let kind: ItemKindResponse = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, expected);
        }

        #[rstest]
        #[case("common", ItemRarityResponse::Common)]
        #[case("uncommon", ItemRarityResponse::Uncommon)]
        #[case("rare", ItemRarityResponse::Rare)]
        #[case("epic", ItemRarityResponse::Epic)]
        #[case("legendary", ItemRarityResponse::Legendary)]
        fn deserialize_all_rarities(
            #[case] rarity_str: &str,
            #[case] expected: ItemRarityResponse,
        ) {
            let json = format!(r#""{}""#, rarity_str);
            let rarity: ItemRarityResponse = serde_json::from_str(&json).unwrap();
            assert_eq!(rarity, expected);
        }
    }

    mod health_response {
        use super::*;

        #[rstest]
        fn serialize_healthy_status() {
            let response = HealthResponse {
                status: HealthStatusResponse::Healthy,
                version: "1.0.0".to_string(),
                components: ComponentsResponse {
                    database: ComponentStatusResponse::Up,
                    cache: ComponentStatusResponse::Up,
                },
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains(r#""status":"healthy""#));
            assert!(json.contains(r#""database":"up""#));
            assert!(json.contains(r#""cache":"up""#));
        }

        #[rstest]
        #[case("healthy", HealthStatusResponse::Healthy)]
        #[case("degraded", HealthStatusResponse::Degraded)]
        #[case("unhealthy", HealthStatusResponse::Unhealthy)]
        fn deserialize_all_health_statuses(
            #[case] status_str: &str,
            #[case] expected: HealthStatusResponse,
        ) {
            let json = format!(r#""{}""#, status_str);
            let status: HealthStatusResponse = serde_json::from_str(&json).unwrap();
            assert_eq!(status, expected);
        }
    }

    mod error_response {
        use super::*;

        #[rstest]
        fn new_creates_error_without_details() {
            let error = ErrorResponse::new("NOT_FOUND", "Resource not found");
            assert_eq!(error.error.code, "NOT_FOUND");
            assert_eq!(error.error.message, "Resource not found");
            assert!(error.error.details.is_none());
        }

        #[rstest]
        fn with_details_creates_error_with_details() {
            let error = ErrorResponse::with_details(
                "VALIDATION_ERROR",
                "Invalid input",
                serde_json::json!({"field": "name"}),
            );
            assert_eq!(error.error.code, "VALIDATION_ERROR");
            assert!(error.error.details.is_some());
        }

        #[rstest]
        fn serialize_error() {
            let error = ErrorResponse::new("NOT_FOUND", "Game not found");
            let json = serde_json::to_string(&error).unwrap();
            assert!(json.contains(r#""code":"NOT_FOUND""#));
            assert!(json.contains(r#""message":"Game not found""#));
        }

        #[rstest]
        fn serialize_error_omits_null_details() {
            let error = ErrorResponse::new("ERROR", "An error");
            let json = serde_json::to_string(&error).unwrap();
            assert!(!json.contains("details"));
        }
    }

    mod events_response {
        use super::*;

        #[rstest]
        fn serialize_events_response() {
            let response = EventsResponse {
                events: vec![GameEventResponse {
                    sequence: 1,
                    event_type: "GameStarted".to_string(),
                    data: serde_json::json!({"seed": 12345}),
                    occurred_at: DateTime::parse_from_rfc3339("2026-01-09T12:00:00Z")
                        .unwrap()
                        .with_timezone(&Utc),
                }],
                next_sequence: 2,
                has_more: false,
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("GameStarted"));
            assert!(json.contains(r#""next_sequence":2"#));
            assert!(json.contains(r#""has_more":false"#));
        }
    }

    mod leaderboard_response {
        use super::*;

        #[rstest]
        fn serialize_leaderboard() {
            let response = LeaderboardResponse {
                leaderboard_type: "global".to_string(),
                entries: vec![LeaderboardEntryResponse {
                    rank: 1,
                    player_name: "Champion".to_string(),
                    score: 50000,
                    dungeon_depth: 10,
                    outcome: "victory".to_string(),
                    completed_at: DateTime::parse_from_rfc3339("2026-01-08T15:30:00Z")
                        .unwrap()
                        .with_timezone(&Utc),
                }],
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains(r#""type":"global""#));
            assert!(json.contains("Champion"));
            assert!(json.contains("50000"));
        }
    }
}
