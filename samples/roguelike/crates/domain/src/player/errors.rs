use std::error::Error;
use std::fmt;

use super::inventory::EquipmentSlot;

// =============================================================================
// PlayerError
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerError {
    PlayerNotFound {
        player_identifier: String,
    },

    HealthExhausted,

    ManaInsufficient {
        required: u32,
        available: u32,
    },

    InventoryFull {
        capacity: u32,
    },

    ItemNotInInventory {
        item_identifier: String,
    },

    EquipmentSlotOccupied {
        slot: EquipmentSlot,
    },

    CannotEquipItemType {
        item_type: String,
        slot: EquipmentSlot,
    },

    LevelCapReached,
}

impl PlayerError {
    #[must_use]
    pub fn player_not_found(player_identifier: impl Into<String>) -> Self {
        Self::PlayerNotFound {
            player_identifier: player_identifier.into(),
        }
    }

    #[must_use]
    pub const fn mana_insufficient(required: u32, available: u32) -> Self {
        Self::ManaInsufficient {
            required,
            available,
        }
    }

    #[must_use]
    pub const fn inventory_full(capacity: u32) -> Self {
        Self::InventoryFull { capacity }
    }

    #[must_use]
    pub fn item_not_in_inventory(item_identifier: impl Into<String>) -> Self {
        Self::ItemNotInInventory {
            item_identifier: item_identifier.into(),
        }
    }

    #[must_use]
    pub const fn equipment_slot_occupied(slot: EquipmentSlot) -> Self {
        Self::EquipmentSlotOccupied { slot }
    }

    #[must_use]
    pub fn cannot_equip_item_type(item_type: impl Into<String>, slot: EquipmentSlot) -> Self {
        Self::CannotEquipItemType {
            item_type: item_type.into(),
            slot,
        }
    }

    #[must_use]
    pub const fn is_death_related(&self) -> bool {
        matches!(self, Self::HealthExhausted)
    }

    #[must_use]
    pub const fn is_inventory_related(&self) -> bool {
        matches!(
            self,
            Self::InventoryFull { .. } | Self::ItemNotInInventory { .. }
        )
    }

    #[must_use]
    pub const fn is_equipment_related(&self) -> bool {
        matches!(
            self,
            Self::EquipmentSlotOccupied { .. } | Self::CannotEquipItemType { .. }
        )
    }

    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        match self {
            Self::PlayerNotFound { .. } => false,
            Self::HealthExhausted => false,
            Self::ManaInsufficient { .. } => true,
            Self::InventoryFull { .. } => true,
            Self::ItemNotInInventory { .. } => true,
            Self::EquipmentSlotOccupied { .. } => true,
            Self::CannotEquipItemType { .. } => true,
            Self::LevelCapReached => true,
        }
    }
}

impl fmt::Display for PlayerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlayerNotFound { player_identifier } => {
                write!(formatter, "Player not found: {}", player_identifier)
            }
            Self::HealthExhausted => {
                write!(formatter, "Player health exhausted")
            }
            Self::ManaInsufficient {
                required,
                available,
            } => {
                write!(
                    formatter,
                    "Insufficient mana: required {}, available {}",
                    required, available
                )
            }
            Self::InventoryFull { capacity } => {
                write!(formatter, "Inventory is full (capacity: {})", capacity)
            }
            Self::ItemNotInInventory { item_identifier } => {
                write!(formatter, "Item not in inventory: {}", item_identifier)
            }
            Self::EquipmentSlotOccupied { slot } => {
                write!(formatter, "Equipment slot already occupied: {}", slot)
            }
            Self::CannotEquipItemType { item_type, slot } => {
                write!(formatter, "Cannot equip {} to {} slot", item_type, slot)
            }
            Self::LevelCapReached => {
                write!(formatter, "Maximum level reached")
            }
        }
    }
}

