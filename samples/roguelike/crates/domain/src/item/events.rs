
use std::fmt;

use crate::common::Position;

use super::effect::ItemEffect;
use super::identifier::ItemIdentifier;

// =============================================================================
// EquipmentSlot
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquipmentSlot {
    Weapon,
    Armor,
    Helmet,
    Accessory,
}

impl EquipmentSlot {
    #[must_use]
    pub const fn all() -> [Self; 4] {
        [Self::Weapon, Self::Armor, Self::Helmet, Self::Accessory]
    }
}

impl fmt::Display for EquipmentSlot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Weapon => "Weapon",
            Self::Armor => "Armor",
            Self::Helmet => "Helmet",
            Self::Accessory => "Accessory",
        };
        write!(formatter, "{}", name)
    }
}

// =============================================================================
// ItemPickedUp
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemPickedUp {
    item_identifier: ItemIdentifier,
}

impl ItemPickedUp {
    #[must_use]
    pub const fn new(item_identifier: ItemIdentifier) -> Self {
        Self { item_identifier }
    }

    #[must_use]
    pub const fn item_identifier(&self) -> ItemIdentifier {
        self.item_identifier
    }
}

impl fmt::Display for ItemPickedUp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Picked up item: {}", self.item_identifier)
    }
}

// =============================================================================
// ItemDropped
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemDropped {
    item_identifier: ItemIdentifier,
    position: Position,
}

impl ItemDropped {
    #[must_use]
    pub const fn new(item_identifier: ItemIdentifier, position: Position) -> Self {
        Self {
            item_identifier,
            position,
        }
    }

    #[must_use]
    pub const fn item_identifier(&self) -> ItemIdentifier {
        self.item_identifier
    }

    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }
}

impl fmt::Display for ItemDropped {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Dropped item {} at {}",
            self.item_identifier, self.position
        )
    }
}

// =============================================================================
// ItemUsed
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemUsed {
    item_identifier: ItemIdentifier,
    effect: ItemEffect,
}

impl ItemUsed {
    #[must_use]
    pub const fn new(item_identifier: ItemIdentifier, effect: ItemEffect) -> Self {
        Self {
            item_identifier,
            effect,
        }
    }

    #[must_use]
    pub const fn item_identifier(&self) -> ItemIdentifier {
        self.item_identifier
    }

    #[must_use]
    pub const fn effect(&self) -> ItemEffect {
        self.effect
    }
}

impl fmt::Display for ItemUsed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Used item {}: {}",
            self.item_identifier, self.effect
        )
    }
}

// =============================================================================
// ItemEquipped
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemEquipped {
    item_identifier: ItemIdentifier,
    slot: EquipmentSlot,
}

impl ItemEquipped {
    #[must_use]
    pub const fn new(item_identifier: ItemIdentifier, slot: EquipmentSlot) -> Self {
        Self {
            item_identifier,
            slot,
        }
    }

    #[must_use]
    pub const fn item_identifier(&self) -> ItemIdentifier {
        self.item_identifier
    }

    #[must_use]
    pub const fn slot(&self) -> EquipmentSlot {
        self.slot
    }
}

impl fmt::Display for ItemEquipped {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Equipped item {} to {} slot",
            self.item_identifier, self.slot
        )
    }
}

// =============================================================================
// ItemUnequipped
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemUnequipped {
    slot: EquipmentSlot,
}

impl ItemUnequipped {
    #[must_use]
    pub const fn new(slot: EquipmentSlot) -> Self {
        Self { slot }
    }

    #[must_use]
    pub const fn slot(&self) -> EquipmentSlot {
        self.slot
    }
}

impl fmt::Display for ItemUnequipped {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Unequipped {} slot", self.slot)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::StatusEffectType;
    use rstest::rstest;
    use uuid::Uuid;

    fn create_item_identifier() -> ItemIdentifier {
        ItemIdentifier::from_uuid(Uuid::new_v4())
    }

    // =========================================================================
    // EquipmentSlot Tests
    // =========================================================================

    mod equipment_slot {
        use super::*;

        #[rstest]
        fn all_returns_four_variants() {
            assert_eq!(EquipmentSlot::all().len(), 4);
        }

        #[rstest]
        fn all_contains_all_variants() {
            let slots = EquipmentSlot::all();
            assert!(slots.contains(&EquipmentSlot::Weapon));
            assert!(slots.contains(&EquipmentSlot::Armor));
            assert!(slots.contains(&EquipmentSlot::Helmet));
            assert!(slots.contains(&EquipmentSlot::Accessory));
        }

