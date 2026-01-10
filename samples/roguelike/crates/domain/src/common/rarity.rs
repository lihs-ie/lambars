//! Item rarity types.
//!
//! This module provides the Rarity enum for categorizing item quality.

use std::fmt;

// =============================================================================
// Rarity
// =============================================================================

/// Item rarity tiers from most common to rarest.
///
/// Rarity affects drop rates and item quality.
///
/// # Examples
///
/// ```
/// use roguelike_domain::common::Rarity;
///
/// assert_eq!(Rarity::Common.tier(), 1);
/// assert_eq!(Rarity::Legendary.tier(), 5);
/// assert!(Rarity::Epic.is_at_least(&Rarity::Rare));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Rarity {
    /// Most common items.
    Common,
    /// Slightly uncommon items.
    Uncommon,
    /// Rare items with better stats.
    Rare,
    /// Very rare epic items.
    Epic,
    /// Extremely rare legendary items.
    Legendary,
}

impl Rarity {
    /// Returns the numeric tier (1-5) for this rarity.
    ///
    /// - Common: 1
    /// - Uncommon: 2
    /// - Rare: 3
    /// - Epic: 4
    /// - Legendary: 5
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::Rarity;
    ///
    /// assert_eq!(Rarity::Common.tier(), 1);
    /// assert_eq!(Rarity::Legendary.tier(), 5);
    /// ```
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

    /// Returns the drop rate multiplier for this rarity.
    ///
    /// Higher rarity items have lower drop rates.
    ///
    /// - Common: 1.0
    /// - Uncommon: 0.5
    /// - Rare: 0.2
    /// - Epic: 0.05
    /// - Legendary: 0.01
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

    /// Returns true if this rarity is at least as high as the given rarity.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::common::Rarity;
    ///
    /// assert!(Rarity::Epic.is_at_least(&Rarity::Rare));
    /// assert!(Rarity::Rare.is_at_least(&Rarity::Rare));
    /// assert!(!Rarity::Common.is_at_least(&Rarity::Rare));
    /// ```
    #[must_use]
    pub fn is_at_least(&self, other: &Self) -> bool {
        self >= other
    }

    /// Returns an array of all rarity variants in order.
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
