//! Item effect types for domain events.
//!
//! This module provides the `ItemEffect` enum that represents
//! the effects produced when items are used.

use std::fmt;

use crate::common::StatusEffectType;

// =============================================================================
// ItemEffect
// =============================================================================

/// Effects produced when an item is used.
///
/// This enum is used in domain events to describe what happened
/// when a consumable item was used.
///
/// # Examples
///
/// ```
/// use roguelike_domain::item::ItemEffect;
///
/// let effect = ItemEffect::Healed { amount: 50 };
/// println!("Effect: {}", effect);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemEffect {
    /// Health was restored.
    Healed {
        /// The amount of health restored.
        amount: u32,
    },
    /// Mana was restored.
    ManaRestored {
        /// The amount of mana restored.
        amount: u32,
    },
    /// A status effect was applied.
    StatusApplied {
        /// The type of status effect that was applied.
        effect: StatusEffectType,
    },
    /// A status effect was removed.
    StatusRemoved {
        /// The type of status effect that was removed.
        effect: StatusEffectType,
    },
}

impl ItemEffect {
    /// Returns true if this effect restored health or mana.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::item::ItemEffect;
    ///
    /// assert!(ItemEffect::Healed { amount: 50 }.is_restoration());
    /// assert!(ItemEffect::ManaRestored { amount: 30 }.is_restoration());
    /// ```
    #[must_use]
    pub const fn is_restoration(&self) -> bool {
        matches!(self, Self::Healed { .. } | Self::ManaRestored { .. })
    }

    /// Returns true if this effect modified a status effect.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::item::ItemEffect;
    /// use roguelike_domain::common::StatusEffectType;
    ///
    /// let applied = ItemEffect::StatusApplied { effect: StatusEffectType::Haste };
    /// assert!(applied.is_status_related());
    /// ```
    #[must_use]
    pub const fn is_status_related(&self) -> bool {
        matches!(self, Self::StatusApplied { .. } | Self::StatusRemoved { .. })
    }

    /// Returns true if this effect is generally positive.
    ///
    /// Healing and mana restoration are always positive.
    /// Applying buffs and removing debuffs are positive.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::item::ItemEffect;
    /// use roguelike_domain::common::StatusEffectType;
    ///
    /// assert!(ItemEffect::Healed { amount: 50 }.is_positive());
    /// assert!(ItemEffect::StatusRemoved { effect: StatusEffectType::Poison }.is_positive());
    /// ```
    #[must_use]
    pub const fn is_positive(&self) -> bool {
        match self {
            Self::Healed { .. } | Self::ManaRestored { .. } => true,
            Self::StatusApplied { effect } => effect.is_buff(),
            Self::StatusRemoved { effect } => effect.is_debuff(),
        }
    }

    /// Returns the amount if this is a restoration effect, None otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::item::ItemEffect;
    ///
    /// let heal = ItemEffect::Healed { amount: 50 };
    /// assert_eq!(heal.restoration_amount(), Some(50));
    /// ```
    #[must_use]
    pub const fn restoration_amount(&self) -> Option<u32> {
        match self {
            Self::Healed { amount } | Self::ManaRestored { amount } => Some(*amount),
            _ => None,
        }
    }

    /// Returns the status effect type if this is a status-related effect.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::item::ItemEffect;
    /// use roguelike_domain::common::StatusEffectType;
    ///
    /// let applied = ItemEffect::StatusApplied { effect: StatusEffectType::Haste };
    /// assert_eq!(applied.status_effect_type(), Some(StatusEffectType::Haste));
    /// ```
    #[must_use]
    pub const fn status_effect_type(&self) -> Option<StatusEffectType> {
        match self {
            Self::StatusApplied { effect } | Self::StatusRemoved { effect } => Some(*effect),
            _ => None,
        }
    }
}

