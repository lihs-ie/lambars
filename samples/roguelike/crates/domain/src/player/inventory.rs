//! Player equipment and inventory types.
//!
//! This module provides types for managing player equipment:
//!
//! - **EquipmentSlot**: Enum representing equipment slot types
//! - **EquipmentSlots**: Struct holding all equipment slots
//!
//! Note: `Inventory` and `ItemStack` are not implemented yet as they depend
//! on the `Item` type which is not yet available.

use std::fmt;

// =============================================================================
// EquipmentSlot
// =============================================================================

/// Represents the type of equipment slot.
///
/// Each slot type can hold a specific category of equipment item.
///
/// # Examples
///
/// ```
/// use roguelike_domain::player::EquipmentSlot;
///
/// let slot = EquipmentSlot::Weapon;
/// println!("Equipping to: {}", slot);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquipmentSlot {
    /// Weapon slot for swords, staffs, bows, etc.
    Weapon,
    /// Armor slot for body armor.
    Armor,
    /// Helmet slot for head protection.
    Helmet,
    /// Accessory slot for rings, amulets, etc.
    Accessory,
}

impl EquipmentSlot {
    /// Returns all equipment slot types.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::EquipmentSlot;
    ///
    /// let slots = EquipmentSlot::all();
    /// assert_eq!(slots.len(), 4);
    /// ```
    #[must_use]
    pub const fn all() -> [Self; 4] {
        [Self::Weapon, Self::Armor, Self::Helmet, Self::Accessory]
    }

    /// Returns the slot name as a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::EquipmentSlot;
    ///
    /// assert_eq!(EquipmentSlot::Weapon.name(), "Weapon");
    /// ```
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Weapon => "Weapon",
            Self::Armor => "Armor",
            Self::Helmet => "Helmet",
            Self::Accessory => "Accessory",
        }
    }
}

impl fmt::Display for EquipmentSlot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.name())
    }
}

// =============================================================================
// EquipmentSlots
// =============================================================================

/// Container for all equipment slots.
///
/// Each slot can optionally hold an item identifier.
/// Note: Currently using placeholder type (`Option<String>`) as `Item` is not yet implemented.
///
/// # Examples
///
/// ```
/// use roguelike_domain::player::{EquipmentSlots, EquipmentSlot};
///
/// let equipment = EquipmentSlots::empty();
/// assert!(equipment.is_slot_empty(EquipmentSlot::Weapon));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EquipmentSlots {
    /// The weapon slot.
    weapon: Option<String>, // TODO: Replace with Option<Item> when Item is implemented
    /// The armor slot.
    armor: Option<String>, // TODO: Replace with Option<Item> when Item is implemented
    /// The helmet slot.
    helmet: Option<String>, // TODO: Replace with Option<Item> when Item is implemented
    /// The accessory slot.
    accessory: Option<String>, // TODO: Replace with Option<Item> when Item is implemented
}