        #[rstest]
        #[case(EquipmentSlot::Weapon, "Weapon")]
        #[case(EquipmentSlot::Armor, "Armor")]
        #[case(EquipmentSlot::Helmet, "Helmet")]
        #[case(EquipmentSlot::Accessory, "Accessory")]
        fn display_format(#[case] slot: EquipmentSlot, #[case] expected: &str) {
            assert_eq!(format!("{}", slot), expected);
        }

        #[rstest]
        fn equality() {
            assert_eq!(EquipmentSlot::Weapon, EquipmentSlot::Weapon);
            assert_ne!(EquipmentSlot::Weapon, EquipmentSlot::Armor);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let mut set = HashSet::new();
            set.insert(EquipmentSlot::Weapon);

            assert!(set.contains(&EquipmentSlot::Weapon));
            assert!(!set.contains(&EquipmentSlot::Armor));
        }
    }

    // =========================================================================
    // ItemPickedUp Tests
    // =========================================================================

    mod item_picked_up {
        use super::*;

        #[rstest]
        fn new_creates_event() {
            let identifier = create_item_identifier();
            let event = ItemPickedUp::new(identifier);
            assert_eq!(event.item_identifier(), identifier);
        }

        #[rstest]
        fn display_format() {
            let identifier = create_item_identifier();
            let event = ItemPickedUp::new(identifier);
            let display = format!("{}", event);
            assert!(display.contains("Picked up item"));
        }

        #[rstest]
        fn equality() {
            let identifier = create_item_identifier();
            let event1 = ItemPickedUp::new(identifier);
            let event2 = ItemPickedUp::new(identifier);
            let event3 = ItemPickedUp::new(create_item_identifier());

            assert_eq!(event1, event2);
            assert_ne!(event1, event3);
        }

        #[rstest]
        fn clone() {
            let event = ItemPickedUp::new(create_item_identifier());
            let cloned = event;
            assert_eq!(event, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let identifier = create_item_identifier();
            let event1 = ItemPickedUp::new(identifier);
            let event2 = ItemPickedUp::new(identifier);

            let mut set = HashSet::new();
            set.insert(event1);

            assert!(set.contains(&event2));
        }
    }

    // =========================================================================
    // ItemDropped Tests
    // =========================================================================

    mod item_dropped {
        use super::*;

        #[rstest]
        fn new_creates_event() {
            let identifier = create_item_identifier();
            let position = Position::new(5, 10);
            let event = ItemDropped::new(identifier, position);

            assert_eq!(event.item_identifier(), identifier);
            assert_eq!(event.position(), position);
        }

        #[rstest]
        fn display_format() {
            let identifier = create_item_identifier();
            let position = Position::new(5, 10);
            let event = ItemDropped::new(identifier, position);
            let display = format!("{}", event);

            assert!(display.contains("Dropped item"));
            assert!(display.contains("(5, 10)"));
        }

        #[rstest]
        fn equality() {
            let identifier = create_item_identifier();
            let position = Position::new(5, 10);
            let event1 = ItemDropped::new(identifier, position);
            let event2 = ItemDropped::new(identifier, position);
            let event3 = ItemDropped::new(identifier, Position::new(1, 1));

            assert_eq!(event1, event2);
            assert_ne!(event1, event3);
        }

        #[rstest]
        fn clone() {
            let event = ItemDropped::new(create_item_identifier(), Position::new(0, 0));
            let cloned = event;
            assert_eq!(event, cloned);
        }
    }

    // =========================================================================
    // ItemUsed Tests
    // =========================================================================

    mod item_used {
        use super::*;

        #[rstest]
        fn new_creates_event() {
            let identifier = create_item_identifier();
            let effect = ItemEffect::Healed { amount: 50 };
            let event = ItemUsed::new(identifier, effect);

            assert_eq!(event.item_identifier(), identifier);
            assert_eq!(event.effect(), effect);
        }

        #[rstest]
        fn display_format() {
            let identifier = create_item_identifier();
            let effect = ItemEffect::Healed { amount: 50 };
            let event = ItemUsed::new(identifier, effect);
            let display = format!("{}", event);

            assert!(display.contains("Used item"));
            assert!(display.contains("Healed 50 HP"));
        }

