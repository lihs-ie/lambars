//! Weapon types and data structures.
//!
//! This module provides types for representing weapons in the game,
//! including weapon types and weapon-specific data.

use std::fmt;

use crate::common::Attack;

// =============================================================================
// WeaponType
// =============================================================================

/// Types of weapons available in the game.
///
/// Each weapon type has different characteristics and use cases.
///
/// # Examples
///
/// ```
/// use roguelike_domain::item::WeaponType;
///
/// let weapon_type = WeaponType::Sword;
/// assert!(weapon_type.is_melee());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeaponType {
    /// A balanced melee weapon.
    Sword,
    /// A heavy melee weapon with high damage.
    Axe,
    /// A long melee weapon with extended reach.
    Spear,
    /// A ranged weapon for attacking from a distance.
    Bow,
    /// A magical weapon for spellcasters.
    Staff,
    /// A fast melee weapon with low damage.
    Dagger,
}

impl WeaponType {
    /// Returns true if this is a melee weapon.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::item::WeaponType;
    ///
    /// assert!(WeaponType::Sword.is_melee());
    /// assert!(!WeaponType::Bow.is_melee());
    /// ```
    #[must_use]
    pub const fn is_melee(&self) -> bool {
        matches!(
            self,
            Self::Sword | Self::Axe | Self::Spear | Self::Dagger
        )
    }

    /// Returns true if this is a ranged weapon.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::item::WeaponType;
    ///
    /// assert!(WeaponType::Bow.is_ranged());
    /// assert!(!WeaponType::Sword.is_ranged());
    /// ```
    #[must_use]
    pub const fn is_ranged(&self) -> bool {
        matches!(self, Self::Bow)
    }

    /// Returns true if this is a magical weapon.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::item::WeaponType;
    ///
    /// assert!(WeaponType::Staff.is_magical());
    /// assert!(!WeaponType::Sword.is_magical());
    /// ```
    #[must_use]
    pub const fn is_magical(&self) -> bool {
        matches!(self, Self::Staff)
    }

    /// Returns the base attack speed modifier for this weapon type.
    ///
    /// Higher values indicate faster attack speeds.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::item::WeaponType;
    ///
    /// assert!(WeaponType::Dagger.attack_speed_modifier() > WeaponType::Axe.attack_speed_modifier());
    /// ```
    #[must_use]
    pub const fn attack_speed_modifier(&self) -> f32 {
        match self {
            Self::Dagger => 1.5,
            Self::Sword => 1.0,
            Self::Spear => 0.9,
            Self::Bow => 0.8,
            Self::Staff => 0.7,
            Self::Axe => 0.6,
        }
    }

    /// Returns an array of all weapon types.
    #[must_use]
    pub const fn all() -> [Self; 6] {
        [
            Self::Sword,
            Self::Axe,
            Self::Spear,
            Self::Bow,
            Self::Staff,
            Self::Dagger,
        ]
    }
}

impl fmt::Display for WeaponType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Sword => "Sword",
            Self::Axe => "Axe",
            Self::Spear => "Spear",
            Self::Bow => "Bow",
            Self::Staff => "Staff",
            Self::Dagger => "Dagger",
        };
        write!(formatter, "{}", name)
    }
}

// =============================================================================
// WeaponData
// =============================================================================

/// Data specific to weapon items.
///
/// Contains the attack bonus, weapon type, and attack range.
///
/// # Examples
///
/// ```
/// use roguelike_domain::item::{WeaponData, WeaponType};
/// use roguelike_domain::common::Attack;
///
/// let weapon = WeaponData::new(Attack::new(25), WeaponType::Sword, 1);
/// assert_eq!(weapon.attack_bonus().value(), 25);
/// assert_eq!(weapon.weapon_type(), WeaponType::Sword);
/// assert_eq!(weapon.range(), 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WeaponData {
    attack_bonus: Attack,
    weapon_type: WeaponType,
    range: u32,
}

