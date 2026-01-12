//! Command DTOs for game actions.
//!
//! This module provides command data structures for player actions.
//! Commands use serde's tagged union pattern for type discrimination.

use serde::{Deserialize, Serialize};

// =============================================================================
// CommandRequest
// =============================================================================

/// All possible game commands that a player can execute.
///
/// Uses serde's internally tagged representation with the "type" field.
///
/// # Examples
///
/// Move command:
/// ```json
/// {
///   "type": "move",
///   "direction": "north"
/// }
/// ```
///
/// Attack command:
/// ```json
/// {
///   "type": "attack",
///   "target_id": "550e8400-e29b-41d4-a716-446655440000"
/// }
/// ```
///
/// Wait command:
/// ```json
/// {
///   "type": "wait"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandRequest {
    /// Move the player in a direction.
    Move {
        /// The direction to move.
        direction: DirectionRequest,
    },

    /// Attack a specific entity.
    Attack {
        /// The UUID of the target entity.
        target_id: String,
    },

    /// Use an item from the player's inventory.
    UseItem {
        /// The UUID of the item to use.
        item_id: String,

        /// Optional target entity for the item effect.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_id: Option<String>,
    },

    /// Pick up an item from the ground.
    PickUp {
        /// The UUID of the item to pick up.
        item_id: String,
    },

    /// Drop an item from the player's inventory.
    Drop {
        /// The UUID of the item to drop.
        item_id: String,
    },

    /// Equip an item from the player's inventory.
    Equip {
        /// The UUID of the item to equip.
        item_id: String,
    },

    /// Unequip an item from a specific equipment slot.
    Unequip {
        /// The equipment slot to unequip.
        slot: EquipmentSlotRequest,
    },

    /// Wait and skip the current turn.
    Wait,

    /// Descend to the next floor.
    Descend,

    /// Ascend to the previous floor.
    Ascend,
}

// =============================================================================
// DirectionRequest
// =============================================================================

/// The four cardinal directions for movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectionRequest {
    /// Move up (negative y direction).
    North,
    /// Move down (positive y direction).
    South,
    /// Move right (positive x direction).
    East,
    /// Move left (negative x direction).
    West,
}

// =============================================================================
// EquipmentSlotRequest
// =============================================================================

