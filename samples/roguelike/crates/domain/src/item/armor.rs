
use std::fmt;

use crate::common::Defense;

// =============================================================================
// ArmorSlot
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArmorSlot {
    Body,
    Head,
    Accessory,
}

impl ArmorSlot {
    #[must_use]
    pub const fn defense_multiplier(&self) -> f32 {
        match self {
            Self::Body => 1.0,
            Self::Head => 0.6,
            Self::Accessory => 0.3,
        }
    }

    #[must_use]
    pub const fn all() -> [Self; 3] {
        [Self::Body, Self::Head, Self::Accessory]
    }
}

impl fmt::Display for ArmorSlot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Body => "Body",
            Self::Head => "Head",
            Self::Accessory => "Accessory",
        };
        write!(formatter, "{}", name)
    }
}

// =============================================================================
// ArmorData
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArmorData {
    defense_bonus: Defense,
    armor_slot: ArmorSlot,
}

impl ArmorData {
    #[must_use]
    pub const fn new(defense_bonus: Defense, armor_slot: ArmorSlot) -> Self {
        Self {
            defense_bonus,
            armor_slot,
        }
    }

    #[must_use]
    pub const fn defense_bonus(&self) -> Defense {
        self.defense_bonus
    }

    #[must_use]
    pub const fn armor_slot(&self) -> ArmorSlot {
        self.armor_slot
    }

    #[must_use]
    pub fn effective_defense(&self) -> u32 {
        let base = self.defense_bonus.value() as f32;
        let multiplier = self.armor_slot.defense_multiplier();
        (base * multiplier) as u32
    }

    #[must_use]
    pub const fn with_defense_bonus(&self, defense_bonus: Defense) -> Self {
        Self {
            defense_bonus,
            armor_slot: self.armor_slot,
        }
    }
}

impl fmt::Display for ArmorData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} Armor (+{} DEF)",
            self.armor_slot,
            self.defense_bonus.value()
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
    // ArmorSlot Tests
    // =========================================================================

    mod armor_slot {
        use super::*;

        #[rstest]
        #[case(ArmorSlot::Body, 1.0)]
        #[case(ArmorSlot::Head, 0.6)]
        #[case(ArmorSlot::Accessory, 0.3)]
        fn defense_multiplier(#[case] slot: ArmorSlot, #[case] expected: f32) {
            let actual = slot.defense_multiplier();
            assert!((actual - expected).abs() < 1e-6);
        }

        #[rstest]
        fn body_has_highest_multiplier() {
            let body_mult = ArmorSlot::Body.defense_multiplier();
            for slot in ArmorSlot::all() {
                if slot != ArmorSlot::Body {
                    assert!(
                        body_mult > slot.defense_multiplier(),
                        "Body should have higher multiplier than {:?}",
                        slot
                    );
                }
            }
        }

        #[rstest]
        fn all_returns_three_variants() {
            assert_eq!(ArmorSlot::all().len(), 3);
        }

        #[rstest]
        #[case(ArmorSlot::Body, "Body")]
        #[case(ArmorSlot::Head, "Head")]
        #[case(ArmorSlot::Accessory, "Accessory")]
        fn display_format(#[case] slot: ArmorSlot, #[case] expected: &str) {
            assert_eq!(format!("{}", slot), expected);
        }

        #[rstest]
        fn equality() {
            assert_eq!(ArmorSlot::Body, ArmorSlot::Body);
            assert_ne!(ArmorSlot::Body, ArmorSlot::Head);
        }

        #[rstest]
        fn clone() {
            let slot = ArmorSlot::Body;
            let cloned = slot;
            assert_eq!(slot, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let mut set = HashSet::new();
            set.insert(ArmorSlot::Body);

            assert!(set.contains(&ArmorSlot::Body));
            assert!(!set.contains(&ArmorSlot::Head));
        }
    }

    // =========================================================================
    // ArmorData Tests
    // =========================================================================

    mod armor_data {
        use super::*;

        fn create_armor_data() -> ArmorData {
            ArmorData::new(Defense::new(15), ArmorSlot::Body)
        }

        #[rstest]
        fn new_creates_armor_data() {
            let armor = create_armor_data();
            assert_eq!(armor.defense_bonus().value(), 15);
            assert_eq!(armor.armor_slot(), ArmorSlot::Body);
        }

        #[rstest]
        fn new_with_zero_defense() {
            let armor = ArmorData::new(Defense::new(0), ArmorSlot::Head);
            assert_eq!(armor.defense_bonus().value(), 0);
        }

        #[rstest]
        fn effective_defense_body() {
            let armor = ArmorData::new(Defense::new(100), ArmorSlot::Body);
            // 100 * 1.0 = 100
            assert_eq!(armor.effective_defense(), 100);
        }

        #[rstest]
        fn effective_defense_head() {
            let armor = ArmorData::new(Defense::new(100), ArmorSlot::Head);
            // 100 * 0.6 = 60
            assert_eq!(armor.effective_defense(), 60);
        }

        #[rstest]
        fn effective_defense_accessory() {
            let armor = ArmorData::new(Defense::new(100), ArmorSlot::Accessory);
            // 100 * 0.3 = 30
            assert_eq!(armor.effective_defense(), 30);
        }

        #[rstest]
        fn with_defense_bonus_changes_defense() {
            let armor = create_armor_data();
            let modified = armor.with_defense_bonus(Defense::new(30));

            assert_eq!(modified.defense_bonus().value(), 30);
            assert_eq!(modified.armor_slot(), ArmorSlot::Body);
        }

        #[rstest]
        fn display_format_body() {
            let armor = create_armor_data();
            assert_eq!(format!("{}", armor), "Body Armor (+15 DEF)");
        }

        #[rstest]
        fn display_format_head() {
            let armor = ArmorData::new(Defense::new(10), ArmorSlot::Head);
            assert_eq!(format!("{}", armor), "Head Armor (+10 DEF)");
        }

        #[rstest]
        fn display_format_accessory() {
            let armor = ArmorData::new(Defense::new(5), ArmorSlot::Accessory);
            assert_eq!(format!("{}", armor), "Accessory Armor (+5 DEF)");
        }

        #[rstest]
        fn equality() {
            let armor1 = create_armor_data();
            let armor2 = create_armor_data();
            let armor3 = ArmorData::new(Defense::new(20), ArmorSlot::Body);

            assert_eq!(armor1, armor2);
            assert_ne!(armor1, armor3);
        }

        #[rstest]
        fn equality_different_slot() {
            let armor1 = create_armor_data();
            let armor2 = ArmorData::new(Defense::new(15), ArmorSlot::Head);

            assert_ne!(armor1, armor2);
        }

        #[rstest]
        fn clone() {
            let armor = create_armor_data();
            let cloned = armor;
            assert_eq!(armor, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let armor1 = create_armor_data();
            let armor2 = create_armor_data();
            let armor3 = ArmorData::new(Defense::new(20), ArmorSlot::Head);

            let mut set = HashSet::new();
            set.insert(armor1);

            assert!(set.contains(&armor2));
            assert!(!set.contains(&armor3));
        }

        #[rstest]
        fn debug_format() {
            let armor = create_armor_data();
            let debug_string = format!("{:?}", armor);
            assert!(debug_string.contains("ArmorData"));
            assert!(debug_string.contains("defense_bonus"));
        }
    }
}
