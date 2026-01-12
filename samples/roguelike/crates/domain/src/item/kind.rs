
use std::fmt;

use super::armor::ArmorData;
use super::consumable::ConsumableData;
use super::material::MaterialData;
use super::weapon::WeaponData;

// =============================================================================
// ItemKind
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemKind {
    Weapon(WeaponData),
    Armor(ArmorData),
    Consumable(ConsumableData),
    Material(MaterialData),
}

impl ItemKind {
    #[must_use]
    pub const fn is_equipable(&self) -> bool {
        matches!(self, Self::Weapon(_) | Self::Armor(_))
    }

    #[must_use]
    pub const fn is_usable(&self) -> bool {
        matches!(self, Self::Consumable(_))
    }

    #[must_use]
    pub const fn is_stackable(&self) -> bool {
        matches!(self, Self::Consumable(_) | Self::Material(_))
    }

    #[must_use]
    pub const fn max_stack(&self) -> u32 {
        match self {
            Self::Weapon(_) | Self::Armor(_) => 1,
            Self::Consumable(data) => data.max_stack(),
            Self::Material(data) => data.max_stack(),
        }
    }

    #[must_use]
    pub const fn as_weapon(&self) -> Option<&WeaponData> {
        match self {
            Self::Weapon(data) => Some(data),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_armor(&self) -> Option<&ArmorData> {
        match self {
            Self::Armor(data) => Some(data),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_consumable(&self) -> Option<&ConsumableData> {
        match self {
            Self::Consumable(data) => Some(data),
            _ => None,
        }
    }

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
