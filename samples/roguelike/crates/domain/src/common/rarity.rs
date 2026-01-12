use std::fmt;

// =============================================================================
// Rarity
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl Rarity {
    #[must_use]
    pub const fn tier(&self) -> u8 {
        match self {
            Self::Common => 1,
            Self::Uncommon => 2,
            Self::Rare => 3,
            Self::Epic => 4,
            Self::Legendary => 5,
        }
    }

    #[must_use]
    pub const fn drop_rate_multiplier(&self) -> f32 {
        match self {
            Self::Common => 1.0,
            Self::Uncommon => 0.5,
            Self::Rare => 0.2,
            Self::Epic => 0.05,
            Self::Legendary => 0.01,
        }
    }

    #[must_use]
    pub fn is_at_least(&self, other: &Self) -> bool {
        self >= other
    }

    #[must_use]
    pub const fn all() -> [Self; 5] {
        [
            Self::Common,
            Self::Uncommon,
            Self::Rare,
            Self::Epic,
            Self::Legendary,
        ]
    }
}

impl fmt::Display for Rarity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Common => "Common",
            Self::Uncommon => "Uncommon",
            Self::Rare => "Rare",
            Self::Epic => "Epic",
            Self::Legendary => "Legendary",
        };
        write!(formatter, "{}", name)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(Rarity::Common, 1)]
    #[case(Rarity::Uncommon, 2)]
    #[case(Rarity::Rare, 3)]
    #[case(Rarity::Epic, 4)]
    #[case(Rarity::Legendary, 5)]
    fn tier_returns_correct_value(#[case] rarity: Rarity, #[case] expected: u8) {
        assert_eq!(rarity.tier(), expected);
    }

    #[rstest]
    #[case(Rarity::Common, 1.0)]
    #[case(Rarity::Uncommon, 0.5)]
    #[case(Rarity::Rare, 0.2)]
    #[case(Rarity::Epic, 0.05)]
    #[case(Rarity::Legendary, 0.01)]
    fn drop_rate_multiplier_returns_correct_value(#[case] rarity: Rarity, #[case] expected: f32) {
        assert!((rarity.drop_rate_multiplier() - expected).abs() < 1e-6);
    }

    #[rstest]
    fn drop_rate_decreases_with_rarity() {
        let rarities = Rarity::all();
        for i in 0..rarities.len() - 1 {
            assert!(rarities[i].drop_rate_multiplier() > rarities[i + 1].drop_rate_multiplier());
        }
    }

    #[rstest]
    #[case(Rarity::Epic, Rarity::Rare, true)]
    #[case(Rarity::Rare, Rarity::Rare, true)]
    #[case(Rarity::Common, Rarity::Rare, false)]
    #[case(Rarity::Legendary, Rarity::Common, true)]
    #[case(Rarity::Common, Rarity::Legendary, false)]
    fn is_at_least(#[case] rarity: Rarity, #[case] threshold: Rarity, #[case] expected: bool) {
        assert_eq!(rarity.is_at_least(&threshold), expected);
    }

    #[rstest]
    fn all_returns_five_variants() {
        let all = Rarity::all();
        assert_eq!(all.len(), 5);
    }

    #[rstest]
    fn all_variants_in_order() {
        let all = Rarity::all();
        assert_eq!(all[0], Rarity::Common);
        assert_eq!(all[1], Rarity::Uncommon);
        assert_eq!(all[2], Rarity::Rare);
        assert_eq!(all[3], Rarity::Epic);
        assert_eq!(all[4], Rarity::Legendary);
    }

    #[rstest]
    #[case(Rarity::Common, "Common")]
    #[case(Rarity::Uncommon, "Uncommon")]
    #[case(Rarity::Rare, "Rare")]
    #[case(Rarity::Epic, "Epic")]
    #[case(Rarity::Legendary, "Legendary")]
    fn display_format(#[case] rarity: Rarity, #[case] expected: &str) {
        assert_eq!(format!("{}", rarity), expected);
    }

    #[rstest]
    fn ordering() {
        assert!(Rarity::Common < Rarity::Uncommon);
        assert!(Rarity::Uncommon < Rarity::Rare);
        assert!(Rarity::Rare < Rarity::Epic);
        assert!(Rarity::Epic < Rarity::Legendary);
    }

    #[rstest]
    fn equality() {
        assert_eq!(Rarity::Common, Rarity::Common);
        assert_ne!(Rarity::Common, Rarity::Rare);
    }

    #[rstest]
    fn clone() {
        let rarity = Rarity::Epic;
        let cloned = rarity;
        assert_eq!(rarity, cloned);
    }

    #[rstest]
    fn hash_consistency() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(Rarity::Epic);

        assert!(set.contains(&Rarity::Epic));
        assert!(!set.contains(&Rarity::Rare));
    }
}