/// Equipment slot types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentSlotRequest {
    /// Weapon slot for swords, staffs, bows, etc.
    Weapon,
    /// Armor slot for body armor.
    Armor,
    /// Helmet slot for head protection.
    Helmet,
    /// Accessory slot for rings, amulets, etc.
    Accessory,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    mod command_request {
        use super::*;

        #[rstest]
        fn deserialize_move_command() {
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
        #[case("north", DirectionRequest::North)]
        #[case("south", DirectionRequest::South)]
        #[case("east", DirectionRequest::East)]
        #[case("west", DirectionRequest::West)]
        fn deserialize_move_all_directions(
            #[case] direction_str: &str,
            #[case] expected: DirectionRequest,
        ) {
            let json = format!(r#"{{"type": "move", "direction": "{}"}}"#, direction_str);
            let command: CommandRequest = serde_json::from_str(&json).unwrap();
            match command {
                CommandRequest::Move { direction } => assert_eq!(direction, expected),
                _ => panic!("Expected Move command"),
            }
        }

        #[rstest]
        fn deserialize_attack_command() {
            let json = r#"{"type": "attack", "target_id": "550e8400-e29b-41d4-a716-446655440000"}"#;
            let command: CommandRequest = serde_json::from_str(json).unwrap();
            match command {
                CommandRequest::Attack { target_id } => {
                    assert_eq!(target_id, "550e8400-e29b-41d4-a716-446655440000");
                }
                _ => panic!("Expected Attack command"),
            }
        }

        #[rstest]
        fn deserialize_use_item_command_without_target() {
            let json = r#"{"type": "use_item", "item_id": "550e8400-e29b-41d4-a716-446655440000"}"#;
            let command: CommandRequest = serde_json::from_str(json).unwrap();
            match command {
                CommandRequest::UseItem { item_id, target_id } => {
                    assert_eq!(item_id, "550e8400-e29b-41d4-a716-446655440000");
                    assert!(target_id.is_none());
                }
                _ => panic!("Expected UseItem command"),
            }
        }

        #[rstest]
        fn deserialize_use_item_command_with_target() {
            let json = r#"{"type": "use_item", "item_id": "item-id", "target_id": "target-id"}"#;
            let command: CommandRequest = serde_json::from_str(json).unwrap();
            match command {
                CommandRequest::UseItem { item_id, target_id } => {
                    assert_eq!(item_id, "item-id");
                    assert_eq!(target_id, Some("target-id".to_string()));
                }
                _ => panic!("Expected UseItem command"),
            }
        }

        #[rstest]
        fn deserialize_pick_up_command() {
            let json = r#"{"type": "pick_up", "item_id": "item-id"}"#;
            let command: CommandRequest = serde_json::from_str(json).unwrap();
            assert!(matches!(command, CommandRequest::PickUp { item_id } if item_id == "item-id"));
        }

        #[rstest]
        fn deserialize_drop_command() {
            let json = r#"{"type": "drop", "item_id": "item-id"}"#;
            let command: CommandRequest = serde_json::from_str(json).unwrap();
            assert!(matches!(command, CommandRequest::Drop { item_id } if item_id == "item-id"));
        }

        #[rstest]
        fn deserialize_equip_command() {
            let json = r#"{"type": "equip", "item_id": "item-id"}"#;
            let command: CommandRequest = serde_json::from_str(json).unwrap();
            assert!(matches!(command, CommandRequest::Equip { item_id } if item_id == "item-id"));
        }

        #[rstest]
        #[case("weapon", EquipmentSlotRequest::Weapon)]
        #[case("armor", EquipmentSlotRequest::Armor)]
        #[case("helmet", EquipmentSlotRequest::Helmet)]
        #[case("accessory", EquipmentSlotRequest::Accessory)]
        fn deserialize_unequip_command(
            #[case] slot_str: &str,
            #[case] expected: EquipmentSlotRequest,
        ) {
            let json = format!(r#"{{"type": "unequip", "slot": "{}"}}"#, slot_str);
            let command: CommandRequest = serde_json::from_str(&json).unwrap();
            match command {
                CommandRequest::Unequip { slot } => assert_eq!(slot, expected),
                _ => panic!("Expected Unequip command"),
            }
        }

        #[rstest]
        fn deserialize_wait_command() {
            let json = r#"{"type": "wait"}"#;
            let command: CommandRequest = serde_json::from_str(json).unwrap();
            assert!(matches!(command, CommandRequest::Wait));
        }

        #[rstest]
        fn deserialize_descend_command() {
            let json = r#"{"type": "descend"}"#;
            let command: CommandRequest = serde_json::from_str(json).unwrap();
            assert!(matches!(command, CommandRequest::Descend));
        }

        #[rstest]
        fn deserialize_ascend_command() {
            let json = r#"{"type": "ascend"}"#;
            let command: CommandRequest = serde_json::from_str(json).unwrap();
            assert!(matches!(command, CommandRequest::Ascend));
        }

        #[rstest]
        fn serialize_move_command() {
            let command = CommandRequest::Move {
                direction: DirectionRequest::North,
            };
            let json = serde_json::to_string(&command).unwrap();
            assert!(json.contains(r#""type":"move""#));
            assert!(json.contains(r#""direction":"north""#));
        }

        #[rstest]
        fn serialize_wait_command() {
            let command = CommandRequest::Wait;
            let json = serde_json::to_string(&command).unwrap();
            assert!(json.contains(r#""type":"wait""#));
        }

        #[rstest]
        fn serialize_use_item_without_target_omits_target_id() {
            let command = CommandRequest::UseItem {
                item_id: "item-id".to_string(),
                target_id: None,
            };
            let json = serde_json::to_string(&command).unwrap();
            assert!(json.contains(r#""item_id":"item-id""#));
            assert!(!json.contains("target_id"));
        }
    }

    mod direction_request {
        use super::*;

        #[rstest]
        fn equality() {
            assert_eq!(DirectionRequest::North, DirectionRequest::North);
            assert_ne!(DirectionRequest::North, DirectionRequest::South);
        }

        #[rstest]
        fn clone() {
            let direction = DirectionRequest::East;
            let cloned = direction;
            assert_eq!(direction, cloned);
        }
    }

    mod equipment_slot_request {
        use super::*;

        #[rstest]
        fn equality() {
            assert_eq!(EquipmentSlotRequest::Weapon, EquipmentSlotRequest::Weapon);
            assert_ne!(EquipmentSlotRequest::Weapon, EquipmentSlotRequest::Armor);
        }

        #[rstest]
        fn clone() {
            let slot = EquipmentSlotRequest::Helmet;
            let cloned = slot;
            assert_eq!(slot, cloned);
        }
    }
}