        #[rstest]
        fn with_status_effect() {
            let identifier = create_item_identifier();
            let effect = ItemEffect::StatusApplied {
                effect: StatusEffectType::Haste,
            };
            let event = ItemUsed::new(identifier, effect);

            assert_eq!(event.effect(), effect);
        }

        #[rstest]
        fn equality() {
            let identifier = create_item_identifier();
            let effect = ItemEffect::Healed { amount: 50 };
            let event1 = ItemUsed::new(identifier, effect);
            let event2 = ItemUsed::new(identifier, effect);
            let event3 = ItemUsed::new(identifier, ItemEffect::Healed { amount: 100 });

            assert_eq!(event1, event2);
            assert_ne!(event1, event3);
        }

        #[rstest]
        fn clone() {
            let event = ItemUsed::new(create_item_identifier(), ItemEffect::Healed { amount: 50 });
            let cloned = event;
            assert_eq!(event, cloned);
        }
    }

    // =========================================================================
    // ItemEquipped Tests
    // =========================================================================

    mod item_equipped {
        use super::*;

        #[rstest]
        fn new_creates_event() {
            let identifier = create_item_identifier();
            let slot = EquipmentSlot::Weapon;
            let event = ItemEquipped::new(identifier, slot);

            assert_eq!(event.item_identifier(), identifier);
            assert_eq!(event.slot(), slot);
        }

        #[rstest]
        fn display_format() {
            let identifier = create_item_identifier();
            let event = ItemEquipped::new(identifier, EquipmentSlot::Weapon);
            let display = format!("{}", event);

            assert!(display.contains("Equipped item"));
            assert!(display.contains("Weapon slot"));
        }

        #[rstest]
        #[case(EquipmentSlot::Weapon)]
        #[case(EquipmentSlot::Armor)]
        #[case(EquipmentSlot::Helmet)]
        #[case(EquipmentSlot::Accessory)]
        fn all_slots(#[case] slot: EquipmentSlot) {
            let identifier = create_item_identifier();
            let event = ItemEquipped::new(identifier, slot);
            assert_eq!(event.slot(), slot);
        }

        #[rstest]
        fn equality() {
            let identifier = create_item_identifier();
            let event1 = ItemEquipped::new(identifier, EquipmentSlot::Weapon);
            let event2 = ItemEquipped::new(identifier, EquipmentSlot::Weapon);
            let event3 = ItemEquipped::new(identifier, EquipmentSlot::Armor);

            assert_eq!(event1, event2);
            assert_ne!(event1, event3);
        }

        #[rstest]
        fn clone() {
            let event = ItemEquipped::new(create_item_identifier(), EquipmentSlot::Armor);
            let cloned = event;
            assert_eq!(event, cloned);
        }
    }

    // =========================================================================
    // ItemUnequipped Tests
    // =========================================================================

    mod item_unequipped {
        use super::*;

        #[rstest]
        fn new_creates_event() {
            let slot = EquipmentSlot::Weapon;
            let event = ItemUnequipped::new(slot);
            assert_eq!(event.slot(), slot);
        }

        #[rstest]
        fn display_format() {
            let event = ItemUnequipped::new(EquipmentSlot::Helmet);
            let display = format!("{}", event);

            assert!(display.contains("Unequipped"));
            assert!(display.contains("Helmet slot"));
        }

        #[rstest]
        #[case(EquipmentSlot::Weapon)]
        #[case(EquipmentSlot::Armor)]
        #[case(EquipmentSlot::Helmet)]
        #[case(EquipmentSlot::Accessory)]
        fn all_slots(#[case] slot: EquipmentSlot) {
            let event = ItemUnequipped::new(slot);
            assert_eq!(event.slot(), slot);
        }

        #[rstest]
        fn equality() {
            let event1 = ItemUnequipped::new(EquipmentSlot::Weapon);
            let event2 = ItemUnequipped::new(EquipmentSlot::Weapon);
            let event3 = ItemUnequipped::new(EquipmentSlot::Armor);

            assert_eq!(event1, event2);
            assert_ne!(event1, event3);
        }

        #[rstest]
        fn clone() {
            let event = ItemUnequipped::new(EquipmentSlot::Accessory);
            let cloned = event;
            assert_eq!(event, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let event1 = ItemUnequipped::new(EquipmentSlot::Weapon);
            let event2 = ItemUnequipped::new(EquipmentSlot::Weapon);

            let mut set = HashSet::new();
            set.insert(event1);

            assert!(set.contains(&event2));
        }
    }
}
