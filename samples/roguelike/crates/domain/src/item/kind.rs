//! Item kind enumeration.
//!
//! This module provides the `ItemKind` enum that categorizes items
//! into different types with their associated data.

use std::fmt;

use super::armor::ArmorData;
use super::consumable::ConsumableData;
use super::material::MaterialData;
use super::weapon::WeaponData;

// =============================================================================
// ItemKind
// =============================================================================

/// The kind of item with associated type-specific data.
///
/// Each variant contains the data specific to that item type.
///
/// # Examples
///
/// ```
/// use roguelike_domain::item::{ItemKind, WeaponData, WeaponType};
/// use roguelike_domain::common::Attack;
///
/// let sword = ItemKind::Weapon(WeaponData::new(Attack::new(25), WeaponType::Sword, 1));
/// assert!(sword.is_equipable());
/// assert!(!sword.is_stackable());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemKind {
    /// A weapon that can be equipped for combat.
    Weapon(WeaponData),
    /// Armor that can be equipped for defense.
    Armor(ArmorData),
    /// A consumable item that can be used for effects.
    Consumable(ConsumableData),
    /// A crafting material.
    Material(MaterialData),
}

impl ItemKind {
    /// Returns true if this item can be equipped.
    ///
    /// Only weapons and armor are equipable.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::item::{ItemKind, WeaponData, WeaponType, ConsumableData, ConsumableEffect};
    /// use roguelike_domain::common::Attack;
    ///
    /// let weapon = ItemKind::Weapon(WeaponData::new(Attack::new(10), WeaponType::Sword, 1));
    /// let potion = ItemKind::Consumable(ConsumableData::new(ConsumableEffect::Heal { amount: 50 }, 10));
    ///
    /// assert!(weapon.is_equipable());
    /// assert!(!potion.is_equipable());
    /// ```
    #[must_use]
    pub const fn is_equipable(&self) -> bool {
        matches!(self, Self::Weapon(_) | Self::Armor(_))
    }

    /// Returns true if this item can be used (consumed).
    ///
    /// Only consumables are usable.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::item::{ItemKind, ConsumableData, ConsumableEffect, MaterialData};
    /// use roguelike_domain::common::Rarity;
    ///
    /// let potion = ItemKind::Consumable(ConsumableData::new(ConsumableEffect::Heal { amount: 50 }, 10));
    /// let ore = ItemKind::Material(MaterialData::new(Rarity::Common, 99));
    ///
    /// assert!(potion.is_usable());
    /// assert!(!ore.is_usable());
    /// ```
    #[must_use]
    pub const fn is_usable(&self) -> bool {
        matches!(self, Self::Consumable(_))
    }

    /// Returns true if this item can stack in inventory.
    ///
    /// Consumables and materials are stackable; weapons and armor are not.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::item::{ItemKind, MaterialData, ArmorData, ArmorSlot};
    /// use roguelike_domain::common::{Rarity, Defense};
    ///
    /// let ore = ItemKind::Material(MaterialData::new(Rarity::Common, 99));
    /// let armor = ItemKind::Armor(ArmorData::new(Defense::new(10), ArmorSlot::Body));
    ///
    /// assert!(ore.is_stackable());
    /// assert!(!armor.is_stackable());
    /// ```
    #[must_use]
    pub const fn is_stackable(&self) -> bool {
        matches!(self, Self::Consumable(_) | Self::Material(_))
    }

    /// Returns the maximum stack size for this item kind.
    ///
    /// Returns 1 for non-stackable items (weapons and armor).
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::item::{ItemKind, MaterialData, WeaponData, WeaponType};
    /// use roguelike_domain::common::{Rarity, Attack};
    ///
    /// let ore = ItemKind::Material(MaterialData::new(Rarity::Common, 99));
    /// let sword = ItemKind::Weapon(WeaponData::new(Attack::new(10), WeaponType::Sword, 1));
    ///
    /// assert_eq!(ore.max_stack(), 99);
    /// assert_eq!(sword.max_stack(), 1);
    /// ```
    #[must_use]
    pub const fn max_stack(&self) -> u32 {
        match self {
            Self::Weapon(_) | Self::Armor(_) => 1,
            Self::Consumable(data) => data.max_stack(),
            Self::Material(data) => data.max_stack(),
        }
    }

    /// Returns the weapon data if this is a weapon, None otherwise.
    #[must_use]
    pub const fn as_weapon(&self) -> Option<&WeaponData> {
        match self {
            Self::Weapon(data) => Some(data),
            _ => None,
        }
    }

