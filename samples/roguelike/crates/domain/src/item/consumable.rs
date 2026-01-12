
use std::fmt;

use crate::common::StatusEffectType;

// =============================================================================
// ConsumableEffect
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConsumableEffect {
    Heal {
        amount: u32,
    },
    RestoreMana {
        amount: u32,
    },
    ApplyStatus {
        effect: StatusEffectType,
        duration: u32,
    },
    RemoveStatus {
        effect: StatusEffectType,
    },
}

impl ConsumableEffect {
    #[must_use]
    pub const fn is_beneficial(&self) -> bool {
        match self {
            Self::Heal { .. } | Self::RestoreMana { .. } | Self::RemoveStatus { .. } => true,
            Self::ApplyStatus { effect, .. } => effect.is_buff(),
        }
    }

    #[must_use]
    pub const fn is_restoration(&self) -> bool {
        matches!(self, Self::Heal { .. } | Self::RestoreMana { .. })
    }

    #[must_use]
    pub const fn is_status_related(&self) -> bool {
        matches!(self, Self::ApplyStatus { .. } | Self::RemoveStatus { .. })
    }
}

impl fmt::Display for ConsumableEffect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Heal { amount } => write!(formatter, "Heal {} HP", amount),
            Self::RestoreMana { amount } => write!(formatter, "Restore {} MP", amount),
            Self::ApplyStatus { effect, duration } => {
                write!(formatter, "Apply {} for {} turns", effect, duration)
            }
            Self::RemoveStatus { effect } => write!(formatter, "Remove {}", effect),
        }
    }
}

// =============================================================================
// ConsumableData
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConsumableData {
    effect: ConsumableEffect,
    max_stack: u32,
}

impl ConsumableData {
    #[must_use]
    pub const fn new(effect: ConsumableEffect, max_stack: u32) -> Self {
        Self { effect, max_stack }
    }

    #[must_use]
    pub const fn effect(&self) -> ConsumableEffect {
        self.effect
    }

    #[must_use]
    pub const fn max_stack(&self) -> u32 {
        self.max_stack
    }

    #[must_use]
    pub const fn with_max_stack(&self, max_stack: u32) -> Self {
        Self {
            effect: self.effect,
            max_stack,
        }
    }
}