impl WeaponData {
    /// Creates a new `WeaponData`.
    ///
    /// # Arguments
    ///
    /// * `attack_bonus` - The attack bonus provided by the weapon
    /// * `weapon_type` - The type of weapon
    /// * `range` - The attack range in tiles
    #[must_use]
    pub const fn new(attack_bonus: Attack, weapon_type: WeaponType, range: u32) -> Self {
        Self {
            attack_bonus,
            weapon_type,
            range,
        }
    }

    /// Returns the attack bonus provided by the weapon.
    #[must_use]
    pub const fn attack_bonus(&self) -> Attack {
        self.attack_bonus
    }

    /// Returns the type of weapon.
    #[must_use]
    pub const fn weapon_type(&self) -> WeaponType {
        self.weapon_type
    }

    /// Returns the attack range in tiles.
    #[must_use]
    pub const fn range(&self) -> u32 {
        self.range
    }

    /// Returns a new `WeaponData` with the given attack bonus.
    #[must_use]
    pub const fn with_attack_bonus(&self, attack_bonus: Attack) -> Self {
        Self {
            attack_bonus,
            weapon_type: self.weapon_type,
            range: self.range,
        }
    }
}

impl fmt::Display for WeaponData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} (+{} ATK, range: {})",
            self.weapon_type,
            self.attack_bonus.value(),
            self.range
        )
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
    // WeaponType Tests
    // =========================================================================

    mod weapon_type {
        use super::*;

        #[rstest]
        #[case(WeaponType::Sword, true)]
        #[case(WeaponType::Axe, true)]
        #[case(WeaponType::Spear, true)]
        #[case(WeaponType::Dagger, true)]
        #[case(WeaponType::Bow, false)]
        #[case(WeaponType::Staff, false)]
        fn is_melee(#[case] weapon_type: WeaponType, #[case] expected: bool) {
            assert_eq!(weapon_type.is_melee(), expected);
        }

        #[rstest]
        #[case(WeaponType::Bow, true)]
        #[case(WeaponType::Sword, false)]
        #[case(WeaponType::Staff, false)]
        fn is_ranged(#[case] weapon_type: WeaponType, #[case] expected: bool) {
            assert_eq!(weapon_type.is_ranged(), expected);
        }

        #[rstest]
        #[case(WeaponType::Staff, true)]
        #[case(WeaponType::Sword, false)]
        #[case(WeaponType::Bow, false)]
        fn is_magical(#[case] weapon_type: WeaponType, #[case] expected: bool) {
            assert_eq!(weapon_type.is_magical(), expected);
        }

        #[rstest]
        fn categories_are_mutually_exclusive() {
            for weapon_type in WeaponType::all() {
                let categories = [
                    weapon_type.is_melee(),
                    weapon_type.is_ranged(),
                    weapon_type.is_magical(),
                ];
                let count = categories.iter().filter(|&&b| b).count();
                // Each weapon should be in exactly one category
                assert_eq!(
                    count, 1,
                    "{:?} should be in exactly one category",
                    weapon_type
                );
            }
        }

        #[rstest]
        fn attack_speed_modifier_dagger_fastest() {
            let dagger_speed = WeaponType::Dagger.attack_speed_modifier();
            for weapon_type in WeaponType::all() {
                if weapon_type != WeaponType::Dagger {
                    assert!(
                        dagger_speed > weapon_type.attack_speed_modifier(),
                        "Dagger should be faster than {:?}",
                        weapon_type
                    );
                }
            }
        }

        #[rstest]
        fn attack_speed_modifier_axe_slowest() {
            let axe_speed = WeaponType::Axe.attack_speed_modifier();
            for weapon_type in WeaponType::all() {
                if weapon_type != WeaponType::Axe {
                    assert!(
                        axe_speed < weapon_type.attack_speed_modifier(),
                        "Axe should be slower than {:?}",
                        weapon_type
                    );
                }
            }
        }

        #[rstest]
        fn all_returns_six_variants() {
            assert_eq!(WeaponType::all().len(), 6);
        }

        #[rstest]
        #[case(WeaponType::Sword, "Sword")]
        #[case(WeaponType::Axe, "Axe")]
        #[case(WeaponType::Spear, "Spear")]
        #[case(WeaponType::Bow, "Bow")]
        #[case(WeaponType::Staff, "Staff")]
        #[case(WeaponType::Dagger, "Dagger")]
        fn display_format(#[case] weapon_type: WeaponType, #[case] expected: &str) {
            assert_eq!(format!("{}", weapon_type), expected);
        }

        #[rstest]
        fn equality() {
            assert_eq!(WeaponType::Sword, WeaponType::Sword);
            assert_ne!(WeaponType::Sword, WeaponType::Axe);
        }

        #[rstest]
        fn clone() {
            let weapon_type = WeaponType::Sword;
            let cloned = weapon_type;
            assert_eq!(weapon_type, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let mut set = HashSet::new();
            set.insert(WeaponType::Sword);

            assert!(set.contains(&WeaponType::Sword));
            assert!(!set.contains(&WeaponType::Axe));
        }
    }

    // =========================================================================
    // WeaponData Tests
    // =========================================================================

    mod weapon_data {
        use super::*;

        fn create_weapon_data() -> WeaponData {
            WeaponData::new(Attack::new(25), WeaponType::Sword, 1)
        }

        #[rstest]
        fn new_creates_weapon_data() {
            let weapon = create_weapon_data();
            assert_eq!(weapon.attack_bonus().value(), 25);
            assert_eq!(weapon.weapon_type(), WeaponType::Sword);
            assert_eq!(weapon.range(), 1);
        }

        #[rstest]
        fn new_with_zero_attack() {
            let weapon = WeaponData::new(Attack::new(0), WeaponType::Dagger, 1);
            assert_eq!(weapon.attack_bonus().value(), 0);
        }

        #[rstest]
        fn new_with_high_range() {
            let weapon = WeaponData::new(Attack::new(10), WeaponType::Bow, 5);
            assert_eq!(weapon.range(), 5);
        }

        #[rstest]
        fn with_attack_bonus_changes_attack() {
            let weapon = create_weapon_data();
            let modified = weapon.with_attack_bonus(Attack::new(50));

            assert_eq!(modified.attack_bonus().value(), 50);
            assert_eq!(modified.weapon_type(), WeaponType::Sword);
            assert_eq!(modified.range(), 1);
        }

        #[rstest]
        fn display_format() {
            let weapon = create_weapon_data();
            assert_eq!(format!("{}", weapon), "Sword (+25 ATK, range: 1)");
        }

        #[rstest]
        fn display_format_bow() {
            let weapon = WeaponData::new(Attack::new(15), WeaponType::Bow, 5);
            assert_eq!(format!("{}", weapon), "Bow (+15 ATK, range: 5)");
        }

        #[rstest]
        fn equality() {
            let weapon1 = create_weapon_data();
            let weapon2 = create_weapon_data();
            let weapon3 = WeaponData::new(Attack::new(30), WeaponType::Sword, 1);

            assert_eq!(weapon1, weapon2);
            assert_ne!(weapon1, weapon3);
        }

        #[rstest]
        fn equality_different_type() {
            let weapon1 = create_weapon_data();
            let weapon2 = WeaponData::new(Attack::new(25), WeaponType::Axe, 1);

            assert_ne!(weapon1, weapon2);
        }

        #[rstest]
        fn equality_different_range() {
            let weapon1 = create_weapon_data();
            let weapon2 = WeaponData::new(Attack::new(25), WeaponType::Sword, 2);

            assert_ne!(weapon1, weapon2);
        }

        #[rstest]
        fn clone() {
            let weapon = create_weapon_data();
            let cloned = weapon;
            assert_eq!(weapon, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let weapon1 = create_weapon_data();
            let weapon2 = create_weapon_data();
            let weapon3 = WeaponData::new(Attack::new(30), WeaponType::Axe, 1);

            let mut set = HashSet::new();
            set.insert(weapon1);

            assert!(set.contains(&weapon2));
            assert!(!set.contains(&weapon3));
        }

        #[rstest]
        fn debug_format() {
            let weapon = create_weapon_data();
            let debug_string = format!("{:?}", weapon);
            assert!(debug_string.contains("WeaponData"));
            assert!(debug_string.contains("attack_bonus"));
        }
    }
}