    /// Returns the armor data if this is armor, None otherwise.
    #[must_use]
    pub const fn as_armor(&self) -> Option<&ArmorData> {
        match self {
            Self::Armor(data) => Some(data),
            _ => None,
        }
    }

    /// Returns the consumable data if this is a consumable, None otherwise.
    #[must_use]
    pub const fn as_consumable(&self) -> Option<&ConsumableData> {
        match self {
            Self::Consumable(data) => Some(data),
            _ => None,
        }
    }

    /// Returns the material data if this is a material, None otherwise.
    #[must_use]
    pub const fn as_material(&self) -> Option<&MaterialData> {
        match self {
            Self::Material(data) => Some(data),
            _ => None,
        }
    }
}

impl fmt::Display for ItemKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Weapon(data) => write!(formatter, "Weapon: {}", data),
            Self::Armor(data) => write!(formatter, "Armor: {}", data),
            Self::Consumable(data) => write!(formatter, "Consumable: {}", data),
            Self::Material(data) => write!(formatter, "Material: {}", data),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{Attack, Defense, Rarity, StatusEffectType};
    use crate::item::{ArmorSlot, ConsumableEffect, WeaponType};
    use rstest::rstest;

    fn create_weapon_kind() -> ItemKind {
        ItemKind::Weapon(WeaponData::new(Attack::new(25), WeaponType::Sword, 1))
    }

    fn create_armor_kind() -> ItemKind {
        ItemKind::Armor(ArmorData::new(Defense::new(15), ArmorSlot::Body))
    }

    fn create_consumable_kind() -> ItemKind {
        ItemKind::Consumable(ConsumableData::new(
            ConsumableEffect::Heal { amount: 50 },
            10,
        ))
    }

    fn create_material_kind() -> ItemKind {
        ItemKind::Material(MaterialData::new(Rarity::Rare, 20))
    }

    // =========================================================================
    // is_equipable Tests
    // =========================================================================

    #[rstest]
    fn weapon_is_equipable() {
        assert!(create_weapon_kind().is_equipable());
    }

    #[rstest]
    fn armor_is_equipable() {
        assert!(create_armor_kind().is_equipable());
    }

    #[rstest]
    fn consumable_is_not_equipable() {
        assert!(!create_consumable_kind().is_equipable());
    }

    #[rstest]
    fn material_is_not_equipable() {
        assert!(!create_material_kind().is_equipable());
    }

    // =========================================================================
    // is_usable Tests
    // =========================================================================

    #[rstest]
    fn consumable_is_usable() {
        assert!(create_consumable_kind().is_usable());
    }

    #[rstest]
    fn weapon_is_not_usable() {
        assert!(!create_weapon_kind().is_usable());
    }

    #[rstest]
    fn armor_is_not_usable() {
        assert!(!create_armor_kind().is_usable());
    }

    #[rstest]
    fn material_is_not_usable() {
        assert!(!create_material_kind().is_usable());
    }

    // =========================================================================
    // is_stackable Tests
    // =========================================================================

    #[rstest]
    fn consumable_is_stackable() {
        assert!(create_consumable_kind().is_stackable());
    }

    #[rstest]
    fn material_is_stackable() {
        assert!(create_material_kind().is_stackable());
    }

    #[rstest]
    fn weapon_is_not_stackable() {
        assert!(!create_weapon_kind().is_stackable());
    }

    #[rstest]
    fn armor_is_not_stackable() {
        assert!(!create_armor_kind().is_stackable());
    }

    // =========================================================================
    // max_stack Tests
    // =========================================================================

    #[rstest]
    fn weapon_max_stack_is_one() {
        assert_eq!(create_weapon_kind().max_stack(), 1);
    }

    #[rstest]
    fn armor_max_stack_is_one() {
        assert_eq!(create_armor_kind().max_stack(), 1);
    }

    #[rstest]
    fn consumable_max_stack() {
        assert_eq!(create_consumable_kind().max_stack(), 10);
    }

    #[rstest]
    fn material_max_stack() {
        assert_eq!(create_material_kind().max_stack(), 20);
    }

    // =========================================================================
    // as_* Methods Tests
    // =========================================================================

    #[rstest]
    fn as_weapon_on_weapon() {
        let kind = create_weapon_kind();
        assert!(kind.as_weapon().is_some());
        assert_eq!(kind.as_weapon().unwrap().weapon_type(), WeaponType::Sword);
    }

    #[rstest]
    fn as_weapon_on_non_weapon() {
        assert!(create_armor_kind().as_weapon().is_none());
        assert!(create_consumable_kind().as_weapon().is_none());
        assert!(create_material_kind().as_weapon().is_none());
    }

    #[rstest]
    fn as_armor_on_armor() {
        let kind = create_armor_kind();
        assert!(kind.as_armor().is_some());
        assert_eq!(kind.as_armor().unwrap().armor_slot(), ArmorSlot::Body);
    }

    #[rstest]
    fn as_armor_on_non_armor() {
        assert!(create_weapon_kind().as_armor().is_none());
        assert!(create_consumable_kind().as_armor().is_none());
        assert!(create_material_kind().as_armor().is_none());
    }

    #[rstest]
    fn as_consumable_on_consumable() {
        let kind = create_consumable_kind();
        assert!(kind.as_consumable().is_some());
        assert_eq!(
            kind.as_consumable().unwrap().effect(),
            ConsumableEffect::Heal { amount: 50 }
        );
    }

    #[rstest]
    fn as_consumable_on_non_consumable() {
        assert!(create_weapon_kind().as_consumable().is_none());
        assert!(create_armor_kind().as_consumable().is_none());
        assert!(create_material_kind().as_consumable().is_none());
    }

    #[rstest]
    fn as_material_on_material() {
        let kind = create_material_kind();
        assert!(kind.as_material().is_some());
        assert_eq!(kind.as_material().unwrap().rarity(), Rarity::Rare);
    }

    #[rstest]
    fn as_material_on_non_material() {
        assert!(create_weapon_kind().as_material().is_none());
        assert!(create_armor_kind().as_material().is_none());
        assert!(create_consumable_kind().as_material().is_none());
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[rstest]
    fn display_weapon() {
        let kind = create_weapon_kind();
        let display = format!("{}", kind);
        assert!(display.starts_with("Weapon:"));
        assert!(display.contains("Sword"));
    }

    #[rstest]
    fn display_armor() {
        let kind = create_armor_kind();
        let display = format!("{}", kind);
        assert!(display.starts_with("Armor:"));
        assert!(display.contains("Body"));
    }

    #[rstest]
    fn display_consumable() {
        let kind = create_consumable_kind();
        let display = format!("{}", kind);
        assert!(display.starts_with("Consumable:"));
        assert!(display.contains("Heal"));
    }

    #[rstest]
    fn display_material() {
        let kind = create_material_kind();
        let display = format!("{}", kind);
        assert!(display.starts_with("Material:"));
        assert!(display.contains("Rare"));
    }

    // =========================================================================
    // Equality and Hash Tests
    // =========================================================================

    #[rstest]
    fn equality_same_kind() {
        let kind1 = create_weapon_kind();
        let kind2 = create_weapon_kind();
        assert_eq!(kind1, kind2);
    }

    #[rstest]
    fn equality_different_kind() {
        assert_ne!(create_weapon_kind(), create_armor_kind());
        assert_ne!(create_consumable_kind(), create_material_kind());
    }

    #[rstest]
    fn equality_same_variant_different_data() {
        let kind1 = ItemKind::Weapon(WeaponData::new(Attack::new(25), WeaponType::Sword, 1));
        let kind2 = ItemKind::Weapon(WeaponData::new(Attack::new(30), WeaponType::Sword, 1));
        assert_ne!(kind1, kind2);
    }

    #[rstest]
    fn clone() {
        let kind = create_weapon_kind();
        let cloned = kind;
        assert_eq!(kind, cloned);
    }

    #[rstest]
    fn hash_consistency() {
        use std::collections::HashSet;

        let kind1 = create_weapon_kind();
        let kind2 = create_weapon_kind();
        let kind3 = create_armor_kind();

        let mut set = HashSet::new();
        set.insert(kind1);

        assert!(set.contains(&kind2));
        assert!(!set.contains(&kind3));
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[rstest]
    fn consumable_with_status_effect() {
        let kind = ItemKind::Consumable(ConsumableData::new(
            ConsumableEffect::ApplyStatus {
                effect: StatusEffectType::Haste,
                duration: 5,
            },
            5,
        ));
        assert!(kind.is_usable());
        assert!(kind.is_stackable());
        assert_eq!(kind.max_stack(), 5);
    }

    #[rstest]
    fn material_with_legendary_rarity() {
        let kind = ItemKind::Material(MaterialData::new(Rarity::Legendary, 5));
        assert!(kind.is_stackable());
        assert_eq!(kind.max_stack(), 5);
    }
}