impl fmt::Display for ItemEffect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Healed { amount } => write!(formatter, "Healed {} HP", amount),
            Self::ManaRestored { amount } => write!(formatter, "Restored {} MP", amount),
            Self::StatusApplied { effect } => write!(formatter, "Applied {}", effect),
            Self::StatusRemoved { effect } => write!(formatter, "Removed {}", effect),
        }
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
    // is_restoration Tests
    // =========================================================================

    #[rstest]
    fn healed_is_restoration() {
        let effect = ItemEffect::Healed { amount: 50 };
        assert!(effect.is_restoration());
    }

    #[rstest]
    fn mana_restored_is_restoration() {
        let effect = ItemEffect::ManaRestored { amount: 30 };
        assert!(effect.is_restoration());
    }

    #[rstest]
    fn status_applied_is_not_restoration() {
        let effect = ItemEffect::StatusApplied {
            effect: StatusEffectType::Haste,
        };
        assert!(!effect.is_restoration());
    }

    #[rstest]
    fn status_removed_is_not_restoration() {
        let effect = ItemEffect::StatusRemoved {
            effect: StatusEffectType::Poison,
        };
        assert!(!effect.is_restoration());
    }

    // =========================================================================
    // is_status_related Tests
    // =========================================================================

    #[rstest]
    fn status_applied_is_status_related() {
        let effect = ItemEffect::StatusApplied {
            effect: StatusEffectType::Shield,
        };
        assert!(effect.is_status_related());
    }

    #[rstest]
    fn status_removed_is_status_related() {
        let effect = ItemEffect::StatusRemoved {
            effect: StatusEffectType::Burn,
        };
        assert!(effect.is_status_related());
    }

    #[rstest]
    fn healed_is_not_status_related() {
        let effect = ItemEffect::Healed { amount: 50 };
        assert!(!effect.is_status_related());
    }

    #[rstest]
    fn mana_restored_is_not_status_related() {
        let effect = ItemEffect::ManaRestored { amount: 30 };
        assert!(!effect.is_status_related());
    }

    // =========================================================================
    // is_positive Tests
    // =========================================================================

    #[rstest]
    fn healed_is_positive() {
        let effect = ItemEffect::Healed { amount: 50 };
        assert!(effect.is_positive());
    }

    #[rstest]
    fn mana_restored_is_positive() {
        let effect = ItemEffect::ManaRestored { amount: 30 };
        assert!(effect.is_positive());
    }

    #[rstest]
    fn applying_buff_is_positive() {
        let effect = ItemEffect::StatusApplied {
            effect: StatusEffectType::Haste,
        };
        assert!(effect.is_positive());
    }

    #[rstest]
    fn applying_debuff_is_not_positive() {
        let effect = ItemEffect::StatusApplied {
            effect: StatusEffectType::Poison,
        };
        assert!(!effect.is_positive());
    }

    #[rstest]
    fn removing_debuff_is_positive() {
        let effect = ItemEffect::StatusRemoved {
            effect: StatusEffectType::Poison,
        };
        assert!(effect.is_positive());
    }

    #[rstest]
    fn removing_buff_is_not_positive() {
        let effect = ItemEffect::StatusRemoved {
            effect: StatusEffectType::Haste,
        };
        assert!(!effect.is_positive());
    }

    // =========================================================================
    // restoration_amount Tests
    // =========================================================================

    #[rstest]
    fn healed_restoration_amount() {
        let effect = ItemEffect::Healed { amount: 50 };
        assert_eq!(effect.restoration_amount(), Some(50));
    }

    #[rstest]
    fn mana_restored_restoration_amount() {
        let effect = ItemEffect::ManaRestored { amount: 30 };
        assert_eq!(effect.restoration_amount(), Some(30));
    }

    #[rstest]
    fn status_applied_restoration_amount() {
        let effect = ItemEffect::StatusApplied {
            effect: StatusEffectType::Haste,
        };
        assert_eq!(effect.restoration_amount(), None);
    }

    #[rstest]
    fn status_removed_restoration_amount() {
        let effect = ItemEffect::StatusRemoved {
            effect: StatusEffectType::Poison,
        };
        assert_eq!(effect.restoration_amount(), None);
    }

    // =========================================================================
    // status_effect_type Tests
    // =========================================================================

    #[rstest]
    fn status_applied_status_effect_type() {
        let effect = ItemEffect::StatusApplied {
            effect: StatusEffectType::Regeneration,
        };
        assert_eq!(effect.status_effect_type(), Some(StatusEffectType::Regeneration));
    }

    #[rstest]
    fn status_removed_status_effect_type() {
        let effect = ItemEffect::StatusRemoved {
            effect: StatusEffectType::Freeze,
        };
        assert_eq!(effect.status_effect_type(), Some(StatusEffectType::Freeze));
    }

    #[rstest]
    fn healed_status_effect_type() {
        let effect = ItemEffect::Healed { amount: 50 };
        assert_eq!(effect.status_effect_type(), None);
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[rstest]
    fn display_healed() {
        let effect = ItemEffect::Healed { amount: 50 };
        assert_eq!(format!("{}", effect), "Healed 50 HP");
    }

    #[rstest]
    fn display_mana_restored() {
        let effect = ItemEffect::ManaRestored { amount: 30 };
        assert_eq!(format!("{}", effect), "Restored 30 MP");
    }

    #[rstest]
    fn display_status_applied() {
        let effect = ItemEffect::StatusApplied {
            effect: StatusEffectType::Haste,
        };
        assert_eq!(format!("{}", effect), "Applied Haste");
    }

    #[rstest]
    fn display_status_removed() {
        let effect = ItemEffect::StatusRemoved {
            effect: StatusEffectType::Poison,
        };
        assert_eq!(format!("{}", effect), "Removed Poison");
    }

    // =========================================================================
    // Equality and Hash Tests
    // =========================================================================

    #[rstest]
    fn equality_same_effect() {
        let effect1 = ItemEffect::Healed { amount: 50 };
        let effect2 = ItemEffect::Healed { amount: 50 };
        assert_eq!(effect1, effect2);
    }

    #[rstest]
    fn equality_different_amount() {
        let effect1 = ItemEffect::Healed { amount: 50 };
        let effect2 = ItemEffect::Healed { amount: 100 };
        assert_ne!(effect1, effect2);
    }

    #[rstest]
    fn equality_different_variant() {
        let effect1 = ItemEffect::Healed { amount: 50 };
        let effect2 = ItemEffect::ManaRestored { amount: 50 };
        assert_ne!(effect1, effect2);
    }

    #[rstest]
    fn clone() {
        let effect = ItemEffect::Healed { amount: 50 };
        let cloned = effect;
        assert_eq!(effect, cloned);
    }

    #[rstest]
    fn hash_consistency() {
        use std::collections::HashSet;

        let effect1 = ItemEffect::Healed { amount: 50 };
        let effect2 = ItemEffect::Healed { amount: 50 };
        let effect3 = ItemEffect::Healed { amount: 100 };

        let mut set = HashSet::new();
        set.insert(effect1);

        assert!(set.contains(&effect2));
        assert!(!set.contains(&effect3));
    }

    #[rstest]
    fn debug_format() {
        let effect = ItemEffect::Healed { amount: 50 };
        let debug_string = format!("{:?}", effect);
        assert!(debug_string.contains("Healed"));
        assert!(debug_string.contains("50"));
    }
}