impl fmt::Display for ConsumableData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} (max stack: {})", self.effect, self.max_stack)
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
    // ConsumableEffect Tests
    // =========================================================================

    mod consumable_effect {
        use super::*;

        #[rstest]
        fn heal_is_beneficial() {
            let effect = ConsumableEffect::Heal { amount: 50 };
            assert!(effect.is_beneficial());
        }

        #[rstest]
        fn restore_mana_is_beneficial() {
            let effect = ConsumableEffect::RestoreMana { amount: 30 };
            assert!(effect.is_beneficial());
        }

        #[rstest]
        fn remove_status_is_beneficial() {
            let effect = ConsumableEffect::RemoveStatus {
                effect: StatusEffectType::Poison,
            };
            assert!(effect.is_beneficial());
        }

        #[rstest]
        fn apply_buff_is_beneficial() {
            let effect = ConsumableEffect::ApplyStatus {
                effect: StatusEffectType::Haste,
                duration: 5,
            };
            assert!(effect.is_beneficial());
        }

        #[rstest]
        fn apply_debuff_is_not_beneficial() {
            let effect = ConsumableEffect::ApplyStatus {
                effect: StatusEffectType::Poison,
                duration: 3,
            };
            assert!(!effect.is_beneficial());
        }

        #[rstest]
        fn heal_is_restoration() {
            let effect = ConsumableEffect::Heal { amount: 50 };
            assert!(effect.is_restoration());
        }

        #[rstest]
        fn restore_mana_is_restoration() {
            let effect = ConsumableEffect::RestoreMana { amount: 30 };
            assert!(effect.is_restoration());
        }

        #[rstest]
        fn apply_status_is_not_restoration() {
            let effect = ConsumableEffect::ApplyStatus {
                effect: StatusEffectType::Shield,
                duration: 10,
            };
            assert!(!effect.is_restoration());
        }

        #[rstest]
        fn apply_status_is_status_related() {
            let effect = ConsumableEffect::ApplyStatus {
                effect: StatusEffectType::Regeneration,
                duration: 5,
            };
            assert!(effect.is_status_related());
        }

        #[rstest]
        fn remove_status_is_status_related() {
            let effect = ConsumableEffect::RemoveStatus {
                effect: StatusEffectType::Burn,
            };
            assert!(effect.is_status_related());
        }

        #[rstest]
        fn heal_is_not_status_related() {
            let effect = ConsumableEffect::Heal { amount: 50 };
            assert!(!effect.is_status_related());
        }

        #[rstest]
        fn display_heal() {
            let effect = ConsumableEffect::Heal { amount: 50 };
            assert_eq!(format!("{}", effect), "Heal 50 HP");
        }

        #[rstest]
        fn display_restore_mana() {
            let effect = ConsumableEffect::RestoreMana { amount: 30 };
            assert_eq!(format!("{}", effect), "Restore 30 MP");
        }

        #[rstest]
        fn display_apply_status() {
            let effect = ConsumableEffect::ApplyStatus {
                effect: StatusEffectType::Haste,
                duration: 5,
            };
            assert_eq!(format!("{}", effect), "Apply Haste for 5 turns");
        }

        #[rstest]
        fn display_remove_status() {
            let effect = ConsumableEffect::RemoveStatus {
                effect: StatusEffectType::Poison,
            };
            assert_eq!(format!("{}", effect), "Remove Poison");
        }

        #[rstest]
        fn equality() {
            let effect1 = ConsumableEffect::Heal { amount: 50 };
            let effect2 = ConsumableEffect::Heal { amount: 50 };
            let effect3 = ConsumableEffect::Heal { amount: 100 };

            assert_eq!(effect1, effect2);
            assert_ne!(effect1, effect3);
        }

        #[rstest]
        fn equality_different_variants() {
            let heal = ConsumableEffect::Heal { amount: 50 };
            let mana = ConsumableEffect::RestoreMana { amount: 50 };

            assert_ne!(heal, mana);
        }

        #[rstest]
        fn clone() {
            let effect = ConsumableEffect::Heal { amount: 50 };
            let cloned = effect;
            assert_eq!(effect, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let effect1 = ConsumableEffect::Heal { amount: 50 };
            let effect2 = ConsumableEffect::Heal { amount: 50 };
            let effect3 = ConsumableEffect::Heal { amount: 100 };

            let mut set = HashSet::new();
            set.insert(effect1);

            assert!(set.contains(&effect2));
            assert!(!set.contains(&effect3));
        }
    }

    // =========================================================================
    // ConsumableData Tests
    // =========================================================================

    mod consumable_data {
        use super::*;

        fn create_consumable_data() -> ConsumableData {
            ConsumableData::new(ConsumableEffect::Heal { amount: 50 }, 10)
        }

        #[rstest]
        fn new_creates_consumable_data() {
            let consumable = create_consumable_data();
            assert_eq!(consumable.effect(), ConsumableEffect::Heal { amount: 50 });
            assert_eq!(consumable.max_stack(), 10);
        }

        #[rstest]
        fn new_with_zero_stack() {
            let consumable = ConsumableData::new(ConsumableEffect::Heal { amount: 50 }, 0);
            assert_eq!(consumable.max_stack(), 0);
        }

        #[rstest]
        fn with_max_stack_changes_stack() {
            let consumable = create_consumable_data();
            let modified = consumable.with_max_stack(20);

            assert_eq!(modified.max_stack(), 20);
            assert_eq!(modified.effect(), ConsumableEffect::Heal { amount: 50 });
        }

        #[rstest]
        fn display_format() {
            let consumable = create_consumable_data();
            assert_eq!(format!("{}", consumable), "Heal 50 HP (max stack: 10)");
        }

        #[rstest]
        fn display_format_status() {
            let consumable = ConsumableData::new(
                ConsumableEffect::ApplyStatus {
                    effect: StatusEffectType::Shield,
                    duration: 3,
                },
                5,
            );
            assert_eq!(
                format!("{}", consumable),
                "Apply Shield for 3 turns (max stack: 5)"
            );
        }

        #[rstest]
        fn equality() {
            let consumable1 = create_consumable_data();
            let consumable2 = create_consumable_data();
            let consumable3 = ConsumableData::new(ConsumableEffect::Heal { amount: 100 }, 10);

            assert_eq!(consumable1, consumable2);
            assert_ne!(consumable1, consumable3);
        }

        #[rstest]
        fn equality_different_stack() {
            let consumable1 = create_consumable_data();
            let consumable2 = ConsumableData::new(ConsumableEffect::Heal { amount: 50 }, 20);

            assert_ne!(consumable1, consumable2);
        }

        #[rstest]
        fn clone() {
            let consumable = create_consumable_data();
            let cloned = consumable;
            assert_eq!(consumable, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let consumable1 = create_consumable_data();
            let consumable2 = create_consumable_data();
            let consumable3 = ConsumableData::new(ConsumableEffect::RestoreMana { amount: 30 }, 5);

            let mut set = HashSet::new();
            set.insert(consumable1);

            assert!(set.contains(&consumable2));
            assert!(!set.contains(&consumable3));
        }

        #[rstest]
        fn debug_format() {
            let consumable = create_consumable_data();
            let debug_string = format!("{:?}", consumable);
            assert!(debug_string.contains("ConsumableData"));
            assert!(debug_string.contains("effect"));
        }
    }
}
