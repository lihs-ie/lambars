use serde::{Deserialize, Serialize};

// =============================================================================
// CommandRequest
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandRequest {
    Move {
        direction: DirectionRequest,
    },

    Attack {
        target_id: String,
    },

    UseItem {
        item_id: String,

        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_id: Option<String>,
    },

    PickUp {
        item_id: String,
    },

    Drop {
        item_id: String,
    },

    Equip {
        item_id: String,
    },

    Unequip {
        slot: EquipmentSlotRequest,
    },

    Wait,

    Descend,

    Ascend,
}

// =============================================================================
// DirectionRequest
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectionRequest {
    North,
    South,
    East,
    West,
}

// =============================================================================
// EquipmentSlotRequest
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentSlotRequest {
    Weapon,
    Armor,
    Helmet,
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
