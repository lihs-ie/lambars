//! Response DTOs for API endpoints.
//!
//! This module provides all response data structures used in API endpoints.
//! All DTOs are immutable and use serde for JSON serialization/deserialization.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// =============================================================================
// Game Session Responses
// =============================================================================

/// Response for game session state.
///
/// Contains the current state of a game session including player, floor,
/// turn count, and game status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameSessionResponse {
    /// The unique identifier for this game session.
    pub game_id: String,

    /// The player's current state.
    pub player: PlayerResponse,

    /// Summary of the current floor.
    pub floor: FloorSummaryResponse,

    /// The current turn number.
    pub turn_count: u32,

    /// The current game status.
    pub status: GameStatusResponse,
}

/// Game status enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameStatusResponse {
    /// Game is currently in progress.
    InProgress,
    /// Player has achieved victory.
    Victory,
    /// Player has been defeated.
    Defeat,
    /// Game is paused.
    Paused,
}

/// Response for ending a game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameEndResponse {
    /// The game session identifier.
    pub game_id: String,

    /// The final score achieved.
    pub final_score: u64,

    /// The deepest dungeon level reached.
    pub dungeon_depth: u32,

    /// The number of turns survived.
    pub turns_survived: u32,

    /// The number of enemies defeated.
    pub enemies_defeated: u32,

    /// The game outcome.
    pub outcome: String,
}

/// Response for turn processing result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnResultResponse {
    /// The updated game state.
    pub game: GameSessionResponse,

    /// Events that occurred during this turn.
    pub turn_events: Vec<GameEventResponse>,

    /// Whether the game has ended.
    pub game_over: bool,

    /// The reason for game over, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_over_reason: Option<String>,
}

// =============================================================================
// Player Responses
// =============================================================================

/// Basic player information for game session responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerResponse {
    /// The unique player identifier.
    pub player_id: String,

    /// The player's display name.
    pub name: String,

    /// The player's current position.
    pub position: PositionResponse,

    /// The player's current health.
    pub health: ResourceResponse,

    /// The player's current mana.
    pub mana: ResourceResponse,

    /// The player's current level.
    pub level: u32,

    /// The player's current experience points.
    pub experience: u64,
}

/// Detailed player information including stats and equipment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerDetailResponse {
    /// The unique player identifier.
    pub player_id: String,

    /// The player's display name.
    pub name: String,

    /// The player's current position.
    pub position: PositionResponse,

    /// The player's current health.
    pub health: ResourceResponse,

    /// The player's current mana.
    pub mana: ResourceResponse,

    /// The player's current level.
    pub level: u32,

    /// The player's current experience points.
    pub experience: u64,

    /// Experience points needed for the next level.
    pub experience_to_next_level: u64,

    /// The player's base stats.
    pub base_stats: BaseStatsResponse,

    /// The player's combat stats.
    pub combat_stats: CombatStatsResponse,

    /// The player's equipped items.
    pub equipment: EquipmentResponse,

    /// Active status effects on the player.
    pub status_effects: Vec<StatusEffectResponse>,
}

/// Position in the game world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionResponse {
    /// The x coordinate.
    pub x: i32,

    /// The y coordinate.
    pub y: i32,
}

/// A resource value with current and maximum amounts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceResponse {
    /// The current value.
    pub current: u32,

    /// The maximum value.
    pub max: u32,
}

/// Base character stats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseStatsResponse {
    /// Strength stat.
    pub strength: u32,

    /// Dexterity stat.
    pub dexterity: u32,

    /// Intelligence stat.
    pub intelligence: u32,

    /// Vitality stat.
    pub vitality: u32,
}

/// Combat-related stats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatStatsResponse {
    /// Attack power.
    pub attack: u32,

    /// Defense value.
    pub defense: u32,

    /// Speed/initiative value.
    pub speed: u32,
}

/// Player's equipped items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipmentResponse {
    /// The equipped weapon, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weapon: Option<ItemResponse>,

    /// The equipped armor, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub armor: Option<ItemResponse>,

    /// The equipped helmet, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub helmet: Option<ItemResponse>,

    /// The equipped accessory, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessory: Option<ItemResponse>,
}

/// Active status effect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusEffectResponse {
    /// The type of status effect.
    pub effect_type: String,

    /// Remaining turns for this effect.
    pub remaining_turns: u32,

    /// The magnitude/strength of the effect.
    pub magnitude: f64,
}

// =============================================================================
// Floor Responses
// =============================================================================

/// Summary information about a floor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FloorSummaryResponse {
    /// The floor level (depth).
    pub level: u32,

    /// The floor width in tiles.
    pub width: u32,

    /// The floor height in tiles.
    pub height: u32,

    /// Percentage of the floor that has been explored.
    pub explored_percentage: f64,
}

/// Detailed floor information including tile map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FloorResponse {
    /// The floor level (depth).
    pub level: u32,

    /// The floor width in tiles.
    pub width: u32,

    /// The floor height in tiles.
    pub height: u32,

    /// 2D array of tiles (rows of columns).
    pub tiles: Vec<Vec<TileResponse>>,

    /// Visible enemies on this floor.
    pub visible_enemies: Vec<EnemySummaryResponse>,

    /// Visible items on this floor.
    pub visible_items: Vec<DroppedItemResponse>,

    /// Position of stairs going up, if visible.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stairs_up: Option<PositionResponse>,

    /// Position of stairs going down, if visible.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stairs_down: Option<PositionResponse>,
}