impl EquipmentSlots {
    /// Creates empty equipment slots.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::EquipmentSlots;
    ///
    /// let equipment = EquipmentSlots::empty();
    /// ```
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            weapon: None,
            armor: None,
            helmet: None,
            accessory: None,
        }
    }

    /// Returns a reference to the weapon slot.
    #[must_use]
    pub fn weapon(&self) -> Option<&str> {
        self.weapon.as_deref()
    }

    /// Returns a reference to the armor slot.
    #[must_use]
    pub fn armor(&self) -> Option<&str> {
        self.armor.as_deref()
    }

    /// Returns a reference to the helmet slot.
    #[must_use]
    pub fn helmet(&self) -> Option<&str> {
        self.helmet.as_deref()
    }

    /// Returns a reference to the accessory slot.
    #[must_use]
    pub fn accessory(&self) -> Option<&str> {
        self.accessory.as_deref()
    }

    /// Returns the item in the specified slot.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::{EquipmentSlots, EquipmentSlot};
    ///
    /// let equipment = EquipmentSlots::empty();
    /// assert!(equipment.get(EquipmentSlot::Weapon).is_none());
    /// ```
    #[must_use]
    pub fn get(&self, slot: EquipmentSlot) -> Option<&str> {
        match slot {
            EquipmentSlot::Weapon => self.weapon(),
            EquipmentSlot::Armor => self.armor(),
            EquipmentSlot::Helmet => self.helmet(),
            EquipmentSlot::Accessory => self.accessory(),
        }
    }

    /// Checks if the specified slot is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::{EquipmentSlots, EquipmentSlot};
    ///
    /// let equipment = EquipmentSlots::empty();
    /// assert!(equipment.is_slot_empty(EquipmentSlot::Weapon));
    /// ```
    #[must_use]
    pub fn is_slot_empty(&self, slot: EquipmentSlot) -> bool {
        self.get(slot).is_none()
    }

    /// Checks if the specified slot is occupied.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::{EquipmentSlots, EquipmentSlot};
    ///
    /// let equipment = EquipmentSlots::empty();
    /// assert!(!equipment.is_slot_occupied(EquipmentSlot::Weapon));
    /// ```
    #[must_use]
    pub fn is_slot_occupied(&self, slot: EquipmentSlot) -> bool {
        self.get(slot).is_some()
    }

    /// Equips an item to the specified slot, returning new `EquipmentSlots`.
    ///
    /// This is an immutable operation that returns a new instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::{EquipmentSlots, EquipmentSlot};
    ///
    /// let equipment = EquipmentSlots::empty();
    /// let equipped = equipment.equip(EquipmentSlot::Weapon, "sword_01".to_string());
    /// assert!(equipped.is_slot_occupied(EquipmentSlot::Weapon));
    /// ```
    #[must_use]
    pub fn equip(&self, slot: EquipmentSlot, item: String) -> Self {
        let mut new_slots = self.clone();
        match slot {
            EquipmentSlot::Weapon => new_slots.weapon = Some(item),
            EquipmentSlot::Armor => new_slots.armor = Some(item),
            EquipmentSlot::Helmet => new_slots.helmet = Some(item),
            EquipmentSlot::Accessory => new_slots.accessory = Some(item),
        }
        new_slots
    }

    /// Unequips an item from the specified slot, returning new `EquipmentSlots`.
    ///
    /// This is an immutable operation that returns a new instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::{EquipmentSlots, EquipmentSlot};
    ///
    /// let equipment = EquipmentSlots::empty()
    ///     .equip(EquipmentSlot::Weapon, "sword_01".to_string());
    /// let unequipped = equipment.unequip(EquipmentSlot::Weapon);
    /// assert!(unequipped.is_slot_empty(EquipmentSlot::Weapon));
    /// ```
    #[must_use]
    pub fn unequip(&self, slot: EquipmentSlot) -> Self {
        let mut new_slots = self.clone();
        match slot {
            EquipmentSlot::Weapon => new_slots.weapon = None,
            EquipmentSlot::Armor => new_slots.armor = None,
            EquipmentSlot::Helmet => new_slots.helmet = None,
            EquipmentSlot::Accessory => new_slots.accessory = None,
        }
        new_slots
    }

    /// Returns the number of equipped items.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::{EquipmentSlots, EquipmentSlot};
    ///
    /// let equipment = EquipmentSlots::empty()
    ///     .equip(EquipmentSlot::Weapon, "sword".to_string())
    ///     .equip(EquipmentSlot::Armor, "plate".to_string());
    /// assert_eq!(equipment.equipped_count(), 2);
    /// ```
    #[must_use]
    pub fn equipped_count(&self) -> usize {
        EquipmentSlot::all()
            .iter()
            .filter(|slot| self.is_slot_occupied(**slot))
            .count()
    }

    /// Returns true if all slots are empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::EquipmentSlots;
    ///
    /// let equipment = EquipmentSlots::empty();
    /// assert!(equipment.is_all_empty());
    /// ```
    #[must_use]
    pub fn is_all_empty(&self) -> bool {
        self.equipped_count() == 0
    }

    /// Returns true if all slots are occupied.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::{EquipmentSlots, EquipmentSlot};
    ///
    /// let equipment = EquipmentSlots::empty()
    ///     .equip(EquipmentSlot::Weapon, "sword".to_string())
    ///     .equip(EquipmentSlot::Armor, "plate".to_string())
    ///     .equip(EquipmentSlot::Helmet, "helm".to_string())
    ///     .equip(EquipmentSlot::Accessory, "ring".to_string());
    /// assert!(equipment.is_fully_equipped());
    /// ```
    #[must_use]
    pub fn is_fully_equipped(&self) -> bool {
        self.equipped_count() == EquipmentSlot::all().len()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // EquipmentSlot Tests
    // =========================================================================

    mod equipment_slot {
        use super::*;

        #[rstest]
        fn all_returns_four_slots() {
            let slots = EquipmentSlot::all();
            assert_eq!(slots.len(), 4);
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
        fn name_returns_correct_string(#[case] slot: EquipmentSlot, #[case] expected: &str) {
            assert_eq!(slot.name(), expected);
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
        fn clone() {
            let slot = EquipmentSlot::Weapon;
            let cloned = slot;
            assert_eq!(slot, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let mut set = HashSet::new();
            set.insert(EquipmentSlot::Weapon);

            assert!(set.contains(&EquipmentSlot::Weapon));
            assert!(!set.contains(&EquipmentSlot::Armor));
        }

        #[rstest]
        fn debug_format() {
            let slot = EquipmentSlot::Weapon;
            let debug_string = format!("{:?}", slot);
            assert!(debug_string.contains("Weapon"));
        }
    }

    // =========================================================================
    // EquipmentSlots Tests
    // =========================================================================

    mod equipment_slots {
        use super::*;

        #[rstest]
        fn empty_creates_empty_slots() {
            let equipment = EquipmentSlots::empty();
            assert!(equipment.is_all_empty());
        }

        #[rstest]
        fn default_creates_empty_slots() {
            let equipment = EquipmentSlots::default();
            assert!(equipment.is_all_empty());
        }

        #[rstest]
        fn weapon_returns_none_when_empty() {
            let equipment = EquipmentSlots::empty();
            assert!(equipment.weapon().is_none());
        }

        #[rstest]
        fn armor_returns_none_when_empty() {
            let equipment = EquipmentSlots::empty();
            assert!(equipment.armor().is_none());
        }

        #[rstest]
        fn helmet_returns_none_when_empty() {
            let equipment = EquipmentSlots::empty();
            assert!(equipment.helmet().is_none());
        }

        #[rstest]
        fn accessory_returns_none_when_empty() {
            let equipment = EquipmentSlots::empty();
            assert!(equipment.accessory().is_none());
        }

        #[rstest]
        fn get_returns_item_in_slot() {
            let equipment =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "sword".to_string());
            assert_eq!(equipment.get(EquipmentSlot::Weapon), Some("sword"));
        }

        #[rstest]
        fn get_returns_none_for_empty_slot() {
            let equipment = EquipmentSlots::empty();
            assert!(equipment.get(EquipmentSlot::Weapon).is_none());
        }

        #[rstest]
        fn is_slot_empty_returns_true_for_empty_slot() {
            let equipment = EquipmentSlots::empty();
            assert!(equipment.is_slot_empty(EquipmentSlot::Weapon));
        }

        #[rstest]
        fn is_slot_empty_returns_false_for_occupied_slot() {
            let equipment =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "sword".to_string());
            assert!(!equipment.is_slot_empty(EquipmentSlot::Weapon));
        }

        #[rstest]
        fn is_slot_occupied_returns_false_for_empty_slot() {
            let equipment = EquipmentSlots::empty();
            assert!(!equipment.is_slot_occupied(EquipmentSlot::Weapon));
        }

        #[rstest]
        fn is_slot_occupied_returns_true_for_occupied_slot() {
            let equipment =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "sword".to_string());
            assert!(equipment.is_slot_occupied(EquipmentSlot::Weapon));
        }

        #[rstest]
        fn equip_adds_item_to_slot() {
            let equipment = EquipmentSlots::empty();
            let equipped = equipment.equip(EquipmentSlot::Weapon, "sword".to_string());
            assert_eq!(equipped.weapon(), Some("sword"));
        }

        #[rstest]
        fn equip_to_armor_slot() {
            let equipment = EquipmentSlots::empty();
            let equipped = equipment.equip(EquipmentSlot::Armor, "plate".to_string());
            assert_eq!(equipped.armor(), Some("plate"));
        }

        #[rstest]
        fn equip_to_helmet_slot() {
            let equipment = EquipmentSlots::empty();
            let equipped = equipment.equip(EquipmentSlot::Helmet, "helm".to_string());
            assert_eq!(equipped.helmet(), Some("helm"));
        }

        #[rstest]
        fn equip_to_accessory_slot() {
            let equipment = EquipmentSlots::empty();
            let equipped = equipment.equip(EquipmentSlot::Accessory, "ring".to_string());
            assert_eq!(equipped.accessory(), Some("ring"));
        }

        #[rstest]
        fn equip_does_not_modify_original() {
            let original = EquipmentSlots::empty();
            let _ = original.equip(EquipmentSlot::Weapon, "sword".to_string());
            assert!(original.is_slot_empty(EquipmentSlot::Weapon));
        }

        #[rstest]
        fn equip_replaces_existing_item() {
            let equipment =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "old_sword".to_string());
            let replaced = equipment.equip(EquipmentSlot::Weapon, "new_sword".to_string());
            assert_eq!(replaced.weapon(), Some("new_sword"));
        }

        #[rstest]
        fn unequip_removes_item_from_slot() {
            let equipment =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "sword".to_string());
            let unequipped = equipment.unequip(EquipmentSlot::Weapon);
            assert!(unequipped.weapon().is_none());
        }

        #[rstest]
        fn unequip_from_armor_slot() {
            let equipment =
                EquipmentSlots::empty().equip(EquipmentSlot::Armor, "plate".to_string());
            let unequipped = equipment.unequip(EquipmentSlot::Armor);
            assert!(unequipped.armor().is_none());
        }

        #[rstest]
        fn unequip_from_helmet_slot() {
            let equipment =
                EquipmentSlots::empty().equip(EquipmentSlot::Helmet, "helm".to_string());
            let unequipped = equipment.unequip(EquipmentSlot::Helmet);
            assert!(unequipped.helmet().is_none());
        }

        #[rstest]
        fn unequip_from_accessory_slot() {
            let equipment =
                EquipmentSlots::empty().equip(EquipmentSlot::Accessory, "ring".to_string());
            let unequipped = equipment.unequip(EquipmentSlot::Accessory);
            assert!(unequipped.accessory().is_none());
        }

        #[rstest]
        fn unequip_does_not_modify_original() {
            let original =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "sword".to_string());
            let _ = original.unequip(EquipmentSlot::Weapon);
            assert!(original.is_slot_occupied(EquipmentSlot::Weapon));
        }

        #[rstest]
        fn unequip_from_empty_slot_returns_empty_slot() {
            let equipment = EquipmentSlots::empty();
            let unequipped = equipment.unequip(EquipmentSlot::Weapon);
            assert!(unequipped.is_slot_empty(EquipmentSlot::Weapon));
        }

        #[rstest]
        fn equipped_count_returns_zero_for_empty() {
            let equipment = EquipmentSlots::empty();
            assert_eq!(equipment.equipped_count(), 0);
        }

        #[rstest]
        fn equipped_count_returns_correct_count() {
            let equipment = EquipmentSlots::empty()
                .equip(EquipmentSlot::Weapon, "sword".to_string())
                .equip(EquipmentSlot::Armor, "plate".to_string());
            assert_eq!(equipment.equipped_count(), 2);
        }

        #[rstest]
        fn equipped_count_returns_four_when_fully_equipped() {
            let equipment = EquipmentSlots::empty()
                .equip(EquipmentSlot::Weapon, "sword".to_string())
                .equip(EquipmentSlot::Armor, "plate".to_string())
                .equip(EquipmentSlot::Helmet, "helm".to_string())
                .equip(EquipmentSlot::Accessory, "ring".to_string());
            assert_eq!(equipment.equipped_count(), 4);
        }

        #[rstest]
        fn is_all_empty_returns_true_for_empty() {
            let equipment = EquipmentSlots::empty();
            assert!(equipment.is_all_empty());
        }

        #[rstest]
        fn is_all_empty_returns_false_when_any_equipped() {
            let equipment =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "sword".to_string());
            assert!(!equipment.is_all_empty());
        }

        #[rstest]
        fn is_fully_equipped_returns_false_for_empty() {
            let equipment = EquipmentSlots::empty();
            assert!(!equipment.is_fully_equipped());
        }

        #[rstest]
        fn is_fully_equipped_returns_false_when_partial() {
            let equipment = EquipmentSlots::empty()
                .equip(EquipmentSlot::Weapon, "sword".to_string())
                .equip(EquipmentSlot::Armor, "plate".to_string());
            assert!(!equipment.is_fully_equipped());
        }

        #[rstest]
        fn is_fully_equipped_returns_true_when_all_slots_filled() {
            let equipment = EquipmentSlots::empty()
                .equip(EquipmentSlot::Weapon, "sword".to_string())
                .equip(EquipmentSlot::Armor, "plate".to_string())
                .equip(EquipmentSlot::Helmet, "helm".to_string())
                .equip(EquipmentSlot::Accessory, "ring".to_string());
            assert!(equipment.is_fully_equipped());
        }

        #[rstest]
        fn equality() {
            let equipment1 =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "sword".to_string());
            let equipment2 =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "sword".to_string());
            assert_eq!(equipment1, equipment2);
        }

        #[rstest]
        fn inequality() {
            let equipment1 =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "sword".to_string());
            let equipment2 =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "axe".to_string());
            assert_ne!(equipment1, equipment2);
        }

        #[rstest]
        fn clone() {
            let equipment =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "sword".to_string());
            let cloned = equipment.clone();
            assert_eq!(equipment, cloned);
        }

        #[rstest]
        fn debug_format() {
            let equipment = EquipmentSlots::empty();
            let debug_string = format!("{:?}", equipment);
            assert!(debug_string.contains("EquipmentSlots"));
        }

        #[rstest]
        fn chain_multiple_equips() {
            let equipment = EquipmentSlots::empty()
                .equip(EquipmentSlot::Weapon, "sword".to_string())
                .equip(EquipmentSlot::Armor, "plate".to_string())
                .equip(EquipmentSlot::Helmet, "helm".to_string())
                .equip(EquipmentSlot::Accessory, "ring".to_string());

            assert_eq!(equipment.weapon(), Some("sword"));
            assert_eq!(equipment.armor(), Some("plate"));
            assert_eq!(equipment.helmet(), Some("helm"));
            assert_eq!(equipment.accessory(), Some("ring"));
        }
    }
}