impl Error for PlayerError {}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // PlayerError Tests
    // =========================================================================

    mod player_error {
        use super::*;

        #[rstest]
        fn player_not_found_factory() {
            let error = PlayerError::player_not_found("player-123");
            assert!(matches!(
                error,
                PlayerError::PlayerNotFound {
                    player_identifier
                } if player_identifier == "player-123"
            ));
        }

        #[rstest]
        fn player_not_found_display() {
            let error = PlayerError::player_not_found("player-123");
            assert_eq!(format!("{}", error), "Player not found: player-123");
        }

        #[rstest]
        fn health_exhausted_display() {
            let error = PlayerError::HealthExhausted;
            assert_eq!(format!("{}", error), "Player health exhausted");
        }

        #[rstest]
        fn mana_insufficient_factory() {
            let error = PlayerError::mana_insufficient(50, 30);
            assert!(matches!(
                error,
                PlayerError::ManaInsufficient {
                    required: 50,
                    available: 30
                }
            ));
        }

        #[rstest]
        fn mana_insufficient_display() {
            let error = PlayerError::mana_insufficient(50, 30);
            assert_eq!(
                format!("{}", error),
                "Insufficient mana: required 50, available 30"
            );
        }

        #[rstest]
        fn inventory_full_factory() {
            let error = PlayerError::inventory_full(20);
            assert!(matches!(error, PlayerError::InventoryFull { capacity: 20 }));
        }

        #[rstest]
        fn inventory_full_display() {
            let error = PlayerError::inventory_full(20);
            assert_eq!(format!("{}", error), "Inventory is full (capacity: 20)");
        }

        #[rstest]
        fn item_not_in_inventory_factory() {
            let error = PlayerError::item_not_in_inventory("potion-001");
            assert!(matches!(
                error,
                PlayerError::ItemNotInInventory {
                    item_identifier
                } if item_identifier == "potion-001"
            ));
        }

        #[rstest]
        fn item_not_in_inventory_display() {
            let error = PlayerError::item_not_in_inventory("potion-001");
            assert_eq!(format!("{}", error), "Item not in inventory: potion-001");
        }

        #[rstest]
        fn equipment_slot_occupied_factory() {
            let error = PlayerError::equipment_slot_occupied(EquipmentSlot::Weapon);
            assert!(matches!(
                error,
                PlayerError::EquipmentSlotOccupied {
                    slot: EquipmentSlot::Weapon
                }
            ));
        }

        #[rstest]
        fn equipment_slot_occupied_display() {
            let error = PlayerError::equipment_slot_occupied(EquipmentSlot::Weapon);
            assert_eq!(
                format!("{}", error),
                "Equipment slot already occupied: Weapon"
            );
        }

        #[rstest]
        fn cannot_equip_item_type_factory() {
            let error = PlayerError::cannot_equip_item_type("Potion", EquipmentSlot::Weapon);
            assert!(matches!(
                error,
                PlayerError::CannotEquipItemType {
                    item_type,
                    slot: EquipmentSlot::Weapon
                } if item_type == "Potion"
            ));
        }

        #[rstest]
        fn cannot_equip_item_type_display() {
            let error = PlayerError::cannot_equip_item_type("Potion", EquipmentSlot::Weapon);
            assert_eq!(format!("{}", error), "Cannot equip Potion to Weapon slot");
        }

        #[rstest]
        fn level_cap_reached_display() {
            let error = PlayerError::LevelCapReached;
            assert_eq!(format!("{}", error), "Maximum level reached");
        }

        #[rstest]
        fn is_death_related_for_health_exhausted() {
            assert!(PlayerError::HealthExhausted.is_death_related());
        }

        #[rstest]
        fn is_death_related_for_other_errors() {
            assert!(!PlayerError::LevelCapReached.is_death_related());
            assert!(!PlayerError::inventory_full(20).is_death_related());
            assert!(!PlayerError::mana_insufficient(50, 30).is_death_related());
        }

        #[rstest]
        fn is_inventory_related_for_inventory_errors() {
            assert!(PlayerError::inventory_full(20).is_inventory_related());
            assert!(PlayerError::item_not_in_inventory("item").is_inventory_related());
        }

        #[rstest]
        fn is_inventory_related_for_other_errors() {
            assert!(!PlayerError::HealthExhausted.is_inventory_related());
            assert!(!PlayerError::LevelCapReached.is_inventory_related());
        }

        #[rstest]
        fn is_equipment_related_for_equipment_errors() {
            assert!(
                PlayerError::equipment_slot_occupied(EquipmentSlot::Weapon).is_equipment_related()
            );
            assert!(
                PlayerError::cannot_equip_item_type("Sword", EquipmentSlot::Helmet)
                    .is_equipment_related()
            );
        }

        #[rstest]
        fn is_equipment_related_for_other_errors() {
            assert!(!PlayerError::HealthExhausted.is_equipment_related());
            assert!(!PlayerError::inventory_full(20).is_equipment_related());
        }

        #[rstest]
        fn is_recoverable_for_recoverable_errors() {
            assert!(PlayerError::mana_insufficient(50, 30).is_recoverable());
            assert!(PlayerError::inventory_full(20).is_recoverable());
            assert!(PlayerError::item_not_in_inventory("item").is_recoverable());
            assert!(PlayerError::equipment_slot_occupied(EquipmentSlot::Weapon).is_recoverable());
            assert!(
                PlayerError::cannot_equip_item_type("Sword", EquipmentSlot::Helmet)
                    .is_recoverable()
            );
            assert!(PlayerError::LevelCapReached.is_recoverable());
        }

        #[rstest]
        fn is_recoverable_for_non_recoverable_errors() {
            assert!(!PlayerError::player_not_found("player").is_recoverable());
            assert!(!PlayerError::HealthExhausted.is_recoverable());
        }

        #[rstest]
        fn equality() {
            let error1 = PlayerError::HealthExhausted;
            let error2 = PlayerError::HealthExhausted;
            let error3 = PlayerError::LevelCapReached;

            assert_eq!(error1, error2);
            assert_ne!(error1, error3);
        }

        #[rstest]
        fn equality_with_fields() {
            let error1 = PlayerError::mana_insufficient(50, 30);
            let error2 = PlayerError::mana_insufficient(50, 30);
            let error3 = PlayerError::mana_insufficient(60, 40);

            assert_eq!(error1, error2);
            assert_ne!(error1, error3);
        }

        #[rstest]
        fn clone() {
            let error = PlayerError::inventory_full(20);
            let cloned = error.clone();
            assert_eq!(error, cloned);
        }

        #[rstest]
        fn debug_format() {
            let error = PlayerError::HealthExhausted;
            let debug_string = format!("{:?}", error);
            assert!(debug_string.contains("HealthExhausted"));
        }

        #[rstest]
        fn implements_error_trait() {
            let error: &dyn Error = &PlayerError::HealthExhausted;
            assert!(error.source().is_none());
        }
    }
}