/// Player's visible area (lightweight version).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisibleAreaResponse {
    /// The player's current position.
    pub player_position: PositionResponse,

    /// Tiles currently visible to the player.
    pub visible_tiles: Vec<VisibleTileResponse>,

    /// Enemies visible to the player.
    pub visible_enemies: Vec<EnemySummaryResponse>,

    /// Items visible to the player.
    pub visible_items: Vec<DroppedItemResponse>,
}

/// A visible tile with position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisibleTileResponse {
    /// The tile's position.
    pub position: PositionResponse,

    /// The tile information.
    pub tile: TileResponse,
}

/// Information about a single tile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileResponse {
    /// The type of tile.
    pub kind: TileKindResponse,

    /// Whether this tile has been explored.
    pub is_explored: bool,

    /// Whether this tile is currently visible.
    pub is_visible: bool,
}

/// Tile types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TileKindResponse {
    /// Walkable floor tile.
    Floor,
    /// Impassable wall.
    Wall,
    /// Open door.
    DoorOpen,
    /// Closed door.
    DoorClosed,
    /// Stairs going up.
    StairsUp,
    /// Stairs going down.
    StairsDown,
    /// Hidden trap.
    Trap,
    /// Unknown/unexplored tile.
    Unknown,
}

// =============================================================================
// Enemy Responses
// =============================================================================

/// Summary information about an enemy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnemySummaryResponse {
    /// The unique enemy identifier.
    pub enemy_id: String,

    /// The type of enemy.
    pub enemy_type: String,

    /// The enemy's position.
    pub position: PositionResponse,

    /// The enemy's health as a percentage (0.0 to 1.0).
    pub health_percentage: f64,
}

// =============================================================================
// Item Responses
// =============================================================================

/// Information about an item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemResponse {
    /// The unique item identifier.
    pub item_id: String,

    /// The item's display name.
    pub name: String,

    /// The item category.
    pub kind: ItemKindResponse,

    /// Item description.
    pub description: String,

    /// Item rarity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rarity: Option<ItemRarityResponse>,
}

/// Item category types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKindResponse {
    /// Weapons (swords, bows, etc.).
    Weapon,
    /// Armor pieces.
    Armor,
    /// Consumable items (potions, scrolls).
    Consumable,
    /// Crafting materials.
    Material,
}

/// Item rarity tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemRarityResponse {
    /// Common items.
    Common,
    /// Uncommon items.
    Uncommon,
    /// Rare items.
    Rare,
    /// Epic items.
    Epic,
    /// Legendary items.
    Legendary,
}

/// A stack of items with quantity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStackResponse {
    /// The item information.
    pub item: ItemResponse,

    /// The quantity in this stack.
    pub quantity: u32,
}

/// An item dropped on the floor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroppedItemResponse {
    /// The unique item identifier.
    pub item_id: String,

    /// The item's display name.
    pub name: String,

    /// The item's position on the floor.
    pub position: PositionResponse,
}

/// Player inventory response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryResponse {
    /// Maximum number of item stacks the inventory can hold.
    pub capacity: u32,

    /// Number of inventory slots currently used.
    pub used_slots: u32,

    /// Items in the inventory.
    pub items: Vec<ItemStackResponse>,
}

// =============================================================================
// Event Responses
// =============================================================================

/// A game event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameEventResponse {
    /// The sequence number of this event.
    pub sequence: u64,

    /// The event type name.
    #[serde(rename = "type")]
    pub event_type: String,

    /// Event-specific data.
    pub data: JsonValue,

    /// When the event occurred.
    pub occurred_at: DateTime<Utc>,
}

/// Response for event listing with pagination.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventsResponse {
    /// The list of events.
    pub events: Vec<GameEventResponse>,

    /// The sequence number to use for fetching the next page.
    pub next_sequence: u64,

    /// Whether there are more events available.
    pub has_more: bool,
}

// =============================================================================
// Leaderboard Responses
// =============================================================================

/// Leaderboard response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeaderboardResponse {
    /// The type of leaderboard.
    #[serde(rename = "type")]
    pub leaderboard_type: String,

    /// Leaderboard entries.
    pub entries: Vec<LeaderboardEntryResponse>,
}

/// A single leaderboard entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeaderboardEntryResponse {
    /// The rank on the leaderboard.
    pub rank: u32,

    /// The player's name.
    pub player_name: String,

    /// The score achieved.
    pub score: u64,

    /// The deepest dungeon level reached.
    pub dungeon_depth: u32,

    /// The game outcome.
    pub outcome: String,

    /// When the game was completed.
    pub completed_at: DateTime<Utc>,
}

// =============================================================================
// Health Check Response
// =============================================================================

/// Health check response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall health status.
    pub status: HealthStatusResponse,

    /// Application version.
    pub version: String,

    /// Component health statuses.
    pub components: ComponentsResponse,
}

/// Health status enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatusResponse {
    /// All systems operational.
    Healthy,
    /// Some systems degraded.
    Degraded,
    /// System is unhealthy.
    Unhealthy,
}

/// Individual component statuses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentsResponse {
    /// Database connection status.
    pub database: ComponentStatusResponse,

    /// Cache connection status.
    pub cache: ComponentStatusResponse,
}

/// Individual component status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentStatusResponse {
    /// Component is operational.
    Up,
    /// Component is down.
    Down,
}

// =============================================================================
// Error Response
// =============================================================================

/// Error response format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// The error details.
    pub error: ErrorDetailResponse,
}

/// Error detail information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorDetailResponse {
    /// Error code for programmatic handling.
    pub code: String,

    /// Human-readable error message.
    pub message: String,

    /// Additional error details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<JsonValue>,
}

impl ErrorResponse {
    /// Creates a new error response.
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

    /// Creates a new error response with details.
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
